use super::exporter::Exporter;
use super::metadata::metadata;
use super::options::{PdfError, PdfOptions};
use super::paint::{clamp_side, ok};
use super::{
    DocLayout, Document, HashMap, HashSet, KrillaError, LayoutCache, LayoutOptions, Page, PageSettings, Size, Transform, WriteError, Writer, krilla,
    layout, write,
};

/// Lay out `doc` and write it as PDF. Without markup the layout is the final text: tracked
/// deletions are left out, as in Word's "No Markup" view. With `comments` each page carries the
/// markup area with the comment balloons.
pub fn export(doc: &Document, opts: &PdfOptions) -> Result<Vec<u8>, PdfError> {
    write::guard(|| {
        let lo = LayoutOptions { hide_deleted: !opts.include_markup, comment_balloons: opts.comments, numeral: opts.numeral, ..Default::default() };
        let lay = layout(doc, &mut LayoutCache::new(), &lo);
        export_layout_inner(doc, &lay, opts)
    })
}

/// Write an existing layout of `doc` as PDF.
pub fn export_layout(doc: &Document, lay: &DocLayout, opts: &PdfOptions) -> Result<Vec<u8>, PdfError> {
    write::guard(|| export_layout_inner(doc, lay, opts))
}

fn export_layout_inner(doc: &Document, lay: &DocLayout, opts: &PdfOptions) -> Result<Vec<u8>, PdfError> {
    let pages: Vec<usize> = match &opts.pages {
        Some(v) => {
            if let Some(bad) = v.iter().find(|p| **p >= lay.pages.len()) {
                return Err(PdfError::BadPage(bad.saturating_add(1)));
            }
            v.clone()
        }
        None => (0..lay.pages.len()).collect(),
    };
    if pages.is_empty() {
        return Err(PdfError::NoPages);
    }
    // krilla subsets fonts only when the document is finished, so one font it can't subset fails
    // the whole export. Draw that font's glyphs as outlines instead and write the document again.
    let mut outlined = HashSet::new();
    loop {
        match write_attempt(doc, lay, opts, &pages, &outlined)? {
            Attempt::Done(bytes) => return Ok(bytes),
            Attempt::BadFont(id, msg) if outlined.len() < MAX_OUTLINED_FONTS && outlined.insert(id) => {
                log::warn!("PDF: {msg}; its text is drawn as outlines");
            }
            Attempt::BadFont(_, msg) => return Err(PdfError::Write(msg)),
        }
    }
}

/// How many unsubsettable fonts we fall back to outlines for before giving up.
const MAX_OUTLINED_FONTS: usize = 32;

/// One try at writing the PDF.
enum Attempt {
    Done(Vec<u8>),
    /// The face with this id could not be embedded (and why).
    BadFont(u32, String),
}

fn write_attempt(doc: &Document, lay: &DocLayout, opts: &PdfOptions, pages: &[usize], outlined: &HashSet<u32>) -> Result<Attempt, PdfError> {
    let settings = krilla::SerializeSettings { compress_content_streams: opts.compress, enable_tagging: opts.tagged, ..Default::default() };
    let language = doc.styles.default_chr.lang.clone().filter(|l| !l.is_empty() && l.is_ascii());
    let mut writer = Writer::new_with_language(settings, metadata(doc, opts), language);
    let pdf = writer.document_mut();

    let mut ex = Exporter {
        doc,
        lay,
        opts,
        out_index: pages.iter().enumerate().map(|(o, p)| (*p, o)).collect(),
        fonts: HashMap::new(),
        font_error: None,
        outlined,
        cmaps: HashMap::new(),
        images: HashMap::new(),
        vectors: HashMap::new(),
        tags: Vec::new(),
        tag_index: HashMap::new(),
        links: Vec::new(),
        bookmarks: std::cell::OnceCell::new(),
    };
    if let Some(o) = ex.outline(pages) {
        pdf.set_outline(o);
    }
    for &pi in pages {
        let Some(page) = lay.pages.get(pi) else { return Err(PdfError::BadPage(pi.saturating_add(1))) };
        let mut fit = Fit::of(page, opts.paper);
        let bleed = if opts.paper.is_none() && ok(opts.bleed) { opts.bleed.clamp(0.0, 72.0) } else { 0.0 };
        if bleed > 0.0 {
            fit = fit.with_bleed(bleed);
        }
        let size = Size::from_wh(clamp_side(fit.paper_w), clamp_side(fit.paper_h)).ok_or(PdfError::NoPages)?;
        let mut settings = PageSettings::new(size);
        // Print production: the finished page (trim box) inside the sheet with its bleed.
        let inset = if bleed > 0.0 {
            bleed
        } else if ok(opts.trim_inset) {
            opts.trim_inset.clamp(0.0, 72.0)
        } else {
            0.0
        };
        if inset > 0.0
            && let (Some(trim), Some(full)) = (
                krilla::geom::Rect::from_xywh(inset, inset, (size.width() - 2.0 * inset).max(1.0), (size.height() - 2.0 * inset).max(1.0)),
                krilla::geom::Rect::from_xywh(0.0, 0.0, size.width(), size.height()),
            )
        {
            settings = settings.with_trim_box(Some(trim)).with_bleed_box(Some(full));
        }
        if opts.page_labels {
            settings = settings.with_page_label(page_label(doc, page));
        }
        let mut kp = pdf.start_page_with(settings);
        let mut s = kp.surface();
        ex.links.clear();
        // A page with a markup area is scaled down to fit its paper with the area beside it; a
        // print on other paper is scaled to it and centred.
        let scaled = fit.k != 1.0 || fit.dx != 0.0 || fit.dy != 0.0;
        if scaled {
            s.push_transform(&Transform::from_row(fit.k, 0.0, 0.0, fit.k, fit.dx, fit.dy));
        }
        // `page_w` is the document page's width: the page colour and watermark stay off the markup area.
        ex.page(&mut s, page, clamp_side(fit.page_w), clamp_side(page.h));
        if scaled {
            s.pop();
        }
        s.finish();
        for a in ex.annotations(fit) {
            kp.add_annotation(a);
        }
        kp.finish();
    }
    if opts.tagged {
        pdf.set_tag_tree(ex.tag_tree());
    }
    if let Some(error) = ex.font_error {
        return Err(error);
    }
    match writer.finish() {
        Ok(bytes) => Ok(Attempt::Done(bytes)),
        Err(WriteError::Serialization(error)) => match *error {
            KrillaError::Font(font, msg) => {
                let bad = ex.fonts.iter().find(|(_, (_, f))| f.as_ref() == Some(&font));
                match bad {
                    Some((id, (face, _))) => Ok(Attempt::BadFont(*id, format!("font {} {} could not be embedded ({msg})", face.family, face.style))),
                    None => Err(PdfError::Write(format!("a font could not be embedded ({msg})"))),
                }
            }
            error => Err(PdfError::Write(format!("{error:?}"))),
        },
        Err(error) => Err(error.into()),
    }
}

