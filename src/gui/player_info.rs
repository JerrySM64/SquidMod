use crate::gui::{AppState, NetworkMode};
use crate::logic::playerinfo::{PlayerRecord, fetch_all_players, gender_label};
use adw::prelude::*;
use gtk::glib;
use gtk4 as gtk;
use libadwaita as adw;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

pub fn get_player_report(p: &PlayerRecord, state: &AppState) -> String {
    let mut report = String::new();
    report.push_str(&format!("Name: {}\n", p.name));
    report.push_str(&format!("PID Hex: {}\n", p.pid_hex));
    report.push_str(&format!("PID Dec: {}\n", p.pid_dec));
    let id_label = if state.network_mode == NetworkMode::Spacebar {
        "SFID"
    } else {
        "PNID"
    };
    report.push_str(&format!(
        "{}: {}\n",
        id_label,
        if p.pnid == "0" {
            "Unknown".to_string()
        } else {
            p.pnid.clone()
        }
    ));
    report.push_str(&format!(
        "Gender: {} ({})\n",
        p.gender,
        gender_label(p.gender)
    ));
    report.push_str(&format!("Skin Tone: {}\n", p.skin_tone));
    report.push_str(&format!(
        "Eye Color: {} ({})\n",
        p.eye_color, p.eye_color_name
    ));
    report.push_str(&format!("Headgear: {} ({})\n", p.headgear, p.headgear_name));
    report.push_str(&format!("Clothes: {} ({})\n", p.clothes, p.clothes_name));
    report.push_str(&format!("Shoes: {} ({})\n", p.shoes, p.shoes_name));
    report.push_str(&format!("Ink Tank: {} ({})\n", p.tank_id, p.tank_name));
    report.push_str(&format!(
        "Main Weapon ID: {} ({})\n",
        p.weapon_id_main, p.weapon_main_name
    ));
    report.push_str(&format!(
        "Sub Weapon ID: {} ({})\n",
        p.weapon_id_sub, p.weapon_sub_name
    ));
    report.push_str(&format!(
        "Special Weapon ID: {} ({})\n",
        p.weapon_id_special, p.weapon_special_name
    ));
    report.push_str(&format!("Level: {}\n", p.rank + 1));
    report.push_str(&format!("Rank: {} ({})\n", p.rank_label, p.rank_points));
    report.push_str(&format!("Fest Team: {}\n", p.fest_team));
    report.push_str(&format!("Fest ID: {}\n", p.fest_id));
    report.push_str(&format!("Fest Title: {}\n", p.fest_grade));
    report
}

