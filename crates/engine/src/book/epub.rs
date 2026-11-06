//! EPUB 3 export of a book (Lusk Texte): one XHTML file per element of the book (title page,
//! dedication, each part and chapter…), a navigation document built from the book's structure,
//! a style sheet with the book's typography, the pictures as files, and the package document
//! with the book's title, author and language.
//!
//! The XHTML comes from the HTML export (`wordcraft_formats::html`), made well-formed XML: void
//! elements closed, `&nbsp;` as a character reference, pictures moved out of `data:` URIs into
//! the package.

use std::io::Write;

use wordcraft_doc::Document;

use super::{ElementKind, settings, structure};

/// XML text escaping (attributes and text).
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c if c.is_control() && c != '\n' && c != '\t' => {}
            c => out.push(c),
        }
    }
    out
}

/// A picture taken out of the XHTML: its file name in the package and bytes.
struct Picture {
    name: String,
    mime: &'static str,
    bytes: Vec<u8>,
}

/// HTML from the HTML export made into well-formed XHTML; `data:` pictures become files.
fn xhtmlify(html: &str, pictures: &mut Vec<Picture>) -> String {
    let mut s = html.replace("&nbsp;", "&#160;");
    // Pictures out of data URIs.
    if let Ok(re) = regex::Regex::new(r#"src="data:([a-zA-Z0-9.+/-]+);base64,([A-Za-z0-9+/=\s]*)""#) {
        let mut out = String::with_capacity(s.len());
        let mut last = 0;
        for c in re.captures_iter(&s) {
            let (Some(whole), Some(mime), Some(data)) = (c.get(0), c.get(1), c.get(2)) else { continue };
            out.push_str(s.get(last..whole.start()).unwrap_or_default());
            last = whole.end();
            let bytes = wordcraft_formats::model::base64_decode(data.as_str()).unwrap_or_default();
            let (ext, mime) = match mime.as_str() {
                "image/png" => ("png", "image/png"),
                "image/jpeg" | "image/jpg" => ("jpg", "image/jpeg"),
                "image/gif" => ("gif", "image/gif"),
                "image/svg+xml" => ("svg", "image/svg+xml"),
                "image/webp" => ("webp", "image/webp"),
                _ => ("", ""),
            };
            if bytes.is_empty() || ext.is_empty() || pictures.len() >= 10_000 {
                out.push_str("src=\"\"");
                continue;
            }
            let name = format!("images/image{}.{ext}", pictures.len() + 1);
            out.push_str(&format!("src=\"{name}\""));
            pictures.push(Picture { name, mime, bytes });
        }
        out.push_str(s.get(last..).unwrap_or_default());
        s = out;
    }
    // Void elements closed.
    if let Ok(re) = regex::Regex::new(r"<(br|hr|img|col|wbr|meta|link|input)(\s[^<>]*?)?\s*/?>") {
        s = re.replace_all(&s, "<$1$2/>").into_owned();
    }
    s
}

/// The marker a note reference becomes in the HTML export (a bookmark), so the EPUB can put a
/// note link in its place.
const NOTE_MARK: &str = "lusk-note-";

/// Every footnote and endnote reference in `blocks` (tables too) becomes a bookmark named
/// [`NOTE_MARK`]`n`, keeping its object character so the runs stay as they are; `found` gets
/// the number, the note's story and its custom mark, in reading order.
fn notes_to_marks(blocks: &mut [std::sync::Arc<wordcraft_doc::Block>], found: &mut Vec<(usize, u32, String)>, depth: usize) {
    if depth > 16 {
        return;
    }
    for b in blocks.iter_mut() {
        let has_notes = |p: &wordcraft_doc::Paragraph| p.objects.iter().any(|o| matches!(o, wordcraft_doc::InlineObject::NoteRef { .. }));
        let needed = match &**b {
            wordcraft_doc::Block::Para(p) => has_notes(p),
            wordcraft_doc::Block::Table(_) => true,
        };
        if !needed {
            continue;
        }
        match std::sync::Arc::make_mut(b) {
            wordcraft_doc::Block::Para(p) => {
                for o in p.objects.iter_mut() {
                    if let wordcraft_doc::InlineObject::NoteRef { id, custom, .. } = o {
                        let n = found.len() + 1;
                        found.push((n, *id, custom.clone()));
                        *o = wordcraft_doc::InlineObject::BookmarkStart { name: format!("{NOTE_MARK}{n}") };
                    }
                }
            }
            wordcraft_doc::Block::Table(t) => {
                for c in t.rows.iter_mut().flat_map(|r| r.cells.iter_mut()) {
                    notes_to_marks(&mut c.blocks, found, depth + 1);
                }
            }
        }
    }
}

