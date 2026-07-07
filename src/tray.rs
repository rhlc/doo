use adw::prelude::*;
use gtk::glib;
use ksni::blocking::TrayMethods;
use ksni::menu::StandardItem;

use crate::{capture, window};

/// What a tray interaction asks the GTK main loop to do.
enum Msg {
    Capture,
    Show,
    Quit,
}

/// The StatusNotifierItem. Its callbacks run on ksni's own D-Bus thread, so
/// they only forward a `Msg` down the channel — all GTK work happens on the
/// main loop (see `start`), which is where the widgets and `Application` live.
struct DooTray {
    tx: async_channel::Sender<Msg>,
}

impl ksni::Tray for DooTray {
    fn id(&self) -> String {
        "dev.rahul.doo".into()
    }

    fn title(&self) -> String {
        "doo".into()
    }

    fn icon_name(&self) -> String {
        "checkbox-checked-symbolic".into()
    }

    // Left-click on the tray icon: jump straight to capture.
    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send_blocking(Msg::Capture);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        vec![
            StandardItem {
                label: "Capture task".into(),
                icon_name: "list-add-symbolic".into(),
                activate: Box::new(|t: &mut Self| {
                    let _ = t.tx.send_blocking(Msg::Capture);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Show tasks".into(),
                icon_name: "view-list-symbolic".into(),
                activate: Box::new(|t: &mut Self| {
                    let _ = t.tx.send_blocking(Msg::Show);
                }),
                ..Default::default()
            }
            .into(),
            ksni::MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|t: &mut Self| {
                    let _ = t.tx.send_blocking(Msg::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Start the tray icon for the resident instance. No-op-with-warning if no
/// StatusNotifier host is available (e.g. a GNOME without the AppIndicator
/// extension), so the app still runs headless via the Super+T shortcut.
pub fn start(app: &adw::Application) {
    let (tx, rx) = async_channel::unbounded::<Msg>();

    match (DooTray { tx }).spawn() {
        // Keep the handle alive for the whole process lifetime, like app.hold().
        Ok(handle) => std::mem::forget(handle),
        Err(e) => {
            eprintln!("doo: tray icon unavailable ({e}); running without it");
            return;
        }
    }

    let app = app.clone();
    glib::spawn_future_local(async move {
        while let Ok(msg) = rx.recv().await {
            match msg {
                Msg::Capture => capture::present(&app),
                Msg::Show => window::present(&app),
                Msg::Quit => app.quit(),
            }
        }
    });
}