pub fn show_player_properties(
    parent: Option<&gtk::Window>,
    player: &PlayerRecord,
    network_mode: NetworkMode,
) {
    let window = adw::Window::builder()
        .modal(true)
        .title(&player.name)
        .default_width(450)
        .default_height(600)
        .build();

    if let Some(parent_win) = parent {
        window.set_transient_for(Some(parent_win));
    }

    let header = adw::HeaderBar::new();

    let page = adw::PreferencesPage::new();

    let add_row = |group: &adw::PreferencesGroup, title: &str, subtitle: String| {
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle(subtitle)
            .build();
        group.add(&row);
    };

    let general_group = adw::PreferencesGroup::builder().title("General").build();
    add_row(&general_group, "Name", player.name.clone());
    add_row(&general_group, "PID (Hex)", player.pid_hex.clone());
    add_row(&general_group, "PID (Dec)", player.pid_dec.to_string());
    let id_label = if network_mode == NetworkMode::Spacebar {
        "SFID"
    } else {
        "PNID"
    };
    add_row(
        &general_group,
        id_label,
        if player.pnid == "0" {
            "Unknown".to_string()
        } else {
            player.pnid.clone()
        },
    );
    page.add(&general_group);

    let appearance_group = adw::PreferencesGroup::builder().title("Appearance").build();
    add_row(
        &appearance_group,
        "Gender",
        format!("{} ({})", player.gender, gender_label(player.gender)),
    );
    add_row(&appearance_group, "Skin Tone", player.skin_tone.to_string());
    add_row(
        &appearance_group,
        "Eye Color",
        format!("{} ({})", player.eye_color, player.eye_color_name),
    );
    page.add(&appearance_group);

    let equip_group = adw::PreferencesGroup::builder().title("Equipment").build();
    add_row(
        &equip_group,
        "Headgear",
        format!("{} ({})", player.headgear, player.headgear_name),
    );
    add_row(
        &equip_group,
        "Clothes",
        format!("{} ({})", player.clothes, player.clothes_name),
    );
    add_row(
        &equip_group,
        "Shoes",
        format!("{} ({})", player.shoes, player.shoes_name),
    );
    add_row(
        &equip_group,
        "Ink Tank",
        format!("{} ({})", player.tank_id, player.tank_name),
    );
    page.add(&equip_group);

    let weapon_group = adw::PreferencesGroup::builder().title("Weapon").build();
    add_row(
        &weapon_group,
        "Main Weapon",
        format!("{} ({})", player.weapon_id_main, player.weapon_main_name),
    );
    add_row(
        &weapon_group,
        "Sub Weapon",
        format!("{} ({})", player.weapon_id_sub, player.weapon_sub_name),
    );
    add_row(
        &weapon_group,
        "Special Weapon",
        format!(
            "{} ({})",
            player.weapon_id_special, player.weapon_special_name
        ),
    );
    add_row(
        &weapon_group,
        "Turf Points",
        format!("{}p", player.weaponturf_total),
    );
    page.add(&weapon_group);

    let prog_group = adw::PreferencesGroup::builder()
        .title("Level, Rank &amp; Fest")
        .build();
    add_row(&prog_group, "Level", (player.rank + 1).to_string());
    add_row(
        &prog_group,
        "Rank",
        format!("{} ({})", player.rank_label, player.rank_points),
    );
    add_row(&prog_group, "Fest ID", player.fest_id.to_string());
    add_row(&prog_group, "Fest Team", player.fest_team.to_string());
    add_row(&prog_group, "Fest Title", player.fest_grade.to_string());
    page.add(&prog_group);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&page));

    window.set_content(Some(&toolbar_view));
    window.present();
}

