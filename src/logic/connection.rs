use crate::logic::config::ConnectionMode;
use crate::{PATTERN, ProcessMemory};
use anyhow::Result;

pub fn target_name(mode: ConnectionMode) -> &'static str {
    if mode == ConnectionMode::WiiU { "Wii U" } else { "Cemu" }
}

pub fn connect_memory(mode: ConnectionMode, wiiu_ip: &str) -> Result<ProcessMemory> {
    match mode {
        ConnectionMode::Cemu => {
            #[cfg(target_os = "macos")]
            {
                let pm = ProcessMemory::open(0, 0)?;
                let check = pm.read_bytes(0x10000000, 20)?;
                if !check.windows(3).any(|w| w == PATTERN) {
                    return Err(anyhow::anyhow!("Pattern not found"));
                }
                Ok(pm)
            }
            #[cfg(not(target_os = "macos"))]
            {
                let pid = crate::find_cemu_process()?;
                let regions = crate::parse_maps(pid)?;
                let region = crate::find_suitable_region(&regions)?;
                let base_address = region.start + 0xE000000 - 0x10000000;
                let pm = ProcessMemory::open(pid, base_address)?;
                let check = pm.read_bytes(0x10000000, 20)?;
                if !check.windows(3).any(|w| w == PATTERN) {
                    return Err(anyhow::anyhow!("Pattern not found"));
                }
                Ok(pm)
            }
        }
        ConnectionMode::WiiU => {
            let wm = crate::platforms::wiiu::WiiUMemory::connect(wiiu_ip)?;
            Ok(ProcessMemory::WiiU(wm))
        }
    }
}
