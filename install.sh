#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# Blossom — Fedora install script
# Installs system deps, builds with Cargo, installs binary + assets.
# Usage: bash install.sh [--uninstall]
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

APP_ID="com.blossom.journal"
BINARY="blossom"
PREFIX="${PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
DATA_DIR="$PREFIX/share"

ICON_DIR="$DATA_DIR/icons/hicolor/scalable/apps"
DESKTOP_DIR="$DATA_DIR/applications"
ICON_SRC="appicons/Assets.xcassets/AppIcon.appiconset"

# ── Uninstall ──────────────────────────────────────────────────────────────
if [[ "${1:-}" == "--uninstall" ]]; then
    echo "Removing blossom…"
    rm -f "$BIN_DIR/$BINARY"
    rm -f "$ICON_DIR/$APP_ID.svg"
    for size in 16 32 48 64 128 256 512; do
        rm -f "$DATA_DIR/icons/hicolor/${size}x${size}/apps/$APP_ID.png"
    done
    rm -f "$DESKTOP_DIR/$APP_ID.desktop"
    gtk-update-icon-cache -f -t "$DATA_DIR/icons/hicolor" 2>/dev/null || true
    update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true
    echo "Done."
    exit 0
fi

# ── System dependencies (Fedora / dnf) ─────────────────────────────────────
echo "Installing system dependencies…"
sudo dnf install -y \
    gtk4-devel \
    libadwaita-devel \
    glib2-devel \
    alsa-lib-devel \
    dbus-devel \
    pkg-config \
    cargo \
    rust \
    git \
    desktop-file-utils \
    librsvg2-tools

# PipeWire ALSA support (ensures cpal finds an output device)
sudo dnf install -y pipewire-alsa || true

# ── Build ───────────────────────────────────────────────────────────────────
echo ""
echo "Building blossom (release)…"
cargo build --release

# ── Install ─────────────────────────────────────────────────────────────────
echo ""
echo "Installing to $PREFIX…"
mkdir -p "$BIN_DIR" "$ICON_DIR" "$DESKTOP_DIR"

cp -f "target/release/$BINARY"                  "$BIN_DIR/$BINARY"
cp -f "data/icons/$APP_ID.svg"                  "$ICON_DIR/$APP_ID.svg"
cp -f "data/$APP_ID.desktop"                    "$DESKTOP_DIR/$APP_ID.desktop"

# Install PNG icons at standard hicolor sizes
for size in 16 32 48 64 128 256 512; do
    dest="$DATA_DIR/icons/hicolor/${size}x${size}/apps"
    mkdir -p "$dest"
    cp -f "$ICON_SRC/${size}.png" "$dest/$APP_ID.png"
done

# Validate and update caches
if command -v desktop-file-validate &>/dev/null; then
    desktop-file-validate "$DESKTOP_DIR/$APP_ID.desktop" || true
fi
gtk-update-icon-cache -f -t "$DATA_DIR/icons/hicolor" 2>/dev/null || true
update-desktop-database "$DESKTOP_DIR" 2>/dev/null || true

echo ""
echo "✓ blossom installed."
echo "  Binary  : $BIN_DIR/$BINARY"
echo "  Desktop : $DESKTOP_DIR/$APP_ID.desktop"
echo ""
echo "Run with:  blossom"
echo "           (or search for 'Blossom' in GNOME Shell)"
