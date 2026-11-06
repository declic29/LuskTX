//! Mode Livre (Lusk Texte): the Document | Livre switch in the title bar, the book creation
//! assistant (also the book's settings), the book pane, the contextual « Livre » ribbon tab and
//! the confirmation before an element is deleted. The book itself is the engine's
//! (`wordcraft_engine::book`); everything here runs its commands.

use egui::{Align2, Color32, CornerRadius, Margin, Rect, Sense, Stroke, Ui, pos2, vec2};
use serde_json::{Value, json};
use wordcraft_engine::book::{self, BookSettings, ElementKind, Entry};

use crate::WordApp;
use crate::theme::{Tokens, medium, regular, semibold};

mod checks;
pub use checks::auto_version;

/// What the assistant does when confirmed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WizardMode {
    /// A new book in its own window.
    #[default]
    New,
    /// The open document becomes a book.
    Convert,
    /// The open book's settings.
    Settings,
}

#[derive(Clone, Default)]
struct Wizard {
    open: bool,
    mode: WizardMode,
    s: BookSettings,
    /// The document open when the assistant came up can be converted instead (New only).
    can_convert: bool,
}

/// The book pane's tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PaneTab {
    /// Parts, chapters, front and back matter.
    #[default]
    Plan,
    /// Page thumbnails.
    Pages,
    /// Quality control (phase 5).
    Quality,
    /// Version history (phase 5).
    Versions,
}

impl PaneTab {
    /// From a command parameter: `plan`, `pages`, `qualite`, `versions`.
    pub fn from_id(id: &str) -> PaneTab {
        match id {
            "pages" => PaneTab::Pages,
            "qualite" | "quality" => PaneTab::Quality,
            "versions" => PaneTab::Versions,
            _ => PaneTab::Plan,
        }
    }
}

/// The book pane's transient state: the element being renamed, the one waiting for a delete
/// confirmation, the tab shown.
#[derive(Clone, Default)]
struct PaneState {
    renaming: Option<(usize, String)>,
    confirm_delete: Option<(usize, String, usize)>,
    tab: PaneTab,
}

/// Page thumbnails, made again when the document changes.
#[derive(Clone, Default)]
struct Thumbs {
    rev: u64,
    width: u32,
    tex: std::collections::HashMap<usize, egui::TextureHandle>,
}

/// What is dragged in the book pane: an element index.
#[derive(Clone, Copy, Debug)]
struct Dragged(usize);

/// The book's structure as last computed, with the body it came from.
#[derive(Clone, Default)]
struct Cached {
    key: u64,
    entries: Vec<Entry>,
    total: usize,
}

fn wizard_id(app: &WordApp) -> egui::Id {
    app.window_id("lusk_book_wizard")
}

fn pane_id(app: &WordApp) -> egui::Id {
    app.window_id("lusk_book_pane")
}

/// The fonts offered for a book (any installed font can be typed in).
const FONTS: &[&str] = &["Source Serif 4", "Georgia", "Garamond", "Book Antiqua", "Palatino Linotype", "Cambria", "Constantia", "Times New Roman"];

/// Paper colours for the preview.
const WHITE_PAPER: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
const CREAM_PAPER: Color32 = Color32::from_rgb(0xF6, 0xEF, 0xDC);

/// Open the assistant: `New` (a new book, or the open document made a book), `Convert`, or
/// `Settings` for the open book.
pub fn open_wizard(app: &WordApp, ctx: &egui::Context, mode: WizardMode) {
    let doc = &app.session.doc;
    let s = match mode {
        WizardMode::Settings => book::settings(doc).unwrap_or_default(),
        _ => BookSettings { author: author_default(app), ..BookSettings::default() },
    };
    let has_text = app.has_document() && doc.word_count() > 0 && !book::is_book(doc);
    let w = Wizard { open: true, mode, s, can_convert: mode == WizardMode::New && has_text };
    ctx.data_mut(|d| d.insert_temp(wizard_id(app), w));
}

fn author_default(app: &WordApp) -> String {
    let a = app.session.author.trim();
    if a.is_empty() || a == "Utilisateur" { String::new() } else { a.to_string() }
}

/// The book's structure, recomputed only when the body changed (blocks are shared between
/// versions, so a changed body has different block pointers).
fn structure(app: &WordApp, ctx: &egui::Context) -> Cached {
    let doc = &app.session.doc;
    let mut key: u64 = doc.body.len() as u64;
    for b in &doc.body {
        key = key.wrapping_mul(1_000_003).wrapping_add(std::sync::Arc::as_ptr(b) as usize as u64);
    }
    let id = app.window_id("lusk_book_structure");
    if let Some(c) = ctx.data(|d| d.get_temp::<Cached>(id)).filter(|c| c.key == key) {
        return c;
    }
    let c = Cached { key, entries: book::structure(doc), total: book::total_words(doc) };
    ctx.data_mut(|d| d.insert_temp(id, c.clone()));
    c
}

fn run(app: &mut WordApp, id: &str, params: Value) {
    if let Err(e) = app.run(id, params) {
        app.status(e);
    }
}

/// Show the book pane on one of its tabs.
pub fn show_pane_tab(app: &mut WordApp, ctx: &egui::Context, tab: PaneTab) {
    let id = pane_id(app);
    let mut st = ctx.data(|d| d.get_temp::<PaneState>(id)).unwrap_or_default();
    st.tab = tab;
    if tab == PaneTab::Quality {
        checks::analyse(app, ctx);
    }
    ctx.data_mut(|d| d.insert_temp(id, st));
    if !app.session.view.book_pane {
        run(app, "view.bookPane", json!({"value": true}));
    }
}

/// Is the open document shown in Mode Livre?
pub fn book_mode(app: &WordApp) -> bool {
    app.session.view.book_pane && book::is_book(&app.session.doc)
}

/// The Document | Livre switch, drawn right to left in the title bar.
pub fn mode_switch(app: &mut WordApp, ui: &mut Ui, h: f32) {
    if !app.has_document() {
        return;
    }
    let t = Tokens::get(ui.ctx());
    let on = book_mode(app);
    let font = medium(11.5);
    let labels = ["Document", "Livre"];
    let widths: Vec<f32> = labels.iter().map(|l| ui.painter().layout_no_wrap((*l).to_string(), font.clone(), t.text).size().x + 20.0).collect();
    let total: f32 = widths.iter().sum();
    let (r, _) = ui.allocate_exact_size(vec2(total, h - 4.0), Sense::hover());
    ui.painter().rect(r, CornerRadius::same(6), t.input, Stroke::new(1.0, t.border_strong), egui::StrokeKind::Inside);
    let mut x = r.min.x;
    for (i, label) in labels.iter().enumerate() {
        let w = widths.get(i).copied().unwrap_or(60.0);
        let half = Rect::from_min_size(pos2(x, r.min.y), vec2(w, r.height()));
        x += w;
        let selected = (i == 1) == on;
        let resp = ui.interact(half, ui.scope_id().with(("lusk_mode", i)), Sense::click());
        if selected {
            ui.painter().rect_filled(half.shrink(2.0), CornerRadius::same(5), t.accent);
        } else if resp.hovered() {
            ui.painter().rect_filled(half.shrink(2.0), CornerRadius::same(5), t.hover);
        }
        let color = if selected { t.on_accent } else { t.text };
        ui.painter().text(half.center(), Align2::CENTER_CENTER, *label, font.clone(), color);
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::RadioButton, true, selected, *label));
        let tip =
            if i == 0 { "Mode Document : le traitement de texte habituel" } else { "Mode Livre : organiser, mettre en page et publier un livre" };
        if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() && !selected {
            if i == 0 {
                run(app, "view.bookPane", json!({"value": false}));
            } else if book::is_book(&app.session.doc) {
                run(app, "view.bookPane", json!({"value": true}));
                app.ui.tab = "Livre".into();
            } else {
                let ctx = ui.ctx().clone();
                open_wizard(app, &ctx, WizardMode::New);
            }
        }
    }
}

