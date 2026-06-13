use anyhow::{anyhow, Context, Result};
use mach2::{
    kern_return::KERN_SUCCESS,
    port::mach_port_t,
    traps,
    vm::{mach_vm_region},
    vm_prot::VM_PROT_READ,
    vm_region::{vm_region_basic_info_64, VM_REGION_BASIC_INFO_64},
    vm_types::{mach_vm_address_t, mach_vm_size_t},
};
use std::io::{Read, Write};
use std::mem;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};

use libproc::libproc::proc_pid;
use libproc::processes;

use crate::platforms::daemon::SOCKET_PATH;

pub const TARGET_NAMES: &[&str] = &["Cemu", "cemu", "cemu_release", "xapfish"];

const VM_REGION_BASIC_INFO_COUNT_64: u32 = 10;

const CMD_READ:  u8 = 0x01;
const CMD_WRITE: u8 = 0x02;
const ACK:       u8 = 0xAA;

pub struct MemoryRegion {
    pub start:       u64,
    pub end:         u64,
    pub permissions: String,
}

#[derive(Clone)]
pub struct ProcessMemory {
    stream:           Arc<Mutex<UnixStream>>,
    pub pid:          i32,
    pub base_address: u64,
}

fn get_password() -> Option<String> {
    let output = std::process::Command::new("/usr/bin/osascript")
        .args([
            "-e", r#"tell application "System Events""#,
            "-e", r#"activate"#,
            "-e", r#"set dlg to display dialog "SquidMod needs your administrator password to read Cemu's memory." with title "SquidMod" default answer "" with icon caution buttons {"OK"} default button "OK" with hidden answer"#,
            "-e", r#"text returned of dlg"#,
            "-e", r#"end tell"#,
        ])
        .output()
        .ok()?;

    if !output.status.success() { return None; }
    let mut pw = String::from_utf8_lossy(&output.stdout).to_string();
    while pw.ends_with('\n') || pw.ends_with('\r') { pw.pop(); }
    if pw.is_empty() { None } else { Some(pw) }
}

impl ProcessMemory {
    pub fn open(_pid: i32, _base_address: u64) -> Result<Self> {
        if unsafe { libc::geteuid() } != 0 {
            let password = get_password().ok_or_else(|| anyhow!("Authentication cancelled by user"))?;
            let exe = std::env::current_exe()?;
            
            let mut child = std::process::Command::new("sudo")
                .arg("-S").arg("--").arg(exe).arg("--root-daemon")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()?;
            
            if let Some(mut stdin) = child.stdin.take() {
                let _ = write!(stdin, "{}\n", password);
                let _ = stdin.flush();
            }
        }

        let mut stream = None;
        for _ in 0..60 {
            match UnixStream::connect(SOCKET_PATH) {
                Ok(s) => { stream = Some(s); break; }
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(500)),
            }
        }
        let s = stream.ok_or_else(|| anyhow!("Could not connect to daemon at {SOCKET_PATH}"))?;
        s.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
        s.set_write_timeout(Some(std::time::Duration::from_secs(10)))?;

