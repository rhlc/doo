use adw::prelude::*;
use gtk::glib;

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
    let row = adw::ActionRow::builder()
        .title(glib::markup_escape_text(&task.text))
        .subtitle(relative_time(&task.created_at))
        .build();

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
