//! Global toggle shortcut through the XDG GlobalShortcuts portal.

use diktu_core::i18n::tr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
/// binding chosen by the compositor; a message on `configure` opens the system dialog, and
/// `can_configure` tells whether the portal can do it (version 2 or later).
pub async fn run(
    toggle: Sender<()>,
    can_configure: Arc<AtomicBool>,
    mut configure: UnboundedReceiver<()>,
    on_trigger: impl Fn(String),
) -> ashpd::Result<()> {
    let proxy = GlobalShortcuts::new().await?;
    let session = bind(&proxy, &on_trigger).await?;
    can_configure.store(proxy.version() >= 2, Ordering::Relaxed);

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
                if can_configure.load(Ordering::Relaxed) {
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
        .unwrap_or_else(|| tr("not assigned"))
}

/// The portal describes the trigger as a localized phrase around a GTK accelerator
/// ("Press <Control><Alt>h"): keep only the key label, or the escaped phrase for
/// the markup labels it ends up in. GTK only: call it on the main thread.
pub fn label(description: &str) -> String {
    let accelerator = description
        .find('<')
        .map_or(description, |i| &description[i..]);
    match gtk::accelerator_parse(accelerator) {
        Some((key, mods)) => gtk::accelerator_get_label(key, mods).to_string(),
        None => gtk::glib::markup_escape_text(description).to_string(),
    }
}
