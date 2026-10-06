//! First-launch assistant, one step at a time: language and model, permissions, shortcut.
//! Each system dialog only opens when its step asks for it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use adw::prelude::*;
use diktu_core::download::{self, Error};
use diktu_core::i18n::{tr, trf};
use diktu_core::registry::Model;
use gtk::glib;
use tokio::sync::oneshot;

use crate::preferences::{Progress, language_name};
use crate::{APP_ID, APP_NAME, Consent, Ui, all_models, language, models_root, runtime};

/// Shown by the shortcut step: the compositor has the final say on the key.
const PREFERRED_SHORTCUT: &str = "F12";

pub fn show(ui: &Rc<Ui>, consent: std::sync::mpsc::Sender<Consent>, bind: oneshot::Sender<()>) {
    let nav = adw::NavigationView::new();
    // The model downloads while the next steps run.
    let progress = gtk::ProgressBar::builder()
        .show_text(true)
        .visible(false)
        .margin_start(24)
        .margin_end(24)
        .margin_top(12)
        .margin_bottom(12)
        .build();
    let view = adw::ToolbarView::new();
    view.set_content(Some(&nav));
    view.add_top_bar(&progress);
    let window = adw::Window::builder()
        .title(APP_NAME)
        .default_width(560)
        .default_height(760)
        .content(&view)
        .application(&ui.app)
        .build();

    // The first page added is shown; the others wait in the pool for their tag.
    nav.add(&language_page(ui, &nav, &progress));
    nav.add(&permissions_page(&nav, consent));
    nav.add(&shortcut_page(ui, &nav, bind));
    nav.add(&finish_page(ui, &window));
    window.present();
}

/// A step: header bar, icon, title, explanation, content, then its buttons.
fn page(
    tag: &str,
    icon: &str,
    title: &str,
    description: &str,
    content: Option<&gtk::Widget>,
    buttons: &[&gtk::Button],
) -> adw::NavigationPage {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 24);
    if let Some(content) = content {
        column.append(content);
    }
    let actions = gtk::Box::new(gtk::Orientation::Vertical, 12);
    actions.set_halign(gtk::Align::Center);
    for b in buttons {
        actions.append(*b);
    }
    column.append(&actions);
    let status = adw::StatusPage::builder()
        .icon_name(icon)
        .title(title)
        .description(description)
        .child(
            &adw::Clamp::builder()
                .maximum_size(420)
                .child(&column)
                .build(),
        )
        .build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&status));
    adw::NavigationPage::builder()
        .tag(tag)
        .title(APP_NAME)
        .child(&toolbar)
        .build()
}

fn button(label: &str, suggested: bool) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.add_css_class(if suggested {
        "suggested-action"
    } else {
        "flat"
    });
    button.add_css_class("pill");
    button
}

fn boxed_list(rows: &[&adw::ActionRow]) -> gtk::Widget {
    let list = gtk::ListBox::builder()
        .css_classes(["boxed-list"])
        .selection_mode(gtk::SelectionMode::None)
        .build();
    for row in rows {
        list.append(*row);
    }
    list.upcast()
}

/// The model being downloaded, and its cancel flag.
type Download = (String, Arc<AtomicBool>);

