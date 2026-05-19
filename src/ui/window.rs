use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use base64::Engine as _;
use chrono::Datelike;
use gtk4::prelude::*;
use libadwaita as adw;
use adw::prelude::*;

use crate::audio::{NoiseMix, NoiseEngine};
use crate::mpris::MprisWatcher;
use crate::settings::{Accent, AppSettings};
use crate::vault::{self, Vault};
use crate::vault::types::MediaItem;
use crate::ui::vault_dialog as dlg;

struct AppState {
    vault: Option<Vault>,
    current_entry_id: Option<String>,
    search_query: String,
    suppress_change: bool,
    settings: AppSettings,
}

impl AppState {
    fn new() -> Self {
        AppState {
            vault: None,
            current_entry_id: None,
            search_query: String::new(),
            suppress_change: false,
            settings: AppSettings::load(),
        }
    }
}

pub fn build_window(app: &adw::Application) {
    let state: Rc<RefCell<AppState>> = Rc::new(RefCell::new(AppState::new()));
    let save_timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    apply_color_scheme(state.borrow().settings.dark_mode);
    let css_provider = gtk4::CssProvider::new();
    css_provider.load_from_string(&state.borrow().settings.css());
    gtk4::style_context_add_provider_for_display(
        &gdk4::Display::default().unwrap(),
        &css_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let noise = Rc::new(NoiseEngine::new());
    let mpris = Rc::new(MprisWatcher::start());

    // ── Window ─────────────────────────────────────────────────────────────
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("blossom")
        .default_width(1100)
        .default_height(720)
        .build();
    window.set_size_request(480, 360);
    window.set_resizable(true);

    // ── Header — title + vault chip + home button only ──────────────────────
    let header = adw::HeaderBar::new();

    let sidebar_toggle = gtk4::ToggleButton::builder()
        .icon_name("sidebar-show-symbolic")
        .visible(false)
        .build();
    header.pack_start(&sidebar_toggle);

    let title_label = gtk4::Label::builder()
        .label("blossom")
        .css_classes(["blossom-title"])
        .build();
    header.set_title_widget(Some(&title_label));

    let vault_chip = gtk4::Label::builder()
        .label("")
        .css_classes(["vault-chip"])
        .visible(false)
        .build();
    header.pack_start(&vault_chip);

    let home_btn = gtk4::Button::builder()
        .label("← Home")
        .css_classes(["home-button"])
        .visible(false)
        .build();
    header.pack_start(&home_btn);

    // ── Stacks ─────────────────────────────────────────────────────────────
    let main_stack = gtk4::Stack::new();
    main_stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    main_stack.set_transition_duration(180);

    let home_page = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    home_page.add_css_class("home-page");
    home_page.set_vexpand(true);
    main_stack.add_named(&home_page, Some("home"));

    let split_view = adw::OverlaySplitView::new();
    split_view.set_min_sidebar_width(260.0);
    split_view.set_max_sidebar_width(320.0);

    let sidebar = build_sidebar();
    split_view.set_sidebar(Some(&sidebar.outer));
    let editor = build_editor();
    split_view.set_content(Some(&editor.outer));

    // Editor page is just the split_view — bottom bar lives in ToolbarView now
    let editor_page = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    editor_page.set_vexpand(true);
    editor_page.set_hexpand(true);
    editor_page.append(&split_view);
    main_stack.add_named(&editor_page, Some("editor"));

    // Bottom bar is always visible (outside the stack)
    let bottom_bar = build_bottom_bar();

    let root_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.set_vexpand(true);
    toolbar_view.set_hexpand(true);
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&main_stack));

    sidebar_toggle.bind_property("active", &split_view, "show-sidebar")
        .bidirectional()
        .sync_create()
        .build();

    root_box.append(&toolbar_view);
    root_box.append(&bottom_bar.outer);
    window.set_content(Some(&root_box));

    let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        820.0,
        adw::LengthUnit::Px,
    ));
    breakpoint.add_setter(&root_box, "orientation", Some(&gtk4::Orientation::Horizontal.to_value()));
    breakpoint.add_setter(&split_view, "collapsed", Some(&true.to_value()));
    breakpoint.add_setter(&sidebar_toggle, "visible", Some(&true.to_value()));
    breakpoint.add_setter(&title_label, "visible", Some(&false.to_value()));
    breakpoint.add_setter(&bottom_bar.sep_h, "visible", Some(&false.to_value()));
    breakpoint.add_setter(&bottom_bar.sep_v, "visible", Some(&true.to_value()));
    breakpoint.add_setter(&bottom_bar.inner, "orientation", Some(&gtk4::Orientation::Vertical.to_value()));
    breakpoint.add_setter(&bottom_bar.inner, "width-request", Some(&60.to_value()));
    breakpoint.add_setter(&bottom_bar.inner, "margin-start", Some(&0.to_value()));
    breakpoint.add_setter(&bottom_bar.inner, "margin-end", Some(&0.to_value()));
    breakpoint.add_setter(&bottom_bar.noise_box, "orientation", Some(&gtk4::Orientation::Vertical.to_value()));
    breakpoint.add_setter(&bottom_bar.noise_box, "spacing", Some(&12.to_value()));
    breakpoint.add_setter(&bottom_bar.config_box, "orientation", Some(&gtk4::Orientation::Vertical.to_value()));
    breakpoint.add_setter(&bottom_bar.mpris_box, "orientation", Some(&gtk4::Orientation::Vertical.to_value()));
    breakpoint.add_setter(&bottom_bar.mpris_btns, "orientation", Some(&gtk4::Orientation::Vertical.to_value()));
    breakpoint.add_setter(&bottom_bar.track_label, "max-width-chars", Some(&6.to_value()));
    breakpoint.add_setter(&bottom_bar.artist_label, "max-width-chars", Some(&6.to_value()));
    breakpoint.add_setter(&bottom_bar.spacer, "visible", Some(&false.to_value()));
    breakpoint.add_setter(&editor.body, "left-margin", Some(&12.to_value()));
    breakpoint.add_setter(&editor.body, "right-margin", Some(&12.to_value()));
    breakpoint.add_setter(&editor.title, "margin-start", Some(&12.to_value()));
    window.add_breakpoint(breakpoint);

    // ── Closures (defined in dependency order) ──────────────────────────────

    // 1. refresh_list
    let refresh_list: Rc<dyn Fn()> = {
        let state  = Rc::clone(&state);
        let list   = sidebar.list.clone();
        let estack = editor.stack.clone();
        Rc::new(move || {
            let st = state.borrow();
            while let Some(c) = list.first_child() { list.remove(&c); }
            let Some(vault) = &st.vault else { return; };
            let entries = vault.search(&st.search_query);
            let current = st.current_entry_id.clone();
            if entries.is_empty() {
                estack.set_visible_child_name("empty");
                return;
            }
            let rows: Vec<_> = entries.iter().map(|e| (make_entry_row(e), e.id.clone())).collect();
            drop(st);
            for (row, id) in rows {
                if Some(&id) == current.as_ref() { row.add_css_class("selected"); }
                list.append(&row);
            }
        })
    };

    // 2. load_entry
    let load_entry: Rc<dyn Fn(&str)> = {
        let state    = Rc::clone(&state);
        let estack   = editor.stack.clone();
        let etitle   = editor.title.clone();
        let ebody    = editor.body.clone();
        let mbox     = editor.media_box.clone();
        let mstrip   = editor.media_strip.clone();
        let date_btn = editor.date_btn.clone();
        Rc::new(move |id: &str| {
            let data = {
                let st = state.borrow();
                let Some(v) = &st.vault else { return; };
                let Some(e) = v.get_entry(id) else { return; };
                Some((e.title.clone(), e.body.clone(), e.created.clone(),
                      e.media.clone(), v.font_family.clone(), v.font_size,
                      v.font_weight.clone(), v.line_height))
            };
            let Some((title, body, created, media, fam, sz, wt, lh)) = data else { return; };
            estack.set_visible_child_name("editor");
            state.borrow_mut().suppress_change = true;
            etitle.set_text(&title);
            ebody.buffer().set_text(&body);
            apply_font(&ebody, &fam, sz, &wt, lh);
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&created) {
                date_btn.set_label(&dt.format("%-d %b %Y").to_string());
            }
            state.borrow_mut().suppress_change = false;
            while let Some(c) = mbox.first_child() { mbox.remove(&c); }
            for item in &media {
                if let Some(t) = make_deletable_thumb(item, Rc::clone(&state), id.to_string(), item.id.clone(), mstrip.clone()) {
                    mbox.append(&t);
                }
            }
            mstrip.set_visible(!media.is_empty());
        })
    };

    // 3. do_save
    let do_save: Rc<dyn Fn()> = {
        let state  = Rc::clone(&state);
        let etitle = editor.title.clone();
        let ebody  = editor.body.clone();
        Rc::new(move || {
            let (suppress, id) = {
                let st = state.borrow();
                (st.suppress_change, st.current_entry_id.clone())
            };
            if suppress { return; }
            let Some(id) = id else { return; };
            let title = etitle.text().to_string();
            let body = {
                let buf = ebody.buffer();
                buf.text(&buf.start_iter(), &buf.end_iter(), false).to_string()
            };
            let mut st = state.borrow_mut();
            if let Some(vault) = st.vault.as_mut() {
                if let Some(entry) = vault.get_entry_mut(&id) {
                    entry.title   = title;
                    entry.body    = body;
                    entry.updated = chrono::Utc::now().to_rfc3339();
                }
                let _ = vault.save();
            }
        })
    };

    // 4. refresh_home (holder breaks the circular dependency with delete buttons)
    let rh_holder: Rc<RefCell<Option<Rc<dyn Fn()>>>> = Rc::new(RefCell::new(None));

    let refresh_home: Rc<dyn Fn()> = {
        let state        = Rc::clone(&state);
        let home_page    = home_page.clone();
        let main_stack   = main_stack.clone();
        let editor_stack = editor.stack.clone();
        let vault_chip   = vault_chip.clone();
        let home_btn     = home_btn.clone();
        let window       = window.clone();
        let refresh_list = Rc::clone(&refresh_list);
        let rh_holder    = Rc::clone(&rh_holder);
        let split_view   = split_view.clone();

        Rc::new(move || {
            while let Some(c) = home_page.first_child() { home_page.remove(&c); }

            let centre = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
            centre.set_halign(gtk4::Align::Center);
            centre.set_valign(gtk4::Align::Start);
            centre.set_margin_top(56);
            centre.set_margin_bottom(56);
            centre.set_margin_start(24);
            centre.set_margin_end(24);

            centre.append(&gtk4::Label::builder()
                .label("blossom").css_classes(["home-title"]).build());
            centre.append(&gtk4::Label::builder()
                .label("your private journal").css_classes(["home-subtitle"]).margin_top(6).build());

            let vaults = vault::list_vaults();
            if !vaults.is_empty() {
                centre.append(&gtk4::Label::builder()
                    .label("JOURNALS").css_classes(["home-section-label"])
                    .halign(gtk4::Align::Start).margin_top(40).margin_bottom(10).build());

                for (name, path) in vaults {
                    let card = build_journal_card(
                        &name, &path,
                        Rc::clone(&state),
                        Rc::clone(&refresh_list),
                        Rc::clone(&rh_holder),
                        main_stack.clone(),
                        editor_stack.clone(),
                        vault_chip.clone(),
                        home_btn.clone(),
                        window.clone(),
                        split_view.clone(),
                    );
                    centre.append(&card);
                }
            }

            let new_btn = gtk4::Button::builder()
                .label("+ New Journal").css_classes(["new-journal-button"])
                .margin_top(28).halign(gtk4::Align::Center).build();
            {
                let state2 = Rc::clone(&state);
                let ms     = main_stack.clone();
                let es     = editor_stack.clone();
                let vc     = vault_chip.clone();
                let hb     = home_btn.clone();
                let rl     = Rc::clone(&refresh_list);
                let win    = window.clone();
                let win2   = window.clone();
                let sv     = split_view.clone();
                new_btn.connect_clicked(move |_| {
                    wire_create_journal(
                        &win, Rc::clone(&state2), ms.clone(), es.clone(),
                        vc.clone(), hb.clone(), Rc::clone(&rl), win2.clone(),
                        sv.clone(),
                    );
                });
            }
            centre.append(&new_btn);

            home_page.append(&gtk4::ScrolledWindow::builder()
                .hscrollbar_policy(gtk4::PolicyType::Never)
                .vscrollbar_policy(gtk4::PolicyType::Automatic)
                .vexpand(true).child(&centre).build());
        })
    };

    *rh_holder.borrow_mut() = Some(Rc::clone(&refresh_home));

    // ── Wire: Home button ──────────────────────────────────────────────────
    {
        let ms2  = main_stack.clone();
        let vc2  = vault_chip.clone();
        let hb2  = home_btn.clone();
        let rh2  = Rc::clone(&refresh_home);
        home_btn.connect_clicked(move |_| {
            vc2.set_visible(false);
            hb2.set_visible(false);
            rh2();
            ms2.set_visible_child_name("home");
        });
    }

    // ── Wire: New entry ────────────────────────────────────────────────────
    {
        let state = Rc::clone(&state);
        let rl    = Rc::clone(&refresh_list);
        let le    = Rc::clone(&load_entry);
        sidebar.new_btn.connect_clicked(move |_| {
            let id = {
                let mut st = state.borrow_mut();
                let Some(v) = st.vault.as_mut() else { return; };
                let e = v.new_entry();
                let id = e.id.clone();
                v.add_entry(e);
                let _ = v.save();
                id
            };
            state.borrow_mut().current_entry_id = Some(id.clone());
            rl();
            le(&id);
        });
    }

    // ── Wire: Search ───────────────────────────────────────────────────────
    {
        let state = Rc::clone(&state);
        let rl    = Rc::clone(&refresh_list);
        sidebar.search.connect_search_changed(move |se| {
            state.borrow_mut().search_query = se.text().to_string();
            rl();
        });
    }

    // ── Wire: Entry row selected ───────────────────────────────────────────
    {
        let state = Rc::clone(&state);
        let le    = Rc::clone(&load_entry);
        let save  = Rc::clone(&do_save);
        sidebar.list.connect_row_selected(move |_, row| {
            let Some(row) = row else { return; };
            let id = row.widget_name().to_string();
            save();
            state.borrow_mut().current_entry_id = Some(id.clone());
            le(&id);
        });
    }

    // ── Wire: Auto-save ────────────────────────────────────────────────────
    {
        let save  = Rc::clone(&do_save);
        let rl    = Rc::clone(&refresh_list);
        let timer = Rc::clone(&save_timer);
        let state = Rc::clone(&state);
        let sched = Rc::new(move || {
            if state.borrow().suppress_change { return; }
            if let Some(id) = timer.borrow_mut().take() { id.remove(); }
            let s2     = Rc::clone(&save);
            let r2     = Rc::clone(&rl);
            let timer2 = Rc::clone(&timer);
            let nid = glib::timeout_add_local_once(Duration::from_millis(800), move || {
                *timer2.borrow_mut() = None; // clear before glib auto-removes the one-shot source
                s2();
                r2();
            });
            *timer.borrow_mut() = Some(nid);
        });
        let s1 = Rc::clone(&sched);
        editor.title.connect_changed(move |_| s1());
        let s2 = Rc::clone(&sched);
        editor.body.buffer().connect_changed(move |_| s2());
    }

    // ── Wire: Date backdate ────────────────────────────────────────────────
    {
        let state     = Rc::clone(&state);
        let date_btn2 = editor.date_btn.clone();
        editor.date_btn.connect_clicked(move |btn| {
            let id = state.borrow().current_entry_id.clone();
            let Some(id) = id else { return; };
            let popover = gtk4::Popover::new();
            popover.set_parent(btn);
            let cal = gtk4::Calendar::new();
            {
                let st = state.borrow();
                if let Some(v) = &st.vault {
                    if let Some(e) = v.get_entry(&id) {
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&e.created) {
                            cal.set_property("year",  dt.year());
                            cal.set_property("month", dt.month() as i32 - 1);
                            cal.set_property("day",   dt.day() as i32);
                        }
                    }
                }
            }
            let state2 = Rc::clone(&state);
            let db3    = date_btn2.clone();
            cal.connect_day_selected(move |c| {
                let y = c.year(); let m = c.month() + 1; let d = c.day();
                let s = format!("{y:04}-{m:02}-{d:02}T00:00:00+00:00");
                let mut st = state2.borrow_mut();
                if let Some(v) = st.vault.as_mut() {
                    if let Some(e) = v.get_entry_mut(&id) { e.created = s.clone(); }
                    let _ = v.save();
                }
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
                    db3.set_label(&dt.format("%-d %b %Y").to_string());
                }
            });
            popover.set_child(Some(&cal));
            popover.popup();
        });
    }

    // ── Wire: Font button — uses native FontDialog (no popover conflict) ──
    {
        let state = Rc::clone(&state);
        let body2 = editor.body.clone();
        let win2  = window.clone();
        editor.font_btn.connect_clicked(move |_| {
            let (fam, sz, wt) = {
                let st = state.borrow();
                st.vault.as_ref()
                    .map(|v| (v.font_family.clone(), v.font_size, v.font_weight.clone()))
                    .unwrap_or_else(|| ("Georgia".into(), 16.0, "400".into()))
            };
            let mut desc = gtk4::pango::FontDescription::new();
            let base_fam = fam.split(',').next().unwrap_or(&fam).trim().to_owned();
            desc.set_family(&base_fam);
            desc.set_size((sz * gtk4::pango::SCALE as f64) as i32);
            desc.set_weight(pango_weight_from_str(&wt));

            let dialog = gtk4::FontDialog::builder().build();
            let st2    = Rc::clone(&state);
            let body3  = body2.clone();
            dialog.choose_font(Some(&win2), Some(&desc), None::<&gio::Cancellable>, move |result| {
                let Ok(desc) = result else { return; };
                let fam = desc.family().map(|s| s.to_string()).unwrap_or_else(|| "Georgia".to_string());
                let sz  = desc.size() as f64 / gtk4::pango::SCALE as f64;
                let wt  = pango_weight_to_str(desc.weight());
                let mut st = st2.borrow_mut();
                let lh = st.vault.as_ref().map(|v| v.line_height).unwrap_or(1.75);
                if let Some(v) = st.vault.as_mut() {
                    v.font_family = fam.clone();
                    v.font_size   = sz;
                    v.font_weight = wt.clone();
                    let _ = v.save();
                }
                drop(st);
                apply_font(&body3, &fam, sz, &wt, lh);
            });
        });
    }

    // ── Wire: Line height button ───────────────────────────────────────────
    {
        let state = Rc::clone(&state);
        let body2 = editor.body.clone();
        editor.lh_btn.connect_clicked(move |btn| {
            let lh = state.borrow().vault.as_ref().map(|v| v.line_height).unwrap_or(1.75);
            let popover = gtk4::Popover::new();
            popover.set_parent(btn);
            let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
            vbox.set_margin_start(12); vbox.set_margin_end(12);
            vbox.set_margin_top(10);   vbox.set_margin_bottom(10);
            vbox.append(&gtk4::Label::builder()
                .label("Line height").css_classes(["dim-label"]).halign(gtk4::Align::Start).build());
            let adj  = gtk4::Adjustment::new(lh, 1.0, 3.0, 0.05, 0.1, 0.0);
            let spin = gtk4::SpinButton::new(Some(&adj), 0.05, 2);
            vbox.append(&spin);
            popover.set_child(Some(&vbox));

            let st2   = Rc::clone(&state);
            let body3 = body2.clone();
            popover.connect_closed(move |_| {
                let lh = spin.value();
                let mut st = st2.borrow_mut();
                let (fam, sz, wt) = st.vault.as_ref()
                    .map(|v| (v.font_family.clone(), v.font_size, v.font_weight.clone()))
                    .unwrap_or_else(|| ("Georgia".into(), 16.0, "400".into()));
                if let Some(v) = st.vault.as_mut() {
                    v.line_height = lh;
                    let _ = v.save();
                }
                drop(st);
                apply_font(&body3, &fam, sz, &wt, lh);
            });
            popover.popup();
        });
    }

    // ── Wire: Media attach ─────────────────────────────────────────────────
    {
        let state  = Rc::clone(&state);
        let mbox2  = editor.media_box.clone();
        let mstrip = editor.media_strip.clone();
        let win2   = window.clone();
        editor.attach_btn.connect_clicked(move |_| {
            let filters = gio::ListStore::new::<gtk4::FileFilter>();
            let f = gtk4::FileFilter::new();
            f.set_name(Some("Images & Video"));
            f.add_mime_type("image/*");
            f.add_mime_type("video/*");
            filters.append(&f);
            let fd = gtk4::FileDialog::builder().title("Attach Media").filters(&filters).build();
            let state2  = Rc::clone(&state);
            let mbox3   = mbox2.clone();
            let mstrip2 = mstrip.clone();
            fd.open(Some(&win2), None::<&gio::Cancellable>, move |result| {
                let Ok(file) = result else { return; };
                let Some(path) = file.path() else { return; };
                let Ok(data)   = std::fs::read(&path) else { return; };
                let mime = guess_mime(&path);
                let kind = if mime.starts_with("video/") { crate::vault::types::MediaKind::Video }
                           else                          { crate::vault::types::MediaKind::Image };
                let item = MediaItem {
                    id:   uuid::Uuid::new_v4().to_string(), kind,
                    data: base64::engine::general_purpose::STANDARD.encode(&data), mime,
                };
                let media_id = item.id.clone();
                let eid = state2.borrow().current_entry_id.clone();
                if let Some(id) = eid {
                    let thumb = make_deletable_thumb(&item, Rc::clone(&state2), id.clone(), media_id, mstrip2.clone());
                    {
                        let mut st = state2.borrow_mut();
                        if let Some(v) = st.vault.as_mut() {
                            if let Some(e) = v.get_entry_mut(&id) {
                                e.media.push(item);
                            }
                            let _ = v.save();
                        }
                    }
                    if let Some(t) = thumb {
                        mbox3.append(&t);
                        mstrip2.set_visible(true);
                    }
                }
            });
        });
    }

    // ── Wire: Delete entry ─────────────────────────────────────────────────
    {
        let state  = Rc::clone(&state);
        let rl     = Rc::clone(&refresh_list);
        let estack = editor.stack.clone();
        editor.delete_btn.connect_clicked(move |_| {
            let id = state.borrow_mut().current_entry_id.take();
            let Some(id) = id else { return; };
            let mut st = state.borrow_mut();
            if let Some(v) = st.vault.as_mut() {
                v.delete_entry(&id);
                let _ = v.save();
            }
            drop(st);
            estack.set_visible_child_name("empty");
            rl();
        });
    }

    // ── Wire: Dark mode toggle (bottom bar) ────────────────────────────────
    {
        let state = Rc::clone(&state);
        let cp2   = css_provider.clone();
        let dbtn2 = bottom_bar.dark_btn.clone();
        bottom_bar.dark_btn.connect_clicked(move |_| {
            let mut st = state.borrow_mut();
            st.settings.dark_mode = !st.settings.dark_mode;
            let _ = st.settings.save();
            apply_color_scheme(st.settings.dark_mode);
            cp2.load_from_string(&st.settings.css());
            dbtn2.set_icon_name(if st.settings.dark_mode { "weather-clear-symbolic" }
                                else                     { "weather-clear-night-symbolic" });
        });
    }

    // ── Wire: Accent color (bottom bar) ───────────────────────────────────
    {
        let state = Rc::clone(&state);
        let cp3   = css_provider.clone();
        let _abtn = bottom_bar.accent_btn.clone();
        bottom_bar.accent_btn.connect_clicked(move |btn| {
            let popover = gtk4::Popover::new();
            popover.set_parent(btn);
            let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            hbox.set_margin_start(10); hbox.set_margin_end(10);
            hbox.set_margin_top(10);   hbox.set_margin_bottom(10);
            let current = state.borrow().settings.accent;
            for &accent in Accent::ALL {
                let c   = accent.colors();
                let btn = gtk4::Button::new();
                btn.add_css_class("color-swatch");
                if accent == current { btn.add_css_class("selected"); }
                btn.set_tooltip_text(Some(c.label));
                btn.add_css_class(&format!("swatch-{}", c.label.to_lowercase()));
                let st2  = Rc::clone(&state);
                let cp4  = cp3.clone();
                let pop2 = popover.clone();
                btn.connect_clicked(move |_| {
                    let mut st = st2.borrow_mut();
                    st.settings.accent = accent;
                    let _ = st.settings.save();
                    cp4.load_from_string(&st.settings.css());
                    drop(st);
                    pop2.popdown();
                });
                hbox.append(&btn);
            }
            popover.set_child(Some(&hbox));
            popover.popup();
        });
    }

    // ── Wire: MPRIS poll ───────────────────────────────────────────────────
    {
        let mpris2    = Rc::clone(&mpris);
        let npbox     = bottom_bar.mpris_box.clone();
        let play_btn  = bottom_bar.play_btn.clone();
        let track_lbl = bottom_bar.track_label.clone();
        let art_lbl   = bottom_bar.artist_label.clone();
        glib::timeout_add_local(Duration::from_secs(2), move || {
            if let Some(np) = mpris2.get() {
                npbox.set_visible(true);
                track_lbl.set_label(&np.title);
                art_lbl.set_label(&np.artist);
                play_btn.set_icon_name(np.status.icon());
            } else {
                npbox.set_visible(false);
            }
            glib::ControlFlow::Continue
        });
    }
    bottom_bar.prev_btn.connect_clicked(|_| crate::mpris::send_command("Previous"));
    bottom_bar.play_btn.connect_clicked(|_| crate::mpris::send_command("PlayPause"));
    bottom_bar.next_btn.connect_clicked(|_| crate::mpris::send_command("Next"));

    // ── Wire: Noise sliders ────────────────────────────────────────────────
    {
        let noise2 = Rc::clone(&noise);
        let ws = bottom_bar.white_scale.clone();
        let ps = bottom_bar.pink_scale.clone();
        let bs = bottom_bar.brown_scale.clone();
        let ms = bottom_bar.master_scale.clone();
        let upd = Rc::new(move || {
            noise2.set_mix(NoiseMix {
                white:  ws.value() as f32, pink:   ps.value() as f32,
                brown:  bs.value() as f32, master: ms.value() as f32,
            });
        });
        macro_rules! conn { ($s:expr) => {{ let u = Rc::clone(&upd); $s.connect_value_changed(move |_| u()); }}; }
        conn!(bottom_bar.white_scale); conn!(bottom_bar.pink_scale);
        conn!(bottom_bar.brown_scale); conn!(bottom_bar.master_scale);
    }

    // ── Calendar: mark days with entries ──────────────────────────────────
    let refresh_calendar: Rc<dyn Fn()> = {
        let state    = Rc::clone(&state);
        let calendar = sidebar.calendar.clone();
        let cal_list = sidebar.cal_list.clone();
        Rc::new(move || {
            let month: u32 = (calendar.property::<i32>("month") + 1) as u32;
            let year:  i32 = calendar.property("year");
            let day:   u32 = calendar.property::<i32>("day") as u32;
            calendar.clear_marks();
            while let Some(c) = cal_list.first_child() { cal_list.remove(&c); }
            let st = state.borrow();
            if let Some(vault) = &st.vault {
                for entry in vault.search("") {
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&entry.created) {
                        if dt.year() == year && dt.month() == month {
                            calendar.mark_day(dt.day());
                            if dt.day() == day {
                                cal_list.append(&make_entry_row(entry));
                            }
                        }
                    }
                }
            }
        })
    };

    // ── Wire: Calendar toggle button ───────────────────────────────────────
    {
        let vs  = sidebar.view_stack.clone();
        let rc2 = Rc::clone(&refresh_calendar);
        sidebar.cal_btn.connect_clicked(move |_| {
            if vs.visible_child_name().as_deref() == Some("list") {
                vs.set_visible_child_name("calendar");
                rc2();
            } else {
                vs.set_visible_child_name("list");
            }
        });
    }

    // ── Wire: Calendar day selected → repopulate list below calendar ──────
    {
        let rc2 = Rc::clone(&refresh_calendar);
        sidebar.calendar.connect_day_selected(move |_| { rc2(); });
    }

    // ── Wire: Calendar month/year navigation → refresh marks ──────────────
    {
        let rc2 = Rc::clone(&refresh_calendar);
        sidebar.calendar.connect_next_month(move |_| { rc2(); });
    }
    {
        let rc2 = Rc::clone(&refresh_calendar);
        sidebar.calendar.connect_prev_month(move |_| { rc2(); });
    }
    {
        let rc2 = Rc::clone(&refresh_calendar);
        sidebar.calendar.connect_next_year(move |_| { rc2(); });
    }
    {
        let rc2 = Rc::clone(&refresh_calendar);
        sidebar.calendar.connect_prev_year(move |_| { rc2(); });
    }

    // ── Wire: Calendar day-entry list row selected ─────────────────────────
    {
        let state = Rc::clone(&state);
        let le    = Rc::clone(&load_entry);
        let save  = Rc::clone(&do_save);
        sidebar.cal_list.connect_row_selected(move |_, row| {
            let Some(row) = row else { return; };
            let id = row.widget_name().to_string();
            save();
            state.borrow_mut().current_entry_id = Some(id.clone());
            le(&id);
        });
    }

    refresh_home();
    main_stack.set_visible_child_name("home");
    window.present();
}

