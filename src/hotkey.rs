use gtk::gio;
use gtk::prelude::*;

const MEDIA_KEYS_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const KEYBINDING_SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
const KEYBINDING_PATH: &str =
    "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/doo-capture/";
const BINDING: &str = "<Super>t";

/// Register the global capture shortcut as a GNOME custom keybinding.
///
/// A .deb can't touch per-user dconf, and on Wayland apps can't grab global
/// hotkeys themselves, so the app registers (and keeps up to date) its own
/// GNOME shortcut that runs `<this binary> capture`. Idempotent.
pub fn ensure_registered() {
    let Some(schemas) = gio::SettingsSchemaSource::default() else {
        return;
    };
    if schemas.lookup(MEDIA_KEYS_SCHEMA, true).is_none()
        || schemas.lookup(KEYBINDING_SCHEMA, true).is_none()
    {
        eprintln!("doo: GNOME media-keys schemas not found; set up a shortcut for 'doo capture' manually");
        return;
    }

    let command = match std::env::current_exe() {
        Ok(exe) => format!("{} capture", exe.display()),
        Err(_) => "doo capture".to_string(),
    };

    let media_keys = gio::Settings::new(MEDIA_KEYS_SCHEMA);
    let mut paths: Vec<String> = media_keys
        .strv("custom-keybindings")
        .iter()
        .map(|s| s.to_string())
        .collect();

    if !paths.iter().any(|p| p == KEYBINDING_PATH) {
        // Don't steal the binding if another custom shortcut already uses it.
        for path in &paths {
            let other = gio::Settings::with_path(KEYBINDING_SCHEMA, path);
            if other.string("binding") == BINDING {
                eprintln!(
                    "doo: {BINDING} is already bound to '{}'; skipping shortcut registration",
                    other.string("name")
                );
                return;
            }
        }
        paths.push(KEYBINDING_PATH.to_string());
        let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
        media_keys
            .set_strv("custom-keybindings", paths)
            .expect("failed to update custom-keybindings list");
    }

    // Create or update our entry (self-heals if the binary moved, e.g. a dev
    // build registered first and the .deb was installed later).
    let ours = gio::Settings::with_path(KEYBINDING_SCHEMA, KEYBINDING_PATH);
    if ours.string("command") != command {
        ours.set_string("name", "doo quick capture").ok();
        ours.set_string("command", &command).ok();
        ours.set_string("binding", BINDING).ok();
        gio::Settings::sync();
        println!("doo: registered global shortcut {BINDING} -> {command}");
    }
}
