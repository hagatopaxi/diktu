//! Preferences window: language, models, shortcut, dictation behaviour.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use adw::prelude::*;
use diktu_core::custom;
use diktu_core::download::{self, Error};
use diktu_core::i18n::{tr, trf};
use diktu_core::registry::Model;
use gtk::glib;

use crate::{Ui, all_models, check_model, models_root, selected_model};

/// Languages offered when importing a model.
const LANGUAGES: &[(&str, &str)] = &[
    ("fr", "Français"),
    ("en", "English"),
    ("de", "Deutsch"),
    ("es", "Español"),
    ("it", "Italiano"),
    ("pt", "Português"),
    ("nl", "Nederlands"),
    ("pl", "Polski"),
    ("ru", "Русский"),
    ("zh", "中文"),
    ("ja", "日本語"),
    ("ko", "한국어"),
    ("ar", "العربية"),
];

fn language_name(code: &str) -> &str {
    LANGUAGES
        .iter()
        .find(|(c, _)| *c == code)
        .map_or(code, |(_, name)| name)
}

pub fn build(ui: &Rc<Ui>) -> adw::PreferencesWindow {
    let window = adw::PreferencesWindow::builder()
        .title(trf("{app} Preferences", &[("app", &crate::APP_NAME)]))
        .default_width(600)
        .default_height(680)
        .search_enabled(false)
        // Downloads keep running, and their rows stay current, while the window is closed.
        .hide_on_close(true)
        .build();
    let page = adw::PreferencesPage::new();
    window.add(&page);
    let settings = &ui.settings;

    let models = all_models();
    let mut languages: Vec<String> = models.iter().flat_map(|m| m.langs.clone()).collect();
    languages.sort();
    languages.dedup();
    let names: Vec<&str> = languages.iter().map(|l| language_name(l)).collect();
    let language = adw::ComboRow::builder()
        .title(tr("Language"))
        .model(&gtk::StringList::new(&names))
        .build();
    let current = crate::language(settings);
    if let Some(i) = languages.iter().position(|l| *l == current.as_str()) {
        language.set_selected(i as u32);
    }
    let group = adw::PreferencesGroup::builder()
        .title(tr("Dictation"))
        .build();
    group.add(&language);
    page.add(&group);

    let group = adw::PreferencesGroup::builder()
        .title(tr("Models"))
        .description(tr(
            "Download a model for the chosen language to start dictating. Models come from Hugging Face, are verified (SHA-256), then used offline.",
        ))
        .build();
    let mut first_check: Option<gtk::CheckButton> = None;
    let rows: Vec<(Model, adw::ActionRow)> = models
        .iter()
        .cloned()
        .map(|m| {
            let row = model_row(ui, &window, &m, &models, &mut first_check);
            group.add(&row);
            (m, row)
        })
        .collect();
    let show_language = move |lang: &str| {
        for (m, row) in &rows {
            row.set_visible(m.langs.iter().any(|l| l == lang));
        }
    };
    show_language(&current);
    group.add(&import_row(ui, &window));
    language.connect_selected_notify(glib::clone!(
        #[strong]
        settings,
        move |combo| {
            let lang = &languages[combo.selected() as usize];
            let _ = settings.set_string("language", lang);
            let _ = settings.set_string("model", "");
            show_language(lang);
        }
    ));
    page.add(&group);

    let group = adw::PreferencesGroup::builder()
        .title(tr("Shortcut"))
        .description(tr(
            "GNOME assigns the shortcut. Fallback: a custom GNOME shortcut that runs “diktu --toggle”.",
        ))
        .build();
    let shortcut = adw::ActionRow::builder()
        .title(tr("Start or stop dictation"))
        .subtitle(
            ui.trigger
                .borrow()
                .clone()
                .unwrap_or_else(|| tr("GlobalShortcuts portal unavailable")),
        )
        .build();
    let change = gtk::Button::builder()
        .label(tr("Change…"))
        .valign(gtk::Align::Center)
        .build();
    let configure = ui.configure.clone();
    let win = window.downgrade();
    change.connect_clicked(move |_| {
        if configure.send(()).is_err()
            && let Some(window) = win.upgrade()
        {
            window.add_toast(adw::Toast::new(&tr("Change it in Settings → Apps → Diktu")));
        }
    });
    shortcut.add_suffix(&change);
    group.add(&shortcut);
    *ui.shortcut_row.borrow_mut() = Some(shortcut);
    page.add(&group);

    let group = adw::PreferencesGroup::builder()
        .title(tr("Behavior"))
        .build();
    let silence = adw::SpinRow::with_range(0.5, 5.0, 0.1);
    silence.set_title(&tr("End-of-speech silence (s)"));
    silence.set_subtitle(&tr("Silence after speech that stops listening"));
    silence.set_digits(1);
    settings.bind("end-silence", &silence, "value").build();
    group.add(&silence);
    let postprocess = adw::SwitchRow::builder()
        .title(tr("Capitalize and end with a period"))
        .build();
    settings.bind("postprocess", &postprocess, "active").build();
    group.add(&postprocess);
    let delay = adw::SpinRow::with_range(0.0, 100.0, 1.0);
    delay.set_title(&tr("Delay between keys (ms)"));
    delay.set_subtitle(&tr("Increase it if some apps drop characters"));
    settings.bind("key-delay", &delay, "value").build();
    group.add(&delay);
    let sounds = adw::SwitchRow::builder()
        .title(tr("Start and stop sounds"))
        .build();
    settings.bind("sounds", &sounds, "active").build();
    group.add(&sounds);
    let volume = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.05);
    volume.set_hexpand(true);
    volume.set_valign(gtk::Align::Center);
    settings
        .bind("volume", &volume.adjustment(), "value")
        .build();
    let row = adw::ActionRow::builder().title(tr("Volume")).build();
    row.add_suffix(&volume);
    sounds
        .bind_property("active", &row, "sensitive")
        .sync_create()
        .build();
    group.add(&row);
    page.add(&group);

    let group = adw::PreferencesGroup::new();
    let about = adw::ActionRow::builder()
        .title(trf("About {app}", &[("app", &crate::APP_NAME)]))
        .subtitle(tr("Version, permissions, licenses, source code"))
        .activatable(true)
        .build();
    about.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
    about.connect_activated(glib::clone!(
        #[weak]
        window,
        move |_| about_dialog().present(Some(&window))
    ));
    group.add(&about);
    page.add(&group);

    window
}

