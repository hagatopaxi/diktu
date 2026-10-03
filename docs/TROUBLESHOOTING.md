# Troubleshooting

## The shortcut does nothing

- **Laptop function keys.** On many laptops the F row sends media keys by default (volume,
  brightness…). `F12` is then obtained with `Fn+F12`, or by turning on Fn-lock.
- **Check the binding.** GNOME decides the final shortcut. Look it up in GNOME Settings →
  Apps → Diktu, and change it there if needed. The "Change…" button in Diktu's preferences
  opens GNOME's dialog only when the GlobalShortcuts portal is version 2 or later; with
  version 1 (for example GNOME 50.1 with xdg-desktop-portal 1.21.1) Settings is the only way.
- **No GlobalShortcuts portal.** It exists since GNOME 48. On older versions, or if you
  refused the shortcut, bind `diktu --toggle` to a custom GNOME shortcut: see
  [Fallback shortcut](INSTALL.md#fallback-shortcut-without-the-globalshortcuts-portal).

## "Could not register app ID" or "An app id is required"

The portals identify a non-sandboxed application through its installed `.desktop` file. A
binary started straight from `cargo build` or `cargo run` has none, so the global shortcut is
refused (only the `--toggle` fallback works). Install Diktu with Meson so that
`~/.local/share/applications/fr.gwenael_leger.Diktu.desktop` exists: see
[Native build with Meson](INSTALL.md#native-build-with-meson). The Flatpak does not have this
problem.

## No icon in the top bar

GNOME does not show tray icons by itself. Install and enable the **AppIndicator** extension
(`gnome-shell-extension-appindicator`, enabled by default on Ubuntu), then log out and back in.
Without it, Diktu still works: the shortcut starts dictation and running `diktu` again opens
the preferences.

## A red remote-control icon appears in the top bar

This is GNOME's indicator for the RemoteDesktop portal, which is how Diktu types text. Diktu
only keeps the keyboard session open while it types: the icon shows during a dictation and
disappears about 3 s after the last word. It also appears briefly at launch, when the
permission to control the keyboard is checked.

## Apostrophes or special characters are missing

Diktu sends each character as a key symbol. GNOME drops symbols that are not on the active
keyboard layout, so characters such as emoji or rare symbols may be missing from the typed
text. The typographic apostrophe (’) produced by the model is typed as a plain ASCII
apostrophe (') so that it works on every layout. Accented letters are typed correctly,
including with a non-French layout such as US.

## Words are wrong or invented

The default model, Kroko FR, is small (71 MB) and runs on the CPU; it sometimes mishears or
invents words. Already typed words are never corrected. Speaking clearly, close to the
microphone and in a quiet room helps. More accurate models are being evaluated: see
[STT_LANDSCAPE.md](STT_LANDSCAPE.md).

## A model I add is refused

The message says which check failed:

- *Whisper…*, *encoder-decoder…*, *single-file model…*: the architecture cannot stream; pick a
  repository from the sherpa-onnx *online* (streaming) transducers.
- *transcribes silence…* or *poor transcription of the test sentence*: the files do not belong
  together (encoder and joiner of different exports, wrong `tokens.txt`) or the model is not
  for the chosen language.
- *too slow for live dictation*: the model takes more than 80 % of real time on this CPU.
- *the speech engine crashed loading this model*: sherpa-onnx rejected the files (often an
  offline model named like a streaming one); the application itself keeps running.
- *added, but its tests did not finish*: the check took more than 2 minutes and was stopped. The
  model is installed and can be tried; if dictation lags or produces nonsense, remove it.
- *private or gated repository*: Diktu does not log in to Hugging Face; download the files
  yourself and add the folder.

## Listening does not stop by itself

End of speech is detected from the signal energy. Loud, irregular background noise (music,
conversation) can delay the automatic stop up to 6 s after the last recognized word. Press the
shortcut again to stop immediately. The silence that ends a dictation can be set between 0.5
and 5 s in the preferences.

## flatpak-builder fails

- `bwrap: Can't find source path /run/user/1000/doc/...`: run
  `systemctl --user restart xdg-document-portal`, or add `--no-documents-portal` right after
  `flatpak run`.
- `Failure spawning rofiles-fuse`: add `--disable-rofiles-fuse` to the builder options.

Full commands in [INSTALL.md](INSTALL.md#if-flatpak-builder-fails).

## Reading debug logs

Quit Diktu from the tray menu first (a second `diktu` would only open the preferences of the
running instance), then start it from a terminal with debug messages enabled:

```sh
G_MESSAGES_DEBUG=diktu diktu                                          # native
flatpak run --env=G_MESSAGES_DEBUG=diktu fr.gwenael_leger.Diktu       # Flatpak
```

Errors are printed even without `G_MESSAGES_DEBUG`. Debug messages include the dictated text,
so check them before sharing them in a bug report.

Report bugs at <https://github.com/hagatopaxi/diktu/issues>.
