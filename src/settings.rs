use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ──────────────────────────────────────────────────────────────────────────────
// Accent presets
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Accent {
    #[default]
    Cherry,
    Wisteria,
    Moss,
    Dusk,
    Ember,
    Mono,
}

pub struct AccentColors {
    pub hex:   &'static str,
    pub light: &'static str,
    pub bg:    &'static str,
    pub rgb:   &'static str,
    pub label: &'static str,
}

impl Accent {
    pub fn colors(self) -> AccentColors {
        match self {
            Accent::Cherry   => AccentColors { hex: "#c55a74", light: "#e8b4be", bg: "#fef5f7", rgb: "197, 90, 116",  label: "Cherry" },
            Accent::Wisteria => AccentColors { hex: "#7b5ea7", light: "#c3b0dc", bg: "#f7f3fc", rgb: "123, 94, 167",  label: "Wisteria" },
            Accent::Moss     => AccentColors { hex: "#4a7c59", light: "#9fc4a8", bg: "#f2f8f4", rgb: "74, 124, 89",   label: "Moss" },
            Accent::Dusk     => AccentColors { hex: "#4a6fa5", light: "#9eb8d9", bg: "#f2f5fb", rgb: "74, 111, 165",  label: "Dusk" },
            Accent::Ember    => AccentColors { hex: "#c05c2a", light: "#e4b49a", bg: "#fdf4ee", rgb: "192, 92, 42",   label: "Ember" },
            Accent::Mono     => AccentColors { hex: "#505050", light: "#b0b0b0", bg: "#f5f5f5", rgb: "80, 80, 80",    label: "Mono" },
        }
    }

    pub const ALL: &'static [Accent] = &[
        Accent::Cherry, Accent::Wisteria, Accent::Moss,
        Accent::Dusk, Accent::Ember, Accent::Mono,
    ];
}

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
    pub accent: Accent,
    #[serde(default)]
    pub dark_mode: bool,
}

impl Default for AppSettings {
    fn default() -> Self { AppSettings { accent: Accent::Cherry, dark_mode: false } }
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
        let a = self.accent.colors();
        let t = if self.dark_mode { &DARK } else { &LIGHT };
        include_str!("../data/blossom.css")
            // accent tokens
            .replace("BLOSSOM_ACCENT", a.hex)
            .replace("BLOSSOM_AL",     a.light)
            .replace("BLOSSOM_AB",     a.bg)
            .replace("BLOSSOM_ARGB",   a.rgb)
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
