use crate::ProcessMemory;
use crate::logic::config::{ConnectionMode, NetworkMode};
use crate::logic::matchinfo::MatchRecord;
use crate::logic::playerinfo::PlayerRecord;
use crate::logic::sceneinfo::SceneRecord;
use gtk4 as gtk;
use gtk4::glib;
use libadwaita as adw;
use std::collections::HashMap;

pub struct AppState {
    pub connection_mode: ConnectionMode,
    pub wiiu_ip: String,
    pub players: Vec<PlayerRecord>,
    pub session_id: Option<u32>,
    pub fetched_at: String,
    pub match_info_data: Option<MatchRecord>,
    pub match_info_auto_update: bool,
    pub scene_info_data: Option<SceneRecord>,
    pub scene_info_auto_update: bool,
    pub memory_viewer_address: u64,
    pub memory_viewer_auto_update: bool,
    pub content_stack: Option<gtk::Stack>,
    pub sidebar_list: Option<gtk::ListBox>,
    pub content_title: Option<adw::WindowTitle>,
    pub injected_groups: HashMap<String, glib::object::WeakRef<adw::PreferencesPage>>,
    pub engine_state: std::sync::Arc<std::sync::Mutex<EngineState>>,
    pub ui_sender: std::sync::mpsc::Sender<crate::plugin::ffi::UiCommand>,
    pub global_tx: std::sync::mpsc::Sender<crate::plugin::PluginEvent>,
    pub network_mode: NetworkMode,
    pub check_for_updates: bool,
    pub tab_table: HashMap<u32, TabEntry>,
    pub group_table: HashMap<u32, GroupEntry>,
}

pub struct EngineState {
    pub pmem: Option<ProcessMemory>,
    pub plugin_manager: crate::plugin::PluginManager,
}

pub struct TabEntry {
    pub page: adw::PreferencesPage,
    pub sidebar_row: gtk::ListBoxRow,
}

pub struct GroupEntry {
    pub group: adw::PreferencesGroup,
    pub is_injected: bool,
    pub host_tab_name: Option<String>,
}