        Ok(Self {
            stream:       Arc::new(Mutex::new(s)),
            pid:          0,
            base_address: 0,
        })
    }

    pub fn read_bytes(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        let mut stream = self.stream.lock().map_err(|_| anyhow!("Stream mutex poisoned"))?;
        let mut req = [0u8; 9];
        req[0] = CMD_READ;
        req[1..5].copy_from_slice(&(address as u32).to_be_bytes());
        req[5..9].copy_from_slice(&(length as u32).to_be_bytes());
        stream.write_all(&req).context("Failed to send read command")?;
        let mut buf = vec![0u8; length];
        stream.read_exact(&mut buf).context("Failed to read response")?;
        Ok(buf)
    }

    pub fn read_u32(&self, address: u64) -> Result<u32> {
        let b = self.read_bytes(address, 4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_utf16be(&self, address: u64, max_chars: usize) -> Result<String> {
        let bytes = self.read_bytes(address, max_chars * 2)?;
        let mut units = Vec::with_capacity(max_chars);
        for chunk in bytes.chunks_exact(2) {
            let unit = u16::from_be_bytes([chunk[0], chunk[1]]);
            if unit == 0 { break; }
            units.push(unit);
        }
        String::from_utf16(&units).context("Invalid UTF-16BE string")
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

    pub fn write_bytes(&self, address: u64, data: &[u8]) -> Result<()> {
        let mut stream = self.stream.lock().map_err(|_| anyhow!("Stream mutex poisoned"))?;
        let mut req = vec![0u8; 9 + data.len()];
        req[0] = CMD_WRITE;
        req[1..5].copy_from_slice(&(address as u32).to_be_bytes());
        req[5..9].copy_from_slice(&(data.len() as u32).to_be_bytes());
        req[9..].copy_from_slice(data);
        stream.write_all(&req).context("Failed to send write command")?;
        let mut ack = [0u8; 1];
        stream.read_exact(&mut ack).context("Failed to read write ACK")?;
        if ack[0] != ACK {
            return Err(anyhow!("Unexpected ACK byte: {:#04x}", ack[0]));
        }
        Ok(())
    }

    pub fn write_u8(&self, address: u64, value: u8) -> Result<()> {
        self.write_bytes(address, &[value])
    }

    pub fn write_u16(&self, address: u64, value: u16) -> Result<()> {
        self.write_bytes(address, &value.to_be_bytes())
    }

    pub fn write_u32(&self, address: u64, value: u32) -> Result<()> {
        self.write_bytes(address, &value.to_be_bytes())
    }

    pub fn write_utf16be(&self, address: u64, text: &str) -> Result<()> {
        let mut buf = Vec::with_capacity(text.len() * 2);
        for unit in text.encode_utf16() {
            buf.extend_from_slice(&unit.to_be_bytes());
        }
        self.write_bytes(address, &buf)
    }
}

pub fn find_cemu_process() -> Result<i32> {
    let pids = processes::pids_by_type(processes::ProcFilter::All)
        .map_err(|e| anyhow!("Failed to enumerate processes: {e}"))?;

    for pid in pids {
        if pid == 0 { continue; }
        let pid_i32 = pid as i32;

        if let Ok(name) = proc_pid::name(pid_i32) {
            let name_lc = name.to_lowercase();
            if TARGET_NAMES.iter().any(|t| name_lc == t.to_lowercase() || name_lc.contains(&t.to_lowercase())) {
                return Ok(pid_i32);
            }
        }

        if let Ok(exe_path) = proc_pid::pidpath(pid_i32) {
            if let Some(stem) = std::path::Path::new(&exe_path).file_stem().and_then(|s| s.to_str()) {
                let stem_lc = stem.to_lowercase();
                if TARGET_NAMES.iter().any(|t| stem_lc == t.to_lowercase() || stem_lc.contains(&t.to_lowercase())) {
                    return Ok(pid_i32);
                }
            }
        }
    }

    Err(anyhow!("Cemu process not found (is it running?)"))
}

pub fn parse_maps(pid: i32) -> Result<Vec<MemoryRegion>> {
    let mut task: mach_port_t = 0;
    let kr = unsafe { traps::task_for_pid(traps::mach_task_self(), pid, &mut task) };
    if kr != KERN_SUCCESS {
        return Err(anyhow!("task_for_pid({pid}) failed (kr={kr:#x})"));
    }

    let mut regions = Vec::new();
    let mut address: mach_vm_address_t = 0;

    loop {
        let mut size:        mach_vm_size_t        = 0;
        let mut count:       u32                   = VM_REGION_BASIC_INFO_COUNT_64;
        let mut info:        vm_region_basic_info_64 = unsafe { mem::zeroed() };
        let mut object_name: mach_port_t           = 0;

        let kr = unsafe {
            mach_vm_region(
                task,
                &mut address,
                &mut size,
                VM_REGION_BASIC_INFO_64 as i32,
                &mut info as *mut vm_region_basic_info_64 as *mut _,
                &mut count,
                &mut object_name,
            )
        };
        if kr != KERN_SUCCESS { break; }

        let mut perm = String::new();
        if (info.protection & VM_PROT_READ) != 0 { perm.push('r'); }

        regions.push(MemoryRegion {
            start:       address as u64,
            end:         (address as u64) + size as u64,
            permissions: perm,
        });

        address = address.wrapping_add(size);
    }

    Ok(regions)
}

pub fn find_suitable_region(regions: &[MemoryRegion]) -> Result<&MemoryRegion> {
    regions
        .iter()
        .find(|r| r.permissions.contains('r') && (r.end - r.start) >= crate::MIN_REGION_SIZE)
        .ok_or_else(|| anyhow!("No suitable memory region found"))
}
