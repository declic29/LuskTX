//! Contrôle qualité d'un livre (Lusk Texte, phase 5) : ce qui gêne à l'impression ou à la
//! lecture, relevé sans rien modifier.
//!
//! [`check`] parcourt le manuscrit et sa mise en page : pages blanches inattendues, lignes
//! veuves et orphelines, titres isolés en bas de page, images de trop faible résolution ou sans
//! texte de remplacement, polices absentes ou simulées, marges différentes des réglages du livre,
//! sommaire périmé, titres mal hiérarchisés, paragraphes vides en série, doubles espaces et
//! typographie française. Chaque problème donne sa position (pour y aller) et, quand une
//! correction sûre existe, le texte à remplacer et son remplaçant : rien n'est corrigé sans que
//! l'utilisateur le demande ([`apply_fixes`], commande `book.fix`), et une correction vérifie
//! d'abord que le texte n'a pas changé depuis l'analyse.

use std::collections::HashMap;
use std::io::Cursor;

use serde::{Deserialize, Serialize};
use wordcraft_doc::para::OBJ;
use wordcraft_doc::{Block, Document, InlineObject, Paragraph, Path, Pos, StoryRef};
use wordcraft_layout::{DocLayout, Placed};

use super::{settings, structure, to_mm};
use crate::Session;

/// Au-delà, l'analyse s'arrête (un manuscrit très fautif reste lisible dans le panneau).
pub const MAX_ISSUES: usize = 2000;

/// Gravité d'un problème.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    /// Gênant à l'impression ou à la lecture : à corriger.
    Erreur,
    /// Probablement involontaire : à vérifier.
    Attention,
    /// Amélioration conseillée.
    Conseil,
}

/// Un problème relevé.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// Famille : `pageBlanche`, `veuve`, `orpheline`, `titreIsole`, `resolution`, `texteAlt`,
    /// `police`, `marges`, `format`, `sommaire`, `titres`, `paragraphesVides`, `doubleEspace`,
    /// `typographie`.
    pub kind: String,
    pub level: Level,
    pub message: String,
    /// Où aller (début de la sélection).
    pub pos: Pos,
    /// Octets sélectionnés à partir de `pos`.
    pub len: usize,
    /// Page (physique, à partir de 1) quand la mise en page la connaît.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    /// Correction proposée : `pos..pos+len` (qui doit encore valoir `was`) devient `fix`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub was: String,
    /// Commande qui règle le problème d'un coup (par exemple `references.updateToc`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

/// Une correction demandée par l'utilisateur (voir [`Issue::fix`]).
#[derive(Clone, Debug, Deserialize)]
pub struct Fix {
    pub pos: Pos,
    pub len: usize,
    pub was: String,
    pub fix: String,
}

struct Report {
    issues: Vec<Issue>,
}