// ──────────────────────────────────────────────────────────────────────────────
// Create-journal helper
// ──────────────────────────────────────────────────────────────────────────────

fn wire_create_journal(
    parent: &adw::ApplicationWindow,
    state: Rc<RefCell<AppState>>,
    main_stack: gtk4::Stack,
    editor_stack: gtk4::Stack,
    vault_chip: gtk4::Label,
    home_btn: gtk4::Button,
    refresh_list: Rc<dyn Fn()>,
    err_parent: adw::ApplicationWindow,
    split_view: adw::OverlaySplitView,
) {
    dlg::show_create(parent, move |name, pass| {
        let path = vault::new_vault_path();
        match Vault::create(&path, &name, &pass) {
            Ok(v) => {
                vault_chip.set_label(&v.name);
                vault_chip.set_visible(true);
                home_btn.set_visible(true);
                state.borrow_mut().vault = Some(v);
                state.borrow_mut().current_entry_id = None;
                editor_stack.set_visible_child_name("empty");
                main_stack.set_visible_child_name("editor");
                split_view.set_show_sidebar(true);
                refresh_list();
            }
            Err(e) => dlg::show_error(&err_parent, &e.to_string()),
        }
    });
}

// ──────────────────────────────────────────────────────────────────────────────
// Journal card
// ──────────────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn build_journal_card(
    name: &str,
    path: &PathBuf,
    state: Rc<RefCell<AppState>>,
    refresh_list: Rc<dyn Fn()>,
    rh_holder: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
    main_stack: gtk4::Stack,
    editor_stack: gtk4::Stack,
    vault_chip: gtk4::Label,
    home_btn: gtk4::Button,
    window: adw::ApplicationWindow,
    split_view: adw::OverlaySplitView,
) -> gtk4::Widget {
    let card = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    card.add_css_class("journal-card");
    card.set_margin_bottom(8);

    let text_box = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    text_box.set_hexpand(true);
    text_box.set_margin_start(16); text_box.set_margin_end(8);
    text_box.set_margin_top(14);   text_box.set_margin_bottom(14);
    text_box.append(&gtk4::Label::builder()
        .label(name).css_classes(["journal-card-name"]).halign(gtk4::Align::Start).build());

    let open_btn = gtk4::Button::builder().label("Open").css_classes(["flat"])
        .valign(gtk4::Align::Center).margin_end(8).build();
    let del_btn  = gtk4::Button::builder().icon_name("user-trash-symbolic")
        .css_classes(["flat", "destructive-action"])
        .valign(gtk4::Align::Center).margin_end(8).build();

    card.append(&text_box);
    card.append(&open_btn);
    card.append(&del_btn);

    // Open
    {
        let name2  = name.to_owned();
        let path2  = path.clone();
        let state2 = Rc::clone(&state);
        let rl     = Rc::clone(&refresh_list);
        let ms     = main_stack.clone();
        let es     = editor_stack.clone();
        let vc     = vault_chip.clone();
        let hb     = home_btn.clone();
        let win    = window.clone();
        let win2   = window.clone();
        let sv     = split_view.clone();
        open_btn.connect_clicked(move |_| {
            let name3  = name2.clone();
            let path3  = path2.clone();
            let state3 = Rc::clone(&state2);
            let rl2    = Rc::clone(&rl);
            let ms2    = ms.clone();
            let es2    = es.clone();
            let vc2    = vc.clone();
            let hb2    = hb.clone();
            let win3   = win2.clone();
            let sv2    = sv.clone();
            dlg::show_unlock(&win, &name3, move |pass| {
                match Vault::open(&path3, &pass) {
                    Ok(v) => {
                        vc2.set_label(&v.name);
                        vc2.set_visible(true);
                        hb2.set_visible(true);
                        state3.borrow_mut().vault = Some(v);
                        state3.borrow_mut().current_entry_id = None;
                        es2.set_visible_child_name("empty");
                        ms2.set_visible_child_name("editor");
                        sv2.set_show_sidebar(true);
                        rl2();
                    }
                    Err(e) => dlg::show_error(&win3, &e.to_string()),
                }
            });
        });
    }

    // Delete
    {
        let name3 = name.to_owned();
        let path3 = path.clone();
        let rh    = Rc::clone(&rh_holder);
        let win   = window.clone();
        del_btn.connect_clicked(move |_| {
            let msg    = format!("Delete \"{}\"? This cannot be undone.", name3);
            let dialog = adw::MessageDialog::new(Some(&win), Some("Delete Journal"), Some(&msg));
            dialog.add_response("cancel", "Cancel");
            dialog.add_response("delete", "Delete");
            dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
            dialog.set_default_response(Some("cancel"));
            dialog.set_close_response("cancel");
            let path4 = path3.clone();
            let rh2   = Rc::clone(&rh);
            dialog.connect_response(None, move |dlg, response| {
                dlg.close();
                if response == "delete" {
                    let _ = std::fs::remove_file(&path4);
                    if let Some(f) = rh2.borrow().as_ref() { f(); }
                }
            });
            dialog.present();
        });
    }

    card.upcast()
}