/// The assistant, the Studio Couverture and the delete confirmation, over the window. Call with
/// the overlays.
pub fn show(app: &mut WordApp, ctx: &egui::Context) {
    wizard(app, ctx);
    studio(app, ctx);
    confirm_delete(app, ctx);
    checks::confirm_version_delete(app, ctx);
}

/// The Studio Couverture's state: the settings being edited and the book's page count.
#[derive(Clone, Default)]
struct Studio {
    open: bool,
    s: BookSettings,
    pages: usize,
}

fn studio_id(app: &WordApp) -> egui::Id {
    app.window_id("lusk_book_studio")
}

/// Open the Studio Couverture for the open book.
pub fn open_studio(app: &mut WordApp, ctx: &egui::Context) {
    let Some(s) = book::settings(&app.session.doc) else { return };
    let pages = app.session.layout().pages.len();
    ctx.data_mut(|d| d.insert_temp(studio_id(app), Studio { open: true, s, pages }));
}

/// The picture chosen for the front cover (from the file dialog).
pub fn cover_image_picked(app: &mut WordApp, ctx: &egui::Context, path: String) {
    let id = studio_id(app);
    if let Some(mut st) = ctx.data(|d| d.get_temp::<Studio>(id)) {
        st.s.cover_image = path;
        ctx.data_mut(|d| d.insert_temp(id, st));
    }
}

/// Ask where to write the book as a PDF for the printer (`print`), a PDF for reading
/// (`reading`) or an EPUB (`epub`).
pub fn ask_export(app: &mut WordApp, what: &str) {
    let s = book::settings(&app.session.doc).unwrap_or_default();
    let stem: String = if s.title.is_empty() {
        "livre".into()
    } else {
        s.title.chars().filter(|c| !matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')).collect()
    };
    let name = match what {
        "epub" => format!("{stem}.epub"),
        "reading" => format!("{stem} - lecture.pdf"),
        _ => format!("{stem} - impression.pdf"),
    };
    let _ = app.ask_file(
        crate::file_dialogs::FileDialogRequest::Save { name, dir: None },
        crate::file_dialogs::AfterPick::BookExport { what: what.to_string() },
    );
}

fn rgb_of(hex: &str, fallback: [u8; 3]) -> [u8; 3] {
    wordcraft_doc::props::Rgb::parse(hex).map_or(fallback, |c| [c.0, c.1, c.2])
}

fn hex_of(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

fn studio(app: &mut WordApp, ctx: &egui::Context) {
    let id = studio_id(app);
    let Some(mut st) = ctx.data(|d| d.get_temp::<Studio>(id)).filter(|w| w.open) else { return };
    let t = Tokens::get(ctx);
    let (mut close, mut save, mut create, mut pick) = (false, false, false, false);
    let modal = egui::Modal::new(id.with("modal")).frame(egui::Frame::NONE.fill(t.panel).corner_radius(CornerRadius::same(10)).inner_margin(Margin::same(22))).show(ctx, |ui| {
        ui.set_width(860.0_f32.min(ctx.content_rect().width() - 80.0));
        ui.label(egui::RichText::new("Studio Couverture").font(semibold(19.0)).color(t.text));
        ui.label(
            egui::RichText::new("Première, dos et quatrième de couverture, avec le fond perdu de votre imprimeur. La couverture s'ouvre ensuite comme un document que vous pouvez retoucher et exporter en PDF.")
                .font(regular(12.0))
                .color(t.text_dim),
        );
        ui.add_space(8.0);
        let max_h = (ctx.content_rect().height() - 230.0).max(240.0);
        ui.horizontal_top(|ui| {
            egui::ScrollArea::vertical().id_salt("lusk_cover_form").max_height(max_h).show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(430.0);
                    section_title(ui, &t, "Spécifications de l'imprimeur");
                    ui.label(egui::RichText::new("Recopiez les valeurs de la fiche technique de votre imprimeur : rien n'est deviné.").font(regular(11.5)).color(t.text_dim));
                    field(ui, "Imprimeur", |ui| ui.add(egui::TextEdit::singleline(&mut st.s.printer).desired_width(230.0).hint_text("Nom (facultatif)")));
                    field(ui, "Fond perdu", |ui| mm_value(ui, &mut st.s.bleed_mm, 0.0, 10.0));
                    field(ui, "Épaisseur d'une feuille", |ui| {
                        ui.add(egui::DragValue::new(&mut st.s.sheet_mm).range(0.0..=1.0).speed(0.001).fixed_decimals(3).suffix(" mm"));
                    });
                    let spine = st.s.spine_mm(st.pages);
                    let info = match spine {
                        Some(sp) => format!("{} pages, soit {} feuilles : dos de {:.2} mm", st.pages, st.pages.div_ceil(2), sp),
                        None => format!("{} pages : renseignez l'épaisseur d'une feuille pour calculer le dos", st.pages),
                    };
                    ui.label(egui::RichText::new(info).font(medium(12.0)).color(if spine.is_some() { t.text } else { t.text_dim }));
                    section_title(ui, &t, "Couleurs");
                    field(ui, "Fond", |ui| {
                        let mut c = rgb_of(&st.s.cover_bg, [0x1F, 0x3A, 0x4D]);
                        if ui.color_edit_button_srgb(&mut c).changed() {
                            st.s.cover_bg = hex_of(c);
                        }
                    });
                    field(ui, "Texte", |ui| {
                        let mut c = rgb_of(&st.s.cover_fg, [0xFF, 0xFF, 0xFF]);
                        if ui.color_edit_button_srgb(&mut c).changed() {
                            st.s.cover_fg = hex_of(c);
                        }
                    });
                    section_title(ui, &t, "Première de couverture");
                    field(ui, "Titre", |ui| ui.add(egui::TextEdit::singleline(&mut st.s.title).desired_width(230.0)));
                    field(ui, "Sous-titre", |ui| ui.add(egui::TextEdit::singleline(&mut st.s.subtitle).desired_width(230.0)));
                    field(ui, "Auteur", |ui| ui.add(egui::TextEdit::singleline(&mut st.s.author).desired_width(230.0)));
                    field(ui, "Image", |ui| {
                        if ui.button("Choisir une image…").clicked() {
                            pick = true;
                        }
                        if !st.s.cover_image.is_empty() && ui.button("Retirer").clicked() {
                            st.s.cover_image.clear();
                        }
                    });
                    if !st.s.cover_image.is_empty() {
                        let name = std::path::Path::new(&st.s.cover_image).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                        ui.label(egui::RichText::new(name).font(regular(11.5)).color(t.text_dim));
                    }
                    section_title(ui, &t, "Quatrième de couverture");
                    ui.add(egui::TextEdit::multiline(&mut st.s.back_text).desired_width(420.0).desired_rows(6).hint_text("Résumé du livre, quelques mots sur l'auteur…"));
                    field(ui, "ISBN", |ui| ui.add(egui::TextEdit::singleline(&mut st.s.isbn).desired_width(180.0).hint_text("978-…")));
                    ui.checkbox(&mut st.s.barcode_space, "Réserver l'emplacement du code-barres");
                });
            });
            ui.add_space(18.0);
            ui.vertical(|ui| cover_preview(ui, &t, &st.s, st.pages));
        });
        ui.add_space(14.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let primary = egui::Button::new(egui::RichText::new("Créer la couverture").font(semibold(13.0)).color(t.on_accent)).fill(t.accent).min_size(vec2(170.0, 32.0));
            if ui.add(primary).on_hover_text("Enregistre ces réglages et ouvre la couverture dans une nouvelle fenêtre").clicked() {
                create = true;
            }
            if ui.add(egui::Button::new("Enregistrer").min_size(vec2(110.0, 32.0))).clicked() {
                save = true;
            }
            if ui.add(egui::Button::new("Annuler").min_size(vec2(100.0, 32.0))).clicked() {
                close = true;
            }
        });
    });
    if modal.should_close() {
        close = true;
    }
    if save || create {
        let settings = serde_json::to_value(st.s.clone().sanitized()).unwrap_or(json!({}));
        run(app, "book.settings", json!({"settings": settings}));
        if create {
            run(app, "book.cover", json!({}));
        }
        close = true;
    }
    if close {
        st.open = false;
    }
    ctx.data_mut(|d| d.insert_temp(id, st));
    if pick {
        let _ = app.ask_file(crate::file_dialogs::FileDialogRequest::Open { purpose: "picture".into() }, crate::file_dialogs::AfterPick::CoverImage);
    }
}

