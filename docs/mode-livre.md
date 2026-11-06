# Mode Livre (Lusk Texte)

> **Mis à jour :** 2026-11-06 · **Phases :** 2 (structure), 3 (mise en page), 4 (publication) et 5 (qualité) terminées

Le Mode Livre transforme Lusk Texte en outil d'écriture et de mise en page de livres. Il s'ajoute
au traitement de texte sans rien en retirer : un livre est un document ordinaire, et le Mode
Document reste inchangé.

## Principes

- **Un livre = un seul document.** Une seule pagination, une seule annulation, un seul PDF fidèle à
  l'aperçu. Les éléments du livre (page de titre, dédicace, parties, chapitres…) sont repérés par
  le style de leur premier paragraphe ; un élément court jusqu'au suivant de même rang ou de rang
  supérieur.
- **Déplacer, c'est déplacer des blocs.** `book.move` déplace la plage de blocs de l'élément (et ce
  qu'il contient) ; le texte n'est jamais recopié ni retapé, donc il ne peut pas se perdre.
- **Les réglages vivent dans le document**, dans la propriété personnalisée `LuskLivre` (JSON). Un
  livre enregistré en `.docx` reste un livre ; un changement de réglage s'annule avec Ctrl+Z.
- **Chaque élément de rang 0 ou 1 ouvre une section** qui commence sur une page de droite. Les
  pages liminaires sont numérotées i, ii, iii, le corps 1, 2, 3 à partir de la première partie ou
  du premier chapitre (`book::normalize`, appelé après chaque modification de structure).

## Mise en page (phase 3)

- **Titres courants et folios** (`book::apply_headers`, appelé par `normalize`) : le livre crée
  ses propres parties d'en-tête et de pied de page (en-têtes pairs et impairs, première page de
  section vide) et les remplace à chaque changement ; leurs identifiants sont gardés dans
  `header_parts`. Réglages `runningHeads` (`auteurTitre`, `titreChapitre` avec un champ
  `STYLEREF 1`, `aucun`, `manuel` : le livre ne touche plus aux en-têtes) et `folio`
  (`basCentre`, `basExterieur`, `hautExterieur`, `aucun`). Page de titre, dédicace, épigraphe et
  pages de partie : rien ; première page d'un chapitre : pas de titre courant.
- **Pages blanches de verso** : la page blanche ajoutée pour qu'un chapitre commence sur une page
  de droite (`Page::filler` dans la mise en page) n'a ni titre courant ni folio dans un livre
  (document portant la propriété `LuskLivre`) ; ailleurs elle garde en-tête et pied, comme Word.
- **Langue du livre** (`lang`, `LANGUAGES`) : français (France) par défaut, appliquée par
  `apply_settings` à la langue par défaut du texte et au style Normal ; orthographe, césure,
  contrôle typographique et langue de l'EPUB en dépendent. Un livre antérieur au réglage garde
  la langue de son texte.
- **Lettrines** (`dropCaps`) : 3 lignes sur le premier paragraphe de chaque chapitre ; une
  lettrine d'une autre taille posée par l'utilisateur est conservée.
- **Préréglages** : roman classique, roman contemporain, roman de poche, essai, biographie,
  poésie (format, papier, corps, interligne, marges, titres courants, folio, lettrines).
- **Livre ouvert** (`view.bookSpreads`, `canvas::geometry::place_spreads`) : doubles pages
  jointives au dos, la page 1 seule à droite.
- **Miniatures** : onglet Pages du panneau Livre (`ui.bookPaneTab`), rendu paresseux des pages,
  numéros au format de leur section.

## Publication (phase 4)

- **Spécifications de l'imprimeur** (réglages `printer`, `bleedMm`, `sheetMm`) : saisies par
  l'utilisateur, jamais devinées (0 = non renseigné). Dos = `ceil(pages / 2) × sheetMm`
  (`BookSettings::spine_mm`).
- **PDF pour l'imprimeur** (`book.exportPdf {"purpose": "print"}`) : `PdfOptions::bleed` agrandit
  chaque page du fond perdu, écrit la page finie en `TrimBox` et la feuille en `BleedBox` ;
  étiquettes de pages (`page_labels`) au format de chaque section.
