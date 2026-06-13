use gtk4 as gtk;
use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;
use gtk::gio;
use std::cell::RefCell;
use std::rc::Rc;
use crate::gui::AppState;
use crate::plugin;

pub fn show_plugin_manager_window(parent: Option<&gtk::Window>, state: &Rc<RefCell<AppState>>) {
    let window = adw::Window::builder()
        .title("Plugin Manager")
        .default_width(480)
        .default_height(400)
        .modal(true)
        .build();

    if let Some(p) = parent {
        window.set_transient_for(Some(p));
    }

    let toolbar_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    let title = adw::WindowTitle::new("Plugin Manager", "");
    header.set_title_widget(Some(&title));
    toolbar_view.add_top_bar(&header);

    let page = adw::PreferencesPage::new();
    let plugins_group = adw::PreferencesGroup::builder()
        .title("Installed Plugins")
        .build();
    page.add(&plugins_group);

    let add_btn = gtk::Button::builder()
        .label("Add Plugin…")
        .halign(gtk::Align::Center)
        .build();
    add_btn.add_css_class("suggested-action");
    add_btn.add_css_class("pill");

    let bottom_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    bottom_box.set_halign(gtk::Align::Center);
    bottom_box.set_margin_top(12);
    bottom_box.set_margin_bottom(12);
    bottom_box.append(&add_btn);
    toolbar_view.add_bottom_bar(&bottom_box);

    let row_store: Rc<RefCell<Vec<adw::ActionRow>>> = Rc::new(RefCell::new(Vec::new()));

    populate_plugins_group(&plugins_group, state, &window, &row_store);

    let state_clone = state.clone();
    let plugins_group_clone = plugins_group.clone();
    let window_clone = window.clone();
    let row_store_clone = row_store.clone();
    add_btn.connect_clicked(move |_| {
        let file_dialog = gtk::FileDialog::builder()
            .title("Select Plugin File")
            .modal(true)
            .build();

        let filter = gtk::FileFilter::new();
        filter.add_pattern("*.smp");
        filter.set_name(Some("SquidMod Plugin Files"));
        let filters = gio::ListStore::new::<gtk::FileFilter>();
        filters.append(&filter);
        file_dialog.set_filters(Some(&filters));

        let state_inner = state_clone.clone();
        let group_inner = plugins_group_clone.clone();
        let window_inner = window_clone.clone();
        let row_store_inner = row_store_clone.clone();

        file_dialog.open(
            Some(&window_clone.clone().upcast::<gtk::Window>()),
            gio::Cancellable::NONE,
            move |result| {
                if let Ok(file) = result
                    && let Some(src_path) = file.path()
                {
                    match plugin::copy_plugin_to_dir(&src_path) {
                        Ok(dest_path) => {
                            let engine = state_inner.borrow().engine_state.lock().unwrap().plugin_manager.engine.clone();
                            let mut linker = wasmtime::Linker::new(&engine);
                            if let Err(e) = crate::plugin::ffi::register_host_functions(&mut linker) {
                                show_error(&window_inner.clone().upcast(), &format!("Linker error: {e}"));
                                return;
                            }
                            let (tx, rx) = std::sync::mpsc::channel();
                            let ui_sender = state_inner.borrow().ui_sender.clone();
                            let host_state = crate::plugin::ffi::HostState::new(tx, ui_sender);
                            match plugin::load_plugin(&dest_path, &engine, &linker, host_state, rx) {
                                Ok(wp) => {
                                    state_inner.borrow().engine_state.lock().unwrap().plugin_manager.plugins.push(wp);
                                    clear_and_repopulate(&group_inner, &state_inner, &window_inner, &row_store_inner);
                                }
                                Err(e) => {
                                    show_error(&window_inner.clone().upcast(), &format!("Failed to load plugin: {e}"));
                                }
                            }
                        }
                        Err(e) => {
                            show_error(&window_inner.clone().upcast(), &format!("Failed to copy plugin: {e}"));
                        }
                    }
                }
            },
        );
    });

    toolbar_view.set_content(Some(&page));
    window.set_content(Some(&toolbar_view));
    window.present();
}

