pub mod crypto;
pub mod types;

use anyhow::{Context, Result};
use chrono::Utc;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub use crypto::KEY_LEN;
pub use crypto::SALT_LEN;
use crypto::*;
use types::*;

// ──────────────────────────────────────────────────────────────────────────────
// Vault
// ──────────────────────────────────────────────────────────────────────────────

pub struct Vault {
    pub path: PathBuf,
    pub name: String,
    pub key: [u8; KEY_LEN],
    pub salt: [u8; SALT_LEN],
    pub entries: Vec<Entry>,
    pub font_family: String,
    pub font_size: f64,
    pub font_weight: String,
    pub line_height: f64,
}

impl Vault {
    /// Creates a new, empty vault at `path` with the given passphrase.
    pub fn create(path: &Path, name: &str, passphrase: &str) -> Result<Self> {
        let salt = gen_salt();
        let key = derive_key(passphrase, &salt)?;
        let v = Vault {
            path: path.to_owned(),
            name: name.to_owned(),
            key,
            salt,
            entries: Vec::new(),
            font_family: "Georgia, serif".into(),
            font_size: 16.0,
            font_weight: "400".into(),
            line_height: 1.75,
        };
        v.save()?;
        Ok(v)
    }

    /// Opens an existing vault, deriving the key from the passphrase and decrypting entries.
    pub fn open(path: &Path, passphrase: &str) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let file: VaultFile =
            serde_json::from_str(&raw).context("invalid vault JSON")?;

        let salt_vec = b64_decode(&file.salt)?;
        let salt: [u8; SALT_LEN] = salt_vec
            .try_into()
            .map_err(|_| anyhow::anyhow!("salt must be {SALT_LEN} bytes"))?;

        let key = derive_key(passphrase, &salt)?;

        let nonce = b64_decode(&file.entries.n)?;
        let ct = b64_decode(&file.entries.c)?;
        let plain = decrypt(&key, &nonce, &ct)?;
        let entries: Vec<Entry> =
            serde_json::from_slice(&plain).context("entries JSON corrupt")?;

        Ok(Vault {
            path: path.to_owned(),
            name: file.name,
            key,
            salt,
            entries,
            font_family: file.font_family,
            font_size: file.font_size,
            font_weight: file.font_weight,
            line_height: file.line_height,
        })
    }

    /// Encrypts and writes the vault to disk.
    pub fn save(&self) -> Result<()> {
        if let Some(p) = self.path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let plain = serde_json::to_vec(&self.entries)?;
        let (ct, nonce) = encrypt(&self.key, &plain)?;

        let file = VaultFile {
            name: self.name.clone(),
            salt: b64_encode(&self.salt),
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            font_weight: self.font_weight.clone(),
            line_height: self.line_height,
            entries: EncryptedBlob {
                n: b64_encode(&nonce),
                c: b64_encode(&ct),
            },
        };

        let json = serde_json::to_string_pretty(&file)?;
        std::fs::write(&self.path, json)
            .with_context(|| format!("cannot write {}", self.path.display()))
    }

    // ── Entry CRUD ──────────────────────────────────────────────────────────

    pub fn new_entry(&self) -> Entry {
        let now = Utc::now().to_rfc3339();
        Entry {
            id: Uuid::new_v4().to_string(),
            title: String::new(),
            body: String::new(),
            created: now.clone(),
            updated: now,
            media: Vec::new(),
        }
    }

    /// Prepends the entry (newest first).
    pub fn add_entry(&mut self, entry: Entry) {
        self.entries.insert(0, entry);
    }

    pub fn _upsert_entry(&mut self, entry: Entry) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.id == entry.id) {
            *e = entry;
        } else {
            self.entries.insert(0, entry);
        }
    }

    pub fn delete_entry(&mut self, id: &str) {
        self.entries.retain(|e| e.id != id);
    }

    pub fn get_entry(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn get_entry_mut(&mut self, id: &str) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    /// Returns entries matching the query (case-insensitive title + body search).
    pub fn search<'a>(&'a self, query: &str) -> Vec<&'a Entry> {
        if query.is_empty() {
            return self.entries.iter().collect();
        }
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| {
                e.title.to_lowercase().contains(&q) || e.body.to_lowercase().contains(&q)
            })
            .collect()
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Vault discovery helpers
// ──────────────────────────────────────────────────────────────────────────────

pub fn vaults_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("blossom")
        .join("profiles")
}

/// Lists all vault files on disk, returning (display_name, path) pairs.
pub fn list_vaults() -> Vec<(String, PathBuf)> {
    let dir = vaults_dir();
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    rd.filter_map(|e| {
        let e = e.ok()?;
        let path = e.path();
        if path.extension()? != "vault" {
            return None;
        }
        let raw = std::fs::read_to_string(&path).ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let name = v["name"].as_str()?.to_owned();
        Some((name, path))
    })
    .collect()
}

/// Generates a unique path for a new vault.
pub fn new_vault_path() -> PathBuf {
    let rand_hex = hex::encode(gen_salt());
    vaults_dir().join(format!("{}.vault", &rand_hex[..16]))
}
