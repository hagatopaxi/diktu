//! Global toggle shortcut through the XDG GlobalShortcuts portal.

use std::sync::mpsc::Sender;

use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut, Shortcut};
use futures_util::StreamExt;
use tokio::sync::mpsc::UnboundedReceiver;

/// `<Super><Alt>d` in the XDG shortcuts notation; the compositor has the final say.
const PREFERRED_TRIGGER: &str = "LOGO+ALT+d";

/// Binds the shortcut and forwards each activation to `toggle`. `on_trigger` receives the
/// binding chosen by the compositor; a message on `configure` opens the system dialog.
pub async fn run(
    toggle: Sender<()>,
    mut configure: UnboundedReceiver<()>,
    on_trigger: impl Fn(String),
) -> ashpd::Result<()> {
    let proxy = GlobalShortcuts::new().await?;
    let session = proxy.create_session(Default::default()).await?;
    let shortcut = NewShortcut::new("toggle", "Démarrer ou arrêter la dictée")
        .preferred_trigger(PREFERRED_TRIGGER);
    let bound = proxy
        .bind_shortcuts(&session, &[shortcut], None, Default::default())
        .await?
        .response()?;
    on_trigger(describe(bound.shortcuts()));

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
                // ConfigureShortcuts appeared in version 2 of the portal.
                if proxy.version() >= 2 {
                    proxy.configure_shortcuts(&session, None, Default::default()).await?;
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
        .unwrap_or_else(|| "non attribué".into())
}
