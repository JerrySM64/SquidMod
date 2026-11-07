use anyhow::{anyhow, Context, Result};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const PORT: u16 = 7331;
const CMD_READ_MEMORY: u8 = 0x04;
const CMD_UPLOAD_MEMORY: u8 = 0x41;
const CMD_RPC: u8 = 0x70;
const CMD_GET_BUFFER_SIZE: u8 = 0x51;
const STATUS_ONLY_ZEROS: u8 = 0xB0;
const RPC_UNUSED_ARG: u32 = 0xfecad0ba;
const DEFAULT_MAX_BUFFER_SIZE: usize = 0x400;
const TIMEOUT: Duration = Duration::from_millis(5000);

fn translate_address(addr: u64) -> u64 {
    const WIIU_OFFSET: u64 = 0x503000;
    let in_rodata = (0x10000000..=0x101DCB94).contains(&addr);
    let in_data   = (0x101DCBA0..=0x101E9710).contains(&addr);
    let in_bss    = (0x101EA000..=0x1026F568).contains(&addr);
    if in_rodata || in_data || in_bss { addr + WIIU_OFFSET } else { addr }
}

#[derive(Clone)]
pub struct WiiUMemory {
    stream: Arc<Mutex<TcpStream>>,
    max_buffer_size: usize,
}

impl WiiUMemory {
    pub fn connect(ip: &str) -> Result<Self> {
        let addr = format!("{}:{}", ip, PORT);
        let mut addrs = addr
            .to_socket_addrs()
            .context("Failed to resolve Wii U address")?;
        let sock_addr = addrs
            .next()
            .ok_or_else(|| anyhow!("No addresses resolved for {}", ip))?;
        let mut stream = TcpStream::connect_timeout(&sock_addr, TIMEOUT)
            .with_context(|| format!("Failed to connect to Wii U at {}", ip))?;
        stream.set_read_timeout(Some(TIMEOUT))?;
        stream.set_write_timeout(Some(TIMEOUT))?;
        stream
            .write_all(&[CMD_GET_BUFFER_SIZE])
            .context("Failed to send buffer size handshake")?;
        let mut buffer = [0u8; 4];
        stream
            .read_exact(&mut buffer)
            .context("Failed to read buffer size handshake response")?;
        let max_buffer_size = u32::from_be_bytes(buffer) as usize;
        Ok(Self {
            stream: Arc::new(Mutex::new(stream)),
            max_buffer_size: if max_buffer_size == 0 {
                DEFAULT_MAX_BUFFER_SIZE
            } else {
                max_buffer_size
            },
        })
    }

    fn tcp_read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        let mut stream = self.stream.lock().map_err(|_| anyhow!("Stream mutex poisoned"))?;
        let target = translate_address(address);
        let start = target as u32;
        let len = u32::try_from(length).map_err(|_| anyhow!("Read length too large"))?;
        let end = start
            .checked_add(len)
            .ok_or_else(|| anyhow!("Read address overflow (Wii U target: 0x{:08X})", target))?;
        let mut header = [0u8; 9];
        header[0] = CMD_READ_MEMORY;
        header[1..5].copy_from_slice(&start.to_be_bytes());
        header[5..9].copy_from_slice(&end.to_be_bytes());
        stream.write_all(&header).context("Failed to send read command")?;

        let mut status = [0u8; 1];
        stream
            .read_exact(&mut status)
            .context("Failed to read status byte")?;
        if status[0] == STATUS_ONLY_ZEROS {
            return Ok(vec![0u8; length]);
        }

        let mut result = Vec::with_capacity(length);
        let mut remaining = length;
        while remaining > 0 {
            let chunk_len = remaining.min(self.max_buffer_size);
            let mut buffer = vec![0u8; chunk_len];
            stream
                .read_exact(&mut buffer)
                .context("Failed to read chunk data")?;
            result.extend_from_slice(&buffer);
            remaining -= chunk_len;
        }
        Ok(result)
    }

    fn tcp_write(&self, address: u64, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        let mut stream = self.stream.lock().map_err(|_| anyhow!("Stream mutex poisoned"))?;
        let target = translate_address(address);
        let start = target as u32;
        let len = u32::try_from(data.len()).map_err(|_| anyhow!("Write length too large"))?;
        let end = start
            .checked_add(len)
            .ok_or_else(|| anyhow!("Write address overflow (Wii U target: 0x{:08X})", target))?;
        let mut header = [0u8; 9];
        header[0] = CMD_UPLOAD_MEMORY;
        header[1..5].copy_from_slice(&start.to_be_bytes());
        header[5..9].copy_from_slice(&end.to_be_bytes());
        stream.write_all(&header).context("Failed to send write command")?;
        let mut offset = 0usize;
        while offset < data.len() {
            let chunk_len = (data.len() - offset).min(self.max_buffer_size);
            stream
                .write_all(&data[offset..offset + chunk_len])
                .context("Failed to send write payload")?;
            offset += chunk_len;
        }
        stream.flush().context("Failed to flush write payload")?;
        Ok(())
    }

    pub fn read_bytes(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        self.tcp_read(address, length)
    }

    pub fn read_u32(&self, address: u64) -> Result<u32> {
        let bytes = self.read_bytes(address, 4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn write_bytes(&self, address: u64, data: &[u8]) -> Result<()> {
        self.tcp_write(address, data)
    }

    pub fn write_u8(&self, address: u64, value: u8) -> Result<()> {
        self.tcp_write(address, &[value])
    }

    pub fn write_u16(&self, address: u64, value: u16) -> Result<()> {
        self.tcp_write(address, &value.to_be_bytes())
    }

    pub fn write_u32(&self, address: u64, value: u32) -> Result<()> {
        self.tcp_write(address, &value.to_be_bytes())
    }

    pub fn call_function64(&self, address: u64, args: &[u32]) -> Result<u64> {
        let mut stream = self.stream.lock().map_err(|_| anyhow!("Stream mutex poisoned"))?;
        let target = translate_address(address) as u32;
        let mut packet = [0u8; 37];
        packet[0] = CMD_RPC;
        packet[1..5].copy_from_slice(&target.to_be_bytes());
        for i in 0..8 {
            let value = args.get(i).copied().unwrap_or(RPC_UNUSED_ARG);
            packet[5 + i * 4..9 + i * 4].copy_from_slice(&value.to_be_bytes());
        }
        stream.write_all(&packet).context("Failed to send RPC command")?;
        let mut response = [0u8; 8];
        stream
            .read_exact(&mut response)
            .context("Failed to read RPC response")?;
        Ok(u64::from_be_bytes(response))
    }

    pub fn call_function(&self, address: u64, args: &[u32]) -> Result<u32> {
        Ok((self.call_function64(address, args)? >> 32) as u32)
    }

    pub fn write_utf16be(&self, address: u64, text: &str) -> Result<()> {
        let mut buffer = Vec::new();
        for c in text.encode_utf16() {
            buffer.extend(&c.to_be_bytes());
        }
        self.tcp_write(address, &buffer)
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
