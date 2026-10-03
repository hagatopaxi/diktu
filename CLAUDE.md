# Diktu

Local streaming voice dictation for GNOME (Rust, GPL-3.0-or-later). Text is typed at the cursor through the RemoteDesktop portal.

## Commands

- Build: `cargo build` (or `meson setup _build && meson compile -C _build` for the full installation)
- Tests: `cargo test`; real network/model tests: `cargo test -- --ignored`
- Lint: `cargo fmt --all && cargo clippy --all-targets -- -D warnings`
- Schema, .desktop, metainfo: `meson test -C _build`
- Flatpak: `flatpak run org.flatpak.Builder --user --force-clean build-dir build-aux/fr.gwenael_leger.Diktu.json`; development build: `build-aux/fr.gwenael_leger.Diktu.Devel.json` (regenerate it after editing the release manifest, D26); after any change to `Cargo.lock`, regenerate `build-aux/cargo-sources.json` (see README). Updating sherpa-onnx and onnxruntime: `docs/MAINTENANCE.md` (the `sherpa-onnx` crate version stays equal to the manifest's tag).
- Without system GTK headers (current dev machine): `. <sysroot>/env.sh` before cargo, see docs/DECISIONS.md D2.

## Architecture

- `core/` (`diktu-core`, no GTK): `audio` (cpal → ringbuf → rubato 16 kHz), `stt` (`SttEngine` trait, sherpa-onnx), `session` (`Dictation`: state machine, energy VAD, `Trigger` trait), `emit` (stable delta), `inject` (keysyms, `TextSink` trait), `pipeline` (inference thread), `registry` + `download` (HF models).
- `app/` (`diktu`, binary): `main` (background GApplication, threads, event channel to the GTK thread), `inject` (RemoteDesktop portal), `shortcut` (GlobalShortcuts portal), `tray` (SNI via ksni), `preferences` (AdwPreferencesWindow).
- Threads: audio (cpal callback) → inference (`pipeline`) → injection (bounded channel); portals and icon on a shared tokio runtime (D17).
- `data/`: GSettings schema, `models.toml` registry, icons, sounds (GResource), `.desktop`, metainfo.
- `build-aux/`: Flatpak manifest (Microsoft's onnxruntime, sherpa-onnx built from source, then Diktu with `-Dshared_sherpa_onnx=true`) and offline cargo sources. `meson.build` calls cargo then installs.

## Documentation

All documentation is in English.

- `README.md`: user-facing overview, quick install, everyday use.
- `docs/INSTALL.md`: every installation method, autostart, fallback shortcut, uninstalling.
- `docs/TROUBLESHOOTING.md`: common problems, debug logs.
- `docs/DECISIONS.md`: technical decisions (D1, D2…).
- `docs/MANUAL_TESTS.md`: checklist for a real GNOME session.
- `docs/MAINTENANCE.md`: updating sherpa-onnx and onnxruntime.
- `docs/STT_LANDSCAPE.md`: survey of local speech recognition models.

## Conventions

- Conventional Commits, one commit per validated milestone (fmt, clippy, tests green).
- Documentation is written in English (American spelling).
- Every ambiguous decision is recorded in `docs/DECISIONS.md`.
- Anything that requires a real GNOME session goes in `docs/MANUAL_TESTS.md`.
- User-visible changes to installation or behavior update `README.md`, `docs/INSTALL.md` or `docs/TROUBLESHOOTING.md`.
- No abstraction without two real implementations or a testing need.