/// The full cover drawn to scale: back, spine and front, the bleed outside the dashed trim.
fn cover_preview(ui: &mut Ui, t: &Tokens, s: &BookSettings, pages: usize) {
    let s = s.clone().sanitized();
    let spine = s.spine_mm(pages).unwrap_or(0.0);
    let total_w = 2.0 * s.page_w_mm + spine + 2.0 * s.bleed_mm;
    let total_h = s.page_h_mm + 2.0 * s.bleed_mm;
    let avail_w = 360.0;
    let k = (avail_w / total_w).min(300.0 / total_h);
    let (r, _) = ui.allocate_exact_size(vec2(avail_w, total_h * k + 48.0), Sense::hover());
    ui.painter().text(pos2(r.min.x, r.min.y), Align2::LEFT_TOP, "Aperçu de la couverture", semibold(12.5), t.text);
    let origin = pos2(r.min.x + (avail_w - total_w * k) / 2.0, r.min.y + 22.0);
    let sheet = Rect::from_min_size(origin, vec2(total_w * k, total_h * k));
    let bg = rgb_of(&s.cover_bg, [0x1F, 0x3A, 0x4D]);
    let fg = rgb_of(&s.cover_fg, [0xFF, 0xFF, 0xFF]);
    let (bg, fg) = (Color32::from_rgb(bg[0], bg[1], bg[2]), Color32::from_rgb(fg[0], fg[1], fg[2]));
    ui.painter().rect_filled(sheet, CornerRadius::ZERO, bg);
    let b = s.bleed_mm * k;
    let trim = sheet.shrink(b);
    let back = Rect::from_min_size(trim.min, vec2(s.page_w_mm * k, trim.height()));
    let spine_r = Rect::from_min_size(pos2(back.max.x, trim.min.y), vec2(spine * k, trim.height()));
    let front = Rect::from_min_max(pos2(spine_r.max.x, trim.min.y), trim.max);
    let guide = Stroke::new(1.0, fg.gamma_multiply(0.55));
    if b > 0.5 {
        for (a, z) in [
            (trim.left_top(), trim.right_top()),
            (trim.right_top(), trim.right_bottom()),
            (trim.right_bottom(), trim.left_bottom()),
            (trim.left_bottom(), trim.left_top()),
        ] {
            ui.painter().add(egui::Shape::dashed_line(&[a, z], guide, 4.0, 3.0));
        }
    }
    if spine_r.width() > 0.5 {
        ui.painter().line_segment([spine_r.left_top(), spine_r.left_bottom()], guide);
        ui.painter().line_segment([spine_r.right_top(), spine_r.right_bottom()], guide);
    }
    // Front: author, title, picture's place.
    let title = if s.title.is_empty() { "Titre du livre" } else { &s.title };
    let author = if s.author.is_empty() { "Auteur" } else { &s.author };
    ui.painter().text(pos2(front.center().x, front.min.y + front.height() * 0.08), Align2::CENTER_TOP, author, regular(8.5), fg);
    ui.painter().text(pos2(front.center().x, front.min.y + front.height() * 0.2), Align2::CENTER_TOP, title, semibold(13.0), fg);
    if !s.subtitle.is_empty() {
        ui.painter().text(pos2(front.center().x, front.min.y + front.height() * 0.3), Align2::CENTER_TOP, &s.subtitle, regular(9.0), fg);
    }
    if !s.cover_image.is_empty() {
        let img =
            Rect::from_center_size(pos2(front.center().x, front.min.y + front.height() * 0.62), vec2(front.width() * 0.7, front.height() * 0.35));
        ui.painter().rect_stroke(img, CornerRadius::same(2), Stroke::new(1.0, fg.gamma_multiply(0.7)), egui::StrokeKind::Inside);
        ui.painter().text(img.center(), Align2::CENTER_CENTER, "image", regular(9.0), fg.gamma_multiply(0.8));
    }
    // Back: lines of text, the ISBN and the bar code's white box.
    let mut y = back.min.y + back.height() * 0.1;
    let lines = if s.back_text.is_empty() { 6 } else { (s.back_text.chars().count() / 60).clamp(2, 14) };
    for _ in 0..lines {
        ui.painter().line_segment(
            [pos2(back.min.x + back.width() * 0.12, y), pos2(back.max.x - back.width() * 0.12, y)],
            Stroke::new(1.0, fg.gamma_multiply(0.6)),
        );
        y += 6.0;
    }
    if s.barcode_space {
        let code = Rect::from_min_size(
            pos2(back.max.x - back.width() * 0.12 - 50.0 * k, back.max.y - back.height() * 0.1 - 28.0 * k),
            vec2(50.0 * k, 28.0 * k),
        );
        ui.painter().rect_filled(code, CornerRadius::ZERO, Color32::WHITE);
    }
    // Spine text, bottom to top.
    if spine_r.width() > 3.0 {
        let text = format!("{author}  —  {title}");
        let galley = ui.painter().layout_no_wrap(text, regular((spine_r.width() * 0.55).clamp(5.0, 9.0)), fg);
        let size = galley.size();
        let pos = pos2(spine_r.center().x - size.y / 2.0, spine_r.center().y + size.x / 2.0);
        ui.painter().add(egui::epaint::TextShape::new(pos, galley, fg).with_angle(-std::f32::consts::FRAC_PI_2));
    }
    let caption = format!(
        "{:.1} × {:.1} mm{} · dos {}",
        total_w,
        total_h,
        if s.bleed_mm > 0.0 { format!(" (fond perdu {:.1} mm inclus)", s.bleed_mm) } else { String::new() },
        s.spine_mm(pages).map_or("non calculé".to_string(), |v| format!("{v:.2} mm"))
    );
    ui.painter().text(pos2(r.min.x, sheet.max.y + 8.0), Align2::LEFT_TOP, caption, regular(11.0), t.text_dim);
}

fn section_title(ui: &mut Ui, t: &Tokens, text: &str) {
    ui.add_space(10.0);
    ui.label(egui::RichText::new(text).font(semibold(13.0)).color(t.text));
    ui.add_space(4.0);
}

fn field<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        let t = Tokens::get(ui.ctx());
        let (r, _) = ui.allocate_exact_size(vec2(150.0, 22.0), Sense::hover());
        ui.painter().text(pos2(r.min.x, r.center().y), Align2::LEFT_CENTER, label, regular(12.0), t.text_dim);
        add(ui)
    })
    .inner
}

fn mm_value(ui: &mut Ui, v: &mut f32, lo: f32, hi: f32) {
    ui.add(egui::DragValue::new(v).range(lo..=hi).speed(0.5).fixed_decimals(1).suffix(" mm"));
}