- **PDF pour la lecture** (`"purpose": "reading"`) : sans fond perdu, avec signets et étiquettes.
- **EPUB 3** (`book.exportEpub`, `book::epub`) : un XHTML par élément de rang 0 ou 1, issu de
  l'export HTML rendu bien formé (balises vides fermées, `&nbsp;` en référence numérique, images
  sorties des URI `data:`), `nav.xhtml` d'après la structure, feuille de style typographique,
  `content.opf` (titre, auteur, langue, ISBN, `dcterms:modified`, identifiant stable). Les appels
  de note deviennent des signets le temps de l'export HTML, puis des liens `epub:type="noteref"`
  vers des `aside epub:type="footnote"` en fin de fichier. L'image de première de couverture
  (`coverImage`) ouvre le livre : `cover.xhtml` en tête du `spine`, image `cover-image`
  (convertie en PNG si besoin) et `<meta name="cover">` pour les anciennes liseuses ; si
  l'image est introuvable, l'export le signale et l'EPUB part sans couverture.
- **Accessibilité EPUB** (automatique) : métadonnées schema.org (`accessMode`,
  `accessModeSufficient`, `accessibilityFeature`, `accessibilityHazard`,
  `accessibilitySummary` en français) déduites du contenu (images décrites ou non) ; rôles
  DPUB-ARIA (`doc-noteref`, `doc-footnote`, `doc-toc`, `doc-cover`) ; une cellule d'en-tête de
  tableau vide devient une cellule ordinaire. Aucune déclaration de conformité n'est écrite à la
  place de l'auteur.
- **Validation EPUBCheck** : le test `book::epub::tests::epubcheck_finds_nothing_wrong` exporte un
  livre complet (pages liminaires, partie, chapitres, sous-chapitre, notes, liste, tableau, image,
  lien) et un document vide, et exige zéro erreur et zéro avertissement d'EPUBCheck (W3C, BSD)
  quand `EPUBCHECK_JAR` est défini, et aucun problème d'accessibilité pour DAISY Ace (MIT) quand
  `ACE_BIN` est défini. `outils/test-epub.ps1` télécharge EPUBCheck 5.2.1 dans
  `outils/epubcheck` et, si Node.js est présent, installe Ace dans `outils/ace` (tous deux
  ignorés par git), puis lance ce test, ou valide les `.epub` d'un dossier.
- **Studio Couverture** (`book.cover`, `book::cover`) : un document d'une page (quatrième + dos +
  première + fond perdu) fait d'un tableau à trois cellules colorées ; texte du dos tourné de bas
  en haut ; image de première ; ISBN et emplacement du code-barres. Sa propriété `LuskCouverture`
  donne le fond perdu : l'export PDF écrit la `TrimBox` à l'intérieur (`PdfOptions::trim_inset`).

## Qualité (phase 5)

- **Contrôle qualité** (`book.check`, `book::quality::check`) : ne modifie rien. Relève, avec la
  position, la page et la gravité (`erreur`, `attention`, `conseil`) : pages blanches que ne
  justifie pas un début de section sur page impaire, lignes veuves et orphelines (d'après les
  morceaux de paragraphe de la mise en page ; un saut de page n'en est pas une), titres seuls en
  bas de page, images sous 150 ppp (erreur) ou 300 ppp (conseil) à leur taille affichée et
  rognage compris, images sans texte de remplacement, polices absentes (conseil si un substitut
  métriquement compatible existe) ou gras/italique simulés, format et marges de chaque section
  comparés aux réglages, sommaire comparé à ce qu'écrirait `references.updateToc` (sur une copie
  de la session), hiérarchie des titres, trois paragraphes vides ou plus, doubles espaces,
  typographie française (espaces insécables fines ou normales, apostrophe, guillemets, points de
  suspension, espace avant virgule ou point), sauf dans les URL, les heures et le texte non
  français. Au plus 2 000 problèmes.
- **Corrections sur demande** (`book.fix`) : chaque problème corrigeable porte `was` et `fix` ;
  la commande applique les corrections choisies de la fin vers le début, en une étape
  d'annulation, et saute celles dont le texte a changé depuis l'analyse.
- **Historique des versions** (`book.versionSave`, `book.versionList`, `book.versionOpen`,
  `book.versionDelete`, `book::versions`) : un `.lusklivre` et une fiche `.json` par version,
  dans `<dossier des réglages>/Versions/<titre>-<empreinte de la date de création>/`. Le
  dossier doit être absolu (jamais le dossier courant). Ouvrir une version ajoute un document ;
  le livre n'est jamais remplacé. Une version automatique suit chaque enregistrement demandé
  d'un livre (pas l'enregistrement automatique) ; les 30 dernières sont gardées, les versions
  nommées jamais supprimées sans confirmation.
