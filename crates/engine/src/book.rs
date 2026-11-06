//! Mode Livre (Lusk Texte): a book is an ordinary document.
//!
//! - Its settings ([`BookSettings`]) live in the custom document property [`PROP`], as JSON, so a
//!   book saved as `.docx` stays a book, and changing them is an ordinary undoable edit.
//! - Its elements (title page, dedication, parts, chapters…) are found from the paragraph styles
//!   of the body ([`structure`]): an element runs from its first paragraph to the next element
//!   of the same or a higher rank. Moving an element moves that range of blocks, never a copy of
//!   its text, so nothing can be lost on the way.
//! - Every element of rank 0 or 1 (front and back matter, parts, chapters) starts a section that
//!   begins on a right-hand (odd) page; the front matter is numbered i, ii, iii and the body
//!   1, 2, 3 from its first part or chapter ([`normalize`]).
//!
//! The `.lusklivre` project file is a ZIP holding `manuscrit.docx` (the book, readable by Word)
//! and `livre.json` (a readable copy of the settings) — see [`write_project`].

use std::ops::Range;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use wordcraft_doc::props::LineSpacing;
use wordcraft_doc::props::Rgb;
use wordcraft_doc::section::{NumFormat, SectionStart};
use wordcraft_doc::{Align, Block, CharProps, Document, ParaProps, Paragraph, Style, StyleKind, para_block};

/// The custom property that holds a book's settings.
pub const PROP: &str = "LuskLivre";

/// Millimetres to points.
pub fn mm(v: f32) -> f32 {
    v * 72.0 / 25.4
}

/// Points to millimetres, rounded to a tenth.
pub fn to_mm(v: f32) -> f32 {
    (v * 25.4 / 72.0 * 10.0).round() / 10.0
}

/// A book's settings, as the creation assistant asks for them. Every field can change later
/// (`book.settings`); sizes are millimetres, the font size points.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BookSettings {
    pub version: u32,
    pub title: String,
    pub subtitle: String,
    pub author: String,
    /// `roman`, `recit`, `biographie`, `essai`, `poesie`, `illustre` or `autre`.
    pub kind: String,
    /// `a5`, `a4`, `a6`, `poche`, `6x9` or `custom` (then [`Self::page_w_mm`] and
    /// [`Self::page_h_mm`] are the user's).
    pub format: String,
    pub page_w_mm: f32,
    pub page_h_mm: f32,
    /// `blanc` or `creme`.
    pub paper: String,
    pub font: String,
    pub size: f32,
    /// A multiple of single spacing (1.0–3.0).
    pub line_spacing: f32,
    /// `justify` or `left`.
    pub align: String,
    pub inner_mm: f32,
    pub outer_mm: f32,
    pub top_mm: f32,
    pub bottom_mm: f32,
    /// First-line indent of body paragraphs.
    pub indent_mm: f32,
    pub cover: bool,
    /// The editorial preset the settings came from ([`PRESETS`]), or empty.
    pub profile: String,
    /// Running heads ([`RUNNING_HEADS`]): `auteurTitre` (the author on left pages, the title on
    /// right pages), `titreChapitre` (the title, then the current chapter), `aucun`, or `manuel`
    /// (the headers and footers are the user's own: the book leaves them alone).
    pub running_heads: String,
    /// Where the page number goes ([`FOLIOS`]): `basCentre`, `basExterieur`, `hautExterieur`
    /// or `aucun`.
    pub folio: String,
    /// A drop cap opens every chapter's first paragraph.
    pub drop_caps: bool,
    /// The header and footer parts the book made last time, replaced when they are made again.
    pub header_parts: Vec<u32>,
    /// The printer's specifications, as the printer gives them (never guessed): its name, the
    /// bleed it asks for around each page and the thickness of one sheet of the chosen paper
    /// (two pages), millimetres; 0 = not given.
    pub printer: String,
    pub bleed_mm: f32,
    pub sheet_mm: f32,
    /// The cover (Studio Couverture): background and text colours (`#RRGGBB`), a picture for the
    /// front (a path on this computer), the back-cover text, the ISBN, and whether to keep a
    /// white space for the bar code on the back.
    pub cover_bg: String,
    pub cover_fg: String,
    pub cover_image: String,
    pub back_text: String,
    pub isbn: String,
    pub barcode_space: bool,
}

/// The running heads offered: id and label.
pub const RUNNING_HEADS: &[(&str, &str)] = &[
    ("auteurTitre", "Auteur à gauche, titre à droite"),
    ("titreChapitre", "Titre à gauche, chapitre à droite"),
    ("aucun", "Aucun titre courant"),
    ("manuel", "Personnalisés (je modifie les en-têtes moi-même)"),
];

/// Where the page number can go: id and label.
pub const FOLIOS: &[(&str, &str)] = &[
    ("basCentre", "En bas, au centre"),
    ("basExterieur", "En bas, côté extérieur"),
    ("hautExterieur", "En haut, côté extérieur"),
    ("aucun", "Pas de numéro"),
];

impl Default for BookSettings {
    fn default() -> Self {
        preset("romanClassique").map(|p| p.settings()).unwrap_or_else(|| BookSettings {
            version: 1,
            title: String::new(),
            subtitle: String::new(),
            author: String::new(),
            kind: "roman".into(),
            format: "a5".into(),
            page_w_mm: 148.0,
            page_h_mm: 210.0,
            paper: "creme".into(),
            font: wordcraft_fonts::DEFAULT_FAMILY.into(),
            size: 11.0,
            line_spacing: 1.2,
            align: "justify".into(),
            inner_mm: 20.0,
            outer_mm: 15.0,
            top_mm: 18.0,
            bottom_mm: 20.0,
            indent_mm: 5.0,
            cover: true,
            profile: String::new(),
            running_heads: "auteurTitre".into(),
            folio: "basCentre".into(),
            drop_caps: false,
            header_parts: Vec::new(),
            printer: String::new(),
            bleed_mm: 0.0,
            sheet_mm: 0.0,
            cover_bg: "#1F3A4D".into(),
            cover_fg: "#FFFFFF".into(),
            cover_image: String::new(),
            back_text: String::new(),
            isbn: String::new(),
            barcode_space: true,
        })
    }
}

/// The page formats the assistant offers: id, label, width and height in millimetres.
pub const FORMATS: &[(&str, &str, f32, f32)] = &[
    ("a5", "A5 (148 × 210 mm)", 148.0, 210.0),
    ("poche", "Poche (110 × 178 mm)", 110.0, 178.0),
    ("6x9", "6 × 9 pouces (152,4 × 228,6 mm)", 152.4, 228.6),
    ("a4", "A4 (210 × 297 mm)", 210.0, 297.0),
    ("a6", "A6 (105 × 148 mm)", 105.0, 148.0),
];

