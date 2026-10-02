//! StatusNotifierItem icon (shown by the AppIndicator extension in GNOME).

use std::sync::mpsc::Sender;

use gtk::gdk_pixbuf::Pixbuf;
use ksni::menu::{MenuItem, StandardItem};
use tokio::sync::mpsc::UnboundedSender;

use crate::{RESOURCE_PREFIX, UiEvent};

pub struct Tray {
    pub recording: bool,
    toggle: Sender<()>,
    events: UnboundedSender<UiEvent>,
    idle: Vec<ksni::Icon>,
    active: Vec<ksni::Icon>,
}

impl Tray {
    pub fn new(toggle: Sender<()>, events: UnboundedSender<UiEvent>) -> Self {
        Self {
            recording: false,
            toggle,
            events,
            idle: icon("status-idle"),
            active: icon("status-recording"),
        }
    }
}

/// Renders an embedded SVG to the ARGB32 pixmaps of the SNI protocol, so that the icon
/// keeps its colour whatever the icon theme, in a source tree as in Flatpak.
fn icon(name: &str) -> Vec<ksni::Icon> {
    [22, 44]
        .into_iter()
        .filter_map(|size| {
            let path = format!("{RESOURCE_PREFIX}/icons/{name}.svg");
            let pixbuf = Pixbuf::from_resource_at_scale(&path, size, size, true).ok()?;
            let pixbuf = pixbuf.add_alpha(false, 0, 0, 0).ok()?;
            let (w, h, stride) = (pixbuf.width(), pixbuf.height(), pixbuf.rowstride() as usize);
            let bytes = pixbuf.read_pixel_bytes();
            let mut data = Vec::with_capacity((w * h * 4) as usize);
            for row in bytes.chunks(stride).take(h as usize) {
                for &[r, g, b, a] in row[..w as usize * 4].as_chunks::<4>().0 {
                    data.extend_from_slice(&[a, r, g, b]);
                }
            }
            Some(ksni::Icon {
                width: w,
                height: h,
                data,
            })
        })
        .collect()
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "diktu".into()
    }

    fn title(&self) -> String {
        "Diktu".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        if self.recording {
            self.active.clone()
        } else {
            self.idle.clone()
        }
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: if self.recording {
                "Diktu : écoute…"
            } else {
                "Diktu"
            }
            .into(),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.toggle.send(());
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let item = |label: &str, activate: fn(&mut Self)| {
            StandardItem {
                label: label.into(),
                activate: Box::new(activate),
                ..Default::default()
            }
            .into()
        };
        vec![
            item(
                if self.recording {
                    "Arrêter la dictée"
                } else {
                    "Démarrer la dictée"
                },
                |t| {
                    let _ = t.toggle.send(());
                },
            ),
            item("Réglages…", |t| {
                let _ = t.events.send(UiEvent::Preferences);
            }),
            MenuItem::Separator,
            item("Quitter", |t| {
                let _ = t.events.send(UiEvent::Quit);
            }),
        ]
    }
}
