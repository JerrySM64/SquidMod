use anyhow::{Context, Result, anyhow};
use libc::{c_void, iovec, process_vm_readv, process_vm_writev};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
};

use crate::platforms::daemon::SOCKET_PATH;

pub struct MemoryRegion {
    pub start: u64,
    pub end: u64,
    pub permissions: String,
}

enum Inner {
    Direct { pid: i32, base_address: u64 },
    Ipc(Arc<Mutex<UnixStream>>),
}

const CMD_READ: u8 = 0x01;
const CMD_WRITE: u8 = 0x02;
const ACK: u8 = 0xAA;

pub struct ProcessMemory {
    inner: Inner,
}

impl Clone for ProcessMemory {
    fn clone(&self) -> Self {
        match &self.inner {
            Inner::Direct { pid, base_address } => Self {
                inner: Inner::Direct {
                    pid: *pid,
                    base_address: *base_address,
                },
            },
            Inner::Ipc(arc) => Self {
                inner: Inner::Ipc(Arc::clone(arc)),
            },
        }
    }
}

impl ProcessMemory {
    pub fn open(pid: i32, base_address: u64) -> Result<Self> {
        let scope = read_ptrace_scope();

        if scope == 0 {
            return Ok(Self {
                inner: Inner::Direct { pid, base_address },
            });
        }

        let mut exe = std::env::var("APPIMAGE").unwrap_or_default();
        if exe.is_empty() || !std::path::Path::new(&exe).exists() {
            let current = std::env::current_exe()?;
            let temp_exe = std::env::temp_dir().join("squidmod_root_daemon");
            std::fs::copy(&current, &temp_exe).context("Failed to copy daemon to tmp dir")?;

            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&temp_exe, std::fs::Permissions::from_mode(0o755))?;
            exe = temp_exe.to_string_lossy().into_owned();
        }

        let mut child = std::process::Command::new("pkexec")
            .arg(&exe)
            .arg("--root-daemon")
            .current_dir("/")
            .spawn()
            .context("Failed to spawn pkexec daemon")?;

        let mut stream = None;
        for _ in 0..60 {
            if let Ok(Some(status)) = child.try_wait() {
                return Err(anyhow::anyhow!(
                    "pkexec daemon crashed prematurely with status: {}",
                    status
                ));
            }
            match UnixStream::connect(crate::platforms::daemon::SOCKET_PATH) {
                Ok(s) => {
                    stream = Some(s);
                    break;
                }
                Err(_) => std::thread::sleep(std::time::Duration::from_millis(500)),
            }
        }
        let s = stream.ok_or_else(|| anyhow!("Could not connect to daemon at {SOCKET_PATH}"))?;
        s.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
        s.set_write_timeout(Some(std::time::Duration::from_secs(10)))?;

