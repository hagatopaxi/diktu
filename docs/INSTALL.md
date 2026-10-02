# Installing Diktu

Diktu can be installed three ways:

1. [Flatpak bundle from GitHub Releases](#flatpak-bundle-from-github-releases): the simplest.
2. [Flatpak built from source](#flatpak-built-from-source).
3. [Native build with Meson](#native-build-with-meson), installed in `~/.local`.

Whichever you choose, also read [Requirements](#requirements), and see
[First launch](../README.md#first-launch) once it is installed. Diktu is not on Flathub yet.

## Requirements

- GNOME on Wayland with `xdg-desktop-portal-gnome`. The GlobalShortcuts portal exists since
  GNOME 48; on older versions, use the [fallback shortcut](#fallback-shortcut-without-the-globalshortcuts-portal).
- The **AppIndicator** GNOME Shell extension, to see the tray icon:
  - Ubuntu: installed and enabled by default (`gnome-shell-extension-appindicator`).
  - Fedora: `sudo dnf install gnome-shell-extension-appindicator`, then enable it in the
    Extensions application and log out and back in.

  Without the extension, everything works except the icon.

X11 is not a target: Diktu may work there through the portals, without any guarantee.

## Flatpak bundle from GitHub Releases

Each release on <https://github.com/hagatopaxi/diktu/releases> provides two bundles,
`diktu-x86_64.flatpak` and `diktu-aarch64.flatpak`, plus a `SHA256SUMS` file. They are built
by GitHub Actions ([`.github/workflows/flatpak.yml`](../.github/workflows/flatpak.yml)) for
every `v*` tag.

> No release has been published yet (v0.1.0 is planned). Until then, use one of the
> build-from-source methods below.

Download the bundle for your architecture (`uname -m` prints it) and `SHA256SUMS` into the
same directory, then check the download:

```sh
sha256sum --check --ignore-missing SHA256SUMS
```

Diktu runs on the GNOME 50 runtime, which comes from Flathub. Add the Flathub remote if you
do not have it yet, then install the bundle:

```sh
flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user diktu-x86_64.flatpak
flatpak run fr.gwenael_leger.Diktu
```

Replace `x86_64` with `aarch64` on an ARM machine.

## Flatpak built from source

Install the runtime, the SDK, the Rust extension and Flatpak Builder from Flathub, then build
and install from a clone of the repository:

```sh
flatpak remote-add --if-not-exists --user flathub https://dl.flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.gnome.Platform//50 org.gnome.Sdk//50 \
    org.freedesktop.Sdk.Extension.rust-stable//25.08 org.flatpak.Builder
git clone https://github.com/hagatopaxi/diktu
cd diktu
flatpak run org.flatpak.Builder --user --install --force-clean build-dir \
    build-aux/fr.gwenael_leger.Diktu.json
flatpak run fr.gwenael_leger.Diktu
```

The build runs offline: crates come from `build-aux/cargo-sources.json`, sherpa-onnx is
built from source (tag pinned by commit) against Microsoft's prebuilt onnxruntime, and every
archive is pinned by SHA-256. Updating these libraries is described in
[MAINTENANCE.md](MAINTENANCE.md).

### If flatpak-builder fails

- `bwrap: Can't find source path /run/user/1000/doc/...`: the document portal's FUSE mount
  has been lost. Restart it:

  ```sh
  systemctl --user restart xdg-document-portal
  ```

  or run the builder without it, by adding `--no-documents-portal` right after `flatpak run`:

  ```sh
  flatpak run --no-documents-portal org.flatpak.Builder --user --install --force-clean \
      build-dir build-aux/fr.gwenael_leger.Diktu.json
  ```

- `Failure spawning rofiles-fuse`: add `--disable-rofiles-fuse` to the builder options:

  ```sh
  flatpak run org.flatpak.Builder --user --install --force-clean --disable-rofiles-fuse \
      build-dir build-aux/fr.gwenael_leger.Diktu.json
  ```

## Native build with Meson

Build dependencies:

```sh
# Ubuntu 24.04 or newer
sudo apt install build-essential meson libgtk-4-dev libadwaita-1-dev libasound2-dev \
    libglib2.0-dev-bin desktop-file-utils appstream
# Fedora
sudo dnf install gcc meson gtk4-devel libadwaita-devel alsa-lib-devel \
    desktop-file-utils appstream
```

Rust 1.92 or newer is required (a requirement of gtk-rs 0.22). This is usually newer than the
`cargo` shipped by distributions, so install it with [rustup](https://rustup.rs). The build
downloads the sherpa-onnx static library from k2-fsa's GitHub releases.

```sh
git clone https://github.com/hagatopaxi/diktu
cd diktu
meson setup _build --prefix=$HOME/.local --buildtype=release
meson install -C _build
diktu
```

Installing (not just running `cargo build`) is required: the portals only accept a
non-sandboxed application whose `fr.gwenael_leger.Diktu.desktop` file is installed. Make sure
`~/.local/bin` is in your `PATH`.

## Start Diktu at login

Copy the application's `.desktop` file into `~/.config/autostart/`.

Native installation:

```sh
mkdir -p ~/.config/autostart
cp ~/.local/share/applications/fr.gwenael_leger.Diktu.desktop ~/.config/autostart/
```

Flatpak installation (`--user`):

```sh
mkdir -p ~/.config/autostart
cp ~/.local/share/flatpak/exports/share/applications/fr.gwenael_leger.Diktu.desktop \
    ~/.config/autostart/
```

## Changing the shortcut

On first launch, GNOME proposes `F12`; accept it or pick another key. To change it later,
open the preferences (tray menu "Preferences…", or run `diktu` again) and click "Change…".
This opens GNOME's shortcut dialog when the GlobalShortcuts portal is version 2 or later.
With version 1 (for example GNOME 50.1 with xdg-desktop-portal 1.21.1), change it in
GNOME Settings → Apps → Diktu instead.

## Fallback shortcut without the GlobalShortcuts portal

If the GlobalShortcuts portal is missing (GNOME < 48) or was refused, create a custom GNOME
shortcut: Settings → Keyboard → Keyboard Shortcuts → Custom Shortcuts → `+`, with the
command:

```sh
diktu --toggle                                 # native installation
flatpak run fr.gwenael_leger.Diktu --toggle    # Flatpak
```

or from the command line:

```sh
KEY=/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/diktu/
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "['$KEY']"
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEY name 'Diktu'
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEY command 'diktu --toggle'
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEY binding 'F12'
```

The second command replaces the existing list of custom shortcuts. If you already have some,
add the path to the list instead of overwriting it. For the Flatpak, use
`'flatpak run fr.gwenael_leger.Diktu --toggle'` as the command.

## Uninstalling

### Flatpak

```sh
flatpak uninstall --user fr.gwenael_leger.Diktu
```

Add `--delete-data` to also remove `~/.var/app/fr.gwenael_leger.Diktu`, which holds the
downloaded model.

### Native

```sh
rm ~/.local/bin/diktu \
   ~/.local/share/applications/fr.gwenael_leger.Diktu.desktop \
   ~/.local/share/glib-2.0/schemas/fr.gwenael_leger.Diktu.gschema.xml \
   ~/.local/share/metainfo/fr.gwenael_leger.Diktu.metainfo.xml \
   ~/.local/share/icons/hicolor/scalable/apps/fr.gwenael_leger.Diktu.svg
glib-compile-schemas ~/.local/share/glib-2.0/schemas
rm -r ~/.local/share/diktu/models    # downloaded models
```

If you set up autostart, also remove `~/.config/autostart/fr.gwenael_leger.Diktu.desktop`.
