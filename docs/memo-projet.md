# Mémo du projet Lusk Texte

Mémoire du projet, pour reprendre le travail dans une nouvelle conversation avec Claude : il suffit
de lui demander de lire ce fichier (`C:\lusk-texte\docs\memo-projet.md`). À tenir à jour à chaque
étape importante. État au 11 novembre 2026.

## Le projet en bref

- **Lusk Texte** : traitement de texte libre et gratuit (Windows, macOS, Linux), en Rust (egui).
  C'est une adaptation française de **WordCraft** (ArtCraft Team, licences MIT ou Apache 2.0).
- Porté par l'**association Lusk** (loi 1901), présidée par Philippe. Contact public :
  contact@lusk.bzh. Site : https://www.lusk.bzh.
- Code : https://github.com/declic29/LuskTX, branche `main`. Sur le PC : `C:\lusk-texte`.
- Version publiée : **1.0.0** (étiquette `v1.0.0`). Il reste une ancienne étiquette `v0.13.0`,
  qu'on peut supprimer.

## Décisions prises (à respecter)

- Éditeur affiché partout : « Lusk » ; copyright « © 2026 Lusk, association loi 1901 », en
  gardant la mention d'origine de WordCraft (NOTICE, LICENSE-MIT).
- Identifiant de l'application : `bzh.lusk.texte`. En interne, le programme s'appelle encore
  `wordcraft` (binaire `wordcraft.exe`, crates `wordcraft-*`, id de mise à jour `wordcraft`) :
  c'est voulu, pour ne pas casser les mises à jour ni les réglages.
- Fichiers publiés uniquement sous le nom `Lusk-Texte-…` (plus aucun `wordcraft-…`).
- Interface en français ; **pas** de traduction de l'interface en breton, seulement le
  dictionnaire breton.
- Dictionnaires intégrés : français (Dicollecte), breton (An Drouizig, converti) et anglais.
- Recherche de mises à jour au démarrage : **cochée par défaut** (Fichier › Mises à jour).
- Onglet Zotero masqué par défaut, réactivable dans Fichier › Options › Général.
- Le bouton Document | Livre n'apparaît que dans les livres, pas dans les documents ordinaires.
- Pas de compte Apple Developer (le Mac reste non signé : clic droit › Ouvrir la première fois).
- Infobulle Google Drive : dire seulement qu'on retrouve le document en ligne (pas « sur le
  téléphone »).
