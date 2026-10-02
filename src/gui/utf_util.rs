use adw::prelude::*;
use gtk4 as gtk;
use gtk4::prelude::*;
use libadwaita as adw;

pub fn show_utf_window(parent: Option<&gtk::Window>) {
    let window = adw::Window::builder()
        .title("UTF Converter")
        .default_width(400)
        .default_height(300)
        .modal(true)
        .build();

    if let Some(p) = parent {
        window.set_transient_for(Some(p));
    }

    let main_box = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let header = adw::HeaderBar::new();
    let title = adw::WindowTitle::new("UTF Converter", "");
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
        .label("Text")
        .halign(gtk::Align::Start)
        .build();
    input_label.add_css_class("heading");
    input_col.append(&input_label);

    let input_view = gtk::TextView::builder()
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
        .label("Hex")
        .halign(gtk::Align::Start)
        .build();
    output_label.add_css_class("heading");
    output_col.append(&output_label);

    let output_view = gtk::TextView::builder()
        .monospace(true)
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

    let mode_model = gtk::StringList::new(&[
        "Text to Hex (UTF-8)",
        "Hex to Text (UTF-8)",
        "Text to Hex (UTF-16BE)",
        "Hex to Text (UTF-16BE)",
    ]);
    let mode_dropdown = gtk::DropDown::builder().model(&mode_model).build();
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

    let input_label_clone = input_label.clone();
    let output_label_clone = output_label.clone();
    mode_dropdown.connect_selected_notify(move |dd| {
        if dd.selected() == 0 || dd.selected() == 2 {
            input_label_clone.set_label("Text");
            output_label_clone.set_label("Hex");
        } else {
            input_label_clone.set_label("Hex");
            output_label_clone.set_label("Text");
        }
    });

    let mode_dd = mode_dropdown.clone();
    let input_v = input_view.clone();
    let output_v = output_view.clone();

    convert_btn.connect_clicked(move |_| {
        let selected = mode_dd.selected();

        let in_buf = input_v.buffer();
        let input = in_buf
            .text(&in_buf.start_iter(), &in_buf.end_iter(), false)
            .to_string();

        let out = match selected {
            0 => input
                .as_bytes()
                .iter()
                .map(|b| format!("{:02X}", b))
                .collect::<Vec<_>>()
                .join(" "),
            1 => {
                let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
                let bytes: Vec<u8> = (0..cleaned.len() / 2)
                    .filter_map(|i| u8::from_str_radix(&cleaned[i * 2..i * 2 + 2], 16).ok())
                    .collect();
                String::from_utf8(bytes).unwrap_or_default()
            }
            2 => input
                .encode_utf16()
                .map(|b| format!("{:04X}", b))
                .collect::<Vec<_>>()
                .join(" "),
            3 => {
                let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
                let units: Vec<u16> = (0..cleaned.len() / 4)
                    .filter_map(|i| u16::from_str_radix(&cleaned[i * 4..i * 4 + 4], 16).ok())
                    .collect();
                String::from_utf16(&units).unwrap_or_default()
            }
            _ => String::new(),
        };

        output_v.buffer().set_text(&out);
    });

    window.present();
}
