use gtk4 as gtk;
use libadwaita as adw;
use adw::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;
use gtk::glib;
use crate::gui::AppState;
use crate::logic::sceneinfo::{fetch_scene_info, get_scene_name};

pub fn create_scene_info_page(state: &Rc<RefCell<AppState>>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title("Scene Info")
        .icon_name("map-location-symbolic")
        .build();
    
    let group = adw::PreferencesGroup::builder()
        .title("Scene Information")
        .build();
    
    let current_scene_row = adw::ActionRow::builder()
        .title("Current Scene ID")
        .subtitle("-")
        .build();
    let current_scene_copy = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .valign(gtk::Align::Center)
        .build();
    current_scene_copy.add_css_class("flat");
    current_scene_row.add_suffix(&current_scene_copy);
    group.add(&current_scene_row);
    
    let scene_info_row = adw::ActionRow::builder()
        .title("Scene Info ID")
        .subtitle("-")
        .build();
    let scene_info_copy = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .valign(gtk::Align::Center)
        .build();
    scene_info_copy.add_css_class("flat");
    scene_info_row.add_suffix(&scene_info_copy);
    group.add(&scene_info_row);

    let last_scene_row = adw::ActionRow::builder()
        .title("Last Scene ID")
        .subtitle("-")
        .build();
    let last_scene_copy = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .valign(gtk::Align::Center)
        .build();
    last_scene_copy.add_css_class("flat");
    last_scene_row.add_suffix(&last_scene_copy);
    group.add(&last_scene_row);

    let next_scene_row = adw::ActionRow::builder()
        .title("Next Scene ID")
        .subtitle("-")
        .build();
    let next_scene_copy = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .valign(gtk::Align::Center)
        .build();
    next_scene_copy.add_css_class("flat");
    next_scene_row.add_suffix(&next_scene_copy);
    group.add(&next_scene_row);
    
    let scene_mode_row = adw::ActionRow::builder()
        .title("Scene Mode")
        .subtitle("-")
        .build();
    let scene_mode_copy = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .valign(gtk::Align::Center)
        .build();
    scene_mode_copy.add_css_class("flat");
    scene_mode_row.add_suffix(&scene_mode_copy);
    group.add(&scene_mode_row);
    
    page.add(&group);

    let refresh_btn = gtk::Button::with_label("Refresh");
    refresh_btn.set_valign(gtk::Align::Center);
    refresh_btn.add_css_class("suggested-action");
    
    let state_clone = state.clone();
    let c_row = current_scene_row.clone();
    let si_row = scene_info_row.clone();
    let l_row = last_scene_row.clone();
    let n_row = next_scene_row.clone();
    let m_row = scene_mode_row.clone();
    
    refresh_btn.connect_clicked(move |_| {
        let mut state_ref = state_clone.borrow_mut();
        
        let info_res = {
            let es = state_ref.engine_state.lock().unwrap();
            if let Some(pm) = &es.pmem {
                Some(fetch_scene_info(pm))
            } else {
                None
            }
        };

        if let Some(res) = info_res {
             match res {
                Ok(info) => {
                    let c_name = get_scene_name(info.current_scene_id);
                    let si_name = get_scene_name(info.scene_info_id);
                    let l_name = get_scene_name(info.last_scene_id);
                    let n_name = get_scene_name(info.next_scene_id);

                    c_row.set_subtitle(&format!("{} ({})", c_name, info.current_scene_id));
                    si_row.set_subtitle(&format!("{} ({})", si_name, info.scene_info_id));
                    l_row.set_subtitle(&format!("{} ({})", l_name, info.last_scene_id));
                    n_row.set_subtitle(&format!("{} ({})", n_name, info.next_scene_id));
                    m_row.set_subtitle(&format!("{} ({})", info.current_mode, info.current_mode_id));
                    
                    state_ref.scene_info_data = Some(info);
                },
                Err(_) => {
                    c_row.set_subtitle("Error reading memory");
                }
             }
        } else {
             c_row.set_subtitle("Not connected");
        }
    });

    let auto_row = adw::ActionRow::builder()
        .title("Auto Update")
        .build();
    let auto_check = gtk::CheckButton::new();
    auto_check.set_valign(gtk::Align::Center);
    let state_clone = state.clone();
    auto_check.connect_toggled(move |cb| {
        state_clone.borrow_mut().scene_info_auto_update = cb.is_active();
    });
    auto_row.add_suffix(&auto_check);

    let actions_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    actions_box.append(&refresh_btn);
    actions_box.append(&auto_row);
    
    let actions_pref_group = adw::PreferencesGroup::new();
    actions_pref_group.add(&actions_box);
    page.add(&actions_pref_group);
    
    let c_row_clone = current_scene_row.clone();
    current_scene_copy.connect_clicked(move |_| {
        if let Some(display) = gtk::gdk::Display::default() {
            let clipboard = display.clipboard();
            if let Some(text) = c_row_clone.subtitle() {
                clipboard.set_text(&text);
            }
        }
    });

    let si_row_clone = scene_info_row.clone();
    scene_info_copy.connect_clicked(move |_| {
        if let Some(display) = gtk::gdk::Display::default() {
            let clipboard = display.clipboard();
            if let Some(text) = si_row_clone.subtitle() {
                clipboard.set_text(&text);
            }
        }
    });

    let l_row_clone = last_scene_row.clone();
    last_scene_copy.connect_clicked(move |_| {
        if let Some(display) = gtk::gdk::Display::default() {
            let clipboard = display.clipboard();
            if let Some(text) = l_row_clone.subtitle() {
                clipboard.set_text(&text);
            }
        }
    });

    let n_row_clone = next_scene_row.clone();
    next_scene_copy.connect_clicked(move |_| {
        if let Some(display) = gtk::gdk::Display::default() {
            let clipboard = display.clipboard();
            if let Some(text) = n_row_clone.subtitle() {
                clipboard.set_text(&text);
            }
        }
    });
    
    let m_row_clone = scene_mode_row.clone();
    scene_mode_copy.connect_clicked(move |_| {
        if let Some(display) = gtk::gdk::Display::default() {
            let clipboard = display.clipboard();
            if let Some(text) = m_row_clone.subtitle() {
                clipboard.set_text(&text);
            }
        }
    });

    let state_clone = state.clone();
    let c_row = current_scene_row.clone();
    let si_row = scene_info_row.clone();
    let l_row = last_scene_row.clone();
    let n_row = next_scene_row.clone();
    let m_row = scene_mode_row.clone();
    glib::timeout_add_local(Duration::from_millis(1000), move || {
        let mut state_ref = state_clone.borrow_mut();
        if state_ref.scene_info_auto_update {
             let info_opt = {
                 let es = state_ref.engine_state.lock().unwrap();
                 if let Some(ref pm) = es.pmem {
                     fetch_scene_info(pm).ok()
                 } else { None }
             };
                 
             if let Some(info) = info_opt {
                 let c_name = get_scene_name(info.current_scene_id);
                 let si_name = get_scene_name(info.scene_info_id);
                 let l_name = get_scene_name(info.last_scene_id);
                 let n_name = get_scene_name(info.next_scene_id);

                 c_row.set_subtitle(&format!("{} ({})", c_name, info.current_scene_id));
                 si_row.set_subtitle(&format!("{} ({})", si_name, info.scene_info_id));
                 l_row.set_subtitle(&format!("{} ({})", l_name, info.last_scene_id));
                 n_row.set_subtitle(&format!("{} ({})", n_name, info.next_scene_id));
                 m_row.set_subtitle(&format!("{} ({})", info.current_mode, info.current_mode_id));
                 
                 state_ref.scene_info_data = Some(info);
             }
        }
        glib::ControlFlow::Continue
    });

    page
}