        Ok(Self {
            inner: Inner::Ipc(Arc::new(Mutex::new(s))),
        })
    }

    pub fn read_bytes(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        match &self.inner {
            Inner::Direct { pid, base_address } => {
                direct_read(*pid, base_address + address, length)
            }
            Inner::Ipc(arc) => {
                let mut stream = arc.lock().map_err(|_| anyhow!("Stream mutex poisoned"))?;
                let mut req = [0u8; 9];
                req[0] = CMD_READ;
                req[1..5].copy_from_slice(&(address as u32).to_be_bytes());
                req[5..9].copy_from_slice(&(length as u32).to_be_bytes());
                stream
                    .write_all(&req)
                    .context("Failed to send read command")?;
                let mut buf = vec![0u8; length];
                stream
                    .read_exact(&mut buf)
                    .context("Failed to read response")?;
                Ok(buf)
            }
        }
    }

    pub fn read_u32(&self, address: u64) -> Result<u32> {
        let b = self.read_bytes(address, 4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn write_bytes(&self, address: u64, data: &[u8]) -> Result<()> {
        match &self.inner {
            Inner::Direct { pid, base_address } => direct_write(*pid, base_address + address, data),
            Inner::Ipc(arc) => {
                let mut stream = arc.lock().map_err(|_| anyhow!("Stream mutex poisoned"))?;
                let mut req = vec![0u8; 9 + data.len()];
                req[0] = CMD_WRITE;
                req[1..5].copy_from_slice(&(address as u32).to_be_bytes());
                req[5..9].copy_from_slice(&(data.len() as u32).to_be_bytes());
                req[9..].copy_from_slice(data);
                stream
                    .write_all(&req)
                    .context("Failed to send write command")?;
                let mut ack = [0u8; 1];
                stream
                    .read_exact(&mut ack)
                    .context("Failed to read write ACK")?;
                if ack[0] != ACK {
                    return Err(anyhow!("Unexpected ACK byte: {:#04x}", ack[0]));
                }
                Ok(())
            }
        }
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

    pub fn read_utf16be(&self, address: u64, max_length: usize) -> Result<String> {
        let bytes = self.read_bytes(address, max_length * 2)?;
        let mut utf16_chars = Vec::new();
        for i in (0..bytes.len()).step_by(2) {
            if i + 1 >= bytes.len() {
                break;
            }
            let char_code = u16::from_be_bytes([bytes[i], bytes[i + 1]]);
            if char_code == 0 {
                break;
            }
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

fn direct_read(pid: i32, target_addr: u64, length: usize) -> Result<Vec<u8>> {
    let mut buffer = vec![0u8; length];
    let local_iov = iovec {
        iov_base: buffer.as_mut_ptr() as *mut c_void,
        iov_len: length,
    };
    let remote_iov = iovec {
        iov_base: target_addr as *mut c_void,
        iov_len: length,
    };
    let result = unsafe { process_vm_readv(pid, &local_iov, 1, &remote_iov, 1, 0) };
    if result == -1 {
        return Err(std::io::Error::last_os_error()).context("process_vm_readv failed");
    }
    Ok(buffer)
}

fn direct_write(pid: i32, target_addr: u64, data: &[u8]) -> Result<()> {
    let local_iov = iovec {
        iov_base: data.as_ptr() as *mut c_void,
        iov_len: data.len(),
    };
    let remote_iov = iovec {
        iov_base: target_addr as *mut c_void,
        iov_len: data.len(),
    };
    let result = unsafe { process_vm_writev(pid, &local_iov, 1, &remote_iov, 1, 0) };
    if result == -1 {
        return Err(std::io::Error::last_os_error()).context("process_vm_writev failed");
    }
    Ok(())
}

fn read_ptrace_scope() -> u8 {
    fs::read_to_string("/proc/sys/kernel/yama/ptrace_scope")
        .ok()
        .and_then(|s| s.trim().parse::<u8>().ok())
        .unwrap_or(1)
}

pub const TARGET_NAMES: &[&str] = &["cemu", "xapfish", ".cemu-wrapped"];

pub fn find_cemu_process() -> Result<i32> {
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let filename = entry.file_name();
        let pid_str = match filename.to_str() {
            Some(s) => s,
            None => continue,
        };
        let pid = match pid_str.parse::<i32>() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let comm_path = format!("/proc/{}/comm", pid);
        let comm = match fs::read_to_string(&comm_path) {
            Ok(c) => c.trim().to_string(),
            Err(_) => continue,
        };
        if TARGET_NAMES.contains(&comm.as_str()) {
            return Ok(pid);
        }
    }
    Err(anyhow!("Cemu process not found"))
}

pub fn parse_maps(pid: i32) -> Result<Vec<MemoryRegion>> {
    let path = format!("/proc/{}/maps", pid);
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut regions = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            continue;
        }
        let addrs: Vec<&str> = parts[0].split('-').collect();
        if addrs.len() != 2 {
            continue;
        }
        let start = u64::from_str_radix(addrs[0], 16)?;
        let end = u64::from_str_radix(addrs[1], 16)?;
        regions.push(MemoryRegion {
            start,
            end,
            permissions: parts[1].to_string(),
        });
    }
    Ok(regions)
}

pub fn find_suitable_region(regions: &[MemoryRegion]) -> Result<&MemoryRegion> {
    regions
        .iter()
        .find(|r| r.permissions.contains('r') && (r.end - r.start) >= crate::MIN_REGION_SIZE)
        .ok_or_else(|| anyhow!("No suitable memory region found"))
}
