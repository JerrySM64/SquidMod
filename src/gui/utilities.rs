use crate::gui::AppState;
use adw::prelude::*;
use anyhow::{Result, anyhow};
use gtk::gio;
use gtk4 as gtk;
use libadwaita as adw;
use std::cell::RefCell;
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
    page.add(&dump_group);

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