// ──────────────────────────────────────────────────────────────────────────────
// Widget builders
// ──────────────────────────────────────────────────────────────────────────────

struct SidebarWidgets {
    outer:      gtk4::Box,
    list:       gtk4::ListBox,
    search:     gtk4::SearchEntry,
    new_btn:    gtk4::Button,
    cal_btn:    gtk4::Button,
    view_stack: gtk4::Stack,
    calendar:   gtk4::Calendar,
    cal_list:   gtk4::ListBox,
}

fn build_sidebar() -> SidebarWidgets {
    let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    outer.add_css_class("sidebar");

    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    header.add_css_class("sidebar-header");
    header.append(&gtk4::Label::builder()
        .label("ENTRIES").css_classes(["vault-label"])
        .hexpand(true).halign(gtk4::Align::Start).build());
    let cal_btn = gtk4::Button::builder()
        .icon_name("calendar-symbolic")
        .tooltip_text("Calendar view")
        .css_classes(["flat"]).build();
    let new_btn = gtk4::Button::builder()
        .icon_name("list-add-symbolic").tooltip_text("New entry")
        .css_classes(["flat", "new-entry-icon"]).build();
    header.append(&cal_btn);
    header.append(&new_btn);

    // List view
    let search = gtk4::SearchEntry::builder()
        .placeholder_text("Search…").css_classes(["sidebar-search"]).build();
    let list = gtk4::ListBox::new();
    list.set_selection_mode(gtk4::SelectionMode::Single);
    list.add_css_class("entry-list");
    let list_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true).child(&list).build();
    let list_view = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    list_view.append(&search);
    list_view.append(&list_scroll);

    // Calendar view
    let calendar = gtk4::Calendar::new();
    let cal_list = gtk4::ListBox::new();
    cal_list.set_selection_mode(gtk4::SelectionMode::Single);
    cal_list.add_css_class("entry-list");
    let cal_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true).child(&cal_list).build();
    let cal_view = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    cal_view.append(&calendar);
    cal_view.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    cal_view.append(&cal_scroll);

    let view_stack = gtk4::Stack::new();
    view_stack.set_vexpand(true);
    view_stack.add_named(&list_view, Some("list"));
    view_stack.add_named(&cal_view, Some("calendar"));

    outer.append(&header);
    outer.append(&view_stack);
    SidebarWidgets { outer, list, search, new_btn, cal_btn, view_stack, calendar, cal_list }
}

