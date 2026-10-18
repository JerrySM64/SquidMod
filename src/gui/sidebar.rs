use crate::logic::config::ConnectionMode;
use crate::logic::state::AppState;
use adw::prelude::*;
use gtk4 as gtk;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;

pub fn create_sidebar(
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
