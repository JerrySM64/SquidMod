#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
mod gui;
mod platforms;
mod platform;
mod id;
mod logic;
mod plugin;

pub const PATTERN: [u8; 3] = [0x02, 0xD4, 0xE7];
pub const MIN_REGION_SIZE: u64 = 1308622848;

pub use platform::{MemoryRegion, ProcessMemory, find_cemu_process, parse_maps, find_suitable_region, TARGET_NAMES};

fn main() -> Result<()> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    if std::env::args().any(|a| a == "--root-daemon") {
        if let Err(e) = platforms::daemon::run_server() {
            eprintln!("[daemon] Fatal error: {e}");
        }
        std::process::exit(0);
    }

    #[cfg(target_os = "macos")]
    if let Ok(mut path) = std::env::current_exe() {
        path.pop();
        path.pop();
        path.push("Resources");
        path.push("share");

        let bundle_share = path.display().to_string();
        let new_val = match std::env::var("XDG_DATA_DIRS") {
            Ok(existing) if !existing.is_empty() => format!("{bundle_share}:{existing}"),
            _ => bundle_share,
        };
        unsafe { std::env::set_var("XDG_DATA_DIRS", new_val); }
    }

    gui::run_gui()
}