/// The kinds of book: id and label.
pub const KINDS: &[(&str, &str)] = &[
    ("roman", "Roman"),
    ("recit", "Récit"),
    ("biographie", "Biographie"),
    ("essai", "Essai"),
    ("poesie", "Poésie"),
    ("illustre", "Livre illustré"),
    ("autre", "Autre"),
];

/// An editorial preset: page, type and margins chosen together.
#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub id: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    kind: &'static str,
    format: &'static str,
    paper: &'static str,
    size: f32,
    line_spacing: f32,
    align: &'static str,
    /// Inner, outer, top, bottom margins and first-line indent, millimetres.
    margins: [f32; 5],
    running_heads: &'static str,
    folio: &'static str,
    drop_caps: bool,
}

impl Preset {
    /// The settings this preset gives (no title or author).
    pub fn settings(&self) -> BookSettings {
        let (w, h) = format_size(self.format).unwrap_or((148.0, 210.0));
        let [inner, outer, top, bottom, indent] = self.margins;
        BookSettings {
            version: 1,
            title: String::new(),
            subtitle: String::new(),
            author: String::new(),
            kind: self.kind.into(),
            format: self.format.into(),
            page_w_mm: w,
            page_h_mm: h,
            paper: self.paper.into(),
            font: wordcraft_fonts::DEFAULT_FAMILY.into(),
            size: self.size,
            line_spacing: self.line_spacing,
            align: self.align.into(),
            inner_mm: inner,
            outer_mm: outer,
            top_mm: top,
            bottom_mm: bottom,
            indent_mm: indent,
            cover: true,
            profile: self.id.into(),
            running_heads: self.running_heads.into(),
            folio: self.folio.into(),
            drop_caps: self.drop_caps,
            header_parts: Vec::new(),
            printer: String::new(),
            bleed_mm: 0.0,
            sheet_mm: 0.0,
            cover_bg: "#1F3A4D".into(),
            cover_fg: "#FFFFFF".into(),
            cover_image: String::new(),
            back_text: String::new(),
            isbn: String::new(),
            barcode_space: true,
        }
    }
}

/// The editorial presets offered by the assistant.
pub const PRESETS: &[Preset] = &[
    Preset {
        id: "romanClassique",
        label: "Roman classique",
        description: "A5, papier crème, justifié, auteur et titre en haut de page",
        kind: "roman",
        format: "a5",
        paper: "creme",
        size: 11.0,
        line_spacing: 1.2,
        align: "justify",
        margins: [20.0, 15.0, 18.0, 20.0, 5.0],
        running_heads: "auteurTitre",
        folio: "basCentre",
        drop_caps: false,
    },
    Preset {
        id: "romanContemporain",
        label: "Roman contemporain",
        description: "A5, papier blanc, lettrines, chapitre en haut de page",
        kind: "roman",
        format: "a5",
        paper: "blanc",
        size: 10.5,
        line_spacing: 1.3,
        align: "justify",
        margins: [18.0, 14.0, 18.0, 20.0, 5.0],
        running_heads: "titreChapitre",
        folio: "basExterieur",
        drop_caps: true,
    },
    Preset {
        id: "romanPoche",
        label: "Roman de poche",
        description: "Format poche, texte compact et justifié",
        kind: "roman",
        format: "poche",
        paper: "creme",
        size: 10.0,
        line_spacing: 1.15,
        align: "justify",
        margins: [15.0, 12.0, 14.0, 16.0, 4.0],
        running_heads: "auteurTitre",
        folio: "basCentre",
        drop_caps: false,
    },
    Preset {
        id: "essai",
        label: "Essai",
        description: "6 × 9 pouces, papier blanc, interligne aéré, folio en haut",
        kind: "essai",
        format: "6x9",
        paper: "blanc",
        size: 11.0,
        line_spacing: 1.3,
        align: "justify",
        margins: [22.0, 16.0, 20.0, 22.0, 6.0],
        running_heads: "titreChapitre",
        folio: "hautExterieur",
        drop_caps: false,
    },
    Preset {
        id: "biographie",
        label: "Biographie",
        description: "A5, papier blanc, texte justifié",
        kind: "biographie",
        format: "a5",
        paper: "blanc",
        size: 11.0,
        line_spacing: 1.25,
        align: "justify",
        margins: [20.0, 15.0, 18.0, 20.0, 5.0],
        running_heads: "auteurTitre",
        folio: "basExterieur",
        drop_caps: false,
    },
    Preset {
        id: "poesie",
        label: "Poésie",
        description: "A5, aligné à gauche, sans retrait ni titre courant",
        kind: "poesie",
        format: "a5",
        paper: "blanc",
        size: 11.0,
        line_spacing: 1.4,
        align: "left",
        margins: [22.0, 18.0, 20.0, 22.0, 0.0],
        running_heads: "aucun",
        folio: "basCentre",
        drop_caps: false,
    },
];

/// The preset with this id.
pub fn preset(id: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.id == id)
}

/// The size of a named format, millimetres.
pub fn format_size(id: &str) -> Option<(f32, f32)> {
    FORMATS.iter().find(|f| f.0 == id).map(|f| (f.2, f.3))
}

