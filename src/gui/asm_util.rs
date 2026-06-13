use adw::prelude::*;
use gtk4 as gtk;
use gtk4::prelude::*;
use libadwaita as adw;

use crate::logic::asmutil;

pub fn show_assembler_window(parent: Option<&gtk::Window>) {
    let window = adw::Window::builder()
        .title("Assembler Utility")
        .default_width(400)
        .default_height(300)
        .modal(true)
        .build();

    if let Some(p) = parent {
        window.set_transient_for(Some(p));
    }

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let header = adw::HeaderBar::new();
    let title = adw::WindowTitle::new("Assembler Utility", "IBM Espresso");
    header.set_title_widget(Some(&title));
    main_box.append(&header);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    content.set_margin_start(12);
    content.set_margin_end(12);
    content.set_vexpand(true);

    let columns_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    columns_box.set_vexpand(true);
    columns_box.set_homogeneous(true);

    let input_col = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let input_label = gtk::Label::builder()
        .label("Gecko")
        .halign(gtk::Align::Start)
        .build();
    input_label.add_css_class("heading");
    input_col.append(&input_label);

    let input_view = gtk::TextView::builder()
        .monospace(true)
        .wrap_mode(gtk::WrapMode::Word)
        .vexpand(true)
        .build();
    let input_scroll = gtk::ScrolledWindow::builder()
        .child(&input_view)
        .vexpand(true)
        .build();
    input_scroll.add_css_class("card");
    input_col.append(&input_scroll);
    columns_box.append(&input_col);

    let output_col = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let output_label = gtk::Label::builder()
        .label("ASM")
        .halign(gtk::Align::Start)
        .build();
    output_label.add_css_class("heading");
    output_col.append(&output_label);

    let output_view = gtk::TextView::builder()
        .monospace(true)
        .editable(false)
        .wrap_mode(gtk::WrapMode::Word)
        .vexpand(true)
        .build();
    let output_scroll = gtk::ScrolledWindow::builder()
        .child(&output_view)
        .vexpand(true)
        .build();
    output_scroll.add_css_class("card");
    output_col.append(&output_scroll);
    columns_box.append(&output_col);

    content.append(&columns_box);

    let bottom_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    bottom_box.set_margin_top(8);

    let mode_label = gtk::Label::new(Some("Mode:"));
    let mode_model = gtk::StringList::new(&["Console → Cemu", "Cemu → Console"]);
    let mode_dropdown = gtk::DropDown::builder().model(&mode_model).build();
    bottom_box.append(&mode_label);
    bottom_box.append(&mode_dropdown);

    let convert_btn = gtk::Button::builder()
        .label("Convert")
        .hexpand(true)
        .halign(gtk::Align::End)
        .build();
    convert_btn.add_css_class("suggested-action");
    convert_btn.add_css_class("pill");
    bottom_box.append(&convert_btn);

    content.append(&bottom_box);

    main_box.append(&content);
    window.set_content(Some(&main_box));

    let mode_dd = mode_dropdown.clone();
    let input_v = input_view.clone();
    let output_v = output_view.clone();

    let input_label_clone = input_label.clone();
    let output_label_clone = output_label.clone();
    let input_v_swap = input_view.clone();
    let output_v_swap = output_view.clone();
    mode_dropdown.connect_selected_item_notify(move |dd| {
        if dd.selected() == 0 {
            input_label_clone.set_label("Gecko");
            output_label_clone.set_label("ASM");
        } else {
            input_label_clone.set_label("ASM");
            output_label_clone.set_label("Gecko");
        }

        let in_buf = input_v_swap.buffer();
        let out_buf = output_v_swap.buffer();
        
        let in_text = in_buf.text(&in_buf.start_iter(), &in_buf.end_iter(), false).to_string();
        let out_text = out_buf.text(&out_buf.start_iter(), &out_buf.end_iter(), false).to_string();
        
        in_buf.set_text(&out_text);
        out_buf.set_text(&in_text);
    });

    convert_btn.connect_clicked(move |_| {
        let buffer = input_v.buffer();
        let text = buffer
            .text(&buffer.start_iter(), &buffer.end_iter(), false)
            .to_string();

        let result = if mode_dd.selected() == 0 {
            let cemu_gecko = asmutil::gecko_convert_addresses(&text, true);
            asmutil::gecko_to_asm(&cemu_gecko)
        } else {
            match asmutil::cemu_asm_to_gecko(&text, 0) {
                Ok(gecko) => Ok(asmutil::gecko_convert_addresses(&gecko, false)),
                Err(e) => Err(e),
            }
        };

        match result {
            Ok(output) => {
                output_v.buffer().set_text(&output);
            }
            Err(e) => {
                output_v.buffer().set_text(&format!("Error: {}", e));
            }
        }
    });

    window.present();
}
