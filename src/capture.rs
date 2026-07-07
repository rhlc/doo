use adw::prelude::*;
use gtk::{gdk, gio, glib};
use std::cell::RefCell;
use std::rc::Rc;

use crate::storage::{self, Store};
use crate::window;

const WINDOW_NAME: &str = "doo-capture";

/// A pasted screenshot held in memory until the task is saved. Keeping it as a
/// texture (not a file) means Esc or "remove" leaves no orphaned file on disk.
type Pending = Rc<RefCell<Option<gdk::Texture>>>;

/// Show the quick-capture popup. Enter saves, Esc dismisses. Ctrl+V pastes an
/// image from the clipboard as a screenshot attachment.
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

    let pending: Pending = Rc::new(RefCell::new(None));

    let entry = gtk::Entry::builder()
        .placeholder_text("Capture a task…  (Ctrl+V to paste a screenshot)")
        .hexpand(true)
        .build();
    entry.add_css_class("capture-entry");
    entry.add_css_class("flat");

    let icon = gtk::Image::from_icon_name("list-add-symbolic");
    icon.add_css_class("capture-icon");

    let input_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    input_row.append(&icon);
    input_row.append(&entry);

    // Screenshot preview: hidden until an image is pasted.
    let preview = gtk::Picture::builder()
        .content_fit(gtk::ContentFit::Contain)
        .height_request(160)
        .build();
    let thumb = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    thumb.set_overflow(gtk::Overflow::Hidden);
    thumb.add_css_class("capture-thumb");
    thumb.append(&preview);

    let remove = gtk::Button::from_icon_name("window-close-symbolic");
    remove.add_css_class("circular");
    remove.add_css_class("osd");
    remove.set_halign(gtk::Align::End);
    remove.set_valign(gtk::Align::Start);
    remove.set_margin_top(6);
    remove.set_margin_end(6);
    remove.set_tooltip_text(Some("Remove screenshot"));

    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&thumb));
    overlay.add_overlay(&remove);
    overlay.set_visible(false);

    let pill = gtk::Box::new(gtk::Orientation::Vertical, 10);
    pill.add_css_class("capture-pill");
    pill.append(&overlay);
    pill.append(&input_row);

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

    remove.connect_clicked(glib::clone!(
        #[weak]
        overlay,
        #[weak]
        preview,
        #[strong]
        pending,
        move |_| {
            pending.borrow_mut().take();
            preview.set_paintable(gdk::Paintable::NONE);
            overlay.set_visible(false);
        }
    ));

    entry.connect_activate(glib::clone!(
        #[weak]
        win,
        #[weak]
        app,
        #[strong]
        pending,
        move |entry| {
            let text = entry.text();
            let text = text.trim();
            let image_path = pending.borrow_mut().take().and_then(|t| save_texture(&t));
            if !text.is_empty() || image_path.is_some() {
                Store::open().add(text, image_path.as_deref());
                window::refresh_if_open(&app);
            }
            win.close();
        }
    ));

    let keys = gtk::EventControllerKey::new();
    // Capture phase so we intercept Ctrl+V before the entry pastes it as text.
    keys.set_propagation_phase(gtk::PropagationPhase::Capture);
    keys.connect_key_pressed(glib::clone!(
        #[weak]
        win,
        #[weak]
        overlay,
        #[weak]
        preview,
        #[strong]
        pending,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, keyval, _, state| {
            if keyval == gdk::Key::Escape {
                win.close();
                return glib::Propagation::Stop;
            }
            let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
            let is_paste = matches!(keyval, gdk::Key::v | gdk::Key::V);
            if ctrl && is_paste && clipboard_has_image(&win) {
                paste_image(&win, &overlay, &preview, &pending);
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        }
    ));
    win.add_controller(keys);

    win.present();
    entry.grab_focus();
}

fn clipboard_has_image(win: &gtk::Window) -> bool {
    let formats = win.clipboard().formats();
    formats.contains_type(gdk::Texture::static_type()) || formats.contain_mime_type("image/png")
}

fn paste_image(win: &gtk::Window, overlay: &gtk::Overlay, preview: &gtk::Picture, pending: &Pending) {
    win.clipboard().read_texture_async(
        gio::Cancellable::NONE,
        glib::clone!(
            #[weak]
            overlay,
            #[weak]
            preview,
            #[strong]
            pending,
            move |result| match result {
                Ok(Some(texture)) => {
                    preview.set_paintable(Some(&texture));
                    overlay.set_visible(true);
                    *pending.borrow_mut() = Some(texture);
                }
                Ok(None) => {}
                Err(e) => eprintln!("doo: could not paste image: {e}"),
            }
        ),
    );
}

/// Persist a pasted texture to the images directory, returning its path.
fn save_texture(texture: &gdk::Texture) -> Option<String> {
    let stamp = glib::DateTime::now_utc()
        .ok()
        .and_then(|d| d.format("%Y%m%d-%H%M%S").ok())?;
    let name = format!("{}-{:08x}.png", stamp, glib::random_int());
    let path = storage::images_dir().join(name);
    match texture.save_to_png(&path) {
        Ok(()) => Some(path.to_string_lossy().into_owned()),
        Err(e) => {
            eprintln!("doo: could not save screenshot: {e}");
            None
        }
    }
}