impl BookSettings {
    /// The settings made safe to apply: sizes within what a page can hold, known choices.
    pub fn sanitized(mut self) -> Self {
        let clean = |s: &str, max: usize| -> String { s.chars().filter(|c| !c.is_control()).take(max).collect::<String>().trim().to_string() };
        self.version = 1;
        self.title = clean(&self.title, 300);
        self.subtitle = clean(&self.subtitle, 300);
        self.author = clean(&self.author, 200);
        self.font = clean(&self.font, 100);
        if self.font.is_empty() {
            self.font = wordcraft_fonts::DEFAULT_FAMILY.into();
        }
        if !KINDS.iter().any(|k| k.0 == self.kind) {
            self.kind = "autre".into();
        }
        if let Some((w, h)) = format_size(&self.format) {
            self.page_w_mm = w;
            self.page_h_mm = h;
        } else {
            self.format = "custom".into();
        }
        let fin = |v: f32, lo: f32, hi: f32, d: f32| if v.is_finite() { v.clamp(lo, hi) } else { d };
        self.page_w_mm = fin(self.page_w_mm, 50.0, 600.0, 148.0);
        self.page_h_mm = fin(self.page_h_mm, 50.0, 600.0, 210.0);
        self.size = fin(self.size, 6.0, 30.0, 11.0);
        self.line_spacing = fin(self.line_spacing, 1.0, 3.0, 1.2);
        self.inner_mm = fin(self.inner_mm, 3.0, 60.0, 20.0);
        self.outer_mm = fin(self.outer_mm, 3.0, 60.0, 15.0);
        self.top_mm = fin(self.top_mm, 3.0, 60.0, 18.0);
        self.bottom_mm = fin(self.bottom_mm, 3.0, 60.0, 20.0);
        self.indent_mm = fin(self.indent_mm, 0.0, 30.0, 5.0);
        // At least 40 mm of text across and down.
        let spare_w = (self.page_w_mm - 40.0).max(6.0);
        if self.inner_mm + self.outer_mm > spare_w {
            let k = spare_w / (self.inner_mm + self.outer_mm);
            self.inner_mm *= k;
            self.outer_mm *= k;
        }
        let spare_h = (self.page_h_mm - 40.0).max(6.0);
        if self.top_mm + self.bottom_mm > spare_h {
            let k = spare_h / (self.top_mm + self.bottom_mm);
            self.top_mm *= k;
            self.bottom_mm *= k;
        }
        if self.paper != "blanc" {
            self.paper = "creme".into();
        }
        if self.align != "left" {
            self.align = "justify".into();
        }
        if !RUNNING_HEADS.iter().any(|r| r.0 == self.running_heads) {
            self.running_heads = "auteurTitre".into();
        }
        if !FOLIOS.iter().any(|f| f.0 == self.folio) {
            self.folio = "basCentre".into();
        }
        self.header_parts.truncate(64);
        self.printer = clean(&self.printer, 120);
        self.bleed_mm = fin(self.bleed_mm, 0.0, 10.0, 0.0);
        self.sheet_mm = fin(self.sheet_mm, 0.0, 1.0, 0.0);
        let colour = |c: &str, d: &str| Rgb::parse(c).map_or_else(|| d.to_string(), |rgb| format!("#{:02X}{:02X}{:02X}", rgb.0, rgb.1, rgb.2));
        self.cover_bg = colour(&self.cover_bg, "#1F3A4D");
        self.cover_fg = colour(&self.cover_fg, "#FFFFFF");
        self.cover_image = clean(&self.cover_image, 1000);
        // The back-cover text keeps its line breaks.
        self.back_text = self.back_text.chars().filter(|c| *c == '\n' || !c.is_control()).take(5000).collect::<String>().trim().to_string();
        self.isbn =
            self.isbn.chars().filter(|c| c.is_ascii_digit() || matches!(c, '-' | ' ' | 'X' | 'x')).take(20).collect::<String>().trim().to_string();
        self
    }

    /// The spine's width for a book of `pages` pages: one sheet holds two pages; `None` while
    /// the sheet thickness is not given.
    pub fn spine_mm(&self, pages: usize) -> Option<f32> {
        (self.sheet_mm > 0.0).then(|| (pages.div_ceil(2) as f32 * self.sheet_mm * 100.0).round() / 100.0)
    }
}

/// The book settings a document holds, if it is a book.
pub fn settings(doc: &Document) -> Option<BookSettings> {
    let json = doc.custom_prop(PROP)?;
    serde_json::from_str::<BookSettings>(json).ok().map(BookSettings::sanitized)
}

/// Is this document a book?
pub fn is_book(doc: &Document) -> bool {
    settings(doc).is_some()
}

/// The kinds of element a book is made of.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ElementKind {
    TitlePage,
    Dedication,
    Epigraph,
    Preface,
    Foreword,
    Part,
    Chapter,
    Subchapter,
    Acknowledgments,
    Bibliography,
}

impl ElementKind {
    pub const ALL: [ElementKind; 10] = [
        ElementKind::TitlePage,
        ElementKind::Dedication,
        ElementKind::Epigraph,
        ElementKind::Preface,
        ElementKind::Foreword,
        ElementKind::Part,
        ElementKind::Chapter,
        ElementKind::Subchapter,
        ElementKind::Acknowledgments,
        ElementKind::Bibliography,
    ];

    /// The id used by commands (`titlePage`, `chapter`…).
    pub fn id(self) -> &'static str {
        match self {
            ElementKind::TitlePage => "titlePage",
            ElementKind::Dedication => "dedication",
            ElementKind::Epigraph => "epigraph",
            ElementKind::Preface => "preface",
            ElementKind::Foreword => "foreword",
            ElementKind::Part => "part",
            ElementKind::Chapter => "chapter",
            ElementKind::Subchapter => "subchapter",
            ElementKind::Acknowledgments => "acknowledgments",
            ElementKind::Bibliography => "bibliography",
        }
    }

    pub fn from_id(id: &str) -> Option<ElementKind> {
        ElementKind::ALL.into_iter().find(|k| k.id() == id)
    }

    /// The French name shown in the book pane.
    pub fn label(self) -> &'static str {
        match self {
            ElementKind::TitlePage => "Page de titre",
            ElementKind::Dedication => "Dédicace",
            ElementKind::Epigraph => "Épigraphe",
            ElementKind::Preface => "Préface",
            ElementKind::Foreword => "Avant-propos",
            ElementKind::Part => "Partie",
            ElementKind::Chapter => "Chapitre",
            ElementKind::Subchapter => "Sous-chapitre",
            ElementKind::Acknowledgments => "Remerciements",
            ElementKind::Bibliography => "Bibliographie",
        }
    }

    /// The style of the element's first paragraph, which is how the element is found again.
    pub fn style(self) -> &'static str {
        match self {
            ElementKind::TitlePage => "LuskPageTitre",
            ElementKind::Dedication => "LuskDedicace",
            ElementKind::Epigraph => "LuskEpigraphe",
            ElementKind::Preface => "LuskPreface",
            ElementKind::Foreword => "LuskAvantPropos",
            ElementKind::Part => "LuskPartie",
            ElementKind::Chapter => "Heading1",
            ElementKind::Subchapter => "Heading2",
            ElementKind::Acknowledgments => "LuskRemerciements",
            ElementKind::Bibliography => "LuskBibliographie",
        }
    }

    /// 0: front matter, parts and back matter; 1: chapters; 2: sub-chapters. What an element
    /// holds is [`Self::ends_at`]'s.
    pub fn rank(self) -> u8 {
        match self {
            ElementKind::Chapter => 1,
            ElementKind::Subchapter => 2,
            _ => 0,
        }
    }

    /// Does an element of this kind end where an element of kind `next` starts? A part holds the
    /// chapters after it, a chapter its sub-chapters; front and back matter hold nothing.
    fn ends_at(self, next: ElementKind) -> bool {
        match self {
            ElementKind::Part => next.rank() == 0,
            ElementKind::Chapter => next.rank() <= 1,
            ElementKind::Subchapter => true,
            _ => next.rank() <= 1,
        }
    }

    /// Front matter: numbered in lower-case roman numerals, before the body.
    pub fn is_front(self) -> bool {
        matches!(self, ElementKind::TitlePage | ElementKind::Dedication | ElementKind::Epigraph | ElementKind::Preface | ElementKind::Foreword)
    }

    /// Back matter: at the end of the book.
    pub fn is_back(self) -> bool {
        matches!(self, ElementKind::Acknowledgments | ElementKind::Bibliography)
    }

    /// The order of front and back matter in a book (the body sits between 4 and 6).
    fn canonical(self) -> u8 {
        match self {
            ElementKind::TitlePage => 0,
            ElementKind::Dedication => 1,
            ElementKind::Epigraph => 2,
            ElementKind::Preface => 3,
            ElementKind::Foreword => 4,
            ElementKind::Part | ElementKind::Chapter | ElementKind::Subchapter => 5,
            ElementKind::Acknowledgments => 6,
            ElementKind::Bibliography => 7,
        }
    }

    /// The title a new element gets.
    fn default_title(self, n: usize) -> String {
        match self {
            ElementKind::Part => format!("Partie {n}"),
            ElementKind::Chapter => format!("Chapitre {n}"),
            ElementKind::Subchapter => "Nouveau sous-chapitre".into(),
            ElementKind::Dedication => "À …".into(),
            ElementKind::Epigraph => "« Citation »".into(),
            ElementKind::TitlePage => "Titre du livre".into(),
            other => other.label().into(),
        }
    }
}

