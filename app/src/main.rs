mod inject;

use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Sender, sync_channel};

use adw::prelude::*;
use gtk::{gio, glib};
use parlotte_core::download;
use parlotte_core::pipeline::{self, Notice};
use parlotte_core::registry::{self, Model};
use parlotte_core::stt::{SherpaTransducer, SttEngine};

const APP_ID: &str = "fr.gwenael_leger.Parlotte";

fn main() -> glib::ExitCode {
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.add_main_option(
        "toggle",
        b't'.into(),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Démarrer ou arrêter la dictée dans l'instance en cours",
        None,
    );
    app.connect_handle_local_options(|app, options| {
        if options.contains("toggle") {
            if let Err(e) = app.register(gio::Cancellable::NONE) {
                eprintln!("parlotte: {e}");
                return ControlFlow::Break(glib::ExitCode::FAILURE);
            }
            app.activate_action("toggle", None);
            if app.is_remote() {
                return ControlFlow::Break(glib::ExitCode::SUCCESS);
            }
        }
        ControlFlow::Continue(())
    });
    app.connect_startup(startup);
    app.connect_activate(|_| {});
    app.run()
}

/// GSettings, from the installed schema or the one compiled by build.rs in a source tree.
pub fn settings() -> gio::Settings {
    let installed = gio::SettingsSchemaSource::default().and_then(|s| s.lookup(APP_ID, true));
    let schema = installed
        .or_else(|| {
            gio::SettingsSchemaSource::from_directory(env!("OUT_DIR"), None, false)
                .ok()
                .and_then(|s| s.lookup(APP_ID, false))
        })
        .expect("GSettings schema not installed");
    gio::Settings::new_full(&schema, None::<&gio::SettingsBackend>, None)
}

pub fn models_root() -> PathBuf {
    glib::user_data_dir().join("parlotte").join("models")
}

/// The model selected for the current language, if any is registered.
pub fn selected_model(settings: &gio::Settings) -> Option<Model> {
    let (lang, id) = (settings.string("language"), settings.string("model"));
    registry::models()
        .into_iter()
        .find(|m| m.id == id.as_str() && m.langs.iter().any(|l| l == lang.as_str()))
        .or_else(|| registry::default_for(&lang))
}

/// Loads the selected model off the main thread and hands it to the pipeline.
fn load_engine(settings: &gio::Settings, engines: &Sender<Box<dyn SttEngine>>) {
    let Some(model) = selected_model(settings) else {
        return;
    };
    let root = models_root();
    if !download::is_installed(&root, &model) {
        return;
    }
    let engines = engines.clone();
    std::thread::spawn(move || {
        let files = model.files.iter().map(|f| f.path.as_str());
        match SherpaTransducer::load(&download::model_dir(&root, &model), files) {
            Ok(engine) => {
                let _ = engines.send(Box::new(engine));
            }
            Err(e) => on_main(move || notify_error(&e)),
        }
    });
}

/// Runs `f` on the GTK main thread.
fn on_main(f: impl FnOnce() + Send + 'static) {
    glib::MainContext::default().invoke(f);
}

fn notify_error(message: &str) {
    let Some(app) = gio::Application::default() else {
        return;
    };
    let n = gio::Notification::new("Parlotte");
    n.set_body(Some(message));
    app.send_notification(Some("error"), &n);
}

fn on_notice(notice: Notice) {
    match notice {
        Notice::Started | Notice::Stopped => {}
        Notice::Error(e) => notify_error(&e),
    }
}

fn startup(app: &adw::Application) {
    // No window: the application lives in the background until quit.
    std::mem::forget(app.hold());
    let settings = settings();

    let (text_tx, text_rx) = sync_channel::<String>(64);
    let handles = pipeline::spawn(text_tx, |n| on_main(move || on_notice(n)));

    let delay = Arc::new(AtomicU32::new(settings.uint("key-delay")));
    let token = Some(settings.string("restore-token").to_string()).filter(|t| !t.is_empty());
    let sink_delay = delay.clone();
    std::thread::Builder::new()
        .name("injection".into())
        .spawn(move || {
            let mut sink = inject::PortalSink::new(token, sink_delay, |t| {
                on_main(move || {
                    let _ = crate::settings().set_string("restore-token", &t);
                })
            });
            // Ask for keyboard access now rather than in the middle of the first dictation.
            if let Err(e) = sink.connect() {
                let e = format!("accès clavier refusé : {e}");
                on_main(move || notify_error(&e));
            }
            inject::run(text_rx, sink, |e| on_main(move || notify_error(&e)));
        })
        .expect("spawn injection thread");

    let sync = {
        let (pp, silence) = (handles.postprocess.clone(), handles.end_silence_ms.clone());
        move |s: &gio::Settings| {
            pp.store(s.boolean("postprocess"), Ordering::Relaxed);
            silence.store((s.double("end-silence") * 1000.0) as u32, Ordering::Relaxed);
            delay.store(s.uint("key-delay"), Ordering::Relaxed);
        }
    };
    sync(&settings);
    settings.connect_changed(None, move |s, key| match key {
        "language" | "model" => {}
        _ => sync(s),
    });
    let engines = handles.engine.clone();
    settings.connect_changed(None, move |s, key| {
        if matches!(key, "language" | "model") {
            load_engine(s, &engines);
        }
    });
    load_engine(&settings, &handles.engine);

    let toggle = gio::SimpleAction::new("toggle", None);
    let toggles = handles.toggle.clone();
    toggle.connect_activate(move |_, _| {
        let _ = toggles.send(());
    });
    app.add_action(&toggle);

    let quit = gio::SimpleAction::new("quit", None);
    quit.connect_activate(glib::clone!(
        #[weak]
        app,
        move |_, _| app.quit()
    ));
    app.add_action(&quit);

    // The settings object and its signal handlers live as long as the process.
    std::mem::forget(settings);
}
