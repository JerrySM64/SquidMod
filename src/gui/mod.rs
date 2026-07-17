pub mod asm_util;
pub mod match_info;
pub mod memory_viewer;
pub mod player_info;
pub mod plugin_manager;
pub mod scene_info;
pub mod utilities;

use crate::logic::config::{AppConfig, load_config, save_config};
pub use crate::logic::config::{ConnectionMode, NetworkMode};
use crate::logic::matchinfo::MatchRecord;
use crate::logic::playerinfo::PlayerRecord;
use crate::logic::sceneinfo::SceneRecord;
use crate::logic::telemetry;
use crate::*;
use adw::prelude::*;
use anyhow::Result;
use gtk::gio;
use gtk::glib;
use gtk4 as gtk;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

pub fn run_gui() -> Result<()> {
    #[cfg(target_os = "macos")]
    let app_flags = gio::ApplicationFlags::HANDLES_OPEN;
    #[cfg(not(target_os = "macos"))]
    let app_flags = gio::ApplicationFlags::FLAGS_NONE;

    let app = adw::Application::builder()
        .application_id("dev.jerrysm64.squidmod")
        .flags(app_flags)
        .build();

    app.connect_activate(build_ui);
    app.run();
    Ok(())
}

fn build_ui(app: &adw::Application) {
    adw::StyleManager::default().set_color_scheme(adw::ColorScheme::ForceDark);

    let engine_state = std::sync::Arc::new(std::sync::Mutex::new(EngineState {
        pmem: None,
        plugin_manager: crate::plugin::PluginManager::new(),
    }));

    let (ui_tx, ui_rx) = std::sync::mpsc::channel();
    let (global_tx, global_rx) = std::sync::mpsc::channel();

    let cfg = load_config();
    let state = Rc::new(RefCell::new(AppState {
        connection_mode: cfg.connection_mode,
        wiiu_ip: String::new(),
        players: Vec::new(),
        session_id: None,
        fetched_at: String::from("-"),
        match_info_data: None,
        match_info_auto_update: false,
        scene_info_data: None,
        scene_info_auto_update: false,
        memory_viewer_address: 0x10000000,
        memory_viewer_auto_update: false,
        content_stack: None,
        sidebar_list: None,
        content_title: None,
        injected_groups: HashMap::new(),
        engine_state: engine_state.clone(),
        ui_sender: ui_tx,
        global_tx,
        network_mode: cfg.network_mode,
        tab_table: HashMap::new(),
        group_table: HashMap::new(),
    }));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("SquidMod")
        .default_width(900)
        .default_height(850)
        .width_request(900)
        .height_request(850)
        .build();

    let content_stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .vexpand(true)
        .build();

    let content_title = adw::WindowTitle::new("Utilities", "");

    let pages = [
        ("utilities", "Utilities", "preferences-other-symbolic"),
        ("memory_viewer", "Memory Viewer", "view-reveal-symbolic"),
        ("player_info", "Player Info", "avatar-default-symbolic"),
        ("match_info", "Match Info", "view-list-bullet-symbolic"),
        ("scene_info", "Scene Info", "tv-symbolic"),
    ];

    for (page_name, _title, _icon_name) in pages.iter() {
        let page: adw::PreferencesPage = match *page_name {
            "utilities" => utilities::create_utilities_page(&state),
            "player_info" => player_info::create_player_info_page(&state),
            "match_info" => match_info::create_match_info_page(&state),
            "scene_info" => scene_info::create_scene_info_page(&state),
            "memory_viewer" => memory_viewer::create_memory_viewer_page(&state),
            _ => adw::PreferencesPage::new(),
        };

        if *page_name == "utilities" {
            state.borrow_mut().injected_groups.insert(
                "utilities".to_string(),
                glib::object::ObjectExt::downgrade(&page),
            );
        }

        content_stack.add_named(&page, Some(page_name));
    }

    state.borrow_mut().content_stack = Some(content_stack.clone());
    state.borrow_mut().content_title = Some(content_title.clone());

    let (sidebar_content, connect_btn, status_label, sidebar_list, ip_entry) =
        create_sidebar(&state);

    ip_entry.set_visible(state.borrow().connection_mode == ConnectionMode::WiiU);

    state.borrow_mut().sidebar_list = Some(sidebar_list.clone());

    let content_stack_clone = content_stack.clone();
    let content_title_clone = content_title.clone();

    sidebar_list.connect_row_selected(move |_, row| {
        if let Some(row) = row {
            let name = row.widget_name();
            if !name.is_empty() {
                content_stack_clone.set_visible_child_name(&name);
                let title = pages
                    .iter()
                    .find(|(pn, _, _)| *pn == name.as_str())
                    .map(|(_, t, _)| *t)
                    .unwrap_or(name.as_str());
                content_title_clone.set_title(title);
            }
        }
    });

    sidebar_list.select_row(sidebar_list.row_at_index(0).as_ref());

    let sidebar_header = adw::HeaderBar::new();
    let sidebar_title = adw::WindowTitle::new("SquidMod", "");
    sidebar_header.set_title_widget(Some(&sidebar_title));

    let menu = gio::Menu::new();
    menu.append(Some("Settings"), Some("app.settings"));
    menu.append(Some("Plugin Manager"), Some("app.plugin_manager"));
    menu.append(Some("About SquidMod"), Some("app.about"));

    #[cfg(target_os = "macos")]
    {
        let app_section = gio::Menu::new();
        app_section.append(Some("Settings..."), Some("app.preferences"));
        app_section.append(Some("Plugin Manager"), Some("app.plugin_manager"));
        app_section.append(Some("About SquidMod"), Some("app.about"));

        let menubar = gio::Menu::new();
        menubar.append_submenu(Some("SquidMod"), &app_section);

        app.set_menubar(Some(&menubar));
    }

    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .menu_model(&menu)
        .build();
    sidebar_header.pack_end(&menu_button);

    let sidebar_scroll = gtk::ScrolledWindow::builder()
        .child(&sidebar_content)
        .vexpand(true)
        .build();

    let sidebar_toolbar = adw::ToolbarView::new();
    sidebar_toolbar.add_top_bar(&sidebar_header);
    sidebar_toolbar.set_content(Some(&sidebar_scroll));

    let content_header = adw::HeaderBar::new();
    content_header.set_title_widget(Some(&content_title));

    let content_toolbar = adw::ToolbarView::new();
    content_toolbar.add_top_bar(&content_header);
    content_toolbar.set_content(Some(&content_stack));

    let sidebar_page = adw::NavigationPage::builder()
        .title("SquidMod")
        .child(&sidebar_toolbar)
        .build();

    let content_page = adw::NavigationPage::builder()
        .title("Settings")
        .child(&content_toolbar)
        .build();

    let split_view = adw::NavigationSplitView::new();
    split_view.set_sidebar(Some(&sidebar_page));
    split_view.set_content(Some(&content_page));
    split_view.set_min_sidebar_width(260.0);
    split_view.set_max_sidebar_width(320.0);
    split_view.set_show_content(true);

    let state_clone = state.clone();
    let status_label_clone = status_label.clone();
    let connect_btn_clone = connect_btn.clone();
    let ip_entry_clone = ip_entry.clone();
    connect_btn.connect_clicked(move |_| {
        let mode = state_clone.borrow().connection_mode;

        let state_ref = state_clone.borrow();
        let mut es = state_ref.engine_state.lock().unwrap();
        if es.pmem.is_some() {
            es.pmem = None;
            drop(es);
            drop(state_ref);
            status_label_clone.set_label("Not connected");
            status_label_clone.remove_css_class("success");
            status_label_clone.add_css_class("error");
            let target_name = if mode == ConnectionMode::WiiU {
                "Wii U"
            } else {
                "Cemu"
            };
            connect_btn_clone.set_label(&format!("Connect to {}", target_name));
            connect_btn_clone.remove_css_class("destructive-action");
            connect_btn_clone.add_css_class("suggested-action");
            return;
        }
        drop(es);
        drop(state_ref);

        let res = (|| -> Result<ProcessMemory> {
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
                        let pid = find_cemu_process()?;
                        let regions = parse_maps(pid)?;
                        let region = find_suitable_region(&regions)?;
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
                    let wiiu_ip = ip_entry_clone.text().to_string();
                    state_clone.borrow_mut().wiiu_ip = wiiu_ip.clone();
                    let wm = crate::platforms::wiiu::WiiUMemory::connect(&wiiu_ip)?;
                    Ok(ProcessMemory::WiiU(wm))
                }
            }
        })();

        match res {
            Ok(pm) => {
                let state_ref2 = state_clone.borrow();
                let mut es = state_ref2.engine_state.lock().unwrap();
                es.pmem = Some(pm);
                drop(es);
                drop(state_ref2);
                let target_name = if mode == ConnectionMode::WiiU {
                    "Wii U"
                } else {
                    "Cemu"
                };
                status_label_clone.set_label(&format!("Connected to {}", target_name));
                status_label_clone.remove_css_class("error");
                status_label_clone.add_css_class("success");
                connect_btn_clone.set_label(&format!("Disconnect from {}", target_name));
                connect_btn_clone.remove_css_class("suggested-action");
                connect_btn_clone.add_css_class("destructive-action");
            }
            Err(e) => {
                status_label_clone.set_label(&e.to_string());
                status_label_clone.remove_css_class("success");
                status_label_clone.add_css_class("error");
            }
        }
    });

    setup_main_loop(&state, global_rx);

    let shared_config = Arc::new(Mutex::new(telemetry::SharedConfig {
        connection_mode: cfg.connection_mode,
        network_mode: cfg.network_mode,
    }));
    telemetry::spawn_telemetry_loop(engine_state.clone(), shared_config.clone());

    let state_clone = state.clone();
    let status_label_clone = status_label.clone();
    let connect_btn_clone = connect_btn.clone();
    glib::timeout_add_local(Duration::from_millis(1000), move || {
        let should_reset = {
            if let Ok(state_ref) = state_clone.try_borrow() {
                if let Ok(es) = state_ref.engine_state.try_lock() {
                    if let Some(ref pm) = es.pmem {
                        pm.read_u32(0x10000000).is_err()
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            }
        };

        if should_reset {
            if let Ok(state_ref) = state_clone.try_borrow() {
                if let Ok(mut es) = state_ref.engine_state.try_lock() {
                    es.pmem = None;
                    let target_name = if state_ref.connection_mode == ConnectionMode::WiiU {
                        "Wii U"
                    } else {
                        "Cemu"
                    };
                    status_label_clone.set_label("Not connected");
                    status_label_clone.remove_css_class("success");
                    status_label_clone.add_css_class("error");
                    connect_btn_clone.set_label(&format!("Connect to {}", target_name));
                    connect_btn_clone.remove_css_class("destructive-action");
                    connect_btn_clone.add_css_class("suggested-action");
                }
            }
        }
        glib::ControlFlow::Continue
    });

    if let Some(display) = gtk::gdk::Display::default() {
        let icon_theme = gtk::IconTheme::for_display(&display);
        let temp_dir = std::env::temp_dir().join("squidmod_runtime_assets");
        let icon_path = temp_dir.join("hicolor/256x256/apps");
        const ICON_BYTES: &[u8] =
            include_bytes!("../../assets/hicolor/256x256/apps/dev.jerrysm64.squidmod.png");
        if std::fs::create_dir_all(&icon_path).is_ok() {
            let file_path = icon_path.join("dev.jerrysm64.squidmod.png");
            if std::fs::write(&file_path, ICON_BYTES).is_ok()
                && let Some(path_str) = temp_dir.to_str()
            {
                icon_theme.add_search_path(path_str);
            }
        }
    }

    let about_action = gio::SimpleAction::new("about", None);
    let window_weak = window.downgrade();
    about_action.connect_activate(move |_, _| {
        if let Some(window) = window_weak.upgrade() {
            let about = adw::AboutDialog::builder()
                .application_name("SquidMod")
                .application_icon("dev.jerrysm64.squidmod")
                .developer_name("Jerry Starke")
                .version(env!("CARGO_PKG_VERSION"))
                .website("https://github.com/JerrySM64/SquidMod")
                .license_type(gtk::License::MitX11)
                .release_notes(
                    "<p>Fixed a bug where the Spacebar telemetry failed with a server error.</p>",
                )
                .build();
            about.add_link("Donate", "https://ko-fi.com/jerrysm64");
            about.present(Some(&window));
        }
    });
    app.add_action(&about_action);

    let settings_action = gio::SimpleAction::new("settings", None);
    let window_weak_settings = window.downgrade();
    let state_clone_settings = state.clone();
    let connect_btn_settings = connect_btn.clone();
    let status_label_settings = status_label.clone();
    let ip_entry_settings = ip_entry.clone();
    let shared_config_settings = shared_config.clone();
    settings_action.connect_activate(move |_, _| {
        if let Some(window) = window_weak_settings.upgrade() {
            show_settings_window(
                Some(&window.upcast()),
                &state_clone_settings,
                &connect_btn_settings,
                &status_label_settings,
                &ip_entry_settings,
                &shared_config_settings,
            );
        }
    });
    app.add_action(&settings_action);

    let prefs_action = gio::SimpleAction::new("preferences", None);
    let window_weak_prefs = window.downgrade();
    let state_clone_prefs = state.clone();
    let connect_btn_prefs = connect_btn.clone();
    let status_label_prefs = status_label.clone();
    let ip_entry_prefs = ip_entry.clone();
    let shared_config_prefs = shared_config.clone();
    prefs_action.connect_activate(move |_, _| {
        if let Some(window) = window_weak_prefs.upgrade() {
            show_settings_window(
                Some(&window.upcast()),
                &state_clone_prefs,
                &connect_btn_prefs,
                &status_label_prefs,
                &ip_entry_prefs,
                &shared_config_prefs,
            );
        }
    });
    app.add_action(&prefs_action);

    let quit_action = gio::SimpleAction::new("quit", None);
    let app_weak = glib::object::ObjectExt::downgrade(app);
    quit_action.connect_activate(move |_, _| {
        if let Some(app) = app_weak.upgrade() {
            app.quit();
        }
    });
    app.add_action(&quit_action);

    let modifier = if cfg!(target_os = "macos") {
        "<Meta>"
    } else {
        "<Primary>"
    };
    app.set_accels_for_action("app.preferences", &[&format!("{modifier}comma")]);
    app.set_accels_for_action("app.quit", &[&format!("{modifier}q")]);

    let plugin_action = gio::SimpleAction::new("plugin_manager", None);
    let window_weak2 = window.downgrade();
    let state_clone = state.clone();
    plugin_action.connect_activate(move |_, _| {
        if let Some(window) = window_weak2.upgrade() {
            plugin_manager::show_plugin_manager_window(Some(&window.upcast()), &state_clone);
        }
    });
    app.add_action(&plugin_action);

    if let Ok(paths) = crate::plugin::list_plugin_paths() {
        for path in paths {
            let engine = {
                state
                    .borrow()
                    .engine_state
                    .lock()
                    .unwrap()
                    .plugin_manager
                    .engine
                    .clone()
            };
            let mut linker = wasmtime::Linker::new(&engine);
            let ui_sender = state.borrow().ui_sender.clone();
            if crate::plugin::ffi::register_host_functions(&mut linker).is_ok() {
                let (tx, rx) = std::sync::mpsc::channel();
                let host_state = crate::plugin::ffi::HostState::new(tx, ui_sender);
                if let Ok(wp) = crate::plugin::load_plugin(&path, &engine, &linker, host_state, rx)
                {
                    state
                        .borrow()
                        .engine_state
                        .lock()
                        .unwrap()
                        .plugin_manager
                        .plugins
                        .push(wp);
                }
            }
        }
    }

    window.set_content(Some(&split_view));
    window.set_icon_name(Some("dev.jerrysm64.squidmod"));
    window.present();

    setup_ui_event_loop(&state, ui_rx);
}

fn create_sidebar(
    state: &Rc<RefCell<AppState>>,
) -> (gtk::Box, gtk::Button, gtk::Label, gtk::ListBox, gtk::Entry) {
    let container = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let status_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    status_box.set_margin_top(12);
    status_box.set_margin_bottom(12);
    status_box.set_margin_start(12);
    status_box.set_margin_end(12);

    let status_label = gtk::Label::builder()
        .label("Not connected")
        .halign(gtk::Align::Center)
        .wrap(true)
        .justify(gtk::Justification::Center)
        .css_classes(vec!["error"])
        .build();
    status_box.append(&status_label);

    let ip_entry = gtk::Entry::builder()
        .placeholder_text("Wii U IP Address")
        .build();
    ip_entry.connect_changed(|entry| {
        let current = entry.text().to_string();
        let filtered: String = current
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect();
        let clamped = filtered
            .split('.')
            .map(|seg| {
                if seg.is_empty() {
                    String::new()
                } else {
                    seg.parse::<u32>()
                        .map(|n| n.min(255).to_string())
                        .unwrap_or_else(|_| String::from("255"))
                }
            })
            .collect::<Vec<_>>()
            .join(".");
        if clamped != current {
            entry.set_text(&clamped);
            entry.set_position(-1);
        }
    });
    status_box.append(&ip_entry);

    let btn_label = if state.borrow().connection_mode == ConnectionMode::WiiU {
        "Connect to Wii U"
    } else {
        "Connect to Cemu"
    };
    let connect_btn = gtk::Button::with_label(btn_label);
    connect_btn.add_css_class("pill");
    connect_btn.add_css_class("suggested-action");
    status_box.append(&connect_btn);

    container.append(&status_box);
    container.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let sidebar_list = gtk::ListBox::new();
    sidebar_list.set_selection_mode(gtk::SelectionMode::Single);
    sidebar_list.add_css_class("navigation-sidebar");

    let categories = vec![
        ("utilities", "Utilities", "preferences-other-symbolic"),
        ("memory_viewer", "Memory Viewer", "view-reveal-symbolic"),
        ("player_info", "Player Info", "avatar-default-symbolic"),
        ("match_info", "Match Info", "view-list-bullet-symbolic"),
        ("scene_info", "Scene Info", "tv-symbolic"),
    ];

    for (page_name, title, icon_name) in &categories {
        let row = adw::ActionRow::builder().title(*title).build();
        row.set_widget_name(page_name);
        let icon = gtk::Image::from_icon_name(icon_name);
        row.add_prefix(&icon);
        sidebar_list.append(&row);
    }

    container.append(&sidebar_list);

    (container, connect_btn, status_label, sidebar_list, ip_entry)
}

fn setup_main_loop(
    state: &Rc<RefCell<AppState>>,
    global_rx: std::sync::mpsc::Receiver<crate::plugin::PluginEvent>,
) {
    let engine_state = state.borrow().engine_state.clone();

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

fn setup_ui_event_loop(
    state: &Rc<RefCell<AppState>>,
    ui_rx: std::sync::mpsc::Receiver<crate::plugin::ffi::UiCommand>,
) {
    let state_clone = state.clone();

    glib::timeout_add_local(Duration::from_millis(16), move || {
        use crate::plugin::ffi::UiCommand;
        while let Ok(cmd) = ui_rx.try_recv() {
            match cmd {
                UiCommand::RegisterTab { handle, name, icon } => {
                    let page = adw::PreferencesPage::builder()
                        .title(&name)
                        .icon_name(&icon)
                        .build();

                    let row = adw::ActionRow::builder().title(&name).build();
                    let img = gtk::Image::from_icon_name(&icon);
                    row.add_prefix(&img);

                    let mut s = state_clone.borrow_mut();

                    if let Some(ref cs) = s.content_stack {
                        cs.add_named(&page, Some(&name));
                    }
                    if let Some(ref sl) = s.sidebar_list {
                        row.set_widget_name(&name);
                        sl.append(&row);

                        let cs_weak = s.content_stack.as_ref().map(|cs| cs.downgrade());
                        let ct_weak = s.content_title.as_ref().map(|ct| ct.downgrade());
                        let pn = name.clone();
                        let listbox_row = sl
                            .row_at_index(sl.observe_children().n_items() as i32 - 1)
                            .and_then(|r| r.downcast::<gtk::ListBoxRow>().ok());
                        if let Some(lbrow) = listbox_row {
                            lbrow.connect_activate(move |_| {
                                if let Some(cs) = cs_weak.as_ref().and_then(|w| w.upgrade()) {
                                    cs.set_visible_child_name(&pn);
                                }
                                if let Some(ct) = ct_weak.as_ref().and_then(|w| w.upgrade()) {
                                    ct.set_title(&pn);
                                }
                            });
                        }
                    }

                    s.tab_table.insert(
                        handle,
                        TabEntry {
                            page,
                            sidebar_row: row.upcast(),
                        },
                    );
                }
                UiCommand::TabAddGroup {
                    handle,
                    tab_handle,
                    title,
                } => {
                    let group = adw::PreferencesGroup::builder().title(&title).build();
                    let mut s = state_clone.borrow_mut();
                    if let Some(tab) = s.tab_table.get(&tab_handle) {
                        tab.page.add(&group);
                    }
                    s.group_table.insert(
                        handle,
                        GroupEntry {
                            group,
                            is_injected: false,
                            host_tab_name: None,
                        },
                    );
                }
                UiCommand::InjectGroupIntoTab {
                    handle,
                    host_tab,
                    title,
                } => {
                    let group = adw::PreferencesGroup::builder().title(&title).build();
                    let mut s = state_clone.borrow_mut();

                    if let Some(page_weak) = s.injected_groups.get(&host_tab) {
                        if let Some(page) = page_weak.upgrade() {
                            page.add(&group);
                        }
                    }

                    s.group_table.insert(
                        handle,
                        GroupEntry {
                            group,
                            is_injected: true,
                            host_tab_name: Some(host_tab),
                        },
                    );
                }
                UiCommand::GroupAddSwitch {
                    handle,
                    group_handle,
                    title,
                    subtitle,
                } => {
                    let switch = gtk::Switch::new();
                    switch.set_valign(gtk::Align::Center);

                    let row = adw::ActionRow::builder()
                        .title(&title)
                        .subtitle(&subtitle)
                        .build();
                    row.add_suffix(&switch);
                    row.set_activatable_widget(Some(&switch));

                    let s = state_clone.borrow();
                    if let Some(ge) = s.group_table.get(&group_handle) {
                        ge.group.add(&row);
                    }

                    let tx = s.global_tx.clone();
                    switch.connect_active_notify(move |sw| {
                        let _ = tx.send(crate::plugin::PluginEvent::ToggleChanged(
                            handle,
                            sw.is_active(),
                        ));
                    });
                }
                UiCommand::GroupAddRow {
                    handles,
                    group_handle,
                    json,
                } => {
                    let row_def: crate::plugin::ui_def::RowDef = match serde_json::from_str(&json) {
                        Ok(rd) => rd,
                        Err(_) => continue,
                    };

                    let row = adw::ActionRow::builder().title(&row_def.title).build();
                    if let Some(sub) = &row_def.subtitle {
                        row.set_subtitle(sub);
                    }

                    let mut widgets = Vec::new();
                    let global_tx = state_clone.borrow().global_tx.clone();

                    for (i, widget_def) in row_def.widgets.into_iter().enumerate() {
                        if i >= handles.len() {
                            break;
                        }
                        let handle = handles[i];
                        let tx = global_tx.clone();

                        match widget_def {
                            crate::plugin::ui_def::WidgetDef::Switch {
                                title: _,
                                subtitle: _,
                            } => {
                                let switch = gtk::Switch::new();
                                switch.set_valign(gtk::Align::Center);
                                switch.connect_active_notify(move |sw| {
                                    let _ = tx.send(crate::plugin::PluginEvent::ToggleChanged(
                                        handle,
                                        sw.is_active(),
                                    ));
                                });
                                widgets.push(switch.upcast::<gtk::Widget>());
                            }
                            crate::plugin::ui_def::WidgetDef::Entry {
                                title: _,
                                placeholder,
                                max_chars,
                            } => {
                                let entry = gtk::Entry::builder()
                                    .placeholder_text(&placeholder)
                                    .max_width_chars(max_chars as i32)
                                    .width_chars(max_chars as i32)
                                    .max_length(max_chars as i32)
                                    .valign(gtk::Align::Center)
                                    .build();
                                entry.connect_changed(move |e| {
                                    let _ = tx.send(crate::plugin::PluginEvent::TextChanged(
                                        handle,
                                        e.text().to_string(),
                                    ));
                                });
                                widgets.push(entry.upcast::<gtk::Widget>());
                            }
                            crate::plugin::ui_def::WidgetDef::Dropdown {
                                title: _,
                                subtitle: _,
                                items: items_raw,
                            } => {
                                let items: Vec<&str> = items_raw.split('\0').collect();
                                let dropdown = gtk::DropDown::from_strings(&items);
                                dropdown.set_valign(gtk::Align::Center);
                                dropdown.connect_selected_notify(move |dd| {
                                    let _ = tx.send(crate::plugin::PluginEvent::DropdownChanged(
                                        handle,
                                        dd.selected() as u64,
                                    ));
                                });
                                widgets.push(dropdown.upcast::<gtk::Widget>());
                            }
                        }
                    }

                    if widgets.len() == 1 {
                        row.add_suffix(&widgets[0]);
                        if widgets[0].is::<gtk::Switch>() {
                            row.set_activatable_widget(Some(&widgets[0]));
                        }
                    } else if widgets.len() > 1 {
                        let suffix_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                        suffix_box.set_valign(gtk::Align::Center);
                        for w in widgets {
                            suffix_box.append(&w);
                        }
                        row.add_suffix(&suffix_box);
                    }

                    let s = state_clone.borrow();
                    if let Some(ge) = s.group_table.get(&group_handle) {
                        ge.group.add(&row);
                    }
                }
                UiCommand::UnregisterAll => {
                    let mut s = state_clone.borrow_mut();
                    for (_handle, tab) in &s.tab_table {
                        if let Some(ref cs) = s.content_stack {
                            cs.remove(&tab.page);
                        }
                        if let Some(ref sl) = s.sidebar_list {
                            sl.remove(&tab.sidebar_row);
                        }
                    }
                    for (_handle, group) in &s.group_table {
                        if group.is_injected {
                            if let Some(ref host_tab_name) = group.host_tab_name {
                                if let Some(page_weak) = s.injected_groups.get(host_tab_name) {
                                    if let Some(page) = page_weak.upgrade() {
                                        page.remove(&group.group);
                                    }
                                }
                            }
                        }
                    }
                    s.tab_table.clear();
                    s.group_table.clear();
                }
            }
        }

        glib::ControlFlow::Continue
    });
}

fn show_settings_window(
    parent: Option<&gtk::Window>,
    state: &Rc<RefCell<AppState>>,
    connect_btn: &gtk::Button,
    status_label: &gtk::Label,
    ip_entry: &gtk::Entry,
    shared_config: &Arc<Mutex<telemetry::SharedConfig>>,
) {
    let window = adw::PreferencesDialog::builder().title("Settings").build();

    let page = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::builder().title("Connection").build();

    let action_row = adw::ActionRow::builder()
        .title("Target Platform")
        .subtitle("Select the platform you want to connect to")
        .build();

    let box_container = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    box_container.add_css_class("linked");
    box_container.set_valign(gtk::Align::Center);

    let cemu_btn = gtk::ToggleButton::builder().label("Cemu").build();
    let wiiu_btn = gtk::ToggleButton::builder().label("Wii U").build();
    wiiu_btn.set_group(Some(&cemu_btn));

    box_container.append(&cemu_btn);
    box_container.append(&wiiu_btn);
    action_row.add_suffix(&box_container);

    let initial_mode = state.borrow().connection_mode;
    if initial_mode == ConnectionMode::WiiU {
        wiiu_btn.set_active(true);
    } else {
        cemu_btn.set_active(true);
    }

    let state_clone = state.clone();
    let connect_btn_clone = connect_btn.clone();
    let status_label_clone = status_label.clone();
    let sidebar_ip_entry_clone = ip_entry.clone();
    let shared_config_clone = shared_config.clone();

    wiiu_btn.connect_toggled(move |btn| {
        let is_wiiu = btn.is_active();
        sidebar_ip_entry_clone.set_visible(is_wiiu);

        let mut sf = state_clone.borrow_mut();
        sf.connection_mode = if is_wiiu {
            ConnectionMode::WiiU
        } else {
            ConnectionMode::Cemu
        };
        save_config(&AppConfig {
            connection_mode: sf.connection_mode,
            network_mode: sf.network_mode,
        });

        if let Ok(mut sc) = shared_config_clone.lock() {
            sc.connection_mode = sf.connection_mode;
        }

        let target_name = if is_wiiu { "Wii U" } else { "Cemu" };
        let mut es = sf.engine_state.lock().unwrap();
        if es.pmem.is_some() {
            es.pmem = None;
            connect_btn_clone.remove_css_class("destructive-action");
            connect_btn_clone.add_css_class("suggested-action");
        }
        drop(es);
        status_label_clone.set_label("Not connected");
        status_label_clone.remove_css_class("success");
        status_label_clone.add_css_class("error");
        connect_btn_clone.set_label(&format!("Connect to {}", target_name));
    });

    group.add(&action_row);

    let network_group = adw::PreferencesGroup::builder().title("Network").build();

    let network_row = adw::ActionRow::builder()
        .title("PID Lookup Network")
        .subtitle("Select the network used to resolve PIDs")
        .build();

    let net_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    net_box.add_css_class("linked");
    net_box.set_valign(gtk::Align::Center);

    let pretendo_btn = gtk::ToggleButton::builder().label("Pretendo").build();
    let spacebar_btn = gtk::ToggleButton::builder().label("Spacebar").build();
    spacebar_btn.set_group(Some(&pretendo_btn));

    net_box.append(&pretendo_btn);
    net_box.append(&spacebar_btn);
    network_row.add_suffix(&net_box);

    let initial_network = state.borrow().network_mode;
    if initial_network == NetworkMode::Spacebar {
        spacebar_btn.set_active(true);
    } else {
        pretendo_btn.set_active(true);
    }

    let state_clone_net = state.clone();
    let shared_config_clone_net = shared_config.clone();
    spacebar_btn.connect_toggled(move |btn| {
        let mut sf = state_clone_net.borrow_mut();
        sf.network_mode = if btn.is_active() {
            NetworkMode::Spacebar
        } else {
            NetworkMode::Pretendo
        };
        save_config(&AppConfig {
            connection_mode: sf.connection_mode,
            network_mode: sf.network_mode,
        });

        if let Ok(mut sc) = shared_config_clone_net.lock() {
            sc.network_mode = sf.network_mode;
        }
    });

    network_group.add(&network_row);
    page.add(&group);
    page.add(&network_group);
    window.add(&page);

    window.present(parent);
}