/// One element of the book, as the book pane lists it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub kind: ElementKind,
    pub label: &'static str,
    pub title: String,
    /// Body block of its first paragraph.
    pub block: usize,
    /// One past its last body block (sub-elements included).
    pub end: usize,
    pub rank: u8,
    /// Words in its blocks, sub-elements included.
    pub words: usize,
}

/// The element a body paragraph starts, from its style.
fn kind_of(doc: &Document, p: &Paragraph) -> Option<ElementKind> {
    let style = p.props.style.as_deref()?;
    if let Some(k) = ElementKind::ALL.into_iter().find(|k| k.style() == style) {
        return Some(k);
    }
    // A document's own heading styles (localized ids such as `Titre1`): its outline level.
    match doc.styles.resolve_para(&p.props).outline_level {
        Some(0) => Some(ElementKind::Chapter),
        Some(1) => Some(ElementKind::Subchapter),
        _ => None,
    }
}

/// The book's elements in document order. An element runs to the next one of the same or a
/// lower rank; the body text before the first element belongs to none.
pub fn structure(doc: &Document) -> Vec<Entry> {
    let mut starts: Vec<(usize, ElementKind, String)> = Vec::new();
    for (i, b) in doc.body.iter().enumerate() {
        if let Block::Para(p) = &**b
            && let Some(k) = kind_of(doc, p)
        {
            let title: String = p.plain_text().chars().filter(|c| !c.is_control()).take(200).collect();
            starts.push((i, k, title.trim().to_string()));
        }
    }
    let n = doc.body.len();
    let mut out = Vec::with_capacity(starts.len());
    for (j, (block, kind, title)) in starts.iter().enumerate() {
        let end = starts.iter().skip(j + 1).find(|(_, k, _)| kind.ends_at(*k)).map_or(n, |(b, _, _)| *b);
        let words = doc.body_word_count(*block..end);
        out.push(Entry { kind: *kind, label: kind.label(), title: title.clone(), block: *block, end, rank: kind.rank(), words });
    }
    out
}

/// The book's word count (the whole body).
pub fn total_words(doc: &Document) -> usize {
    doc.body_word_count(0..doc.body.len())
}

/// Why a structural edit can't be made.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BookError {
    #[error("aucun élément n° {0} dans le livre")]
    NoElement(usize),
    #[error("un élément ne peut pas être déplacé à l'intérieur de lui-même")]
    IntoItself,
    #[error("{0}")]
    Invalid(String),
}

/// A paragraph of `text` in style `style`.
fn styled(text: &str, style: &str) -> Paragraph {
    Paragraph::with_text(text, CharProps::default()).styled(style)
}

/// The blocks a new element starts with.
fn element_blocks(kind: ElementKind, title: &str, settings: Option<&BookSettings>) -> Vec<Block> {
    let body = || Block::Para(styled("", "Normal"));
    match kind {
        ElementKind::TitlePage => {
            let mut v = vec![Block::Para(styled(title, kind.style()))];
            if let Some(s) = settings {
                if !s.subtitle.is_empty() {
                    v.push(Block::Para(styled(&s.subtitle, "LuskSousTitre")));
                }
                if !s.author.is_empty() {
                    v.push(Block::Para(styled(&s.author, "LuskAuteur")));
                }
            }
            v
        }
        ElementKind::Dedication => vec![Block::Para(styled(title, kind.style()))],
        ElementKind::Epigraph => vec![Block::Para(styled(title, kind.style())), Block::Para(styled("— Auteur", "LuskEpigrapheSource"))],
        ElementKind::Part => vec![Block::Para(styled(title, kind.style()))],
        _ => vec![Block::Para(styled(title, kind.style())), body()],
    }
}

/// Where a new element of `kind` goes when no place is given: front matter in its usual order,
/// parts and chapters after the last of the body (before the back matter), a sub-chapter at the
/// end of the chapter holding the caret block `near`, back matter in its order at the end.
fn default_place(doc: &Document, entries: &[Entry], kind: ElementKind, near: Option<usize>) -> usize {
    let n = doc.body.len();
    match kind {
        ElementKind::Subchapter => {
            let chapter = near
                .and_then(|b| entries.iter().rfind(|e| e.kind == ElementKind::Chapter && e.block <= b && b < e.end))
                .or_else(|| entries.iter().rfind(|e| e.kind == ElementKind::Chapter));
            chapter.map_or_else(|| first_back(entries).unwrap_or(n), |c| c.end)
        }
        k if k.is_front() => {
            let order = k.canonical();
            // After the front matter that comes before it in the book, else before the first
            // element that comes after it.
            match entries.iter().rfind(|e| e.kind.is_front() && e.kind.canonical() <= order) {
                Some(e) => e.end,
                None => entries.iter().find(|e| e.kind.canonical() > order).map_or(n, |e| e.block),
            }
        }
        k if k.is_back() => {
            let order = k.canonical();
            match entries.iter().rfind(|e| e.kind.is_back() && e.kind.canonical() <= order) {
                Some(e) => e.end,
                None => entries.iter().find(|e| e.kind.is_back() && e.kind.canonical() > order).map_or(n, |e| e.block),
            }
        }
        // The book's first part takes in the chapters already written: it goes before the first.
        ElementKind::Part if !entries.iter().any(|e| e.kind == ElementKind::Part) => {
            entries.iter().find(|e| e.kind == ElementKind::Chapter).map_or_else(|| first_back(entries).unwrap_or(n), |e| e.block)
        }
        _ => first_back(entries).unwrap_or(n),
    }
}

