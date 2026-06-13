use anyhow::{anyhow, Context, Result};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const PORT: u16 = 7331;
const CMD_READ_MEMORY: u8 = 0x04;
const CMD_POKE_8: u8 = 0x01;
const CMD_POKE_16: u8 = 0x02;
const CMD_POKE_32: u8 = 0x03;
const STATUS_NON_ZEROS: u8 = 0xBD;
const STATUS_ONLY_ZEROS: u8 = 0xB0;
const CHUNK_SIZE: usize = 0x5000;

fn translate_address(addr: u64) -> u64 {
    const WIIU_OFFSET: u64 = 0x503000;
    let in_rodata = addr >= 0x10000000 && addr <= 0x101DCB94;
    let in_data   = addr >= 0x101DCBA0 && addr <= 0x101E9710;
    let in_bss    = addr >= 0x101EA000 && addr <= 0x1026F568;
    if in_rodata || in_data || in_bss { addr + WIIU_OFFSET } else { addr }
}

#[derive(Clone)]
pub struct WiiUMemory {
    stream: Arc<Mutex<TcpStream>>,
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
        let stream =
            TcpStream::connect_timeout(&sock_addr, Duration::from_secs(5))
                .with_context(|| format!("Failed to connect to Wii U at {}", ip))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        Ok(Self {
            stream: Arc::new(Mutex::new(stream)),
        })
    }

    fn tcp_read(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        let mut stream = self.stream.lock().map_err(|_| anyhow!("Stream mutex poisoned"))?;
        let target = translate_address(address);
        let start = target as u32;
        let end = start.checked_add(length as u32)
            .ok_or_else(|| anyhow!("Read address overflow (Wii U target: 0x{:08X})", target))?;
        let mut header = [0u8; 9];
        header[0] = CMD_READ_MEMORY;
        header[1..5].copy_from_slice(&start.to_be_bytes());
        header[5..9].copy_from_slice(&end.to_be_bytes());
        stream.write_all(&header).context("Failed to send read command")?;

        let mut result = Vec::with_capacity(length);
        let mut remaining = length;
        while remaining > 0 {
            let chunk_len = remaining.min(CHUNK_SIZE);
            let mut status = [0u8; 1];
            stream.read_exact(&mut status).context("Failed to read chunk status")?;
            match status[0] {
                STATUS_NON_ZEROS => {
                    let mut buf = vec![0u8; chunk_len];
                    stream.read_exact(&mut buf).context("Failed to read chunk data")?;
                    result.extend_from_slice(&buf);
                }
                STATUS_ONLY_ZEROS => {
                    result.extend(std::iter::repeat(0u8).take(chunk_len));
                }
                b => return Err(anyhow!("Unexpected read status byte: 0x{:02X}", b)),
            }
            remaining -= chunk_len;
        }
        Ok(result)
    }

    fn tcp_write(&self, cmd: u8, address: u64, value: u32) -> Result<()> {
        let mut stream = self.stream.lock().map_err(|_| anyhow!("Stream mutex poisoned"))?;
        let target = translate_address(address);
        let addr = target as u32;
        let mut packet = [0u8; 9];
        packet[0] = cmd;
        packet[1..5].copy_from_slice(&addr.to_be_bytes());
        packet[5..9].copy_from_slice(&value.to_be_bytes());
        stream.write_all(&packet).context("Failed to send write command")
    }

    pub fn read_bytes(&self, address: u64, length: usize) -> Result<Vec<u8>> {
        let mut result = Vec::with_capacity(length);
        let mut remaining = length;
        let mut offset = 0u64;
        while remaining > 0 {
            let chunk = remaining.min(0x400);
            let bytes = self.tcp_read(address + offset, chunk)?;
            result.extend_from_slice(&bytes);
            remaining -= chunk;
            offset += chunk as u64;
        }
        Ok(result)
    }

    pub fn read_u32(&self, address: u64) -> Result<u32> {
        let bytes = self.read_bytes(address, 4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn write_bytes(&self, address: u64, data: &[u8]) -> Result<()> {
        let padded_len = (data.len() + 3) & !3;
        let mut padded = data.to_vec();
        padded.resize(padded_len, 0);
        let mut offset = 0usize;
        while offset < padded.len() {
            let value = u32::from_be_bytes([
                padded[offset],
                padded[offset + 1],
                padded[offset + 2],
                padded[offset + 3],
            ]);
            self.tcp_write(CMD_POKE_32, address + offset as u64, value)?;
            offset += 4;
        }
        Ok(())
    }

    pub fn write_u8(&self, address: u64, value: u8) -> Result<()> {
        self.tcp_write(CMD_POKE_8, address, value as u32)
    }

    pub fn write_u16(&self, address: u64, value: u16) -> Result<()> {
        self.tcp_write(CMD_POKE_16, address, value as u32)
    }

    pub fn write_u32(&self, address: u64, value: u32) -> Result<()> {
        self.tcp_write(CMD_POKE_32, address, value)
    }

    pub fn write_utf16be(&self, address: u64, text: &str) -> Result<()> {
        let mut buffer = Vec::new();
        for c in text.encode_utf16() {
            buffer.extend(&c.to_be_bytes());
        }
        self.write_bytes(address, &buffer)
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
