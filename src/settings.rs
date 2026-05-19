use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ──────────────────────────────────────────────────────────────────────────────
// Accent presets
// ──────────────────────────────────────────────────────────────────────────────


// ──────────────────────────────────────────────────────────────────────────────
// Theme (light / dark) tokens
// ──────────────────────────────────────────────────────────────────────────────

struct ThemeTokens {
    bg:         &'static str,  // main page background
    surf:       &'static str,  // sidebar, bottom bar, cards
    input:      &'static str,  // text entry / popover bg
    fg:         &'static str,  // primary text
    fg2:        &'static str,  // secondary/muted text
    fg3:        &'static str,  // very muted (placeholders, empty state)
    border_rgb: &'static str,  // for rgba(BORDER_RGB, 0.xx) rules
}

const LIGHT: ThemeTokens = ThemeTokens {
    bg:         "#f5efe6",
    surf:       "#ede6db",
    input:      "#faf6f1",
    fg:         "#2c1f1a",
    fg2:        "#7a6b68",
    fg3:        "#b5a5a1",
    border_rgb: "100, 60, 40",
};

const DARK: ThemeTokens = ThemeTokens {
    bg:         "#1c1814",
    surf:       "#231e19",
    input:      "#2e2925",
    fg:         "#e8ddd5",
    fg2:        "#9e9087",
    fg3:        "#564e48",
    border_rgb: "200, 160, 130",
};

// ──────────────────────────────────────────────────────────────────────────────
// App-wide settings
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub dark_mode: bool,
}

impl Default for AppSettings {
    fn default() -> Self { AppSettings { dark_mode: false } }
}

impl AppSettings {
    pub fn load() -> Self {
        let path = Self::path();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(p) = path.parent() { std::fs::create_dir_all(p)?; }
        std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    fn path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("blossom")
            .join("settings.json")
    }

    pub fn css(&self) -> String {
        let t = if self.dark_mode { &DARK } else { &LIGHT };
        include_str!("../data/blossom.css")
            // accent tokens (hardcoded pink)
            .replace("BLOSSOM_ACCENT", "#c55a74")
            .replace("BLOSSOM_AL",     "#e8b4be")
            .replace("BLOSSOM_AB",     "#fef5f7")
            .replace("BLOSSOM_ARGB",   "197, 90, 116")
            // theme tokens (order matters: longer tokens first)
            .replace("BLOSSOM_BORDER_RGB", t.border_rgb)
            .replace("BLOSSOM_INPUT",  t.input)
            .replace("BLOSSOM_SURF",   t.surf)
            .replace("BLOSSOM_FG3",    t.fg3)
            .replace("BLOSSOM_FG2",    t.fg2)
            .replace("BLOSSOM_FG",     t.fg)
            .replace("BLOSSOM_BG",     t.bg)
    }
}
