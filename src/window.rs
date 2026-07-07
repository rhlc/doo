use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::storage::{Store, Task};

const WINDOW_NAME: &str = "doo-main";

/// Show the main window, rebuilding it from storage.
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
                .default_width(1120)
                .default_height(720)
                .build();
            win.set_widget_name(WINDOW_NAME);
            win
        });
    populate(&win);
    win.present();
}

/// Refresh the main window if it is currently open.
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
    let tasks: Rc<Vec<Task>> = Rc::new(Store::open().all());

    let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    root.add_css_class("doo-root");

    root.append(&build_sidebar());

    // The right-hand detail panel, hidden until a task is selected.
    let detail = DetailPane::new(win);
    let list_pane = build_list_pane(win, &tasks, &detail);

    root.append(&list_pane);
    root.append(&detail.container);

    win.set_content(Some(&root));
}

// ---------------------------------------------------------------------------
// Sidebar (icon rail + nav panel — nav intentionally empty for now)
// ---------------------------------------------------------------------------

fn build_sidebar() -> gtk::Box {
    let rail = gtk::Box::new(gtk::Orientation::Vertical, 8);
    rail.add_css_class("icon-rail");

    let avatar = adw::Avatar::new(28, Some("doo"), true);
    avatar.set_margin_bottom(6);
    rail.append(&avatar);

    for icon in ["checkbox-checked-symbolic"] {
        let btn = gtk::Button::from_icon_name(icon);
        btn.add_css_class("flat");
        btn.set_sensitive(false);
        rail.append(&btn);
    }

    // Empty nav panel — Inbox / Lists / Filters / Tags land here later.
    let nav = gtk::Box::new(gtk::Orientation::Vertical, 0);
    nav.add_css_class("sidebar-nav");

    let sidebar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    sidebar.append(&rail);
    sidebar.append(&nav);
    sidebar
}

// ---------------------------------------------------------------------------
// Task list pane (header + add-task field + list)
// ---------------------------------------------------------------------------

fn build_list_pane(
    win: &adw::ApplicationWindow,
    tasks: &Rc<Vec<Task>>,
    detail: &DetailPane,
) -> gtk::Box {
    let pane = gtk::Box::new(gtk::Orientation::Vertical, 0);
    pane.add_css_class("list-pane");
    pane.set_hexpand(true);

    pane.append(&build_header());
    pane.append(&build_add_task(win));

    let list = gtk::ListBox::new();
    list.add_css_class("task-list");
    list.set_selection_mode(gtk::SelectionMode::Single);

    for task in tasks.iter() {
        list.append(&task_row(task));
    }

    list.connect_row_selected(glib::clone!(
        #[strong]
        tasks,
        #[strong]
        detail,
        move |_, row| match row {
            Some(row) => {
                if let Some(task) = tasks.get(row.index() as usize) {
                    detail.show_task(task);
                }
            }
            None => detail.hide(),
        }
    ));

    let scroller = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&list)
        .build();

    if tasks.is_empty() {
        let empty = adw::StatusPage::builder()
            .icon_name("checkbox-checked-symbolic")
            .title("No tasks yet")
            .description("Press Super+T anywhere to capture one")
            .vexpand(true)
            .build();
        pane.append(&empty);
    } else {
        pane.append(&scroller);
    }

    pane
}

fn build_header() -> gtk::WindowHandle {
    let hamburger = gtk::Image::from_icon_name("open-menu-symbolic");
    hamburger.add_css_class("dim-label");

    let title = gtk::Label::new(Some("Inbox"));
    title.add_css_class("list-title");

    let sort = gtk::Button::from_icon_name("view-sort-descending-symbolic");
    sort.add_css_class("flat");
    sort.set_sensitive(false);
    let more = gtk::Button::from_icon_name("view-more-symbolic");
    more.add_css_class("flat");
    more.set_sensitive(false);

    let bar = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    bar.add_css_class("list-header");
    bar.append(&hamburger);
    bar.append(&title);

    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    bar.append(&spacer);
    bar.append(&sort);
    bar.append(&more);
    bar.append(&gtk::WindowControls::new(gtk::PackType::End));

    // Wrap in a WindowHandle so the custom header is draggable (no titlebar).
    let handle = gtk::WindowHandle::new();
    handle.set_child(Some(&bar));
    handle
}

fn build_add_task(win: &adw::ApplicationWindow) -> gtk::Entry {
    let entry = gtk::Entry::builder()
        .placeholder_text("Add task")
        .primary_icon_name("list-add-symbolic")
        .build();
    entry.add_css_class("add-task");

    entry.connect_activate(glib::clone!(
        #[weak]
        win,
        move |entry| {
            let text = entry.text();
            let text = text.trim();
            if !text.is_empty() {
                Store::open().add(text, None);
                entry.set_text("");
                populate(&win);
            }
        }
    ));
    entry
}