impl Report {
    fn full(&self) -> bool {
        self.issues.len() >= MAX_ISSUES
    }
    fn push(&mut self, kind: &str, level: Level, message: String, pos: Pos, len: usize, page: Option<usize>) {
        if !self.full() {
            self.issues.push(Issue { kind: kind.into(), level, message, pos, len, page, fix: None, was: String::new(), command: None });
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn push_fix(&mut self, kind: &str, level: Level, message: String, pos: Pos, was: &str, fix: &str, page: Option<usize>) {
        if !self.full() {
            self.issues.push(Issue {
                kind: kind.into(),
                level,
                message,
                pos,
                len: was.len(),
                page,
                fix: Some(fix.into()),
                was: was.into(),
                command: None,
            });
        }
    }
}

/// La page (à partir de 1) où commence le paragraphe `path` du corps.
fn page_of(layout: &DocLayout, path: &Path) -> Option<usize> {
    layout.index.get(&(StoryRef::Body, path.clone())).and_then(|v| v.first()).map(|(p, _)| p + 1)
}

/// Les morceaux (page, lignes) d'un paragraphe du corps, dans l'ordre.
fn pieces(layout: &DocLayout, path: &Path) -> Vec<(usize, usize, usize)> {
    let Some(v) = layout.index.get(&(StoryRef::Body, path.clone())) else { return Vec::new() };
    v.iter()
        .filter_map(|&(page, item)| match layout.pages.get(page)?.items.get(item)? {
            Placed::Lines { l0, l1, para, .. } => Some((page, l1.saturating_sub(*l0), para.lines.len())),
            _ => None,
        })
        .collect()
}

/// Le texte du paragraphe sans les caractères d'objet ni de contrôle.
fn visible(p: &Paragraph) -> String {
    p.text.chars().filter(|c| *c != OBJ && !c.is_control()).collect()
}

/// Niveau hiérarchique (0 = titre 1) d'un paragraphe, s'il en a un.
fn outline(doc: &Document, p: &Paragraph) -> Option<u8> {
    doc.styles.resolve_para(&p.props).outline_level.filter(|l| *l < 9)
}

/// Le paragraphe est-il en français ?
fn french(doc: &Document, p: &Paragraph) -> bool {
    let rc = doc.styles.resolve_char(p.props.style.as_deref(), p.props_at(0));
    rc.lang.as_deref().or(doc.styles.default_chr.lang.as_deref()).is_none_or(|l| l.to_ascii_lowercase().starts_with("fr"))
}

fn short(s: &str, n: usize) -> String {
    let t: String = s.chars().take(n).collect();
    if s.chars().count() > n { format!("{}…", t.trim_end()) } else { t }
}

/// Pages blanches que rien n'explique (une page blanche avant une section qui commence sur une
/// page de droite est voulue).
fn blank_pages(doc: &Document, layout: &DocLayout, r: &mut Report) {
    let n = layout.pages.len();
    for (i, page) in layout.pages.iter().enumerate() {
        let inked = page.items.iter().any(|it| match it {
            Placed::Lines { story, path, para, l0, l1, .. } => doc.para(*story, path).is_some_and(|p| {
                para.lines
                    .get(*l0..*l1)
                    .unwrap_or_default()
                    .iter()
                    .any(|ln| p.text.get(ln.start..ln.stop).is_some_and(|t| t.chars().any(|c| c != OBJ && !c.is_control() && !c.is_whitespace())))
            }),
            Placed::Image { .. } | Placed::Graphic { .. } | Placed::Shape { .. } | Placed::Clip { .. } | Placed::Cell { .. } => true,
            _ => false,
        });
        if inked {
            continue;
        }
        let before_new_section = layout.pages.get(i + 1).is_some_and(|next| next.section != page.section);
        if before_new_section && i + 1 < n {
            continue;
        }
        let why = if i + 1 == n { "en fin de livre" } else { "au milieu du texte" };
        r.push(
            "pageBlanche",
            Level::Attention,
            format!("Page {} blanche {why} : des paragraphes vides ou un saut de page en trop ?", i + 1),
            Pos::body(page.first_block, 0),
            0,
            Some(i + 1),
        );
    }
}

/// Lignes veuves (dernière ligne seule en haut d'une page) et orphelines (première ligne seule
/// en bas d'une page), titres seuls en bas de page.
fn lines_and_headings(doc: &Document, layout: &DocLayout, r: &mut Report) {
    for (i, b) in doc.body.iter().enumerate() {
        if r.full() {
            return;
        }
        let Block::Para(p) = &**b else { continue };
        let path = Path::top(i);
        let ps = pieces(layout, &path);
        if outline(doc, p).is_some() {
            // Un titre dont le texte suivant commence à la page d'après.
            let (Some(last), Some(next)) = (ps.last(), doc.body.get(i + 1)) else { continue };
            if !matches!(&**next, Block::Para(q) if !visible(q).trim().is_empty() && outline(doc, q).is_none()) {
                continue;
            }
            let next_page = pieces(layout, &Path::top(i + 1)).first().map(|x| x.0);
            if next_page.is_some_and(|np| np > last.0) {
                r.push(
                    "titreIsole",
                    Level::Attention,
                    format!("Titre « {} » seul en bas de la page {} : son texte commence à la page suivante.", short(&visible(p), 40), last.0 + 1),
                    Pos::body(i, 0),
                    p.text.len(),
                    Some(last.0 + 1),
                );
            }
            continue;
        }
        // A page or column break ends its line on purpose: no widow or orphan there.
        if ps.len() < 2 || p.text.contains([wordcraft_doc::para::PAGE_BREAK, wordcraft_doc::para::COLUMN_BREAK]) {
            continue;
        }
        let total = ps.first().map_or(0, |x| x.2);
        if total < 2 {
            continue;
        }
        if let Some(first) = ps.first().filter(|x| x.1 == 1) {
            r.push(
                "orpheline",
                Level::Attention,
                format!("Ligne orpheline : la première ligne du paragraphe reste seule en bas de la page {}.", first.0 + 1),
                Pos::body(i, 0),
                0,
                Some(first.0 + 1),
            );
        }
        if let Some(last) = ps.last().filter(|x| x.1 == 1) {
            r.push(
                "veuve",
                Level::Attention,
                format!("Ligne veuve : la dernière ligne du paragraphe passe seule en haut de la page {}.", last.0 + 1),
                Pos::body(i, p.text.len()),
                0,
                Some(last.0 + 1),
            );
        }
    }
}

/// Octet de chaque objet du paragraphe (le k-ième U+FFFC est l'objet k).
fn object_offsets(p: &Paragraph) -> Vec<usize> {
    p.text.char_indices().filter(|(_, c)| *c == OBJ).map(|(i, _)| i).collect()
}

/// Résolution des images et texte de remplacement.
fn pictures(doc: &Document, layout: &DocLayout, r: &mut Report) {
    for path in doc.para_paths(StoryRef::Body) {
        let Some(p) = doc.para(StoryRef::Body, &path) else { continue };
        let offs = object_offsets(p);
        for (k, o) in p.objects.iter().enumerate() {
            if r.full() {
                return;
            }
            let InlineObject::Image { media, w, h, alt, crop, .. } = o else { continue };
            let off = offs.get(k).copied().unwrap_or(0);
            let pos = Pos { story: StoryRef::Body, path: path.clone(), off };
            let page = page_of(layout, &path);
            if let Some(bytes) = doc.media.get(media)
                && let Ok(reader) = image::ImageReader::new(Cursor::new(bytes.as_slice())).with_guessed_format()
                && let Ok((px_w, px_h)) = reader.into_dimensions()
                && *w > 0.5
                && *h > 0.5
            {
                let fw = (1.0 - crop[0] - crop[2]).clamp(0.01, 1.0);
                let fh = (1.0 - crop[1] - crop[3]).clamp(0.01, 1.0);
                let dpi = ((px_w as f32 * fw) / (w / 72.0)).min((px_h as f32 * fh) / (h / 72.0));
                let size = format!("{:.0} × {:.0} mm", to_mm(*w), to_mm(*h));
                if dpi < 150.0 {
                    r.push(
                        "resolution",
                        Level::Erreur,
                        format!(
                            "Image trop peu définie pour l'impression : {dpi:.0} ppp à {size} (300 ppp conseillés). Elle sera floue ou pixellisée."
                        ),
                        pos.clone(),
                        OBJ.len_utf8(),
                        page,
                    );
                } else if dpi < 300.0 {
                    r.push(
                        "resolution",
                        Level::Conseil,
                        format!("Image à {dpi:.0} ppp à {size} : 300 ppp sont conseillés pour l'impression."),
                        pos.clone(),
                        OBJ.len_utf8(),
                        page,
                    );
                }
            }
            if alt.trim().is_empty() {
                r.push(
                    "texteAlt",
                    Level::Conseil,
                    "Image sans texte de remplacement : il est lu à la place de l'image dans l'EPUB (accessibilité).".into(),
                    pos,
                    OBJ.len_utf8(),
                    page,
                );
            }
        }
    }
}

/// Polices absentes de l'ordinateur (remplacées) ou sans gras ou italique (simulés).
fn fonts(doc: &Document, layout: &DocLayout, r: &mut Report) {
    let mut seen: HashMap<(String, bool, bool), ()> = HashMap::new();
    let mut reported_family: HashMap<String, ()> = HashMap::new();
    for path in doc.para_paths(StoryRef::Body) {
        let Some(p) = doc.para(StoryRef::Body, &path) else { continue };
        let mut off = 0;
        for run in &p.runs {
            let start = off;
            off += run.len;
            let text = p.text.get(start..off).unwrap_or_default();
            if text.chars().all(|c| c == OBJ || c.is_whitespace()) {
                continue;
            }
            let rc = doc.styles.resolve_char(p.props.style.as_deref(), &run.props);
            let key = (rc.font.clone(), rc.bold, rc.italic);
            if seen.contains_key(&key) || seen.len() > 500 || r.full() {
                continue;
            }
            seen.insert(key, ());
            let res = wordcraft_fonts::word::resolve(&rc.font, rc.bold, rc.italic);
            let pos = Pos { story: StoryRef::Body, path: path.clone(), off: start };
            let page = page_of(layout, &path);
            if res.substituted {
                if !reported_family.contains_key(&rc.font) {
                    reported_family.insert(rc.font.clone(), ());
                    // A metric-compatible stand-in (Carlito for Calibri…) keeps the pagination.
                    let (level, same) = if wordcraft_fonts::word::has_named_substitutes(&rc.font) {
                        (Level::Conseil, ", de mêmes dimensions,")
                    } else {
                        (Level::Attention, "")
                    };
                    r.push(
                        "police",
                        level,
                        format!(
                            "Police « {} » absente de cet ordinateur : remplacée par « {} »{same} à l'écran et dans le PDF. Installez-la ou choisissez une police présente.",
                            rc.font, res.face.family
                        ),
                        pos,
                        run.len,
                        page,
                    );
                }
            } else if res.synth_bold || res.synth_italic {
                let what = match (res.synth_bold, res.synth_italic) {
                    (true, true) => "gras italique",
                    (true, false) => "gras",
                    _ => "italique",
                };
                r.push(
                    "police",
                    Level::Conseil,
                    format!("« {} » n'a pas de version {what} sur cet ordinateur : elle est simulée, ce qui se voit à l'impression.", rc.font),
                    pos,
                    run.len,
                    page,
                );
            }
        }
    }
}

/// Format et marges de chaque section comparés aux réglages du livre.
fn margins(doc: &Document, layout: &DocLayout, r: &mut Report) {
    let Some(s) = settings(doc) else { return };
    let mut start = 0;
    for (n, (end, sp)) in doc.sections().into_iter().enumerate() {
        let near = |a: f32, b: f32| (a - b).abs() <= 0.5;
        let page = page_of(layout, &Path::top(start));
        let (w, h) = (to_mm(sp.page_w), to_mm(sp.page_h));
        if !near(w, s.page_w_mm) || !near(h, s.page_h_mm) {
            r.push(
                "format",
                Level::Attention,
                format!("Section {} : page de {w:.0} × {h:.0} mm au lieu du format du livre ({:.0} × {:.0} mm).", n + 1, s.page_w_mm, s.page_h_mm),
                Pos::body(start, 0),
                0,
                page,
            );
        }
        let pairs = [
            ("intérieure", sp.margin_left, s.inner_mm),
            ("extérieure", sp.margin_right, s.outer_mm),
            ("haute", sp.margin_top, s.top_mm),
            ("basse", sp.margin_bottom, s.bottom_mm),
        ];
        let off: Vec<String> = pairs
            .iter()
            .filter(|(_, got, want)| !near(to_mm(*got), *want))
            .map(|(name, got, want)| format!("{name} {:.1} mm au lieu de {want:.1} mm", to_mm(*got)))
            .collect();
        if !off.is_empty() && !r.full() {
            let mut i = Issue {
                kind: "marges".into(),
                level: Level::Attention,
                message: format!("Section {} : marges différentes des réglages du livre ({}).", n + 1, off.join(", ")),
                pos: Pos::body(start, 0),
                len: 0,
                page,
                fix: None,
                was: String::new(),
                command: Some("book.settings".into()),
            };
            if sp.gutter > 0.0 {
                i.message.push_str(&format!(" Reliure de {:.1} mm en plus.", to_mm(sp.gutter)));
            }
            r.issues.push(i);
        }
        start = end + 1;
    }
}

/// Le sommaire, s'il y en a un, comparé à ce qu'une mise à jour écrirait.
fn toc(s: &Session, r: &mut Report) {
    let Some((start, end)) = crate::cmd::references::toc_range(s) else { return };
    let texts = |doc: &Document, a: usize, b: usize| -> Vec<String> {
        (a..=b).filter_map(|i| doc.body.get(i)).filter_map(|b| b.as_para()).map(|p| visible(p).trim().to_string()).filter(|t| !t.is_empty()).collect()
    };
    let now = texts(&s.doc, start, end);
    let mut fresh = Session::new(s.doc.clone());
    if crate::cmd::references::update_toc(&mut fresh).is_err() {
        return;
    }
    let Some((a, b)) = crate::cmd::references::toc_range(&fresh) else { return };
    if texts(&fresh.doc, a, b) != now {
        r.issues.push(Issue {
            kind: "sommaire".into(),
            level: Level::Attention,
            message: "Le sommaire n'est plus à jour (titres ou numéros de page changés).".into(),
            pos: Pos::body(start, 0),
            len: 0,
            page: None,
            fix: None,
            was: String::new(),
            command: Some("references.updateToc".into()),
        });
    }
}

/// Hiérarchie des titres : titres vides, niveaux sautés, sous-chapitre avant tout chapitre,
/// titres anormalement longs, paragraphes qui ressemblent à un titre de chapitre sans en avoir
/// le style.
fn headings(doc: &Document, layout: &DocLayout, r: &mut Report) {
    let mut prev: Option<u8> = None;
    let entries = structure(doc);
    let first_chapter = entries.iter().find(|e| e.rank == 1).map(|e| e.block);
    for (i, b) in doc.body.iter().enumerate() {
        if r.full() {
            return;
        }
        let Block::Para(p) = &**b else { continue };
        let text = visible(p);
        let page = page_of(layout, &Path::top(i));
        let Some(level) = outline(doc, p) else {
            let t = text.trim();
            let lower = t.to_lowercase();
            if t.chars().count() <= 40
                && (lower.starts_with("chapitre ") || lower.starts_with("partie ") || lower.starts_with("épilogue") || lower.starts_with("prologue"))
                && !t.ends_with('.')
            {
                r.push(
                    "titres",
                    Level::Conseil,
                    format!(
                        "« {} » ressemble à un titre sans en avoir le style : il manquera au sommaire, à la navigation et à l'EPUB.",
                        short(t, 40)
                    ),
                    Pos::body(i, 0),
                    p.text.len(),
                    page,
                );
            }
            continue;
        };
        if text.trim().is_empty() {
            r.push("titres", Level::Erreur, format!("Titre de niveau {} vide.", level + 1), Pos::body(i, 0), 0, page);
            continue;
        }
        if text.chars().count() > 150 {
            r.push(
                "titres",
                Level::Conseil,
                format!("Titre de {} caractères : est-ce bien un titre ?", text.chars().count()),
                Pos::body(i, 0),
                p.text.len(),
                page,
            );
        }
        if let Some(pl) = prev
            && level > pl + 1
        {
            r.push(
                "titres",
                Level::Attention,
                format!("Niveau de titre sauté : titre {} directement après un titre {}.", level + 1, pl + 1),
                Pos::body(i, 0),
                p.text.len(),
                page,
            );
        } else if prev.is_none() && level >= 1 && first_chapter.is_some_and(|c| i < c) {
            r.push("titres", Level::Attention, "Sous-titre avant le premier chapitre.".to_string(), Pos::body(i, 0), p.text.len(), page);
        }
        prev = Some(level);
    }
}

/// Paragraphes vides à la suite (pour faire de l'espace ou changer de page).
fn empty_runs(doc: &Document, layout: &DocLayout, r: &mut Report) {
    let mut i = 0;
    let n = doc.body.len();
    while i < n {
        let empty = |j: usize| {
            doc.body
                .get(j)
                .is_some_and(|b| matches!(&**b, Block::Para(p) if p.objects.is_empty() && visible(p).trim().is_empty() && p.section.is_none()))
        };
        if !empty(i) {
            i += 1;
            continue;
        }
        let start = i;
        while i < n && empty(i) {
            i += 1;
        }
        let count = i - start;
        if count >= 3 {
            r.push(
                "paragraphesVides",
                Level::Conseil,
                format!("{count} paragraphes vides à la suite : un saut de page ou l'espacement du paragraphe tient mieux quand le texte change."),
                Pos::body(start, 0),
                0,
                page_of(layout, &Path::top(start)),
            );
        }
    }
}

/// Doubles espaces et typographie française, avec la correction proposée.
fn typography(doc: &Document, layout: &DocLayout, r: &mut Report) {
    const NBSP: &str = "\u{a0}";
    const THIN: &str = "\u{202f}";
    for path in doc.para_paths(StoryRef::Body) {
        if r.full() {
            return;
        }
        let Some(p) = doc.para(StoryRef::Body, &path) else { continue };
        let text = p.text.as_str();
        if text.is_empty() {
            continue;
        }
        let fr = french(doc, p);
        let page = page_of(layout, &path);
        let at = |off: usize| Pos { story: StoryRef::Body, path: path.clone(), off };
        let chars: Vec<(usize, char)> = text.char_indices().collect();
        let in_url = |off: usize| {
            let from = text.get(..off).and_then(|t| t.rfind(char::is_whitespace)).map_or(0, |w| w + 1);
            let word = text.get(from..).unwrap_or_default().split(char::is_whitespace).next().unwrap_or_default();
            word.contains("://") || word.starts_with("www.") || word.contains('@')
        };
        let mut k = 0;
        while k < chars.len() {
            if r.full() {
                return;
            }
            let (off, c) = chars[k];
            let prev = k.checked_sub(1).and_then(|j| chars.get(j)).map(|x| x.1);
            let next = chars.get(k + 1).map(|x| x.1);
            // Espaces multiples entre deux mots.
            if c == ' ' && next == Some(' ') && prev.is_some_and(|p| !p.is_whitespace()) {
                let mut e = k;
                while chars.get(e + 1).is_some_and(|x| x.1 == ' ') {
                    e += 1;
                }
                let end = chars.get(e + 1).map_or(text.len(), |x| x.0);
                if chars.get(e + 1).is_some() {
                    let was = text.get(off..end).unwrap_or_default();
                    r.push_fix("doubleEspace", Level::Conseil, format!("{} espaces à la suite.", e - k + 1), at(off), was, " ", page);
                }
                k = e + 1;
                continue;
            }
            if !fr {
                k += 1;
                continue;
            }
            match c {
                // Espace ordinaire avant ; ! ? : » (sécable : la ponctuation peut partir seule à la ligne).
                ' ' if matches!(next, Some(';' | '!' | '?' | ':' | '»')) && prev.is_some_and(|p| !p.is_whitespace()) => {
                    let punct = next.unwrap_or(' ');
                    let (fix, name) = if matches!(punct, ':' | '»') { (NBSP, "insécable") } else { (THIN, "fine insécable") };
                    r.push_fix(
                        "typographie",
                        Level::Conseil,
                        format!("Espace ordinaire avant « {punct} » : une espace {name} l'empêche de passer seule en début de ligne."),
                        at(off),
                        " ",
                        fix,
                        page,
                    );
                }
                // Ponctuation haute collée au mot.
                ';' | '!' | '?' if prev.is_some_and(char::is_alphanumeric) && !in_url(off) => {
                    r.push_fix("typographie", Level::Conseil, format!("Il manque une espace fine insécable avant « {c} »."), at(off), "", THIN, page);
                }
                ':' if prev.is_some_and(char::is_alphabetic) && next.is_none_or(char::is_whitespace) && !in_url(off) => {
                    r.push_fix("typographie", Level::Conseil, "Il manque une espace insécable avant « : ».".into(), at(off), "", NBSP, page);
                }
                // Guillemets français.
                '«' if next == Some(' ') => {
                    let o = off + c.len_utf8();
                    r.push_fix(
                        "typographie",
                        Level::Conseil,
                        "Espace ordinaire après « « » : une espace insécable la garde avec le mot.".into(),
                        at(o),
                        " ",
                        NBSP,
                        page,
                    );
                }
                '«' if next.is_some_and(char::is_alphanumeric) => {
                    let o = off + c.len_utf8();
                    r.push_fix("typographie", Level::Conseil, "Il manque une espace insécable après « « ».".into(), at(o), "", NBSP, page);
                }
                '»' if prev.is_some_and(|p| p.is_alphanumeric() || matches!(p, '.' | '!' | '?' | '…')) => {
                    r.push_fix("typographie", Level::Conseil, "Il manque une espace insécable avant « » ».".into(), at(off), "", NBSP, page);
                }
                // Apostrophe droite entre deux lettres.
                '\'' if prev.is_some_and(char::is_alphabetic) && next.is_some_and(char::is_alphabetic) => {
                    r.push_fix(
                        "typographie",
                        Level::Conseil,
                        "Apostrophe droite : l'apostrophe typographique est « ’ ».".into(),
                        at(off),
                        "'",
                        "’",
                        page,
                    );
                }
                // Guillemets droits.
                '"' if !in_url(off) => {
                    let opening = prev.is_none_or(|p| p.is_whitespace() || matches!(p, '(' | '[' | '—' | '–'));
                    let (was_len, fix) = if opening {
                        let skip = usize::from(next == Some(' '));
                        (1 + skip, format!("«{NBSP}"))
                    } else {
                        (1, format!("{NBSP}»"))
                    };
                    let (from, was) = if opening {
                        (off, text.get(off..off + was_len).unwrap_or("\""))
                    } else if prev == Some(' ') {
                        (off - 1, text.get(off - 1..off + 1).unwrap_or("\""))
                    } else {
                        (off, "\"")
                    };
                    r.push_fix("typographie", Level::Conseil, "Guillemets droits : en français, « guillemets ».".into(), at(from), was, &fix, page);
                }
                // Trois points.
                '.' if next == Some('.') && chars.get(k + 2).is_some_and(|x| x.1 == '.') && prev != Some('.') => {
                    if chars.get(k + 3).is_none_or(|x| x.1 != '.') {
                        r.push_fix(
                            "typographie",
                            Level::Conseil,
                            "Trois points : le caractère « … » ne se coupe pas en fin de ligne.".into(),
                            at(off),
                            "...",
                            "…",
                            page,
                        );
                        k += 3;
                        continue;
                    }
                }
                // Espace avant une virgule ou un point.
                ' ' if matches!(next, Some(',' | '.')) && prev.is_some_and(char::is_alphanumeric) && chars.get(k + 2).is_none_or(|x| x.1 != '.') => {
                    r.push_fix(
                        "typographie",
                        Level::Conseil,
                        format!("Espace avant « {} » : à supprimer.", next.unwrap_or(',')),
                        at(off),
                        " ",
                        "",
                        page,
                    );
                }
                _ => {}
            }
            k += 1;
        }
    }
}

/// Analyse complète du document actif (un livre ou n'importe quel document : les contrôles liés
/// aux réglages du livre sont alors sautés). Ne modifie rien.
pub fn check(s: &mut Session) -> Vec<Issue> {
    let layout = s.layout();
    let mut r = Report { issues: Vec::new() };
    let doc = &s.doc;
    blank_pages(doc, &layout, &mut r);
    lines_and_headings(doc, &layout, &mut r);
    headings(doc, &layout, &mut r);
    pictures(doc, &layout, &mut r);
    fonts(doc, &layout, &mut r);
    margins(doc, &layout, &mut r);
    empty_runs(doc, &layout, &mut r);
    toc(s, &mut r);
    typography(&s.doc, &layout, &mut r);
    let mut issues = r.issues;
    issues.truncate(MAX_ISSUES);
    // Dans l'ordre du texte, les plus graves d'abord à position égale.
    issues.sort_by(|a, b| a.pos.path.cmp(&b.pos.path).then(a.pos.off.cmp(&b.pos.off)).then(a.level.cmp(&b.level)));
    issues
}

/// Applique les corrections choisies par l'utilisateur, de la fin du texte vers le début pour que
/// les positions restent justes. Une correction dont le texte a changé depuis l'analyse est
/// laissée de côté. Rend le nombre de corrections faites.
pub fn apply_fixes(doc: &mut Document, fixes: &[Fix]) -> usize {
    let mut list: Vec<&Fix> = fixes.iter().filter(|f| f.fix.chars().count() <= 8 && f.len <= 64).collect();
    list.sort_by(|a, b| b.pos.story.cmp(&a.pos.story).then(b.pos.path.cmp(&a.pos.path)).then(b.pos.off.cmp(&a.pos.off)));
    list.dedup_by(|a, b| a.pos == b.pos);
    let mut done = 0;
    for f in list {
        let Some(p) = doc.para(f.pos.story, &f.pos.path) else { continue };
        let Some(end) = f.pos.off.checked_add(f.len) else { continue };
        if p.text.get(f.pos.off..end) != Some(f.was.as_str()) {
            continue;
        }
        let props = p.props_at(f.pos.off).clone();
        let b = Pos { off: end, ..f.pos.clone() };
        if f.len > 0 && doc.delete_range(&f.pos, &b).is_err() {
            continue;
        }
        if !f.fix.is_empty() && doc.insert_text(&f.pos, &f.fix, &props).is_err() {
            continue;
        }
        done += 1;
    }
    done
}

#[cfg(test)]
mod tests;
