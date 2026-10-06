# Manual tests (real GNOME Wayland session)

These items cannot be checked in CI or in an isolated D-Bus session: they require
the compositor, the GNOME portals, the AppIndicator extension, a microphone and speakers.
Check each box on Ubuntu (GNOME, Wayland) and Fedora (GNOME, Wayland, AppIndicator
extension installed), natively (`meson install`) and in Flatpak.

Setup: French model downloaded from the preferences, a target application open
(text editor, browser input field, terminal).

### Dictating without speaking

Where you cannot talk, `build-aux/fake-mic.sh` replaces the microphone with recordings
(PipeWire's `pw-loopback`, `wpctl`, `pw-play`; nothing is heard on the speakers):

```sh
build-aux/fake-mic.sh on        # virtual microphone becomes the default input
build-aux/fake-mic.sh play core/tests/data/common_voice_fr_27024649.wav   # dictation + clip after 3 s
build-aux/fake-mic.sh off       # previous input back, virtual microphone removed
```

During the 3 s countdown (`play FILE 5` for 5 s), put the cursor in the target application: the
script then starts dictation itself (`--toggle` of the development Flatpak; set `DIKTU_TOGGLE`, e.g.
`DIKTU_TOGGLE="flatpak run fr.gwenael_leger.Diktu --toggle"`, for another build) and plays the clip, so the
two stay in step. Do not press the shortcut. Expected texts are in `core/tests/data/trans.txt`; listening
should stop by itself about 1.2 s after the clip. Any 16 kHz-or-more WAV works, e.g. one made
with `espeak-ng -v fr -w phrase.wav "Bonjour"` (synthetic voices are transcribed less well).
Run `off` when done: the virtual microphone otherwise lasts until the end of the session.

UI labels are quoted in English; when the interface runs in another language, look for the
equivalent label.

## Results

| Date | Environment | Result |
|---|---|---|
| 2026-10-02 | Ubuntu 26.04, GNOME Shell 50.1 Wayland, xdg-desktop-portal 1.21.1 (GlobalShortcuts v1), native and Flatpak | All boxes checked, except the Fedora-specific ones (no machine available). F12 is obtained with Fn+F12 on the laptop tested. |

## Startup

- [ ] First launch (`gsettings reset fr.gwenael_leger.Diktu onboarded` and `restore-token`): only the assistant opens, on the language step; no system dialog appears before the permissions step.
- [ ] Assistant, language: the model row shows the size and license; "Continue" starts the download, whose progress stays visible at the bottom during the next steps; going back and picking another model cancels the first download.
- [ ] Assistant, permissions: "Allow…" opens the Background dialog, then the RemoteDesktop one only once the first is answered; each row then shows a check mark (or a warning with the reason), and the next step follows.
- [ ] Assistant, shortcut: "Choose the Shortcut…" opens GNOME's dialog proposing `F12`; once confirmed, the shortcut appears in the row and the last step follows; the last step names it.
- [ ] Assistant: "Start Using Diktu" closes it; the next launch shows no assistant and no dialog. Closing the assistant before the end shows it again at the next launch.
- [ ] Assistant in another interface language (`LANGUAGE=fr diktu`, then `eo`, `de`, `es`, `ru`, after resetting `onboarded`): every step (titles, descriptions, buttons, rows, progress bar, final step naming the shortcut) is translated, with no truncated text.
- [ ] The symbolic icons of the assistant (lock, keyboard, check mark) are sharp (they looked pixelated under the Broadway backend only).
- [x] Later launches (model installed, consent remembered): `diktu` started from a terminal shows no window and only returns control to the shell on "Quit".
- [x] A second `diktu` command does not create a second instance; it opens the preferences.
- [x] First launch: the "Remote control" (RemoteDesktop) dialog appears before the first dictation; it only asks for the keyboard.
- [x] After accepting and restarting the application, the RemoteDesktop dialog does not reappear (`restore-token` saved: `gsettings get fr.gwenael_leger.Diktu restore-token` is not empty).
- [x] GNOME's red remote-control icon disappears about 3 s after the consent dialog, then only appears while a dictation is being typed (up to 3 s after the last word), without the consent dialog coming back.

## Icon (AppIndicator extension)

- [ ] The light grey Diktu icon (a D holding a microphone) appears in the top bar.
- [x] The menu offers "Start dictation", "Preferences…", "Quit".
- [ ] While listening the icon does not change and the menu entry becomes "Stop dictation"; it goes back to "Start dictation" at the end.
- [ ] The new application icon (blue tile, D holding a microphone) appears in the app grid, the About dialog and Settings → Apps; the development build shows the orange striped variant.
- [x] The icon keeps its color in light and dark themes.
- [ ] Fedora without the extension: the application works (shortcut, dictation) without an icon, and the error message is only logged.

## Shortcut and dictation

- [x] `F12` starts listening: short rising sound, GNOME microphone indicator visible.
- [x] Text is typed at the cursor as you speak, word by word (never a half word), without erasing.
- [x] Accents (é, è, à, ç, ù, œ) and the typographic apostrophe (’) are typed correctly, including with a non-French keyboard layout (e.g. US).
- [x] After ~1.2 s of silence, listening stops by itself: the rest of the text is typed, followed by a period and a space; falling sound; GNOME microphone indicator off.
- [x] A second press of the shortcut while listening stops it immediately and types the rest.
- [x] Nothing said for 6 s: listening stops without typing anything.
- [x] The start beep is not transcribed as a word (speakers, no headset).
- [x] Dictation in a terminal (GNOME Console/Ptyxis), in Firefox and in a GTK application: identical text.
- [x] Steady background noise (fan, quiet music): listening still stops after speech (at worst after 6 s without a new word).

## Fallback without the GlobalShortcuts portal

- [x] GNOME custom shortcut (Settings → Keyboard → Custom Shortcuts) with the command `diktu --toggle` (or `flatpak run fr.gwenael_leger.Diktu --toggle`): starts and stops dictation.

## Sounds

- [x] Both sounds last less than 300 ms and do not clip.
- [x] The volume from the preferences is applied; the "Start and stop sounds" switch mutes them.

## Preferences

- [x] First launch without a model: the preferences window opens by itself; once a model is installed, later launches open no window.
- [x] The icon menu's "Preferences…" and a second launch of `diktu` open the same window (no duplicate).
- [x] Download: progress bar "x / 71 MB", then "installed" state and a remove button; dictation works without restarting the application.
- [x] Cancel during a download, then start again: the download resumes where it stopped (`.part` file in `~/.local/share/diktu/models/`, or `~/.var/app/fr.gwenael_leger.Diktu/data/diktu/models/` in Flatpak).
- [x] Close the window during a download then reopen it: progress continues.
- [x] Cut the network during a download: an error message is shown in the window; starting again resumes.
- [x] Remove: the model folder disappears, the state goes back to "not installed".
- [x] The shortcut's "Change…" button opens the GNOME dialog if the GlobalShortcuts portal is version 2 or later, and the new shortcut is then shown in the row.
- [ ] With version 1 (GNOME 50.1 + xdg-desktop-portal 1.21.1), "Change…" shows the same dialog as at first launch; the chosen shortcut is shown in the row and works, the previous one no longer does.
- [x] End silence, capitalization/final period, pause between keys, sounds, volume: each setting applies to the next dictation without a restart, and persists after a restart.
- [ ] Add a model → Hugging Face `csukuangfj/sherpa-onnx-streaming-zipformer-fr-kroko-2025-08-06`, French: progress "Downloading… x / 71 MB", then "Checking…", then a toast with the check summary; the new model appears, is selected, and dictation works.
- [ ] Add a model → From a Folder… on a folder holding a streaming transducer (in Flatpak, through the file chooser portal): same result, and a copy is in the models folder.
- [ ] Add a Whisper repository (e.g. `csukuangfj/sherpa-onnx-whisper-tiny`): refused with an explanation, nothing left in the models folder (no `.staging-*`).
- [ ] Add a model in a language without a registered one (e.g. English): the language appears in the Language list.
- [ ] Remove an imported model: its row disappears; if it was selected, the default model is used again.
- [ ] Desktop in German, Spanish, Russian or Esperanto (`LANGUAGE=eo diktu`, or `LANGUAGE=eo flatpak run fr.gwenael_leger.Diktu`): menus, preferences, toasts, the About dialog and the app grid entry are translated; with no Diktu setting yet, the Language row shows the desktop language, and English when no model exists for it (e.g. Esperanto).
- [ ] Switch the Language row to each built-in language, download its model and dictate a sentence: the text is typed correctly.
- [ ] "About Diktu" (bottom of the preferences): the dialog shows the version, the GitHub links (website, issues, source code), the permissions list under Details, and under Legal the GPL-3.0, sherpa-onnx (Apache-2.0), ONNX Runtime (MIT) and one license section per model.

## Flatpak

- [x] `flatpak run fr.gwenael_leger.Diktu`: same checks as above (icon, shortcut, dictation, sounds, preferences).
- [x] GNOME lists Diktu among background applications (quick settings menu) and does not close it.
- [x] The microphone is accessible (PulseAudio/PipeWire socket); GNOME shows its indicator only while listening.
- [x] The model is stored in `~/.var/app/fr.gwenael_leger.Diktu/data/diktu/models/`.