struct EditorWidgets {
    outer:       gtk4::Box,
    stack:       gtk4::Stack,
    title:       gtk4::Entry,
    body:        gtk4::TextView,
    media_box:   gtk4::Box,
    media_strip: gtk4::Box,
    attach_btn:  gtk4::Button,
    delete_btn:  gtk4::Button,
    date_btn:    gtk4::Button,
    font_btn:    gtk4::Button,
    lh_btn:      gtk4::Button,
}

fn build_editor() -> EditorWidgets {
    let outer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    outer.add_css_class("editor-area");
    outer.set_hexpand(true);

    let stack = gtk4::Stack::new();
    stack.set_vexpand(true);
    stack.set_hexpand(true);
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);

    let empty = gtk4::Box::new(gtk4::Orientation::Vertical, 16);
    empty.set_valign(gtk4::Align::Center);
    empty.set_halign(gtk4::Align::Center);
    empty.append(&gtk4::Image::builder()
        .icon_name("document-edit-symbolic").pixel_size(64).css_classes(["empty-icon"]).build());
    empty.append(&gtk4::Label::builder()
        .label("Select or create an entry").css_classes(["empty-label"]).build());
    stack.add_named(&empty, Some("empty"));

    let evbox    = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    let title_bar = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    title_bar.add_css_class("editor-title-bar");

    let title_entry = gtk4::Entry::builder()
        .placeholder_text("Untitled").css_classes(["editor-title"]).hexpand(true).build();
    let date_btn = gtk4::Button::builder()
        .label("Set date").css_classes(["entry-date-btn"])
        .tooltip_text("Backdate entry").valign(gtk4::Align::Center).build();
    let font_btn = gtk4::Button::builder()
        .icon_name("preferences-desktop-font-symbolic")
        .tooltip_text("Journal font settings")
        .css_classes(["flat"]).build();
    let lh_btn = gtk4::Button::builder()
        .label("↕")
        .tooltip_text("Line height")
        .css_classes(["flat"]).build();
    let attach_btn = gtk4::Button::builder()
        .icon_name("mail-attachment-symbolic").tooltip_text("Attach image or video")
        .css_classes(["flat"]).build();
    let delete_btn = gtk4::Button::builder()
        .icon_name("user-trash-symbolic").tooltip_text("Delete entry")
        .css_classes(["flat", "destructive-action"]).build();

    title_bar.append(&title_entry);
    title_bar.append(&date_btn);
    title_bar.append(&font_btn);
    title_bar.append(&lh_btn);
    title_bar.append(&attach_btn);
    title_bar.append(&delete_btn);

    let body_view = gtk4::TextView::builder()
        .css_classes(["editor-body"]).wrap_mode(gtk4::WrapMode::WordChar)
        .left_margin(48).right_margin(48).top_margin(24).bottom_margin(48).build();
    let body_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true).child(&body_view).build();

    let media_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    media_box.add_css_class("media-strip");
    media_box.set_margin_start(16); media_box.set_margin_end(16);
    media_box.set_margin_top(8);    media_box.set_margin_bottom(8);
    let media_scroll = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Never)
        .child(&media_box).build();

    let media_strip = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    media_strip.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    media_strip.append(&media_scroll);
    media_strip.set_visible(false);

    evbox.append(&title_bar);
    evbox.append(&gtk4::Separator::new(gtk4::Orientation::Horizontal));
    evbox.append(&body_scroll);
    evbox.append(&media_strip);

    stack.add_named(&evbox, Some("editor"));
    stack.set_visible_child_name("empty");
    outer.append(&stack);

    EditorWidgets { outer, stack, title: title_entry, body: body_view,
                    media_box, media_strip, attach_btn, delete_btn, date_btn, font_btn, lh_btn }
}