- **Interface** : onglets Qualité et Versions du panneau Livre (`book_ui/checks.rs`), groupe
  « Vérifier » de l'onglet Livre, `ui.bookPaneTab {"tab": "qualite" | "versions"}`.

## Éléments

| Élément | Style du premier paragraphe | Rang | Place par défaut |
|---|---|---|---|
| Page de titre | `LuskPageTitre` (+ `LuskSousTitre`, `LuskAuteur`) | 0 | début |
| Dédicace | `LuskDedicace` | 0 | après la page de titre |
| Épigraphe | `LuskEpigraphe` (+ `LuskEpigrapheSource`) | 0 | après la dédicace |
| Préface | `LuskPreface` (niveau hiérarchique 1) | 0 | après l'épigraphe |
| Avant-propos | `LuskAvantPropos` (niveau 1) | 0 | après la préface |
| Partie | `LuskPartie` (niveau 1) | 0 | fin du corps |
| Chapitre | `Heading1` (ou tout style de niveau 1) | 1 | fin du corps |
| Sous-chapitre | `Heading2` (ou tout style de niveau 2) | 2 | fin du chapitre du curseur |
| Remerciements | `LuskRemerciements` (niveau 1) | 0 | fin du livre |
| Bibliographie | `LuskBibliographie` (niveau 1) | 0 | fin du livre |

Une partie contient les chapitres qui la suivent, un chapitre ses sous-chapitres ; les pages
liminaires et de fin ne contiennent rien. Un `.docx` ordinaire converti en livre garde ses titres :
les styles de niveau 1 deviennent des chapitres, ceux de niveau 2 des sous-chapitres.

## Fichier de projet `.lusklivre`

Une archive ZIP : `mimetype` (`application/x-lusk-livre`), `manuscrit.docx` (le livre, lisible par
Word) et `livre.json` (copie lisible des réglages, utilisée si le manuscrit les a perdus). Les
ressources et la couverture s'y ajouteront en phase 4.

## Code

| Fichier | Rôle |
|---|---|
| `crates/engine/src/book.rs` | Modèle : réglages, préréglages éditoriaux, structure, insertion, renommage, suppression, déplacement, sections, styles, projet `.lusklivre` |
| `crates/engine/src/book/tests.rs` | Tests du modèle (déplacement sans perte, sections, projet, réglages bornés…) |
| `crates/engine/src/cmd/book.rs` | Commandes `book.info`, `book.convert`, `book.settings`, `book.insert`, `book.rename`, `book.delete`, `book.move`, `book.goto`, `book.exportPdf`, `book.exportEpub`, `book.cover`, `book.check`, `book.fix`, `book.versionSave`, `book.versionList`, `book.versionOpen`, `book.versionDelete`, `view.bookPane`, `view.bookSpreads` |
| `crates/ui-egui/src/canvas/geometry.rs` | Disposition des pages en livre ouvert (`place_spreads`) |
| `crates/engine/src/book/cover.rs` | Couverture complète (Studio Couverture) |
| `crates/engine/src/book/epub.rs` | Export EPUB 3 (notes, validation EPUBCheck) |
| `crates/engine/src/book/quality.rs` | Contrôle qualité et corrections sur demande |
| `crates/engine/src/book/versions.rs` | Historique des versions |
| `crates/ui-egui/src/book_ui/checks.rs` | Onglets Qualité et Versions du panneau Livre |
| `outils/test-epub.ps1` | Validation EPUBCheck sous Windows |
| `crates/pdf/src/export.rs`, `options.rs` | Fond perdu, TrimBox/BleedBox, étiquettes de pages |
| `crates/engine/src/cmd/file.rs` | `file.new {"template": "book", "book": {…}}` : un nouveau livre |
| `crates/engine/src/io_ext.rs`, `io.rs` | Ouverture et enregistrement `.lusklivre` |
| `crates/proof/src/hyphen.rs`, `assets/hyphenation/fr.pat` | Césure française (motifs TeX hyph-fr, MIT) |
| `crates/ui-egui/src/book_ui.rs` | Bascule Document \| Livre, assistant, panneau Livre (glisser-déposer, renommage, menu contextuel), confirmation de suppression, onglet de ruban « Livre » |
| `crates/ui-egui/aide/08-mode-livre.md` | Aide intégrée en français |

Toutes les actions passent par des commandes : elles sont annulables, accessibles au ruban, aux
raccourcis, à la recherche de commandes, à la ligne de commande et au pilotage MCP. Le test
`hostile_params_never_panic` les exerce avec des paramètres invalides.

