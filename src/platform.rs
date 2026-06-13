use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(target_os = "linux")] {
        use crate::platforms::linux as platform_impl;
    } else if #[cfg(target_family = "windows")] {
        use crate::platforms::windows as platform_impl;
    } else if #[cfg(target_os = "macos")] {
        use crate::platforms::macos as platform_impl;
    } else {
        compile_error!("Unsupported target family");
    }
}


pub use platform_impl::{MemoryRegion, find_cemu_process, parse_maps, find_suitable_region, TARGET_NAMES};

use crate::platforms::wiiu::WiiUMemory;

#[derive(Clone)]
pub enum ProcessMemory {
    Native(platform_impl::ProcessMemory),
    WiiU(WiiUMemory),
}

impl ProcessMemory {
    pub fn open(pid: i32, base_address: u64) -> anyhow::Result<Self> {
        platform_impl::ProcessMemory::open(pid, base_address).map(Self::Native)
    }

    pub fn read_bytes(&self, address: u64, length: usize) -> anyhow::Result<Vec<u8>> {
        match self {
            Self::Native(pm) => pm.read_bytes(address, length),
            Self::WiiU(wm) => wm.read_bytes(address, length),
        }
    }

    pub fn read_u32(&self, address: u64) -> anyhow::Result<u32> {
        match self {
            Self::Native(pm) => pm.read_u32(address),
            Self::WiiU(wm) => wm.read_u32(address),
        }
    }

    pub fn write_bytes(&self, address: u64, data: &[u8]) -> anyhow::Result<()> {
        match self {
            Self::Native(pm) => pm.write_bytes(address, data),
            Self::WiiU(wm) => wm.write_bytes(address, data),
        }
    }

    pub fn write_u8(&self, address: u64, value: u8) -> anyhow::Result<()> {
        match self {
            Self::Native(pm) => pm.write_u8(address, value),
            Self::WiiU(wm) => wm.write_u8(address, value),
        }
    }

    pub fn write_u16(&self, address: u64, value: u16) -> anyhow::Result<()> {
        match self {
            Self::Native(pm) => pm.write_u16(address, value),
            Self::WiiU(wm) => wm.write_u16(address, value),
        }
    }

    pub fn write_u32(&self, address: u64, value: u32) -> anyhow::Result<()> {
        match self {
            Self::Native(pm) => pm.write_u32(address, value),
            Self::WiiU(wm) => wm.write_u32(address, value),
        }
    }

    pub fn write_utf16be(&self, address: u64, text: &str) -> anyhow::Result<()> {
        match self {
            Self::Native(pm) => pm.write_utf16be(address, text),
            Self::WiiU(wm) => wm.write_utf16be(address, text),
        }
    }

    pub fn read_utf16be(&self, address: u64, max_length: usize) -> anyhow::Result<String> {
        match self {
            Self::Native(pm) => pm.read_utf16be(address, max_length),
            Self::WiiU(wm) => wm.read_utf16be(address, max_length),
        }
    }

    pub fn read_pointer_chain(&self, offsets: &[u64]) -> anyhow::Result<u64> {
        match self {
            Self::Native(pm) => pm.read_pointer_chain(offsets),
            Self::WiiU(wm) => wm.read_pointer_chain(offsets),
        }
    }
}
