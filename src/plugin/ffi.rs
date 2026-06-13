use anyhow::Result;
use wasmtime::{Caller, Linker};
use crate::ProcessMemory;

pub enum UiCommand {
    RegisterTab {
        handle: u32,
        name: String,
        icon: String,
    },
    TabAddGroup {
        handle: u32,
        tab_handle: u32,
        title: String,
    },
    InjectGroupIntoTab {
        handle: u32,
        host_tab: String,
        title: String,
    },
    GroupAddSwitch {
        handle: u32,
        group_handle: u32,
        title: String,
        subtitle: String,
    },
    GroupAddRow {
        handles: Vec<u32>,
        group_handle: u32,
        json: String,
    },
    UnregisterAll,
}

pub struct HostState {
    pub pmem: Option<ProcessMemory>,
    pub next_handle: u32,
    pub event_sender: std::sync::mpsc::Sender<crate::plugin::PluginEvent>,
    pub ui_sender: std::sync::mpsc::Sender<UiCommand>,
}

impl HostState {
    pub fn new(event_sender: std::sync::mpsc::Sender<crate::plugin::PluginEvent>, ui_sender: std::sync::mpsc::Sender<UiCommand>) -> Self {
        Self {
            pmem: None,
            next_handle: 1,
            event_sender,
            ui_sender,
        }
    }

    pub fn alloc_handle(&mut self) -> u32 {
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }
}

fn read_guest_str(caller: &mut Caller<'_, HostState>, ptr: u32, len: u32) -> String {
    let mem = match caller.get_export("memory") {
        Some(wasmtime::Extern::Memory(m)) => m,
        _ => return String::new(),
    };
    let data = mem.data(caller);
    let start = ptr as usize;
    let end = start + len as usize;
    if end > data.len() {
        return String::new();
    }
    String::from_utf8_lossy(&data[start..end]).into_owned()
}

fn write_guest_scratch(caller: &mut Caller<'_, HostState>, s: &str) {
    let mut sqm_ptr = None;
    if let Some(wasmtime::Extern::Func(f)) = caller.get_export("sqm_scratch_ptr") {
        if let Ok(tf) = f.typed::<(), u32>(&*caller) {
            sqm_ptr = tf.call(&mut *caller, ()).ok();
        }
    }

    let mut sqm_len = None;
    if let Some(wasmtime::Extern::Func(f)) = caller.get_export("sqm_scratch_len") {
        if let Ok(tf) = f.typed::<(), u32>(&*caller) {
            sqm_len = tf.call(&mut *caller, ()).ok();
        }
    }

    if let (Some(ptr), Some(cap)) = (sqm_ptr, sqm_len) {
        let mem = match caller.get_export("memory") {
            Some(wasmtime::Extern::Memory(m)) => m,
            _ => return,
        };
        let bytes = s.as_bytes();
        let write_len = bytes.len().min(cap as usize - 1);
        let data = mem.data_mut(caller);
        if (ptr as usize) + write_len < data.len() {
            data[ptr as usize..(ptr as usize + write_len)].copy_from_slice(&bytes[..write_len]);
            data[ptr as usize + write_len] = 0;
        }
    }
}

pub fn register_host_functions(linker: &mut Linker<HostState>) -> Result<()> {
    linker.func_wrap(
        "env",
        "sqm_read_u32",
        |caller: Caller<'_, HostState>, addr: u32| -> u32 {
            match &caller.data().pmem {
                Some(pm) => pm.read_u32(addr as u64).unwrap_or(0),
                None => 0,
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "sqm_read_u16",
        |caller: Caller<'_, HostState>, addr: u32| -> u32 {
            let data = caller.data();
            let pm = match &data.pmem {
                Some(pm) => pm,
                None => return 0,
            };
            let bytes = pm.read_bytes(addr as u64, 2).unwrap_or_default();
            if bytes.len() < 2 { return 0; }
            u16::from_be_bytes([bytes[0], bytes[1]]) as u32
        },
    )?;

    linker.func_wrap(
        "env",
        "sqm_write_u32",
        |caller: Caller<'_, HostState>, addr: u32, val: u32| -> i32 {
            match &caller.data().pmem {
                Some(pm) => if pm.write_u32(addr as u64, val).is_ok() { 0 } else { -1 },
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "sqm_write_u16",
        |caller: Caller<'_, HostState>, addr: u32, val: u32| -> i32 {
            match &caller.data().pmem {
                Some(pm) => if pm.write_u16(addr as u64, val as u16).is_ok() { 0 } else { -1 },
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "sqm_write_u8",
        |caller: Caller<'_, HostState>, addr: u32, val: u32| -> i32 {
            match &caller.data().pmem {
                Some(pm) => if pm.write_u8(addr as u64, val as u8).is_ok() { 0 } else { -1 },
                None => -1,
            }
        },
    )?;

    linker.func_wrap(
        "env",
        "sqm_write_bytes",
        |mut caller: Caller<'_, HostState>, addr: u32, buf_ptr: u32, buf_len: u32| -> i32 {
            let pm = match caller.data().pmem.clone() {
                Some(pm) => pm,
                None => return -1,
            };
            let mem = match caller.get_export("memory") {
                Some(wasmtime::Extern::Memory(m)) => m,
                _ => return -1,
            };
            let data = mem.data(&caller);
            let start = buf_ptr as usize;
            let end = start + buf_len as usize;
            if end > data.len() { return -1; }
            let bytes = data[start..end].to_vec();
            if pm.write_bytes(addr as u64, &bytes).is_ok() { 0 } else { -1 }
        },
    )?;

    linker.func_wrap(
        "env",
        "sqm_write_utf16be",
        |mut caller: Caller<'_, HostState>, addr: u32, str_ptr: u32, str_len: u32| -> i32 {
            let pm = match caller.data().pmem.clone() {
                Some(pm) => pm,
                None => return -1,
            };
            let s = read_guest_str(&mut caller, str_ptr, str_len);
            if pm.write_utf16be(addr as u64, &s).is_ok() { 0 } else { -1 }
        },
    )?;

    linker.func_wrap(
        "env",
        "sqm_deref_u32",
        |caller: Caller<'_, HostState>, addr: u32| -> u32 {
            match &caller.data().pmem {
                Some(pm) => pm.read_u32(addr as u64).unwrap_or(0),
                None => 0,
            }
        },
    )?;

    register_ui_functions(linker)?;
    Ok(())
}

