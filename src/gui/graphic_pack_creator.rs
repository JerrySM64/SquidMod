use crate::logic::graphic_pack_creator::OutputEntry;
use adw::prelude::*;
use gtk::gio;
use gtk4 as gtk;
use gtk4::prelude::*;
use libadwaita as adw;
use std::path::{Path, PathBuf};

pub fn show_graphic_pack_window(parent: Option<&gtk::Window>) {
    let window = adw::Window::builder()
        .title("Cemu Graphic Pack Creator")
        .default_width(520)
        .default_height(560)
        .modal(true)
        .build();

    if let Some(p) = parent {
        window.set_transient_for(Some(p));
    }

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let header = adw::HeaderBar::new();
    let title = adw::WindowTitle::new("Cemu Graphic Pack Creator", "");
    header.set_title_widget(Some(&title));
    main_box.append(&header);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_vexpand(true);

    let name_label = gtk::Label::builder()
        .label("Name")
        .halign(gtk::Align::Start)
        .build();
    name_label.add_css_class("heading");
    content.append(&name_label);

    let name_entry = gtk::Entry::builder()
        .placeholder_text("My Graphic Pack")
        .hexpand(true)
        .build();
    content.append(&name_entry);

    let rules_path_label = gtk::Label::builder()
        .label("Rules Path")
        .halign(gtk::Align::Start)
        .build();
    rules_path_label.add_css_class("heading");
    content.append(&rules_path_label);

    let rules_path_entry = gtk::Entry::builder()
        .placeholder_text("Splatoon/Mods/My Pack")
        .hexpand(true)
        .build();
    content.append(&rules_path_entry);

    let content_label = gtk::Label::builder()
        .label("Content Directory")
        .halign(gtk::Align::Start)
        .build();
    content_label.add_css_class("heading");
    content.append(&content_label);

    let content_entry = gtk::Entry::builder()
        .placeholder_text("/path/to/content")
        .hexpand(true)
        .build();

    let content_browse_btn = gtk::Button::builder().label("Browse...").build();

    let content_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content_box.append(&content_entry);
    content_box.append(&content_browse_btn);
    content.append(&content_box);

    let desc_label = gtk::Label::builder()
        .label("Description")
        .halign(gtk::Align::Start)
        .build();
    desc_label.add_css_class("heading");
    content.append(&desc_label);

    let desc_entry = gtk::Entry::builder()
        .placeholder_text("Describe your pack")
        .hexpand(true)
        .build();
    content.append(&desc_entry);

    let patch_label = gtk::Label::builder()
        .label("Patch")
        .halign(gtk::Align::Start)
        .build();
    patch_label.add_css_class("heading");
    content.append(&patch_label);

    let patch_view = gtk::TextView::builder()
        .monospace(true)
        .wrap_mode(gtk::WrapMode::Word)
        .vexpand(true)
        .build();
    let patch_scroll = gtk::ScrolledWindow::builder()
        .child(&patch_view)
        .vexpand(true)
        .build();
    patch_scroll.add_css_class("card");
    content.append(&patch_scroll);

    let save_btn = gtk::Button::builder()
        .label("Save Pack")
        .halign(gtk::Align::End)
        .build();
    save_btn.add_css_class("suggested-action");
    save_btn.add_css_class("pill");
    content.append(&save_btn);

    main_box.append(&content);
    window.set_content(Some(&main_box));

    {
        let content_entry = content_entry.clone();
        content_browse_btn.connect_clicked(move |btn| {
            let dialog = gtk::FileDialog::builder()
                .title("Select Modpack Content")
                .build();
            let content_entry = content_entry.clone();
            if let Some(root) = btn.root().and_then(|r| r.downcast::<gtk::Window>().ok()) {
                dialog.select_folder(Some(&root), gio::Cancellable::NONE, move |res| {
                    if let Ok(folder) = res
                        && let Some(path) = folder.path()
                    {
                        content_entry.set_text(&path.display().to_string());
                    }
                });
            }
        });
    }

    {
        let name_entry = name_entry.clone();
        let rules_path_entry = rules_path_entry.clone();
        let desc_entry = desc_entry.clone();
        let content_entry = content_entry.clone();
        let patch_view = patch_view.clone();
        save_btn.connect_clicked(move |btn| {
            let name = name_entry.text().to_string();
            let rules_path = rules_path_entry.text().to_string();
            let description = desc_entry.text().to_string();
            let patch_buffer = patch_view.buffer();
            let patch_text = patch_buffer
                .text(&patch_buffer.start_iter(), &patch_buffer.end_iter(), false)
                .to_string();

            let patch_header = name.replace(' ', "-");

            let content_dir = PathBuf::from(content_entry.text().trim());
            let content_dir = if content_dir.as_os_str().is_empty() {
                None
            } else {
                Some(content_dir)
            };

            let root = btn.root().and_then(|r| r.downcast::<gtk::Window>().ok());

            if name.trim().is_empty() {
                let alert = adw::AlertDialog::builder()
                    .heading("Save Failed")
                    .body("Please enter a pack name.")
                    .build();
                alert.add_response("ok", "OK");
                if let Some(window) = root.as_ref() {
                    alert.present(Some(window));
                }
                return;
            }

            let dialog = gtk::FileDialog::builder()
                .title("Select Output Location")
                .build();

            if let Some(parent) = root {
                let parent_clone = parent.clone();
                dialog.select_folder(Some(&parent), gio::Cancellable::NONE, move |res| {
                    let (heading, body) = match res {
                        Ok(folder) => match folder.path() {
                            Some(output_dir) => {
                                let pack = crate::logic::graphic_pack_creator::generate_pack(
                                    &name,
                                    &rules_path,
                                    &description,
                                    &patch_header,
                                    &patch_text,
                                    content_dir.as_deref(),
                                );
                                let entries =
                                    crate::logic::graphic_pack_creator::build_output_entries(
                                        &pack,
                                    );
                                let pack_dir = output_dir.join(&pack.name);
                                match execute_output_entries(&output_dir, &entries, &pack_dir) {
                                    Ok(_) => (
                                        "Save Successful",
                                        format!("Graphic pack saved to: {}", pack_dir.display()),
                                    ),
                                    Err(e) => ("Save Failed", format!("Error: {}", e)),
                                }
                            }
                            None => (
                                "Save Failed",
                                "Could not resolve the selected folder.".to_string(),
                            ),
                        },
                        Err(_) => return,
                    };

                    let alert = adw::AlertDialog::builder()
                        .heading(heading)
                        .body(body)
                        .build();
                    alert.add_response("ok", "OK");
                    alert.present(Some(&parent_clone));
                });
            }
        });
    }

    window.present();
}

fn execute_output_entries(
    output_dir: &Path,
    entries: &[OutputEntry],
    skip_dir: &Path,
) -> std::io::Result<()> {
    for entry in entries {
        match entry {
            OutputEntry::File {
                relative_path,
                contents,
            } => {
                let target = output_dir.join(relative_path);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&target, contents)?;
            }
            OutputEntry::CopyDir {
                source,
                relative_path,
            } => {
                let target = output_dir.join(relative_path);
                copy_dir_recursive(source, &target, skip_dir)?;
            }
        }
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, target: &Path, skip_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        if source_path == skip_dir {
            continue;
        }
        let target_path = target.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &target_path, skip_dir)?;
        } else {
            std::fs::copy(&source_path, &target_path)?;
        }
    }
    Ok(())
}