fn first_back(entries: &[Entry]) -> Option<usize> {
    entries.iter().find(|e| e.kind.is_back()).map(|e| e.block)
}

/// Insert a new element. `title` empty: a default title. `after` (an element index) puts it
/// after that element and what it holds; otherwise [`default_place`]. Returns the new element's
/// first body block.
pub fn insert(doc: &mut Document, kind: ElementKind, title: &str, after: Option<usize>, near: Option<usize>) -> Result<usize, BookError> {
    ensure_styles(doc);
    let entries = structure(doc);
    let at = match after {
        Some(i) => entries.get(i).map(|e| e.end).ok_or(BookError::NoElement(i))?,
        None => default_place(doc, &entries, kind, near),
    };
    let count = entries.iter().filter(|e| e.kind == kind).count() + 1;
    let title = title.trim();
    let title = if title.is_empty() { kind.default_title(count) } else { title.chars().filter(|c| !c.is_control()).take(300).collect() };
    let s = settings(doc);
    let blocks = element_blocks(kind, &title, s.as_ref());
    let at = at.min(doc.body.len());
    // An element added to an untouched empty document takes the place of its empty paragraph.
    let lone_empty =
        doc.body.len() == 1 && matches!(doc.body.first().map(|b| &**b), Some(Block::Para(p)) if p.text.is_empty() && p.objects.is_empty());
    let added = blocks.len();
    let tail: Vec<Arc<Block>> = doc.body.split_off(at);
    doc.body.extend(blocks.into_iter().map(Arc::new));
    doc.body.extend(tail);
    let mut first = at;
    if lone_empty {
        // The empty paragraph is before the new blocks (at 0) or after them.
        if at >= 1 {
            doc.body.remove(0);
            first = at - 1;
        } else if doc.body.len() > added {
            doc.body.remove(added);
        }
    }
    normalize(doc);
    Ok(first)
}

/// Rename element `i`: its first paragraph's text, in that paragraph's first run's formatting.
pub fn rename(doc: &mut Document, i: usize, title: &str) -> Result<(), BookError> {
    let entries = structure(doc);
    let e = entries.get(i).ok_or(BookError::NoElement(i))?;
    let title: String = title.chars().filter(|c| !c.is_control()).take(300).collect();
    let title = title.trim();
    if title.is_empty() {
        return Err(BookError::Invalid("le titre ne peut pas être vide".into()));
    }
    let Some(Block::Para(p)) = doc.body.get_mut(e.block).map(Arc::make_mut) else { return Err(BookError::NoElement(i)) };
    let props = p.runs.first().map(|r| r.props.clone()).unwrap_or_else(|| p.mark.clone());
    let mut fresh = Paragraph::with_text(title, props);
    fresh.props = p.props.clone();
    fresh.mark = p.mark.clone();
    fresh.section = p.section.take();
    *p = fresh;
    Ok(())
}

/// Delete element `i` and everything it holds. The body never ends up empty.
pub fn delete(doc: &mut Document, i: usize) -> Result<Range<usize>, BookError> {
    let entries = structure(doc);
    let e = entries.get(i).ok_or(BookError::NoElement(i))?;
    let range = e.block..e.end.min(doc.body.len());
    doc.body.drain(range.clone());
    if doc.body.is_empty() {
        doc.body.push(para_block(Paragraph::new()));
    }
    normalize(doc);
    Ok(range)
}

/// Where [`move_to`] puts an element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Before the target element.
    Before,
    /// After the target element and everything it holds.
    After,
}

/// Move element `from` (with everything it holds) before or after element `to`. The blocks
/// themselves move; their text is never copied. Returns the moved element's new first block.
pub fn move_to(doc: &mut Document, from: usize, to: usize, place: Place) -> Result<usize, BookError> {
    let entries = structure(doc);
    let src = entries.get(from).ok_or(BookError::NoElement(from))?;
    let dst = entries.get(to).ok_or(BookError::NoElement(to))?;
    let range = src.block..src.end.min(doc.body.len());
    let at = match place {
        Place::Before => dst.block,
        Place::After => dst.end,
    };
    if at > range.start && at < range.end {
        return Err(BookError::IntoItself);
    }
    if at == range.start || at == range.end {
        return Ok(range.start);
    }
    let moved: Vec<Arc<Block>> = doc.body.drain(range.clone()).collect();
    let len = moved.len();
    let at = if at > range.start { at - len } else { at };
    let tail = doc.body.split_off(at.min(doc.body.len()));
    doc.body.extend(moved);
    doc.body.extend(tail);
    normalize(doc);
    Ok(at)
}

