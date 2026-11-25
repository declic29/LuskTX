# craft-update, copie Lusk Texte

Ce dossier est une copie du module de mise à jour `craft-update` de
[storytold/craft-libs](https://github.com/storytold/craft-libs) (dossier `crates/update`, commit
`92d392cff132eaf53e13afff77f38b5c11b73aa0`), sous licence MIT ou Apache 2.0 (voir `LICENSE-MIT` et `LICENSE-APACHE`).

Le `Cargo.toml` racine de Lusk Texte la met à la place de l'original (`[patch]`).

## Ce qui change par rapport à l'original

1. **Mise à jour sans signature de code** (`src/self_update.rs`, marqué « Lusk ») : les
   logiciels de l'association ne sont pas signés. L'original ne proposait alors que le lien de
   téléchargement. Ici, une copie non signée se met à jour elle-même : le fichier vient de la
   Release GitHub de l'application et son empreinte SHA-256, publiée dans la même Release
   (`SHA256SUMS.txt`), est vérifiée avant toute installation. Une copie signée n'accepte toujours
   qu'un fichier signé par le même éditeur.
2. **Éditeur « Lusk »** (`src/detect.rs`, `PUBLISHERS`) : une installation Windows est reconnue
   quand son éditeur est « Lusk » (celui de l'installateur), et plus seulement « Learning Machines
   LLC ». Sans cela, la mise à jour d'une copie installée par le .msi ne trouverait pas son code
   produit.

Le `Cargo.toml` de la copie remplace les réglages du workspace de craft-libs par leurs valeurs, et
n'a pas les dépendances de test d'écran.

## Sur Mac

**Mise à jour sans certificat Apple** (`src/install.rs`, marqué « Lusk ») : les applis Mac de
l'association portent une signature « ad hoc », sans certificat de développeur Apple. L'original
n'acceptait que la signature de Learning Machines et refusait donc la mise à jour. Ici, toute
signature intacte est acceptée ; une appli non signée ou abîmée reste refusée, et le fichier
téléchargé est déjà vérifié par son empreinte SHA-256 (`SHA256SUMS.txt` de la Release).

## Mettre à jour la copie

Pour passer à une version plus récente de craft-libs : recopier `crates/update/src` du nouveau
commit, refaire ces modifications, et mettre à jour le commit cité ici et dans le `Cargo.toml` de
la copie.
