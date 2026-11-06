use serde_json::json;
use wordcraft_doc::{Document, Pos};

use super::super::{BookSettings, ElementKind, insert, new_document};
use super::*;

fn french() -> Document {
    let mut d = Document::new();
    d.styles.default_chr.lang = Some("fr-FR".into());
    d
}

fn session_with(text: &str) -> Session {
    let mut s = Session::new(french());
    s.run("text.insert", &json!({ "text": text })).unwrap();
    s
}

fn kinds(issues: &[Issue]) -> Vec<&str> {
    issues.iter().map(|i| i.kind.as_str()).collect()
}

fn fixes(issues: &[Issue]) -> Vec<Fix> {
    issues.iter().filter_map(|i| i.fix.as_ref().map(|f| Fix { pos: i.pos.clone(), len: i.len, was: i.was.clone(), fix: f.clone() })).collect()
}

#[test]
fn french_typography_is_found_and_fixed_only_on_request() {
    let text = "Il dit : « Partons ! » Puis  il part ; bon , voilà... C'est \"fini\" ! Vrai?";
    let mut s = session_with(text);
    let issues = check(&mut s);
    // L'analyse ne change rien.
    assert_eq!(s.doc.plain_text(StoryRef::Body).trim_end(), text);
    assert!(kinds(&issues).contains(&"doubleEspace"), "{issues:#?}");
    assert!(issues.iter().filter(|i| i.kind == "typographie").count() >= 8, "{issues:#?}");
    let n = apply_fixes(&mut s.doc, &fixes(&issues));
    assert!(n >= 9, "{n}");
    let after = s.doc.plain_text(StoryRef::Body);
    assert_eq!(
        after.trim_end(),
        "Il dit\u{a0}: «\u{a0}Partons\u{202f}!\u{a0}» Puis il part\u{202f}; bon, voilà… C’est «\u{a0}fini\u{a0}»\u{202f}! Vrai\u{202f}?"
    );
    // Une seconde analyse ne trouve plus rien à corriger dans ce paragraphe.
    let again = check(&mut s);
    assert!(again.iter().all(|i| i.fix.is_none()), "{again:#?}");
}

#[test]
fn urls_times_and_english_text_are_left_alone() {
    let mut s = session_with("Rendez-vous à 10:30 sur https://lusk.bzh/?a=1 ou www.lusk.bzh!");
    let issues = check(&mut s);
    assert!(issues.iter().all(|i| i.kind != "typographie"), "{issues:#?}");
    let mut en = Session::new(Document::new());
    en.run("text.insert", &json!({"text": "He said: \"Go!\" It's fine."})).unwrap();
    let issues = check(&mut en);
    assert!(issues.iter().all(|i| i.kind != "typographie"), "{issues:#?}");
}

#[test]
fn a_fix_on_text_that_changed_is_skipped() {
    let mut s = session_with("Bonjour  toi");
    let issues = check(&mut s);
    let f = fixes(&issues);
    assert_eq!(f.len(), 1);
    s.run("text.insert", &json!({"text": "!"})).unwrap();
    let mut stale = f.clone();
    stale[0].was = "xx".into();
    assert_eq!(apply_fixes(&mut s.doc, &stale), 0);
    assert_eq!(apply_fixes(&mut s.doc, &f), 1);
    assert!(s.doc.plain_text(StoryRef::Body).starts_with("Bonjour toi"));
}

#[test]
fn headings_out_of_order_and_empty_are_reported() {
    let mut s = Session::new(french());
    s.run("text.insert", &json!({"text": "Grand titre"})).unwrap();
    s.run("para.style", &json!({"style": "Heading1"})).unwrap();
    s.run("text.newParagraph", &json!({})).unwrap();
    s.run("text.insert", &json!({"text": "Trop bas"})).unwrap();
    s.run("para.style", &json!({"style": "Heading3"})).unwrap();
    s.run("text.newParagraph", &json!({})).unwrap();
    s.run("para.style", &json!({"style": "Heading2"})).unwrap();
    s.run("text.newParagraph", &json!({})).unwrap();
    s.run("para.style", &json!({"style": "Normal"})).unwrap();
    s.run("text.insert", &json!({"text": "Chapitre deux"})).unwrap();
    let issues = check(&mut s);
    let titres: Vec<&Issue> = issues.iter().filter(|i| i.kind == "titres").collect();
    assert!(titres.iter().any(|i| i.message.contains("sauté")), "{titres:#?}");
    assert!(titres.iter().any(|i| i.level == Level::Erreur && i.message.contains("vide")), "{titres:#?}");
    assert!(titres.iter().any(|i| i.message.contains("ressemble à un titre")), "{titres:#?}");
}