/// Put the section breaks where the book needs them — before every element of rank 0 or 1, and
/// nowhere else — each section starting on a right-hand page, the front matter numbered in
/// lower-case roman numerals from i and the body in arabic numerals from 1.
pub fn normalize(doc: &mut Document) {
    let entries = structure(doc);
    let tops: Vec<&Entry> = entries.iter().filter(|e| e.rank <= 1).collect();
    let mut wanted: Vec<usize> = tops.iter().filter(|e| e.block > 0).map(|e| e.block - 1).collect();
    wanted.dedup();
    // A break must sit on a paragraph: a table just before an element gets an empty paragraph.
    let mut shift = 0usize;
    for w in wanted.clone() {
        let w = w + shift;
        if matches!(doc.body.get(w).map(|b| &**b), Some(Block::Table(_))) {
            doc.body.insert(w + 1, para_block(Paragraph::new()));
            shift += 1;
        }
    }
    let entries = structure(doc);
    let tops: Vec<&Entry> = entries.iter().filter(|e| e.rank <= 1).collect();
    let wanted: Vec<usize> = tops.iter().filter(|e| e.block > 0).map(|e| e.block - 1).collect();
    // Breaks that are not wanted join their section to the next one; work from the end so the
    // indices before stay put.
    let existing: Vec<usize> =
        doc.body.iter().enumerate().filter_map(|(i, b)| matches!(&**b, Block::Para(p) if p.section.is_some()).then_some(i)).collect();
    for i in existing.into_iter().rev() {
        if !wanted.contains(&i) || i + 1 == doc.body.len() {
            doc.remove_section_break(i);
        }
    }
    for &w in &wanted {
        if w + 1 >= doc.body.len() {
            continue;
        }
        let has = matches!(doc.body.get(w).map(|b| &**b), Some(Block::Para(p)) if p.section.is_some());
        if !has {
            let props = doc.section_mut(w).clone();
            if let Some(Block::Para(p)) = doc.body.get_mut(w).map(Arc::make_mut) {
                p.section = Some(Box::new(props));
            }
        }
    }
    // Starts and numbering, section by section: a section is identified by the element that
    // opens it (or none, for text before the first element).
    let first_body = tops.iter().find(|e| !e.kind.is_front()).map(|e| e.block);
    let first_front = tops.iter().find(|e| e.kind.is_front()).map(|e| e.block);
    let ends: Vec<usize> = doc.sections().iter().map(|(end, _)| *end).collect();
    let mut start = 0usize;
    for end in ends {
        let opener = tops.iter().find(|e| e.block == start).map(|e| e.kind);
        let front = opener.is_some_and(ElementKind::is_front) || (opener.is_none() && first_front.is_some_and(|f| start < f));
        let sp = doc.section_mut(end);
        sp.start = if start == 0 { SectionStart::NextPage } else { SectionStart::OddPage };
        sp.page_num_format = if front { NumFormat::LowerRoman } else { NumFormat::Decimal };
        sp.page_num_start = if Some(start) == first_body || (front && Some(start) == first_front) || start == 0 { Some(1) } else { None };
        start = end + 1;
    }
    apply_drop_caps(doc);
    apply_headers(doc);
}

/// Lines a book's drop caps drop.
const DROP_CAP_LINES: u8 = 3;

/// The first paragraph of every chapter gets a drop cap when the book asks for them, and loses
/// the one the book gave it when it doesn't.
fn apply_drop_caps(doc: &mut Document) {
    let Some(s) = settings(doc) else { return };
    let firsts: Vec<usize> = structure(doc)
        .iter()
        .filter(|e| e.kind == ElementKind::Chapter)
        .filter_map(|e| {
            (e.block + 1..e.end).find(|&i| match doc.body.get(i).map(|b| &**b) {
                Some(Block::Para(p)) => kind_of(doc, p).is_none(),
                _ => false,
            })
        })
        .collect();
    for i in firsts {
        let Some(Block::Para(p)) = doc.body.get(i).map(|b| &**b) else { continue };
        let want = s.drop_caps.then_some(DROP_CAP_LINES);
        let has = p.props.drop_cap.filter(|n| *n > 0);
        // A drop cap the user set to another size stays; the book only adds or removes its own.
        if want == has || (want.is_some() && has.is_some()) || (want.is_none() && has != Some(DROP_CAP_LINES)) {
            continue;
        }
        if let Some(Block::Para(p)) = doc.body.get_mut(i).map(Arc::make_mut) {
            p.props.drop_cap = want;
            p.touch();
        }
    }
}