struct BottomBarWidgets {
    outer:        gtk4::Box,
    inner:        gtk4::Box,
    noise_box:    gtk4::Box,
    config_box:   gtk4::Box,
    mpris_box:    gtk4::Box,
    mpris_btns:   gtk4::Box,
    spacer:       gtk4::Box,
    sep_h:        gtk4::Separator,
    sep_v:        gtk4::Separator,
    white_scale:  gtk4::Scale,
    pink_scale:   gtk4::Scale,
    brown_scale:  gtk4::Scale,
    master_scale: gtk4::Scale,
    dark_btn:     gtk4::Button,
    accent_btn:   gtk4::Button,
    track_label:  gtk4::Label,
    artist_label: gtk4::Label,
    prev_btn:     gtk4::Button,
    play_btn:     gtk4::Button,
    next_btn:     gtk4::Button,
}

fn build_bottom_bar() -> BottomBarWidgets {
    let outer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    outer.add_css_class("bottom-bar");
    let sep_h = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    let sep_v = gtk4::Separator::new(gtk4::Orientation::Vertical);
    sep_v.set_visible(false);

    let inner = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    inner.set_margin_start(16); inner.set_margin_end(16);
    inner.set_margin_top(7);    inner.set_margin_bottom(7);

    // Noise section
    let noise_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    noise_box.append(&gtk4::Label::builder()
        .label("AMBIENT").css_classes(["noise-section-label"]).build());
    let (wg, ws) = noise_slider_group("W");   let (pg, ps) = noise_slider_group("P");
    let (bg, bs) = noise_slider_group("Br");  let (mg, ms) = noise_slider_group("VOL");
    noise_box.append(&wg); noise_box.append(&pg);
    noise_box.append(&bg); noise_box.append(&mg);

    // Spacer
    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);

    // Controls: dark mode + accent
    let config_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let dark_btn = gtk4::Button::builder()
        .icon_name("weather-clear-night-symbolic")
        .tooltip_text("Toggle dark mode")
        .css_classes(["flat"]).build();
    let accent_btn = gtk4::Button::builder()
        .icon_name("preferences-color-symbolic")
        .tooltip_text("Accent color")
        .css_classes(["flat"]).build();
    config_box.append(&dark_btn);
    config_box.append(&accent_btn);

    // MPRIS
    let mpris_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    mpris_box.set_visible(false);
    let track_label  = gtk4::Label::builder().css_classes(["mpris-title"])
        .ellipsize(gtk4::pango::EllipsizeMode::End).max_width_chars(28).build();
    let artist_label = gtk4::Label::builder().css_classes(["mpris-artist"])
        .ellipsize(gtk4::pango::EllipsizeMode::End).max_width_chars(18).build();
    let tbox = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    tbox.set_valign(gtk4::Align::Center);
    tbox.append(&track_label); tbox.append(&artist_label);

    let mpris_btns = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
    let prev_btn = gtk4::Button::builder()
        .icon_name("media-skip-backward-symbolic").css_classes(["mpris-button"]).build();
    let play_btn = gtk4::Button::builder()
        .icon_name("media-playback-start-symbolic").css_classes(["mpris-button"]).build();
    let next_btn = gtk4::Button::builder()
        .icon_name("media-skip-forward-symbolic").css_classes(["mpris-button"]).build();
    mpris_btns.append(&prev_btn); mpris_btns.append(&play_btn); mpris_btns.append(&next_btn);

    mpris_box.append(&tbox);
    mpris_box.append(&mpris_btns);

    inner.append(&noise_box);
    inner.append(&spacer);
    inner.append(&config_box);
    inner.append(&mpris_box);

    let main_content = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    main_content.append(&sep_h);
    main_content.append(&inner);

    outer.append(&sep_v);
    outer.append(&main_content);

    BottomBarWidgets {
        outer, inner, noise_box, config_box, mpris_box, mpris_btns, spacer, sep_h, sep_v,
        white_scale: ws, pink_scale: ps, brown_scale: bs, master_scale: ms,
        dark_btn, accent_btn,
        track_label, artist_label, prev_btn, play_btn, next_btn,
    }
}

