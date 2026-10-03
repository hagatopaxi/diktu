//! Global toggle shortcut through the XDG GlobalShortcuts portal.

use diktu_core::i18n::tr;
use std::sync::mpsc::Sender;

use ashpd::desktop::Session;
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut, Shortcut};
use futures_util::StreamExt;
use tokio::sync::mpsc::UnboundedReceiver;

/// F12 alone: one free key, reachable without looking; the compositor has the final say.
const PREFERRED_TRIGGER: &str = "F12";

/// Opens a session and binds the shortcut: the compositor shows its dialog to choose the key.
async fn bind(
    proxy: &GlobalShortcuts,
    on_trigger: &impl Fn(String),
) -> ashpd::Result<Session<GlobalShortcuts>> {
    let session = proxy.create_session(Default::default()).await?;
    let shortcut = NewShortcut::new("toggle", tr("Start or stop dictation"))
        .preferred_trigger(PREFERRED_TRIGGER);
    let bound = proxy
        .bind_shortcuts(&session, &[shortcut], None, Default::default())
        .await?
        .response()?;
    on_trigger(describe(bound.shortcuts()));
    Ok(session)
}

/// Binds the shortcut and forwards each activation to `toggle`. `on_trigger` receives the
/// binding chosen by the compositor; a message on `configure` opens the system dialog.
pub async fn run(
    toggle: Sender<()>,
    mut configure: UnboundedReceiver<()>,
    on_trigger: impl Fn(String),
) -> ashpd::Result<()> {
    let proxy = GlobalShortcuts::new().await?;
    let mut session = bind(&proxy, &on_trigger).await?;

    let mut activated = proxy.receive_activated().await?;
    let mut changed = proxy.receive_shortcuts_changed().await?;
    loop {
        tokio::select! {
            Some(a) = activated.next() => {
                if a.shortcut_id() == "toggle" {
                    let _ = toggle.send(());
                }
            }
            Some(c) = changed.next() => on_trigger(describe(c.shortcuts())),
            Some(()) = configure.recv() => {
                // ConfigureShortcuts appeared in version 2 of the portal; version 1 shows the
                // same dialog as at startup, through a new binding.
                if proxy.version() >= 2 {
                    proxy.configure_shortcuts(&session, None, Default::default()).await?;
                } else {
                    let _ = session.close().await;
                    session = bind(&proxy, &on_trigger).await?;
                }
            }
            else => return Ok(()),
        }
    }
}

fn describe(shortcuts: &[Shortcut]) -> String {
    shortcuts
        .iter()
        .find(|s| s.id() == "toggle")
        .map(|s| s.trigger_description().to_owned())
        .filter(|d| !d.is_empty())
        .unwrap_or_else(|| tr("not assigned"))
}
