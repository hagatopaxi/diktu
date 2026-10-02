# Tests manuels (session GNOME Wayland réelle)

Ces points ne peuvent pas être vérifiés en CI ni dans une session D-Bus isolée : ils exigent
le compositeur, les portails GNOME, l'extension AppIndicator, un micro et des haut-parleurs.
Cocher chaque case sur Ubuntu (GNOME, Wayland) et Fedora (GNOME, Wayland, extension
AppIndicator installée), en natif (`meson install`) et en Flatpak.

Préparation : modèle français téléchargé depuis les réglages, une application cible ouverte
(éditeur de texte, champ de saisie d'un navigateur, terminal).

## Résultats

| Date | Environnement | Résultat |
|---|---|---|
| 2026-10-02 | Ubuntu 26.04, GNOME Shell 50.1 Wayland, xdg-desktop-portal 1.21.1 (GlobalShortcuts v1), natif et Flatpak | Toutes les cases validées, sauf celles propres à Fedora (pas de machine disponible). F12 s'obtient avec Fn+F12 sur le portable testé. |

## Démarrage

- [x] Premier lancement (aucun modèle installé) : la fenêtre de réglages s'ouvre, ainsi que le dialogue RemoteDesktop (point suivant).
- [x] Lancements suivants (modèle installé, consentement mémorisé) : `diktu` lancé depuis un terminal ne montre aucune fenêtre et rend la main au shell seulement à `Quitter`.
- [x] Une deuxième commande `diktu` ne crée pas de seconde instance ; elle ouvre les réglages.
- [x] Premier lancement : le dialogue « Contrôle à distance » (RemoteDesktop) apparaît au démarrage, pas pendant la première dictée ; il ne demande que le clavier.
- [x] Après acceptation et redémarrage de l'application, le dialogue RemoteDesktop ne réapparaît pas (jeton `restore-token` enregistré : `gsettings get fr.gwenael_leger.Diktu restore-token` non vide).
- [x] Premier lancement : le dialogue GNOME de raccourci global propose `F12`.
- [x] L'icône rouge de contrôle à distance de GNOME disparaît environ 3 s après le dialogue de consentement, puis n'apparaît que pendant la frappe d'une dictée (jusqu'à 3 s après le dernier mot), sans que le dialogue de consentement ne revienne.

## Icône (extension AppIndicator)

- [x] L'icône micro gris clair apparaît dans la barre supérieure.
- [x] Le menu propose « Démarrer la dictée », « Réglages… », « Quitter ».
- [x] Pendant l'écoute l'icône est rouge et l'entrée de menu devient « Arrêter la dictée » ; elle redevient grise à la fin.
- [x] L'icône garde sa couleur en thème clair et en thème sombre.
- [ ] Fedora sans l'extension : l'application fonctionne (raccourci, dictée) sans icône, et le message d'erreur est seulement journalisé.

## Raccourci et dictée

- [x] `F12` démarre l'écoute : son montant court, icône rouge, indicateur micro de GNOME visible.
- [x] Le texte est tapé au curseur au fil de la parole, mot par mot (jamais de mot coupé), sans effacement.
- [x] Les accents (é, è, à, ç, ù, œ) et l'apostrophe typographique (’) sont tapés correctement, y compris avec une disposition clavier non française (ex. US).
- [x] Après ~1,2 s de silence, l'écoute s'arrête seule : le reste du texte est tapé, suivi d'un point et d'une espace ; son descendant ; icône grise ; indicateur micro de GNOME éteint.
- [x] Un second appui sur le raccourci pendant l'écoute l'arrête aussitôt et tape le reste.
- [x] Rien dit pendant 6 s : l'écoute s'arrête sans rien taper.
- [x] Le bip de début n'est pas transcrit comme un mot (haut-parleurs, pas de casque).
- [x] Dictée dans un terminal (GNOME Console/Ptyxis), dans Firefox et dans une application GTK : texte identique.
- [x] Bruit de fond régulier (ventilateur, musique faible) : l'écoute s'arrête quand même après la parole (au pire après 6 s sans nouveau mot).

## Repli sans portail GlobalShortcuts

- [x] Raccourci personnalisé GNOME (Paramètres → Clavier → Raccourcis personnalisés) avec la commande `diktu --toggle` (ou `flatpak run fr.gwenael_leger.Diktu --toggle`) : démarre et arrête la dictée.

## Sons

- [x] Les deux sons durent moins de 300 ms et ne saturent pas.
- [x] Le volume des réglages s'applique ; la case « Sons » les coupe.

## Réglages

- [x] Premier lancement sans modèle : la fenêtre de réglages s'ouvre seule ; une fois un modèle installé, les lancements suivants n'ouvrent aucune fenêtre.
- [x] Le menu de l'icône « Réglages… » et un second lancement de `diktu` ouvrent la même fenêtre (pas de doublon).
- [x] Télécharger : barre de progression « x / 71 Mo », puis état « installé » et bouton de suppression ; la dictée fonctionne sans redémarrer l'application.
- [x] Annuler en cours de téléchargement, puis relancer : le téléchargement reprend où il s'était arrêté (fichier `.part` dans `~/.local/share/diktu/models/`, ou `~/.var/app/fr.gwenael_leger.Diktu/data/diktu/models/` en Flatpak).
- [x] Fermer la fenêtre pendant un téléchargement puis la rouvrir : la progression continue.
- [x] Couper le réseau pendant un téléchargement : un message d'erreur s'affiche dans la fenêtre ; relancer reprend.
- [x] Supprimer : le dossier du modèle disparaît, l'état repasse à « non installé ».
- [x] « Modifier… » du raccourci ouvre la boîte de dialogue GNOME si le portail GlobalShortcuts est en version 2 ou plus, et le nouveau raccourci s'affiche ensuite dans la ligne ; en version 1 (GNOME 50.1 + xdg-desktop-portal 1.21.1), un toast renvoie à Paramètres → Applications → Diktu.
- [x] Silence de fin, majuscule/point final, pause entre touches, sons, volume : chaque réglage s'applique à la dictée suivante sans redémarrage, et persiste après redémarrage.

## Flatpak

- [x] `flatpak run fr.gwenael_leger.Diktu` : mêmes vérifications que ci-dessus (icône, raccourci, dictée, sons, réglages).
- [x] GNOME liste Diktu parmi les applications en arrière-plan (menu des réglages rapides) et ne la ferme pas.
- [x] Le micro est accessible (socket PulseAudio/PipeWire) ; GNOME affiche son indicateur pendant l'écoute seulement.
- [x] Le modèle est stocké dans `~/.var/app/fr.gwenael_leger.Diktu/data/diktu/models/`.
