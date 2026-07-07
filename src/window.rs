use adw::prelude::*;
use gtk::{gdk, glib};
use std::path::Path;

use crate::storage::{Store, Task};

const WINDOW_NAME: &str = "doo-main";

/// Show the main window, rebuilding the task list from storage.
pub fn present(app: &adw::Application) {
    let win = app
        .windows()
        .into_iter()
        .find(|w| w.widget_name() == WINDOW_NAME)
        .and_then(|w| w.downcast::<adw::ApplicationWindow>().ok())
        .unwrap_or_else(|| {
            let win = adw::ApplicationWindow::builder()
                .application(app)
                .title("doo")
                .default_width(480)
                .default_height(560)
                .build();
            win.set_widget_name(WINDOW_NAME);
            win
        });
    populate(&win);
    win.present();
}

/// Refresh the main window's task list if it is currently open.
pub fn refresh_if_open(app: &adw::Application) {
    if let Some(win) = app
        .windows()
        .into_iter()
        .find(|w| w.widget_name() == WINDOW_NAME)
        .and_then(|w| w.downcast::<adw::ApplicationWindow>().ok())
    {
        populate(&win);
    }
}

fn populate(win: &adw::ApplicationWindow) {
    let tasks = Store::open().all();

    let body: gtk::Widget = if tasks.is_empty() {
        adw::StatusPage::builder()
            .icon_name("checkbox-checked-symbolic")
            .title("No tasks yet")
            .description("Press Super+T anywhere to capture one")
            .build()
            .upcast()
    } else {
        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(12)
            .margin_end(12)
            .valign(gtk::Align::Start)
            .build();
        list.add_css_class("boxed-list");
        for task in &tasks {
            list.append(&task_row(win, task));
        }
        gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&list)
            .build()
            .upcast()
    };

    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&body));
    win.set_content(Some(&view));
}

fn task_row(win: &adw::ApplicationWindow, task: &Task) -> adw::ActionRow {
    let has_text = !task.text.trim().is_empty();
    let title = if has_text {
        glib::markup_escape_text(&task.text).to_string()
    } else {
        "Screenshot".to_string()
    };
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle(relative_time(&task.created_at))
        .build();

    // A task with a screenshot shows a thumbnail and opens a full preview when
    // the row is clicked.
    if let Some(path) = task.image_path.as_deref().filter(|p| Path::new(p).exists()) {
        if !has_text {
            row.add_css_class("dim-label");
        }
        row.add_prefix(&thumbnail(path, 44));
        row.set_activatable(true);
        row.connect_activated(glib::clone!(
            #[weak]
            win,
            #[strong(rename_to = path)]
            path.to_owned(),
            move |_| open_preview(&win, &path)
        ));
    }

    let delete = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .valign(gtk::Align::Center)
        .tooltip_text("Delete task")
        .build();
    delete.add_css_class("flat");
    delete.connect_clicked(glib::clone!(
        #[weak]
        win,
        #[strong(rename_to = id)]
        task.id,
        move |_| {
            Store::open().delete(id);
            populate(&win);
        }
    ));
    row.add_suffix(&delete);

    row
}

/// A square, rounded thumbnail of a screenshot for a list row.
fn thumbnail(path: &str, size: i32) -> gtk::Widget {
    let picture = gtk::Picture::builder()
        .content_fit(gtk::ContentFit::Cover)
        .width_request(size)
        .height_request(size)
        .build();
    picture.set_filename(Some(path));

    let frame = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    frame.set_overflow(gtk::Overflow::Hidden);
    frame.set_valign(gtk::Align::Center);
    frame.add_css_class("task-thumb");
    frame.append(&picture);
    frame.upcast()
}

/// Open a full-size preview of a screenshot in a modal window. Esc closes it.
fn open_preview(parent: &adw::ApplicationWindow, path: &str) {
    let picture = gtk::Picture::builder()
        .content_fit(gtk::ContentFit::Contain)
        .build();
    picture.set_filename(Some(path));

    let scroller = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&picture)
        .build();

    let view = adw::ToolbarView::new();
    view.add_top_bar(&adw::HeaderBar::new());
    view.set_content(Some(&scroller));

    let dialog = adw::Window::builder()
        .title("Screenshot")
        .transient_for(parent)
        .modal(true)
        .default_width(820)
        .default_height(640)
        .content(&view)
        .build();

    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(glib::clone!(
        #[weak]
        dialog,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, keyval, _, _| {
            if keyval == gdk::Key::Escape {
                dialog.close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    ));
    dialog.add_controller(keys);
    dialog.present();
}

fn relative_time(iso: &str) -> String {
    let Ok(dt) = glib::DateTime::from_iso8601(iso, None) else {
        return iso.to_string();
    };
    let Ok(now) = glib::DateTime::now_utc() else {
        return iso.to_string();
    };
    let secs = now.difference(&dt).as_seconds();
    match secs {
        s if s < 60 => "just now".to_string(),
        s if s < 3600 => format!("{}m ago", s / 60),
        s if s < 86_400 => format!("{}h ago", s / 3600),
        _ => dt
            .to_local()
            .ok()
            .and_then(|local| local.format("%b %e, %H:%M").ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| iso.to_string()),
    }
}