fn wizard(app: &mut WordApp, ctx: &egui::Context) {
    let id = wizard_id(app);
    let Some(mut w) = ctx.data(|d| d.get_temp::<Wizard>(id)).filter(|w| w.open) else { return };
    let t = Tokens::get(ctx);
    let mut close = false;
    let mut confirm = false;
    let title = match w.mode {
        WizardMode::New => "Nouveau livre",
        WizardMode::Convert => "Transformer ce document en livre",
        WizardMode::Settings => "Réglages du livre",
    };
    let modal = egui::Modal::new(id.with("modal"))
        .frame(egui::Frame::NONE.fill(t.panel).corner_radius(CornerRadius::same(10)).inner_margin(Margin::same(22)))
        .show(ctx, |ui| {
            ui.set_width(820.0_f32.min(ctx.content_rect().width() - 80.0));
            ui.label(egui::RichText::new(title).font(semibold(19.0)).color(t.text));
            ui.label(
                egui::RichText::new("Tous ces réglages restent modifiables à tout moment (onglet Livre › Réglages du livre).")
                    .font(regular(12.0))
                    .color(t.text_dim),
            );
            if w.can_convert {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.radio_value(&mut w.mode, WizardMode::New, "Créer un nouveau livre");
                    ui.radio_value(&mut w.mode, WizardMode::Convert, "Transformer le document ouvert en livre (ses titres deviennent les chapitres)");
                });
            }
            ui.add_space(8.0);
            let max_h = (ctx.content_rect().height() - 220.0).max(240.0);
            ui.horizontal_top(|ui| {
                egui::ScrollArea::vertical().id_salt("lusk_book_form").max_height(max_h).min_scrolled_height(max_h.min(520.0)).show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(470.0);
                        if w.mode != WizardMode::Settings || !w.s.profile.is_empty() {
                            section_title(ui, &t, "Modèle éditorial");
                        }
                        // Three presets a row (a wrapping row would stretch the assistant instead).
                        for row in book::PRESETS.chunks(3) {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                                for p in row {
                                    let selected = w.s.profile == p.id;
                                    let resp = egui::Frame::NONE
                                        .fill(if selected { t.accent.gamma_multiply(0.15) } else { t.input })
                                        .stroke(Stroke::new(if selected { 2.0 } else { 1.0 }, if selected { t.accent } else { t.border }))
                                        .corner_radius(CornerRadius::same(8))
                                        .inner_margin(Margin::symmetric(10, 8))
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                ui.set_width(126.0);
                                                ui.set_min_height(54.0);
                                                ui.label(egui::RichText::new(p.label).font(semibold(12.5)).color(t.text));
                                                ui.label(egui::RichText::new(p.description).font(regular(11.0)).color(t.text_dim));
                                            });
                                        })
                                        .response
                                        .interact(Sense::click())
                                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                                    if resp.clicked() {
                                        let keep = (
                                            w.s.title.clone(),
                                            w.s.subtitle.clone(),
                                            w.s.author.clone(),
                                            w.s.cover,
                                            w.s.header_parts.clone(),
                                            w.s.lang.clone(),
                                        );
                                        w.s = p.settings();
                                        (w.s.title, w.s.subtitle, w.s.author, w.s.cover, w.s.header_parts, w.s.lang) = keep;
                                    }
                                }
                            });
                            ui.add_space(8.0);
                        }
                        section_title(ui, &t, "Le livre");
                        field(ui, "Titre", |ui| ui.add(egui::TextEdit::singleline(&mut w.s.title).desired_width(280.0).hint_text("Titre du livre")));
                        field(ui, "Sous-titre (facultatif)", |ui| ui.add(egui::TextEdit::singleline(&mut w.s.subtitle).desired_width(280.0)));
                        field(ui, "Auteur", |ui| ui.add(egui::TextEdit::singleline(&mut w.s.author).desired_width(280.0)));
                        field(ui, "Type d'ouvrage", |ui| {
                            let current = book::KINDS.iter().find(|k| k.0 == w.s.kind).map_or("Autre", |k| k.1);
                            craft_ui::ComboBox::from_id_salt("lusk_book_kind").selected_text(current).width(200.0).show_ui(ui, |ui| {
                                for (kid, label) in book::KINDS {
                                    ui.selectable_value(&mut w.s.kind, (*kid).to_string(), *label);
                                }
                            });
                        });
                        field(ui, "Langue du livre", |ui| {
                            let current = book::LANGUAGES.iter().find(|l| l.0 == w.s.lang).map_or("Français (France)", |l| l.1);
                            craft_ui::ComboBox::from_id_salt("lusk_book_lang")
                                .selected_text(current)
                                .width(200.0)
                                .show_ui(ui, |ui| {
                                    for (tag, label) in book::LANGUAGES {
                                        ui.selectable_value(&mut w.s.lang, (*tag).to_string(), *label);
                                    }
                                })
                                .response
                                .on_hover_text("Orthographe, césure, contrôle de la typographie et langue de l'EPUB");
                        });
                        section_title(ui, &t, "Format et papier");
                        field(ui, "Format", |ui| {
                            let current = book::FORMATS.iter().find(|f| f.0 == w.s.format).map_or("Personnalisé", |f| f.1);
                            craft_ui::ComboBox::from_id_salt("lusk_book_format").selected_text(current).width(240.0).show_ui(ui, |ui| {
                                for (fid, label, fw, fh) in book::FORMATS {
                                    if ui.selectable_label(w.s.format == *fid, *label).clicked() {
                                        w.s.format = (*fid).to_string();
                                        w.s.page_w_mm = *fw;
                                        w.s.page_h_mm = *fh;
                                    }
                                }
                                if ui.selectable_label(w.s.format == "custom", "Personnalisé…").clicked() {
                                    w.s.format = "custom".into();
                                }
                            });
                        });
                        if w.s.format == "custom" {
                            field(ui, "Largeur × hauteur", |ui| {
                                mm_value(ui, &mut w.s.page_w_mm, 50.0, 600.0);
                                ui.label("×");
                                mm_value(ui, &mut w.s.page_h_mm, 50.0, 600.0);
                            });
                        }
                        field(ui, "Papier", |ui| {
                            ui.radio_value(&mut w.s.paper, "blanc".to_string(), "Blanc");
                            ui.radio_value(&mut w.s.paper, "creme".to_string(), "Crème");
                        });
                        section_title(ui, &t, "Texte");
                        field(ui, "Police principale", |ui| {
                            craft_ui::ComboBox::from_id_salt("lusk_book_font").selected_text(w.s.font.clone()).width(200.0).show_ui(ui, |ui| {
                                for f in FONTS {
                                    ui.selectable_value(&mut w.s.font, (*f).to_string(), *f);
                                }
                            });
                        });
                        field(ui, "Taille des caractères", |ui| {
                            ui.add(egui::DragValue::new(&mut w.s.size).range(6.0..=30.0).speed(0.1).fixed_decimals(1).suffix(" pt"));
                        });
                        field(ui, "Interlignage", |ui| {
                            ui.add(egui::DragValue::new(&mut w.s.line_spacing).range(1.0..=3.0).speed(0.01).fixed_decimals(2).suffix(" ×"));
                        });
                        field(ui, "Alignement", |ui| {
                            ui.radio_value(&mut w.s.align, "justify".to_string(), "Justifié");
                            ui.radio_value(&mut w.s.align, "left".to_string(), "Aligné à gauche");
                        });
                        field(ui, "Retrait de première ligne", |ui| mm_value(ui, &mut w.s.indent_mm, 0.0, 30.0));
                        section_title(ui, &t, "Marges (miroir : l'intérieur est côté reliure)");
                        field(ui, "Intérieure", |ui| mm_value(ui, &mut w.s.inner_mm, 3.0, 60.0));
                        field(ui, "Extérieure", |ui| mm_value(ui, &mut w.s.outer_mm, 3.0, 60.0));
                        field(ui, "Haut", |ui| mm_value(ui, &mut w.s.top_mm, 3.0, 60.0));
                        field(ui, "Bas", |ui| mm_value(ui, &mut w.s.bottom_mm, 3.0, 60.0));
                        section_title(ui, &t, "Mise en page");
                        field(ui, "Titres courants", |ui| {
                            let current = book::RUNNING_HEADS.iter().find(|r| r.0 == w.s.running_heads).map_or("", |r| r.1);
                            craft_ui::ComboBox::from_id_salt("lusk_book_heads").selected_text(current).width(290.0).show_ui(ui, |ui| {
                                for (rid, label) in book::RUNNING_HEADS {
                                    ui.selectable_value(&mut w.s.running_heads, (*rid).to_string(), *label);
                                }
                            });
                        });
                        field(ui, "Numéro de page", |ui| {
                            let current = book::FOLIOS.iter().find(|f| f.0 == w.s.folio).map_or("", |f| f.1);
                            craft_ui::ComboBox::from_id_salt("lusk_book_folio").selected_text(current).width(200.0).show_ui(ui, |ui| {
                                for (fid, label) in book::FOLIOS {
                                    ui.selectable_value(&mut w.s.folio, (*fid).to_string(), *label);
                                }
                            });
                        });
                        field(ui, "Lettrines", |ui| ui.checkbox(&mut w.s.drop_caps, "Une lettrine ouvre chaque chapitre"));
                        section_title(ui, &t, "Couverture");
                        ui.checkbox(&mut w.s.cover, "Le livre aura une couverture").on_hover_text(
                            "Choix enregistré avec le livre ; la couverture se conçoit dans le Studio Couverture (étape suivante du Mode Livre).",
                        );
                    })
                });
                ui.add_space(18.0);
                ui.vertical(|ui| {
                    preview(ui, &t, &w.s);
                });
            });
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let action = match w.mode {
                        WizardMode::New => "Créer le livre",
                        WizardMode::Convert => "Transformer en livre",
                        WizardMode::Settings => "Appliquer",
                    };
                    let primary = egui::Button::new(egui::RichText::new(action).font(semibold(13.0)).color(t.on_accent))
                        .fill(t.accent)
                        .min_size(vec2(150.0, 32.0));
                    if ui.add(primary).clicked() {
                        confirm = true;
                    }
                    if ui.add(egui::Button::new("Annuler").min_size(vec2(100.0, 32.0))).clicked() {
                        close = true;
                    }
                });
            });
        });
    if modal.should_close() {
        close = true;
    }
    if confirm {
        let settings = serde_json::to_value(w.s.clone().sanitized()).unwrap_or(json!({}));
        match w.mode {
            WizardMode::New => {
                app.ui.backstage = false;
                run(app, "file.new", json!({"template": "book", "book": settings}));
            }
            WizardMode::Convert => run(app, "book.convert", json!({"settings": settings})),
            WizardMode::Settings => run(app, "book.settings", json!({"settings": settings})),
        }
        if book_mode(app) {
            app.ui.tab = "Livre".into();
        }
        close = true;
    }
    if close {
        w.open = false;
    }
    ctx.data_mut(|d| d.insert_temp(id, w));
}

