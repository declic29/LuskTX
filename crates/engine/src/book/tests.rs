use super::*;
use wordcraft_doc::StoryRef;

fn book() -> Document {
    let s = BookSettings { title: "Les Marées".into(), author: "Jeanne Martin".into(), ..BookSettings::default() };
    new_document(Document::new(), &s)
}

fn kinds(doc: &Document) -> Vec<(ElementKind, String)> {
    structure(doc).into_iter().map(|e| (e.kind, e.title)).collect()
}

/// Body paragraphs' texts, in order.
fn texts(doc: &Document) -> Vec<String> {
    doc.body.iter().filter_map(|b| b.as_para().map(|p| p.text.clone())).collect()
}

fn type_into(doc: &mut Document, element: usize, text: &str) {
    let e = structure(doc).get(element).cloned().unwrap();
    // The element's first body paragraph after its heading.
    let at = e.block + 1;
    let p = styled(text, "Normal");
    doc.body.insert(at, para_block(p));
}

#[test]
fn a_new_book_has_its_settings_title_page_and_first_chapter() {
    let doc = book();
    let s = settings(&doc).unwrap();
    assert_eq!(s.title, "Les Marées");
    assert_eq!(s.format, "a5");
    assert_eq!(kinds(&doc), vec![(ElementKind::TitlePage, "Les Marées".to_string()), (ElementKind::Chapter, "Chapitre 1".to_string())]);
    // The author follows the title on the title page.
    assert!(texts(&doc).contains(&"Jeanne Martin".to_string()));
    // A5, mirrored margins, French hyphenation.
    assert!((doc.last_section.page_w - mm(148.0)).abs() < 0.01);
    assert!((doc.last_section.margin_left - mm(20.0)).abs() < 0.01);
    assert!(doc.settings.mirror_margins && doc.settings.auto_hyphenation);
    // The chapter starts a section on a right-hand page, numbered from 1; the title page is roman.
    let sections = doc.sections();
    assert_eq!(sections.len(), 2);
    let (_, front) = sections[0];
    let (_, body) = sections[1];
    assert_eq!(front.page_num_format, NumFormat::LowerRoman);
    assert_eq!(body.start, SectionStart::OddPage);
    assert_eq!(body.page_num_format, NumFormat::Decimal);
    assert_eq!(body.page_num_start, Some(1));
}

#[test]
fn elements_go_where_a_book_puts_them() {
    let mut doc = book();
    insert(&mut doc, ElementKind::Bibliography, "", None, None).unwrap();
    insert(&mut doc, ElementKind::Chapter, "", None, None).unwrap();
    insert(&mut doc, ElementKind::Dedication, "À ma mère", None, None).unwrap();
    insert(&mut doc, ElementKind::Preface, "", None, None).unwrap();
    insert(&mut doc, ElementKind::Acknowledgments, "", None, None).unwrap();
    let chapter1 = structure(&doc).iter().find(|e| e.title == "Chapitre 1").map(|e| e.block).unwrap();
    insert(&mut doc, ElementKind::Subchapter, "Le port", None, Some(chapter1)).unwrap();
    let order: Vec<ElementKind> = structure(&doc).iter().map(|e| e.kind).collect();
    assert_eq!(
        order,
        vec![
            ElementKind::TitlePage,
            ElementKind::Dedication,
            ElementKind::Preface,
            ElementKind::Chapter,
            ElementKind::Subchapter,
            ElementKind::Chapter,
            ElementKind::Acknowledgments,
            ElementKind::Bibliography,
        ]
    );
    let titles: Vec<String> = structure(&doc).iter().map(|e| e.title.clone()).collect();
    assert_eq!(titles[3], "Chapitre 1");
    assert_eq!(titles[5], "Chapitre 2");
    // One section per element of rank 0 or 1; the sub-chapter stays in its chapter's.
    assert_eq!(doc.sections().len(), 7);
    // Front matter roman, body arabic from the first chapter.
    let entries = structure(&doc);
    let format_at = |block: usize| doc.sections()[doc.section_index_of(block)].1.page_num_format;
    assert_eq!(format_at(entries[2].block), NumFormat::LowerRoman);
    assert_eq!(format_at(entries[3].block), NumFormat::Decimal);
}