#[test]
fn a_low_resolution_picture_and_its_missing_alt_text_are_reported() {
    let mut s = Session::new(french());
    let mut png = Vec::new();
    let img = image::RgbaImage::from_pixel(40, 20, image::Rgba([200, 30, 30, 255]));
    image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
    s.run("insert.picture", &json!({"data": crate::cmd::insert::base64_encode(&png), "width": 160})).unwrap();
    let issues = check(&mut s);
    let res = issues.iter().find(|i| i.kind == "resolution").expect("resolution");
    assert_eq!(res.level, Level::Erreur);
    assert!(res.message.contains("18 ppp"), "{}", res.message);
    assert!(issues.iter().any(|i| i.kind == "texteAlt"));
    assert_eq!(res.page, Some(1));
}

#[test]
fn blank_pages_and_runs_of_empty_paragraphs_are_reported() {
    let mut s = session_with("Début");
    for _ in 0..4 {
        s.run("text.newParagraph", &json!({})).unwrap();
    }
    s.run("text.insert", &json!({"text": "Suite"})).unwrap();
    s.run("insert.pageBreak", &json!({})).unwrap();
    s.run("insert.pageBreak", &json!({})).unwrap();
    s.run("text.insert", &json!({"text": "Fin"})).unwrap();
    let issues = check(&mut s);
    assert!(issues.iter().any(|i| i.kind == "paragraphesVides" && i.message.starts_with("3 ")), "{issues:#?}");
    assert!(issues.iter().any(|i| i.kind == "pageBlanche" && i.page == Some(2)), "{issues:#?}");
    assert!(issues.iter().all(|i| i.kind != "veuve" && i.kind != "orpheline"), "page breaks are not widows: {issues:#?}");
}

#[test]
fn a_book_without_problems_has_no_warnings_and_its_margins_are_checked() {
    let s0 = BookSettings { title: "Les Marées".into(), author: "Jeanne Martin".into(), ..BookSettings::default() };
    let mut doc = new_document(french(), &s0);
    insert(&mut doc, ElementKind::Chapter, "La traversée", None, None).unwrap();
    let mut s = Session::new(doc);
    let issues = check(&mut s);
    assert!(issues.iter().all(|i| i.level == Level::Conseil), "{issues:#?}");
    // Une section aux marges changées à la main.
    let last = s.doc.body.len() - 1;
    if let Ok(p) = s.doc.para_mut(StoryRef::Body, &Path::top(last)) {
        let mut sp = p.section.clone().unwrap_or_default();
        sp.margin_left = 20.0;
        p.section = Some(sp);
    }
    let issues = check(&mut s);
    let m = issues.iter().find(|i| i.kind == "marges").expect("marges");
    assert!(m.message.contains("intérieure 7.1 mm"), "{}", m.message);
    assert_eq!(m.command.as_deref(), Some("book.settings"));
}

#[test]
fn an_outdated_table_of_contents_is_reported() {
    let mut s = Session::new(french());
    s.run("text.insert", &json!({"text": "Premier"})).unwrap();
    s.run("para.style", &json!({"style": "Heading1"})).unwrap();
    s.run("text.newParagraph", &json!({})).unwrap();
    s.sel = crate::Selection::caret(Pos::body(0, 0));
    s.run("references.toc", &json!({})).unwrap();
    assert!(check(&mut s).iter().all(|i| i.kind != "sommaire"));
    let h = s.doc.body.iter().rposition(|b| b.as_para().is_some_and(|p| p.plain_text() == "Premier")).unwrap();
    s.sel = crate::Selection::caret(s.doc.clamp(&Pos::body(h, 7)));
    s.run("text.insert", &json!({"text": " chapitre"})).unwrap();
    let issues = check(&mut s);
    let t = issues.iter().find(|i| i.kind == "sommaire").expect("sommaire");
    assert_eq!(t.command.as_deref(), Some("references.updateToc"));
}

#[test]
fn junk_fixes_never_panic() {
    let mut s = session_with("abc");
    let junk = vec![
        Fix { pos: Pos::body(99, 0), len: 1, was: "a".into(), fix: "b".into() },
        Fix { pos: Pos::body(0, 2), len: usize::MAX, was: "c".into(), fix: "d".into() },
        Fix { pos: Pos::body(0, 1), len: 1, was: "é".into(), fix: "x".into() },
    ];
    assert_eq!(apply_fixes(&mut s.doc, &junk), 0);
}
