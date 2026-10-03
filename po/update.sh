#!/bin/sh
# Extracts translatable messages into po/diktu.pot, then merges them into every catalog.
# Rust has no xgettext parser before gettext 0.24: its strings parse as C, so translatable
# literals stay on one line (a Rust `\` line continuation would read differently in C).
set -eu
cd "$(dirname "$0")/.."
xgettext --from-code=UTF-8 --language=C --keyword= --keyword=tr --keyword=trf \
    --add-comments=Translators --package-name=diktu --msgid-bugs-address=me@gwenael-leger.fr \
    -o po/rust.pot core/src/*.rs app/src/*.rs
# The desktop file's Name is the app name, set by Meson: not translated.
xgettext --from-code=UTF-8 --keyword= --keyword=GenericName --keyword=Comment --keyword=Keywords \
    -o po/desktop.pot data/fr.gwenael_leger.Diktu.desktop.in
xgettext --from-code=UTF-8 -o po/metainfo.pot data/fr.gwenael_leger.Diktu.metainfo.xml.in
msgcat --use-first -o po/diktu.pot po/rust.pot po/desktop.pot po/metainfo.pot
rm po/rust.pot po/desktop.pot po/metainfo.pot
for lang in $(cat po/LINGUAS); do
    if [ -f "po/$lang.po" ]; then
        msgmerge --quiet --update --backup=none "po/$lang.po" po/diktu.pot
    else
        msginit --no-translator --locale="$lang" -i po/diktu.pot -o "po/$lang.po"
    fi
done