/// The note links in place of the markers, and the notes themselves (EPUB 3 `noteref` and
/// `footnote` asides, which reading systems show as pop-ups) after the content.
fn with_notes(body: String, doc: &Document, notes: &[(usize, u32, String)], first: usize) -> String {
    if notes.is_empty() {
        return body;
    }
    let mut body = body;
    let mut asides = String::new();
    for (n, id, custom) in notes {
        let k = n + first;
        let mark = if custom.trim().is_empty() { k.to_string() } else { esc(custom.trim()) };
        body = body.replacen(
            &format!("<a id=\"{NOTE_MARK}{n}\"></a>"),
            &format!("<sup><a epub:type=\"noteref\" class=\"noteref\" id=\"noteref-{k}\" href=\"#note-{k}\">{mark}</a></sup>"),
            1,
        );
        let text = doc.plain_text(wordcraft_doc::StoryRef::Part(*id)).replace(wordcraft_doc::para::OBJ, "");
        let paras: Vec<String> = text.split('\n').map(str::trim).filter(|t| !t.is_empty()).map(esc).collect();
        let paras = if paras.is_empty() { vec![String::new()] } else { paras };
        asides.push_str(&format!(
            "<aside epub:type=\"footnote\" class=\"note\" id=\"note-{k}\">\n<p><a href=\"#noteref-{k}\">{mark}</a>. {}</p>\n",
            paras.first().map_or("", String::as_str)
        ));
        for p in paras.iter().skip(1) {
            asides.push_str(&format!("<p>{p}</p>\n"));
        }
        asides.push_str("</aside>\n");
    }
    format!("{body}\n<section class=\"notes\" epub:type=\"footnotes\">\n{asides}</section>")
}

/// One XHTML content document.
fn page(title: &str, lang: &str, body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE html>\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:epub=\"http://www.idpf.org/2007/ops\" xml:lang=\"{lang}\" lang=\"{lang}\">\n<head>\n<meta charset=\"utf-8\"/>\n<title>{}</title>\n<link rel=\"stylesheet\" type=\"text/css\" href=\"style.css\"/>\n</head>\n<body>\n{body}\n</body>\n</html>\n",
        esc(title)
    )
}

/// The book's typography for reading devices.
fn style_sheet(font: &str, indent_mm: f32, justify: bool) -> String {
    format!(
        "body {{ font-family: \"{}\", serif; line-height: 1.4; margin: 0 5%; }}\n\
         p {{ margin: 0; text-indent: {:.1}mm; text-align: {}; hyphens: auto; -epub-hyphens: auto; }}\n\
         h1, h2, h3 {{ text-align: center; text-indent: 0; page-break-after: avoid; break-after: avoid; }}\n\
         h1 {{ margin: 3em 0 2em; font-size: 1.6em; font-weight: normal; }}\n\
         h2 {{ margin: 1.5em 0 0.8em; font-size: 1.2em; }}\n\
         img {{ max-width: 100%; height: auto; }}\n\
         table {{ border-collapse: collapse; margin: 1em auto; }}\n\
         nav ol {{ list-style: none; padding-left: 1em; }}\n\
         a.noteref {{ text-decoration: none; }}\n\
         section.notes {{ margin-top: 2em; border-top: 1px solid #999; font-size: 0.85em; }}\n\
         aside.note p {{ text-indent: 0; margin: 0.4em 0; }}\n",
        font.replace(['"', '\\', '<', '>'], ""),
        indent_mm.clamp(0.0, 30.0),
        if justify { "justify" } else { "left" }
    )
}