fn noise_slider_group(label: &str) -> (gtk4::Box, gtk4::Scale) {
    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    let lbl  = gtk4::Label::builder().label(label).css_classes(["noise-label"]).build();
    let scale = gtk4::Scale::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .adjustment(&gtk4::Adjustment::new(0.0, 0.0, 1.0, 0.01, 0.1, 0.0))
        .width_request(48).draw_value(false).css_classes(["noise-slider"]).build();
    if label == "VOL" { scale.set_value(0.5); }
    hbox.append(&lbl);
    hbox.append(&scale);
    (hbox, scale)
}

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

fn apply_color_scheme(dark: bool) {
    let scheme = if dark { adw::ColorScheme::ForceDark } else { adw::ColorScheme::ForceLight };
    adw::StyleManager::default().set_color_scheme(scheme);
}

fn make_entry_row(entry: &crate::vault::types::Entry) -> gtk4::ListBoxRow {
    let row = gtk4::ListBoxRow::new();
    row.set_widget_name(&entry.id);
    row.add_css_class("entry-row");
    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
    vbox.set_margin_start(12); vbox.set_margin_end(12);
    vbox.set_margin_top(8);    vbox.set_margin_bottom(8);
    let display = if entry.title.is_empty() { "Untitled" } else { &entry.title };
    vbox.append(&gtk4::Label::builder()
        .label(display).css_classes(["entry-title-label"])
        .halign(gtk4::Align::Start).ellipsize(gtk4::pango::EllipsizeMode::End).single_line_mode(true).build());
    let date = chrono::DateTime::parse_from_rfc3339(&entry.created)
        .map(|dt| dt.format("%-d %b %Y").to_string()).unwrap_or_else(|_| "—".into());
    vbox.append(&gtk4::Label::builder()
        .label(&date).css_classes(["entry-date-label"]).halign(gtk4::Align::Start).build());
    row.set_child(Some(&vbox));
    row
}

