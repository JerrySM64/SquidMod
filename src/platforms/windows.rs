#![allow(non_snake_case)]
use anyhow::{anyhow, Context, Result};
use std::ffi::OsString;
use std::os::windows::prelude::OsStringExt;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ, PROCESS_VM_WRITE, PROCESS_VM_OPERATION};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
use windows_sys::Win32::System::Memory::{VirtualQueryEx, MEMORY_BASIC_INFORMATION};
use windows_sys::Win32::System::Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory};

use std::sync::Arc;

pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    pub permissions: String,
}

pub struct SafeHandle(HANDLE);

unsafe impl Send for SafeHandle {}
unsafe impl Sync for SafeHandle {}

impl Drop for SafeHandle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0); }
    }
}

#[derive(Clone)]
pub struct ProcessMemory {
    pub pid: i32,
    pub handle: Arc<SafeHandle>,
    pub base_address: u64,
}

unsafe impl Send for ProcessMemory {}
unsafe impl Sync for ProcessMemory {}

impl ProcessMemory {
    pub fn open(pid: i32, base_address: u64) -> Result<Self> {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION, 0, pid as u32);
            if handle == std::ptr::null_mut() { return Err(std::io::Error::last_os_error()).context("OpenProcess failed"); }
            Ok(ProcessMemory { pid, handle: Arc::new(SafeHandle(handle)), base_address })
        }
    }

    pub fn read_bytes(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        let mut buffer = vec![0u8; length];
        let mut read = 0usize;
        let ok = unsafe { ReadProcessMemory(self.handle.0, (self.base_address + address) as *const _, buffer.as_mut_ptr() as *mut _, length, &mut read as *mut usize) };
        if ok == 0 { return Err(std::io::Error::last_os_error()).context("ReadProcessMemory failed"); }
        Ok(buffer)
    }

    pub fn read_u32(&self, address: u64) -> Result<u32> {
        let bytes = self.read_bytes(address, 4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn write_bytes(&self, address: u64, data: &[u8]) -> Result<()> {
        let mut written = 0usize;
        let ok = unsafe { WriteProcessMemory(self.handle.0, (self.base_address + address) as *mut _, data.as_ptr() as *const _, data.len(), &mut written as *mut usize) };
        if ok == 0 { return Err(std::io::Error::last_os_error()).context("WriteProcessMemory failed"); }
        Ok(())
    }

    pub fn write_u8(&self, address: u64, value: u8) -> Result<()> { self.write_bytes(address, &[value]) }
    pub fn write_u16(&self, address: u64, value: u16) -> Result<()> { self.write_bytes(address, &value.to_be_bytes()) }
    pub fn write_u32(&self, address: u64, value: u32) -> Result<()> { self.write_bytes(address, &value.to_be_bytes()) }
    pub fn write_utf16be(&self, address: u64, text: &str) -> Result<()> {
        let mut buffer = Vec::new();
        for c in text.encode_utf16() { buffer.extend(&c.to_be_bytes()); }
        self.write_bytes(address, &buffer)
    }

    pub fn read_utf16be(&self, address: u64, max_length: usize) -> Result<String> {
        let bytes = self.read_bytes(address, max_length * 2)?;
        let mut utf16_chars = Vec::new();
        for i in (0..bytes.len()).step_by(2) {
            if i + 1 >= bytes.len() { break; }
            let char_code = u16::from_be_bytes([bytes[i], bytes[i + 1]]);
            if char_code == 0 { break; }
            utf16_chars.push(char_code);
        }
        String::from_utf16(&utf16_chars).context("Invalid UTF-16 string")
    }

    pub fn read_pointer_chain(&self, offsets: &[u64]) -> Result<u64> {
        if offsets.is_empty() {
            return Err(anyhow!("Empty pointer chain"));
        }
        let mut addr = offsets[0];
        for &offset in &offsets[1..] {
            addr = self.read_u32(addr)? as u64;
            if addr == 0 {
                return Err(anyhow!("Null pointer in chain"));
            }
            addr += offset;
        }
        Ok(addr)
    }
}

pub const TARGET_NAMES: &[&str] = &["Cemu.exe", "xapfish.exe", "cemu.exe", "Xapfish.exe"];

fn utf16_cstr_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    OsString::from_wide(&buf[..len]).to_string_lossy().into_owned()
}

pub fn find_cemu_process() -> Result<i32> {
    unsafe {
    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if snapshot == std::ptr::null_mut() { return Err(std::io::Error::last_os_error()).context("CreateToolhelp32Snapshot failed"); }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snapshot, &mut entry) == 0 { CloseHandle(snapshot); return Err(std::io::Error::last_os_error()).context("Process32FirstW failed"); }
        loop {
            let name = utf16_cstr_to_string(&entry.szExeFile);
            for &t in TARGET_NAMES {
                if name.eq_ignore_ascii_case(t) { CloseHandle(snapshot); return Ok(entry.th32ProcessID as i32); }
            }
            if Process32NextW(snapshot, &mut entry) == 0 { break; }
        }
        CloseHandle(snapshot);
    }
    Err(anyhow!("Cemu process not found"))
}

pub fn parse_maps(pid: i32) -> Result<Vec<MemoryRegion>> {
    let mut regions = Vec::new();
    unsafe {
    let process_handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, 0, pid as u32);
    if process_handle == std::ptr::null_mut() { return Err(std::io::Error::last_os_error()).context("OpenProcess for VirtualQueryEx failed"); }
        let mut addr = 0usize;
        while addr < 0x7FFFFFFF_FFFFFFFFusize {
            let mut mbi: MEMORY_BASIC_INFORMATION = std::mem::zeroed();
            let res = VirtualQueryEx(process_handle, addr as *const _, &mut mbi, std::mem::size_of::<MEMORY_BASIC_INFORMATION>());
            if res == 0 { break; }
            let start = mbi.BaseAddress as u64;
            let end = (mbi.BaseAddress as u64).saturating_add(mbi.RegionSize as u64);
            let mut perms = String::new();
            if mbi.State == windows_sys::Win32::System::Memory::MEM_COMMIT { perms.push('r'); }
            regions.push(MemoryRegion { start, end, permissions: perms });
            addr = (mbi.BaseAddress as usize).saturating_add(mbi.RegionSize as usize);
        }
        CloseHandle(process_handle);
    }
    Ok(regions)
}

pub fn find_suitable_region(regions: &[MemoryRegion]) -> Result<&MemoryRegion> {
    regions.iter().find(|r| r.permissions.contains('r') && (r.end - r.start) >= crate::MIN_REGION_SIZE)
        .ok_or_else(|| anyhow!("No suitable memory region found"))
}
