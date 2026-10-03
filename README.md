# Diktu

Diktu is a voice dictation tool for GNOME on Wayland. Press a shortcut, speak, and your words
are typed at the cursor of whatever application you are using, as you talk. Speech
recognition runs entirely on your computer.

Diktu understands English, French, German, Russian and Spanish out of the box; any other
language works with an imported sherpa-onnx streaming model.

## Pronunciation

*Diktu* is said **DEEK-too** (/ˈdik.tu/). It is Esperanto for "dictate!", the imperative of
*dikti*, "to dictate".

## Features

- **Types where you are.** Text goes to the focused application: editor, browser, terminal,
  chat. It is typed through the XDG RemoteDesktop portal, without touching the clipboard and
  without `uinput` or `xdotool`.
- **Streaming.** Words appear while you speak, once the model is sure of them: never half a
  word, never a backspace.
- **Stops by itself.** Listening ends after a short silence (1.2 s by default). The first
  letter is capitalized and a final period is added (this can be turned off).
- **Stays out of the way.** No window, just the Diktu icon in the top bar; GNOME's microphone
  indicator shows when you are being listened to. A short sound tells you when the microphone is open.
- **Local.** Recognition runs on the CPU with [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx).
  No account, no online service.
- **Multilingual.** Dictation in English, French, German, Russian and Spanish; the interface
  is also translated into Esperanto. The dictation language follows your desktop's by
  default.

## Privacy

No audio ever leaves your machine. The microphone is open only while you dictate. The only
network access is the one-time download of the speech model from Hugging Face, which you start
yourself from the preferences.

## Requirements