fn populate_plugins_group(
    group: &adw::PreferencesGroup,
    state: &Rc<RefCell<AppState>>,
    window: &adw::Window,
    row_store: &Rc<RefCell<Vec<adw::ActionRow>>>,
) {
    let count = state.borrow().engine_state.lock().unwrap().plugin_manager.plugins.len();
    for i in 0..count {
        let (enabled, metadata) = {
            let s = state.borrow();
            let p = &s.engine_state.lock().unwrap().plugin_manager.plugins[i];
            (p.enabled, p.metadata.clone())
        };

        let row = adw::ActionRow::builder().title(&metadata.name).build();
        if !metadata.version.is_empty() {
            row.set_subtitle(&format!("v{} - {}", metadata.version, metadata.description));
        }


        let enable_switch = gtk::Switch::new();
        enable_switch.set_valign(gtk::Align::Center);
        enable_switch.set_active(enabled);

        let state_sw = state.clone();
        let idx = i;
        enable_switch.connect_active_notify(move |sw| {
            let active = sw.is_active();
            if let Ok(s) = state_sw.try_borrow() {
                if let Ok(mut es) = s.engine_state.try_lock() {
                    if let Some(plugin) = es.plugin_manager.plugins.get_mut(idx) {
                        plugin.enabled = active;
                    }
                }
            }
            let state_clone = state_sw.clone();
            gtk::glib::MainContext::default().spawn_local(async move {
                if let Ok(mut s) = state_clone.try_borrow_mut() {
                    let tabs = std::mem::take(&mut s.tab_table);
                    let groups = std::mem::take(&mut s.group_table);

                    for (_, tab) in tabs {
                        if let Some(ref cs) = s.content_stack {
                            cs.remove(&tab.page);
                        }
                        if let Some(ref sl) = s.sidebar_list {
                            sl.remove(&tab.sidebar_row);
                        }
                    }
                    for (_, group) in groups {
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
                    if let Ok(mut es) = s.engine_state.lock() {
                        let pmem = es.pmem.clone();
                        let engine = es.plugin_manager.engine.clone();
                        let ui_sender = s.ui_sender.clone();
                        for p in es.plugin_manager.plugins.iter_mut() {
                            if p.enabled {
                                let mut linker = wasmtime::Linker::new(&engine);
                                if crate::plugin::ffi::register_host_functions(&mut linker).is_ok() {
                                    let (tx, rx) = std::sync::mpsc::channel();
                                    let host_state = crate::plugin::ffi::HostState::new(tx, ui_sender.clone());
                                    if let Ok(mut new_plugin) = plugin::load_plugin(&p.path, &engine, &linker, host_state, rx) {
                                        new_plugin.enabled = true;
                                        new_plugin.set_pmem(pmem.clone());
                                        *p = new_plugin;
                                    }
                                }
                            }
                        }
                    }
                }
            });
        });

        let remove_btn = gtk::Button::builder()
            .icon_name("edit-delete-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text("Remove plugin")
            .build();
        remove_btn.add_css_class("destructive-action");
        remove_btn.add_css_class("flat");

        let state_del = state.clone();
        let group_del = group.clone();
        let window_del = window.clone();
        let row_store_del = row_store.clone();
        let idx = i;
        remove_btn.connect_clicked(move |_| {
            let path_to_remove = {
                let s = state_del.borrow();
                let es = s.engine_state.lock().unwrap();
                let pm = &es.plugin_manager;
                if idx < pm.plugins.len() {
                    pm.plugins[idx].path.clone()
                } else {
                    return;
                }
            };

            if std::fs::remove_file(&path_to_remove).is_ok() {
                {
                    let s = state_del.borrow();
                    let mut es = s.engine_state.lock().unwrap();
                    es.plugin_manager.plugins.remove(idx);
                }
                clear_and_repopulate(&group_del, &state_del, &window_del, &row_store_del);
            }
        });

        row.add_suffix(&enable_switch);
        row.add_suffix(&remove_btn);
        group.add(&row);
        row_store.borrow_mut().push(row);
    }
}

fn clear_and_repopulate(
    group: &adw::PreferencesGroup,
    state: &Rc<RefCell<AppState>>,
    window: &adw::Window,
    row_store: &Rc<RefCell<Vec<adw::ActionRow>>>,
) {
    for row in row_store.borrow().iter() {
        group.remove(row);
    }
    row_store.borrow_mut().clear();
    populate_plugins_group(group, state, window, row_store);
}

fn show_error(window: &gtk::Window, message: &str) {
    let dialog = adw::AlertDialog::builder()
        .heading("Error")
        .body(message)
        .build();
    dialog.add_response("ok", "OK");
    dialog.set_default_response(Some("ok"));
    dialog.present(Some(window));
}