- Documentation technique de WordCraft (AGENTS.md, CLAUDE.md, ROADMAP.md, docs/*.md) supprimée ;
  services hérités retirés (SonarCloud, CodeRabbit, workflows FreeBSD et Nix, contributeurs).
- Dependabot : une fois par mois, à 7 h, heure de Paris.

## Ce qui a été ajouté ou changé par rapport à WordCraft

- Nom, icônes (plume « TX »), écran de démarrage, À propos, paquets Windows/Mac/Linux au nom de
  Lusk Texte et de l'association.
- Accueil : salutation avec l'avatar du renard Lusk (`assets/avatar/lusk-renard.png`).
- Barre de gauche : page **Mises à jour** (sous Options) avec bouton de recherche et case
  « au démarrage ».
- Enregistrer : boutons **OneDrive** et **Google Drive** (grisés s'ils ne sont pas installés),
  avec une icône « ? » d'aide au survol. Google Drive passe par Drive pour ordinateur, sans
  connexion Google dans l'application.
- Aide : page **Confidentialité** (08-confidentialite.md) ; Lusk Texte n'utilise Internet que
  pour les mises à jour et quand l'utilisateur le demande.
- Options › Vérification : liste des dictionnaires intégrés, puis ceux de l'ordinateur.
- README en français, avec les liens de téléchargement directs.
- **Installateur Windows** (novembre 2026) : tout en français, images aux couleurs de Lusk (logo
  et renard, `packaging/windows/images/generate.py`), case « Créer un raccourci sur le Bureau »
  cochée par défaut (choix gardé lors des mises à jour), types de fichiers en français, licence
  précédée d'un résumé en français. Page d'accueil propre : `packaging/windows/lusk-welcome.wxs`
  (adaptée de WiX, licence MS-RL mentionnée dans NOTICE).

## Façon de travailler avec Claude

- Claude travaille dans une copie du dépôt, dans son espace, puis **copie les fichiers modifiés sur
  le PC** (`C:\lusk-texte`), en gardant les fins de ligne Windows du fichier existant.
- **Philippe fait lui-même `git add -A`, `git commit` et `git push`** depuis le PC. Claude ne
  pousse jamais (les rappels automatiques « commit and push » reçoivent toujours cette réponse).
- Les fichiers de `.github\` ne peuvent pas être écrits par Claude : il les envoie en pièce jointe
  et Philippe les copie. Les suppressions se font avec `git rm`, que Claude indique.
- Claude ne peut pas lire les journaux de GitHub Actions : copier-coller le message d'erreur.
- Contrôle de l'installateur Windows sur le PC (WiX 5.0.2 et .NET 8 déjà installés), à lancer
  avant de pousser une modification de `packaging\windows\` :
  `.\packaging\windows\test-install-policy.ps1 -OutputDirectory $env:TEMP\lusk-msi-test`
  (trois lignes `PASS … metadata-only package` attendues).

## Règles du code (héritées de WordCraft, toujours valables)

- Jamais de plantage : pas de `unwrap`, `expect` ou `panic!` dans le code de production ;
  `cargo clippy -D warnings` doit passer.
- Chaque image, police ou dictionnaire du dépôt a sa ligne dans `ATTRIBUTION.md`
  (`cargo xtask assets`). Aucune image ni icône venant de Microsoft ou d'un autre logiciel
  commercial.
- Textes de l'interface : en français, passés par `tl!` ou reconnus par l'audit
  (`crates/ui-egui/src/i18n/audit.rs`).
- Vérification complète : `cargo xtask ci`.

## Publier une nouvelle version

1. Augmenter `version` dans `Cargo.toml` (puis `Cargo.lock`), envoyer sur GitHub.
2. Créer et pousser l'étiquette : `git tag v1.0.1` puis `git push origin v1.0.1`.
3. Le workflow **Publication** (`.github/workflows/release.yml`) fabrique Windows (`.msi`,
   portable), Mac (`.dmg`, runner macos-15) et Linux (AppImage, `.deb`, `.rpm`, `.tar.gz`), avec
   `SHA256SUMS.txt` et des copies aux noms fixes (`Lusk-Texte-Windows.msi`, `Lusk-Texte-Mac.dmg`,
   `Lusk-Texte-Linux.AppImage`, `Lusk-Texte-Linux.deb`).
4. La Release est créée **en brouillon** : la tester, puis cliquer sur **Publish release**. Tant
   qu'elle est en brouillon, les liens du README donnent une erreur 404 et les applications
   installées ne voient pas la mise à jour.
5. Si une étiquette a été posée sur le mauvais commit : supprimer le brouillon, puis
   `git tag -d vX` ; `git push origin :refs/tags/vX` ; `git tag vX` ; `git push origin vX`.
- Pour un essai sans nouvelle version : onglet Actions › Publication › **Run workflow**.

## En attente et idées

- **Essayer le vrai installateur Windows** (pages, images, textes) : jamais vu en vrai, seulement
  en aperçu. Le fabriquer avec Run workflow et donner l'avis à Claude.
- **Signature Windows** (pour éviter l'avertissement SmartScreen) : SignPath a refusé (projet trop
  récent). Pistes : Azure Artifact Signing (environ 10 $/mois, au nom de l'association, déjà
  prévu dans `package.ps1` via les variables AZURE_*) ou Certum open source (environ 49 €/an, au
  nom d'une personne). À décider par l'association.
- **Page de téléchargement sur lusk.bzh**, avec les liens
  `https://github.com/declic29/LuskTX/releases/latest/download/Lusk-Texte-Windows.msi` (etc.).
- Les contrôles GitHub `ci.yml` et `packaging-lint.yml` utilisent encore des actions de l'équipe
  WordCraft (`storytold/...`) : à rendre indépendants s'ils cassent un jour.
- La version web (wasm) n'a pas été vérifiée depuis l'ajout des dictionnaires.
- **Version mobile** (idée) : enregistrer sur Google Drive via le sélecteur de fichiers du
  téléphone, sans connexion Google dans l'application.
