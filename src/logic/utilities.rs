use crate::gui::EngineState;
use std::sync::{Arc, Mutex};

pub fn write_name_to_memory(engine_state: &Arc<Mutex<EngineState>>, name: &str) {
    let es_clone = engine_state.clone();
    let name_string: String = name.chars().take(16).collect();

    std::thread::spawn(move || {
        if let Ok(es) = es_clone.lock() {
            if let Some(ref pm) = es.pmem {
                let ptr = pm.read_u32(0x101E80A4).unwrap_or(0) as u64;
                if ptr != 0 {
                    let addr = ptr + 0x8C;
                    let _ = pm.write_bytes(addr, &[0u8; 32]);
                    if !name_string.is_empty() {
                        let _ = pm.write_utf16be(addr, &name_string);
                    }
                }
            }
        }
    });
}