/// A double page drawn to scale: the paper, the mirrored margins and lines of text.
fn preview(ui: &mut Ui, t: &Tokens, s: &BookSettings) {
    let s = s.clone().sanitized();
    let avail = vec2(290.0, 300.0);
    let page_ratio = s.page_w_mm / s.page_h_mm;
    let ph = (avail.x / 2.0 / page_ratio).min(avail.y - 40.0);
    let pw = ph * page_ratio;
    let (r, _) = ui.allocate_exact_size(vec2(avail.x, ph + 46.0), Sense::hover());
    ui.painter().text(pos2(r.min.x, r.min.y), Align2::LEFT_TOP, "Aperçu de la double page", semibold(12.5), t.text);
    let top = r.min.y + 22.0;
    let cx = r.center().x;
    let paper = if s.paper == "blanc" { WHITE_PAPER } else { CREAM_PAPER };
    let k = pw / s.page_w_mm;
    for left in [true, false] {
        let page = if left { Rect::from_min_size(pos2(cx - pw, top), vec2(pw, ph)) } else { Rect::from_min_size(pos2(cx, top), vec2(pw, ph)) };
        ui.painter().rect(page, CornerRadius::ZERO, paper, Stroke::new(1.0, Color32::from_gray(150)), egui::StrokeKind::Inside);
        // The inside margin is on the spine side: right on a left page, left on a right page.
        let (ml, mr) = if left { (s.outer_mm * k, s.inner_mm * k) } else { (s.inner_mm * k, s.outer_mm * k) };
        let text = Rect::from_min_max(pos2(page.min.x + ml, page.min.y + s.top_mm * k), pos2(page.max.x - mr, page.max.y - s.bottom_mm * k));
        ui.painter().rect_stroke(
            text,
            CornerRadius::ZERO,
            Stroke::new(0.5, Color32::from_rgba_unmultiplied(29, 156, 155, 120)),
            egui::StrokeKind::Inside,
        );
        // Lines of text: the type size and line spacing to scale, at least a hairline apart.
        let line = (s.size * s.line_spacing * 25.4 / 72.0 * k).max(1.6);
        let indent = s.indent_mm * k;
        let mut y = text.min.y + line * 0.8;
        let mut n = 0usize;
        while y < text.max.y && n < 400 {
            let paragraph_start = n.is_multiple_of(7);
            let last_of_paragraph = n % 7 == 6;
            let x0 = text.min.x + if paragraph_start { indent } else { 0.0 };
            let x1 = if last_of_paragraph {
                text.min.x + text.width() * 0.55
            } else if s.align == "left" {
                text.max.x - text.width() * (0.04 + 0.08 * ((n * 37 % 5) as f32 / 5.0))
            } else {
                text.max.x
            };
            ui.painter().line_segment([pos2(x0, y), pos2(x1.max(x0), y)], Stroke::new((line * 0.35).clamp(0.4, 1.4), Color32::from_gray(120)));
            y += line;
            n += 1;
        }
        // The running head and the folio where they will print.
        let mark = Color32::from_rgb(29, 156, 155);
        if matches!(s.running_heads.as_str(), "auteurTitre" | "titreChapitre") {
            let y = page.min.y + s.top_mm * k * 0.5;
            ui.painter().line_segment(
                [pos2(text.center().x - text.width() * 0.18, y), pos2(text.center().x + text.width() * 0.18, y)],
                Stroke::new(1.5, mark),
            );
        }
        let outer_x = if left { text.min.x + 3.0 } else { text.max.x - 3.0 };
        let folio = match s.folio.as_str() {
            "basCentre" => Some(pos2(text.center().x, page.max.y - s.bottom_mm * k * 0.5)),
            "basExterieur" => Some(pos2(outer_x, page.max.y - s.bottom_mm * k * 0.5)),
            "hautExterieur" => Some(pos2(outer_x, page.min.y + s.top_mm * k * 0.5)),
            _ => None,
        };
        if let Some(c) = folio {
            ui.painter().rect_filled(Rect::from_center_size(c, vec2(5.0, 4.0)), CornerRadius::same(1), mark);
        }
    }
    // The spine.
    ui.painter().line_segment([pos2(cx, top), pos2(cx, top + ph)], Stroke::new(1.5, Color32::from_gray(110)));
    let format =
        book::FORMATS.iter().find(|f| f.0 == s.format).map_or_else(|| format!("{:.0} × {:.0} mm", s.page_w_mm, s.page_h_mm), |f| f.1.to_string());
    let caption = format!("{format} · {} {:.1} pt · interligne {:.2}", s.font, s.size, s.line_spacing);
    ui.painter().text(pos2(r.min.x, top + ph + 8.0), Align2::LEFT_TOP, caption, regular(11.0), t.text_dim);
}

fn confirm_delete(app: &mut WordApp, ctx: &egui::Context) {
    let id = pane_id(app);
    let mut st = ctx.data(|d| d.get_temp::<PaneState>(id)).unwrap_or_default();
    let Some((index, title, words)) = st.confirm_delete.clone() else { return };
    let t = Tokens::get(ctx);
    let mut done = false;
    let modal = egui::Modal::new(id.with("confirm"))
        .frame(egui::Frame::NONE.fill(t.panel).corner_radius(CornerRadius::same(10)).inner_margin(Margin::same(20)))
        .show(ctx, |ui| {
            ui.set_width(420.0);
            ui.label(egui::RichText::new("Supprimer cet élément ?").font(semibold(16.0)).color(t.text));
            ui.add_space(6.0);
            let words = if words == 1 { "1 mot".to_string() } else { format!("{words} mots") };
            ui.label(egui::RichText::new(format!("« {title} » et tout son contenu ({words}) seront supprimés.")).font(regular(13.0)).color(t.text));
            ui.label(egui::RichText::new("Vous pourrez annuler avec Ctrl+Z.").font(regular(12.0)).color(t.text_dim));
            ui.add_space(14.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let danger = egui::Button::new(egui::RichText::new("Supprimer").font(semibold(13.0)).color(Color32::WHITE))
                    .fill(Color32::from_rgb(0xC4, 0x2B, 0x1C));
                if ui.add(danger.min_size(vec2(110.0, 30.0))).clicked() {
                    run(app, "book.delete", json!({"index": index}));
                    done = true;
                }
                if ui.add(egui::Button::new("Annuler").min_size(vec2(90.0, 30.0))).clicked() {
                    done = true;
                }
            });
        });
    if modal.should_close() {
        done = true;
    }
    if done {
        st.confirm_delete = None;
        ctx.data_mut(|d| d.insert_temp(id, st));
    }
}