fn task_row(task: &Task) -> gtk::ListBoxRow {
    let check = gtk::CheckButton::new();
    check.add_css_class("task-check");
    check.set_valign(gtk::Align::Center);

    let has_text = !task.text.trim().is_empty();
    let label = gtk::Label::new(None);
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    if has_text {
        label.set_text(&task.text);
        if is_linkish(&task.text) {
            label.add_css_class("task-link");
        }
    } else {
        label.set_text("Screenshot");
        label.add_css_class("dim-label");
    }

    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row_box.add_css_class("task-row");
    row_box.append(&check);
    row_box.append(&label);

    // Paperclip indicator for tasks that carry a screenshot.
    if task.image_path.as_deref().is_some_and(|p| Path::new(p).exists()) {
        let clip = gtk::Image::from_icon_name("mail-attachment-symbolic");
        clip.add_css_class("task-attach");
        row_box.append(&clip);
    }

    let row = gtk::ListBoxRow::new();
    row.set_child(Some(&row_box));
    row
}

// ---------------------------------------------------------------------------
// Detail pane
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct DetailPane {
    container: gtk::Box,
    title: gtk::Label,
    created: gtk::Label,
    image: gtk::Picture,
    image_frame: gtk::Box,
    current_id: Rc<RefCell<Option<i64>>>,
}

impl DetailPane {
    fn new(win: &adw::ApplicationWindow) -> Self {
        let current_id: Rc<RefCell<Option<i64>>> = Rc::new(RefCell::new(None));

        // Header: completion check, due-date pill, flag, and delete.
        let check = gtk::CheckButton::new();
        check.set_valign(gtk::Align::Center);

        let due = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        due.add_css_class("due-pill");
        due.append(&gtk::Image::from_icon_name("x-office-calendar-symbolic"));
        due.append(&gtk::Label::new(Some("Due Date")));

        let flag = gtk::Button::from_icon_name("emblem-important-symbolic");
        flag.add_css_class("flat");
        flag.set_sensitive(false);

        let delete = gtk::Button::from_icon_name("user-trash-symbolic");
        delete.add_css_class("flat");
        delete.set_tooltip_text(Some("Delete task"));

        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        header.add_css_class("detail-header");
        header.append(&check);
        header.append(&due);
        header.append(&spacer);
        header.append(&flag);
        header.append(&delete);

        // Body: title + large screenshot.
        let title = gtk::Label::new(None);
        title.add_css_class("detail-title");
        title.set_xalign(0.0);
        title.set_wrap(true);
        title.set_wrap_mode(gtk::pango::WrapMode::WordChar);

        let image = gtk::Picture::builder()
            .content_fit(gtk::ContentFit::Contain)
            .halign(gtk::Align::Start)
            .build();
        let image_frame = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        image_frame.set_overflow(gtk::Overflow::Hidden);
        image_frame.add_css_class("detail-image");
        image_frame.set_halign(gtk::Align::Start);
        image_frame.append(&image);

        let created = gtk::Label::new(None);
        created.add_css_class("detail-created");
        created.add_css_class("dim-label");
        created.set_xalign(0.0);

        let body = gtk::Box::new(gtk::Orientation::Vertical, 14);
        body.add_css_class("detail-body");
        body.append(&title);
        body.append(&created);
        body.append(&image_frame);

        let body_scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .vexpand(true)
            .child(&body)
            .build();

        // Footer: task location.
        let footer = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        footer.add_css_class("detail-footer");
        footer.append(&gtk::Image::from_icon_name("mail-inbox-symbolic"));
        footer.append(&gtk::Label::new(Some("Inbox")));

        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        container.add_css_class("detail-pane");
        container.append(&header);
        container.append(&body_scroll);
        container.append(&footer);
        container.set_visible(false);

        delete.connect_clicked(glib::clone!(
            #[weak]
            win,
            #[strong]
            current_id,
            move |_| {
                if let Some(id) = *current_id.borrow() {
                    Store::open().delete(id);
                    populate(&win);
                }
            }
        ));

        Self {
            container,
            title,
            created,
            image,
            image_frame,
            current_id,
        }
    }

    fn show_task(&self, task: &Task) {
        *self.current_id.borrow_mut() = Some(task.id);
        let has_text = !task.text.trim().is_empty();
        self.title.set_text(if has_text { &task.text } else { "Screenshot" });
        self.created
            .set_text(&format!("Captured {}", relative_time(&task.created_at)));

        match task.image_path.as_deref().filter(|p| Path::new(p).exists()) {
            Some(path) => {
                self.image.set_filename(Some(path));
                self.image_frame.set_visible(true);
            }
            None => self.image_frame.set_visible(false),
        }
        self.container.set_visible(true);
    }

    fn hide(&self) {
        *self.current_id.borrow_mut() = None;
        self.container.set_visible(false);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

/// Rough heuristic: does this text look like a bare URL/domain?
fn is_linkish(text: &str) -> bool {
    let t = text.trim();
    if t.starts_with("http://") || t.starts_with("https://") {
        return true;
    }
    !t.contains(char::is_whitespace)
        && t.contains('.')
        && t.rsplit('.').next().is_some_and(|tld| {
            tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic())
        })
}