const REPOSITORY: &str = "https://github.com/hagatopaxi/diktu";

/// Version, project links, permissions, and the licenses of what Diktu ships or downloads.
fn about_dialog() -> adw::AboutDialog {
    let dialog = adw::AboutDialog::builder()
        .application_name(crate::APP_NAME)
        .application_icon(crate::APP_ID)
        .version(env!("CARGO_PKG_VERSION"))
        .developer_name("Gwenaël Léger")
        .copyright("© Gwenaël Léger")
        .license_type(gtk::License::Gpl30)
        .website(REPOSITORY)
        .issue_url(format!("{REPOSITORY}/issues"))
        .comments(
            [
                tr("Permissions"),
                String::new(),
                tr("• Microphone: open only while you dictate; no audio leaves your machine."),
                tr("• Remote desktop (keyboard): types the recognized text at the cursor."),
                tr("• Global shortcut: starts and stops dictation from any app."),
                tr("• Run in background: listens for the shortcut with no window open."),
                tr("• Network: only to download models from Hugging Face, when you ask."),
                tr("• Status icon: the tray icon showing whether dictation is on."),
            ]
            .join("\n"),
        )
        .build();
    dialog.add_link(&tr("Source code"), REPOSITORY);
    // Translators: your names and emails, one per line.
    let credits = tr("translator-credits");
    if credits != "translator-credits" {
        dialog.set_translator_credits(&credits);
    }
    dialog.add_legal_section(
        "sherpa-onnx",
        Some("© k2-fsa"),
        gtk::License::Apache20,
        None,
    );
    dialog.add_legal_section(
        "ONNX Runtime",
        Some("© Microsoft Corporation"),
        gtk::License::MitX11,
        None,
    );
    for model in all_models() {
        dialog.add_legal_section(
            &model.name,
            None,
            gtk::License::Custom,
            Some(&trf("License: {license}", &[("license", &model.license)])),
        );
    }
    dialog
}

enum Progress {
    Bytes(u64, u64),
    Done(Result<(), Error>),
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(tooltip)
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
}

