use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub body: String,
    pub created: String,  // RFC3339; user-editable for backdating
    pub updated: String,  // RFC3339; auto-updated on save
    #[serde(default)]
    pub media: Vec<MediaItem>,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaItem {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: MediaKind,
    pub data: String, // base64-encoded bytes
    pub mime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Image,
    Video,
}

/// On-disk vault file.
///
/// Compatible with the spec:
/// { "salt": "<b64>", "entries": { "n": "<b64 nonce>", "c": "<b64 ciphertext>" } }
#[derive(Debug, Serialize, Deserialize)]
pub struct VaultFile {
    pub name: String,
    pub salt: String,
    #[serde(default = "default_font")]
    pub font_family: String,
    #[serde(default = "default_size")]
    pub font_size: f64,
    #[serde(default = "default_weight")]
    pub font_weight: String,  // CSS weight string: "300","400","500","600","700"
    #[serde(default = "default_line_height")]
    pub line_height: f64,
    pub entries: EncryptedBlob,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedBlob {
    pub n: String,
    pub c: String,
}

fn default_font()        -> String  { "Georgia, serif".into() }
fn default_size()        -> f64     { 16.0 }
fn default_weight()      -> String  { "400".into() }
fn default_line_height() -> f64     { 1.75 }
