//! Preferences window: language, models, shortcut, dictation behaviour.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use adw::prelude::*;
use gtk::glib;
use parlotte_core::download::{self, Error};
use parlotte_core::registry::{self, Model};

use crate::{Ui, models_root, selected_model};

fn language_name(code: &str) -> &str {
    match code {
        "fr" => "Français",
        "en" => "English",
        "de" => "Deutsch",
        "es" => "Español",
        "it" => "Italiano",
        other => other,
    }
}

pub fn build(ui: &Rc<Ui>) -> adw::PreferencesWindow {
    let window = adw::PreferencesWindow::builder()
        .title("Réglages de Parlotte")
        .default_width(600)
        .default_height(680)
        .search_enabled(false)
        // Downloads keep running, and their rows stay current, while the window is closed.
        .hide_on_close(true)
        .build();
    let page = adw::PreferencesPage::new();
    window.add(&page);
    let settings = &ui.settings;

    let mut languages: Vec<String> = registry::models()
        .into_iter()
        .flat_map(|m| m.langs)
        .collect();
    languages.sort();
    languages.dedup();
    let names: Vec<&str> = languages.iter().map(|l| language_name(l)).collect();
    let language = adw::ComboRow::builder()
        .title("Langue")
        .model(&gtk::StringList::new(&names))
        .build();
    let current = settings.string("language");
    if let Some(i) = languages.iter().position(|l| *l == current.as_str()) {
        language.set_selected(i as u32);
    }
    let group = adw::PreferencesGroup::builder().title("Dictée").build();
    group.add(&language);
    page.add(&group);

    let group = adw::PreferencesGroup::builder()
        .title("Modèles")
        .description(
            "Téléchargés depuis Hugging Face, vérifiés (SHA-256), puis utilisés hors ligne.",
        )
        .build();
    let mut first_check: Option<gtk::CheckButton> = None;
    let rows: Vec<(Model, adw::ActionRow)> = registry::models()
        .into_iter()
        .map(|m| {
            let row = model_row(ui, &window, &m, &mut first_check);
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
        .title("Raccourci")
        .description("Le raccourci est attribué par GNOME. Repli : un raccourci personnalisé GNOME qui lance « parlotte --toggle ».")
        .build();
    let shortcut = adw::ActionRow::builder()
        .title("Démarrer ou arrêter la dictée")
        .subtitle(
            ui.trigger
                .borrow()
                .as_deref()
                .unwrap_or("Portail GlobalShortcuts indisponible"),
        )
        .build();
    let change = gtk::Button::builder()
        .label("Modifier…")
        .valign(gtk::Align::Center)
        .build();
    let configure = ui.configure.clone();
    let win = window.downgrade();
    change.connect_clicked(move |_| {
        if configure.send(()).is_err()
            && let Some(window) = win.upgrade()
        {
            window.add_toast(adw::Toast::new(
                "À modifier dans Paramètres → Applications → Parlotte",
            ));
        }
    });
    shortcut.add_suffix(&change);
    group.add(&shortcut);
    *ui.shortcut_row.borrow_mut() = Some(shortcut);
    page.add(&group);

    let group = adw::PreferencesGroup::builder()
        .title("Comportement")
        .build();
    let silence = adw::SpinRow::with_range(0.5, 5.0, 0.1);
    silence.set_title("Silence de fin (s)");
    silence.set_subtitle("Durée de silence après la parole qui arrête l'écoute");
    silence.set_digits(1);
    settings.bind("end-silence", &silence, "value").build();
    group.add(&silence);
    let postprocess = adw::SwitchRow::builder()
        .title("Majuscule initiale et point final")
        .build();
    settings.bind("postprocess", &postprocess, "active").build();
    group.add(&postprocess);
    let delay = adw::SpinRow::with_range(0.0, 100.0, 1.0);
    delay.set_title("Pause entre les touches (ms)");
    delay.set_subtitle("À augmenter si des caractères manquent dans certaines applications");
    settings.bind("key-delay", &delay, "value").build();
    group.add(&delay);
    let sounds = adw::SwitchRow::builder()
        .title("Sons de début et de fin")
        .build();
    settings.bind("sounds", &sounds, "active").build();
    group.add(&sounds);
    let volume = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 1.0, 0.05);
    volume.set_hexpand(true);
    volume.set_valign(gtk::Align::Center);
    settings
        .bind("volume", &volume.adjustment(), "value")
        .build();
    let row = adw::ActionRow::builder().title("Volume").build();
    row.add_suffix(&volume);
    sounds
        .bind_property("active", &row, "sensitive")
        .sync_create()
        .build();
    group.add(&row);
    page.add(&group);

    window
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
    first_check: &mut Option<gtk::CheckButton>,
) -> adw::ActionRow {
    let size = format!("{} Mo", (model.total_size() + 500_000) / 1_000_000);
    let row = adw::ActionRow::builder().title(&model.name).build();

    // A choice only exists when a language has several models.
    let alternatives = registry::models()
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
    let download = icon_button("folder-download-symbolic", "Télécharger");
    let cancel = icon_button("process-stop-symbolic", "Annuler le téléchargement");
    let remove = icon_button("user-trash-symbolic", "Supprimer");
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
                "téléchargement"
            } else if installed {
                "installé"
            } else {
                "non installé"
            };
            row.set_subtitle(&format!("{size} · {state} · licence {license}"));
            progress.set_visible(busy);
            cancel.set_visible(busy);
            download.set_visible(!busy && !installed);
            remove.set_visible(installed);
        })
    };
    refresh();

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
                                progress.set_text(Some(&format!(
                                    "{} / {} Mo",
                                    done / 1_000_000,
                                    (total + 500_000) / 1_000_000
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
        #[weak]
        window,
        #[to_owned]
        model,
        move |_| {
            if let Err(e) = download::remove(&models_root(), &model) {
                window.add_toast(adw::Toast::new(&format!("Suppression impossible : {e}")));
            }
            refresh();
        }
    ));

    row
}
