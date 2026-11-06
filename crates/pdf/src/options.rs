use super::{NumeralMode, WriteError};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum PdfError {
    #[error("the document has no pages to export")]
    NoPages,
    #[error("page {0} does not exist")]
    BadPage(usize),
    #[error("PDF writing failed: {0}")]
    Write(String),
}

impl From<WriteError> for PdfError {
    fn from(error: WriteError) -> Self {
        Self::Write(error.to_string())
    }
}

/// Export options.
#[derive(Clone, Debug)]
pub struct PdfOptions {
    /// Pages to export (0-based layout page indexes, in output order); `None` = all.
    pub pages: Option<Vec<usize>>,
    /// Title / author overrides (default: the document properties).
    pub title: Option<String>,
    pub author: Option<String>,
    /// Write a structure tree (tagged PDF, best effort).
    pub tagged: bool,
    /// Show tracked changes as markup (coloured, underlined / struck through).
    pub include_markup: bool,
    /// Print Markup's comments: each page is scaled down to make room on its right for the
    /// comments in balloons joined to their anchors (Word's "Document showing markup").
    pub comments: bool,
    /// Compress content streams.
    pub compress: bool,
    /// Print on this paper (width, height in points) instead of each page's own size: the page
    /// is scaled to fit and centred, the paper turned to the page's orientation (Scale to paper
    /// size, #2108). `None` keeps the document's page sizes.
    pub paper: Option<(f32, f32)>,
    /// How [`export`] shows digits (the Numeral setting; callers resolve `System`, see
    /// [`NumeralMode::resolve`]). [`export_layout`] keeps the layout it is given.
    pub numeral: NumeralMode,
    /// Bleed added around every page, in points (Lusk Texte's print PDF for a printer): the PDF
    /// page grows by this much on each side, the page's own size is written as its trim box and
    /// the whole sheet as its bleed box. 0 = none. Ignored with [`Self::paper`].
    pub bleed: f32,
    /// The page already includes `trim_inset` points of bleed on each side (a book cover drawn to
    /// its full size): the trim box is written that far inside the page. 0 = none.
    pub trim_inset: f32,
    /// Write page labels (i, ii… for pages numbered in roman numerals), so a PDF viewer shows
    /// the book's own page numbers.
    pub page_labels: bool,
}

impl Default for PdfOptions {
    fn default() -> Self {
        PdfOptions {
            pages: None,
            title: None,
            author: None,
            tagged: true,
            include_markup: false,
            comments: false,
            compress: true,
            paper: None,
            numeral: NumeralMode::Arabic,
            bleed: 0.0,
            trim_inset: 0.0,
            page_labels: false,
        }
    }
}