fn make_deletable_thumb(
    item:        &MediaItem,
    state:       Rc<RefCell<AppState>>,
    entry_id:    String,
    media_id:    String,
    media_strip: gtk4::Box,
) -> Option<gtk4::Widget> {
    let thumb   = make_media_thumb(item)?;
    let overlay = gtk4::Overlay::new();
    overlay.set_child(Some(&thumb));

    let del_btn = gtk4::Button::builder()
        .icon_name("window-close-symbolic")
        .css_classes(["media-delete-btn"])
        .halign(gtk4::Align::End)
        .valign(gtk4::Align::Start)
        .build();

    let ov2 = overlay.clone();
    del_btn.connect_clicked(move |_| {
        {
            let mut st = state.borrow_mut();
            if let Some(v) = st.vault.as_mut() {
                if let Some(e) = v.get_entry_mut(&entry_id) {
                    e.media.retain(|m| m.id != media_id);
                }
                let _ = v.save();
            }
        }
        let parent = ov2.parent();
        ov2.unparent();
        if parent.map(|p| p.first_child().is_none()).unwrap_or(false) {
            media_strip.set_visible(false);
        }
    });

    overlay.add_overlay(&del_btn);
    Some(overlay.upcast())
}

fn make_media_thumb(item: &MediaItem) -> Option<gtk4::Widget> {
    use base64::Engine;
    match item.kind {
        crate::vault::types::MediaKind::Image => {
            let bytes   = base64::engine::general_purpose::STANDARD.decode(&item.data).ok()?;
            let texture = gdk4::Texture::from_bytes(&glib::Bytes::from_owned(bytes)).ok()?;
            let pic     = gtk4::Picture::for_paintable(&texture);
            pic.set_size_request(120, 80); pic.add_css_class("media-thumb");
            Some(pic.upcast())
        }
        crate::vault::types::MediaKind::Video => {
            let bytes = base64::engine::general_purpose::STANDARD.decode(&item.data).ok()?;
            let tmp   = std::env::temp_dir().join(format!("blossom_{}.tmp", &item.id[..8]));
            std::fs::write(&tmp, &bytes).ok()?;
            let video = gtk4::Video::for_file(Some(&gio::File::for_path(&tmp)));
            video.set_size_request(160, 90); video.add_css_class("media-thumb");
            Some(video.upcast())
        }
    }
}

