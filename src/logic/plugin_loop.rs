use crate::logic::state::EngineState;
use crate::plugin::ffi::UiCommand;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub fn spawn_plugin_loop(
    engine_state: Arc<Mutex<EngineState>>,
    global_rx: Receiver<crate::plugin::PluginEvent>,
) {
    std::thread::spawn(move || {
        loop {
            let mut pending = Vec::new();
            while let Ok(evt) = global_rx.try_recv() {
                pending.push(evt);
            }

            if let Ok(mut es) = engine_state.try_lock() {
                let pm_opt = es.pmem.clone();
                for plugin in &mut es.plugin_manager.plugins {
                    if plugin.enabled {
                        plugin.set_pmem(pm_opt.clone());
                        for evt in &pending {
                            let _ = plugin.event_sender.send(evt.clone());
                        }
                        if pm_opt.is_some() {
                            plugin.tick();
                        }
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    });
}

pub fn load_startup_plugins(
    engine_state: &Arc<Mutex<EngineState>>,
    ui_sender: &Sender<UiCommand>,
) {
    if let Ok(paths) = crate::plugin::list_plugin_paths() {
        for path in paths {
            let engine = { engine_state.lock().unwrap().plugin_manager.engine.clone() };
            let mut linker = wasmtime::Linker::new(&engine);
            if crate::plugin::ffi::register_host_functions(&mut linker).is_ok() {
                let (tx, rx) = std::sync::mpsc::channel();
                let host_state = crate::plugin::ffi::HostState::new(tx, ui_sender.clone());
                if let Ok(wp) = crate::plugin::load_plugin(&path, &engine, &linker, host_state, rx)
                {
                    engine_state
                        .lock()
                        .unwrap()
                        .plugin_manager
                        .plugins
                        .push(wp);
                }
            }
        }
    }
}