#[test]
fn moving_a_chapter_moves_its_text_and_loses_nothing() {
    let mut doc = book();
    insert(&mut doc, ElementKind::Chapter, "", None, None).unwrap();
    insert(&mut doc, ElementKind::Chapter, "", None, None).unwrap();
    type_into(&mut doc, 1, "Il pleuvait sur la ville.");
    type_into(&mut doc, 2, "Le lendemain, le vent tomba.");
    type_into(&mut doc, 3, "Ce fut la fin.");
    let c1 = structure(&doc).iter().find(|e| e.title == "Chapitre 1").map(|e| e.block).unwrap();
    insert(&mut doc, ElementKind::Subchapter, "Sous la pluie", None, Some(c1)).unwrap();
    normalize(&mut doc);
    let mut before = texts(&doc);
    let words = total_words(&doc);
    // Chapter 3 before chapter 1.
    let entries = structure(&doc);
    let from = entries.iter().position(|e| e.title == "Chapitre 3").unwrap();
    let to = entries.iter().position(|e| e.title == "Chapitre 1").unwrap();
    move_to(&mut doc, from, to, Place::Before).unwrap();
    let titles: Vec<String> = structure(&doc).iter().map(|e| e.title.clone()).collect();
    assert_eq!(titles, ["Les Marées", "Chapitre 3", "Chapitre 1", "Sous la pluie", "Chapitre 2"]);
    let mut after = texts(&doc);
    before.sort();
    after.sort();
    assert_eq!(before, after, "the same paragraphs, only reordered");
    assert_eq!(total_words(&doc), words);
    // The text stays with its chapter.
    let body = doc.plain_text(StoryRef::Body);
    let p3 = body.find("Ce fut la fin.").unwrap();
    let p1 = body.find("Il pleuvait").unwrap();
    assert!(p3 < p1);
    // Chapter 1 with its sub-chapter to the end.
    let entries = structure(&doc);
    let from = entries.iter().position(|e| e.title == "Chapitre 1").unwrap();
    let to = entries.iter().position(|e| e.title == "Chapitre 2").unwrap();
    move_to(&mut doc, from, to, Place::After).unwrap();
    let titles: Vec<String> = structure(&doc).iter().map(|e| e.title.clone()).collect();
    assert_eq!(titles, ["Les Marées", "Chapitre 3", "Chapitre 2", "Chapitre 1", "Sous la pluie"]);
    // Sections: title page + 3 chapters, every chapter on a right-hand page, the body from 1.
    let sections = doc.sections();
    assert_eq!(sections.len(), 4);
    assert!(sections[1..].iter().all(|(_, sp)| sp.start == SectionStart::OddPage));
    assert_eq!(sections[1].1.page_num_start, Some(1));
    assert!(sections[2..].iter().all(|(_, sp)| sp.page_num_start.is_none()));
    // No section break on the last paragraph (that would add an empty page).
    assert!(doc.body.last().and_then(|b| b.as_para()).is_some_and(|p| p.section.is_none()));
}

#[test]
fn the_first_part_takes_in_the_chapters_already_written() {
    let mut doc = book();
    insert(&mut doc, ElementKind::Chapter, "", None, None).unwrap();
    insert(&mut doc, ElementKind::Part, "Première partie", None, None).unwrap();
    insert(&mut doc, ElementKind::Part, "Deuxième partie", None, None).unwrap();
    insert(&mut doc, ElementKind::Chapter, "", None, None).unwrap();
    let titles: Vec<String> = structure(&doc).iter().map(|e| e.title.clone()).collect();
    assert_eq!(titles, ["Les Marées", "Première partie", "Chapitre 1", "Chapitre 2", "Deuxième partie", "Chapitre 3"]);
    let part = &structure(&doc)[1];
    assert_eq!(part.end, structure(&doc)[4].block, "the part holds its two chapters");
}

#[test]
fn an_element_cannot_move_into_itself() {
    let mut doc = book();
    let c1 = structure(&doc).iter().find(|e| e.kind == ElementKind::Chapter).map(|e| e.block).unwrap();
    insert(&mut doc, ElementKind::Subchapter, "A", None, Some(c1)).unwrap();
    insert(&mut doc, ElementKind::Subchapter, "B", None, Some(c1)).unwrap();
    let entries = structure(&doc);
    let chapter = entries.iter().position(|e| e.kind == ElementKind::Chapter).unwrap();
    let b = entries.iter().position(|e| e.title == "B").unwrap();
    assert_eq!(move_to(&mut doc, chapter, b, Place::Before), Err(BookError::IntoItself));
    assert_eq!(move_to(&mut doc, 99, 0, Place::Before), Err(BookError::NoElement(99)));
    assert_eq!(move_to(&mut doc, 0, 99, Place::Before), Err(BookError::NoElement(99)));
    // Sub-chapters reorder inside their chapter.
    let a = entries.iter().position(|e| e.title == "A").unwrap();
    move_to(&mut doc, b, a, Place::Before).unwrap();
    let titles: Vec<String> = structure(&doc).iter().map(|e| e.title.clone()).collect();
    assert_eq!(titles, ["Les Marées", "Chapitre 1", "B", "A"]);
}

