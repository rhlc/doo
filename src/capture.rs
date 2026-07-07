use adw::prelude::*;
use gtk::{gdk, glib};

use crate::storage::Store;
use crate::window;

const WINDOW_NAME: &str = "doo-capture";

/// Show the quick-capture popup. Enter saves, Esc dismisses.
pub fn present(app: &adw::Application) {
    // If a capture popup is already open, just re-focus it.
    if let Some(existing) = app
        .windows()
        .into_iter()
        .find(|w| w.widget_name() == WINDOW_NAME)
    {
        existing.present();
        return;
    }

    let entry = gtk::Entry::builder()
        .placeholder_text("Capture a task…")
        .hexpand(true)
        .build();
    entry.add_css_class("capture-entry");
    entry.add_css_class("flat");

    let icon = gtk::Image::from_icon_name("list-add-symbolic");
    icon.add_css_class("capture-icon");

    let pill = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    pill.add_css_class("capture-pill");
    pill.append(&icon);
    pill.append(&entry);

    let win = gtk::Window::builder()
        .application(app)
        .title("Quick capture")
        .decorated(false)
        .resizable(false)
        .default_width(560)
        .child(&pill)
        .build();
    win.set_widget_name(WINDOW_NAME);
    win.add_css_class("capture-window");

    entry.connect_activate(glib::clone!(
        #[weak]
        win,
        #[weak]
        app,
        move |entry| {
            let text = entry.text();
            let text = text.trim();
            if !text.is_empty() {
                Store::open().add(text);
                window::refresh_if_open(&app);
            }
            win.close();
        }
    ));

    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(glib::clone!(
        #[weak]
        win,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, keyval, _, _| {
            if keyval == gdk::Key::Escape {
                win.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    ));
    win.add_controller(keys);

    win.present();
    entry.grab_focus();
}
