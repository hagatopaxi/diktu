# Tests manuels (session GNOME Wayland réelle)

Ces points ne peuvent pas être vérifiés en CI ni dans une session D-Bus isolée : ils exigent
le compositeur, les portails GNOME, l'extension AppIndicator, un micro et des haut-parleurs.
Cocher chaque case sur Ubuntu (GNOME, Wayland) et Fedora (GNOME, Wayland, extension
AppIndicator installée), en natif (`meson install`) et en Flatpak.

Préparation : modèle français téléchargé depuis les réglages, une application cible ouverte
(éditeur de texte, champ de saisie d'un navigateur, terminal).

## Démarrage

- [ ] `parlotte` lancé depuis un terminal ne montre aucune fenêtre et rend la main au shell seulement à `Quitter`.
- [ ] Une deuxième commande `parlotte` ne crée pas de seconde instance ; elle ouvre les réglages.
- [ ] Premier lancement : le dialogue « Contrôle à distance » (RemoteDesktop) apparaît au démarrage, pas pendant la première dictée ; il ne demande que le clavier.
- [ ] Après acceptation et redémarrage de l'application, le dialogue RemoteDesktop ne réapparaît pas (jeton `restore-token` enregistré : `gsettings get fr.gwenael_leger.Parlotte restore-token` non vide).
- [ ] Premier lancement : le dialogue GNOME de raccourci global propose `Super+Alt+D`.
- [ ] Noter si GNOME affiche un indicateur permanent de « contrôle à distance » pendant que la session clavier est ouverte (comportement du shell, à documenter dans le README le cas échéant).

## Icône (extension AppIndicator)

- [ ] L'icône micro gris clair apparaît dans la barre supérieure.
- [ ] Le menu propose « Démarrer la dictée », « Réglages… », « Quitter ».
- [ ] Pendant l'écoute l'icône est rouge et l'entrée de menu devient « Arrêter la dictée » ; elle redevient grise à la fin.
- [ ] L'icône garde sa couleur en thème clair et en thème sombre.
- [ ] Fedora sans l'extension : l'application fonctionne (raccourci, dictée) sans icône, et le message d'erreur est seulement journalisé.

## Raccourci et dictée

- [ ] `Super+Alt+D` démarre l'écoute : son montant court, icône rouge, indicateur micro de GNOME visible.
- [ ] Le texte est tapé au curseur au fil de la parole, mot par mot (jamais de mot coupé), sans effacement.
- [ ] Les accents (é, è, à, ç, ù, œ) et l'apostrophe typographique (’) sont tapés correctement, y compris avec une disposition clavier non française (ex. US).
- [ ] Après ~1,2 s de silence, l'écoute s'arrête seule : le reste du texte est tapé, suivi d'un point et d'une espace ; son descendant ; icône grise ; indicateur micro de GNOME éteint.
- [ ] Un second appui sur le raccourci pendant l'écoute l'arrête aussitôt et tape le reste.
- [ ] Rien dit pendant 6 s : l'écoute s'arrête sans rien taper.
- [ ] Le bip de début n'est pas transcrit comme un mot (haut-parleurs, pas de casque).
- [ ] Dictée dans un terminal (GNOME Console/Ptyxis), dans Firefox et dans une application GTK : texte identique.
- [ ] Bruit de fond régulier (ventilateur, musique faible) : l'écoute s'arrête quand même après la parole (au pire après 6 s sans nouveau mot).

## Repli sans portail GlobalShortcuts

- [ ] Raccourci personnalisé GNOME (Paramètres → Clavier → Raccourcis personnalisés) avec la commande `parlotte --toggle` (ou `flatpak run fr.gwenael_leger.Parlotte --toggle`) : démarre et arrête la dictée.

## Sons

- [ ] Les deux sons durent moins de 300 ms et ne saturent pas.
- [ ] Le volume des réglages s'applique ; la case « Sons » les coupe.

## Réglages

- [ ] Premier lancement sans modèle : la fenêtre de réglages s'ouvre seule ; une fois un modèle installé, les lancements suivants n'ouvrent aucune fenêtre.
- [ ] Le menu de l'icône « Réglages… » et un second lancement de `parlotte` ouvrent la même fenêtre (pas de doublon).
- [ ] Télécharger : barre de progression « x / 71 Mo », puis état « installé » et bouton de suppression ; la dictée fonctionne sans redémarrer l'application.
- [ ] Annuler en cours de téléchargement, puis relancer : le téléchargement reprend où il s'était arrêté (fichier `.part` dans `~/.local/share/parlotte/models/`, ou `~/.var/app/fr.gwenael_leger.Parlotte/data/parlotte/models/` en Flatpak).
- [ ] Fermer la fenêtre pendant un téléchargement puis la rouvrir : la progression continue.
- [ ] Couper le réseau pendant un téléchargement : un message d'erreur s'affiche dans la fenêtre ; relancer reprend.
- [ ] Supprimer : le dossier du modèle disparaît, l'état repasse à « non installé ».
- [ ] « Modifier… » du raccourci ouvre la boîte de dialogue GNOME (portail GlobalShortcuts version 2) ; le nouveau raccourci s'affiche ensuite dans la ligne.
- [ ] Silence de fin, majuscule/point final, pause entre touches, sons, volume : chaque réglage s'applique à la dictée suivante sans redémarrage, et persiste après redémarrage.

## Flatpak

- [ ] `flatpak run fr.gwenael_leger.Parlotte` : mêmes vérifications que ci-dessus (icône, raccourci, dictée, sons, réglages).
- [ ] GNOME liste Parlotte parmi les applications en arrière-plan (menu des réglages rapides) et ne la ferme pas.
- [ ] Le micro est accessible (socket PulseAudio/PipeWire) ; GNOME affiche son indicateur pendant l'écoute seulement.
- [ ] Le modèle est stocké dans `~/.var/app/fr.gwenael_leger.Parlotte/data/parlotte/models/`.
