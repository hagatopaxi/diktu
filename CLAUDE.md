# Diktu

Dictée vocale locale en streaming pour GNOME (Rust, GPL-3.0-or-later). Le texte est tapé au curseur via le portail RemoteDesktop.

## Commandes

- Build : `cargo build` (ou `meson setup _build && meson compile -C _build` pour l'installation complète)
- Tests : `cargo test` ; tests réseau/modèle réels : `cargo test -- --ignored`
- Lint : `cargo fmt --all && cargo clippy --all-targets -- -D warnings`
- Schéma, .desktop, metainfo : `meson test -C _build`
- Flatpak : `flatpak run org.flatpak.Builder --user --force-clean build-dir build-aux/fr.gwenael_leger.Diktu.json` ; après tout changement de `Cargo.lock`, régénérer `build-aux/cargo-sources.json` (voir README). Mise à jour de sherpa-onnx et onnxruntime : `docs/MAINTENANCE.md` (la version de la crate `sherpa-onnx` reste égale au tag du manifeste).
- Sans en-têtes GTK système (machine de dev actuelle) : `. <sysroot>/env.sh` avant cargo, cf. docs/DECISIONS.md D2.

## Architecture

- `core/` (`diktu-core`, sans GTK) : `audio` (cpal → ringbuf → rubato 16 kHz), `stt` (trait `SttEngine`, sherpa-onnx), `session` (`Dictation` : machine à états, VAD énergétique, trait `Trigger`), `emit` (delta stable), `inject` (keysyms, trait `TextSink`), `pipeline` (thread d'inférence), `registry` + `download` (modèles HF).
- `app/` (`diktu`, binaire) : `main` (GApplication en fond, threads, canal d'événements vers le thread GTK), `inject` (portail RemoteDesktop), `shortcut` (portail GlobalShortcuts), `tray` (SNI via ksni), `preferences` (AdwPreferencesWindow).
- Threads : audio (callback cpal) → inférence (`pipeline`) → injection (canal borné) ; portails et icône sur un runtime tokio partagé (D17).
- `data/` : schéma GSettings, registre `models.toml`, icônes, sons (GResource), `.desktop`, metainfo.
- `build-aux/` : manifeste Flatpak (onnxruntime de Microsoft, sherpa-onnx compilé depuis les sources, puis Diktu en `-Dshared_sherpa_onnx=true`) et sources cargo hors ligne. `meson.build` appelle cargo puis installe.

## Conventions

- Commits Conventional Commits, un commit par jalon validé (fmt, clippy, test verts).
- Toute décision ambiguë est consignée dans `docs/DECISIONS.md`.
- Ce qui exige une session GNOME réelle va dans `docs/MANUAL_TESTS.md`.
- Pas d'abstraction sans deux implémentations réelles ou un besoin de test.
