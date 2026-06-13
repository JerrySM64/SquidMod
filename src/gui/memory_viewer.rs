use crate::gui::AppState;
use gtk4::prelude::*;
use gtk4 as gtk;
use libadwaita as adw;
use adw::prelude::*;
use gtk::glib;
use gtk::gio;
use gtk::gdk;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub fn create_memory_viewer_page(state: &Rc<RefCell<AppState>>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title("Memory Viewer")
        .icon_name("view-reveal-symbolic")
        .build();

    if let Some(display) = gdk::Display::default() {
        let provider = gtk::CssProvider::new();
        provider.load_from_string("
            .selected-cell {
                background-color: #3584e4;
                color: white;
                border-radius: 3px;
            }
        ");
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // --- MEMORY VIEWER ---
    let mem_group = adw::PreferencesGroup::builder()
        .title("Memory Viewer")
        .build();

    let controls_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    controls_box.set_halign(gtk::Align::Fill);
    controls_box.set_margin_bottom(12);

    let update_btn = gtk::Button::builder().label("Update").build();
    let dump_btn = gtk::Button::builder().label("Dump RAM").build();
    let auto_label = gtk::Label::builder().label("Auto Update").build();
    let auto_check = gtk::CheckButton::new();
    
    let addr_prefix = gtk::Label::builder().label("0x").build();

    let addr_entry = gtk::Entry::builder()
        .placeholder_text("Address (Hex)")
        .hexpand(true)
        .build();
    
    controls_box.append(&addr_prefix);
    controls_box.append(&addr_entry);
    controls_box.append(&dump_btn);
    controls_box.append(&update_btn);
    controls_box.append(&auto_label);
    controls_box.append(&auto_check);

    mem_group.add(&controls_box);

    let grid_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    grid_box.add_css_class("card");
    grid_box.set_hexpand(true);
    
    let header_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    header_box.set_margin_top(6);
    header_box.set_margin_bottom(6);
    header_box.set_margin_start(12);
    header_box.set_margin_end(12);
    header_box.set_hexpand(true);
    
    let addr_header = gtk::Label::builder().label("Address").width_chars(10).xalign(0.0).hexpand(true).build();
    addr_header.add_css_class("heading");
    header_box.append(&addr_header);
    
    for offset in &[0, 4, 8, 0xC] {
        let lbl = gtk::Label::builder().label(format!("{:X}", offset)).width_chars(8).xalign(0.5).hexpand(true).build();
        lbl.add_css_class("heading");
        header_box.append(&lbl);
    }
    grid_box.append(&header_box);
    grid_box.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let selected_cell: Rc<RefCell<Option<gtk::Label>>> = Rc::new(RefCell::new(None));
    let highlighted_address: Rc<RefCell<Option<u64>>> = Rc::new(RefCell::new(None));
    let context_data: Rc<RefCell<Option<(u64, String)>>> = Rc::new(RefCell::new(None));

    let grid_click = gtk::GestureClick::new();
    let selected_cell_bg = selected_cell.clone();
    let highlighted_address_bg = highlighted_address.clone();
    grid_click.connect_pressed(move |_, _, _, _| {
        if let Some(prev) = selected_cell_bg.borrow_mut().take() {
            prev.remove_css_class("selected-cell");
        }
        *highlighted_address_bg.borrow_mut() = None;
    });
    grid_box.add_controller(grid_click);

    let mut row_labels: Vec<(gtk::Label, Vec<gtk::Label>)> = Vec::new();

    let context_menu = gtk::Popover::new();
    let copy_btn = gtk::Button::builder()
        .label("Copy")
        .has_frame(false)
        .build();
    context_menu.set_child(Some(&copy_btn));

    let context_data_clone = context_data.clone();
    let context_menu_clone_copy = context_menu.clone();
    copy_btn.connect_clicked(move |_| {
        if let Some((addr, value)) = &*context_data_clone.borrow() {
            let text = format!("0x{:08X}: {}", addr, value);
            if let Some(display) = gdk::Display::default() {
                display.clipboard().set_text(&text);
            }
        }
        context_menu_clone_copy.popdown();
    });

    for i in 0..16 {
        let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row_box.set_margin_top(2);
        row_box.set_margin_bottom(2);
        row_box.set_margin_start(12);
        row_box.set_margin_end(12);
        row_box.set_hexpand(true);

        let addr_lbl = gtk::Label::builder().label("00000000").width_chars(10).xalign(0.0).hexpand(true).build();
        addr_lbl.add_css_class("monospace");
        row_box.append(&addr_lbl);

        let mut data_lbls = Vec::new();
        for j in 0..4 {
            let d_lbl = gtk::Label::builder().label("00000000").width_chars(8).xalign(0.5).hexpand(true).build();
            d_lbl.add_css_class("monospace");

            let click = gtk::GestureClick::new();
            click.set_button(0);

            let state_clone = state.clone();
            let selected_cell_clone = selected_cell.clone();
            let highlighted_address_clone = highlighted_address.clone();
            let context_menu_clone = context_menu.clone();
            let context_data_clone = context_data.clone();
            let d_lbl_clone = d_lbl.clone();

            click.connect_pressed(move |gesture, _, _, _| {
                let button = gesture.current_button();
                let mut sel = selected_cell_clone.borrow_mut();

                if let Some(prev) = sel.take() {
                    prev.remove_css_class("selected-cell");
                }
                d_lbl_clone.add_css_class("selected-cell");
                *sel = Some(d_lbl_clone.clone());

                let addr = state_clone.borrow().memory_viewer_address + (i as u64 * 16) + (j as u64 * 4);
                *highlighted_address_clone.borrow_mut() = Some(addr);

                if button == 3 {
                    let value = d_lbl_clone.text().to_string();
                    *context_data_clone.borrow_mut() = Some((addr, value));

                    context_menu_clone.unparent();
                    context_menu_clone.set_parent(&d_lbl_clone);
                    context_menu_clone.popup();
                }
                gesture.set_state(gtk::EventSequenceState::Claimed);
            });
            d_lbl.add_controller(click);

            row_box.append(&d_lbl);
            data_lbls.push(d_lbl);
        }
        grid_box.append(&row_box);
        row_labels.push((addr_lbl, data_lbls));
    }
    mem_group.add(&grid_box);
    page.add(&mem_group);


    // --- POINTER VIEWER ---
    
    let ptr_group = adw::PreferencesGroup::builder()
        .title("Pointer Viewer")
        .build();

    let mode_row = adw::ActionRow::builder()
        .title("Address Type")
        .subtitle("Select input address type")
        .build();
    let mode_dropdown = gtk::DropDown::from_strings(&["Wii U", "Cemu"]);
    mode_dropdown.set_valign(gtk::Align::Center);
    mode_row.add_suffix(&mode_dropdown);
    ptr_group.add(&mode_row);

    let input_row = adw::ActionRow::builder()
        .title("Pointer Chain")
        .subtitle("e.g., [0x101E6770] + 0x15C")
        .build();
    
    let input_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    input_box.set_valign(gtk::Align::Center);

    let input_entry = gtk::Entry::builder()
        .placeholder_text("[0x101E6770] + 0x15C")
        .width_chars(24)
        .build();
    
    let resolve_btn = gtk::Button::builder()
        .label("Go")
        .css_classes(vec!["suggested-action".to_string()])
        .build();
    
    input_box.append(&input_entry);
    input_box.append(&resolve_btn);
    
    input_row.add_suffix(&input_box);
    ptr_group.add(&input_row);

    let addr_row = adw::ActionRow::builder()
        .title("Resolved Address")
        .build();
    let addr_label = gtk::Label::new(Some("-"));
    addr_label.set_selectable(true);
    addr_row.add_suffix(&addr_label);
    ptr_group.add(&addr_row);

    let val_row = adw::ActionRow::builder()
        .title("Value")
        .build();
    let val_label = gtk::Label::new(Some("-"));
    val_label.set_selectable(true);
    val_row.add_suffix(&val_label);
    ptr_group.add(&val_row);

    page.add(&ptr_group);

    // --- LOGIC ---

    let state_clone = state.clone();
    let entry_clone = input_entry.clone();
    let dd_clone = mode_dropdown.clone();
    let addr_label_clone = addr_label.clone();
    let val_label_clone = val_label.clone();
    let highlighted_address_resolve = highlighted_address.clone();
    
    resolve_btn.connect_clicked(move |_| {
        let input = entry_clone.text().to_string();
        let is_wiiu = dd_clone.selected() == 0;
        let mut state_ref = state_clone.borrow_mut();
        
        let pm_opt = {
            let es = state_ref.engine_state.lock().unwrap();
            es.pmem.clone()
        };
        
        if let Some(pm) = pm_opt {
            let clean = input.replace('[', "").replace(']', "");
            let parts: Vec<&str> = clean.split('+').map(|s| s.trim()).collect();
            
            if parts.is_empty() || parts[0].is_empty() {
                addr_label_clone.set_label("Empty pointer chain");
                val_label_clone.set_label("-");
                return;
            }

            let mut chain = Vec::new();
            for part in parts {
                let trimmed = part.trim_start_matches("0x").trim_start_matches("0X");
                if let Ok(val) = u64::from_str_radix(trimmed, 16) {
                    chain.push(val);
                } else {
                    addr_label_clone.set_label("Invalid offset format");
                    val_label_clone.set_label("-");
                    return;
                }
            }

            if is_wiiu && chain[0] >= 0x503000 {
                chain[0] -= 0x503000;
            }

            let final_addr_result = if chain.len() == 1 {
                Ok(chain[0])
            } else {
                pm.read_pointer_chain(&chain)
            };

            match final_addr_result {
                Ok(current_addr) => {
                    match pm.read_u32(current_addr) {
                         Ok(val) => {
                             addr_label_clone.set_label(&format!("0x{:X}", current_addr));
                             val_label_clone.set_label(&format!("0x{:X}", val));
                             *highlighted_address_resolve.borrow_mut() = Some(current_addr);
                             state_ref.memory_viewer_address = current_addr & !0xF;
                         },
                         Err(_) => {
                             addr_label_clone.set_label(&format!("0x{:X}", current_addr));
                             val_label_clone.set_label("???");
                             *highlighted_address_resolve.borrow_mut() = Some(current_addr);
                             state_ref.memory_viewer_address = current_addr & !0xF;
                         }
                    }
                },
                Err(e) => {
                    // e.g. "Null pointer in chain"
                    addr_label_clone.set_label(&format!("{}", e));
                    val_label_clone.set_label("-");
                }
            }
            
        } else {
            addr_label_clone.set_label("Not connected");
            val_label_clone.set_label("-");
        }
    });

    let rows_rc = Rc::new(row_labels);
    let addr_entry_rc = addr_entry.clone(); 
    
    let update_view = {
        let state_clone = state.clone();
        let rows = rows_rc.clone();
        let addr_entry = addr_entry_rc.clone();
        let selected_cell = selected_cell.clone();
        let highlighted_address_view = highlighted_address.clone();

        Rc::new(move || {
            let state_ref = state_clone.borrow();
            let hl_addr = *highlighted_address_view.borrow();
            let es = state_ref.engine_state.lock().unwrap();
            if let Some(pm) = &es.pmem {
                let start_addr = state_ref.memory_viewer_address;
                addr_entry.set_text(&format!("{:08X}", start_addr));
                
                let rows_vec: &Vec<(gtk::Label, Vec<gtk::Label>)> = rows.as_ref();

                match pm.read_bytes(start_addr, 256) {
                    Ok(bytes) => {
                        for (i, (addr_lbl, data_lbls)) in rows_vec.iter().enumerate() {
                            let row_addr = start_addr + (i as u64 * 16);
                            addr_lbl.set_label(&format!("{:07X}X", row_addr >> 4));
                            
                            for (j, lbl) in data_lbls.iter().enumerate() {
                                let offset = (i * 16) + (j * 4);
                                let cell_addr = row_addr + (j as u64 * 4);
                                
                                if Some(cell_addr) == hl_addr {
                                    lbl.add_css_class("selected-cell");
                                    if let Ok(mut sel) = selected_cell.try_borrow_mut() {
                                        *sel = Some(lbl.clone());
                                    }
                                } else {
                                    lbl.remove_css_class("selected-cell");
                                }
                                
                                if offset + 3 < bytes.len() {
                                    let val = u32::from_be_bytes([
                                        bytes[offset], bytes[offset+1], bytes[offset+2], bytes[offset+3]
                                    ]);
                                    lbl.set_label(&format!("{:08X}", val));
                                } else {
                                    lbl.set_label("XXXXXXXX");
                                }
                            }
                        }
                    },
                    Err(_) => {
                         for (addr_lbl, data_lbls) in rows_vec.iter() {
                            addr_lbl.set_label("????????");
                            for lbl in data_lbls { 
                                lbl.set_label("????????");
                                lbl.remove_css_class("selected-cell");
                            }
                         }
                    }
                }
            }
        })
    };

    let update_view_clone = update_view.clone();
    update_btn.connect_clicked(move |_| {
        update_view_clone();
    });

    let scroll_controller = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
    let state_clone = state.clone();
    let update_view_clone = update_view.clone();
    scroll_controller.connect_scroll(move |_, _dx, dy| {
        let mut state_ref = state_clone.borrow_mut();
        
        let scroll_amount: i64 = if dy > 0.0 { 0x10 } else { -0x10 };        
        if scroll_amount < 0 {
             if state_ref.memory_viewer_address >= scroll_amount.unsigned_abs() {
                 state_ref.memory_viewer_address -= scroll_amount.unsigned_abs();
             }
        } else {
             state_ref.memory_viewer_address += scroll_amount as u64;
        }
        
        drop(state_ref);
        update_view_clone();
        
        gtk::glib::Propagation::Stop
    });
    grid_box.add_controller(scroll_controller);

    
    let state_clone = state.clone();
    let update_view_clone = update_view.clone();
    addr_entry.connect_activate(move |entry: &gtk::Entry| {
        let text = entry.text();
         if let Ok(addr) = u64::from_str_radix(text.trim().trim_start_matches("0x"), 16) {
            let mut state_ref = state_clone.borrow_mut();
            state_ref.memory_viewer_address = addr;
            drop(state_ref);
            update_view_clone();
         }
    });

    let state_clone = state.clone();
    auto_check.connect_toggled(move |btn: &gtk::CheckButton| {
        state_clone.borrow_mut().memory_viewer_auto_update = btn.is_active();
    });

    let state_clone = state.clone();
    let update_view_clone = update_view.clone();
    glib::timeout_add_local(Duration::from_millis(1000), move || {
        if state_clone.borrow().memory_viewer_auto_update {
            update_view_clone();
        }
        glib::ControlFlow::Continue
    });

    let update_view_clone = update_view.clone();
    resolve_btn.connect_clicked(move |_| {
        update_view_clone();
    });

    let state_clone = state.clone();
    dump_btn.connect_clicked(move |_| {
        let file_dialog = gtk::FileDialog::builder()
            .title("Save RAM Dump")
            .initial_name("ram_dump.txt")
            .build();
        
        let state_inner = state_clone.clone();
        file_dialog.save(None::<&gtk::Window>, gio::Cancellable::NONE, move |result| {
             if let Ok(file) = result
                 && let Some(path) = file.path() {
                     let state_ref = state_inner.borrow();
                     let es = state_ref.engine_state.lock().unwrap();
                     if let Some(pm) = &es.pmem {
                         let pm_clone = pm.clone();
                         let path_clone = path.clone();
                         std::thread::spawn(move || {
                             let dump_start = 0x10000000;
                             let dump_size = 0x10000000;
                             match pm_clone.read_bytes(dump_start, dump_size) {
                                  Ok(data) => {
                                      use std::io::Write;
                                      if let Ok(mut file) = std::fs::File::create(path_clone) {
                                          for (i, chunk) in data.chunks(16).enumerate() {
                                              let addr = dump_start + (i as u64 * 16);
                                              let hex_part: String = chunk.iter()
                                                  .map(|b| format!("{:02X}", b))
                                                  .collect::<Vec<String>>()
                                                  .join(" ");
                                              
                                              let ascii_part: String = chunk.iter()
                                                  .map(|b| if *b >= 32 && *b <= 126 { *b as char } else { '.' })
                                                  .collect();
                                              
                                              let _ = writeln!(file, "{:08X}: {:<47} | {}", addr, hex_part, ascii_part);
                                          }
                                      }
                                  },
                                  Err(e) => {
                                      eprintln!("Failed to dump RAM: {}", e);
                                  }
                             }
                         });
                     }
                 }
        });
    });

    page
}
