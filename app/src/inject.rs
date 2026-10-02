//! Types text through the XDG RemoteDesktop portal (`NotifyKeyboardKeysym`).

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::Receiver;
use std::time::Duration;

use ashpd::desktop::PersistMode;
use ashpd::desktop::Session;
use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop, SelectDevicesOptions};
use parlotte_core::inject::{TextSink, keysym};

pub struct PortalSink {
    runtime: tokio::runtime::Runtime,
    connection: Option<(RemoteDesktop, Session<RemoteDesktop>)>,
    restore_token: Option<String>,
    /// Pause between two key events, in milliseconds.
    delay_ms: Arc<AtomicU32>,
    on_token: Box<dyn Fn(String)>,
}

impl PortalSink {
    /// `on_token` receives each new restore token, so that consent survives restarts.
    pub fn new(
        restore_token: Option<String>,
        delay_ms: Arc<AtomicU32>,
        on_token: impl Fn(String) + 'static,
    ) -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .expect("tokio runtime");
        Self {
            runtime,
            connection: None,
            restore_token,
            delay_ms,
            on_token: Box::new(on_token),
        }
    }

    /// Opens the session, showing the consent dialog unless a restore token is accepted.
    pub fn connect(&mut self) -> Result<(), ashpd::Error> {
        let token = self.restore_token.clone();
        let (proxy, session, new_token) = self.runtime.block_on(async {
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

    fn send(&self, text: &str) -> Result<(), ashpd::Error> {
        let (proxy, session) = self.connection.as_ref().expect("connected");
        let delay = Duration::from_millis(self.delay_ms.load(Ordering::Relaxed).into());
        self.runtime.block_on(async {
            for k in text.chars().filter_map(keysym) {
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

/// Injection thread: types every chunk received, reports failures.
pub fn run(texts: Receiver<String>, mut sink: impl TextSink, on_error: impl Fn(String)) {
    for text in texts {
        if let Err(e) = sink.type_text(&text) {
            on_error(e);
        }
    }
}