/// Step 1: the language, and the model for it, which starts downloading.
fn language_page(
    ui: &Rc<Ui>,
    nav: &adw::NavigationView,
    progress: &gtk::ProgressBar,
) -> adw::NavigationPage {
    let models = all_models();
    let mut languages: Vec<String> = models.iter().flat_map(|m| m.langs.clone()).collect();
    languages.sort();
    languages.dedup();
    let names: Vec<&str> = languages.iter().map(|l| language_name(l)).collect();
    let language_row = adw::ComboRow::builder()
        .title(tr("Language"))
        .model(&gtk::StringList::new(&names))
        .build();
    let current = language(&ui.settings);
    if let Some(i) = languages.iter().position(|l| *l == current) {
        language_row.set_selected(i as u32);
    }
    let model_row = adw::ComboRow::builder().title(tr("Model")).build();
    // The models of the selected language, in the order of the model row.
    let listed: Rc<RefCell<Vec<Model>>> = Rc::default();
    let describe = glib::clone!(
        #[strong]
        listed,
        move |row: &adw::ComboRow| {
            if let Some(m) = listed.borrow().get(row.selected() as usize) {
                let size = (m.total_size() + 500_000) / 1_000_000;
                row.set_subtitle(&trf(
                    "{size} MB · license {license}",
                    &[("size", &size), ("license", &m.license)],
                ));
            }
        }
    );
    model_row.connect_selected_notify(describe.clone());
    let fill = glib::clone!(
        #[strong]
        listed,
        #[weak]
        model_row,
        move |lang: &str| {
            let models: Vec<Model> = models
                .iter()
                .filter(|m| m.langs.iter().any(|l| l == lang))
                .cloned()
                .collect();
            let names: Vec<&str> = models.iter().map(|m| m.name.as_str()).collect();
            *listed.borrow_mut() = models.clone();
            model_row.set_model(Some(&gtk::StringList::new(&names)));
            describe(&model_row);
        }
    );
    fill(&current);
    let languages = Rc::new(languages);
    language_row.connect_selected_notify(glib::clone!(
        #[strong]
        languages,
        move |row| fill(&languages[row.selected() as usize])
    ));
    let list = gtk::ListBox::builder()
        .css_classes(["boxed-list"])
        .selection_mode(gtk::SelectionMode::None)
        .build();
    list.append(&language_row);
    list.append(&model_row);

    let next_button = button(&tr("Continue"), true);
    let page = page(
        "language",
        APP_ID,
        &trf("Welcome to {app}", &[("app", &APP_NAME)]),
        &tr(
            "Dictate text anywhere, recognized on your computer. Which language will you speak? Its model downloads while you set up the rest.",
        ),
        Some(list.upcast_ref()),
        &[&next_button],
    );
    let download: Rc<RefCell<Option<Download>>> = Rc::default();
    next_button.connect_clicked(glib::clone!(
        #[strong]
        ui,
        #[weak]
        nav,
        #[weak]
        progress,
        move |_| {
            let lang = &languages[language_row.selected() as usize];
            let Some(model) = listed.borrow().get(model_row.selected() as usize).cloned() else {
                return;
            };
            let _ = ui.settings.set_string("language", lang);
            let _ = ui.settings.set_string("model", &model.id);
            nav.push_by_tag("permissions");
            // Going back and picking another model cancels the first download.
            let mut current = download.borrow_mut();
            if current.as_ref().is_some_and(|(id, _)| *id == model.id) {
                return;
            }
            if let Some((_, cancel)) = current.take() {
                cancel.store(true, Ordering::Relaxed);
            }
            if download::is_installed(&models_root(), &model) {
                progress.set_visible(false);
                return;
            }
            let cancel = Arc::new(AtomicBool::new(false));
            *current = Some((model.id.clone(), cancel.clone()));
            start_download(&ui, model, cancel, &progress, &nav);
        }
    ));
    page
}

