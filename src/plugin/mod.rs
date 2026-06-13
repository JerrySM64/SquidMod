pub mod ffi;
pub mod ui_def;

use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use wasmtime::{Engine, Instance, Linker, Module, Store};

#[derive(Debug, Clone)]
pub enum PluginEvent {
    ToggleChanged(u32, bool),
    TextChanged(u32, String),
    DropdownChanged(u32, u64),
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
}

pub struct PluginManager {
    pub plugins: Vec<WasmPlugin>,
    pub engine: Engine,
}

pub struct WasmPlugin {
    pub path: PathBuf,
    pub enabled: bool,
    pub metadata: PluginMetadata,
    store: Store<ffi::HostState>,
    instance: Instance,
    pub event_receiver: std::sync::mpsc::Receiver<PluginEvent>,
    pub event_sender: std::sync::mpsc::Sender<PluginEvent>,
}

pub fn plugin_dir() -> Result<PathBuf> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| anyhow!("Could not determine config directory"))?;
    let dir = base.config_dir().join("SquidMod").join("plugins");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn copy_plugin_to_dir(src: &Path) -> Result<PathBuf> {
    let dest_dir = plugin_dir()?;
    let filename = src
        .file_name()
        .ok_or_else(|| anyhow!("Source has no filename"))?;
    let dest = dest_dir.join(filename);
    std::fs::copy(src, &dest)?;
    Ok(dest)
}

pub fn list_plugin_paths() -> Result<Vec<PathBuf>> {
    let dir = plugin_dir()?;
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("smp") {
            paths.push(path);
        }
    }
    Ok(paths)
}

pub fn load_plugin(
    path: &Path,
    engine: &Engine,
    linker: &Linker<ffi::HostState>,
    host_state: ffi::HostState,
    event_receiver: std::sync::mpsc::Receiver<PluginEvent>,
) -> Result<WasmPlugin> {
    let bytes = std::fs::read(path)?;

    let module = Module::from_binary(engine, &bytes)?;
    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut metadata = PluginMetadata {
        name: id.clone(),
        version: String::new(),
        description: String::new(),
    };

    for payload in wasmparser::Parser::new(0).parse_all(&bytes) {
        if let Ok(wasmparser::Payload::CustomSection(section)) = payload {
            if section.name() == "plugin_metadata" {
                if let Ok(m) = serde_json::from_slice::<PluginMetadata>(section.data()) {
                    metadata = m;
                    break;
                }
            }
        }
    }

    let host_state_clone_event_sender = host_state.event_sender.clone();
    let mut store = Store::new(engine, host_state);
    let instance = linker.instantiate(&mut store, &module)?;

    let init_fn = instance.get_typed_func::<(), ()>(&mut store, "plugin_init")?;
    init_fn.call(&mut store, ())?;

    Ok(WasmPlugin {
        path: path.to_path_buf(),
        enabled: true,
        metadata,
        store,
        instance,
        event_receiver,
        event_sender: host_state_clone_event_sender,
    })
}

impl WasmPlugin {
    pub fn tick(&mut self) {
        let on_event = self
            .instance
            .get_typed_func::<(u32, u64), ()>(&mut self.store, "plugin_on_event")
            .ok();

        if let Some(f) = on_event {
            while let Ok(event) = self.event_receiver.try_recv() {
                match event {
                    PluginEvent::ToggleChanged(handle, val) => {
                        let _ = f.call(&mut self.store, (handle, val as u64));
                    }
                    PluginEvent::DropdownChanged(handle, val) => {
                        let _ = f.call(&mut self.store, (handle, val));
                    }
                    PluginEvent::TextChanged(handle, text) => {
                        self.write_scratch(&text);
                        let _ = f.call(&mut self.store, (handle, 0));
                    }
                }
            }
        }

        if let Ok(tick_fn) =
            self.instance.get_typed_func::<(), ()>(&mut self.store, "plugin_tick")
        {
            let _ = tick_fn.call(&mut self.store, ());
        }
    }

    /*
    pub fn cleanup(&mut self) {
        if let Ok(f) = self.instance.get_typed_func::<(), ()>(&mut self.store, "plugin_cleanup") {
            let _ = f.call(&mut self.store, ());
        }
    }
    */

    pub fn set_pmem(&mut self, pmem: Option<crate::ProcessMemory>) {
        self.store.data_mut().pmem = pmem;
    }

    fn write_scratch(&mut self, text: &str) {
        let bytes = text.as_bytes();

        let ptr = self
            .instance
            .get_typed_func::<(), u32>(&mut self.store, "sqm_scratch_ptr")
            .ok()
            .and_then(|f| f.call(&mut self.store, ()).ok())
            .unwrap_or(0) as usize;

        let cap = self
            .instance
            .get_typed_func::<(), u32>(&mut self.store, "sqm_scratch_len")
            .ok()
            .and_then(|f| f.call(&mut self.store, ()).ok())
            .unwrap_or(0) as usize;

        if let Some(wasmtime::Extern::Memory(mem)) =
            self.instance.get_export(&mut self.store, "memory")
        {
            let write_len = bytes.len().min(cap.saturating_sub(1));
            let data = mem.data_mut(&mut self.store);
            if ptr + write_len < data.len() {
                data[ptr..ptr + write_len].copy_from_slice(&bytes[..write_len]);
                data[ptr + write_len] = 0;
            }
        }
    }
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
            engine: Engine::default(),
        }
    }
}
