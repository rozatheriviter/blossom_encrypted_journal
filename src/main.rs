mod audio;
mod mpris;
mod settings;
mod ui;
mod vault;

use libadwaita as adw;
use adw::prelude::*;

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id("com.blossom.journal")
        .flags(gio::ApplicationFlags::FLAGS_NONE)
        .build();

    app.connect_activate(|app| {
        ui::window::build_window(app);
    });

    app.run()
}