/// A piece of a running head or folio.
enum Piece {
    Text(String),
    Field(&'static str),
    Tab,
}

/// A header or footer paragraph of `pieces`, aligned `align`, with the tab stops `tabs`.
fn head_para(pieces: &[Piece], align: Align, tabs: Vec<wordcraft_doc::props::TabStop>, props: &CharProps, style: &str) -> Paragraph {
    let mut p = Paragraph::new().styled(style);
    p.props.align = Some(align);
    p.props.indent_first = Some(0.0);
    p.props.indent_left = Some(0.0);
    p.props.space_before = Some(0.0);
    p.props.space_after = Some(0.0);
    if !tabs.is_empty() {
        p.props.tabs = Some(tabs);
    }
    p.mark = props.clone();
    for piece in pieces {
        let off = p.len();
        let _ = match piece {
            Piece::Text(t) => p.insert_text(off, t, props).map(|_| ()),
            Piece::Tab => p.insert_text(off, "\t", props).map(|_| ()),
            Piece::Field(instr) => p.insert_object(
                off,
                wordcraft_doc::InlineObject::Field {
                    instr: (*instr).into(),
                    result: String::new(),
                    locked: false,
                    dirty: false,
                    code: false,
                    form: String::new(),
                },
                props,
            ),
        };
    }
    p
}

/// The book's headers and footers: running heads on left (even) and right (odd) pages, the
/// folio where the settings put it; nothing on a chapter's first page nor on the title page,
/// the dedication, the epigraph and part pages. Made again on every change of structure or
/// settings, replacing the ones made before; left alone when the settings say `manuel`.
fn apply_headers(doc: &mut Document) {
    use wordcraft_doc::PartKind;
    use wordcraft_doc::props::{TabAlign, TabStop};
    let Some(mut s) = settings(doc) else { return };
    if s.running_heads == "manuel" {
        return;
    }
    for id in std::mem::take(&mut s.header_parts) {
        if doc.parts.get(&id).is_some_and(|p| matches!(p.kind, PartKind::Header | PartKind::Footer)) {
            doc.parts.remove(&id);
        }
    }
    let width = doc.last_section.text_width();
    let props = CharProps { italic: Some(true), size: Some((s.size - 1.0).max(6.0)), font: Some(s.font.clone()), ..CharProps::default() };
    let folio_props = CharProps { italic: None, ..props.clone() };
    let title = if s.title.is_empty() { "Sans titre".to_string() } else { s.title.clone() };
    let author = if s.author.is_empty() { title.clone() } else { s.author.clone() };
    // What the left and right pages say at their top.
    let (left, right) = match s.running_heads.as_str() {
        "auteurTitre" => (Some(Piece::Text(author)), Some(Piece::Text(title))),
        "titreChapitre" => (Some(Piece::Text(title)), Some(Piece::Field("STYLEREF 1"))),
        _ => (None, None),
    };
    let centre = TabStop { pos: width / 2.0, align: TabAlign::Center, leader: Default::default() };
    let edge = TabStop { pos: width, align: TabAlign::Right, leader: Default::default() };
    let top_folio = s.folio == "hautExterieur";
    // Headers: with the folio at the top it sits on the outside edge (left on a left page),
    // the running head centred by a tab; otherwise the running head alone, centred.
    let (even_header, odd_header) = if top_folio {
        let mut even = vec![Piece::Field("PAGE"), Piece::Tab];
        even.extend(left);
        let mut odd = vec![Piece::Tab];
        odd.extend(right);
        odd.extend([Piece::Tab, Piece::Field("PAGE")]);
        (head_para(&even, Align::Left, vec![centre], &props, "Header"), head_para(&odd, Align::Left, vec![centre, edge], &props, "Header"))
    } else {
        let even: Vec<Piece> = left.into_iter().collect();
        let odd: Vec<Piece> = right.into_iter().collect();
        (head_para(&even, Align::Center, Vec::new(), &props, "Header"), head_para(&odd, Align::Center, Vec::new(), &props, "Header"))
    };
    let (even_footer, odd_footer) = match s.folio.as_str() {
        "basCentre" => (
            head_para(&[Piece::Field("PAGE")], Align::Center, Vec::new(), &folio_props, "Footer"),
            head_para(&[Piece::Field("PAGE")], Align::Center, Vec::new(), &folio_props, "Footer"),
        ),
        "basExterieur" => (
            head_para(&[Piece::Field("PAGE")], Align::Left, Vec::new(), &folio_props, "Footer"),
            head_para(&[Piece::Field("PAGE")], Align::Right, Vec::new(), &folio_props, "Footer"),
        ),
        _ => (head_para(&[], Align::Center, Vec::new(), &folio_props, "Footer"), head_para(&[], Align::Center, Vec::new(), &folio_props, "Footer")),
    };
    let mut made = Vec::new();
    let mut add = |doc: &mut Document, kind: PartKind, p: Paragraph| {
        let id = doc.add_part(kind, vec![para_block(p)]);
        made.push(id);
        id
    };
    let eh = add(doc, PartKind::Header, even_header);
    let oh = add(doc, PartKind::Header, odd_header);
    let ef = add(doc, PartKind::Footer, even_footer);
    let of = add(doc, PartKind::Footer, odd_footer);
    let blank_h = add(doc, PartKind::Header, head_para(&[], Align::Center, Vec::new(), &props, "Header"));
    let blank_f = add(doc, PartKind::Footer, head_para(&[], Align::Center, Vec::new(), &props, "Footer"));
    let quiet = |k: Option<ElementKind>| {
        matches!(k, None | Some(ElementKind::TitlePage | ElementKind::Dedication | ElementKind::Epigraph | ElementKind::Part))
    };
    let openers: Vec<Option<ElementKind>> = {
        let entries = structure(doc);
        let mut start = 0usize;
        doc.sections()
            .iter()
            .map(|(end, _)| {
                let k = entries.iter().find(|e| e.block == start && e.rank <= 1).map(|e| e.kind);
                start = end + 1;
                k
            })
            .collect()
    };
    let ends: Vec<usize> = doc.sections().iter().map(|(end, _)| *end).collect();
    for (end, opener) in ends.into_iter().zip(openers) {
        let sp = doc.section_mut(end);
        if quiet(opener) {
            sp.headers = wordcraft_doc::section::HeaderSet { default: Some(blank_h), first: Some(blank_h), even: Some(blank_h) };
            sp.footers = wordcraft_doc::section::HeaderSet { default: Some(blank_f), first: Some(blank_f), even: Some(blank_f) };
            sp.title_page = false;
        } else {
            sp.headers = wordcraft_doc::section::HeaderSet { default: Some(oh), first: Some(blank_h), even: Some(eh) };
            sp.footers = wordcraft_doc::section::HeaderSet { default: Some(of), first: Some(blank_f), even: Some(ef) };
            sp.title_page = true;
        }
    }
    doc.settings.even_odd_headers = true;
    s.header_parts = made;
    if let Ok(json) = serde_json::to_string(&s) {
        doc.set_custom_prop(PROP, &json);
    }
}

/// The paragraph styles a book uses, added when missing (a document's own, edited ones stay).
pub fn ensure_styles(doc: &mut Document) {
    let font = settings(doc).map(|s| s.font);
    let base = |id: &str, name: &str, based: &str, para: ParaProps, chr: CharProps| Style {
        id: id.into(),
        name: name.into(),
        kind: StyleKind::Paragraph,
        based_on: Some(based.into()),
        next: Some("Normal".into()),
        para,
        chr,
        priority: Some(20),
        quick: false,
        ..Style::default()
    };
    let centered = |before: f32, after: f32| ParaProps {
        align: Some(Align::Center),
        indent_first: Some(0.0),
        space_before: Some(before),
        space_after: Some(after),
        keep_next: Some(true),
        ..ParaProps::default()
    };
    let chr = |size: f32, bold: bool, italic: bool| CharProps {
        size: Some(size),
        bold: bold.then_some(true),
        italic: italic.then_some(true),
        font: font.clone(),
        ..CharProps::default()
    };
    let heading_front =
        |id: &str, name: &str| base(id, name, "Heading1", ParaProps { outline_level: Some(0), ..centered(72.0, 36.0) }, chr(18.0, false, false));
    let styles = [
        base("LuskPageTitre", "Titre du livre", "Normal", centered(144.0, 24.0), chr(26.0, true, false)),
        base("LuskSousTitre", "Sous-titre du livre", "Normal", centered(0.0, 48.0), chr(15.0, false, true)),
        base("LuskAuteur", "Auteur du livre", "Normal", centered(24.0, 0.0), chr(14.0, false, false)),
        base(
            "LuskDedicace",
            "Dédicace",
            "Normal",
            ParaProps { align: Some(Align::Right), indent_first: Some(0.0), space_before: Some(144.0), ..ParaProps::default() },
            chr(11.0, false, true),
        ),
        base(
            "LuskEpigraphe",
            "Épigraphe",
            "Normal",
            ParaProps {
                align: Some(Align::Right),
                indent_first: Some(0.0),
                indent_left: Some(mm(30.0)),
                space_before: Some(144.0),
                ..ParaProps::default()
            },
            chr(10.5, false, true),
        ),
        base(
            "LuskEpigrapheSource",
            "Source de l'épigraphe",
            "Normal",
            ParaProps { align: Some(Align::Right), indent_first: Some(0.0), space_before: Some(6.0), ..ParaProps::default() },
            chr(10.0, false, false),
        ),
        heading_front("LuskPreface", "Titre de préface"),
        heading_front("LuskAvantPropos", "Titre d'avant-propos"),
        heading_front("LuskRemerciements", "Titre des remerciements"),
        heading_front("LuskBibliographie", "Titre de bibliographie"),
        base("LuskPartie", "Titre de partie", "Heading1", ParaProps { outline_level: Some(0), ..centered(180.0, 0.0) }, chr(22.0, true, false)),
    ];
    for st in styles {
        if doc.styles.get(&st.id).is_none() {
            doc.styles.styles.push(st);
        }
    }
}

/// Apply a book's settings to the document: page size and mirrored margins in every section,
/// body type, French hyphenation, heading layout; the settings are kept in [`PROP`].
pub fn apply_settings(doc: &mut Document, s: &BookSettings) {
    let s = s.clone().sanitized();
    let (w, h) = (mm(s.page_w_mm), mm(s.page_h_mm));
    let set_page = |sp: &mut wordcraft_doc::SectionProps| {
        sp.page_w = w;
        sp.page_h = h;
        sp.landscape = false;
        sp.margin_left = mm(s.inner_mm);
        sp.margin_right = mm(s.outer_mm);
        sp.margin_top = mm(s.top_mm);
        sp.margin_bottom = mm(s.bottom_mm);
        sp.gutter = 0.0;
        sp.header = mm((s.top_mm * 0.5).max(5.0));
        sp.footer = mm((s.bottom_mm * 0.5).max(5.0));
    };
    set_page(&mut doc.last_section);
    for b in doc.body.iter_mut() {
        if matches!(&**b, Block::Para(p) if p.section.is_some())
            && let Block::Para(p) = Arc::make_mut(b)
            && let Some(sp) = p.section.as_mut()
        {
            set_page(sp);
        }
    }
    doc.settings.mirror_margins = true;
    doc.settings.auto_hyphenation = true;
    if doc.styles.default_chr.lang.is_none() {
        doc.styles.default_chr.lang = Some("fr-FR".into());
    }
    let align = if s.align == "left" { Align::Left } else { Align::Justify };
    if let Some(normal) = doc.styles.get_mut("Normal") {
        normal.chr.font = Some(s.font.clone());
        normal.chr.size = Some(s.size);
        normal.para.align = Some(align);
        normal.para.line_spacing = Some(LineSpacing::Multiple(s.line_spacing));
        normal.para.space_before = Some(0.0);
        normal.para.space_after = Some(0.0);
        normal.para.indent_first = Some(mm(s.indent_mm));
        normal.para.widow_control = Some(true);
    }
    for id in ["Heading1", "Heading2"] {
        if let Some(st) = doc.styles.get_mut(id) {
            st.chr.font = Some(s.font.clone());
            st.para.indent_first = Some(0.0);
            st.para.keep_next = Some(true);
            if id == "Heading1" {
                st.para.align = Some(Align::Center);
                st.para.space_before = Some(72.0);
                st.para.space_after = Some(36.0);
                st.chr.size = Some((s.size * 1.6).round());
            } else {
                st.para.space_before = Some(18.0);
                st.para.space_after = Some(9.0);
                st.chr.size = Some((s.size * 1.2).round());
            }
        }
    }
    // The Lusk styles carry the book's font too.
    for st in doc.styles.styles.iter_mut().filter(|st| st.id.starts_with("Lusk")) {
        st.chr.font = Some(s.font.clone());
    }
    doc.core.title = s.title.clone();
    if !s.author.is_empty() {
        doc.core.creator = s.author.clone();
    }
    if let Ok(json) = serde_json::to_string(&s) {
        doc.set_custom_prop(PROP, &json);
    }
}

/// A new book: its settings applied, a title page when it has a title, and a first chapter.
pub fn new_document(base: Document, s: &BookSettings) -> Document {
    let s = s.clone().sanitized();
    let mut doc = base;
    doc.body = vec![para_block(Paragraph::new())];
    apply_settings(&mut doc, &s);
    ensure_styles(&mut doc);
    if !s.title.is_empty() {
        let title = s.title.clone();
        let _ = insert(&mut doc, ElementKind::TitlePage, &title, None, None);
    }
    let _ = insert(&mut doc, ElementKind::Chapter, "", None, None);
    normalize(&mut doc);
    doc
}

/// Make an ordinary document a book: its headings become its chapters.
pub fn convert(doc: &mut Document, s: &BookSettings) {
    apply_settings(doc, s);
    ensure_styles(doc);
    normalize(doc);
}

/// The bytes of a `.lusklivre` project: `mimetype`, `manuscrit.docx` and `livre.json`.
pub fn write_project(doc: &Document) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let docx = wordcraft_docx::write(doc).map_err(|e| e.to_string())?;
    let settings = settings(doc).unwrap_or_default();
    let json = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut out);
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let deflated = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("mimetype", stored).map_err(|e| e.to_string())?;
        zip.write_all(b"application/x-lusk-livre").map_err(|e| e.to_string())?;
        zip.start_file("manuscrit.docx", stored).map_err(|e| e.to_string())?;
        zip.write_all(&docx).map_err(|e| e.to_string())?;
        zip.start_file("livre.json", deflated).map_err(|e| e.to_string())?;
        zip.write_all(&json).map_err(|e| e.to_string())?;
        zip.finish().map_err(|e| e.to_string())?;
    }
    Ok(out.into_inner())
}

