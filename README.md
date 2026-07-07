# doo

Minimalist quick task capture for GNOME/Ubuntu. Runs in the background;
press **Super+T** anywhere to pop up a capture entry — type the task, hit
Enter, done. Open **doo** from the app grid to review captured tasks.

## How it works

- Rust + GTK4 + libadwaita; single-instance `GApplication` (`dev.rahul.doo`).
- On Wayland apps can't grab global hotkeys, so on first launch doo registers
  a GNOME custom keyboard shortcut (**Super+T** → `doo capture`). The shortcut
  invocation is forwarded to the running background instance over D-Bus.
- Tasks are stored in SQLite at `~/.local/share/doo/doo.db`.
- An autostart entry (`/etc/xdg/autostart`) launches `doo --background` at login.

## Build & install

Build dependencies (Ubuntu):

```bash
sudo apt install build-essential pkg-config libgtk-4-dev libadwaita-1-dev
cargo install cargo-deb   # once
```

Build the package and install it:

```bash
cargo deb
sudo apt install ./target/debian/doo_*.deb
```

Then launch `doo` once (or re-login) — that registers the Super+T shortcut
and leaves the background instance running.

## Development

```bash
cargo test                  # storage unit tests
cargo run -- --background   # start resident instance
cargo run -- capture        # trigger the capture popup (forwards via D-Bus)
cargo run                   # open the main window
```

Note: a dev run registers the shortcut against the dev binary path; installing
the .deb later self-heals the shortcut to `/usr/bin/doo`.

## CLI

| Command            | Effect                                   |
| ------------------ | ---------------------------------------- |
| `doo`              | Open the main window (task list)         |
| `doo capture`      | Show the quick-capture popup             |
| `doo --background` | Start resident with no window (autostart) |
