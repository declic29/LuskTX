#!/usr/bin/env python3
"""Adapte le dictionnaire breton d'An Drouizig (Hunspell br_FR) au lecteur de Lusk Texte.

Le lecteur de dictionnaires de Lusk Texte (crates/proof/src/affix.rs) ne gère que des drapeaux
d'un seul caractère. Ce script remplace chaque drapeau long du dictionnaire (« FLAG long » :
a0, m0, p10…) par un seul caractère, et retire des entrées les champs morphologiques
(« po:g ») et les commentaires, que le lecteur n'utilise pas. Les mots et les règles d'affixes
ne changent pas.

    python3 outils/convertir-dictionnaire-breton.py <dossier de hunspell-br> assets/dictionaries/br

Source : https://github.com/Drouizig/hunspell-br (br_FR.aff, br_FR.dic), utilisé sous la licence
MPL 1.1 (le dictionnaire est proposé sous GPL, LGPL ou MPL, au choix).
"""
import os
import sys

src, out = sys.argv[1], sys.argv[2]
aff = open(os.path.join(src, "br_FR.aff"), encoding="utf-8").read().split("\n")
dic = open(os.path.join(src, "br_FR.dic"), encoding="utf-8").read().split("\n")

# Les drapeaux déclarés par les en-têtes PFX/SFX, chacun remplacé par une lettre.
flags = []
for line in aff:
    f = line.split()
    if len(f) >= 4 and f[0] in ("PFX", "SFX") and f[2] in ("Y", "N") and f[3].isdigit() and f[1] not in flags:
        flags.append(f[1])
letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"
if len(flags) > len(letters):
    sys.exit(f"trop de drapeaux : {len(flags)}")
new = {flag: letters[i] for i, flag in enumerate(flags)}
longest_first = sorted(flags, key=len, reverse=True)


def convert(flagstr):
    """Les drapeaux d'une entrée (« a0p10 »), du plus long au plus court ; un reste inconnu est ignoré."""
    out_flags, i = "", 0
    while i < len(flagstr):
        for flag in longest_first:
            if flagstr.startswith(flag, i):
                out_flags += new[flag]
                i += len(flag)
                break
        else:
            i += 1
    return out_flags


aff_out = []
for line in aff:
    f = line.split()
    if f and f[0] == "FLAG":
        continue
    if len(f) >= 4 and f[0] in ("PFX", "SFX") and f[1] in new:
        f[1] = new[f[1]]
        if len(f) >= 5 and "/" in f[3]:
            word, cont = f[3].split("/", 1)
            f[3] = word + "/" + convert(cont)
        line = " ".join(f)
    aff_out.append(line)

entries = []
for line in dic[1:]:
    word = line.split("\t")[0].split(" ")[0].strip()
    if not word or word.startswith("#"):
        continue
    if "/" in word:
        stem, fl = word.split("/", 1)
        fl = convert(fl)
        word = stem + ("/" + fl if fl else "")
    entries.append(word)

os.makedirs(out, exist_ok=True)
with open(os.path.join(out, "br_FR.aff"), "w", encoding="utf-8", newline="\n") as fh:
    fh.write("# Converti pour Lusk Texte par outils/convertir-dictionnaire-breton.py : drapeaux d'un caractère.\n")
    fh.write("\n".join(aff_out))
with open(os.path.join(out, "br_FR.dic"), "w", encoding="utf-8", newline="\n") as fh:
    fh.write(f"{len(entries)}\n" + "\n".join(entries) + "\n")
print(f"{len(flags)} drapeaux, {len(entries)} entrées")