/// How a layout page maps onto its PDF page: a page with a markup area (Print Markup) is scaled
/// by `k` to its paper width and centred vertically (`dy`). With [`PdfOptions::paper`] that
/// sheet is then scaled to the chosen paper and centred on it (`dx`, `dy`). Any other page is
/// drawn as it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Fit {
    /// The PDF page.
    paper_w: f32,
    paper_h: f32,
    /// The document page's width in layout points (without the markup area).
    page_w: f32,
    k: f32,
    dx: f32,
    dy: f32,
}

/// The label a PDF viewer shows for `page`: its number in its section's numbering format.
fn page_label(doc: &Document, page: &Page) -> krilla::page::PageLabel {
    use krilla::page::{NumberingStyle, PageLabel};
    use wordcraft_doc::section::NumFormat;
    let format = doc.sections().get(page.section).map(|(_, sp)| sp.page_num_format).unwrap_or(NumFormat::Decimal);
    let style = match format {
        NumFormat::LowerRoman => NumberingStyle::LowerRoman,
        NumFormat::UpperRoman => NumberingStyle::UpperRoman,
        NumFormat::LowerLetter => NumberingStyle::LowerAlpha,
        NumFormat::UpperLetter => NumberingStyle::UpperAlpha,
        _ => NumberingStyle::Arabic,
    };
    PageLabel::new(Some(style), None, std::num::NonZeroU32::new(page.number.max(1)))
}

impl Fit {
    /// The same page on a sheet grown by `bleed` on every side.
    pub(crate) fn with_bleed(self, bleed: f32) -> Fit {
        Fit { paper_w: self.paper_w + 2.0 * bleed, paper_h: self.paper_h + 2.0 * bleed, dx: self.dx + bleed, dy: self.dy + bleed, ..self }
    }

    pub(crate) fn of(page: &Page, paper: Option<(f32, f32)>) -> Fit {
        let paper_w = page.w - page.markup;
        let own = if page.markup > 0.0 && ok(page.markup) && ok(page.w) && paper_w > 1.0 {
            let k = paper_w / page.w;
            Fit { paper_w, paper_h: page.h, page_w: paper_w, k, dx: 0.0, dy: page.h * (1.0 - k) / 2.0 }
        } else {
            Fit { paper_w: page.w, paper_h: page.h, page_w: page.w, k: 1.0, dx: 0.0, dy: 0.0 }
        };
        let Some((w, h)) = paper else { return own };
        let f = wordcraft_geom::fit_to_paper(own.paper_w, own.paper_h, w, h);
        Fit { paper_w: f.paper_w, paper_h: f.paper_h, page_w: own.page_w, k: own.k * f.k, dx: f.dx, dy: own.dy * f.k + f.dy }
    }

    /// A layout point on the PDF page.
    pub(crate) fn point(self, x: f32, y: f32) -> (f32, f32) {
        (x * self.k + self.dx, y * self.k + self.dy)
    }
}
