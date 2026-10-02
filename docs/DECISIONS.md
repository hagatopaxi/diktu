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

## D4 — Bindings sherpa-onnx

- **Contexte** : il faut un reconnaisseur transducteur streaming utilisable depuis Rust.
- **Choix** : crate officielle `sherpa-onnx` 1.13.8 (k2-fsa, Apache-2.0), liaison statique. Son `build.rs` télécharge l'archive précompilée `sherpa-onnx-v1.13.8-linux-x64-static-lib.tar.bz2` depuis les releases GitHub, ou la prend dans `SHERPA_ONNX_ARCHIVE_DIR` pour un build hors ligne (Flatpak).
- **Écarté** : wrapper FFI maison sur l'API C ; crate tierce `sherpa-rs`.
- **Raison** : les bindings officiels exposent tout ce qu'il faut (flux, endpointing, `input_finished`), un wrapper maison n'apporterait que de la maintenance.

## D5 — Modèle français par défaut

- **Contexte** : deux transducteurs zipformer streaming français non gated existent sur HF. Mesures sur 3 extraits Common Voice (CC0), chunks de 100 ms, 2 threads :
  | Modèle | Taille | WER | RTF | Sortie |
  |---|---|---|---|---|
  | `csukuangfj/sherpa-onnx-streaming-zipformer-fr-kroko-2025-08-06` | 71 Mo | 19,4 % | 0,048 | casse et ponctuation |
  | `shaojieli/sherpa-onnx-streaming-zipformer-fr-2023-04-14` (int8) | 128 Mo | 13,9 % | 0,089 | MAJUSCULES, sans ponctuation |
- **Choix** : Kroko FR. Licence CC-BY-SA (modèles « community » de Banafo, version de la licence non précisée par l'éditeur ; le dépôt HF de conversion renvoie vers `Banafo/Kroko-ASR`). Consignée dans le registre.
- **Écarté** : le modèle shaojieli (Apache-2.0), meilleur WER sur ces 3 extraits mais entraîné sur Common Voice, donc dans son domaine ; tout en capitales et sans ponctuation, inutilisable tel quel pour dicter des messages.
- **Raison** : pour de la dictée, casse et ponctuation natives valent plus que quelques points de WER sur 36 mots ; modèle deux fois plus petit et deux fois plus rapide.

## D6 — Seuil du test de transcription

- **Choix** : WER agrégé ≤ 25 % sur les 3 extraits Common Voice de `core/tests/data` (mesuré : 19,4 %). Normalisation : minuscules, apostrophes et tirets → espaces, ponctuation retirée.
- **Raison** : marge suffisante pour absorber une variation de version de sherpa-onnx, assez basse pour détecter un modèle mal chargé ou un flux mal découpé.

## D7 — Registre : champs `name` et `size`

- **Contexte** : le schéma imposé (`id`, `langs`, `engine`, `license`, `repo`, `revision`, `files[{path, sha256}]`) ne donne ni libellé pour l'UI ni taille totale pour la progression multi-fichiers.
- **Choix** : ajout de `name` (libellé affiché) et `files[].size` (octets, relevé via l'API HF). Le rôle de chaque fichier (encodeur, décodeur, joiner, tokens) est déduit du préfixe de son nom, sans champ dédié.
- **Écarté** : progression par fichier via `Content-Length` (barre qui repart à zéro à chaque fichier).
- **Raison** : une seule barre de progression cohérente.

## D8 — Client HTTP bloquant dans un thread

- **Choix** : `reqwest::blocking` (TLS rustls par défaut), exécuté dans un thread dédié ; annulation par `AtomicBool` vérifié à chaque bloc de 64 Kio, progression par callback.
- **Écarté** : client async intégré à la boucle GLib.
- **Raison** : code linéaire, testable sans runtime async ; l'UI reçoit la progression par canal.