/// A stable identifier for the book (`urn:uuid:` from its title, author and creation date), so
/// the same book keeps its identifier from one export to the next.
fn identifier(doc: &Document, title: &str, author: &str) -> String {
    let mut h: u128 = 0x6c62272e07bb014262b821756295c58d;
    for b in title.bytes().chain(author.bytes()).chain(doc.core.created.bytes()) {
        h ^= u128::from(b);
        h = h.wrapping_mul(0x0000000001000000000000000000013B);
    }
    let x = format!("{h:032x}");
    let p = |a: usize, b: usize| x.get(a..b).unwrap_or("0");
    format!("urn:uuid:{}-{}-4{}-a{}-{}", p(0, 8), p(8, 12), p(13, 16), p(17, 20), p(20, 32))
}

/// The EPUB 3 package of `doc` (a book, or any document: then one content document).
pub fn write(doc: &Document) -> Result<Vec<u8>, String> {
    let s = settings(doc).unwrap_or_default();
    let title = if !s.title.is_empty() {
        s.title.clone()
    } else if !doc.core.title.is_empty() {
        doc.core.title.clone()
    } else {
        "Sans titre".into()
    };
    let author = if !s.author.is_empty() { s.author.clone() } else { doc.core.creator.clone() };
    let lang = doc
        .styles
        .default_chr
        .lang
        .clone()
        .filter(|l| !l.is_empty() && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .unwrap_or_else(|| "fr-FR".into());
    let entries = structure(doc);
    // Split points: every element of rank 0 or 1 starts a content document; text before the
    // first one is a document of its own.
    let mut starts: Vec<usize> = entries.iter().filter(|e| e.rank <= 1).map(|e| e.block).collect();
    if starts.first() != Some(&0) {
        starts.insert(0, 0);
    }
    starts.dedup();
    let n = doc.body.len();
    let mut pictures = Vec::new();
    let mut files: Vec<(String, String, String)> = Vec::new(); // (file, title, xhtml)
    let mut note_count = 0;
    for (i, &start) in starts.iter().enumerate() {
        let end = starts.get(i + 1).copied().unwrap_or(n).min(n);
        if start >= end {
            continue;
        }
        let mut part = doc.clone();
        part.body = doc.body.get(start..end).map(<[_]>::to_vec).unwrap_or_default();
        let mut notes = Vec::new();
        notes_to_marks(&mut part.body, &mut notes, 0);
        let html = wordcraft_formats::html::export_fragment(&part);
        let html = html.strip_prefix("<meta charset=\"utf-8\">").unwrap_or(&html);
        let body = with_notes(xhtmlify(html, &mut pictures), doc, &notes, note_count);
        note_count += notes.len();
        let chunk_title = entries
            .iter()
            .find(|e| e.block == start)
            .map(|e| if e.title.is_empty() { e.label.to_string() } else { e.title.clone() })
            .unwrap_or_else(|| title.clone());
        let file = format!("text{:03}.xhtml", files.len() + 1);
        files.push((file, chunk_title.clone(), page(&chunk_title, &lang, &body)));
    }
    if files.is_empty() {
        files.push(("text001.xhtml".into(), title.clone(), page(&title, &lang, "<p/>")));
    }
    // Navigation: the elements in their tree (sub-chapters under their chapter).
    let file_of = |block: usize| -> Option<&str> {
        let idx = starts.iter().rposition(|&st| st <= block)?;
        files.get(idx.min(files.len().saturating_sub(1))).map(|f| f.0.as_str())
    };
    let mut nav = String::from("<nav epub:type=\"toc\" id=\"toc\">\n<h1>Table des matières</h1>\n<ol>\n");
    let mut open_sub = false;
    for e in entries.iter().filter(|e| !matches!(e.kind, ElementKind::TitlePage | ElementKind::Dedication | ElementKind::Epigraph)) {
        let Some(file) = file_of(e.block) else { continue };
        let label = if e.title.is_empty() { e.label.to_string() } else { e.title.clone() };
        if e.rank == 2 {
            if !open_sub {
                nav.push_str("<ol>\n");
                open_sub = true;
            }
            nav.push_str(&format!("<li><a href=\"{file}\">{}</a></li>\n", esc(&label)));
            continue;
        }
        if open_sub {
            nav.push_str("</ol></li>\n");
            open_sub = false;
        } else if nav.ends_with("</a>\n") {
            nav.push_str("</li>\n");
        }
        nav.push_str(&format!("<li><a href=\"{file}\">{}</a>\n", esc(&label)));
    }
    if open_sub {
        nav.push_str("</ol></li>\n");
    } else if nav.ends_with("</a>\n") {
        nav.push_str("</li>\n");
    }
    if !nav.contains("<li>") {
        nav.push_str(&format!("<li><a href=\"{}\">{}</a></li>\n", files.first().map_or("text001.xhtml", |f| f.0.as_str()), esc(&title)));
    }
    nav.push_str("</ol>\n</nav>");
    let nav_doc = page("Table des matières", &lang, &nav);
    // Package document.
    let id = identifier(doc, &title, &author);
    let modified = crate::cmd::now_iso();
    let mut manifest = String::from(
        "<item id=\"nav\" href=\"nav.xhtml\" media-type=\"application/xhtml+xml\" properties=\"nav\"/>\n<item id=\"css\" href=\"style.css\" media-type=\"text/css\"/>\n",
    );
    let mut spine = String::new();
    for (i, (file, _, _)) in files.iter().enumerate() {
        manifest.push_str(&format!("<item id=\"t{i}\" href=\"{file}\" media-type=\"application/xhtml+xml\"/>\n"));
        spine.push_str(&format!("<itemref idref=\"t{i}\"/>\n"));
    }
    for (i, p) in pictures.iter().enumerate() {
        manifest.push_str(&format!("<item id=\"img{i}\" href=\"{}\" media-type=\"{}\"/>\n", p.name, p.mime));
    }
    let creator = if author.is_empty() { String::new() } else { format!("<dc:creator>{}</dc:creator>\n", esc(&author)) };
    let isbn = if s.isbn.is_empty() { String::new() } else { format!("<dc:source>urn:isbn:{}</dc:source>\n", esc(&s.isbn.replace([' ', '-'], ""))) };
    let opf = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<package xmlns=\"http://www.idpf.org/2007/opf\" version=\"3.0\" unique-identifier=\"bookid\" xml:lang=\"{lang}\">\n<metadata xmlns:dc=\"http://purl.org/dc/elements/1.1/\">\n<dc:identifier id=\"bookid\">{id}</dc:identifier>\n<dc:title>{}</dc:title>\n{creator}{isbn}<dc:language>{lang}</dc:language>\n<meta property=\"dcterms:modified\">{modified}</meta>\n</metadata>\n<manifest>\n{manifest}</manifest>\n<spine>\n{spine}</spine>\n</package>\n",
        esc(&title)
    );
    let container = "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<container version=\"1.0\" xmlns=\"urn:oasis:names:tc:opendocument:xmlns:container\">\n<rootfiles>\n<rootfile full-path=\"OEBPS/content.opf\" media-type=\"application/oebps-package+xml\"/>\n</rootfiles>\n</container>\n";
    let css = style_sheet(&s.font, if super::is_book(doc) { s.indent_mm } else { 0.0 }, s.align != "left");
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut zip = zip::ZipWriter::new(&mut out);
        let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        let deflated = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        let mut put = |name: &str, bytes: &[u8], opts| -> Result<(), String> {
            zip.start_file(name, opts).map_err(|e| e.to_string())?;
            zip.write_all(bytes).map_err(|e| e.to_string())
        };
        // `mimetype` first and uncompressed, as EPUB requires.
        put("mimetype", b"application/epub+zip", stored)?;
        put("META-INF/container.xml", container.as_bytes(), deflated)?;
        put("OEBPS/content.opf", opf.as_bytes(), deflated)?;
        put("OEBPS/nav.xhtml", nav_doc.as_bytes(), deflated)?;
        put("OEBPS/style.css", css.as_bytes(), deflated)?;
        for (file, _, xhtml) in &files {
            put(&format!("OEBPS/{file}"), xhtml.as_bytes(), deflated)?;
        }
        for p in &pictures {
            put(&format!("OEBPS/{}", p.name), &p.bytes, stored)?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    Ok(out.into_inner())
}

#[cfg(test)]
mod tests {
    use std::io::Read;

    use super::super::{BookSettings, ElementKind, insert, new_document};
    use super::*;

    fn entry(zip: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>, name: &str) -> String {
        let mut s = String::new();
        zip.by_name(name).unwrap().read_to_string(&mut s).unwrap();
        s
    }

    /// Every XML file of the package parses.
    fn well_formed(xml: &str) -> bool {
        let mut r = quick_xml::Reader::from_str(xml);
        loop {
            match r.read_event() {
                Ok(quick_xml::events::Event::Eof) => return true,
                Ok(_) => {}
                Err(e) => {
                    eprintln!("{e}: {xml}");
                    return false;
                }
            }
        }
    }

    #[test]
    fn a_book_exports_as_a_valid_epub_package() {
        let s = BookSettings {
            title: "Les Marées & la mer".into(),
            author: "Jeanne Martin".into(),
            isbn: "978-2-1234-5680-3".into(),
            ..BookSettings::default()
        };
        let mut doc = new_document(Document::new(), &s);
        insert(&mut doc, ElementKind::Dedication, "À Paul", None, None).unwrap();
        insert(&mut doc, ElementKind::Chapter, "La traversée", None, None).unwrap();
        let c1 = structure(&doc).iter().find(|e| e.title == "Chapitre 1").map(|e| e.block).unwrap();
        insert(&mut doc, ElementKind::Subchapter, "Le port", None, Some(c1)).unwrap();
        let bytes = write(&doc).unwrap();
        // `mimetype` first, stored.
        assert_eq!(bytes.get(30..38), Some(&b"mimetype"[..]));
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        assert_eq!(entry(&mut zip, "mimetype"), "application/epub+zip");
        let opf = entry(&mut zip, "OEBPS/content.opf");
        assert!(opf.contains("<dc:title>Les Marées &amp; la mer</dc:title>"));
        assert!(opf.contains("<dc:creator>Jeanne Martin</dc:creator>") && opf.contains("urn:isbn:9782123456803"));
        let lang = doc.styles.default_chr.lang.clone().unwrap_or_else(|| "fr-FR".into());
        assert!(opf.contains("dcterms:modified") && opf.contains(&format!("<dc:language>{lang}</dc:language>")));
        // Title page, dedication, chapter 1 (with its sub-chapter), chapter 2.
        assert_eq!(opf.matches("<itemref").count(), 4);
        let nav = entry(&mut zip, "OEBPS/nav.xhtml");
        assert!(nav.contains(">Chapitre 1</a>") && nav.contains(">Le port</a>") && nav.contains(">La traversée</a>"));
        assert!(!nav.contains("À Paul"), "the dedication is not in the table of contents");
        let names: Vec<String> = zip.file_names().map(str::to_string).collect();
        for name in names.iter().filter(|n| n.ends_with(".xhtml") || n.ends_with(".opf") || n.ends_with(".xml")) {
            let xml = entry(&mut zip, name);
            assert!(well_formed(&xml), "{name} is well-formed XML");
        }
        let ch2 = entry(&mut zip, "OEBPS/text004.xhtml");
        assert!(ch2.contains("La traversée"));
        // Stable identifier.
        assert_eq!(identifier(&doc, "a", "b"), identifier(&doc, "a", "b"));
    }

    /// A book with everything a manuscript holds: front matter, a part, chapters and
    /// sub-chapters, French typography, bold and italics, a bulleted list, a table, a picture, a
    /// link and a footnote.
    fn rich_book() -> Document {
        let s = BookSettings {
            title: "Les Marées & la mer".into(),
            subtitle: "Roman".into(),
            author: "Jeanne Martin".into(),
            isbn: "978-2-1234-5680-3".into(),
            ..BookSettings::default()
        };
        let mut doc = new_document(Document::new(), &s);
        insert(&mut doc, ElementKind::Dedication, "À Paul", None, None).unwrap();
        insert(&mut doc, ElementKind::Epigraph, "« La mer, la mer, toujours recommencée. »", None, None).unwrap();
        insert(&mut doc, ElementKind::Preface, "Préface", None, None).unwrap();
        insert(&mut doc, ElementKind::Part, "Première partie", None, None).unwrap();
        insert(&mut doc, ElementKind::Chapter, "La traversée", None, None).unwrap();
        let c1 = structure(&doc).iter().find(|e| e.title == "Chapitre 1").map(|e| e.block).unwrap();
        insert(&mut doc, ElementKind::Subchapter, "Le port", None, Some(c1)).unwrap();
        insert(&mut doc, ElementKind::Acknowledgments, "Remerciements", None, None).unwrap();
        let mut ses = crate::Session::new(doc);
        // The caret at the end of the paragraph after an element's heading.
        let caret_after = |ses: &mut crate::Session, block: usize| {
            let p = wordcraft_doc::Pos::body(block + 1, usize::MAX);
            ses.sel = crate::Selection::caret(ses.doc.clamp(&p));
        };
        let last = structure(&ses.doc).iter().find(|e| e.title == "La traversée").map(|e| e.block).unwrap();
        caret_after(&mut ses, last);
        let run = |ses: &mut crate::Session, id: &str, v: serde_json::Value| {
            ses.run(id, &v).unwrap_or_else(|e| panic!("{id}: {e:?}"));
        };
        run(&mut ses, "text.insert", serde_json::json!({"text": "Il dit : « Partons ! » Elle répondit : l’aube, déjà… "}));
        run(&mut ses, "format.bold", serde_json::json!({}));
        run(&mut ses, "text.insert", serde_json::json!({"text": "Gras & <balises>"}));
        run(&mut ses, "format.bold", serde_json::json!({}));
        run(&mut ses, "references.footnote", serde_json::json!({}));
        run(&mut ses, "text.insert", serde_json::json!({"text": "Une note de bas de page."}));
        ses.run("references.footnote", &serde_json::json!({})).ok();
        let last = structure(&ses.doc).iter().find(|e| e.title == "La traversée").map(|e| e.block).unwrap();
        caret_after(&mut ses, last);
        run(&mut ses, "text.newParagraph", serde_json::json!({}));
        run(&mut ses, "text.insert", serde_json::json!({"text": "Premier point"}));
        run(&mut ses, "para.bullets", serde_json::json!({}));
        run(&mut ses, "text.newParagraph", serde_json::json!({}));
        run(&mut ses, "text.insert", serde_json::json!({"text": "Second point"}));
        run(&mut ses, "text.newParagraph", serde_json::json!({}));
        run(&mut ses, "para.bullets", serde_json::json!({"off": true}));
        run(&mut ses, "insert.link", serde_json::json!({"url": "https://lusk.bzh/?a=1&b=2", "text": "un lien"}));
        run(&mut ses, "text.newParagraph", serde_json::json!({}));
        run(&mut ses, "insert.table", serde_json::json!({"rows": 2, "cols": 2}));
        run(&mut ses, "text.insert", serde_json::json!({"text": "Case"}));
        let mut png = Vec::new();
        let img = image::RgbaImage::from_pixel(8, 4, image::Rgba([20, 120, 200, 255]));
        image::DynamicImage::ImageRgba8(img).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        let last = structure(&ses.doc).iter().find(|e| e.title == "Le port").map(|e| e.block).unwrap();
        caret_after(&mut ses, last);
        run(&mut ses, "insert.picture", serde_json::json!({"data": crate::cmd::insert::base64_encode(&png), "width": 80}));
        ses.doc.clone()
    }

    /// The official validator, EPUBCheck (W3C, BSD licence), finds no error and no warning in a
    /// book's EPUB. It runs when `EPUBCHECK_JAR` names `epubcheck.jar` (and `java` is on the
    /// path): `outils/test-epub.ps1` downloads it and sets it. Without it the test only says so.
    #[test]
    fn epubcheck_finds_nothing_wrong() {
        let Some(jar) = std::env::var_os("EPUBCHECK_JAR") else {
            eprintln!("EPUBCHECK_JAR not set: EPUBCheck validation skipped");
            return;
        };
        let dir = std::env::temp_dir().join(format!("lusk-epubcheck-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for (name, doc) in [("livre.epub", rich_book()), ("vide.epub", Document::new())] {
            let path = dir.join(name);
            std::fs::write(&path, write(&doc).unwrap()).unwrap();
            let out = std::process::Command::new("java").arg("-jar").arg(&jar).arg(&path).output().expect("java runs");
            let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
            let bad: Vec<&str> = text.lines().filter(|l| l.starts_with("ERROR") || l.starts_with("FATAL") || l.starts_with("WARNING")).collect();
            assert!(out.status.success() && bad.is_empty(), "EPUBCheck on {name}:\n{text}");
        }
        if std::env::var_os("LUSK_EPUB_KEEP").is_none() {
            let _ = std::fs::remove_dir_all(&dir);
        } else {
            eprintln!("EPUB kept in {}", dir.display());
        }
    }

    /// Footnotes become EPUB 3 note references and asides; lists, tables, links and pictures
    /// come through; every file stays well-formed.
    #[test]
    fn a_rich_book_keeps_its_notes_lists_tables_and_pictures() {
        let bytes = write(&rich_book()).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let names: Vec<String> = zip.file_names().map(str::to_string).collect();
        let mut all = String::new();
        for name in names.iter().filter(|n| n.ends_with(".xhtml")) {
            let xml = entry(&mut zip, name);
            assert!(well_formed(&xml), "{name} is well-formed XML");
            all.push_str(&xml);
        }
        assert!(all.contains("epub:type=\"noteref\"") && all.contains("href=\"#note-1\""), "{all}");
        assert!(all.contains("<aside epub:type=\"footnote\" class=\"note\" id=\"note-1\">"));
        assert!(all.contains("Une note de bas de page."));
        assert!(!all.contains(NOTE_MARK), "no marker left");
        assert!(all.contains("<li>") && all.contains("<table") && all.contains("https://lusk.bzh/?a=1&amp;b=2"));
        assert!(names.iter().any(|n| n.starts_with("OEBPS/images/")));
    }

    #[test]
    fn html_becomes_well_formed_xhtml_with_pictures_as_files() {
        let mut pics = Vec::new();
        let x = xhtmlify("<p>a<br>b&nbsp;c</p><hr><img src=\"data:image/png;base64,iVBORw0KGgo=\" alt=\"x\" width=\"2\">", &mut pics);
        assert_eq!(x, "<p>a<br/>b&#160;c</p><hr/><img src=\"images/image1.png\" alt=\"x\" width=\"2\"/>");
        assert_eq!(pics.len(), 1);
        assert_eq!(pics[0].mime, "image/png");
        // Junk in the data URI: no picture, no crash.
        let x = xhtmlify("<img src=\"data:application/x-evil;base64,AAAA\">", &mut pics);
        assert_eq!(x, "<img src=\"\"/>");
        // Any document exports, even an empty one.
        assert!(write(&Document::new()).is_ok());
    }
}
