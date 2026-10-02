# Parlotte

Dictée vocale locale en streaming pour GNOME (Rust, GPL-3.0-or-later). Le texte est tapé au curseur via le portail RemoteDesktop.

## Commandes

- Build : `cargo build` (ou `meson setup _build && meson compile -C _build` pour l'installation complète)
- Tests : `cargo test` ; tests réseau/modèle réels : `cargo test -- --ignored`
- Lint : `cargo fmt --all && cargo clippy --all-targets -- -D warnings`
- Schéma : `glib-compile-schemas --strict --dry-run data`
- Sans en-têtes GTK système (machine de dev actuelle) : `. <sysroot>/env.sh` avant cargo, cf. docs/DECISIONS.md D2.

## Architecture

- `core/` (`parlotte-core`, sans GTK) : audio, moteur STT, émission du texte, registre et téléchargement des modèles.
- `app/` (`parlotte`, binaire) : GTK4/libadwaita, portails (ashpd), icône SNI, sons, réglages.
- `data/` : schéma GSettings, icônes, sons, `.desktop`, metainfo, registre des modèles.
- `meson.build` : appelle cargo puis installe binaire et données.

## Conventions

- Commits Conventional Commits, un commit par jalon validé (fmt, clippy, test verts).
- Toute décision ambiguë est consignée dans `docs/DECISIONS.md`.
- Ce qui exige une session GNOME réelle va dans `docs/MANUAL_TESTS.md`.
- Pas d'abstraction sans deux implémentations réelles ou un besoin de test.