/// How deep an element sits in the pane's tree: parts at 0, their chapters at 1, sub-chapters one
/// level under their chapter; front and back matter at 0.
fn depths(entries: &[Entry]) -> Vec<usize> {
    let mut in_part = false;
    entries
        .iter()
        .map(|e| match e.kind {
            ElementKind::Part => {
                in_part = true;
                0
            }
            ElementKind::Chapter => usize::from(in_part),
            ElementKind::Subchapter => usize::from(in_part) + 1,
            _ => {
                in_part = false;
                0
            }
        })
        .collect()
}

/// The element just before `i` of the same rank, under the same parent.
fn previous_sibling(entries: &[Entry], i: usize) -> Option<usize> {
    let rank = entries.get(i)?.rank;
    for k in (0..i).rev() {
        let e = entries.get(k)?;
        if e.rank < rank {
            return None;
        }
        if e.rank == rank {
            return Some(k);
        }
    }
    None
}

/// The element just after `i` (and what it holds) of the same rank, under the same parent.
fn next_sibling(entries: &[Entry], i: usize) -> Option<usize> {
    let me = entries.get(i)?;
    for (k, e) in entries.iter().enumerate().skip(i + 1) {
        if e.block < me.end {
            continue;
        }
        if e.rank < me.rank {
            return None;
        }
        if e.rank == me.rank {
            return Some(k);
        }
    }
    None
}

/// The book pane: the book's elements as a tree, their word counts and the total; click to go
/// there, double-click to rename, drag to move, right-click for more.
pub fn pane(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let ctx = ui.ctx().clone();
    if !book::is_book(&app.session.doc) {
        ui.label(egui::RichText::new("Ce document n'est pas encore un livre.").font(regular(12.5)).color(t.text));
        ui.add_space(8.0);
        if ui.button("Transformer en livre…").clicked() {
            open_wizard(app, &ctx, WizardMode::Convert);
        }
        return;
    }
    let settings = book::settings(&app.session.doc).unwrap_or_default();
    let cached = structure(app, &ctx);
    let entries = cached.entries;
    let title = if settings.title.is_empty() { "Livre sans titre".to_string() } else { settings.title.clone() };
    ui.label(egui::RichText::new(title).font(semibold(15.0)).color(t.text));
    let chapters = entries.iter().filter(|e| e.kind == ElementKind::Chapter).count();
    ui.label(
        egui::RichText::new(format!("{} mots · {} chapitre{}", group_digits(cached.total), chapters, if chapters > 1 { "s" } else { "" }))
            .font(regular(11.5))
            .color(t.text_dim),
    );
    ui.add_space(6.0);
    let id = pane_id(app);
    let mut st = ctx.data(|d| d.get_temp::<PaneState>(id)).unwrap_or_default();
    let was = st.tab;
    ui.horizontal(|ui| {
        let mut tab = |ui: &mut Ui, which: PaneTab, label: &str, tip: &str| {
            if ui.selectable_label(st.tab == which, label).on_hover_text(tip).clicked() {
                st.tab = which;
            }
        };
        tab(ui, PaneTab::Plan, "Plan", "Les parties, chapitres et pages liminaires");
        tab(ui, PaneTab::Pages, "Pages", "Les pages du livre en miniatures, par doubles pages");
        tab(ui, PaneTab::Quality, "Qualité", "Contrôle qualité : ce qui gêne à l'impression ou à la lecture");
        if app.services.versions_dir.is_some() {
            tab(ui, PaneTab::Versions, "Versions", "Historique des versions du livre, sur cet ordinateur");
        }
    });
    ctx.data_mut(|d| d.insert_temp(id, st.clone()));
    ui.separator();
    match st.tab {
        PaneTab::Pages => return pages_tab(app, ui),
        PaneTab::Quality => {
            if was != PaneTab::Quality && !checks::has_results(app, &ctx) {
                checks::analyse(app, &ctx);
            }
            return checks::quality_tab(app, ui);
        }
        PaneTab::Versions => return checks::versions_tab(app, ui),
        PaneTab::Plan => {}
    }
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
        if ui.button("+ Chapitre").on_hover_text("Ajouter un chapitre à la fin du corps du livre").clicked() {
            run(app, "book.insert", json!({"kind": "chapter"}));
        }
        if ui.button("+ Partie").clicked() {
            run(app, "book.insert", json!({"kind": "part"}));
        }
        if ui.button("+ Sous-chapitre").on_hover_text("Dans le chapitre où se trouve le curseur").clicked() {
            run(app, "book.insert", json!({"kind": "subchapter"}));
        }
        ui.menu_button("Insérer ▾", |ui| {
            for kind in ElementKind::ALL.into_iter().filter(|k| k.is_front() || k.is_back()) {
                if ui.button(kind.label()).clicked() {
                    run(app, "book.insert", json!({"kind": kind.id()}));
                    ui.close();
                }
            }
        });
    });
    ui.add_space(4.0);
    ui.separator();
    let mut st = ctx.data(|d| d.get_temp::<PaneState>(id)).unwrap_or_default();
    let caret_block =
        (app.session.sel.focus.story == wordcraft_doc::StoryRef::Body).then(|| app.session.sel.focus.path.0.first().map(|b| *b as usize)).flatten();
    let current = caret_block.and_then(|b| entries.iter().rposition(|e| e.block <= b && b < e.end));
    let depth = depths(&entries);
    let mut action: Option<(&'static str, Value)> = None;
    if entries.is_empty() {
        ui.label(egui::RichText::new("Aucun élément : ajoutez un chapitre pour commencer.").font(regular(12.0)).color(t.text_dim));
    }
    egui::ScrollArea::vertical().id_salt("lusk_book_list").auto_shrink([false, false]).show(ui, |ui| {
        let width = ui.available_width();
        for (i, e) in entries.iter().enumerate() {
            let indent = 14.0 * depth.get(i).copied().unwrap_or(0) as f32;
            let renaming = st.renaming.as_ref().filter(|(r, _)| *r == i).map(|(_, s)| s.clone());
            let row_h = 36.0;
            let (row, resp) = ui.allocate_exact_size(vec2(width, row_h), Sense::click_and_drag());
            let selected = current == Some(i);
            if selected {
                ui.painter().rect_filled(row, CornerRadius::same(5), t.accent.gamma_multiply(0.16));
            } else if resp.hovered() {
                ui.painter().rect_filled(row, CornerRadius::same(5), t.hover);
            }
            let x = row.min.x + 6.0 + indent;
            // The drag handle.
            for dy in [-4.0, 0.0, 4.0] {
                for dx in [0.0, 4.0] {
                    ui.painter().circle_filled(pos2(x + dx, row.center().y + dy), 1.1, t.text_dim.gamma_multiply(0.7));
                }
            }
            let words = group_digits(e.words);
            let words_w = ui.painter().layout_no_wrap(words.clone(), regular(11.0), t.text_dim).size().x;
            ui.painter().text(pos2(row.max.x - 6.0, row.center().y), Align2::RIGHT_CENTER, &words, regular(11.0), t.text_dim);
            let text_x = x + 14.0;
            let text_w = (row.max.x - 14.0 - words_w - text_x).max(20.0);
            ui.painter().text(pos2(text_x, row.min.y + 4.0), Align2::LEFT_TOP, e.label, regular(10.0), t.text_dim);
            if let Some(mut buffer) = renaming {
                let edit_rect = Rect::from_min_size(pos2(text_x, row.min.y + 15.0), vec2(text_w, 19.0));
                let out = ui.put(edit_rect, egui::TextEdit::singleline(&mut buffer).font(regular(12.5)).margin(Margin::symmetric(2, 0)));
                out.request_focus();
                let enter = ui.input(|inp| inp.key_pressed(egui::Key::Enter));
                let escape = ui.input(|inp| inp.key_pressed(egui::Key::Escape));
                if escape {
                    st.renaming = None;
                } else if enter || out.lost_focus() {
                    if buffer.trim() != e.title && !buffer.trim().is_empty() {
                        action = Some(("book.rename", json!({"index": i, "title": buffer.trim()})));
                    }
                    st.renaming = None;
                } else {
                    st.renaming = Some((i, buffer));
                }
            } else {
                let shown = if e.title.is_empty() { "(sans titre)".to_string() } else { e.title.clone() };
                let font = if e.rank <= 1 { medium(12.5) } else { regular(12.5) };
                let galley = ui.painter().layout(shown, font, t.text, text_w);
                let first_row = galley.rows.first().map(|r| r.text()).unwrap_or_default();
                let clipped = if galley.rows.len() > 1 { format!("{}…", first_row.trim_end()) } else { first_row };
                ui.painter().text(
                    pos2(text_x, row.min.y + 16.0),
                    Align2::LEFT_TOP,
                    clipped,
                    if e.rank <= 1 { medium(12.5) } else { regular(12.5) },
                    t.text,
                );
            }
            let resp = resp.on_hover_text(format!(
                "{} « {} » — {} mots\nClic : y aller · double-clic : renommer · glisser : déplacer",
                e.label, e.title, e.words
            ));
            if resp.drag_started() {
                egui::DragAndDrop::set_payload(&ctx, Dragged(i));
            }
            if resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            }
            // Drop: before the row when over its top half, after it otherwise.
            if let Some(dragged) = resp.dnd_hover_payload::<Dragged>()
                && dragged.0 != i
            {
                let pointer = ui.ctx().pointer_hover_pos().unwrap_or(row.center());
                let y = if pointer.y < row.center().y { row.min.y } else { row.max.y };
                ui.painter().line_segment([pos2(row.min.x + indent, y), pos2(row.max.x, y)], Stroke::new(2.5, t.accent));
            }
            if let Some(dragged) = resp.dnd_release_payload::<Dragged>()
                && dragged.0 != i
            {
                let pointer = ui.ctx().pointer_interact_pos().unwrap_or(row.center());
                let place = if pointer.y < row.center().y { "before" } else { "after" };
                action = Some(("book.move", json!({"from": dragged.0, "to": i, "place": place})));
            }
            if resp.double_clicked() {
                st.renaming = Some((i, e.title.clone()));
            } else if resp.clicked() {
                action = Some(("book.goto", json!({"index": i})));
            }
            resp.context_menu(|ui| {
                if ui.button("Aller à cet élément").clicked() {
                    action = Some(("book.goto", json!({"index": i})));
                    ui.close();
                }
                if ui.button("Renommer").clicked() {
                    st.renaming = Some((i, e.title.clone()));
                    ui.close();
                }
                if let Some(prev) = previous_sibling(&entries, i)
                    && ui.button("Monter").clicked()
                {
                    action = Some(("book.move", json!({"from": i, "to": prev, "place": "before"})));
                    ui.close();
                }
                if let Some(next) = next_sibling(&entries, i)
                    && ui.button("Descendre").clicked()
                {
                    action = Some(("book.move", json!({"from": i, "to": next, "place": "after"})));
                    ui.close();
                }
                if e.kind == ElementKind::Chapter && ui.button("Ajouter un sous-chapitre").clicked() {
                    let sub = entries.iter().enumerate().skip(i + 1).take_while(|(_, x)| x.block < e.end).last().map_or(i, |(k, _)| k);
                    action = Some(("book.insert", json!({"kind": "subchapter", "after": sub})));
                    ui.close();
                }
                ui.separator();
                if ui.button("Supprimer…").clicked() {
                    st.confirm_delete = Some((i, e.title.clone(), e.words));
                    ui.close();
                }
            });
        }
    });
    ctx.data_mut(|d| d.insert_temp(id, st));
    if let Some((cmd, params)) = action {
        run(app, cmd, params);
        if cmd == "book.goto" {
            app.canvas.want_focus = true;
        }
    }
}