#[test]
fn rename_and_delete() {
    let mut doc = book();
    insert(&mut doc, ElementKind::Chapter, "", None, None).unwrap();
    rename(&mut doc, 1, "  L'arrivée ").unwrap();
    assert_eq!(structure(&doc)[1].title, "L'arrivée");
    assert_eq!(structure(&doc)[1].kind, ElementKind::Chapter);
    assert!(rename(&mut doc, 1, "   ").is_err());
    assert!(rename(&mut doc, 42, "x").is_err());
    delete(&mut doc, 1).unwrap();
    let titles: Vec<String> = structure(&doc).iter().map(|e| e.title.clone()).collect();
    assert_eq!(titles, ["Les Marées", "Chapitre 2"]);
    delete(&mut doc, 0).unwrap();
    delete(&mut doc, 0).unwrap();
    assert!(structure(&doc).is_empty());
    assert_eq!(doc.body.len(), 1, "the body is never left empty");
    assert!(delete(&mut doc, 0).is_err());
}

#[test]
fn word_counts_per_element_and_in_total() {
    let mut doc = book();
    type_into(&mut doc, 1, "Un deux trois quatre.");
    let entries = structure(&doc);
    assert_eq!(entries[1].words, 2 + 4, "the heading's words and the text's");
    assert_eq!(total_words(&doc), entries.iter().filter(|e| e.rank == 0 || e.rank == 1).map(|e| e.words).sum::<usize>());
}

#[test]
fn an_ordinary_document_becomes_a_book_from_its_headings() {
    let mut doc = Document::new();
    doc.body = vec![
        para_block(styled("Avant-texte", "Normal")),
        para_block(styled("Premier", "Heading1")),
        para_block(styled("Texte un.", "Normal")),
        para_block(styled("Détail", "Heading2")),
        para_block(styled("Second", "Heading1")),
        para_block(styled("Texte deux.", "Normal")),
    ];
    convert(&mut doc, &BookSettings::default());
    assert!(is_book(&doc));
    let k: Vec<ElementKind> = structure(&doc).iter().map(|e| e.kind).collect();
    assert_eq!(k, [ElementKind::Chapter, ElementKind::Subchapter, ElementKind::Chapter]);
    assert_eq!(doc.sections().len(), 3, "the text before, then one section per chapter");
    assert_eq!(texts(&doc).len(), 6, "no paragraph added or lost");
}

#[test]
fn a_project_file_reopens_the_whole_book() {
    let mut doc = book();
    insert(&mut doc, ElementKind::Dedication, "Pour Paul", None, None).unwrap();
    type_into(&mut doc, 2, "Il était une fois.");
    let bytes = write_project(&doc).unwrap();
    let back = read_project(&bytes).unwrap();
    assert_eq!(settings(&back), settings(&doc));
    assert_eq!(kinds(&back), kinds(&doc));
    assert_eq!(texts(&back), texts(&doc));
    // Through the dispatcher by extension too.
    let saved = crate::io::save_bytes("roman.lusklivre", &doc).unwrap();
    let opened = crate::io::open_bytes("roman.lusklivre", &saved).unwrap();
    assert_eq!(kinds(&opened), kinds(&doc));
    // Damaged projects are errors, not crashes.
    assert!(read_project(b"PK\x03\x04 nonsense").is_err());
    assert!(read_project(&[]).is_err());
}

#[test]
fn settings_are_clamped_to_what_a_page_can_hold() {
    let wild = BookSettings {
        format: "custom".into(),
        page_w_mm: f32::NAN,
        page_h_mm: 10_000.0,
        size: -3.0,
        inner_mm: 59.0,
        outer_mm: 59.0,
        line_spacing: 99.0,
        title: "x\u{0}y".into(),
        ..BookSettings::default()
    }
    .sanitized();
    assert_eq!(wild.page_w_mm, 148.0);
    assert_eq!(wild.page_h_mm, 600.0);
    assert_eq!(wild.size, 6.0);
    assert_eq!(wild.line_spacing, 3.0);
    assert!(wild.inner_mm + wild.outer_mm <= wild.page_w_mm - 40.0 + 0.01);
    assert_eq!(wild.title, "xy");
    // Settings in a document that are not JSON: not a book.
    let mut doc = Document::new();
    doc.set_custom_prop(PROP, "{not json");
    assert!(!is_book(&doc));
}