/// One model: selection, download with progress and cancel, removal.
fn model_row(
    ui: &Rc<Ui>,
    window: &adw::PreferencesWindow,
    model: &Model,
    models: &[Model],
    first_check: &mut Option<gtk::CheckButton>,
) -> adw::ActionRow {
    let size = trf(
        "{size} MB",
        &[("size", &((model.total_size() + 500_000) / 1_000_000))],
    );
    let row = adw::ActionRow::builder().title(&model.name).build();

    // A choice only exists when a language has several models.
    let alternatives = models
        .iter()
        .filter(|m| m.langs.iter().any(|l| model.langs.contains(l)))
        .count();
    if alternatives > 1 {
        let check = gtk::CheckButton::builder()
            .valign(gtk::Align::Center)
            .build();
        check.set_group(first_check.as_ref());
        first_check.get_or_insert_with(|| check.clone());
        check.set_active(selected_model(&ui.settings).is_some_and(|m| m.id == model.id));
        let (settings, id) = (ui.settings.clone(), model.id.clone());
        check.connect_toggled(move |c| {
            if c.is_active() {
                let _ = settings.set_string("model", &id);
            }
        });
        row.add_prefix(&check);
        row.set_activatable_widget(Some(&check));
    }

    let progress = gtk::ProgressBar::builder()
        .valign(gtk::Align::Center)
        .width_request(140)
        .show_text(true)
        .build();
    let download = icon_button("folder-download-symbolic", &tr("Download"));
    let cancel = icon_button("process-stop-symbolic", &tr("Cancel download"));
    let remove = icon_button("user-trash-symbolic", &tr("Remove"));
    for w in [
        progress.upcast_ref::<gtk::Widget>(),
        download.upcast_ref(),
        cancel.upcast_ref(),
        remove.upcast_ref(),
    ] {
        row.add_suffix(w);
    }

    let running: Rc<RefCell<Option<Arc<AtomicBool>>>> = Rc::default();
    let refresh = {
        let (row, model, running) = (row.clone(), model.clone(), running.clone());
        let (progress, download, cancel, remove) = (
            progress.clone(),
            download.clone(),
            cancel.clone(),
            remove.clone(),
        );
        let license = model.license.clone();
        Rc::new(move || {
            let busy = running.borrow().is_some();
            let installed = !busy && download::is_installed(&models_root(), &model);
            let state = if busy {
                tr("downloading")
            } else if installed {
                tr("installed")
            } else {
                tr("not installed")
            };
            row.set_subtitle(&trf(
                "{size} · {state} · license {license}",
                &[("size", &size), ("state", &state), ("license", &license)],
            ));
            progress.set_visible(busy);
            cancel.set_visible(busy);
            // An imported model has no source to download again from.
            download.set_visible(!busy && !installed && !custom::is_custom(&model));
            remove.set_visible(installed);
        })
    };
    refresh();
    // The window is reused: re-read the disk each time it shows, files may have changed.
    window.connect_show(glib::clone!(
        #[strong]
        refresh,
        move |_| refresh()
    ));

    download.connect_clicked(glib::clone!(
        #[strong]
        ui,
        #[strong]
        refresh,
        #[strong]
        running,
        #[strong]
        progress,
        #[weak]
        window,
        #[to_owned]
        model,
        move |_| {
            let flag = Arc::new(AtomicBool::new(false));
            *running.borrow_mut() = Some(flag.clone());
            progress.set_fraction(0.0);
            refresh();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let thread_model = model.clone();
            std::thread::spawn(move || {
                let result = download::install(
                    download::HF_BASE_URL,
                    &models_root(),
                    &thread_model,
                    &flag,
                    |done, total| {
                        let _ = tx.send(Progress::Bytes(done, total));
                    },
                );
                let _ = tx.send(Progress::Done(result));
            });
            glib::spawn_future_local(glib::clone!(
                #[strong]
                ui,
                #[strong]
                refresh,
                #[strong]
                running,
                #[strong]
                progress,
                #[strong]
                model,
                async move {
                    while let Some(p) = rx.recv().await {
                        match p {
                            Progress::Bytes(done, total) => {
                                progress.set_fraction(done as f64 / total.max(1) as f64);
                                progress.set_text(Some(&trf(
                                    "{done} / {total} MB",
                                    &[
                                        ("done", &(done / 1_000_000)),
                                        ("total", &((total + 500_000) / 1_000_000)),
                                    ],
                                )));
                            }
                            Progress::Done(result) => {
                                *running.borrow_mut() = None;
                                refresh();
                                match result {
                                    Ok(()) => {
                                        if selected_model(&ui.settings)
                                            .is_some_and(|m| m.id == model.id)
                                        {
                                            ui.reload_engine();
                                        }
                                    }
                                    Err(Error::Cancelled) => {}
                                    Err(e) => window.add_toast(adw::Toast::new(&e.to_string())),
                                }
                            }
                        }
                    }
                }
            ));
        }
    ));

    let flag = running.clone();
    cancel.connect_clicked(move |_| {
        if let Some(f) = flag.borrow().as_ref() {
            f.store(true, Ordering::Relaxed);
        }
    });

    remove.connect_clicked(glib::clone!(
        #[strong]
        ui,
        #[weak]
        window,
        #[to_owned]
        model,
        move |_| {
            if let Err(e) = download::remove(&models_root(), &model) {
                window.add_toast(adw::Toast::new(&trf(
                    "Could not remove: {error}",
                    &[("error", &e)],
                )));
            }
            if custom::is_custom(&model) {
                if ui.settings.string("model") == model.id.as_str() {
                    let _ = ui.settings.set_string("model", "");
                }
                ui.rebuild_preferences();
            } else {
                refresh();
            }
        }
    ));

    row
}

