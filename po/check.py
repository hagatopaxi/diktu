#!/usr/bin/env python3
"""Fails when a catalog in po/LINGUAS lacks a message the Rust sources use (po/update.sh not
run), has an untranslated or fuzzy message, or a translation whose {placeholders} differ from
its msgid. translator-credits may stay empty."""
import re
import subprocess
import sys
from pathlib import Path

po_dir = Path(__file__).resolve().parent
placeholders = lambda s: sorted(re.findall(r"\{\w+\}", s))
errors = []
root = po_dir.parent
sources = sorted(str(p) for d in ("core/src", "app/src") for p in (root / d).glob("*.rs"))
# Same extraction as po/update.sh.
pot = subprocess.run(["xgettext", "--from-code=UTF-8", "--language=C", "--keyword=",
                      "--keyword=tr", "--keyword=trf", "--no-wrap", "-o", "-", *sources],
                     capture_output=True, text=True, check=True).stdout
used = set(re.findall(r'^msgid "(.+)"$', pot, re.M))
for lang in (po_dir / "LINGUAS").read_text().split():
    # msgcat normalizes the file: one line per string, no wrapping.
    text = subprocess.run(["msgcat", "--no-wrap", str(po_dir / f"{lang}.po")],
                          capture_output=True, text=True, check=True).stdout
    for missing in sorted(used - set(re.findall(r'^msgid "(.+)"$', text, re.M))):
        errors.append(f"{lang}: missing, run po/update.sh: {missing}")
    for entry in text.split("\n\n"):
        m = re.search(r'^msgid "(.*)"\nmsgstr "(.*)"', entry, re.M)
        if not m or not m[1] or m[1] == "translator-credits":
            continue
        msgid, msgstr = m[1], m[2]
        if "#, fuzzy" in entry:
            errors.append(f"{lang}: fuzzy: {msgid}")
        elif not msgstr:
            errors.append(f"{lang}: untranslated: {msgid}")
        elif placeholders(msgid) != placeholders(msgstr):
            errors.append(f"{lang}: placeholders differ: {msgid} → {msgstr}")
print("\n".join(errors) or "all catalogs complete")
sys.exit(1 if errors else 0)
