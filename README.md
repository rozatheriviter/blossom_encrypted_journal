# blossom

A private, encrypted journal for Linux with ambient sound.

Built with Rust + GTK4 + libadwaita.

![blossom screenshot](icon.png)

## Features

- **AES-256-GCM encryption** — each journal is a single encrypted vault file; nothing is stored in plaintext
- **Multiple journals** — create as many vaults as you need, each with its own passphrase
- **Ambient noise** — white, pink, and brown noise with per-channel volume controls, always accessible even before unlocking a journal
- **MPRIS integration** — shows the currently playing track and exposes play/pause/skip controls in the bottom bar
- **Rich entries** — titles, backdatable creation dates, image and video attachments (stored encrypted inside the vault)
- **Font customization** — choose any installed system font from a searchable dropdown, set size, weight, and line height per journal
- **Accent colors** — six built-in palettes (Cherry, Wisteria, Moss, Dusk, Ember, Mono)
- **Dark / light mode** — toggle from the bottom bar, persists across sessions
- **Retrowave aesthetic** — minimal, warm-toned UI built on libadwaita

## Vault format

Vaults live at `~/.local/share/blossom/profiles/<16hexchars>.vault` as JSON:

```json
{
  "name": "My Journal",
  "salt": "<base64 scrypt salt>",
  "font_family": "Georgia",
  "font_size": 16.0,
  "font_weight": "400",
  "line_height": 1.75,
  "entries": {
    "n": "<base64 nonce>",
    "c": "<base64 AES-256-GCM ciphertext>"
  }
}
```

The `entries` blob decrypts to a JSON array of entry objects. The key is derived from the passphrase with **scrypt** (N=32768, r=8, p=1).

## Requirements

Fedora / DNFS-based Linux. Other distros need the GTK4 + libadwaita + ALSA development packages.

```
gtk4-devel  libadwaita-devel  alsa-lib-devel  dbus-devel  pkg-config
```

## Install

```bash
bash install.sh
```

The script installs system dependencies via `dnf`, builds a release binary, and places it at `~/.local/bin/blossom`. A `.desktop` entry is installed to `~/.local/share/applications/` so the app appears in GNOME Shell search.

## Build manually

```bash
cargo build --release
cp target/release/blossom ~/.local/bin/
```

## Usage

Launch with `blossom` or search for **Blossom** in GNOME Shell.

- **New Journal** — create a vault with a name and passphrase (minimum 4 characters)
- **Open** — unlock an existing vault; the sidebar shows all entries
- **+ button** in the sidebar — create a new entry
- **Font button** (Aa icon) — choose typeface, size, weight, and line height
- **Paperclip button** — attach an image or video; hover attached media to reveal the × delete button
- **Date button** — backdate an entry using a calendar popover
- **← Home** — lock the current vault and return to the home screen

## License

MIT