enum Source {
    HuggingFace(String),
    Folder(std::path::PathBuf),
}

enum ImportProgress {
    Bytes(u64, u64),
    Checking,
    /// The check summary, `None` when it did not finish in time.
    Done(Result<(Model, Option<String>), String>),
}

/// Downloads or copies a model to the staging area, checks it, then installs it.
fn import(
    source: Source,
    lang: &str,
    cancel: &AtomicBool,
    tx: &tokio::sync::mpsc::UnboundedSender<ImportProgress>,
) -> Result<(Model, Option<String>), String> {
    let root = models_root();
    std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let model = match source {
        Source::HuggingFace(repo) => {
            custom::stage_hf(download::HF_BASE_URL, &root, &repo, lang, cancel, |d, t| {
                let _ = tx.send(ImportProgress::Bytes(d, t));
            })?
        }
        Source::Folder(dir) => custom::stage_folder(&root, &dir, lang)?,
    };
    let _ = tx.send(ImportProgress::Checking);
    let checked = check_model(&custom::staging_dir(&root, &model))
        .and_then(|summary| custom::commit(&root, &model).map(|()| summary));
    if checked.is_err() {
        custom::discard(&root, &model);
    }
    checked.map(|summary| (model, summary))
}

/// “Add a model”: a Hugging Face repository or a local folder, for a chosen language.
fn import_row(ui: &Rc<Ui>, window: &adw::PreferencesWindow) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(tr("Add a model"))
        .subtitle(tr(
            "sherpa-onnx streaming transducer, from Hugging Face or a folder",
        ))
        .build();
    let add = icon_button("list-add-symbolic", &tr("Add a model"));
    let cancel = icon_button("process-stop-symbolic", &tr("Cancel"));
    cancel.set_visible(false);
    row.add_suffix(&add);
    row.add_suffix(&cancel);
    let running: Rc<RefCell<Option<Arc<AtomicBool>>>> = Rc::default();

    let start = Rc::new(glib::clone!(
        #[strong]
        ui,
        #[weak]
        window,
        #[weak]
        row,
        #[weak]
        add,
        #[weak]
        cancel,
        #[strong]
        running,
        move |source: Source, lang: String| {
            let flag = Arc::new(AtomicBool::new(false));
            *running.borrow_mut() = Some(flag.clone());
            add.set_visible(false);
            cancel.set_visible(true);
            row.set_subtitle(&tr("Preparing…"));
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let thread_lang = lang.clone();
            std::thread::spawn(move || {
                let result = import(source, &thread_lang, &flag, &tx);
                let _ = tx.send(ImportProgress::Done(result));
            });
            glib::spawn_future_local(glib::clone!(
                #[strong]
                ui,
                #[strong]
                running,
                async move {
                    while let Some(p) = rx.recv().await {
                        match p {
                            ImportProgress::Bytes(done, total) => row.set_subtitle(&trf(
                                "Downloading… {done} / {total} MB",
                                &[
                                    ("done", &(done / 1_000_000)),
                                    ("total", &((total + 500_000) / 1_000_000)),
                                ],
                            )),
                            ImportProgress::Checking => {
                                cancel.set_visible(false);
                                row.set_subtitle(&tr(
                                    "Checking the model (loading, speed, test sentence)…",
                                ));
                            }
                            ImportProgress::Done(Ok((model, summary))) => {
                                *running.borrow_mut() = None;
                                let _ = ui.settings.set_string("language", &lang);
                                let _ = ui.settings.set_string("model", &model.id);
                                ui.rebuild_preferences();
                                if let Some(w) = ui.preferences.borrow().as_ref() {
                                    let toast = match summary {
                                        Some(s) => {
                                            adw::Toast::new(&trf(
                                                "{name} added: {summary}",
                                                &[("name", &model.name), ("summary", &s)],
                                            ))
                                        }
                                        None => adw::Toast::builder()
                                            .title(trf(
                                                "{name} added, but its tests did not finish: you can still try it, it may misbehave",
                                                &[("name", &model.name)],
                                            ))
                                            .timeout(0)
                                            .build(),
                                    };
                                    w.add_toast(toast);
                                }
                            }
                            ImportProgress::Done(Err(e)) => {
                                *running.borrow_mut() = None;
                                add.set_visible(true);
                                cancel.set_visible(false);
                                row.set_subtitle(&trf("Not added: {error}", &[("error", &e)]));
                                window.add_toast(adw::Toast::new(&tr("The model was not added")));
                            }
                        }
                    }
                }
            ));
        }
    ));

    let flag = running.clone();
    cancel.connect_clicked(move |_| {
        if let Some(f) = flag.borrow().as_ref() {
            f.store(true, Ordering::Relaxed);
        }
    });

    let settings = ui.settings.clone();
    add.connect_clicked(glib::clone!(
        #[weak]
        window,
        move |_| {
            let repo = adw::EntryRow::builder()
                .title(tr("Hugging Face repository (owner/name)"))
                .build();
            let names: Vec<&str> = LANGUAGES.iter().map(|(_, n)| *n).collect();
            let language = adw::ComboRow::builder()
                .title(tr("Language"))
                .model(&gtk::StringList::new(&names))
                .build();
            let current = crate::language(&settings);
            if let Some(i) = LANGUAGES.iter().position(|(c, _)| *c == current.as_str()) {
                language.set_selected(i as u32);
            }
            let list = gtk::ListBox::builder()
                .css_classes(["boxed-list"])
                .selection_mode(gtk::SelectionMode::None)
                .build();
            list.append(&repo);
            list.append(&language);
            let dialog = adw::AlertDialog::builder()
                .heading(tr("Add a model"))
                .body(tr(
                    "Only sherpa-onnx streaming transducers work (encoder, decoder, joiner, tokens). The model is copied, then tested before use.",
                ))
                .extra_child(&list)
                .build();
            dialog.add_responses(&[
                ("cancel", &tr("Cancel")),
                ("folder", &tr("From a Folder…")),
                ("download", &tr("Download")),
            ]);
            dialog.set_response_appearance("download", adw::ResponseAppearance::Suggested);
            dialog.set_response_enabled("download", false);
            repo.connect_changed(glib::clone!(
                #[weak]
                dialog,
                move |e| dialog
                    .set_response_enabled("download", custom::parse_repo(&e.text()).is_ok())
            ));
            let (start, parent) = (start.clone(), window.clone());
            dialog.connect_response(None, move |_, response| {
                let lang = LANGUAGES[language.selected() as usize].0.to_owned();
                match response {
                    "download" => start(Source::HuggingFace(repo.text().to_string()), lang),
                    "folder" => {
                        let start = start.clone();
                        gtk::FileDialog::builder()
                            .title(tr("Folder containing the model"))
                            .build()
                            .select_folder(Some(&parent), gtk::gio::Cancellable::NONE, move |r| {
                                if let Some(path) = r.ok().and_then(|f| f.path()) {
                                    start(Source::Folder(path), lang);
                                }
                            });
                    }
                    _ => {}
                }
            });
            dialog.present(Some(&window));
        }
    ));
    row
}