/// The Pages tab: every page as a thumbnail, in double pages with the first page alone on the
/// right, as the printed book will show them; click one to go there.
fn pages_tab(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let ctx = ui.ctx().clone();
    let layout = app.session.layout();
    let sections: Vec<wordcraft_doc::section::NumFormat> = app.session.doc.sections().iter().map(|(_, sp)| sp.page_num_format).collect();
    let tid = app.window_id("lusk_book_thumbs");
    let ppp = ctx.pixels_per_point();
    let avail = ui.available_width();
    let thumb_w = ((avail - 26.0) / 2.0).clamp(40.0, 130.0);
    let mut th = ctx.data(|d| d.get_temp::<Thumbs>(tid)).unwrap_or_default();
    let rev = app.session.rev();
    if th.rev != rev || th.width != thumb_w as u32 {
        th = Thumbs { rev, width: thumb_w as u32, tex: Default::default() };
    }
    ui.label(egui::RichText::new(format!("{} pages", layout.pages.len())).font(regular(11.5)).color(t.text_dim));
    let current = app.session.page_hint;
    let mut go = None;
    let mut rendered = 0usize;
    egui::ScrollArea::vertical().id_salt("lusk_book_pages").auto_shrink([false, false]).show(ui, |ui| {
        let spine = ui.max_rect().center().x;
        let mut i = 0usize;
        while i < layout.pages.len() {
            // Row: the first page alone on the right, then left and right pages.
            let row: Vec<usize> = if i == 0 { vec![0] } else { (i..(i + 2).min(layout.pages.len())).collect() };
            let row_h = row
                .iter()
                .filter_map(|k| layout.pages.get(*k))
                .map(|p| thumb_w * p.h.max(1.0) / p.w.max(1.0))
                .fold(0.0f32, f32::max)
                .min(thumb_w * 3.0);
            let (band, _) = ui.allocate_exact_size(vec2(ui.available_width(), row_h + 20.0), Sense::hover());
            for &k in &row {
                let Some(page) = layout.pages.get(k) else { continue };
                let h = (thumb_w * page.h.max(1.0) / page.w.max(1.0)).min(thumb_w * 3.0);
                let right = k.is_multiple_of(2);
                let x = if right { spine } else { spine - thumb_w };
                let r = Rect::from_min_size(pos2(x, band.min.y), vec2(thumb_w, h));
                let resp = ui.interact(r, ui.scope_id().with(("lusk_thumb", k)), Sense::click());
                if ui.is_rect_visible(r) && !th.tex.contains_key(&k) && rendered < 3 {
                    let scale = (thumb_w * ppp / page.w.max(1.0)).clamp(0.02, 2.0);
                    let img = wordcraft_render::render_page(&app.session.doc, page, scale, &crate::canvas::screen_render_options());
                    let color = egui::ColorImage::from_rgba_unmultiplied([img.width as usize, img.height as usize], &img.to_straight());
                    th.tex.insert(k, ctx.load_texture(format!("lusk_thumb_{k}"), color, egui::TextureOptions::LINEAR));
                    rendered += 1;
                } else if ui.is_rect_visible(r) && !th.tex.contains_key(&k) {
                    ctx.request_repaint();
                }
                ui.painter().rect_filled(r, CornerRadius::ZERO, Color32::WHITE);
                if let Some(tex) = th.tex.get(&k) {
                    ui.painter().image(tex.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }
                let selected = k == current;
                let stroke = if selected {
                    Stroke::new(2.5, t.accent)
                } else if resp.hovered() {
                    Stroke::new(1.5, t.accent)
                } else {
                    Stroke::new(1.0, t.border_strong)
                };
                ui.painter().rect_stroke(r, CornerRadius::ZERO, stroke, egui::StrokeKind::Outside);
                let format = sections.get(page.section).copied().unwrap_or(wordcraft_doc::section::NumFormat::Decimal);
                let label = format.format(page.number);
                let anchor = if right { Align2::LEFT_TOP } else { Align2::RIGHT_TOP };
                let lx = if right { r.min.x + 2.0 } else { r.max.x - 2.0 };
                ui.painter().text(pos2(lx, r.max.y + 3.0), anchor, label, regular(10.5), if selected { t.text } else { t.text_dim });
                if resp.on_hover_text(format!("Page {} — clic pour y aller", k + 1)).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                    go = Some(k);
                }
            }
            // The spine between the two pages of a spread.
            if row.len() == 2 {
                ui.painter()
                    .line_segment([pos2(spine, band.min.y), pos2(spine, band.min.y + row_h)], Stroke::new(1.0, t.text_dim.gamma_multiply(0.6)));
            }
            i = if i == 0 { 1 } else { i + 2 };
        }
    });
    ctx.data_mut(|d| d.insert_temp(tid, th));
    if let Some(k) = go {
        run(app, "edit.goto", json!({"page": k + 1}));
        app.canvas.want_focus = true;
    }
}

