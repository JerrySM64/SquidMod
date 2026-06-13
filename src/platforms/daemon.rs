use anyhow::{Result, anyhow};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};

pub const SOCKET_PATH: &str = "/tmp/squidmod.sock";

const CMD_READ: u8 = 0x01;
const CMD_WRITE: u8 = 0x02;
const ACK: u8 = 0xAA;

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use libc::{c_void, vm_deallocate};
    use libproc::libproc::proc_pid;
    use libproc::processes;
    use mach2::{
        kern_return::KERN_SUCCESS,
        port::mach_port_t,
        traps,
        vm::{mach_vm_read, mach_vm_region, mach_vm_write},
        vm_prot::VM_PROT_READ,
        vm_region::{VM_REGION_BASIC_INFO_64, vm_region_basic_info_64},
        vm_types::{mach_vm_address_t, mach_vm_size_t},
    };
    use std::mem;

    const TARGET_NAMES: &[&str] = &["Cemu", "cemu", "cemu_release", "xapfish"];
    const VM_REGION_BASIC_INFO_COUNT_64: u32 = 10;
    const PROBE_OFFSET: u64 = 0x0E00_0000;
    const PROBE_READ_LEN: usize = 20;

    pub fn run() -> Result<()> {
        let pid = find_cemu_process()?;

        let mut task: mach_port_t = 0;
        let kr = unsafe { traps::task_for_pid(traps::mach_task_self(), pid, &mut task) };
        if kr != KERN_SUCCESS {
            return Err(anyhow!("task_for_pid({pid}) failed (kr={kr:#x})"));
        }

        let region_start = unsafe { find_region_with_probe(task) }?;
        let base_address = region_start
            .wrapping_add(PROBE_OFFSET)
            .wrapping_sub(0x1000_0000_u64);

        let verify_addr = base_address.wrapping_add(0x1000_0000_u64);
        let verify_bytes = read_raw(task, verify_addr, PROBE_READ_LEN)?;
        if !verify_bytes
            .windows(crate::PATTERN.len())
            .any(|w| w == crate::PATTERN)
        {
            return Err(anyhow!("Pattern verification failed at {verify_addr:#x}"));
        }

        serve(|stream| handle_client(stream, task, base_address))
    }

    fn handle_client(mut stream: UnixStream, task: mach_port_t, base_address: u64) -> Result<()> {
        loop {
            let mut cmd = [0u8; 1];
            if stream.read_exact(&mut cmd).is_err() {
                break;
            }
            match cmd[0] {
                CMD_READ => {
                    let mut hdr = [0u8; 8];
                    stream.read_exact(&mut hdr)?;
                    let address = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]) as u64;
                    let length = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
                    let host_addr = base_address.wrapping_add(address);
                    let data = read_raw(task, host_addr, length)?;
                    stream.write_all(&data)?;
                }
                CMD_WRITE => {
                    let mut hdr = [0u8; 8];
                    stream.read_exact(&mut hdr)?;
                    let address = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]) as u64;
                    let length = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
                    let mut payload = vec![0u8; length];
                    stream.read_exact(&mut payload)?;
                    let host_addr = base_address.wrapping_add(address);
                    write_raw(task, host_addr, &payload)?;
                    stream.write_all(&[ACK])?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn read_raw(task: mach_port_t, target_addr: u64, length: usize) -> Result<Vec<u8>> {
        let mut data: *mut c_void = std::ptr::null_mut();
        let mut data_count: mach_vm_size_t = 0;
        let kr = unsafe {
            mach_vm_read(
                task,
                target_addr as mach_vm_address_t,
                length as mach_vm_size_t,
                &mut data as *mut *mut c_void as *mut _,
                &mut data_count as *mut mach_vm_size_t as *mut _,
            )
        };
        if kr != KERN_SUCCESS {
            return Err(anyhow!(
                "mach_vm_read({target_addr:#x}, {length}) failed (kr={kr:#x})"
            ));
        }
        let out =
            unsafe { std::slice::from_raw_parts(data as *const u8, data_count as usize).to_vec() };
        unsafe {
            vm_deallocate(
                traps::mach_task_self(),
                data as libc::vm_address_t,
                data_count as usize,
            );
        }
        Ok(out)
    }

    fn write_raw(task: mach_port_t, target_addr: u64, data: &[u8]) -> Result<()> {
        use mach2::message::mach_msg_type_number_t;
        let kr = unsafe {
            mach_vm_write(
                task,
                target_addr as mach_vm_address_t,
                data.as_ptr() as libc::vm_offset_t,
                data.len() as mach_msg_type_number_t,
            )
        };
        if kr != KERN_SUCCESS {
            return Err(anyhow!(
                "mach_vm_write({target_addr:#x}) failed (kr={kr:#x})"
            ));
        }
        Ok(())
    }

    unsafe fn find_region_with_probe(task: mach_port_t) -> Result<u64> {
        let mut address: mach_vm_address_t = 0;
        loop {
            let mut size: mach_vm_size_t = 0;
            let mut count: u32 = VM_REGION_BASIC_INFO_COUNT_64;
            let mut info: vm_region_basic_info_64 = unsafe { mem::zeroed() };
            let mut object_name: mach_port_t = 0;
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
            if kr != KERN_SUCCESS {
                break;
            }
            if (info.protection & VM_PROT_READ) != 0 {
                let probe_addr = (address as u64).wrapping_add(PROBE_OFFSET);
                match read_raw(task, probe_addr, PROBE_READ_LEN) {
                    Ok(bytes)
                        if bytes
                            .windows(crate::PATTERN.len())
                            .any(|w| w == crate::PATTERN) =>
                    {
                        return Ok(address as u64);
                    }
                    _ => {}
                }
            }
            address = address.wrapping_add(size);
        }
        Err(anyhow!("No Wii U RAM region found"))
    }

    fn find_cemu_process() -> Result<i32> {
        let pids = processes::pids_by_type(processes::ProcFilter::All)
            .map_err(|e| anyhow!("Failed to enumerate processes: {e}"))?;
        for pid in pids {
            if pid == 0 {
                continue;
            }
            let pid_i32 = pid as i32;
            if let Ok(name) = proc_pid::name(pid_i32) {
                let name_lc = name.to_lowercase();
                if TARGET_NAMES
                    .iter()
                    .any(|t| name_lc == t.to_lowercase() || name_lc.contains(&t.to_lowercase()))
                {
                    return Ok(pid_i32);
                }
            }
            if let Ok(exe_path) = proc_pid::pidpath(pid_i32) {
                if let Some(stem) = std::path::Path::new(&exe_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                {
                    let stem_lc = stem.to_lowercase();
                    if TARGET_NAMES
                        .iter()
                        .any(|t| stem_lc == t.to_lowercase() || stem_lc.contains(&t.to_lowercase()))
                    {
                        return Ok(pid_i32);
                    }
                }
            }
        }
        Err(anyhow!("Cemu process not found"))
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use std::fs;
    use std::io::BufRead;
    use std::os::unix::fs::FileExt;

    const TARGET_NAMES: &[&str] = &["cemu", "xapfish", ".cemu-wrapped"];
    const PROBE_OFFSET: u64 = 0x0E00_0000;
    const PROBE_READ_LEN: usize = 20;

    pub fn run() -> Result<()> {
        let pid = find_cemu_process()?;
        let mem_file = open_mem(pid)?;
        let base_address = find_base_address(pid, &mem_file)?;

        let mut verify = vec![0u8; PROBE_READ_LEN];
        mem_file.read_exact_at(&mut verify, base_address + 0x1000_0000)?;
        if !verify
            .windows(crate::PATTERN.len())
            .any(|w| w == crate::PATTERN)
        {
            return Err(anyhow!("Failed to find pattern"));
        }

        serve(|stream| handle_client(stream, &mem_file, base_address))
    }

    fn open_mem(pid: i32) -> Result<fs::File> {
        let path = format!("/proc/{pid}/mem");
        fs::File::options()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| anyhow!("Failed to open {path}: {e}"))
    }

    fn handle_client(mut stream: UnixStream, mem_file: &fs::File, base_address: u64) -> Result<()> {
        loop {
            let mut cmd = [0u8; 1];
            if stream.read_exact(&mut cmd).is_err() {
                break;
            }
            match cmd[0] {
                CMD_READ => {
                    let mut hdr = [0u8; 8];
                    stream.read_exact(&mut hdr)?;
                    let address = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]) as u64;
                    let length = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
                    let host_addr = base_address.wrapping_add(address);
                    let mut data = vec![0u8; length];
                    mem_file.read_exact_at(&mut data, host_addr)?;
                    stream.write_all(&data)?;
                }
                CMD_WRITE => {
                    let mut hdr = [0u8; 8];
                    stream.read_exact(&mut hdr)?;
                    let address = u32::from_be_bytes([hdr[0], hdr[1], hdr[2], hdr[3]]) as u64;
                    let length = u32::from_be_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
                    let mut payload = vec![0u8; length];
                    stream.read_exact(&mut payload)?;
                    let host_addr = base_address.wrapping_add(address);
                    mem_file.write_all_at(&payload, host_addr)?;
                    stream.write_all(&[ACK])?;
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn find_cemu_process() -> Result<i32> {
        for entry in fs::read_dir("/proc")? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let fname = entry.file_name();
            let pid_str = match fname.to_str() {
                Some(s) => s,
                None => continue,
            };
            let pid = match pid_str.parse::<i32>() {
                Ok(p) => p,
                Err(_) => continue,
            };
            let comm = match fs::read_to_string(format!("/proc/{pid}/comm")) {
                Ok(c) => c.trim().to_string(),
                Err(_) => continue,
            };
            if TARGET_NAMES.contains(&comm.as_str()) {
                return Ok(pid);
            }
        }
        Err(anyhow!("Cemu process not found"))
    }

    fn find_base_address(pid: i32, mem_file: &fs::File) -> Result<u64> {
        let path = format!("/proc/{pid}/maps");
        let file = fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 2 {
                continue;
            }
            if !parts[1].contains('r') {
                continue;
            }
            let addrs: Vec<&str> = parts[0].split('-').collect();
            if addrs.len() != 2 {
                continue;
            }
            let start = u64::from_str_radix(addrs[0], 16)?;
            let end = u64::from_str_radix(addrs[1], 16)?;
            if (end - start) < crate::MIN_REGION_SIZE {
                continue;
            }
            let probe_addr = start.wrapping_add(PROBE_OFFSET);
            let mut buf = vec![0u8; PROBE_READ_LEN];
            if mem_file.read_exact_at(&mut buf, probe_addr).is_ok() {
                if buf
                    .windows(crate::PATTERN.len())
                    .any(|w| w == crate::PATTERN)
                {
                    return Ok(start.wrapping_add(PROBE_OFFSET).wrapping_sub(0x1000_0000));
                }
            }
        }
        Err(anyhow!("Failed to find Wii U RAM region"))
    }
}

fn serve<F>(handler: F) -> Result<()>
where
    F: Fn(UnixStream) -> Result<()>,
{
    let _ = std::fs::remove_file(SOCKET_PATH);
    let listener = UnixListener::bind(SOCKET_PATH)
        .map_err(|e| anyhow!("Failed to bind {SOCKET_PATH}: {e}"))?;
    std::fs::set_permissions(SOCKET_PATH, std::fs::Permissions::from_mode(0o777))?;

    match listener.accept() {
        Ok((stream, _)) => {
            if let Err(e) = handler(stream) {
                eprintln!("[daemon] Client error: {e}");
            }
        }
        Err(e) => eprintln!("[daemon] Accept error: {e}"),
    }

    let _ = std::fs::remove_file(SOCKET_PATH);
    Ok(())
}

pub fn run_server() -> Result<()> {
    imp::run()
}
