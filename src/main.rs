mod capture;
mod hotkey;
mod storage;
mod tray;
mod window;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

const APP_ID: &str = "dev.rahul.doo";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();

    app.connect_startup(|app| {
        load_css();
        hotkey::ensure_registered();
        tray::start(app);
        // Stay resident with no windows open; the tray's Quit item exits.
        std::mem::forget(app.hold());
    });

    // Single-instance: GApplication forwards a second invocation's command
    // line here (the primary instance) over D-Bus, so `doo capture` from the
    // GNOME shortcut reaches the already-running process instantly.
    app.connect_command_line(|app, cmdline| {
        let args: Vec<String> = cmdline
            .arguments()
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        match args.get(1).map(String::as_str) {
            Some("capture") => capture::present(app),
            Some("--background") => {} // autostart: no window, just stay resident
            _ => window::present(app),
        }
        glib::ExitCode::SUCCESS
    });

    app.run()
}

fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("style.css"));
    gtk::style_context_add_provider_for_display(
        &gdk::Display::default().expect("no display available"),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