/// 12345 → « 12 345 », as French writes numbers.
fn group_digits(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('\u{202F}');
        }
        out.push(c);
    }
    out
}

/// The contextual « Livre » ribbon tab, shown in Mode Livre.
pub fn ribbon(app: &mut WordApp, ui: &mut Ui) {
    use crate::widgets::{ButtonAction, big, big_toggle, group, row, small};
    group(ui, "Livre", None, app, |ui, app| {
        big(ui, app, "properties", "Réglages\ndu livre", "ui.bookWizard", json!({"mode": "settings"}), false);
        let on = app.session.view.book_pane;
        big_toggle(ui, app, "navPane", "Panneau\nLivre", "view.bookPane", on);
        let spreads = app.session.view.book_spreads;
        big_toggle(ui, app, "multiplePages", "Livre\nouvert", "view.bookSpreads", spreads);
        big(ui, app, "onePage", "Miniatures", "ui.bookPaneTab", json!({"tab": "pages"}), false);
    });
    group(ui, "Structure", None, app, |ui, app| {
        big(ui, app, "pageBreak", "Chapitre", "book.insert", json!({"kind": "chapter"}), false);
        crate::ribbon::stack(ui, |ui| {
            row(ui, |ui| {
                small(ui, app, "coverPage", Some("Partie"), "Ajouter une partie", ButtonAction::new("book.insert", json!({"kind": "part"}), false));
            });
            row(ui, |ui| {
                small(
                    ui,
                    app,
                    "outline",
                    Some("Sous-chapitre"),
                    "Ajouter un sous-chapitre au chapitre du curseur",
                    ButtonAction::new("book.insert", json!({"kind": "subchapter"}), false),
                );
            });
        });
    });
    group(ui, "Pages liminaires", None, app, |ui, app| {
        crate::ribbon::stack(ui, |ui| {
            row(ui, |ui| {
                small(
                    ui,
                    app,
                    "coverPage",
                    Some("Page de titre"),
                    "Insérer la page de titre",
                    ButtonAction::new("book.insert", json!({"kind": "titlePage"}), false),
                );
                small(
                    ui,
                    app,
                    "signature",
                    Some("Dédicace"),
                    "Insérer une dédicace",
                    ButtonAction::new("book.insert", json!({"kind": "dedication"}), false),
                );
            });
            row(ui, |ui| {
                small(
                    ui,
                    app,
                    "quickParts",
                    Some("Épigraphe"),
                    "Insérer une épigraphe",
                    ButtonAction::new("book.insert", json!({"kind": "epigraph"}), false),
                );
                small(
                    ui,
                    app,
                    "addText",
                    Some("Préface"),
                    "Insérer une préface",
                    ButtonAction::new("book.insert", json!({"kind": "preface"}), false),
                );
            });
            row(ui, |ui| {
                small(
                    ui,
                    app,
                    "addText",
                    Some("Avant-propos"),
                    "Insérer un avant-propos",
                    ButtonAction::new("book.insert", json!({"kind": "foreword"}), false),
                );
            });
        });
    });
    group(ui, "Fin d'ouvrage", None, app, |ui, app| {
        crate::ribbon::stack(ui, |ui| {
            row(ui, |ui| {
                small(
                    ui,
                    app,
                    "addText",
                    Some("Remerciements"),
                    "Insérer des remerciements",
                    ButtonAction::new("book.insert", json!({"kind": "acknowledgments"}), false),
                );
            });
            row(ui, |ui| {
                small(
                    ui,
                    app,
                    "sources",
                    Some("Bibliographie"),
                    "Insérer une bibliographie",
                    ButtonAction::new("book.insert", json!({"kind": "bibliography"}), false),
                );
            });
        });
    });
    group(ui, "Publier", None, app, |ui, app| {
        big(ui, app, "coverPage", "Studio\nCouverture", "ui.bookCover", json!({}), false);
        crate::ribbon::stack(ui, |ui| {
            row(ui, |ui| {
                small(
                    ui,
                    app,
                    "print",
                    Some("PDF pour l'imprimeur"),
                    "PDF avec le fond perdu et la zone de coupe de votre imprimeur, polices incorporées",
                    ButtonAction::new("ui.bookExport", json!({"what": "print"}), false),
                );
            });
            row(ui, |ui| {
                small(
                    ui,
                    app,
                    "export",
                    Some("PDF pour la lecture"),
                    "PDF pour lire à l'écran, avec signets et numéros de page",
                    ButtonAction::new("ui.bookExport", json!({"what": "reading"}), false),
                );
            });
            row(ui, |ui| {
                small(
                    ui,
                    app,
                    "documentTemplate",
                    Some("EPUB"),
                    "Livre numérique EPUB 3 pour liseuses",
                    ButtonAction::new("ui.bookExport", json!({"what": "epub"}), false),
                );
            });
        });
    });
    group(ui, "Vérifier", None, app, |ui, app| {
        big(ui, app, "checkErrors", "Contrôle\nqualité", "ui.bookPaneTab", json!({"tab": "qualite"}), false);
        if app.services.versions_dir.is_some() {
            big(ui, app, "recover", "Versions", "ui.bookPaneTab", json!({"tab": "versions"}), false);
        }
    });
    group(ui, "Sommaire", None, app, |ui, app| {
        big(ui, app, "toc", "Table des\nmatières", "references.toc", json!({}), false);
    });
    group(ui, "Nouveau", None, app, |ui, app| {
        big(ui, app, "documentTemplate", "Nouveau\nlivre", "ui.bookWizard", json!({"mode": "new"}), false);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: ElementKind, block: usize, end: usize) -> Entry {
        Entry { kind, label: kind.label(), title: String::new(), block, end, rank: kind.rank(), words: 0 }
    }

    #[test]
    fn siblings_stay_under_their_parent() {
        let e = vec![
            entry(ElementKind::TitlePage, 0, 2),
            entry(ElementKind::Part, 2, 9),
            entry(ElementKind::Chapter, 3, 6),
            entry(ElementKind::Subchapter, 4, 5),
            entry(ElementKind::Subchapter, 5, 6),
            entry(ElementKind::Chapter, 6, 9),
            entry(ElementKind::Part, 9, 12),
            entry(ElementKind::Chapter, 10, 12),
        ];
        assert_eq!(depths(&e), [0, 0, 1, 2, 2, 1, 0, 1]);
        assert_eq!(previous_sibling(&e, 4), Some(3));
        assert_eq!(previous_sibling(&e, 3), None, "the first sub-chapter of its chapter");
        assert_eq!(next_sibling(&e, 2), Some(5), "the next chapter, past the sub-chapters");
        assert_eq!(next_sibling(&e, 5), None, "the last chapter of its part");
        assert_eq!(next_sibling(&e, 1), Some(6));
        assert_eq!(previous_sibling(&e, 7), None);
        assert_eq!(next_sibling(&e, 99), None);
    }

    #[test]
    fn numbers_are_grouped_the_french_way() {
        assert_eq!(group_digits(0), "0");
        assert_eq!(group_digits(999), "999");
        assert_eq!(group_digits(12_345), "12\u{202F}345");
        assert_eq!(group_digits(1_234_567), "1\u{202F}234\u{202F}567");
    }
}