fn start_download(
    ui: &Rc<Ui>,
    model: Model,
    cancel: Arc<AtomicBool>,
    progress: &gtk::ProgressBar,
    nav: &adw::NavigationView,
) {
    progress.set_visible(true);
    progress.set_fraction(0.0);
    progress.set_text(Some(&trf(
        "Downloading {model}…",
        &[("model", &model.name)],
    )));
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let thread_model = model.clone();
    std::thread::spawn(move || {
        let result = download::install(
            download::HF_BASE_URL,
            &models_root(),
            &thread_model,
            &cancel,
            |done, total| {
                let _ = tx.send(Progress::Bytes(done, total));
            },
        );
        let _ = tx.send(Progress::Done(result));
    });
    glib::spawn_future_local(glib::clone!(
        #[strong]
        ui,
        #[weak]
        progress,
        #[weak]
        nav,
        async move {
            while let Some(p) = rx.recv().await {
                match p {
                    Progress::Bytes(done, total) => {
                        progress.set_fraction(done as f64 / total.max(1) as f64);
                        progress.set_text(Some(&trf(
                            "Downloading {model}… {done} / {total} MB",
                            &[
                                ("model", &model.name),
                                ("done", &(done / 1_000_000)),
                                ("total", &((total + 500_000) / 1_000_000)),
                            ],
                        )));
                    }
                    Progress::Done(Ok(())) => {
                        progress.set_fraction(1.0);
                        progress
                            .set_text(Some(&trf("{model} is ready", &[("model", &model.name)])));
                        ui.reload_engine();
                        // The bar stays until the user moves on to another step.
                        let handler: Rc<RefCell<Option<glib::SignalHandlerId>>> = Rc::default();
                        let id = nav.connect_visible_page_notify(glib::clone!(
                            #[weak]
                            progress,
                            #[strong]
                            handler,
                            move |nav| {
                                progress.set_visible(false);
                                if let Some(id) = handler.take() {
                                    nav.disconnect(id);
                                }
                            }
                        ));
                        *handler.borrow_mut() = Some(id);
                    }
                    Progress::Done(Err(Error::Cancelled)) => {}
                    Progress::Done(Err(e)) => progress.set_text(Some(&trf(
                        "Download failed: {error}. Retry from the preferences.",
                        &[("error", &e)],
                    ))),
                }
            }
        }
    ));
}

/// Step 2: each permission explained, then the system dialogs one after the other.
fn permissions_page(
    nav: &adw::NavigationView,
    consent: std::sync::mpsc::Sender<Consent>,
) -> adw::NavigationPage {
    let row = |icon: &str, title: String, subtitle: String| {
        let row = adw::ActionRow::builder()
            .title(title)
            .subtitle(subtitle)
            .build();
        row.add_prefix(&gtk::Image::from_icon_name(icon));
        row
    };
    let microphone = row(
        "audio-input-microphone-symbolic",
        tr("Microphone"),
        tr("Open only while you dictate. No audio leaves your computer."),
    );
    let background = row(
        "preferences-system-symbolic",
        tr("Run in the background"),
        tr("Listen for the shortcut with no window open."),
    );
    let keyboard = row(
        "input-keyboard-symbolic",
        tr("Type at the cursor"),
        trf(
            "Type the recognized text in the focused app. GNOME shows a remote control icon while {app} types.",
            &[("app", &APP_NAME)],
        ),
    );
    let allow = button(&tr("Allow…"), true);
    let skip = button(&tr("Skip"), false);
    let page = page(
        "permissions",
        "channel-secure-symbolic",
        &tr("Permissions"),
        &tr("GNOME asks you to confirm each one, one dialog after the other."),
        Some(&boxed_list(&[&microphone, &background, &keyboard])),
        &[&allow, &skip],
    );
    skip.connect_clicked(glib::clone!(
        #[weak]
        nav,
        move |_| nav.push_by_tag("shortcut")
    ));
    let done = |row: &adw::ActionRow, result: Result<(), String>| {
        let icon = match &result {
            Ok(()) => "object-select-symbolic",
            Err(_) => "dialog-warning-symbolic",
        };
        row.add_suffix(&gtk::Image::from_icon_name(icon));
        if let Err(e) = result {
            row.set_subtitle(&trf(
                "Not granted ({error}). It will be asked again later.",
                &[("error", &e)],
            ));
        }
    };
    let granted = Rc::new(Cell::new(false));
    allow.connect_clicked(glib::clone!(
        #[weak]
        nav,
        move |allow| {
            if granted.get() {
                nav.push_by_tag("shortcut");
                return;
            }
            allow.set_sensitive(false);
            let (reply, answer) = oneshot::channel();
            let _ = consent.send(reply);
            glib::spawn_future_local(glib::clone!(
                #[weak]
                nav,
                #[weak]
                allow,
                #[weak]
                background,
                #[weak]
                keyboard,
                #[strong]
                granted,
                async move {
                    let result = runtime().spawn(crate::request_background()).await;
                    done(&background, result.unwrap_or_else(|e| Err(e.to_string())));
                    // The keyboard dialog only opens once the background one is answered.
                    let result = answer.await.unwrap_or_else(|e| Err(e.to_string()));
                    let ok = result.is_ok();
                    done(&keyboard, result);
                    granted.set(true);
                    allow.set_label(&tr("Continue"));
                    allow.set_sensitive(true);
                    if ok {
                        nav.push_by_tag("shortcut");
                    }
                }
            ));
        }
    ));
    page
}

