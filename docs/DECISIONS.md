# Decisions

Each entry: context, choice, rejected alternative, reason.

## D1 — Application ID

- **Context**: a reverse-DNS ID is needed (GApplication, GSettings, Flatpak, AppStream).
- **Choice**: `fr.gwenael_leger.Diktu` (the author's domain `gwenael-leger.fr`, with the hyphen replaced by an underscore as Flatpak recommends).
- **Rejected**: `io.github.<account>.Diktu`, as no verifiable GitHub account existed at the time of the choice.
- **Reason**: a stable ID, tied to a domain the author owns.

## D2 — GTK development headers missing on the build machine

- **Context**: the development machine has neither `libgtk-4-dev`, `libadwaita-1-dev` nor `libasound2-dev`, and no `sudo` access.
- **Choice**: `-dev` packages downloaded with `apt-get download` and extracted into a local sysroot, pointed to by `PKG_CONFIG_SYSROOT_DIR`/`PKG_CONFIG_LIBDIR`. Purely local, none of it is in the repository; CI and the README use system packages.
- **Rejected**: building only inside the Flatpak SDK (slow, ~1 GB to download before the first `cargo test`).
- **Reason**: keep `cargo test` native and fast.

## D3 — Minimum GTK/libadwaita versions

- **Context**: CI runs on Ubuntu 24.04 (GTK 4.14, libadwaita 1.5).
- **Choice**: features `gtk4/v4_14` and `libadwaita/v1_5`. `AdwPreferencesWindow` (required) therefore emits no deprecation warning (it is deprecated from libadwaita 1.6 onward in favor of `AdwPreferencesDialog`).
- **Rejected**: targeting libadwaita 1.6+ and `AdwPreferencesDialog`.
- **Reason**: a constraint of the specification, and compatibility with Ubuntu 24.04 LTS.

## D4 — sherpa-onnx bindings

- **Context**: a streaming transducer recognizer usable from Rust is needed.
- **Choice**: the official `sherpa-onnx` 1.13.8 crate (k2-fsa, Apache-2.0), statically linked. Its `build.rs` downloads the prebuilt archive `sherpa-onnx-v1.13.8-linux-x64-static-lib.tar.bz2` from the GitHub releases, or takes it from `SHERPA_ONNX_ARCHIVE_DIR` for an offline build. The Flatpak dynamically links a sherpa-onnx built from source (D24).
- **Rejected**: a home-made FFI wrapper over the C API; the third-party `sherpa-rs` crate.
- **Reason**: the official bindings expose everything needed (streams, endpointing, `input_finished`); a home-made wrapper would only add maintenance.

## D5 — Default French model

- **Context**: two non-gated French streaming zipformer transducers exist on HF. Measured on 3 Common Voice clips (CC0), 100 ms chunks, 2 threads:
  | Model | Size | WER | RTF | Output |
  |---|---|---|---|---|
  | `csukuangfj/sherpa-onnx-streaming-zipformer-fr-kroko-2025-08-06` | 71 MB | 19.4% | 0.048 | casing and punctuation |
  | `shaojieli/sherpa-onnx-streaming-zipformer-fr-2023-04-14` (int8) | 128 MB | 13.9% | 0.089 | UPPERCASE, no punctuation |
- **Choice**: Kroko FR. CC-BY-SA license (Banafo's "community" models; the license version is not specified by the publisher; the HF conversion repository points to `Banafo/Kroko-ASR`). Recorded in the registry.
- **Rejected**: the shaojieli model (Apache-2.0), with a better WER on these 3 clips but trained on Common Voice, hence in-domain; all uppercase and without punctuation, unusable as is for dictating messages.
- **Reason**: for dictation, native casing and punctuation are worth more than a few WER points on 36 words; the model is half the size and twice as fast.

## D6 — Transcription test threshold

- **Choice**: aggregate WER ≤ 25% on the 3 Common Voice clips in `core/tests/data`, measured on the text actually typed by `Dictation` (engine + emission + end of speech): 22.2% (19.4% for raw transcription). Normalization: lowercase, apostrophes and hyphens → spaces, punctuation removed.
- **Reason**: enough margin to absorb a change of sherpa-onnx version, low enough to detect a badly loaded model or a badly chunked stream.

## D7 — Registry: `name` and `size` fields

- **Context**: the required schema (`id`, `langs`, `engine`, `license`, `repo`, `revision`, `files[{path, sha256}]`) gives neither a label for the UI nor a total size for multi-file progress.
- **Choice**: added `name` (displayed label) and `files[].size` (bytes, read from the HF API). The role of each file (encoder, decoder, joiner, tokens) is inferred from its name prefix, without a dedicated field.
- **Rejected**: per-file progress from `Content-Length` (a bar that resets to zero for each file).
- **Reason**: a single, consistent progress bar.

## D8 — Blocking HTTP client in a thread

- **Choice**: `reqwest::blocking` (rustls TLS by default), run in a dedicated thread; cancellation through an `AtomicBool` checked at every 64 KiB block, progress through a callback.
- **Rejected**: an async client integrated into the GLib main loop.
- **Reason**: linear code, testable without an async runtime; the UI receives progress through a channel.

## D9 — End-of-speech detection: energy VAD, not sherpa-onnx endpointing

- **Context**: sherpa-onnx's built-in endpointing (rule 2 = 1.2 s of silence after speech) was tried and measured unusable with the default model: (1) the model emits the final `.` ~1.5 s after the end of speech, which resets the silence counter (end detected after 2.8 s); (2) the encoder decodes in blocks of 128 frames (1.28 s), so any token-based measurement is quantized to 1.28 s, above the 1.2 s threshold.
- **Choice**: `Dictation` (core) measures silence on the signal: an energy detector on 30 ms frames, an adaptive noise floor (follows drops immediately, rises in ~15 s, initialized to at most −40 dBFS), speech = 12 dB above the floor and above −50 dBFS. Listening ends if: ≥ 200 ms of speech followed by `end-silence` (1.2 s by default, configurable) of silence; or no new word from the model for 6 s (nothing said, or continuous noise); or 5 min of dictation (safety net).
- **Rejected**: sherpa endpointing; Silero VAD (a second model to download and manage in the registry).
- **Reason**: 30 ms precision, no extra model, testable with simulated audio. The "no new word" fallback covers non-stationary noise that energy cannot tell apart from voice.

## D10 — Microphone open only while listening, no continuous pre-roll

- **Context**: a 200 ms pre-roll implies a permanently open microphone to keep the last 200 ms before the shortcut.
- **Choice**: the `cpal` stream is opened when triggered and closed at the end of the dictation. The start sound is played once the stream is actually open: everything the user says after the beep is captured. The ring buffer (2 s) serves as a queue between the audio callback and the inference thread.
- **Rejected**: a continuously open microphone with a 200 ms pre-roll.
- **Reason**: GNOME shows the microphone indicator as long as a stream is open; a microphone permanently open by a background application is unacceptable for privacy and battery life. The "microphone ready" beep makes the pre-roll unnecessary.

## D11 — One dictation = one segment

- **Choice**: end of speech ends both the segment (model flushed with `input_finished`, remaining text typed followed by a space, final period) and listening. A second press of the shortcut does the same.
- **Reason**: the expected behavior stops listening at the first silence; splitting into sub-segments would have no observable effect.

## D12 — RemoteDesktop session open only while typing

- **Context**: the portal asks for consent when a session is created (unless a valid restore token exists), and GNOME shows a red remote-control icon as long as a session is open, which is worrying (observed on GNOME 50.1).
- **Choice**: the keyboard session is opened at launch to obtain consent (the dialog appears then rather than in the middle of the first dictation), then closed after 3 s with no text to type; it is reopened on the next text thanks to the `restore_token` (GSettings `restore-token`, `PersistMode::ExplicitlyRevoked`), without a dialog. The red icon therefore only appears during a dictation. If sending fails, the session is reopened once.
- **Cost**: the first word of each dictation waits for the session to be created.

## D13 — Fallback command `diktu --toggle`

- **Choice**: a local command-line option that registers the application, activates the `app.toggle` action (forwarded over D-Bus to the primary instance), then exits. This is the command to bind to a GNOME custom shortcut if the GlobalShortcuts portal is missing.
- **Rejected**: `gapplication action …`, which requires a D-Bus-activatable application (an extra service file).
- **Reason**: a single command, identical natively and in Flatpak (`flatpak run fr.gwenael_leger.Diktu --toggle`).

## D14 — cpal 0.17 instead of 0.18

- **Context**: rodio 0.22.2 (latest version) depends on cpal 0.17; two versions of cpal cannot coexist (same native library `alsa-sys`).
- **Choice**: `cpal = "0.17.3"` in `core`, shared with rodio.
- **Rejected**: cpal 0.18 with a home-made sound player on top of cpal.
- **Reason**: the API used is identical except for one reference; a single audio stack.

## D15 — Sounds: synthesized WAV, GResource, ephemeral output stream

- **Choice**: `tools/gen-sounds.py` synthesizes two 160 ms sounds (two rising or falling sine notes, 8 ms fades), versioned in `data/sounds/`, embedded as a GResource, played by rodio on an output stream opened for the duration of the sound. The start sound is played when the microphone is actually open. An original work generated by code, released under CC0.
- **Rejected**: libcanberra (an extra C dependency, system sound theme).
- **Reason**: no third-party files, nothing held on the output device between two dictations.

## D16 — SNI icon as a pixmap rendered from the embedded SVG

- **Choice**: a single light grey SVG (a D holding a microphone, the logo) is embedded as a GResource and rendered by gdk-pixbuf into ARGB32 pixmaps (22 and 44 px) sent through `IconPixmap`. In Flatpak, the icon is published without its own D-Bus name (`disable_dbus_name`), as the sandbox requires.
- **Rejected**: `IconName` + `IconThemePath`, which assumes icons installed and visible to the host shell (false when running from the sources, and names constrained by the Flatpak export).
- **Reason**: the same rendering in development, native and Flatpak; the color does not depend on the theme.
- **Update**: the icon no longer turns red while listening; GNOME's own microphone indicator already shows it, and the menu entry reads "Stop dictation".

## D17 — A single tokio runtime for the portals and the icon

- **Context**: ashpd and ksni share a zbus connection whose background tasks run on the runtime that created it; any zbus object dropped outside a tokio context makes the program panic (observed during the first smoke test).
- **Choice**: a global multi-thread runtime (1 worker); the main and injection threads enter it for their whole lifetime (`Runtime::enter`).
- **Reason**: no more deadlocks or panics, however a thread waits.

## D18 — Preferred shortcut format and host application registration

- **Choice**: preferred trigger `F12` (notation of the XDG "shortcuts" specification; a single free key, chosen by the user instead of `Super+Alt+D`). At startup, the application registers with `org.freedesktop.host.portal.Registry` (no effect in Flatpak).
- **Finding**: without this registration, the GlobalShortcuts portal rejects a non-sandboxed application ("An app id is required"); the registration requires `fr.gwenael_leger.Diktu.desktop` to be installed. Running the binary from the source tree without `meson install` therefore only gives the `--toggle` fallback.

## D19 — Single preferences window, hidden on close

- **Choice**: the `AdwPreferencesWindow` is built once and hidden on close; each model has a row whose visibility follows the selected language. Downloads continue while the window is closed.
- **Rejected**: rebuilding the window and the list at every opening or language change (in-progress downloads would then have to be found again).
- **Reason**: a single state, no synchronization to write.
- The model selector (radio button) only appears if a language has several models.

## D20 — Flatpak: GNOME 50 runtime, offline build

- **Choice**: `org.gnome.Platform//50` (51 exists but 50 is the most widely deployed at the time of the choice and already present on the dev machine), extension `rust-stable//25.08` (the freedesktop SDK version of GNOME 50). Crates vendored by `flatpak-cargo-generator` (flatpak-builder-tools pinned at commit `74697c75`) into `build-aux/cargo-sources.json`; sherpa-onnx v1.13.8 static archive (x86_64 and aarch64) as a `file` source with a SHA-256 identical to the digest published by GitHub, provided to the build through `SHERPA_ONNX_ARCHIVE_DIR` (replaced by a build from source, D24).
- **Permissions**: those requested, plus `--talk-name=org.kde.StatusNotifierWatcher` for the SNI icon. The portals (Background, RemoteDesktop, GlobalShortcuts, notifications) are always reachable from the sandbox without extra permissions; the application calls the Background portal at startup.
- **Rejected**: `cargo vendor` committed to the repository (hundreds of MB of sources).
- **Verified**: `flatpak-builder` (org.flatpak.Builder 1.4.9) builds and exports the package offline; the binary starts in the runtime with no missing library. On the dev machine, `flatpak run --no-documents-portal … --disable-rofiles-fuse` was needed (no FUSE in the agent's sandbox); these options are not needed on a normal workstation.

## D21 — AppStream homepage

- **Choice**: the public repository `https://github.com/hagatopaxi/diktu` (homepage, issue tracker, sources).

## D22 — Minimum Rust version: 1.92

- **Finding**: the initial value (1.88) was wrong; checked with the 1.88 toolchain, compilation fails (cairo-rs/gtk-rs 0.22 require 1.92). `cargo +1.92 check --all-targets` passes.
- **Choice**: `rust-version = "1.92"`; the README asks for rustup, as the `cargo` packages of Ubuntu 24.04 and Fedora are too old or borderline.

## D23 — Name: Diktu

- **Context**: "Parlotte" is also spelled "parlote" and only reads well in French; the name must be pronounced and spelled unambiguously in every language.
- **Choice**: "Diktu" ("dictate!" in Esperanto, a phonetic language). Checked as available on 2026-10-02: no GitHub repository with that name, absent from crates.io, PyPI and Flathub (trademark registries not consulted). ID `fr.gwenael_leger.Diktu`, crates `diktu` and `diktu-core`.

## D24 — sherpa-onnx built from source in the Flatpak

- **Context**: Flathub requires any software whose sources are available to be built from those sources, without network access during the build; binaries from major vendors are accepted case by case when the tooling does not allow an offline build (precedent: `net.mkiol.SpeechNote` bundles Microsoft's prebuilt onnxruntime). The k2-fsa static archive from D20 does not meet this requirement.
- **Choice**: the manifest builds three modules.
  - `onnxruntime`: Microsoft's official 1.28.2 release (shared library, the version sherpa-onnx v1.13.8 expects), installed in `/app/lib`; its headers are removed at the end of the build. Reason: onnxruntime relies on dozens of `FetchContent` dependencies and submodules (abseil, protobuf, flatbuffers, onnx…), Microsoft is a major vendor, and the SpeechNote precedent exists.
  - `sherpa-onnx`: tag v1.13.8 pinned at commit `11afbd009a7f8c08f4bcf2fc1b265d0df4670fbf`, CMake with `BUILD_SHARED_LIBS=ON` reduced to the C API (TTS, diarization, websocket, PortAudio, Python, tests and executables disabled: Diktu only uses streaming recognition). It finds onnxruntime through `SHERPA_ONNXRUNTIME_LIB_DIR`/`SHERPA_ONNXRUNTIME_INCLUDE_DIR`. Each archive of its `FetchContent` dependencies (kaldi-native-fbank, kissfft, kaldi-decoder, kaldifst, openfst, eigen, simple-sentencepiece, nlohmann/json) is a `file` source placed in the sources directory under the name its CMake files look for, which also check the SHA-256.
  - `diktu`: Meson option `shared_sherpa_onnx` → Cargo feature `diktu-core/shared-sherpa` (`sherpa-onnx/shared`) with `SHERPA_ONNX_LIB_DIR=/app/lib`; the binary links `libsherpa-onnx-c-api.so` and `libonnxruntime.so`, found at runtime in `/app/lib`.
- The native build keeps the prebuilt static archive: the `sherpa-onnx` crate is declared without its default features, and `sherpa-onnx-sys` links statically as long as `shared` is not requested.
- The version of the `sherpa-onnx` crate stays equal to the built tag: the FFI structures of `sherpa-onnx-sys` follow the C API of the same version. `x-checker-data` blocks on the sherpa-onnx git source and on the onnxruntime archives make the Flathub bot report new versions; the update procedure is in [MAINTENANCE.md](MAINTENANCE.md).

## D25 — User-imported models

- **Context**: users want other models and languages than the registry's. sherpa-onnx has many architectures, but the pipeline needs a streaming (online) recognizer, and only the transducer is wired (`SherpaTransducer`).
- **Choice**: import only sherpa-onnx streaming transducers, from a Hugging Face repository or a local folder. Files are picked by name (`encoder`/`decoder`/`joiner` `.onnx` and `tokens`; full precision before `int8`, then the shortest name); other architectures are recognized by their files and refused with an explanation. A local import is copied, so the model keeps working if the source moves.
- The import goes to `<models>/.staging-custom-<slug>`, is checked, then renamed to `custom-<slug>` with a `model.toml` manifest (a registry `Model`) and the `.installed` marker. A Hugging Face import is pinned to the repository's current commit; LFS files are verified against their SHA-256, small non-LFS files (`tokens.txt`) cannot be (Hugging Face only gives their SHA-1). A local import's revision is the encoder's SHA-256.
- The check runs `diktu --check-model <dir>` in a child process, because sherpa-onnx may abort the process on malformed files. It verifies sizes and hashes against the manifest, the ONNX header byte, the `tokens.txt` format (ids 0..n without gaps), loading, that 2 s of silence give at most two words, a real-time factor ≤ 0.8, and, when the language has a reference clip (`data/samples/<lang>.wav` + `.txt`, CC0 Common Voice), a word error rate ≤ 50 % on it. Other languages skip the accuracy check, and the summary says so; adding a clip enables it. A check still running after 120 s is killed and the model is installed anyway, with a persistent warning: a slow machine should not block the user, who stays informed.
- The language is chosen at import (one per model). The Language list in the preferences is the union of the registry's and imported models' languages.

## D26 — Development build next to the release

- **Context**: testing a development version must not overwrite or reconfigure the installed release.
- **Choice**: a Meson option `profile=development` (Cargo feature `diktu/devel`) builds a separate application: ID `fr.gwenael_leger.Diktu.Devel` (GNOME's `.Devel` convention), name "Diktu dev", binary `diktu-dev`, its own GSettings schema (same keys under the new ID and path, rewritten by `sed` in Meson and by `build.rs`), its own data folder `diktu-dev`, and an orange striped icon. Models are not shared: a development build may import or remove models freely.
- `build-aux/fr.gwenael_leger.Diktu.Devel.json` is generated from the release manifest, to be rerun after editing it:
  `jq --indent 2 '.id += ".Devel" | .command = "diktu-dev" | (.modules[] | select(.name == "diktu") | ."config-opts") += ["-Dprofile=development"]' build-aux/fr.gwenael_leger.Diktu.json > build-aux/fr.gwenael_leger.Diktu.Devel.json`

## D27 — Built-in languages

- **Context**: Diktu offered French only. The five most spoken languages in Europe are Russian, German, French, English and Italian (native speakers).
- **Choice**: one small streaming transducer per language, like Kroko FR: Kroko EN, DE, ES (71, 71 and 156 MB, same family and license as Kroko FR) and Vosk small RU (94 MB, Apache-2.0), all from `csukuangfj`'s sherpa-onnx conversions. Italian is replaced by Spanish: the only sherpa-onnx Italian streaming transducers on Hugging Face are re-uploads of Kroko IT by unknown accounts, with no provenance; Banafo publishes Kroko IT only in its own `.data` format. Italian stays available through the import.
- Each language has a CC0 Common Voice 17 test clip in `data/samples/` (16 kHz mono, re-encoded from the `fixie-ai/common_voice_17_0` test split): `common_voice_en_27340672`, `common_voice_de_38272982`, `common_voice_es_19653917`, `common_voice_ru_32271856`. The registry models score 0–33 % word errors on them, below the 50 % import threshold.
- The `language` setting defaults to empty: the desktop language when a model exists for it, otherwise English.

## D28 — Translations

- **Context**: the interface was English only; Diktu now dictates in five languages and is named in Esperanto.
- **Choice**: GNU gettext, domain `diktu`, catalogs in `po/` for French, German, Spanish, Russian and Esperanto. `diktu_core::i18n` calls the C library's `setlocale`, `bindtextdomain` and `dgettext` directly: three functions do not justify a crate. Catalogs are looked up in `<prefix>/share/locale`, the prefix being the executable's parent folder, which covers Meson installs and Flatpak (`/app`); `cargo run` is untranslated. The model check child process also initializes gettext, so its summary is translated.
- Messages built from values use named placeholders replaced after translation (`trf("{repo}: no such repository", &[("repo", &repo)])`), so translations can reorder them.
- gettext 0.23 has no Rust parser: `po/update.sh` extracts the `tr`/`trf` keywords with the C parser, then merges the desktop file (GenericName, Comment, Keywords) and metainfo messages. A translatable literal therefore stays on one line: a Rust `\` line continuation would keep the indentation in C. Meson compiles the catalogs and merges translations into the desktop file and metainfo; the `[fr]` entries written by hand there are gone.
- The `.po` files were translated by Claude and are open to review by native speakers.
- `po/check.py` (a Meson test, so in CI) fails when a catalog misses a message used in the Rust sources, leaves one untranslated or fuzzy, or changes a `{placeholder}`. It cannot judge whether a translation is right.
- The interface follows the session language, like any GNOME app: GNOME apps do not offer an interface-language picker of their own, so Diktu has none (a short-lived `interface-language` setting was removed). `LANGUAGE=xx` still works through gettext, e.g. `LANGUAGE=eo flatpak run fr.gwenael_leger.Diktu`. The dictation language is a separate setting. The Flatpak keeps every catalog in the app (`separate-locales: false`, a few kB): by default Flatpak installs only the system languages' translations, which would leave a `LANGUAGE=xx` test untranslated.

## D29 — First-launch assistant

- **Context**: at first launch, the preferences window, the Background, RemoteDesktop and GlobalShortcuts dialogs all opened at once, each asking for something different.
- **Choice**: an `AdwNavigationView` assistant, shown until it is completed (GSettings `onboarded`): language and model (the download starts and continues during the next steps), permissions (an explanation, then one button that opens the Background dialog, then the RemoteDesktop one once the first is answered), shortcut (GNOME's dialog, opened by a button), then how to dictate. The injection thread waits for a consent request before connecting, and the shortcut task waits for the shortcut step; closing the assistant early releases both, so the dialogs appear later (first dictation, right away for the shortcut). Users who already have a `restore-token` count as onboarded.
- **Reason**: the system dialogs belong to GNOME and cannot be merged; showing them one at a time, each after its explanation, is what the application controls.
- The assistant is translated like the rest (D28) in the session language. Its language step picks the dictation language only and does not retranslate the assistant live: its pages hold the consent and shortcut channels, which a rebuild would lose.

## D30 — Changing the shortcut on portal v1 points to GNOME Settings

- **Context**: version 1 of the GlobalShortcuts portal (GNOME 50) has no ConfigureShortcuts, and GNOME remembers the binding, so calling BindShortcuts again shows nothing.
- **Choice**: "Change…" calls ConfigureShortcuts on portal version 2 or later. On version 1, or without a portal, a toast points to Settings → Apps → Diktu, with no extra sandbox permission. The onboarding's second click does nothing in that case, since its window has no toasts.
- **Rejected**: opening Settings' panel through D-Bus, because it needs `--talk-name=org.gnome.Settings`; binding again under a new shortcut id, because it leaves stale entries in Settings.
