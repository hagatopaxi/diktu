mod inject;
mod preferences;
mod shortcut;
mod tray;

use std::cell::RefCell;
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Sender, sync_channel};
use std::sync::{Arc, OnceLock};

use adw::prelude::*;
use diktu_core::pipeline::{self, Notice};
use diktu_core::registry::{self, Model};
use diktu_core::stt::{SherpaTransducer, SttEngine};
use diktu_core::{custom, download};
use gtk::{gio, glib};
use ksni::TrayMethods;
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

/// The development build (feature `devel`) is a separate app that installs and runs
/// next to the release: its own ID, settings, data folder, name and icon.
const DEVEL: bool = cfg!(feature = "devel");
pub const APP_ID: &str = if DEVEL {
    "fr.gwenael_leger.Diktu.Devel"
} else {
    "fr.gwenael_leger.Diktu"
};
pub const APP_NAME: &str = if DEVEL { "Diktu dev" } else { "Diktu" };
const RESOURCE_PREFIX: &str = "/fr/gwenael_leger/Diktu";
/// The launch itself activates the app: only later activations open a window.
const CHECK_FLAG: &str = "--check-model";
static LAUNCHED: AtomicBool = AtomicBool::new(false);

/// Everything other threads ask of the GTK main thread.
pub enum UiEvent {
    Notice(Notice),
    Trigger(String),
    RestoreToken(String),
    Preferences,
    Quit,
}

/// Shared runtime for portal and tray D-Bus traffic: ashpd and ksni tasks keep running
/// while other threads block on it.
pub fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .thread_name("portals")
            .enable_all()
            .build()
            .expect("tokio runtime")
    })
}

