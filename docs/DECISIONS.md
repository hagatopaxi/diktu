# Décisions

Chaque entrée : contexte, choix, alternative écartée, raison.

## D1 — Identifiant d'application

- **Contexte** : il faut un identifiant inverse-DNS (GApplication, GSettings, Flatpak, AppStream).
- **Choix** : `fr.gwenael_leger.Parlotte` (domaine de l'auteur `gwenael-leger.fr`, tiret remplacé par un souligné comme le recommande Flatpak).
- **Écarté** : `io.github.<compte>.Parlotte`, aucun compte GitHub vérifiable au moment du choix.
- **Raison** : identifiant stable, rattaché à un domaine possédé.

## D2 — En-têtes GTK de développement absents sur la machine de build

- **Contexte** : la machine de développement n'a ni `libgtk-4-dev` ni `libadwaita-1-dev` ni `libasound2-dev`, sans accès `sudo`.
- **Choix** : paquets `-dev` téléchargés avec `apt-get download` et extraits dans un sysroot local, pointé par `PKG_CONFIG_SYSROOT_DIR`/`PKG_CONFIG_LIBDIR`. Purement local, rien de tout ça n'est dans le dépôt ; la CI et le README utilisent les paquets système.
- **Écarté** : construire uniquement dans le SDK Flatpak (lent, ~1 Go à télécharger avant le premier `cargo test`).
- **Raison** : garder `cargo test` natif et rapide.

## D3 — Versions minimales GTK/libadwaita

- **Contexte** : la CI tourne sur Ubuntu 24.04 (GTK 4.14, libadwaita 1.5).
- **Choix** : features `gtk4/v4_14` et `libadwaita/v1_5`. `AdwPreferencesWindow` (imposé) n'émet donc pas d'avertissement de dépréciation (déprécié à partir de libadwaita 1.6 au profit d'`AdwPreferencesDialog`).
- **Écarté** : viser libadwaita 1.6+ et `AdwPreferencesDialog`.
- **Raison** : contrainte du cahier des charges et compatibilité Ubuntu 24.04 LTS.
