use gtk4 as gtk;
use libadwaita as adw;
use adw::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;
use gtk::glib;
use crate::gui::AppState;
use crate::logic::matchinfo::fetch_match_info;

pub fn create_match_info_page(state: &Rc<RefCell<AppState>>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title("Match Info")
        .icon_name("view-list-bullet-symbolic")
        .build();

    let group = adw::PreferencesGroup::builder()
        .title("Match Information")
        .build();

    let hour_row = adw::ActionRow::builder().title("Match Hour").build();
    let hour_label = gtk::Label::new(Some("---"));
    hour_row.add_suffix(&hour_label);

    let match_id_row = adw::ActionRow::builder().title("Match ID").build();
    let match_id_label = gtk::Label::new(Some("---"));
    match_id_row.add_suffix(&match_id_label);

    let gamemode_row = adw::ActionRow::builder().title("Gamemode").build();
    let gamemode_label = gtk::Label::new(Some("---"));
    gamemode_row.add_suffix(&gamemode_label);

    let map_row = adw::ActionRow::builder().title("Map Name").build();
    let map_label = gtk::Label::new(Some("---"));
    map_row.add_suffix(&map_label);

    group.add(&hour_row);
    group.add(&match_id_row);
    group.add(&gamemode_row);
    group.add(&map_row);
    page.add(&group);

    
    let refresh_btn = gtk::Button::with_label("Refresh");
    refresh_btn.set_valign(gtk::Align::Center);
    refresh_btn.add_css_class("suggested-action");
    
    let state_clone = state.clone();
    let hl = hour_label.clone();
    let mil = match_id_label.clone();
    let gl = gamemode_label.clone();
    let ml = map_label.clone();
    
    refresh_btn.connect_clicked(move |_| {
        let mut state_ref = state_clone.borrow_mut();
        let fetch_res = {
            let es = state_ref.engine_state.lock().unwrap();
            if let Some(ref pm) = es.pmem {
                fetch_match_info(pm).ok()
            } else { None }
        };
        if let Some(res) = fetch_res {
            hl.set_text(&format!("{} ({})", res.hour, res.hour_label));
            mil.set_text(&format!("{} ({})", res.match_id, res.match_label));
            gl.set_text(&format!("{} ({})", res.gamemode_id, res.gamemode_label));
            ml.set_text(&res.map_name);
            state_ref.match_info_data = Some(res);
        }
    });

    let auto_row = adw::ActionRow::builder()
        .title("Auto Update")
        .build();
    let auto_check = gtk::CheckButton::new();
    auto_check.set_valign(gtk::Align::Center);
    let state_clone = state.clone();
    auto_check.connect_toggled(move |cb| {
        state_clone.borrow_mut().match_info_auto_update = cb.is_active();
    });
    auto_row.add_suffix(&auto_check);
    
    let actions_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    actions_box.append(&refresh_btn);
    actions_box.append(&auto_row);
    
    let actions_pref_group = adw::PreferencesGroup::new();
    actions_pref_group.add(&actions_box);
    page.add(&actions_pref_group);

    let state_clone = state.clone();
    glib::timeout_add_local(Duration::from_millis(1000), move || {
        let mut state_ref = state_clone.borrow_mut();
        if state_ref.match_info_auto_update {
            let fetch_res = {
                let es = state_ref.engine_state.lock().unwrap();
                if let Some(ref pm) = es.pmem {
                    fetch_match_info(pm).ok()
                } else { None }
            };
            if let Some(res) = fetch_res {
                hour_label.set_text(&format!("{} ({})", res.hour, res.hour_label));
                match_id_label.set_text(&format!("{} ({})", res.match_id, res.match_label));
                gamemode_label.set_text(&format!("{} ({})", res.gamemode_id, res.gamemode_label));
                map_label.set_text(&res.map_name);
                state_ref.match_info_data = Some(res);
            }
        }
        glib::ControlFlow::Continue
    });

    page
}