fn main() -> glib::ExitCode {
    if let [_, flag, dir] = &std::env::args().collect::<Vec<_>>()[..]
        && flag == CHECK_FLAG
    {
        // Our result goes to stdout: sherpa-onnx logs to stderr.
        return match custom::check(std::path::Path::new(dir)) {
            Ok(summary) => {
                println!("{summary}");
                glib::ExitCode::SUCCESS
            }
            Err(e) => {
                println!("{e}");
                glib::ExitCode::FAILURE
            }
        };
    }
    // zbus objects (portal proxies, tray) may only be dropped inside a tokio context.
    let _tokio = runtime().enter();
    gio::resources_register_include!("diktu.gresource").expect("embedded resources");
    let app = adw::Application::builder().application_id(APP_ID).build();
    app.add_main_option(
        "toggle",
        b't'.into(),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        "Start or stop dictation in the running instance",
        None,
    );
    app.connect_handle_local_options(|app, options| {
        if options.contains("toggle") {
            if let Err(e) = app.register(gio::Cancellable::NONE) {
                eprintln!("diktu: {e}");
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
    // Launching the app again (e.g. from the app grid) opens the preferences.
    app.connect_activate(|app| {
        if LAUNCHED.swap(true, Ordering::Relaxed) {
            app.activate_action("preferences", None);
        }
    });
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
    glib::user_data_dir()
        .join(if DEVEL { "diktu-dev" } else { "diktu" })
        .join("models")
}

/// Registered models, then the imported ones.
pub fn all_models() -> Vec<Model> {
    let mut models = registry::models();
    models.extend(custom::list(&models_root()));
    models
}

/// The model selected for the current language, if any exists.
pub fn selected_model(settings: &gio::Settings) -> Option<Model> {
    let (lang, id) = (settings.string("language"), settings.string("model"));
    let models: Vec<Model> = all_models()
        .into_iter()
        .filter(|m| m.langs.iter().any(|l| l == lang.as_str()))
        .collect();
    models
        .iter()
        .find(|m| m.id == id.as_str())
        .or(models.first())
        .cloned()
}

/// Time the model check may take; a large model on a slow CPU loads in tens of seconds.
const CHECK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// Runs [`custom::check`] in a child process, so a model that crashes sherpa-onnx
/// cannot take the application down. `Ok(None)` means the check did not finish in time.
pub fn check_model(dir: &std::path::Path) -> Result<Option<String>, String> {
    use std::io::Read;
    use std::process::Stdio;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut child = std::process::Command::new(exe)
        .arg(CHECK_FLAG)
        .arg(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start the model check: {e}"))?;
    // Pipes are drained on threads: a full pipe would block the child until the timeout.
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut p) = pipe {
                let _ = p.read_to_end(&mut buf);
            }
            String::from_utf8_lossy(&buf).into_owned()
        })
    };
    let stdout = drain(child.stdout.take().map(|p| Box::new(p) as _));
    let stderr = drain(child.stderr.take().map(|p| Box::new(p) as _));
    let start = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        if start.elapsed() > CHECK_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Ok(None);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    let message = stdout.join().unwrap_or_default().trim().to_owned();
    let stderr = stderr.join().unwrap_or_default();
    match status.code() {
        Some(0) => Ok(Some(message)),
        Some(_) if !message.is_empty() => Err(message),
        _ => {
            let last = stderr.lines().rev().find(|l| !l.trim().is_empty());
            Err(format!(
                "the speech engine crashed loading this model ({})",
                last.unwrap_or("no message")
            ))
        }
    }
}

/// Loads the selected model off the main thread and hands it to the pipeline.
fn load_engine(
    settings: &gio::Settings,
    engines: &Sender<Box<dyn SttEngine>>,
    events: &UnboundedSender<UiEvent>,
) {
    let Some(model) = selected_model(settings) else {
        return;
    };
    let root = models_root();
    if !download::is_installed(&root, &model) {
        return;
    }
    let (engines, events) = (engines.clone(), events.clone());
    std::thread::spawn(move || {
        let files = model.files.iter().map(|f| f.path.as_str());
        match SherpaTransducer::load(&download::model_dir(&root, &model), files) {
            Ok(engine) => {
                let _ = engines.send(Box::new(engine));
            }
            Err(e) => {
                let _ = events.send(UiEvent::Notice(Notice::Error(e)));
            }
        }
    });
}

fn notify_error(app: &adw::Application, message: &str) {
    let n = gio::Notification::new(APP_NAME);
    n.set_body(Some(message));
    app.send_notification(Some("error"), &n);
}

/// Plays an embedded sound on a short-lived output stream, so the device is not held open.
fn play_sound(settings: &gio::Settings, name: &str) {
    if !settings.boolean("sounds") {
        return;
    }
    let volume = settings.double("volume") as f32;
    let path = format!("{RESOURCE_PREFIX}/sounds/{name}.wav");
    let Ok(bytes) = gio::resources_lookup_data(&path, gio::ResourceLookupFlags::NONE) else {
        return;
    };
    let bytes = bytes.to_vec();
    std::thread::spawn(move || {
        let Ok(mut output) = rodio::DeviceSinkBuilder::open_default_sink() else {
            return;
        };
        output.log_on_drop(false);
        if let Ok(player) = rodio::play(output.mixer(), std::io::Cursor::new(bytes)) {
            player.set_volume(volume);
            player.sleep_until_end();
        }
    });
}

/// State owned by the main thread.
pub struct Ui {
    app: adw::Application,
    settings: gio::Settings,
    tray: Option<ksni::Handle<tray::Tray>>,
    events: UnboundedSender<UiEvent>,
    engines: Sender<Box<dyn SttEngine>>,
    /// Asks the GlobalShortcuts portal to show its configuration dialog.
    configure: UnboundedSender<()>,
    /// Shortcut as bound by the compositor, once the portal answered.
    trigger: RefCell<Option<String>>,
    preferences: RefCell<Option<adw::PreferencesWindow>>,
    shortcut_row: RefCell<Option<adw::ActionRow>>,
}

impl Ui {
    fn reload_engine(&self) {
        load_engine(&self.settings, &self.engines, &self.events);
    }

    /// Builds the window anew, for when the list of models changed.
    fn rebuild_preferences(self: &Rc<Self>) {
        if let Some(old) = self.preferences.take() {
            old.destroy();
        }
        self.show_preferences();
    }

    fn show_preferences(self: &Rc<Self>) {
        let window = self
            .preferences
            .borrow_mut()
            .get_or_insert_with(|| preferences::build(self))
            .clone();
        window.set_application(Some(&self.app));
        window.present();
    }

    fn handle(self: &Rc<Self>, event: UiEvent) {
        if let UiEvent::Notice(n) = &event {
            glib::g_debug!("diktu", "{n:?}");
        }
        match event {
            UiEvent::Notice(Notice::Started) => {
                play_sound(&self.settings, "start");
                self.set_recording(true);
            }
            UiEvent::Notice(Notice::Stopped) => {
                play_sound(&self.settings, "stop");
                self.set_recording(false);
            }
            UiEvent::Notice(Notice::Error(e)) => notify_error(&self.app, &e),
            UiEvent::Trigger(t) => {
                if let Some(row) = self.shortcut_row.borrow().as_ref() {
                    row.set_subtitle(&t);
                }
                *self.trigger.borrow_mut() = Some(t);
            }
            UiEvent::RestoreToken(t) => {
                let _ = self.settings.set_string("restore-token", &t);
            }
            UiEvent::Preferences => self.show_preferences(),
            UiEvent::Quit => self.app.quit(),
        }
    }

    fn set_recording(&self, recording: bool) {
        if let Some(tray) = self.tray.clone() {
            runtime().spawn(async move { tray.update(|t| t.recording = recording).await });
        }
    }
}

fn startup(app: &adw::Application) {
    // No window: the application lives in the background until quit.
    std::mem::forget(app.hold());
    let settings = settings();
    let (events, mut event_rx) = unbounded_channel::<UiEvent>();

    // Portals identify a non-Flatpak app by this registration; it is a no-op in a sandbox.
    if let Err(e) = runtime().block_on(ashpd::register_host_app(APP_ID.try_into().unwrap())) {
        eprintln!("diktu: portal registration: {e}");
    }

    let (text_tx, text_rx) = sync_channel::<String>(64);
    let notices = events.clone();
    let handles = pipeline::spawn(text_tx, move |n| {
        let _ = notices.send(UiEvent::Notice(n));
    });

    let delay = Arc::new(AtomicU32::new(settings.uint("key-delay")));
    let token = Some(settings.string("restore-token").to_string()).filter(|t| !t.is_empty());
    let (sink_delay, sink_events) = (delay.clone(), events.clone());
    std::thread::Builder::new()
        .name("injection".into())
        .spawn(move || {
            let _tokio = runtime().enter();
            let tokens = sink_events.clone();
            let mut sink = inject::PortalSink::new(token, sink_delay, move |t| {
                let _ = tokens.send(UiEvent::RestoreToken(t));
            });
            let error = |e: String| {
                let _ = sink_events.send(UiEvent::Notice(Notice::Error(e)));
            };
            // Ask for keyboard access now rather than in the middle of the first dictation;
            // `run` closes this session once idle.
            if let Err(e) = sink.connect() {
                error(format!("keyboard access denied: {e}"));
            }
            inject::run(text_rx, sink, error);
        })
        .expect("spawn injection thread");

    let (configure, configure_rx) = unbounded_channel();
    let (toggle, trigger_events) = (handles.toggle.clone(), events.clone());
    runtime().spawn(async move {
        let on_trigger = |t| {
            let _ = trigger_events.send(UiEvent::Trigger(t));
        };
        if let Err(e) = shortcut::run(toggle, configure_rx, on_trigger).await {
            let message = format!(
                "global shortcut unavailable ({e}): bind a GNOME shortcut to “diktu --toggle”"
            );
            let _ = trigger_events.send(UiEvent::Notice(Notice::Error(message)));
        }
    });

    let background = runtime().block_on(async {
        ashpd::desktop::background::Background::request()
            .reason("Listen for the dictation shortcut with no window open")
            .send()
            .await
    });
    if let Err(e) = background {
        eprintln!("diktu: Background portal: {e}");
    }

    let tray = tray::Tray::new(handles.toggle.clone(), events.clone());
    let sandboxed = std::path::Path::new("/.flatpak-info").exists();
    let tray = runtime().block_on(async {
        // A sandbox may not own the per-process D-Bus name the SNI spec asks for.
        if sandboxed {
            tray.disable_dbus_name(true).spawn().await
        } else {
            tray.spawn().await
        }
    });
    let tray = tray
        .inspect_err(|e| eprintln!("diktu: tray icon unavailable: {e}"))
        .ok();

    let sync = {
        let (pp, silence) = (handles.postprocess.clone(), handles.end_silence_ms.clone());
        move |s: &gio::Settings| {
            pp.store(s.boolean("postprocess"), Ordering::Relaxed);
            silence.store((s.double("end-silence") * 1000.0) as u32, Ordering::Relaxed);
            delay.store(s.uint("key-delay"), Ordering::Relaxed);
        }
    };
    sync(&settings);
    let (engines, engine_events) = (handles.engine.clone(), events.clone());
    settings.connect_changed(None, move |s, key| {
        if matches!(key, "language" | "model") {
            load_engine(s, &engines, &engine_events);
        } else {
            sync(s);
        }
    });
    load_engine(&settings, &handles.engine, &events);

    let ui = Rc::new(Ui {
        app: app.clone(),
        settings,
        tray,
        events,
        engines: handles.engine.clone(),
        configure,
        trigger: RefCell::new(None),
        preferences: RefCell::new(None),
        shortcut_row: RefCell::new(None),
    });

    let action = gio::SimpleAction::new("toggle", None);
    let toggle = handles.toggle.clone();
    action.connect_activate(move |_, _| {
        let _ = toggle.send(());
    });
    app.add_action(&action);

    let action = gio::SimpleAction::new("preferences", None);
    action.connect_activate(glib::clone!(
        #[weak]
        ui,
        move |_, _| ui.show_preferences()
    ));
    app.add_action(&action);

    let action = gio::SimpleAction::new("quit", None);
    action.connect_activate(glib::clone!(
        #[weak]
        app,
        move |_, _| app.quit()
    ));
    app.add_action(&action);

    // Nothing to dictate with until a model is downloaded: show where to get one.
    if selected_model(&ui.settings).is_none_or(|m| !download::is_installed(&models_root(), &m)) {
        ui.show_preferences();
    }

    glib::spawn_future_local(async move {
        while let Some(event) = event_rx.recv().await {
            ui.handle(event);
        }
    });
}
