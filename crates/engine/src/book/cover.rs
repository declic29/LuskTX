//! Studio Couverture: a book's full cover — back cover, spine and front cover side by side,
//! with the printer's bleed all round — as an ordinary one-page document, so it is edited with
//! everything the editor already does and exported to PDF like any document (the trim box is
//! written from [`COVER_PROP`]).
//!
//! Sizes come only from the book's settings and the printer's specifications the user typed in:
//! the spine is `pages ÷ 2 × sheet thickness`, and stays 0 until that thickness is given.

use std::sync::Arc;

use wordcraft_doc::props::{Align, HeightRule, Rgb, TextColor, TextDirection, VAlign};
use wordcraft_doc::{Block, CharProps, Document, InlineObject, ParaProps, Paragraph, Table, para_block};

use super::{BookSettings, mm, settings};

/// The custom property a cover document carries: its bleed, in points (the PDF export writes the
/// trim box that far inside the page).
pub const COVER_PROP: &str = "LuskCouverture";

/// The largest cover picture read, bytes.
const MAX_PICTURE: u64 = 64 << 20;

/// Borders drawn nowhere (the cover's colour runs to its edges).
fn no_borders() -> wordcraft_doc::props::Borders {
    let none = Some(wordcraft_doc::props::Border { style: wordcraft_doc::props::BorderStyle::None, width: 0.0, color: None, space: 0.0 });
    wordcraft_doc::props::Borders { top: none, left: none, bottom: none, right: none, between: none, inside_v: none }
}

fn colour(hex: &str, fallback: Rgb) -> Rgb {
    Rgb::parse(hex).unwrap_or(fallback)
}