fn apply_font(_view: &gtk4::TextView, family: &str, size: f64, weight: &str, line_height: f64) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(&format!(
        "textview.editor-body {{ font-family: {family}; font-size: {size}pt; \
         font-weight: {weight}; line-height: {line_height}; }}"
    ));
    gtk4::style_context_add_provider_for_display(
        &gdk4::Display::default().unwrap(), &provider, gtk4::STYLE_PROVIDER_PRIORITY_USER,
    );
}

fn guess_mime(path: &std::path::Path) -> String {
    match path.extension().and_then(|e| e.to_str()) {
        Some("jpg"|"jpeg") => "image/jpeg".into(),
        Some("png")  => "image/png".into(),
        Some("gif")  => "image/gif".into(),
        Some("webp") => "image/webp".into(),
        Some("mp4")  => "video/mp4".into(),
        Some("webm") => "video/webm".into(),
        Some("mkv")  => "video/x-matroska".into(),
        Some("mov")  => "video/quicktime".into(),
        _ => "application/octet-stream".into(),
    }
}

fn pango_weight_from_str(w: &str) -> gtk4::pango::Weight {
    match w {
        "100" => gtk4::pango::Weight::Thin,
        "200" => gtk4::pango::Weight::Ultralight,
        "300" => gtk4::pango::Weight::Light,
        "500" => gtk4::pango::Weight::Medium,
        "600" => gtk4::pango::Weight::Semibold,
        "700" => gtk4::pango::Weight::Bold,
        "800" => gtk4::pango::Weight::Ultrabold,
        "900" => gtk4::pango::Weight::Heavy,
        _     => gtk4::pango::Weight::Normal,
    }
}

fn pango_weight_to_str(w: gtk4::pango::Weight) -> String {
    match w {
        gtk4::pango::Weight::Thin       => "100",
        gtk4::pango::Weight::Ultralight => "200",
        gtk4::pango::Weight::Light      => "300",
        gtk4::pango::Weight::Normal     => "400",
        gtk4::pango::Weight::Medium     => "500",
        gtk4::pango::Weight::Semibold   => "600",
        gtk4::pango::Weight::Bold       => "700",
        gtk4::pango::Weight::Ultrabold  => "800",
        gtk4::pango::Weight::Heavy      => "900",
        _                               => "400",
    }.to_owned()
}
