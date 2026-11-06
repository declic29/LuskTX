//! Mode Livre commands (Lusk Texte): a book's settings and its elements — title page, dedication,
//! parts, chapters… — inserted, renamed, moved and deleted as whole ranges of blocks. The model is
//! [`crate::book`]; each command is one undo step.

use serde_json::{Value, json};
use wordcraft_doc::Pos;

use super::sel_result;
use crate::book::{self, BookSettings, ElementKind, Place};
use crate::{CmdError, CmdResult, CommandSpec, Selection, Session, p};

fn in_book(s: &Session) -> Option<&'static str> {
    (!book::is_book(&s.doc)).then_some("seulement dans un livre (Mode Livre)")
}

fn not_book(s: &Session) -> Option<&'static str> {
    book::is_book(&s.doc).then_some("ce document est déjà un livre")
}

/// The settings in `v["settings"]` laid over `base` (fields left out keep their value).
pub(crate) fn settings_from(v: &Value, base: BookSettings) -> Result<BookSettings, CmdError> {
    let Some(given) = v.get("settings") else { return Ok(base) };
    if !given.is_object() {
        return Err(CmdError::Params("`settings` must be an object".into()));
    }
    let mut merged = serde_json::to_value(&base).map_err(|e| CmdError::Failed(e.to_string()))?;
    if let (Some(m), Some(g)) = (merged.as_object_mut(), given.as_object()) {
        for (k, val) in g {
            m.insert(k.clone(), val.clone());
        }
    }
    // A preset chosen by id fills in its own values first, then the explicit ones win.
    let preset = merged.get("profile").and_then(Value::as_str).and_then(book::preset).filter(|_| given.get("profile").is_some());
    let merged = match preset {
        Some(pr) => {
            let mut v2 = serde_json::to_value(pr.settings()).map_err(|e| CmdError::Failed(e.to_string()))?;
            if let (Some(m), Some(b)) = (v2.as_object_mut(), serde_json::to_value(&base).ok().as_ref().and_then(Value::as_object)) {
                for key in ["title", "subtitle", "author", "cover"] {
                    if let Some(x) = b.get(key) {
                        m.insert(key.into(), x.clone());
                    }
                }
            }
            if let (Some(m), Some(g)) = (v2.as_object_mut(), given.as_object()) {
                for (k, val) in g {
                    m.insert(k.clone(), val.clone());
                }
            }
            v2
        }
        None => merged,
    };
    serde_json::from_value::<BookSettings>(merged).map(BookSettings::sanitized).map_err(|e| CmdError::Params(format!("`settings`: {e}")))
}

/// The versions folder given by the app (never empty, never relative: no file lands in the
/// working directory by accident).
fn versions_dir(v: &Value) -> Result<std::path::PathBuf, CmdError> {
    let d = p::req_str(v, "dir")?;
    let path = std::path::PathBuf::from(d);
    if d.trim().is_empty() || !path.is_absolute() {
        return Err(CmdError::Params("`dir` must be an absolute folder".into()));
    }
    Ok(path)
}

fn index(v: &Value, k: &str) -> Result<usize, CmdError> {
    p::u64(v, k).map(|n| n.min(usize::MAX as u64) as usize).ok_or_else(|| CmdError::Params(format!("`{k}` (element index) is required")))
}

fn caret_at(s: &mut Session, block: usize) {
    let block = block.min(s.doc.body.len().saturating_sub(1));
    s.sel = Selection::caret(s.doc.clamp(&Pos::body(block, 0)));
}

fn err(e: book::BookError) -> CmdError {
    match e {
        book::BookError::Invalid(m) => CmdError::Params(m),
        other => CmdError::Failed(other.to_string()),
    }
}

/// `book.info`: whether this is a book, its settings, elements and word counts.
fn info(s: &mut Session, _: &Value) -> CmdResult {
    let settings = book::settings(&s.doc);
    let elements: Vec<Value> = book::structure(&s.doc)
        .into_iter()
        .map(|e| json!({"kind": e.kind.id(), "label": e.label, "title": e.title, "block": e.block, "end": e.end, "rank": e.rank, "words": e.words}))
        .collect();
    Ok(json!({"isBook": settings.is_some(), "settings": settings, "elements": elements, "totalWords": book::total_words(&s.doc)}))
}