/// A paragraph of `text` in `props`, aligned `align`, with `before` points above it.
fn para(text: &str, props: &CharProps, align: Align, before: f32) -> Block {
    let mut p = Paragraph::with_text(text, props.clone());
    p.props = ParaProps {
        align: Some(align),
        indent_first: Some(0.0),
        indent_left: Some(0.0),
        indent_right: Some(0.0),
        space_before: Some(before),
        space_after: Some(0.0),
        ..ParaProps::default()
    };
    p.mark = props.clone();
    Block::Para(p)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_picture(path: &str) -> Result<Vec<u8>, String> {
    crate::io::read_file_capped(path, MAX_PICTURE, "image de couverture")
}

/// The browser can't read a path on the user's computer.
#[cfg(target_arch = "wasm32")]
fn read_picture(path: &str) -> Result<Vec<u8>, String> {
    let _ = MAX_PICTURE;
    Err(format!("{path} : image inaccessible depuis le navigateur"))
}

/// The front picture: read from `path`, scaled to fit `max_w` × `max_h` points. `None` when
/// there is no path; an error when the file can't be read or isn't a picture.
fn picture(doc: &mut Document, path: &str, max_w: f32, max_h: f32) -> Result<Option<Block>, String> {
    if path.trim().is_empty() {
        return Ok(None);
    }
    let bytes = read_picture(path)?;
    let ext = wordcraft_formats::model::sniff_image(&bytes).ok_or_else(|| format!("le fichier {path} n'est pas une image"))?;
    let (pw, ph) = wordcraft_formats::model::image_px(&bytes)
        .filter(|(w, h)| *w > 0 && *h > 0)
        .ok_or_else(|| format!("image de couverture illisible ({path})"))?;
    let k = (max_w / pw as f32).min(max_h / ph as f32);
    let (w, h) = ((pw as f32 * k).max(1.0), (ph as f32 * k).max(1.0));
    let media = doc.add_media(bytes, ext);
    let mut p = Paragraph::new();
    p.props = ParaProps { align: Some(Align::Center), indent_first: Some(0.0), space_before: Some(18.0), ..ParaProps::default() };
    let _ = p.insert_object(
        0,
        InlineObject::Image { media, w, h, alt: "Illustration de couverture".into(), float: Default::default(), crop: [0.0; 4], ole: None },
        &CharProps::default(),
    );
    Ok(Some(Block::Para(p)))
}

/// The cover of the book `book`, `pages` pages long: one page as wide as back + spine + front
/// plus the bleed on both sides, as tall as a page plus the bleed above and below.
pub fn cover_document(book: &Document, pages: usize) -> Result<Document, String> {
    let s: BookSettings = settings(book).ok_or("ce document n'est pas un livre")?;
    let (w, h) = (mm(s.page_w_mm), mm(s.page_h_mm));
    let bleed = mm(s.bleed_mm);
    let spine = s.spine_mm(pages).map_or(0.0, mm);
    let mut doc = Document::new();
    doc.core.title = format!("Couverture — {}", if s.title.is_empty() { "livre sans titre" } else { &s.title });
    doc.core.creator = s.author.clone();
    doc.styles.default_chr.lang = Some("fr-FR".into());
    let sp = &mut doc.last_section;
    sp.page_w = 2.0 * w + spine + 2.0 * bleed;
    sp.page_h = h + 2.0 * bleed;
    sp.landscape = false;
    for m in [&mut sp.margin_left, &mut sp.margin_right, &mut sp.margin_top, &mut sp.margin_bottom, &mut sp.header, &mut sp.footer, &mut sp.gutter] {
        *m = 0.0;
    }
    let bg = colour(&s.cover_bg, Rgb(0x1F, 0x3A, 0x4D));
    let fg = colour(&s.cover_fg, Rgb::WHITE);
    let font = Some(s.font.clone());
    let text = |size: f32, bold: bool, italic: bool| CharProps {
        size: Some(size),
        bold: bold.then_some(true),
        italic: italic.then_some(true),
        color: Some(TextColor::Rgb(fg)),
        font: font.clone(),
        ..CharProps::default()
    };
    // Safe margins inside the trimmed cover.
    let safe = mm(15.0);
    // Back cover: the text, then the ISBN and the bar code's place at the foot.
    let mut back = Vec::new();
    let summary = if s.back_text.is_empty() {
        "Texte de la quatrième de couverture : résumé du livre, quelques mots sur l'auteur…".to_string()
    } else {
        s.back_text.clone()
    };
    for (i, line) in summary.lines().enumerate() {
        back.push(para(line, &text(10.5, false, false), Align::Justify, if i == 0 { 0.0 } else { 6.0 }));
    }
    if !s.isbn.is_empty() {
        back.push(para(&format!("ISBN {}", s.isbn), &text(9.0, false, false), Align::Right, 24.0));
    }
    if s.barcode_space {
        // A white box for the bar code the printer or the ISBN agency provides.
        let mut code = Table::new(1, 1, mm(50.0));
        code.props.align = Some(Align::Right);
        code.props.borders = Some(no_borders());
        code.props.style = None;
        if let Some(row) = code.rows.first_mut() {
            row.props.height = Some(mm(28.0));
            row.props.height_rule = HeightRule::Exact;
            if let Some(cell) = row.cells.first_mut() {
                cell.props.shading = Some(Rgb::WHITE);
                cell.props.valign = Some(VAlign::Center);
                let dark =
                    CharProps { size: Some(8.0), color: Some(TextColor::Rgb(Rgb(0x55, 0x55, 0x55))), font: font.clone(), ..CharProps::default() };
                cell.blocks = vec![Arc::new(para("Emplacement du code-barres", &dark, Align::Center, 0.0))];
            }
        }
        back.push(para("", &text(6.0, false, false), Align::Left, 18.0));
        back.push(Block::Table(code));
    }
    // Spine: author and title, read from bottom to top as French books do.
    let spine_text = match (s.author.is_empty(), s.title.is_empty()) {
        (false, false) => format!("{}  —  {}", s.author, s.title),
        (true, false) => s.title.clone(),
        (false, true) => s.author.clone(),
        (true, true) => String::new(),
    };
    let spine_size = (spine * 0.45).clamp(6.0, 14.0);
    // Front cover: author, title, subtitle, then the picture.
    let mut front = vec![
        para(if s.author.is_empty() { "Nom de l'auteur" } else { &s.author }, &text(15.0, false, false), Align::Center, 0.0),
        para(if s.title.is_empty() { "Titre du livre" } else { &s.title }, &text(30.0, true, false), Align::Center, 48.0),
    ];
    if !s.subtitle.is_empty() {
        front.push(para(&s.subtitle, &text(16.0, false, true), Align::Center, 12.0));
    }
    let mut cover_doc = doc;
    if let Some(pic) = picture(&mut cover_doc, &s.cover_image, (w - 2.0 * safe).max(36.0), (h * 0.45).max(36.0))? {
        front.push(pic);
    }
    // One row, three cells: back | spine | front, all the cover's colour, to the bleed's edge.
    let mut table = Table::new(1, 3, cover_doc.last_section.page_w);
    table.grid = vec![w + bleed, spine.max(0.5), w + bleed];
    table.props.borders = Some(no_borders());
    table.props.style = None;
    table.props.cell_margins = Some([0.0; 4]);
    table.props.indent = Some(0.0);
    table.props.fixed = true;
    if let Some(row) = table.rows.first_mut() {
        // An exact row height runs to the bottom of the text area: the cells' bottom margin comes
        // on top of it. A hair short of the page, so the closing paragraph (a document ends with
        // one) stays on it; the page colour fills that hair.
        row.props.height = Some((h + 2.0 * bleed - 3.0 - (bleed + safe)).max(1.0));
        row.props.height_rule = HeightRule::Exact;
        row.props.cant_split = true;
        let margins = [
            // top, left, bottom, right (Word's order for cell margins in this model: t, l, b, r).
            [bleed + safe, bleed + safe, bleed + safe, safe],
            [bleed + mm(8.0), 0.0, bleed + safe, 0.0],
            [bleed + safe * 2.0, safe, bleed + safe, bleed + safe],
        ];
        let contents = [back, vec![para(&spine_text, &text(spine_size, true, false), Align::Center, 0.0)], front];
        for (i, (cell, blocks)) in row.cells.iter_mut().zip(contents).enumerate() {
            cell.props.shading = Some(bg);
            cell.props.width = table.grid.get(i).copied();
            cell.props.margins = margins.get(i).copied();
            cell.props.valign = Some(if i == 1 { VAlign::Center } else { VAlign::Top });
            if i == 1 {
                cell.props.text_direction = TextDirection::Up;
                cell.props.text_direction_set = true;
            }
            cell.blocks = blocks.into_iter().map(Arc::new).collect();
        }
    }
    cover_doc.body = vec![Arc::new(Block::Table(table)), para_block(Paragraph::new())];
    // The empty paragraph after the table stays out of sight (a one-point line).
    if let Some(Block::Para(p)) = cover_doc.body.last_mut().map(Arc::make_mut) {
        p.mark.size = Some(1.0);
        p.props.space_before = Some(0.0);
        p.props.space_after = Some(0.0);
        p.props.line_spacing = Some(wordcraft_doc::props::LineSpacing::Exactly(1.0));
    }
    cover_doc.settings.page_color = Some(bg);
    cover_doc.set_custom_prop(COVER_PROP, &format!("{bleed:.3}"));
    Ok(cover_doc)
}

#[cfg(test)]
mod tests {
    use super::super::{BookSettings, apply_settings, new_document};
    use super::*;

    fn book(s: BookSettings) -> Document {
        new_document(Document::new(), &s)
    }

    #[test]
    fn the_cover_is_back_spine_and_front_with_the_bleed() {
        let s = BookSettings {
            title: "Les Marées".into(),
            author: "Jeanne Martin".into(),
            bleed_mm: 3.0,
            sheet_mm: 0.1,
            isbn: "978-2-1234-5680-3".into(),
            back_text: "Un été sur la côte.\nUne lettre jamais ouverte.".into(),
            ..BookSettings::default()
        };
        let doc = book(s.clone());
        // 200 pages on 0.1 mm sheets: a 10 mm spine.
        assert_eq!(s.spine_mm(200), Some(10.0));
        assert_eq!(s.spine_mm(201), Some(10.1));
        let cover = cover_document(&doc, 200).unwrap();
        let sp = &cover.last_section;
        assert!((sp.page_w - mm(2.0 * 148.0 + 10.0 + 6.0)).abs() < 0.05, "back + spine + front + bleed");
        assert!((sp.page_h - mm(210.0 + 6.0)).abs() < 0.05);
        assert_eq!(cover.custom_prop(COVER_PROP).and_then(|v| v.parse::<f32>().ok()).map(|v| (v - mm(3.0)).abs() < 0.01), Some(true));
        let text = cover.plain_text(wordcraft_doc::StoryRef::Body);
        for needle in ["Les Marées", "Jeanne Martin", "Un été sur la côte.", "ISBN 978-2-1234-5680-3", "Emplacement du code-barres"] {
            assert!(text.contains(needle), "{needle} on the cover: {text}");
        }
        // The spine reads bottom to top.
        let Some(Block::Table(t)) = cover.body.first().map(|b| &**b) else { panic!("a table") };
        let spine = &t.rows[0].cells[1];
        assert_eq!(spine.props.text_direction, TextDirection::Up);
        // It lays out on one page.
        let lay = wordcraft_layout::layout(&cover, &mut wordcraft_layout::LayoutCache::new(), &Default::default());
        assert_eq!(lay.pages.len(), 1);
        // The PDF has its trim box.
        let mut doc2 = doc.clone();
        apply_settings(&mut doc2, &s);
        let pdf = crate::io::save_bytes("couverture.pdf", &cover).unwrap();
        assert!(String::from_utf8_lossy(&pdf).contains("TrimBox") || pdf.len() > 1000);
    }

    #[test]
    fn without_printer_specs_the_spine_is_not_guessed() {
        let doc = book(BookSettings { title: "Essai".into(), ..BookSettings::default() });
        assert_eq!(settings(&doc).unwrap().spine_mm(300), None);
        let cover = cover_document(&doc, 300).unwrap();
        assert!((cover.last_section.page_w - mm(2.0 * 148.0)).abs() < 0.6, "no spine, no bleed until the printer's figures are typed in");
        // A missing picture is an error, not a crash.
        let mut s = settings(&doc).unwrap();
        s.cover_image = "/nonexistent/picture.png".into();
        let mut doc = doc;
        apply_settings(&mut doc, &s);
        assert!(cover_document(&doc, 10).is_err());
        assert!(cover_document(&Document::new(), 10).is_err(), "not a book");
    }
}