fn register_ui_functions(linker: &mut Linker<HostState>) -> Result<()> {
    linker.func_wrap(
        "env",
        "sqm_ui_register_tab",
        |mut caller: Caller<'_, HostState>,
         name_ptr: u32,
         name_len: u32,
         icon_ptr: u32,
         icon_len: u32|
         -> u32 {
            let name = read_guest_str(&mut caller, name_ptr, name_len);
            let icon = read_guest_str(&mut caller, icon_ptr, icon_len);

            let handle = caller.data_mut().alloc_handle();
            let _ = caller.data().ui_sender.send(UiCommand::RegisterTab {
                handle,
                name,
                icon,
            });

            handle
        },
    )?;

    linker.func_wrap(
        "env",
        "sqm_ui_tab_add_group",
        |mut caller: Caller<'_, HostState>,
         tab_handle: u32,
         title_ptr: u32,
         title_len: u32|
         -> u32 {
            let title = read_guest_str(&mut caller, title_ptr, title_len);
            let handle = caller.data_mut().alloc_handle();

            let _ = caller.data().ui_sender.send(UiCommand::TabAddGroup {
                handle,
                tab_handle,
                title,
            });

            handle
        },
    )?;

    linker.func_wrap(
        "env",
        "sqm_ui_inject_group_into_tab",
        |mut caller: Caller<'_, HostState>,
         host_tab_ptr: u32,
         host_tab_len: u32,
         title_ptr: u32,
         title_len: u32|
         -> u32 {
            let host_tab = read_guest_str(&mut caller, host_tab_ptr, host_tab_len);
            let title = read_guest_str(&mut caller, title_ptr, title_len);

            let handle = caller.data_mut().alloc_handle();

            let _ = caller.data().ui_sender.send(UiCommand::InjectGroupIntoTab {
                handle,
                host_tab,
                title,
            });

            handle
        },
    )?;

    linker.func_wrap(
        "env",
        "sqm_ui_group_add_switch",
        |mut caller: Caller<'_, HostState>,
         group_handle: u32,
         title_ptr: u32,
         title_len: u32,
         sub_ptr: u32,
         sub_len: u32|
         -> u32 {
            let title = read_guest_str(&mut caller, title_ptr, title_len);
            let subtitle = read_guest_str(&mut caller, sub_ptr, sub_len);

            let handle = caller.data_mut().alloc_handle();

            let _ = caller.data().ui_sender.send(UiCommand::GroupAddSwitch {
                handle,
                group_handle,
                title,
                subtitle,
            });

            handle
        },
    )?;

    linker.func_wrap(
        "env",
        "sqm_ui_group_add_row",
        |mut caller: Caller<'_, HostState>,
         group_handle: u32,
         json_ptr: u32,
         json_len: u32|
         -> u32 {
            let json = read_guest_str(&mut caller, json_ptr, json_len);
            let row_def: crate::plugin::ui_def::RowDef = match serde_json::from_str(&json) {
                Ok(rd) => rd,
                Err(_) => return 0,
            };

            let mut handles = Vec::new();

            for _ in row_def.widgets {
                let handle = caller.data_mut().alloc_handle();
                handles.push(handle);
            }

            let _ = caller.data().ui_sender.send(UiCommand::GroupAddRow {
                handles: handles.clone(),
                group_handle,
                json,
            });

            let handles_json = serde_json::to_string(&handles).unwrap_or_else(|_| "[]".to_string());
            write_guest_scratch(&mut caller, &handles_json);

            1
        },
    )?;

    linker.func_wrap(
        "env",
        "sqm_ui_unregister_all",
        |caller: Caller<'_, HostState>| {
            let _ = caller.data().ui_sender.send(UiCommand::UnregisterAll);
        },
    )?;

    Ok(())
}
