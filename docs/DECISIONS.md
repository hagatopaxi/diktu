# Décisions

Chaque entrée : contexte, choix, alternative écartée, raison.

## D1 — Identifiant d'application

- **Contexte** : il faut un identifiant inverse-DNS (GApplication, GSettings, Flatpak, AppStream).
- **Choix** : `fr.gwenael_leger.Diktu` (domaine de l'auteur `gwenael-leger.fr`, tiret remplacé par un souligné comme le recommande Flatpak).
- **Écarté** : `io.github.<compte>.Diktu`, aucun compte GitHub vérifiable au moment du choix.
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

- **Choix** : WER agrégé ≤ 25 % sur les 3 extraits Common Voice de `core/tests/data`, mesuré sur le texte réellement tapé par `Dictation` (moteur + émission + fin de parole) : 22,2 % (19,4 % en transcription brute). Normalisation : minuscules, apostrophes et tirets → espaces, ponctuation retirée.
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

## D9 — Détection de fin de parole : VAD énergétique, pas l'endpointing de sherpa-onnx

- **Contexte** : l'endpointing intégré de sherpa-onnx (règle 2 = 1,2 s de silence après parole) a été essayé puis mesuré inutilisable avec le modèle par défaut : (1) le modèle émet le `.` final ~1,5 s après la fin de la parole, ce qui remet à zéro le compteur de silence (fin détectée après 2,8 s) ; (2) l'encodeur décode par blocs de 128 trames (1,28 s), donc toute mesure fondée sur les tokens est quantifiée à 1,28 s, au-dessus du seuil de 1,2 s.
- **Choix** : `Dictation` (core) mesure le silence sur le signal : détecteur d'énergie par trames de 30 ms, plancher de bruit adaptatif (suit les baisses immédiatement, les hausses en ~15 s, initialisé à au plus −40 dBFS), parole = 12 dB au-dessus du plancher et au-dessus de −50 dBFS. Fin d'écoute si : ≥ 200 ms de parole puis `end-silence` (1,2 s par défaut, réglable) de silence ; ou aucun nouveau mot du modèle pendant 6 s (rien dit, ou bruit continu) ; ou 5 min de dictée (garde-fou).
- **Écarté** : endpointing sherpa ; VAD Silero (second modèle à télécharger et à gérer dans le registre).
- **Raison** : précision de 30 ms, aucun modèle supplémentaire, testable avec de l'audio simulé. Le repli « aucun nouveau mot » couvre le bruit non stationnaire que l'énergie ne distingue pas de la voix.

## D10 — Micro ouvert seulement pendant l'écoute, pas de pré-roll continu

- **Contexte** : un pré-roll de 200 ms suppose un micro ouvert en permanence pour garder les 200 dernières ms avant le raccourci.
- **Choix** : le flux `cpal` est ouvert au déclenchement et fermé à la fin de la dictée. Le son de début est joué une fois le flux effectivement ouvert : tout ce que l'utilisateur dit après le bip est capté. Le tampon circulaire (2 s) sert de file entre le callback audio et le thread d'inférence.
- **Écarté** : micro ouvert en continu avec pré-roll de 200 ms.
- **Raison** : GNOME affiche l'indicateur de micro tant qu'un flux est ouvert ; un micro ouvert en permanence par une application de fond est inacceptable pour la vie privée et la batterie. Le bip « micro prêt » rend le pré-roll inutile.

## D11 — Une dictée = un segment

- **Choix** : la fin de parole termine à la fois le segment (vidage du modèle avec `input_finished`, reste tapé suivi d'une espace, point final) et l'écoute. Un second appui sur le raccourci fait de même.
- **Raison** : le comportement attendu arrête l'écoute au premier silence ; un découpage en sous-segments n'aurait aucun effet observable.

## D12 — Session RemoteDesktop ouverte seulement pendant la frappe

- **Contexte** : le portail demande un consentement à la création de session (sauf jeton de restauration valide), et GNOME affiche une icône rouge de contrôle à distance tant qu'une session est ouverte, ce qui inquiète (constaté sur GNOME 50.1).
- **Choix** : la session clavier est ouverte au lancement pour obtenir le consentement (dialogue à ce moment-là plutôt qu'au milieu de la première dictée), puis fermée après 3 s sans texte à taper ; elle est rouverte au premier texte suivant grâce au jeton `restore_token` (GSettings `restore-token`, `PersistMode::ExplicitlyRevoked`), sans dialogue. L'icône rouge n'apparaît donc que pendant une dictée. En cas d'échec d'envoi, la session est rouverte une fois.
- **Coût** : le premier mot de chaque dictée attend la création de la session.

## D13 — Commande de repli `diktu --toggle`

- **Choix** : option de ligne de commande locale qui enregistre l'application, active l'action `app.toggle` (transmise par D-Bus à l'instance principale) puis quitte. C'est la commande à associer à un raccourci personnalisé GNOME si le portail GlobalShortcuts manque.
- **Écarté** : `gapplication action …`, qui exige une application activable par D-Bus (fichier service supplémentaire).
- **Raison** : une seule commande, identique en natif et en Flatpak (`flatpak run fr.gwenael_leger.Diktu --toggle`).

## D14 — cpal 0.17 au lieu de 0.18

- **Contexte** : rodio 0.22.2 (dernière version) dépend de cpal 0.17 ; deux versions de cpal ne peuvent pas coexister (même bibliothèque native `alsa-sys`).
- **Choix** : `cpal = "0.17.3"` dans `core`, partagé avec rodio.
- **Écarté** : cpal 0.18 avec un lecteur de sons maison sur cpal.
- **Raison** : l'API utilisée est identique à une référence près ; une seule pile audio.

## D15 — Sons : WAV synthétisés, GResource, flux de sortie éphémère

- **Choix** : `tools/gen-sounds.py` synthétise deux sons de 160 ms (deux notes sinusoïdales montantes ou descendantes, fondus de 8 ms), versionnés dans `data/sounds/`, embarqués en GResource, joués par rodio sur un flux de sortie ouvert le temps du son. Le son de début est joué quand le micro est réellement ouvert. Œuvre originale générée par code, placée sous CC0.
- **Écarté** : libcanberra (dépendance C supplémentaire, thème de sons système).
- **Raison** : aucun fichier tiers, rien de bloqué sur le périphérique de sortie entre deux dictées.

## D16 — Icône SNI en pixmap rendue depuis le SVG embarqué

- **Choix** : les deux SVG colorés (micro gris clair, micro rouge) sont embarqués en GResource et rendus par gdk-pixbuf en pixmaps ARGB32 (22 et 44 px) transmis par `IconPixmap`. En Flatpak, l'icône est publiée sans nom D-Bus propre (`disable_dbus_name`), comme l'exige le bac à sable.
- **Écarté** : `IconName` + `IconThemePath`, qui suppose des icônes installées et visibles du shell hôte (faux en lançant depuis les sources, et noms imposés par l'export Flatpak).
- **Raison** : même rendu en développement, en natif et en Flatpak ; la couleur ne dépend pas du thème.

## D17 — Un seul runtime tokio pour les portails et l'icône

- **Contexte** : ashpd et ksni partagent une connexion zbus dont les tâches de fond tournent sur le runtime qui l'a créée ; tout objet zbus détruit hors contexte tokio fait paniquer le programme (constaté au premier test de fumée).
- **Choix** : un runtime multi-thread (1 worker) global ; les threads principal et d'injection y entrent pour toute leur durée de vie (`Runtime::enter`).
- **Raison** : plus aucun blocage ni panique quelle que soit la façon dont un thread attend.

## D18 — Format du raccourci préféré et enregistrement de l'application hôte

- **Choix** : déclencheur préféré `F12` (notation de la spécification XDG « shortcuts » ; une touche seule et libre, choisie par l'utilisateur à la place de `Super+Alt+D`). Au démarrage, l'application s'enregistre auprès de `org.freedesktop.host.portal.Registry` (sans effet en Flatpak).
- **Constat** : sans cet enregistrement, le portail GlobalShortcuts refuse une application non sandboxée (« An app id is required ») ; l'enregistrement exige que `fr.gwenael_leger.Diktu.desktop` soit installé. Lancer le binaire depuis l'arbre des sources sans `meson install` ne donne donc que le repli `--toggle`.

## D19 — Fenêtre de réglages unique, masquée à la fermeture

- **Choix** : la fenêtre `AdwPreferencesWindow` est construite une fois et masquée à la fermeture ; chaque modèle a une ligne dont la visibilité suit la langue choisie. Les téléchargements continuent fenêtre fermée.
- **Écarté** : reconstruire la fenêtre et la liste à chaque ouverture ou changement de langue (il faudrait alors retrouver les téléchargements en cours).
- **Raison** : état unique, aucune synchronisation à écrire.
- Le sélecteur de modèle (case radio) n'apparaît que si une langue a plusieurs modèles.

## D20 — Flatpak : runtime GNOME 50, build hors ligne

- **Choix** : `org.gnome.Platform//50` (la 51 existe mais la 50 est la plus déployée au moment du choix et déjà présente sur la machine de dev), extension `rust-stable//25.08` (version du SDK freedesktop de GNOME 50). Crates vendorisées par `flatpak-cargo-generator` (flatpak-builder-tools épinglé au commit `74697c75`) dans `build-aux/cargo-sources.json` ; archive statique sherpa-onnx v1.13.8 (x86_64 et aarch64) en source `file` avec SHA-256 identique au digest publié par GitHub, fournie au build via `SHERPA_ONNX_ARCHIVE_DIR`.
- **Permissions** : celles demandées, plus `--talk-name=org.kde.StatusNotifierWatcher` pour l'icône SNI. Les portails (Background, RemoteDesktop, GlobalShortcuts, notifications) sont toujours accessibles depuis le bac à sable sans permission supplémentaire ; l'application appelle le portail Background au démarrage.
- **Écarté** : `cargo vendor` versionné dans le dépôt (des centaines de Mo de sources).
- **Vérifié** : `flatpak-builder` (org.flatpak.Builder 1.4.9) construit et exporte le paquet hors ligne ; le binaire démarre dans le runtime sans bibliothèque manquante. Sur la machine de dev, il a fallu `flatpak run --no-documents-portal … --disable-rofiles-fuse` (pas de FUSE dans le bac à sable de l'agent) ; ces options ne sont pas nécessaires sur un poste normal.

## D21 — Page d'accueil AppStream

- **Choix** : le dépôt public `https://github.com/hagatopaxi/diktu` (page d'accueil, suivi des tickets, sources).

## D22 — Version minimale de Rust : 1.92

- **Constat** : la valeur initiale (1.88) était fausse ; vérifiée avec la chaîne 1.88, la compilation échoue (cairo-rs/gtk-rs 0.22 exigent 1.92). `cargo +1.92 check --all-targets` passe.
- **Choix** : `rust-version = "1.92"` ; le README demande rustup, les paquets `cargo` d'Ubuntu 24.04 et de Fedora étant trop anciens ou à la limite.

## D23 — Nom : Diktu

- **Contexte** : « Parlotte » s'écrit aussi « parlote » et ne se lit bien qu'en français ; le nom doit se prononcer et s'écrire sans ambiguïté dans toutes les langues.
- **Choix** : « Diktu » (« dicte ! » en espéranto, langue phonétique). Vérifié libre le 2026-10-02 : aucun dépôt GitHub de ce nom, absent de crates.io, PyPI et Flathub (registres de marques non consultés). Identifiant `fr.gwenael_leger.Diktu`, crates `diktu` et `diktu-core`.