/// Step 3: the global shortcut, chosen in GNOME's dialog.
fn shortcut_page(
    ui: &Rc<Ui>,
    nav: &adw::NavigationView,
    bind: oneshot::Sender<()>,
) -> adw::NavigationPage {
    let row = adw::ActionRow::builder()
        .title(tr("Start or stop dictation"))
        .subtitle(tr("Not chosen yet"))
        .build();
    ui.shortcut_rows.borrow_mut().push(row.downgrade());
    let choose = button(&tr("Choose the Shortcut…"), true);
    let skip = button(&tr("Skip"), false);
    let page = page(
        "shortcut",
        "input-keyboard-symbolic",
        &tr("Keyboard Shortcut"),
        &trf(
            "Press it anywhere to start dictating, and again to stop. {app} suggests {key}; GNOME lets you confirm it or pick another one.\n\nOn a laptop, {key} often controls the volume or brightness: press Fn+{key}, or turn on Fn Lock (often Fn+Esc).",
            &[("app", &APP_NAME), ("key", &PREFERRED_SHORTCUT)],
        ),
        Some(&boxed_list(&[&row])),
        &[&choose, &skip],
    );
    skip.connect_clicked(glib::clone!(
        #[weak]
        nav,
        move |_| nav.push_by_tag("finish")
    ));
    // The first click binds the shortcut; later ones open GNOME's dialog again, which
    // portal v1 cannot do: the button then turns insensitive.
    let bind = RefCell::new(Some(bind));
    let ui = ui.clone();
    choose.connect_clicked(move |b| match bind.take() {
        Some(bind) => {
            let _ = bind.send(());
        }
        None => ui.change_shortcut(|| b.set_sensitive(false)),
    });
    let pushed = Cell::new(false);
    row.connect_subtitle_notify(glib::clone!(
        #[weak]
        nav,
        move |_| {
            if !pushed.replace(true) {
                nav.push_by_tag("finish");
            }
        }
    ));
    page
}

/// Step 4: how to dictate; closing marks the onboarding as done.
fn finish_page(ui: &Rc<Ui>, window: &adw::Window) -> adw::NavigationPage {
    let start = button(&trf("Start Using {app}", &[("app", &APP_NAME)]), true);
    let page = page(
        "finish",
        "object-select-symbolic",
        &tr("All Set"),
        "",
        None,
        &[&start],
    );
    page.connect_shown(glib::clone!(
        #[strong]
        ui,
        move |page| {
            let start = match ui.trigger.borrow().as_deref() {
                Some(t) => trf("Press {key} anywhere", &[("key", &t)]),
                None => trf(
                    "Choose “Start dictation” in the {app} menu of the top bar",
                    &[("app", &APP_NAME)],
                ),
            };
            let ready = crate::selected_model(&ui.settings)
                .is_some_and(|m| download::is_installed(&models_root(), &m));
            let wait = if ready {
                String::new()
            } else {
                format!(" {}", tr("Dictation works once the model is downloaded."))
            };
            let status = page
                .child()
                .and_downcast::<adw::ToolbarView>()
                .and_then(|t| t.content())
                .and_downcast::<adw::StatusPage>();
            if let Some(status) = status {
                status.set_description(Some(&format!(
                    "{}{wait}\n\n{}",
                    trf(
                        "{start}, speak, then pause: the text is typed at the cursor.",
                        &[("start", &start)]
                    ),
                    trf(
                        "The {app} menu in the top bar also opens the preferences.",
                        &[("app", &APP_NAME)]
                    )
                )));
            }
        }
    ));
    start.connect_clicked(glib::clone!(
        #[strong]
        ui,
        #[weak]
        window,
        move |_| {
            let _ = ui.settings.set_boolean("onboarded", true);
            window.close();
        }
    ));
    page
}
