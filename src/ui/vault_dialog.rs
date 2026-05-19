use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

// ──────────────────────────────────────────────────────────────────────────────
// Create a new journal
// ──────────────────────────────────────────────────────────────────────────────

/// Presents a dialog and calls `cb` with (name, passphrase) on success.
pub fn show_create<F>(parent: &adw::ApplicationWindow, cb: F)
where
    F: Fn(String, String) + 'static,
{
    let dialog = adw::MessageDialog::new(Some(parent), Some("New Journal"), None);
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("create", "Create");
    dialog.set_response_appearance("create", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("create"));
    dialog.set_close_response("cancel");

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    content.set_margin_start(8);
    content.set_margin_end(8);
    content.set_margin_bottom(8);

    let name_entry = gtk4::Entry::builder()
        .placeholder_text("Journal name")
        .build();

    let pass_entry = gtk4::PasswordEntry::builder()
        .placeholder_text("Passphrase")
        .show_peek_icon(true)
        .build();

    let confirm_entry = gtk4::PasswordEntry::builder()
        .placeholder_text("Confirm passphrase")
        .show_peek_icon(true)
        .build();

    let error_label = gtk4::Label::builder()
        .label("")
        .css_classes(["error"])
        .halign(gtk4::Align::Start)
        .visible(false)
        .build();

    content.append(&name_entry);
    content.append(&pass_entry);
    content.append(&confirm_entry);
    content.append(&error_label);

    dialog.set_extra_child(Some(&content));

    // Enable Create button only when name is non-empty
    {
        let dialog2 = dialog.clone();
        let dialog3 = dialog.clone();
        let name2 = name_entry.clone();
        name_entry.connect_changed(move |e| {
            dialog2.set_response_enabled("create", !e.text().is_empty());
        });
        dialog3.set_response_enabled("create", !name2.text().is_empty());
    }

    let name_entry2 = name_entry.clone();
    let pass_entry2 = pass_entry.clone();
    let confirm2 = confirm_entry.clone();
    let error2 = error_label.clone();

    dialog.connect_response(None, move |dlg, response| {
        if response != "create" {
            dlg.close();
            return;
        }
        let name = name_entry2.text().to_string();
        let pass = pass_entry2.text().to_string();
        let confirm = confirm2.text().to_string();

        if pass.len() < 4 {
            error2.set_label("Passphrase must be at least 4 characters.");
            error2.set_visible(true);
            return;
        }
        if pass != confirm {
            error2.set_label("Passphrases do not match.");
            error2.set_visible(true);
            return;
        }
        dlg.close();
        cb(name, pass);
    });

    dialog.present();
}

// ──────────────────────────────────────────────────────────────────────────────
// Unlock an existing vault
// ──────────────────────────────────────────────────────────────────────────────

/// Presents a passphrase prompt and calls `cb` with the passphrase on success.
pub fn show_unlock<F>(
    parent: &adw::ApplicationWindow,
    vault_name: &str,
    cb: F,
)
where
    F: Fn(String) + 'static,
{
    let heading = format!("Unlock \"{vault_name}\"");
    let dialog = adw::MessageDialog::new(Some(parent), Some(&heading), None);
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("unlock", "Unlock");
    dialog.set_response_appearance("unlock", adw::ResponseAppearance::Suggested);
    dialog.set_default_response(Some("unlock"));
    dialog.set_close_response("cancel");

    let content = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    content.set_margin_start(8);
    content.set_margin_end(8);
    content.set_margin_bottom(8);

    let pass_entry = gtk4::PasswordEntry::builder()
        .placeholder_text("Passphrase")
        .show_peek_icon(true)
        .build();

    let error_label = gtk4::Label::builder()
        .label("")
        .css_classes(["error"])
        .halign(gtk4::Align::Start)
        .visible(false)
        .build();

    content.append(&pass_entry);
    content.append(&error_label);
    dialog.set_extra_child(Some(&content));

    // allow Enter to confirm
    {
        let dialog2 = dialog.clone();
        pass_entry.connect_activate(move |_| {
            dialog2.response("unlock");
        });
    }

    let pass2 = pass_entry.clone();

    dialog.connect_response(None, move |dlg, response| {
        if response != "unlock" {
            dlg.close();
            return;
        }
        let pass = pass2.text().to_string();
        dlg.close();
        cb(pass);
    });

    dialog.present();
}

// ──────────────────────────────────────────────────────────────────────────────
// Generic error toast
// ──────────────────────────────────────────────────────────────────────────────

pub fn show_error(parent: &adw::ApplicationWindow, msg: &str) {
    let dialog = adw::MessageDialog::new(Some(parent), Some("Error"), Some(msg));
    dialog.add_response("ok", "OK");
    dialog.set_default_response(Some("ok"));
    dialog.connect_response(None, |d, _| d.close());
    dialog.present();
}

