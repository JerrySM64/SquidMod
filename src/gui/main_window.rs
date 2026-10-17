use crate::gui::{
    match_info, memory_viewer, player_info, plugin_manager, scene_info, settings, sidebar,
    ui_commands, utilities,
};
use crate::logic::config::{ConnectionMode, load_config};
use crate::logic::connection;
use crate::logic::plugin_loop;
use crate::logic::state::{AppState, EngineState};
use crate::logic::telemetry;
use adw::prelude::*;
use gtk4 as gtk;
use gtk4::gio;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(crate) fn build_ui(app: &adw::Application) {
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
        ui_sender: ui_tx.clone(),
        global_tx,
        network_mode: cfg.network_mode,
        check_for_updates: cfg.check_for_updates,
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
        sidebar::create_sidebar(&state);

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
            let target_name = connection::target_name(mode);
            connect_btn_clone.set_label(&format!("Connect to {}", target_name));
            connect_btn_clone.remove_css_class("destructive-action");
            connect_btn_clone.add_css_class("suggested-action");
            return;
        }
        drop(es);
        drop(state_ref);

        let wiiu_ip = ip_entry_clone.text().to_string();
        if mode == ConnectionMode::WiiU {
            state_clone.borrow_mut().wiiu_ip = wiiu_ip.clone();
        }
        let res = connection::connect_memory(mode, &wiiu_ip);

        match res {
            Ok(pm) => {
                let state_ref2 = state_clone.borrow();
                let mut es = state_ref2.engine_state.lock().unwrap();
                es.pmem = Some(pm);
                drop(es);
                drop(state_ref2);
                let target_name = connection::target_name(mode);
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

    plugin_loop::spawn_plugin_loop(engine_state.clone(), global_rx);

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
                    let target_name = connection::target_name(state_ref.connection_mode);
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
            settings::show_settings_window(
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
            settings::show_settings_window(
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

    plugin_loop::load_startup_plugins(&engine_state, &ui_tx);

    window.set_content(Some(&split_view));
    window.set_icon_name(Some("dev.jerrysm64.squidmod"));
    window.present();

    let check_updates_enabled = state.borrow().check_for_updates;
    if check_updates_enabled {
        std::thread::spawn(move || {
            if let Some(status) = crate::logic::update::check_for_updates() {
                if status.update_available {
                    glib::idle_add(move || {
                        let dialog = adw::AlertDialog::builder()
                            .heading("Update Available")
                            .body(format!(
                                "SquidMod {} is available (running {})",
                                status.latest_tag, status.current_version
                            ))
                            .build();
                        dialog.add_response("cancel", "Not now");
                        dialog.add_response("update", "Update");
                        dialog.set_response_appearance("update", adw::ResponseAppearance::Suggested);
                        dialog.connect_response(None, move |_, response| {
                            if response == "update" {
                                let _ = gtk::gio::AppInfo::launch_default_for_uri(
                                    "https://github.com/JerrySM64/SquidMod/releases",
                                    None::<&gio::AppLaunchContext>,
                                );
                            }
                        });
                        dialog.present(None::<&gtk::Window>);
                        glib::ControlFlow::Break
                    });
                }
            }
        });
    }

    ui_commands::setup_ui_event_loop(&state, ui_rx);
}