const SETTINGS: &str = r#"{"settings": {"title"?, "subtitle"?, "author"?, "kind"?: "roman"|"recit"|"biographie"|"essai"|"poesie"|"illustre"|"autre", "format"?: "a5"|"poche"|"6x9"|"a4"|"a6"|"custom", "pageWMm"?, "pageHMm"?, "paper"?: "blanc"|"creme", "font"?, "size"?, "lineSpacing"?, "align"?: "justify"|"left", "innerMm"?, "outerMm"?, "topMm"?, "bottomMm"?, "indentMm"?, "cover"?, "profile"?, "runningHeads"?: "auteurTitre"|"titreChapitre"|"aucun"|"manuel", "folio"?: "basCentre"|"basExterieur"|"hautExterieur"|"aucun", "dropCaps"?: bool}}"#;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("book.info", "Book Information", "Livre", info).pure(),
        // Phase 5: quality control. The analysis changes nothing; fixes run only when asked for.
        CommandSpec::new("book.check", "Quality Check", "Livre", |s, _| {
            let issues = book::quality::check(s);
            let count = |l: book::quality::Level| issues.iter().filter(|i| i.level == l).count();
            Ok(json!({
                "erreurs": count(book::quality::Level::Erreur),
                "attention": count(book::quality::Level::Attention),
                "conseils": count(book::quality::Level::Conseil),
                "issues": issues,
            }))
        })
        .pure(),
        CommandSpec::new("book.fix", "Apply Quality Fixes", "Livre", |s, v| {
            let Some(list) = v.get("fixes").and_then(Value::as_array) else {
                return Err(CmdError::Params("`fixes` (array of {pos, len, was, fix}) is required".into()));
            };
            let fixes: Vec<book::quality::Fix> = list.iter().take(10_000).filter_map(|f| serde_json::from_value(f.clone()).ok()).collect();
            let fixed = book::quality::apply_fixes(&mut s.doc, &fixes);
            let sel = s.sel.clone();
            s.sel = Selection { anchor: s.doc.clamp(&sel.anchor), focus: s.doc.clamp(&sel.focus) };
            Ok(json!({"fixed": fixed}))
        })
        .params(r#"{"fixes": [{"pos": position, "len": bytes, "was": text, "fix": text}]} (the corrections book.check proposed; one whose text changed since is skipped)"#),
        // Phase 5: version history, on this computer only (`dir`: the app's versions folder).
        CommandSpec::new("book.versionSave", "Save Version", "Livre", |s, v| {
            let dir = versions_dir(v)?;
            let label = p::str(v, "label").unwrap_or_default();
            let auto = p::bool(v, "auto").unwrap_or(false);
            let ver = book::versions::save(&dir, &s.doc, label, auto).map_err(CmdError::Failed)?;
            Ok(json!(ver))
        })
        .params(r#"{"dir": versions folder, "label"?: string, "auto"?: bool}"#)
        .when(in_book)
        .pure(),
        CommandSpec::new("book.versionList", "Versions", "Livre", |s, v| {
            let dir = versions_dir(v)?;
            Ok(json!({"versions": book::versions::list(&dir, &s.doc)}))
        })
        .params(r#"{"dir": versions folder}"#)
        .when(in_book)
        .pure(),
        CommandSpec::new("book.versionOpen", "Open Version", "Livre", |s, v| {
            let dir = versions_dir(v)?;
            let id = p::req_str(v, "id")?;
            let list = book::versions::list(&dir, &s.doc);
            let ver = list.iter().find(|x| x.id == id).ok_or_else(|| CmdError::Failed("version introuvable".into()))?;
            let mut doc = book::versions::load(&dir, &s.doc, id).map_err(CmdError::Failed)?;
            let date = ver.date.get(..16).unwrap_or(&ver.date).replace('T', " à ");
            doc.core.title = format!("{} — {} ({date})", if ver.title.is_empty() { "Livre" } else { &ver.title }, ver.label);
            s.add_document(doc);
            s.view.book_pane = true;
            sel_result(s)
        })
        .params(r#"{"dir": versions folder, "id": version id} (opens the version as a new document; the book stays as it is)"#)
        .when(in_book)
        .pure(),
        CommandSpec::new("book.versionDelete", "Delete Version", "Livre", |s, v| {
            let dir = versions_dir(v)?;
            let id = p::req_str(v, "id")?;
            book::versions::delete(&dir, &s.doc, id).map_err(CmdError::Failed)?;
            Ok(json!({"deleted": id}))
        })
        .params(r#"{"dir": versions folder, "id": version id}"#)
        .when(in_book)
        .pure(),
        // Mode Document | Mode Livre: the book pane beside the document.
        CommandSpec::new("view.bookPane", "Book Pane", "Livre", |s, v| {
            s.view.book_pane = p::bool(v, "value").unwrap_or(!s.view.book_pane);
            Ok(json!({"value": s.view.book_pane}))
        })
        .params(r#"{"value"?: bool}"#)
        .pure(),
        // Mode Livre's « livre ouvert » view: double pages, the first page alone on the right.
        CommandSpec::new("view.bookSpreads", "Open Book View", "Livre", |s, v| {
            let on = p::bool(v, "value").unwrap_or(!s.view.book_spreads);
            s.view.book_spreads = on;
            if on {
                if s.view.mode != wordcraft_layout::ViewMode::Print {
                    s.view.mode = wordcraft_layout::ViewMode::Print;
                    s.relayout();
                }
                s.view.fit = "multiplePages".into();
                s.view.single_column = false;
            } else if s.view.fit == "multiplePages" {
                s.view.fit.clear();
            }
            Ok(json!({"value": on}))
        })
        .params(r#"{"value"?: bool}"#)
        .pure(),
        CommandSpec::new("book.convert", "Make This Document a Book", "Livre", |s, v| {
            let mut base = BookSettings { title: s.doc.core.title.clone(), ..BookSettings::default() };
            if !s.doc.core.creator.is_empty() && s.doc.core.creator != "Lusk Texte" {
                base.author = s.doc.core.creator.clone();
            }
            // A document with headers or footers of its own keeps them.
            if s.doc.parts.values().any(|p| matches!(p.kind, wordcraft_doc::PartKind::Header | wordcraft_doc::PartKind::Footer)) {
                base.running_heads = "manuel".into();
            }
            let settings = settings_from(v, base)?;
            book::convert(&mut s.doc, &settings);
            s.view.book_pane = true;
            caret_at(s, 0);
            info(s, v)
        })
        .params(SETTINGS)
        .when(not_book),
        CommandSpec::new("book.settings", "Book Settings", "Livre", |s, v| {
            let base = book::settings(&s.doc).unwrap_or_default();
            let settings = settings_from(v, base)?;
            book::apply_settings(&mut s.doc, &settings);
            book::ensure_styles(&mut s.doc);
            // Running heads, folios and drop caps follow the new settings.
            book::normalize(&mut s.doc);
            let sel = s.sel.clone();
            s.sel = Selection { anchor: s.doc.clamp(&sel.anchor), focus: s.doc.clamp(&sel.focus) };
            info(s, v)
        })
        .params(SETTINGS)
        .when(in_book),
        CommandSpec::new("book.insert", "Insert Book Element", "Livre", |s, v| {
            let kind_id = p::req_str(v, "kind")?;
            let kind = ElementKind::from_id(kind_id).ok_or_else(|| {
                CmdError::Params(format!("unknown `kind` {kind_id:?}: {}", ElementKind::ALL.map(ElementKind::id).join(", ")))
            })?;
            let title = p::str(v, "title").unwrap_or("");
            let after = match v.get("after") {
                None | Some(Value::Null) => None,
                Some(_) => Some(index(v, "after")?),
            };
            let near = (s.sel.focus.story == wordcraft_doc::StoryRef::Body).then(|| s.sel.focus.path.0.first().map(|b| *b as usize)).flatten();
            let block = book::insert(&mut s.doc, kind, title, after, near).map_err(err)?;
            // Ready to write: in the empty paragraph under a new chapter's (or preface's…) title.
            let body = !matches!(kind, ElementKind::Part | ElementKind::TitlePage | ElementKind::Dedication | ElementKind::Epigraph);
            caret_at(s, if body { block + 1 } else { block });
            sel_result(s)
        })
        .params(r#"{"kind": "titlePage"|"dedication"|"epigraph"|"preface"|"foreword"|"part"|"chapter"|"subchapter"|"acknowledgments"|"bibliography", "title"?: string, "after"?: element index}"#)
        .when(in_book),
        CommandSpec::new("book.rename", "Rename Book Element", "Livre", |s, v| {
            let i = index(v, "index")?;
            let title = p::req_str(v, "title")?;
            book::rename(&mut s.doc, i, title).map_err(err)?;
            let sel = s.sel.clone();
            s.sel = Selection { anchor: s.doc.clamp(&sel.anchor), focus: s.doc.clamp(&sel.focus) };
            sel_result(s)
        })
        .params(r#"{"index": element index, "title": string}"#)
        .when(in_book),
        CommandSpec::new("book.delete", "Delete Book Element", "Livre", |s, v| {
            let i = index(v, "index")?;
            let range = book::delete(&mut s.doc, i).map_err(err)?;
            caret_at(s, range.start.saturating_sub(usize::from(range.start >= s.doc.body.len())));
            sel_result(s)
        })
        .params(r#"{"index": element index}"#)
        .when(in_book),
        CommandSpec::new("book.move", "Move Book Element", "Livre", |s, v| {
            let from = index(v, "from")?;
            let to = index(v, "to")?;
            let place = match p::str(v, "place").unwrap_or("before") {
                "before" => Place::Before,
                "after" => Place::After,
                other => return Err(CmdError::Params(format!("`place` must be \"before\" or \"after\", not {other:?}"))),
            };
            let block = book::move_to(&mut s.doc, from, to, place).map_err(err)?;
            caret_at(s, block);
            sel_result(s)
        })
        .params(r#"{"from": element index, "to": element index, "place"?: "before"|"after"}"#)
        .when(in_book),
        CommandSpec::new("book.goto", "Go to Book Element", "Livre", |s, v| {
            let i = index(v, "index")?;
            let block = book::structure(&s.doc).get(i).map(|e| e.block).ok_or_else(|| CmdError::Failed(book::BookError::NoElement(i).to_string()))?;
            caret_at(s, block);
            sel_result(s)
        })
        .params(r#"{"index": element index}"#)
        .pure(),
        // Mode Livre › Publier: the book as PDF, for the printer (its bleed around every page,
        // the trim box, the fonts embedded) or for reading on a screen.
        CommandSpec::new("book.exportPdf", "Export Book PDF", "Livre", |s, v| {
            let path = p::req_str(v, "path")?;
            let path = if path.to_ascii_lowercase().ends_with(".pdf") { path.to_string() } else { format!("{path}.pdf") };
            let print = match p::str(v, "purpose").unwrap_or("print") {
                "print" => true,
                "reading" => false,
                other => return Err(CmdError::Params(format!("`purpose` must be \"print\" or \"reading\", not {other:?}"))),
            };
            let settings = book::settings(&s.doc).unwrap_or_default();
            let opts = wordcraft_pdf::PdfOptions {
                bleed: if print { book::mm(settings.bleed_mm) } else { 0.0 },
                page_labels: true,
                title: (!settings.title.is_empty()).then(|| settings.title.clone()),
                author: (!settings.author.is_empty()).then(|| settings.author.clone()),
                ..Default::default()
            };
            let bytes = wordcraft_pdf::export(&s.doc, &opts).map_err(|e| CmdError::Failed(e.to_string()))?;
            crate::io::write_file(&path, &bytes).map_err(CmdError::Failed)?;
            Ok(json!({"path": path, "purpose": if print { "print" } else { "reading" }, "bleedMm": if print { settings.bleed_mm } else { 0.0 }, "pages": s.layout().pages.len()}))
        })
        .params(r#"{"path": string, "purpose"?: "print" (the printer's bleed and trim box; default) | "reading"}"#)
        .when(in_book)
        .pure(),
        // Mode Livre › Publier: the book as EPUB 3 for e-readers (a copy: the book stays where
        // it is saved).
        CommandSpec::new("book.exportEpub", "Export EPUB", "Livre", |s, v| {
            let path = p::req_str(v, "path")?;
            let path = if path.to_ascii_lowercase().ends_with(".epub") { path.to_string() } else { format!("{path}.epub") };
            let bytes = book::epub::write(&s.doc).map_err(CmdError::Failed)?;
            crate::io::write_file(&path, &bytes).map_err(CmdError::Failed)?;
            Ok(json!({"path": path, "bytes": bytes.len()}))
        })
        .params(r#"{"path": string}"#)
        .pure(),
        // Studio Couverture: the full cover (back, spine, front, bleed) as a new document.
        CommandSpec::new("book.cover", "Book Cover", "Livre", |s, _| {
            let pages = s.layout().pages.len();
            let settings = book::settings(&s.doc).unwrap_or_default();
            let cover = book::cover_document(&s.doc, pages).map_err(CmdError::Failed)?;
            let spine = settings.spine_mm(pages);
            s.add_document(cover);
            Ok(json!({"pages": pages, "spineMm": spine, "bleedMm": settings.bleed_mm}))
        })
        .when(in_book),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_merge_keeps_unnamed_fields_and_applies_presets() {
        let base = BookSettings { title: "Le Titre".into(), author: "A. Auteur".into(), ..BookSettings::default() };
        let s = settings_from(&json!({"settings": {"size": 12}}), base.clone()).unwrap();
        assert_eq!(s.size, 12.0);
        assert_eq!(s.title, "Le Titre");
        let s = settings_from(&json!({"settings": {"profile": "essai"}}), base.clone()).unwrap();
        assert_eq!(s.format, "6x9");
        assert_eq!(s.author, "A. Auteur");
        let s = settings_from(&json!({"settings": {"profile": "essai", "size": 13}}), base).unwrap();
        assert_eq!((s.format.as_str(), s.size), ("6x9", 13.0));
        assert!(settings_from(&json!({"settings": 3}), BookSettings::default()).is_err());
    }
}
