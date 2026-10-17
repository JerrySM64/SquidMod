use adw::prelude::*;
use anyhow::Result;
use gtk4::gio;
use libadwaita as adw;

pub fn run_gui() -> Result<()> {
    #[cfg(target_os = "macos")]
    let app_flags = gio::ApplicationFlags::HANDLES_OPEN;
    #[cfg(not(target_os = "macos"))]
    let app_flags = gio::ApplicationFlags::FLAGS_NONE;

    let app = adw::Application::builder()
        .application_id("dev.jerrysm64.squidmod")
        .flags(app_flags)
        .build();

    app.connect_activate(crate::gui::main_window::build_ui);
    app.run();
    Ok(())
}
