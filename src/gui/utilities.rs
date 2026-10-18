use crate::gui::AppState;
use adw::prelude::*;
use anyhow::{Result, anyhow};
use gtk::gio;
use gtk4 as gtk;
use libadwaita as adw;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub fn create_utilities_page(state: &Rc<RefCell<AppState>>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title("Utilities")
        .icon_name("preferences-other-symbolic")
        .build();

    let dump_group = adw::PreferencesGroup::builder().build();

    let format_model = gtk::StringList::new(&["PNG", "TGA"]);
    let format_dropdown = gtk::DropDown::builder()
        .model(&format_model)
        .valign(gtk::Align::Center)
        .build();

    let dump_btn = gtk::Button::builder()
        .label("Dump")
        .valign(gtk::Align::Center)
        .build();

    let faceimg_row = adw::ActionRow::builder()
        .title("Dump faceimg.tga")
        .subtitle("Disable Anti Telemetry &amp; play a match first")
        .build();

    let state_clone = state.clone();
    let format_dropdown_clone = format_dropdown.clone();
    dump_btn.connect_clicked(move |btn| {
        let state_ref = state_clone.borrow();
        let es = state_ref.engine_state.lock().unwrap();
        let pm = match es.pmem.clone() {
            Some(pm) => pm,
            None => {
                let dialog = adw::AlertDialog::builder()
                    .heading("Not Connected")
                    .body("Please connect to Cemu first.")
                    .build();
                dialog.add_response("ok", "OK");
                if let Some(root) = btn.root() {
                    if let Ok(window) = root.downcast::<gtk::Window>() {
                        dialog.present(Some(&window));
                    }
                }
                return;
            }
        };
        drop(es);

        let selected_index = format_dropdown_clone.selected();
        let extension = if selected_index == 0 { "png" } else { "tga" };

        let file_dialog = gtk::FileDialog::builder()
            .title("Save faceimg.tga")
            .initial_name(format!("faceimg.{}", extension))
            .build();

        let filter_png = gtk::FileFilter::new();
        filter_png.add_pattern("*.png");
        filter_png.set_name(Some("PNG Image"));

        let filter_tga = gtk::FileFilter::new();
        filter_tga.add_pattern("*.tga");
        filter_tga.set_name(Some("TGA Image"));

        let filters = gio::ListStore::new::<gtk::FileFilter>();
        if selected_index == 0 {
            filters.append(&filter_png);
            filters.append(&filter_tga);
        } else {
            filters.append(&filter_tga);
            filters.append(&filter_png);
        }
        file_dialog.set_filters(Some(&filters));

        if let Some(root) = btn.root() {
            if let Ok(root_window) = root.downcast::<gtk::Window>() {
                let root_clone = root_window.clone();
                file_dialog.save(Some(&root_window), gio::Cancellable::NONE, move |res| {
                    if let Ok(file) = res
                        && let Some(path) = file.path()
                    {
                        let dump_res = (|| -> Result<()> {
                            let p = pm.read_u32(0x101DCDB0)? as u64 + 0x150;
                            let local_player = pm.read_bytes(p, 5376)?;
                            let pattern = [
                                0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                                0x00, 0x80, 0x00, 0x80, 0x00,
                            ];

                            let mut faceimg_offset = None;
                            if local_player.len() >= pattern.len() {
                                for i in 0..=local_player.len() - pattern.len() {
                                    let mut matched = true;
                                    for j in 0..pattern.len() {
                                        if local_player[i + j] != pattern[j] {
                                            matched = false;
                                            break;
                                        }
                                    }
                                    if matched {
                                        faceimg_offset = Some(i);
                                        break;
                                    }
                                }
                            }

                            if let Some(offset) = faceimg_offset {
                                let faceimg = pm.read_bytes(p + offset as u64, 65580)?;
                                let is_png = path
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .map(|s| s.eq_ignore_ascii_case("png"))
                                    .unwrap_or(false);

                                if is_png {
                                    let img = image::load_from_memory_with_format(
                                        &faceimg,
                                        image::ImageFormat::Tga,
                                    )
                                    .map_err(|e| anyhow!("Failed to load TGA: {}", e))?;
                                    img.save_with_format(&path, image::ImageFormat::Png)
                                        .map_err(|e| anyhow!("Failed to save PNG: {}", e))?;
                                } else {
                                    std::fs::write(&path, &faceimg)?;
                                }

                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::PermissionsExt;
                                    let perms = std::fs::Permissions::from_mode(0o777);
                                    let _ = std::fs::set_permissions(&path, perms);
                                }

                                Ok(())
                            } else {
                                Err(anyhow!("Could not find FaceImg.tga pattern!"))
                            }
                        })();

                        let root = root_clone.clone();
                        match dump_res {
                            Ok(_) => {
                                let dialog = adw::AlertDialog::builder()
                                    .heading("Dump Successful")
                                    .body(format!("File saved to: {}", path.display()))
                                    .build();
                                dialog.add_response("ok", "OK");
                                dialog.present(Some(&root));
                            }
                            Err(e) => {
                                let dialog = adw::AlertDialog::builder()
                                    .heading("Dump Failed")
                                    .body(format!("Error: {}", e))
                                    .build();
                                dialog.add_response("ok", "OK");
                                dialog.present(Some(&root));
                            }
                        }
                    }
                });
            }
        }
    });

    faceimg_row.add_suffix(&format_dropdown);
    faceimg_row.add_suffix(&dump_btn);
    dump_group.add(&faceimg_row);

    let asm_row = adw::ActionRow::builder()
        .title("Assembler Utility")
        .subtitle("Convert between Gecko codes and PowerPC ASM")
        .build();

    let asm_btn = gtk::Button::builder()
        .label("Open")
        .valign(gtk::Align::Center)
        .build();
    asm_btn.add_css_class("suggested-action");

    asm_btn.connect_clicked(move |btn| {
        let root = btn.root().and_then(|r| r.downcast::<gtk::Window>().ok());
        crate::gui::asm_util::show_assembler_window(root.as_ref());
    });

    asm_row.add_suffix(&asm_btn);
    dump_group.add(&asm_row);

    let utf_row = adw::ActionRow::builder()
        .title("UTF Converter")
        .subtitle("Convert between Text and Hex")
        .build();

    let utf_btn = gtk::Button::builder()
        .label("Open")
        .valign(gtk::Align::Center)
        .build();
    utf_btn.add_css_class("suggested-action");

    utf_btn.connect_clicked(move |btn| {
        let root = btn.root().and_then(|r| r.downcast::<gtk::Window>().ok());
        crate::gui::utf_util::show_utf_window(root.as_ref());
    });

    utf_row.add_suffix(&utf_btn);
    dump_group.add(&utf_row);

    let gpc_row = adw::ActionRow::builder()
        .title("Cemu Graphic Pack Creator")
        .subtitle("Create Graphic Packs for Cemu")
        .build();

    let gpc_btn = gtk::Button::builder()
        .label("Open")
        .valign(gtk::Align::Center)
        .build();
    gpc_btn.add_css_class("suggested-action");

    gpc_btn.connect_clicked(move |btn| {
        let root = btn.root().and_then(|r| r.downcast::<gtk::Window>().ok());
        crate::gui::graphic_pack_creator::show_graphic_pack_window(root.as_ref());
    });

    gpc_row.add_suffix(&gpc_btn);
    dump_group.add(&gpc_row);

    page.add(&dump_group);

    let hexdec_group = adw::PreferencesGroup::builder()
        .title("Hex / Decimal Converter")
        .build();

    let conv_model = gtk::StringList::new(&["Hex to Dec", "Dec to Hex"]);
    let conv_dropdown = gtk::DropDown::builder()
        .model(&conv_model)
        .valign(gtk::Align::Center)
        .build();

    let conv_input_entry = gtk::Entry::builder()
        .placeholder_text("0x00000000")
        .width_chars(14)
        .valign(gtk::Align::Center)
        .build();

    let conv_input_row = adw::ActionRow::builder()
        .title("Input")
        .subtitle("Hex uses optional 0x prefix")
        .build();
    conv_input_row.add_suffix(&conv_dropdown);
    conv_input_row.add_suffix(&conv_input_entry);
    hexdec_group.add(&conv_input_row);

    let conv_result_entry = gtk::Entry::builder()
        .editable(false)
        .width_chars(14)
        .valign(gtk::Align::Center)
        .build();
    conv_result_entry.add_css_class("monospace");

    let conv_result_row = adw::ActionRow::builder()
        .title("Result")
        .build();
    conv_result_row.add_suffix(&conv_result_entry);
    hexdec_group.add(&conv_result_row);

    let conv_selected = Rc::new(Cell::new(0u32));
    {
        let conv_selected = conv_selected.clone();
        let conv_result_entry = conv_result_entry.clone();
        let conv_input_entry = conv_input_entry.clone();
        conv_dropdown.connect_selected_notify(move |dd| {
            let selected = dd.selected();
            conv_selected.set(selected);
            match selected {
                0 => {
                    conv_input_entry.set_placeholder_text(Some("0x00000000"));
                    let raw = conv_input_entry.text().to_string();
                    let trimmed = raw.trim().trim_start_matches("0x").trim_start_matches("0X");
                    if raw.trim().is_empty() {
                        conv_result_entry.set_text("");
                        conv_input_entry.remove_css_class("error");
                    } else if let Ok(value) = u64::from_str_radix(trimmed, 16) {
                        conv_result_entry.set_text(&value.to_string());
                        conv_input_entry.remove_css_class("error");
                    } else {
                        conv_result_entry.set_text("");
                        conv_input_entry.add_css_class("error");
                    }
                }
                1 => {
                    conv_input_entry.set_placeholder_text(Some("0"));
                    let raw = conv_input_entry.text().to_string();
                    let trimmed = raw.trim();
                    if raw.trim().is_empty() {
                        conv_result_entry.set_text("");
                        conv_input_entry.remove_css_class("error");
                    } else if let Ok(value) = trimmed.parse::<u64>() {
                        conv_result_entry.set_text(&format!("{:X}", value));
                        conv_input_entry.remove_css_class("error");
                    } else {
                        conv_result_entry.set_text("");
                        conv_input_entry.add_css_class("error");
                    }
                }
                _ => {}
            }
        });
    }

    {
        let conv_selected = conv_selected.clone();
        let conv_result_entry = conv_result_entry.clone();
        conv_input_entry.connect_changed(move |e| {
            let selected = conv_selected.get();
            if selected == 0 {
                let raw = e.text().to_string();
                let trimmed = raw.trim().trim_start_matches("0x").trim_start_matches("0X");
                if raw.trim().is_empty() {
                    conv_result_entry.set_text("");
                    e.remove_css_class("error");
                } else if let Ok(value) = u64::from_str_radix(trimmed, 16) {
                    conv_result_entry.set_text(&value.to_string());
                    e.remove_css_class("error");
                } else {
                    conv_result_entry.set_text("");
                    e.add_css_class("error");
                }
            } else if selected == 1 {
                let raw = e.text().to_string();
                let trimmed = raw.trim();
                if raw.trim().is_empty() {
                    conv_result_entry.set_text("");
                    e.remove_css_class("error");
                } else if let Ok(value) = trimmed.parse::<u64>() {
                    conv_result_entry.set_text(&format!("{:X}", value));
                    e.remove_css_class("error");
                } else {
                    conv_result_entry.set_text("");
                    e.add_css_class("error");
                }
            }
        });
    }

    page.add(&hexdec_group);

    let offset_group = adw::PreferencesGroup::builder()
        .title("Address Offset Calculator")
        .build();

    let offset_model = gtk::StringList::new(&["Wii U to Cemu", "Cemu to Wii U"]);
    let offset_dropdown = gtk::DropDown::builder()
        .model(&offset_model)
        .valign(gtk::Align::Center)
        .build();

    let offset_input_entry = gtk::Entry::builder()
        .placeholder_text("0x10000000")
        .width_chars(14)
        .valign(gtk::Align::Center)
        .build();

    let offset_input_row = adw::ActionRow::builder()
        .title("Input Address")
        .subtitle("Enter Address to be converted")
        .build();
    offset_input_row.add_suffix(&offset_dropdown);
    offset_input_row.add_suffix(&offset_input_entry);
    offset_group.add(&offset_input_row);

    let offset_result_entry = gtk::Entry::builder()
        .editable(false)
        .width_chars(14)
        .valign(gtk::Align::Center)
        .build();
    offset_result_entry.add_css_class("monospace");

    let offset_result_row = adw::ActionRow::builder()
        .title("Output Address")
        .build();
    offset_result_row.add_suffix(&offset_result_entry);
    offset_group.add(&offset_result_row);

    let recalculate: Rc<dyn Fn()> = {
        let offset_input_entry = offset_input_entry.clone();
        let offset_result_entry = offset_result_entry.clone();
        let offset_dropdown = offset_dropdown.clone();
        Rc::new(move || {
            let raw = offset_input_entry.text().to_string();
            let trimmed = raw.trim().trim_start_matches("0x").trim_start_matches("0X");
            if raw.trim().is_empty() {
                offset_result_entry.set_text("");
                offset_input_entry.remove_css_class("error");
                return;
            }
            match u64::from_str_radix(trimmed, 16) {
                Ok(value) if value <= u32::MAX as u64 => {
                    let result = if offset_dropdown.selected() == 0 {
                        (value as i64 - 0x503000) as u64
                    } else {
                        value + 0x503000
                    };
                    offset_result_entry.set_text(&format!("0x{:08X}", result));
                    offset_input_entry.remove_css_class("error");
                }
                _ => {
                    offset_result_entry.set_text("");
                    offset_input_entry.add_css_class("error");
                }
            }
        })
    };

    {
        let recalculate = recalculate.clone();
        offset_input_entry.connect_changed(move |_| recalculate());
    }

    {
        let recalculate = recalculate.clone();
        offset_dropdown.connect_selected_notify(move |_| recalculate());
    }

    page.add(&offset_group);

    let name_group = adw::PreferencesGroup::builder()
        .title("Name Changer")
        .build();

    let name_row = adw::ActionRow::builder()
        .title("Player Name")
        .subtitle("Set in-game name")
        .build();

    let name_entry = gtk::Entry::builder()
        .placeholder_text("Up to 16 characters")
        .max_length(16)
        .width_chars(18)
        .valign(gtk::Align::Center)
        .build();

    let name_switch = gtk::Switch::new();
    name_switch.set_valign(gtk::Align::Center);
    name_switch.set_sensitive(false);

    let switch_entry = name_switch.clone();
    let state_name_entry = state.clone();
    name_entry.connect_changed(move |e| {
        let text = e.text().to_string();
        switch_entry.set_sensitive(!text.is_empty());
        if switch_entry.is_active() {
            if let Ok(s) = state_name_entry.try_borrow() {
                crate::logic::utilities::write_name_to_memory(&s.engine_state, &text);
            }
        }
    });

    let state_name = state.clone();
    let entry_sw = name_entry.clone();
    name_switch.connect_active_notify(move |sw| {
        let active = sw.is_active();
        if let Ok(s) = state_name.try_borrow() {
            if active {
                crate::logic::utilities::write_name_to_memory(&s.engine_state, &entry_sw.text().to_string());
            } else {
                crate::logic::utilities::write_name_to_memory(&s.engine_state, "");
            }
        }
    });

    name_row.add_suffix(&name_entry);
    name_row.add_suffix(&name_switch);
    name_group.add(&name_row);
    page.add(&name_group);

    page
}
