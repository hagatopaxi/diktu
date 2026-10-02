# Parlotte

Dictée vocale locale et en streaming pour GNOME sous Wayland. Un raccourci global lance
l'écoute ; le texte est tapé au curseur de l'application active au fil de la parole ;
l'écoute s'arrête seule après un court silence. La reconnaissance tourne sur le processeur,
sans service en ligne : seul le modèle est téléchargé, une fois, depuis Hugging Face.

Licence : GPL-3.0-or-later.

## Fonctionnement

- L'application tourne en tâche de fond, sans fenêtre, avec une icône micro dans la barre
  supérieure (grise au repos, rouge pendant l'écoute).
- `F12` (proposé à GNOME, qui décide du raccourci final) démarre l'écoute ; un son
  court confirme que le micro est ouvert. Un second appui l'arrête.
- L'écoute s'arrête aussi d'elle-même après 1,2 s de silence (réglable), ou après 6 s sans
  aucun mot reconnu.
- Les mots sont tapés dès qu'ils sont stables (jamais de mot coupé, jamais d'effacement). En fin
  de dictée, le reste est tapé avec une majuscule initiale et un point final (désactivable).
- Le texte est injecté par le portail XDG RemoteDesktop (aucun accès au presse-papiers, ni
  `uinput`, ni `xdotool`).
- Les réglages (menu de l'icône, ou relancer `parlotte`) permettent de choisir la langue et le
  modèle, de le télécharger (progression, annulation, reprise, vérification SHA-256) ou de le
  supprimer, et de changer le raccourci.

Modèle par défaut : [Kroko FR](https://huggingface.co/csukuangfj/sherpa-onnx-streaming-zipformer-fr-kroko-2025-08-06)
(zipformer streaming pour sherpa-onnx, 71 Mo, licence CC-BY-SA des modèles « community »
de [Banafo](https://huggingface.co/Banafo/Kroko-ASR)).

## Prérequis

- GNOME sous Wayland avec `xdg-desktop-portal-gnome`. Le portail GlobalShortcuts existe depuis
  GNOME 48 ; avant, utiliser le [repli](#raccourci-de-repli).
- L'extension **AppIndicator** pour voir l'icône :
  - Ubuntu : présente par défaut (`gnome-shell-extension-appindicator`).
  - Fedora : `sudo dnf install gnome-shell-extension-appindicator`, puis l'activer dans
    l'application Extensions et se reconnecter.

  Sans l'extension, tout fonctionne sauf l'icône.

## Installation avec Flatpak

```sh
flatpak install --user flathub org.gnome.Platform//50 org.gnome.Sdk//50 \
    org.freedesktop.Sdk.Extension.rust-stable//25.08 org.flatpak.Builder
flatpak run org.flatpak.Builder --user --install --force-clean build-dir \
    build-aux/fr.gwenael_leger.Parlotte.json
flatpak run fr.gwenael_leger.Parlotte
```

Le build est hors ligne : les crates viennent de `build-aux/cargo-sources.json`, et la
bibliothèque sherpa-onnx précompilée d'une archive épinglée par SHA-256.

## Installation native

Dépendances de build :

```sh
# Ubuntu 24.04 ou plus récent
sudo apt install build-essential meson libgtk-4-dev libadwaita-1-dev libasound2-dev \
    libglib2.0-dev-bin desktop-file-utils appstream
# Fedora
sudo dnf install gcc meson gtk4-devel libadwaita-devel alsa-lib-devel \
    desktop-file-utils appstream
```

Rust 1.92 ou plus récent est requis (exigence de gtk-rs 0.22), en général plus récent que le
`cargo` des distributions : l'installer avec [rustup](https://rustup.rs). Le build
télécharge la bibliothèque statique sherpa-onnx depuis les releases GitHub de k2-fsa.

```sh
meson setup _build --prefix=$HOME/.local --buildtype=release
meson install -C _build
parlotte
```

L'installation (et pas seulement `cargo build`) est nécessaire : le portail GlobalShortcuts
n'accepte une application non sandboxée que si son fichier
`fr.gwenael_leger.Parlotte.desktop` est installé. Vérifier que `~/.local/bin` est dans le
`PATH`.

Pour lancer Parlotte à l'ouverture de session : copier
`~/.local/share/applications/fr.gwenael_leger.Parlotte.desktop` dans `~/.config/autostart/`.

## Premier lancement

1. La fenêtre de réglages s'ouvre : télécharger le modèle français.
2. GNOME demande l'autorisation de « contrôler le clavier » (portail RemoteDesktop). Accepter ;
   l'autorisation est mémorisée.
3. GNOME propose le raccourci `F12` : accepter ou en choisir un autre.

## Raccourci de repli

Si le portail GlobalShortcuts est absent (GNOME < 48) ou refusé, créer un raccourci
personnalisé : Paramètres → Clavier → Raccourcis clavier → Raccourcis personnalisés → `+`,
avec la commande :

```sh
parlotte --toggle                                 # installation native
flatpak run fr.gwenael_leger.Parlotte --toggle    # Flatpak
```

ou en ligne de commande :

```sh
KEY=/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/parlotte/
gsettings set org.gnome.settings-daemon.plugins.media-keys custom-keybindings "['$KEY']"
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEY name 'Parlotte'
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEY command 'parlotte --toggle'
gsettings set org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:$KEY binding 'F12'
```

(La première commande remplace la liste existante de raccourcis personnalisés ; s'il y en a
déjà, ajouter le chemin à la liste au lieu de l'écraser.)

## Limites connues

- Français uniquement pour l'instant (le registre `data/models.toml` et l'interface gèrent
  plusieurs langues et modèles).
- Le modèle décode par blocs de 1,28 s : les mots apparaissent par salves, avec 1 à 2 s de
  retard sur la parole.
- La fin de parole se détecte à l'énergie du signal : un bruit de fond fort et irrégulier
  (musique, conversation) peut retarder l'arrêt automatique jusqu'à 6 s. Le raccourci arrête
  toujours l'écoute immédiatement.
- Pas de ponctuation dictée par commande vocale ; celle du modèle (virgules, points,
  points d'interrogation) est conservée.
- Les mots déjà tapés ne sont jamais corrigés (pas de BackSpace) : si le modèle révise un mot
  déjà stable, la correction est perdue.
- Les caractères hors de la disposition clavier active (emoji, symboles rares) dépendent de la
  prise en charge des keysyms Unicode par le compositeur.
- X11 n'est pas une cible : cela peut fonctionner via les portails, sans garantie.

## Développement

Voir [CLAUDE.md](CLAUDE.md) pour les commandes et l'architecture,
[docs/DECISIONS.md](docs/DECISIONS.md) pour les choix techniques,
[docs/MANUAL_TESTS.md](docs/MANUAL_TESTS.md) pour la recette manuelle.

```sh
cargo test                                   # tests unitaires et d'intégration
cargo test -p parlotte-core -- --ignored     # télécharge le modèle puis teste la dictée réelle
```

Après un changement de `Cargo.lock`, régénérer les sources Flatpak avec
[flatpak-cargo-generator](https://github.com/flatpak/flatpak-builder-tools/tree/master/cargo) :

```sh
uv run flatpak-cargo-generator.py Cargo.lock -o build-aux/cargo-sources.json
```