pub fn create_player_info_page(state: &Rc<RefCell<AppState>>) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title("Player Info")
        .icon_name("avatar-default-symbolic")
        .build();

    let header_group = adw::PreferencesGroup::builder()
        .title("Session ID")
        .description("Fetched at: -")
        .build();

    let session_row = adw::ActionRow::builder()
        .title("Session ID")
        .subtitle("Not loaded")
        .build();

    let refresh_btn = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .valign(gtk::Align::Center)
        .tooltip_text("Refresh all data")
        .build();

    let copy_all_btn = gtk::Button::builder()
        .icon_name("edit-copy-symbolic")
        .valign(gtk::Align::Center)
        .tooltip_text("Copy Full Report")
        .build();

    let state_clone = state.clone();
    let session_row_clone = session_row.clone();
    let header_group_clone = header_group.clone();

    refresh_btn.connect_clicked(move |_| {
        let mut state_ref = state_clone.borrow_mut();
        let fetch_res = {
            let es = state_ref.engine_state.lock().unwrap();
            let network_mode = state_ref.network_mode;
            if let Some(ref pm) = es.pmem {
                Some(fetch_all_players(pm, network_mode))
            } else {
                None
            }
        };
        if let Some(res) = fetch_res {
            match res {
                Ok(fetch_result) => {
                    state_ref.players = fetch_result.players;
                    state_ref.session_id = fetch_result.session_id;
                    state_ref.fetched_at = fetch_result
                        .fetched_at
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string();
                    header_group_clone
                        .set_description(Some(&format!("Fetched at: {}", state_ref.fetched_at)));

                    if let Some(sid) = state_ref.session_id {
                        session_row_clone.set_subtitle(&format!("{:08X} ({})", sid, sid));
                    } else {
                        session_row_clone.set_subtitle("None");
                    }
                }
                Err(e) => {
                    eprintln!("Fetch error: {}", e);
                    session_row_clone.set_subtitle("Error reading session");
                }
            }
        }
    });

    let state_clone_copy = state.clone();
    copy_all_btn.connect_clicked(move |_| {
        if let Some(display) = gtk::gdk::Display::default() {
            let clipboard = display.clipboard();
            let state_ref = state_clone_copy.borrow();

            let mut report = String::new();
            for p in &state_ref.players {
                report.push_str(&format!("Player {}\n", p.index + 1));
                report.push_str(&get_player_report(p, &state_ref));
                report.push('\n');
            }
            if let Some(sid) = state_ref.session_id {
                report.push_str(&format!("Session ID: {:08X} (Dec: {})\n", sid, sid));
            } else {
                report.push_str("Session ID: None\n");
            }
            report.push_str(&format!("Fetched at: {}\n", state_ref.fetched_at));

            clipboard.set_text(&report);
        }
    });

    session_row.add_suffix(&refresh_btn);
    session_row.add_suffix(&copy_all_btn);
    header_group.add(&session_row);
    page.add(&header_group);

    let players_group = adw::PreferencesGroup::builder().title("Players").build();

    for i in 0..8 {
        let state_clone = state.clone();

        let player_row = adw::ActionRow::builder()
            .title(format!("Player {}", i + 1))
            .subtitle("No data")
            .activatable(true)
            .selectable(false)
            .build();

        let copy_pid_btn = gtk::Button::builder()
            .icon_name("edit-copy-symbolic")
            .valign(gtk::Align::Center)
            .has_frame(false)
            .tooltip_text("Copy PID")
            .build();

        let state_clone_pid = state.clone();
        copy_pid_btn.connect_clicked(move |btn| {
            btn.stop_signal_emission_by_name("clicked");
            if let Some(display) = gtk::gdk::Display::default() {
                let clipboard = display.clipboard();
                let state_ref = state_clone_pid.borrow();

                if let Some(player) = state_ref.players.iter().find(|p| p.index == i as u8) {
                    let mut report = get_player_report(player, &state_ref);
                    report.push('\n');
                    if let Some(sid) = state_ref.session_id {
                        report.push_str(&format!("Session ID: {:08X} (Dec: {})\n", sid, sid));
                    } else {
                        report.push_str("Session ID: None\n");
                    }
                    report.push_str(&format!("Fetched at: {}\n", state_ref.fetched_at));
                    clipboard.set_text(&report);
                }
            }
        });
        player_row.add_suffix(&copy_pid_btn);

        let state_clone_details = state.clone();
        player_row.connect_activated(move |row| {
            let state_ref = state_clone_details.borrow();
            if let Some(player) = state_ref.players.iter().find(|p| p.index == i as u8) {
                let root = row.root().and_then(|r| r.downcast::<gtk::Window>().ok());
                show_player_properties(root.as_ref(), player, state_ref.network_mode);
            }
        });

        let player_row_clone = player_row.clone();
        let copy_btn_clone = copy_pid_btn.clone();

        glib::timeout_add_local(Duration::from_millis(500), move || {
            let state_ref = state_clone.borrow();

            if let Some(player) = state_ref.players.iter().find(|p| p.index == i as u8) {
                player_row_clone.set_title(&player.name);
                let id_label = if state_ref.network_mode == NetworkMode::Spacebar {
                    "SFID"
                } else {
                    "PNID"
                };
                player_row_clone.set_subtitle(&format!(
                    "PID: {} (Dec: {}), {}: {}",
                    player.pid_hex,
                    player.pid_dec,
                    id_label,
                    if player.pnid == "0" {
                        "Unknown"
                    } else {
                        &player.pnid
                    }
                ));
                copy_btn_clone.set_sensitive(true);
                player_row_clone.set_activatable(true);
            } else {
                player_row_clone.set_title(&format!("Player {}", i + 1));
                player_row_clone.set_subtitle("No data");
                copy_btn_clone.set_sensitive(false);
                player_row_clone.set_activatable(false);
            }
            glib::ControlFlow::Continue
        });

        players_group.add(&player_row);
    }

    page.add(&players_group);
    page
}
