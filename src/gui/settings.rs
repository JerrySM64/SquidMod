use crate::logic::config::{ConnectionMode, NetworkMode};
use crate::logic::settings::persist_settings;
use crate::logic::state::AppState;
use crate::logic::telemetry;
use adw::prelude::*;
use gtk4 as gtk;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub fn show_settings_window(
    parent: Option<&gtk::Window>,
    state: &Rc<RefCell<AppState>>,
    connect_btn: &gtk::Button,
    status_label: &gtk::Label,
    ip_entry: &gtk::Entry,
    shared_config: &Arc<Mutex<telemetry::SharedConfig>>,
) {
    let window = adw::PreferencesDialog::builder().title("Settings").build();

    let page = adw::PreferencesPage::new();
    let group = adw::PreferencesGroup::builder()
        .title("Connection")
        .description("Wii U Connection is experimental. Expect crashes!")
        .build();

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
        persist_settings(sf.connection_mode, sf.network_mode, sf.check_for_updates);

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

    let network_row = adw::ActionRow::builder()
        .title("PID Lookup Network")
        .subtitle("Select the network used to resolve PIDs")
        .build();

    let net_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    net_box.add_css_class("linked");
    net_box.set_valign(gtk::Align::Center);

    let pretendo_btn = gtk::ToggleButton::builder().label("Pretendo").build();
    let altendo_btn = gtk::ToggleButton::builder().label("Altendo").build();
    altendo_btn.set_group(Some(&pretendo_btn));

    net_box.append(&pretendo_btn);
    net_box.append(&altendo_btn);
    network_row.add_suffix(&net_box);

    let initial_network = state.borrow().network_mode;
    if initial_network == NetworkMode::Altendo {
        altendo_btn.set_active(true);
    } else {
        pretendo_btn.set_active(true);
    }

    let state_clone_net = state.clone();
    let shared_config_clone_net = shared_config.clone();
    altendo_btn.connect_toggled(move |btn| {
        let mut sf = state_clone_net.borrow_mut();
        sf.network_mode = if btn.is_active() {
            NetworkMode::Altendo
        } else {
            NetworkMode::Pretendo
        };
        persist_settings(sf.connection_mode, sf.network_mode, sf.check_for_updates);

        if let Ok(mut sc) = shared_config_clone_net.lock() {
            sc.network_mode = sf.network_mode;
        }
    });

    group.add(&network_row);

    let updates_row = adw::ActionRow::builder()
        .title("Check for updates")
        .subtitle("Notify when a new release is available")
        .build();

    let updates_switch = gtk::Switch::builder()
        .valign(gtk::Align::Center)
        .active(state.borrow().check_for_updates)
        .build();

    let state_clone_updates = state.clone();
    updates_switch.connect_active_notify(move |sw| {
        let mut sf = state_clone_updates.borrow_mut();
        sf.check_for_updates = sw.is_active();
        persist_settings(sf.connection_mode, sf.network_mode, sf.check_for_updates);
    });

    updates_row.add_suffix(&updates_switch);
    updates_row.set_activatable_widget(Some(&updates_switch));

    let updates_group = adw::PreferencesGroup::new();

    updates_group.add(&updates_row);

    page.add(&updates_group);
    page.add(&group);
    window.add(&page);

    window.present(parent);
}