/// The ids of a section's header and footer parts, and whether its first page differs.
fn heads(doc: &Document) -> Vec<(Option<u32>, Option<u32>, Option<u32>, bool)> {
    doc.sections().iter().map(|(_, sp)| (sp.headers.default, sp.headers.even, sp.footers.default, sp.title_page)).collect()
}

fn part_text(doc: &Document, id: Option<u32>) -> String {
    id.and_then(|id| doc.parts.get(&id)).map(|p| p.blocks.iter().filter_map(|b| b.as_para()).map(|p| p.text.clone()).collect()).unwrap_or_default()
}

fn part_fields(doc: &Document, id: Option<u32>) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(p) = id.and_then(|id| doc.parts.get(&id)) {
        for b in &p.blocks {
            for o in b.as_para().map(|p| p.objects.as_slice()).unwrap_or_default() {
                if let wordcraft_doc::InlineObject::Field { instr, .. } = o {
                    out.push(instr.clone());
                }
            }
        }
    }
    out
}

#[test]
fn running_heads_and_folios_follow_the_settings() {
    let mut doc = book();
    insert(&mut doc, ElementKind::Dedication, "Pour Paul", None, None).unwrap();
    insert(&mut doc, ElementKind::Chapter, "", None, None).unwrap();
    let h = heads(&doc);
    // Title page and dedication: nothing at all; chapters: running heads and folios, but not on
    // their first page.
    assert_eq!(h.len(), 4);
    for (default, even, _, first_differs) in &h[..2] {
        assert_eq!(part_text(&doc, *default), "");
        assert_eq!(part_text(&doc, *even), "");
        assert!(!first_differs);
    }
    let (odd, even, footer, first_differs) = h[2];
    assert!(first_differs, "a chapter's first page has no running head");
    assert!(doc.settings.even_odd_headers);
    assert_eq!(part_text(&doc, even), "Jeanne Martin", "author on left pages");
    assert_eq!(part_text(&doc, odd), "Les Marées", "title on right pages");
    assert_eq!(part_fields(&doc, footer), ["PAGE"], "the folio at the bottom");
    let first = doc.sections()[2].1.headers.first;
    assert_eq!(part_text(&doc, first), "");
    // Title and chapter, folio at the top outside edge.
    let mut s = settings(&doc).unwrap();
    s.running_heads = "titreChapitre".into();
    s.folio = "hautExterieur".into();
    apply_settings(&mut doc, &s);
    normalize(&mut doc);
    let (odd, even, footer, _) = heads(&doc)[2];
    assert_eq!(part_fields(&doc, odd), ["STYLEREF 1", "PAGE"]);
    assert_eq!(part_fields(&doc, even), ["PAGE"]);
    assert!(part_text(&doc, even).ends_with("Les Marées"));
    assert!(part_fields(&doc, footer).is_empty(), "no folio at the bottom");
    // Made again on every change, never piling up.
    let parts = doc.parts.len();
    for _ in 0..5 {
        normalize(&mut doc);
    }
    assert_eq!(doc.parts.len(), parts);
    // Hand-made headers are left alone.
    let mut s = settings(&doc).unwrap();
    s.running_heads = "manuel".into();
    apply_settings(&mut doc, &s);
    let before = heads(&doc);
    normalize(&mut doc);
    assert_eq!(heads(&doc), before);
}

#[test]
fn drop_caps_open_chapters_when_asked() {
    let mut doc = book();
    type_into(&mut doc, 1, "Il était une fois.");
    let first_para = |doc: &Document| {
        let e = structure(doc).into_iter().find(|e| e.kind == ElementKind::Chapter).unwrap();
        doc.body[e.block + 1].as_para().unwrap().props.drop_cap
    };
    assert_eq!(first_para(&doc), None);
    let mut s = settings(&doc).unwrap();
    s.drop_caps = true;
    apply_settings(&mut doc, &s);
    normalize(&mut doc);
    assert_eq!(first_para(&doc), Some(DROP_CAP_LINES));
    s.drop_caps = false;
    apply_settings(&mut doc, &s);
    normalize(&mut doc);
    assert_eq!(first_para(&doc), None);
    // The preset of the contemporary novel asks for them.
    assert!(preset("romanContemporain").unwrap().settings().drop_caps);
}
