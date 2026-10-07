//! Types text through the XDG RemoteDesktop portal (`NotifyKeyboardKeysym`).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ashpd::desktop::PersistMode;
use ashpd::desktop::Session;
use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions};
use diktu_core::inject::{Keymap, TextSink, keysyms};
use gtk::gdk;
use gtk::glib::translate::IntoGlib;
use gtk::prelude::*;

/// The keyboard layouts of the session, read on the GTK thread.
pub fn keymap() -> Keymap {
    let mut keymap = Keymap {
        keysym: |c| gdk::unicode_to_keyval(c as u32),
        ..Keymap::default()
    };
    let Some(display) = gdk::Display::default() else {
        return keymap;
    };
    // evdev keycodes, offset by 8 as in xkb.
    for keycode in 8..256 {
        for (key, keyval) in display.map_keycode(keycode).unwrap_or_default() {
            // GNOME types levels 1 to 3 only: none, Shift, AltGr.
            if key.level() > 2 {
                continue;
            }
            let group = key.group() as usize;
            if keymap.layouts.len() <= group {
                keymap.layouts.resize_with(group + 1, Default::default);
            }
            keymap.layouts[group].insert(keyval.into_glib());
        }
    }
    if let Some(keyboard) = display.default_seat().and_then(|s| s.keyboard()) {
        keymap.active = keyboard.property::<i32>("active-layout-index").max(0) as usize;
    }
    keymap
}

pub struct PortalSink {
    connection: Option<(RemoteDesktop, Session<RemoteDesktop>)>,
    restore_token: Option<String>,
    keymap: Arc<Mutex<Keymap>>,
    /// Pause between two key events, in milliseconds.
    delay_ms: Arc<AtomicU32>,
    on_token: Box<dyn Fn(String)>,
}

impl PortalSink {
    /// `on_token` receives each new restore token, so that consent survives restarts.
    pub fn new(
        restore_token: Option<String>,
        keymap: Arc<Mutex<Keymap>>,
        delay_ms: Arc<AtomicU32>,
        on_token: impl Fn(String) + 'static,
    ) -> Self {
        Self {
            connection: None,
            restore_token,
            keymap,
            delay_ms,
            on_token: Box::new(on_token),
        }
    }

    /// Opens the session, showing the consent dialog unless a restore token is accepted.
    pub fn connect(&mut self) -> Result<(), ashpd::Error> {
        let token = self.restore_token.clone();
        let (proxy, session, new_token) = crate::runtime().block_on(async {
            let proxy = RemoteDesktop::new().await?;
            let session = proxy.create_session(Default::default()).await?;
            proxy
                .select_devices(
                    &session,
                    SelectDevicesOptions::default()
                        .set_devices(ashpd::enumflags2::BitFlags::from(DeviceType::Keyboard))
                        .set_persist_mode(PersistMode::ExplicitlyRevoked)
                        .set_restore_token(token.as_deref()),
                )
                .await?;
            let selected = proxy
                .start(&session, None, Default::default())
                .await?
                .response()?;
            let new_token = selected.restore_token().map(str::to_owned);
            Ok::<_, ashpd::Error>((proxy, session, new_token))
        })?;
        if let Some(t) = new_token {
            self.restore_token = Some(t.clone());
            (self.on_token)(t);
        }
        self.connection = Some((proxy, session));
        Ok(())
    }

    /// Closes the session, which removes GNOME's remote-control indicator; the restore token
    /// lets the next `connect` skip the consent dialog.
    pub fn disconnect(&mut self) {
        if let Some((_, session)) = self.connection.take() {
            let _ = crate::runtime().block_on(session.close());
        }
    }

    fn send(&self, text: &str) -> Result<(), ashpd::Error> {
        let (proxy, session) = self.connection.as_ref().expect("connected");
        let delay = Duration::from_millis(self.delay_ms.load(Ordering::Relaxed).into());
        let keys: Vec<u32> = {
            let keymap = self.keymap.lock().unwrap();
            text.chars()
                .flat_map(|c| {
                    let keys = keysyms(c, &keymap);
                    if keys.is_empty() {
                        gtk::glib::g_debug!("diktu", "no keyboard layout types {c:?}");
                    }
                    keys
                })
                .collect()
        };
        crate::runtime().block_on(async {
            for k in keys {
                for state in [KeyState::Pressed, KeyState::Released] {
                    proxy
                        .notify_keyboard_keysym(session, k as i32, state, Default::default())
                        .await?;
                }
                tokio::time::sleep(delay).await;
            }
            Ok(())
        })
    }
}

impl TextSink for PortalSink {
    fn type_text(&mut self, text: &str) -> Result<(), String> {
        if self.connection.is_none() {
            self.connect().map_err(|e| e.to_string())?;
        }
        if self.send(text).is_ok() {
            return Ok(());
        }
        // The session may have been closed (consent revoked, portal restarted): retry once.
        self.connection = None;
        self.connect().map_err(|e| e.to_string())?;
        self.send(text).map_err(|e| e.to_string())
    }
}

/// Idle time after which the keyboard session is closed. It outlasts the gaps between the
/// model's bursts, so one dictation keeps one session.
const IDLE: Duration = Duration::from_secs(3);

/// Injection thread: types every chunk received, reports failures, and keeps the session open
/// only while text flows.
pub fn run(texts: Receiver<String>, mut sink: PortalSink, on_error: impl Fn(String)) {
    loop {
        match texts.recv_timeout(IDLE) {
            Ok(text) => {
                gtk::glib::g_debug!("diktu", "typing {text:?}");
                if let Err(e) = sink.type_text(&text) {
                    on_error(e);
                }
            }
            Err(RecvTimeoutError::Timeout) => sink.disconnect(),
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}
