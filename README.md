# doo

Capture a task the moment it crosses your mind, without leaving what you're
doing.

doo is a tiny GNOME/Ubuntu app that sits in the background. Press
**Super+T** anywhere and a small entry pops up:

- type the task and hit **Enter** — saved
- **Ctrl+V** — paste a screenshot to attach it (with an optional note)
- **Esc** — never mind

Open **doo** from the app grid whenever you want to see what you captured,
or use the **tray icon**: left-click to capture, right-click for Show tasks /
Quit. Tasks with a screenshot show a thumbnail — click the row to view it
full-size. Delete a task from the list with its trash button.

That's the whole app (for now — more in later phases).

## Install

Grab the `.deb` from the releases (or build it yourself, below), then:

```bash
sudo apt install ./doo_0.0.1-1_amd64.deb
```

Launch **doo** once (or just log out and back in). The first launch:

- registers the **Super+T** shortcut for you (it won't steal the combo if
  you already use it for something else — check *Settings → Keyboard →
  Custom Shortcuts* in that case)
- leaves a background instance running; it also autostarts on every login

Requirements: Ubuntu 24.04+ (or any Linux with GNOME 45+, GTK 4.12+ and
libadwaita 1.5+). Wayland and X11 both work. The tray icon uses the
StatusNotifierItem standard — Ubuntu ships the needed AppIndicator support by
default; on vanilla GNOME install the "AppIndicator and KStatusNotifierItem
Support" extension. Without it the app still runs fine via Super+T.

## Where your tasks live

A plain SQLite file: `~/.local/share/doo/doo.db`, with pasted screenshots
alongside in `~/.local/share/doo/images/`. Yours to back up, sync, or query:

```bash
sqlite3 ~/.local/share/doo/doo.db 'SELECT * FROM tasks;'
```

## Build from source

```bash
# toolchain (once)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev
cargo install cargo-deb

# build the package
cargo deb
sudo apt install ./target/debian/doo_*.deb
```

First build takes a few minutes (it compiles the GTK bindings); after that,
seconds.

## Hacking on it

```bash
cargo test                  # storage unit tests
cargo run -- --background   # start the resident instance
cargo run -- capture        # pop the capture entry (forwards via D-Bus)
cargo run                   # open the task list
```

The app is single-instance: any later invocation is forwarded to the running
process over D-Bus, which is exactly how the global shortcut reaches it.
A dev run registers the shortcut against your build directory; installing the
.deb self-heals it to `/usr/bin/doo` next time the installed binary starts.

Code map:

| File             | Does                                                    |
| ---------------- | ------------------------------------------------------- |
| `src/main.rs`    | app entry, CLI routing, stays resident                  |
| `src/capture.rs` | the quick-capture popup                                 |
| `src/window.rs`  | the task list window                                    |
| `src/storage.rs` | SQLite store (`add`, `all`, `delete`) — extend here for new phases |
| `src/hotkey.rs`  | self-registers the Super+T GNOME shortcut               |
| `src/tray.rs`    | StatusNotifierItem tray icon + menu                     |

## CLI

| Command            | Effect                                     |
| ------------------ | ------------------------------------------ |
| `doo`              | Open the task list                         |
| `doo capture`      | Show the quick-capture popup               |
| `doo --background` | Start resident with no window (autostart)  |