/// The largest manuscript a project may hold, bytes.
const MAX_MANUSCRIPT: u64 = 1 << 30;

/// Read a `.lusklivre` project. Its settings come from the manuscript; `livre.json` fills in
/// when the manuscript has lost them (edited elsewhere).
pub fn read_project(bytes: &[u8]) -> Result<Document, String> {
    use std::io::Read;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| format!("projet de livre illisible : {e}"))?;
    let docx = {
        let entry = zip.by_name("manuscrit.docx").map_err(|_| "projet de livre sans manuscrit (manuscrit.docx)".to_string())?;
        let mut buf = Vec::new();
        entry.take(MAX_MANUSCRIPT).read_to_end(&mut buf).map_err(|e| e.to_string())?;
        buf
    };
    let mut doc = wordcraft_docx::read(&docx).map_err(|e| e.to_string())?;
    if !is_book(&doc)
        && let Ok(entry) = zip.by_name("livre.json")
    {
        let mut buf = Vec::new();
        if entry.take(1 << 20).read_to_end(&mut buf).is_ok()
            && let Ok(s) = serde_json::from_slice::<BookSettings>(&buf)
            && let Ok(json) = serde_json::to_string(&s.sanitized())
        {
            doc.set_custom_prop(PROP, &json);
        }
    }
    Ok(doc)
}

mod cover;
pub mod epub;
pub mod quality;
pub mod versions;
pub use cover::{COVER_PROP, cover_document};

#[cfg(test)]
mod tests;
