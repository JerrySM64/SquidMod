use crate::logic::state::{AppState, GroupEntry, TabEntry};
use adw::prelude::*;
use gtk4 as gtk;
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub fn setup_ui_event_loop(
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
