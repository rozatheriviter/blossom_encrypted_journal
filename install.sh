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

# ── Uninstall ──────────────────────────────────────────────────────────────
if [[ "${1:-}" == "--uninstall" ]]; then
    echo "Removing blossom…"
    rm -f "$BIN_DIR/$BINARY"
    rm -f "$ICON_DIR/$APP_ID.svg"
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