- GNOME on Wayland, version 48 or later for the global shortcut (older versions can use a
  [fallback shortcut](docs/INSTALL.md#fallback-shortcut-without-the-globalshortcuts-portal)).
- The [AppIndicator](https://extensions.gnome.org/extension/615/appindicator-support/)
  extension to see the icon. It is enabled by default on Ubuntu; on Fedora, install
  `gnome-shell-extension-appindicator`. Diktu works without it, minus the icon.
- About 71 MB of disk space for the default model.

## Install

The easiest way is the Flatpak bundle from
[GitHub Releases](https://github.com/hagatopaxi/diktu/releases). No release has been published
yet (v0.1.0 is planned); until then, build it from source as described in
[docs/INSTALL.md](docs/INSTALL.md).

Once a release is out, download `diktu-x86_64.flatpak` (or `diktu-aarch64.flatpak`) and run:

```sh
flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user diktu-x86_64.flatpak
flatpak run fr.gwenael_leger.Diktu
```

Flathub provides the GNOME 50 runtime Diktu needs. Diktu itself is not on Flathub yet.

[docs/INSTALL.md](docs/INSTALL.md) also covers building the Flatpak from source, the native
build with Meson, starting at login, and uninstalling.

## First launch

An assistant walks you through the setup, one step at a time:

1. **Language.** Pick the language you will speak. Its model (about 70 MB) downloads while
   you go through the next steps; the download is checked with SHA-256.
2. **Permissions.** Diktu explains what it needs, then GNOME asks you to confirm, one dialog
   after the other: running in the background, and controlling the keyboard (this is how
   Diktu types). The answers are remembered.
3. **Shortcut.** GNOME proposes `F12`. Accept it or choose another key. On many laptops the
   F row sends media keys by default; press `Fn+F12` then, or turn on Fn-lock.

## Everyday use

1. Put the cursor where you want the text.
2. Press the shortcut. After the short sound, speak.
3. Stop talking. After 1.2 s of silence, Diktu types the rest and stops listening. Press the
   shortcut again to stop immediately.

Listening also stops after 6 s without any recognized word, and after 5 minutes at most.

While text is being typed, GNOME shows its red remote-control icon in the top bar. It goes
away about 3 s after the last word.

Open the preferences from the icon menu ("Preferences…") or by running `diktu` again. There
you can choose the language and model, download or remove models, change the shortcut, and
add your own model, and set the end-of-speech silence (0.5 to 5 s), automatic capitalization and final period, the
delay between typed keys, and the sounds and their volume.

### Your own models

"Add a model" in the preferences imports a model from a Hugging Face repository (`owner/name`)
or from a folder, for the language you choose. Only sherpa-onnx **streaming transducers** work
(files `encoder*.onnx`, `decoder*.onnx`, `joiner*.onnx`, `tokens.txt`, as in the
[sherpa-onnx streaming models](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-transducer/index.html));
Whisper, CTC, Paraformer and other offline models are refused. The model is copied under
`~/.local/share/diktu/models/`, then tested before use: file integrity, loading, silence,
decoding speed, and for French the transcription of a test sentence. Imported models can be
removed like the others.

## Known limitations

- Only French has a ready-made model; other languages need a model you import, whose accuracy
  is not tested (no test sentence yet).
- The model decodes speech in 1.28 s chunks, so words arrive in bursts, 1 to 2 s behind
  your voice.
- Accuracy is limited: the model sometimes invents words. Words already typed are never
  corrected. Better models are being evaluated in [docs/STT_LANDSCAPE.md](docs/STT_LANDSCAPE.md).
- No voice commands for punctuation; the punctuation the model produces is kept.
- Characters missing from your keyboard layout (emoji, rare symbols) may be dropped.
- Loud, irregular background noise can delay the automatic stop by a few seconds.
- X11 is not supported.

If something does not work, see [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md).

## Documentation

- [docs/INSTALL.md](docs/INSTALL.md): all installation methods, fallback shortcut, uninstalling.
- [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md): common problems and debug logs.
- [docs/DECISIONS.md](docs/DECISIONS.md): technical choices and their reasons.
- [docs/MANUAL_TESTS.md](docs/MANUAL_TESTS.md): manual test checklist for a real GNOME session.
- [docs/MAINTENANCE.md](docs/MAINTENANCE.md): updating sherpa-onnx and onnxruntime.
- [docs/STT_LANDSCAPE.md](docs/STT_LANDSCAPE.md): survey of local speech recognition models.

## Contributing

Bug reports and pull requests are welcome on
[GitHub](https://github.com/hagatopaxi/diktu/issues). [CLAUDE.md](CLAUDE.md) gives the build,
test and lint commands and an overview of the architecture.

```sh
cargo test                                # unit and integration tests
cargo test -p diktu-core -- --ignored     # downloads the model, then tests real dictation
cargo fmt --all && cargo clippy --all-targets -- -D warnings
```

Commits follow [Conventional Commits](https://www.conventionalcommits.org). Record non-obvious
choices in [docs/DECISIONS.md](docs/DECISIONS.md). After changing `Cargo.lock`, regenerate the
Flatpak sources with
[flatpak-cargo-generator](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo):

```sh
uv run flatpak-cargo-generator.py Cargo.lock -o build-aux/cargo-sources.json
```

Translations live in `po/` (gettext). After changing a user-visible message, run
`po/update.sh` to refresh `po/diktu.pot` and the catalogs, then translate the new entries.
To add a language, add its code to `po/LINGUAS` and run the script. Messages built from
values use named placeholders (`{name}`) that translations must keep.

## License and credits

Diktu is written by Gwenaël Léger and released under the
[GNU General Public License v3.0 or later](LICENSE).

It builds on:

- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) by k2-fsa (Apache-2.0), for speech
  recognition;
- [ONNX Runtime](https://github.com/microsoft/onnxruntime) by Microsoft (MIT);
- the Kroko FR, EN, DE and ES models by [Banafo](https://huggingface.co/Banafo/Kroko-ASR)
  (CC-BY-SA) and the Vosk small RU model by
  [Alpha Cephei](https://huggingface.co/alphacep/vosk-model-small-streaming-ru) (Apache-2.0),
  converted by [csukuangfj](https://huggingface.co/csukuangfj), downloaded separately;
- reference clips from [Common Voice](https://commonvoice.mozilla.org) (CC0) in `data/samples/`;
- [gtk-rs](https://gtk-rs.org) and libadwaita, [ashpd](https://github.com/bilelmoussaoui/ashpd),
  [ksni](https://github.com/iovxw/ksni), [rodio](https://github.com/RustAudio/rodio) and
  [cpal](https://github.com/RustAudio/cpal).
