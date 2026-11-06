//! L'aide de Lusk Texte (Aide › Aide) : des guides en français embarqués dans le programme
//! (dossier `aide/`, un Markdown simple) et une référence des commandes et des raccourcis
//! clavier générée à partir du registre des commandes, donc toujours à jour. Tout fonctionne
//! sans connexion.
//!
//! Une vraie fenêtre du système, ouverte agrandie, dans l'esprit du menu Fichier : un bandeau
//! dans la couleur de l'application (logo, recherche, sommaire) et la page à droite.

use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Align2, Color32, CornerRadius, Layout, Margin, Rect, RichText, ScrollArea, Sense, Stroke, Ui, pos2, vec2};

use crate::theme::{Tokens, medium, regular, semibold};
use crate::{WordApp, icons};

/// Les guides : titre et icône dans le sommaire, texte. Le texte accepte `# titre`,
/// `## sous-titre`, les listes `- …` et `1. …`, et `**gras**`.
const GUIDES: [(&str, &str, &str); 7] = [
    ("Bien démarrer", "homeFolder", include_str!("../aide/01-bien-demarrer.md")),
    ("Créer, ouvrir et enregistrer", "open", include_str!("../aide/02-fichiers.md")),
    ("Mettre en forme le texte", "bold", include_str!("../aide/03-mise-en-forme.md")),
    ("Mise en page et structure", "orientation", include_str!("../aide/04-mise-en-page.md")),
    ("Images, tableaux et objets", "picture", include_str!("../aide/05-images-tableaux.md")),
    ("Vérifier et réviser", "spelling", include_str!("../aide/06-verifier-reviser.md")),
    ("Imprimer, exporter et partager", "print", include_str!("../aide/07-imprimer-partager.md")),
];

/// La rubrique du Mode Livre (son onglet dans le sommaire), après les guides.
const BOOK: [(&str, &str, &str); 6] = [
    ("Découvrir le Mode Livre", "coverPage", include_str!("../aide/livre/01-decouvrir.md")),
    ("Structurer le livre", "toc", include_str!("../aide/livre/02-structurer.md")),
    ("Mise en page d'édition", "onePage", include_str!("../aide/livre/03-mise-en-page.md")),
    ("Couverture et impression", "documentTemplate", include_str!("../aide/livre/04-couverture-impression.md")),
    ("Livre numérique (EPUB)", "export", include_str!("../aide/livre/05-livre-numerique.md")),
    ("Vérifier et conserver", "checkErrors", include_str!("../aide/livre/06-verifier-conserver.md")),
];
/// La première page de la rubrique Mode Livre.
pub(crate) const BOOK_START: usize = GUIDES.len();

/// Les deux pages générées, après les guides et la rubrique Mode Livre.
const REFERENCE: usize = GUIDES.len() + BOOK.len();
const SHORTCUTS: usize = REFERENCE + 1;

/// Les pages écrites (guides puis Mode Livre) : numéro de page, rubrique, titre, icône et texte.
fn written() -> impl Iterator<Item = (usize, &'static str, &'static str, &'static str, &'static str)> {
    GUIDES
        .iter()
        .map(|g| ("Guide", g))
        .chain(BOOK.iter().map(|g| ("Mode Livre", g)))
        .enumerate()
        .map(|(i, (kicker, (title, icon, text)))| (i, kicker, *title, *icon, *text))
}
const GENERATED: [(usize, &str, &str); 2] = [(REFERENCE, "Toutes les commandes", "search"), (SHORTCUTS, "Raccourcis clavier", "bullets")];

/// L'ordre des onglets du ruban, pour classer la référence.
const TAB_ORDER: [&str; 13] =
    ["File", "Home", "Insert", "Draw", "Design", "Layout", "References", "Mailings", "Review", "View", "Developer", "Zotero", "Help"];

/// Taille et interligne du texte des pages.
const TEXT: f32 = 14.0;
const LINE: f32 = 22.0;
/// Largeur du bandeau et largeur maximale de la page.
const NAV_W: f32 = 260.0;
const PAGE_W: f32 = 820.0;

#[derive(Clone, Default)]
struct State {
    open: bool,
    page: usize,
    query: String,
    /// Images restantes pendant lesquelles la fenêtre qui vient de s'ouvrir est agrandie : Windows
    /// ignore « agrandie » pour une fenêtre créée cachée, comme eframe les crée.
    maximize: u8,
}

fn state_id() -> egui::Id {
    egui::Id::unique("lusk_help_state")
}

/// Ouvre la fenêtre d'aide (commande `ui.help`).
/// Sur la page `page` quand elle est donnée (`ui.help {"page": "livre"}`).
pub(crate) fn open_at(ctx: &egui::Context, page: Option<usize>) {
    ctx.data_mut(|d| {
        let st = d.get_temp_mut_or_default::<State>(state_id());
        if !st.open {
            st.maximize = 3;
        }
        st.open = true;
        if let Some(p) = page.filter(|p| *p <= SHORTCUTS) {
            st.page = p;
            st.query.clear();
        }
    });
    ctx.request_repaint();
}

/// Dessine la fenêtre d'aide, si elle est ouverte. Là où l'interface n'a qu'une fenêtre
/// (l'application web), egui la dessine à l'intérieur, avec un bouton Fermer.
pub(crate) fn show(app: &WordApp, ctx: &egui::Context) {
    let mut st = ctx.data(|d| d.get_temp::<State>(state_id())).unwrap_or_default();
    if !st.open {
        return;
    }
    // Chaque fenêtre de document passe par ici : l'aide n'est montrée que depuis la fenêtre active.
    if ctx.viewport_id() != app.active_viewport() {
        return;
    }
    let builder = egui::ViewportBuilder::default()
        .with_title(format!("{} — Lusk Texte", tl!("Help")))
        .with_inner_size([1180.0, 800.0])
        .with_min_inner_size([760.0, 480.0]);
    let mut close = false;
    ctx.show_viewport_immediate(egui::ViewportId::from_hash_of("lusk_help"), builder, |ui, class| {
        let embedded = matches!(class, egui::ViewportClass::EmbeddedWindow);
        if !embedded && st.maximize > 0 {
            st.maximize -= 1;
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
            ui.ctx().request_repaint();
        }
        if ui.ctx().input(|i| i.viewport().close_requested()) {
            close = true;
        }
        let t = Tokens::get(ui.ctx());
        egui::Panel::left("help_nav")
            .exact_size(NAV_W)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::NONE.fill(t.nav).inner_margin(Margin { left: 0, right: 0, top: 18, bottom: 12 }))
            .show(ui, |ui| nav(ui, &mut st, embedded, &mut close));
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(t.panel).inner_margin(Margin { left: 56, right: 32, top: 32, bottom: 0 })).show(
            ui,
            |ui| {
                let searching = !st.query.trim().is_empty();
                ScrollArea::vertical().id_salt(("help_page", st.page, searching)).auto_shrink([false, false]).show(ui, |ui| {
                    ui.set_max_width(PAGE_W.min(ui.available_width() - 24.0));
                    if searching {
                        results(app, ui, &mut st);
                    } else {
                        page(app, ui, st.page);
                    }
                    ui.add_space(48.0);
                });
            },
        );
    });
    if close {
        st.open = false;
    }
    ctx.data_mut(|d| d.insert_temp(state_id(), st));
}

// --- Le bandeau ----------------------------------------------------------------------------------

/// Le bandeau : logo et titre, recherche, sommaire.
fn nav(ui: &mut Ui, st: &mut State, embedded: bool, close: &mut bool) {
    let white = Color32::WHITE;
    let soft = Color32::from_white_alpha(170);
    // Logo et titre.
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        let logo = crate::chrome::account_logo(ui.ctx());
        let size = logo.size_vec2();
        let h = 40.0;
        let (r, _) = ui.allocate_exact_size(vec2(h * size.x / size.y, h), Sense::hover());
        ui.painter().image(logo.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        ui.add_space(8.0);
        ui.vertical(|ui| {
            ui.add_space(1.0);
            ui.label(RichText::new("Aide").font(semibold(21.0)).color(white));
            ui.label(RichText::new("Lusk Texte").font(regular(12.5)).color(soft));
        });
    });
    ui.add_space(18.0);
    // Recherche.
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        let edit = egui::TextEdit::singleline(&mut st.query)
            .hint_text("Rechercher dans l'aide…")
            .desired_width(NAV_W - 32.0)
            .margin(Margin::symmetric(10, 7))
            .font(regular(13.5));
        ui.add(edit);
    });
    ui.add_space(20.0);
    let searching = !st.query.trim().is_empty();
    // Le sommaire défile quand la fenêtre est trop basse pour tout montrer.
    let room = ui.available_height() - if embedded { 44.0 } else { 0.0 };
    ScrollArea::vertical().id_salt("help_nav_scroll").max_height(room.max(80.0)).auto_shrink([false, true]).show(ui, |ui| {
        section(ui, "GUIDES");
        for (i, _, title, icon, _) in written().take(GUIDES.len()) {
            if nav_item(ui, icon, title, !searching && st.page == i) {
                st.page = i;
                st.query.clear();
            }
        }
        ui.add_space(14.0);
        section(ui, "MODE LIVRE");
        for (i, _, title, icon, _) in written().skip(GUIDES.len()) {
            if nav_item(ui, icon, title, !searching && st.page == i) {
                st.page = i;
                st.query.clear();
            }
        }
        ui.add_space(14.0);
        section(ui, "RÉFÉRENCE");
        for (i, title, icon) in GENERATED {
            if nav_item(ui, icon, title, !searching && st.page == i) {
                st.page = i;
                st.query.clear();
            }
        }
    });
    if embedded {
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            if nav_item(ui, "close", tl!("Close"), false) {
                *close = true;
            }
        });
    }
}

/// Un intitulé de section du bandeau, en petites capitales.
fn section(ui: &mut Ui, text: &str) {
    ui.horizontal(|ui| {
        ui.add_space(22.0);
        ui.label(RichText::new(text).font(semibold(11.0)).color(Color32::from_white_alpha(150)).extra_letter_spacing(1.2));
    });
    ui.add_space(4.0);
}

/// Une ligne du sommaire, comme celles du menu Fichier : icône et titre en blanc, la page affichée
/// surlignée d'un voile clair et d'un filet à gauche.
fn nav_item(ui: &mut Ui, icon: &str, label: &str, active: bool) -> bool {
    let (r, resp) = ui.allocate_exact_size(vec2(NAV_W, 38.0), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    let p = ui.painter();
    if active {
        p.rect_filled(r, 0.0, Color32::from_white_alpha(46));
        p.rect_filled(Rect::from_min_size(r.min, vec2(4.0, r.height())), 0.0, Color32::WHITE);
    } else if resp.hovered() {
        p.rect_filled(r, 0.0, Color32::from_white_alpha(26));
    }
    icons::paint(p, Rect::from_center_size(pos2(r.min.x + 32.0, r.center().y), vec2(18.0, 18.0)), icon, Color32::WHITE, Color32::WHITE);
    p.text(pos2(r.min.x + 54.0, r.center().y), Align2::LEFT_CENTER, label, if active { semibold(14.0) } else { medium(14.0) }, Color32::WHITE);
    resp.clicked()
}

// --- Les pages -----------------------------------------------------------------------------------

fn page(app: &WordApp, ui: &mut Ui, page: usize) {
    match page {
        REFERENCE => reference(app, ui, false),
        SHORTCUTS => reference(app, ui, true),
        i => {
            let (kicker, text) = written().find(|w| w.0 == i).map_or(("Guide", GUIDES[0].2), |w| (w.1, w.4));
            markdown(ui, kicker, text);
        }
    }
}

/// L'en-tête d'une page : la rubrique en petites capitales, le titre et un filet de couleur.
fn page_title(ui: &mut Ui, kicker: &str, title: &str) {
    let t = Tokens::get(ui.ctx());
    ui.label(RichText::new(kicker.to_uppercase()).font(semibold(11.5)).color(t.accent_text).extra_letter_spacing(1.2));
    ui.add_space(4.0);
    ui.label(RichText::new(title).font(semibold(30.0)).color(t.text));
    ui.add_space(10.0);
    let (r, _) = ui.allocate_exact_size(vec2(56.0, 4.0), Sense::hover());
    ui.painter().rect_filled(r, CornerRadius::same(2), t.accent);
    ui.add_space(18.0);
}

/// Un sous-titre de page, avec un filet fin sur toute la largeur.
fn subheading(ui: &mut Ui, text: &str) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(22.0);
    ui.label(RichText::new(text).font(semibold(18.0)).color(t.accent_text));
    ui.add_space(4.0);
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
    ui.painter().rect_filled(r, 0.0, t.border);
    ui.add_space(8.0);
}

/// Affiche un guide au format Markdown simple ; son `# titre` devient l'en-tête de la page.
fn markdown(ui: &mut Ui, kicker: &str, text: &str) {
    let t = Tokens::get(ui.ctx());
    for line in text.lines() {
        let l = line.trim_end();
        if let Some(h) = l.strip_prefix("# ") {
            page_title(ui, kicker, h);
        } else if let Some(h) = l.strip_prefix("## ") {
            subheading(ui, h);
        } else if let Some(h) = l.strip_prefix("### ") {
            ui.add_space(10.0);
            ui.label(RichText::new(h).font(semibold(15.5)).color(t.text));
            ui.add_space(4.0);
        } else if l.trim().is_empty() {
            ui.add_space(6.0);
        } else if let Some(item) = l.strip_prefix("- ") {
            item_line(ui, None, item, &t);
        } else if let Some((n, item)) = numbered(l) {
            item_line(ui, Some(n), item, &t);
        } else {
            let w = ui.available_width();
            ui.label(inline(l, t.text, w));
            ui.add_space(4.0);
        }
    }
}

/// `1. texte` → ("1", "texte").
fn numbered(l: &str) -> Option<(&str, &str)> {
    let (n, rest) = l.split_once(". ")?;
    (!n.is_empty() && n.chars().all(|c| c.is_ascii_digit())).then_some((n, rest))
}

/// Un élément de liste : une pastille de couleur, ou le numéro dans un rond de couleur pour une
/// étape, puis le texte qui revient à la ligne en retrait.
fn item_line(ui: &mut Ui, number: Option<&str>, text: &str, t: &Tokens) {
    ui.horizontal_top(|ui| {
        ui.add_space(4.0);
        let (r, _) = ui.allocate_exact_size(vec2(30.0, LINE), Sense::hover());
        let c = pos2(r.min.x + 11.0, r.center().y);
        match number {
            Some(n) => {
                ui.painter().circle_filled(c, 10.0, t.accent);
                ui.painter().text(c, Align2::CENTER_CENTER, n, semibold(11.5), t.on_accent);
            }
            None => {
                ui.painter().circle_filled(c, 3.5, t.accent);
            }
        }
        let w = ui.available_width();
        ui.label(inline(text, t.text, w));
    });
    ui.add_space(3.0);
}

/// Une ligne avec ses passages `**en gras**`, qui revient à la ligne à la largeur `width`.
fn inline(text: &str, color: Color32, width: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    job.wrap.max_width = width;
    for (i, part) in text.split("**").enumerate() {
        if part.is_empty() {
            continue;
        }
        let font = if i % 2 == 1 { semibold(TEXT) } else { regular(TEXT) };
        job.append(part, 0.0, TextFormat { font_id: font, color, line_height: Some(LINE), ..Default::default() });
    }
    job
}

// --- La référence --------------------------------------------------------------------------------

/// Une commande du registre telle que l'aide la montre : nom, emplacement et raccourci traduits.
struct Row {
    tab: usize,
    english_location: &'static str,
    location: String,
    label: String,
    shortcut: String,
}

fn rows(app: &WordApp) -> Vec<Row> {
    let mut rows: Vec<Row> = app
        .session
        .registry
        .all()
        .iter()
        .filter(|c| !c.location.is_empty() && !c.label.is_empty())
        .map(|c| {
            let first = c.location.split(" › ").next().unwrap_or("");
            Row {
                tab: TAB_ORDER.iter().position(|t| *t == first).unwrap_or(TAB_ORDER.len()),
                english_location: c.location,
                location: crate::i18n::location(c.location),
                label: tl!(c.label).replace('\n', " "),
                shortcut: crate::widgets::shortcut_text(app, c.id),
            }
        })
        .collect();
    // Tri stable : les commandes d'un même groupe gardent l'ordre du registre.
    rows.sort_by(|a, b| a.tab.cmp(&b.tab).then_with(|| a.english_location.cmp(b.english_location)));
    rows
}

/// Un raccourci dessiné comme une touche de clavier.
fn keycap(ui: &mut Ui, text: &str) {
    if text.is_empty() {
        return;
    }
    let t = Tokens::get(ui.ctx());
    egui::Frame::NONE
        .fill(t.input)
        .stroke(Stroke::new(1.0, t.border_strong))
        .corner_radius(CornerRadius::same(4))
        .inner_margin(Margin::symmetric(7, 2))
        .show(ui, |ui| ui.label(RichText::new(text).font(egui::FontId::monospace(12.0)).color(t.text)));
}

/// Un groupe de la référence dans une carte : son titre, puis ses commandes et leurs raccourcis.
fn card(ui: &mut Ui, title: &str, rows: &[Row], key: (bool, usize)) {
    let t = Tokens::get(ui.ctx());
    ui.add_space(12.0);
    egui::Frame::NONE
        .fill(t.ribbon)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin { left: 18, right: 18, top: 12, bottom: 12 })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).font(semibold(15.0)).color(t.accent_text));
            ui.add_space(6.0);
            egui::Grid::new(("help_reference", key)).num_columns(2).spacing(vec2(32.0, 7.0)).min_col_width(260.0).show(ui, |ui| {
                for r in rows {
                    ui.label(RichText::new(&r.label).font(regular(TEXT)).color(t.text));
                    keycap(ui, &r.shortcut);
                    ui.end_row();
                }
            });
        });
}

/// La référence de toutes les commandes, ou seulement de celles qui ont un raccourci.
fn reference(app: &WordApp, ui: &mut Ui, shortcuts_only: bool) {
    let t = Tokens::get(ui.ctx());
    if shortcuts_only {
        page_title(ui, "Référence", "Raccourcis clavier");
        ui.label(inline(
            "Les raccourcis clavier de Lusk Texte, classés par onglet du ruban. Ils s'affichent aussi quand vous survolez un bouton.",
            t.text,
            ui.available_width(),
        ));
    } else {
        page_title(ui, "Référence", "Toutes les commandes");
        ui.label(inline(
            "Toutes les commandes de Lusk Texte, classées selon leur emplacement dans le ruban, avec leur raccourci clavier quand elles en ont un. Pour en trouver une rapidement, tapez son nom dans la zone **Rechercher** du bandeau ou de la barre de titre.",
            t.text,
            ui.available_width(),
        ));
    }
    let rows: Vec<Row> = rows(app).into_iter().filter(|r| !shortcuts_only || !r.shortcut.is_empty()).collect();
    let group_of = |r: &Row| if shortcuts_only { r.location.split(" › ").next().unwrap_or("").to_string() } else { r.location.clone() };
    let mut i = 0;
    while i < rows.len() {
        let group = group_of(&rows[i]);
        let end = rows[i..].iter().position(|r| group_of(r) != group).map_or(rows.len(), |n| i + n);
        card(ui, &group, &rows[i..end], (shortcuts_only, i));
        i = end;
    }
}

/// Les résultats d'une recherche : les guides qui contiennent les mots cherchés, puis les commandes.
fn results(app: &WordApp, ui: &mut Ui, st: &mut State) {
    let t = Tokens::get(ui.ctx());
    let q = fold(st.query.trim());
    page_title(ui, "Recherche", &format!("« {} »", st.query.trim()));
    let mut found = false;
    for (i, _, title, icon, text) in written() {
        let hits: Vec<&str> = text.lines().filter(|l| !l.starts_with('#') && fold(l).contains(&q)).take(3).collect();
        if hits.is_empty() && !fold(title).contains(&q) {
            continue;
        }
        found = true;
        ui.add_space(10.0);
        let resp = egui::Frame::NONE
            .fill(t.ribbon)
            .stroke(Stroke::new(1.0, t.border))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(Margin { left: 18, right: 18, top: 12, bottom: 12 })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    icons::paint(ui.painter(), r, icon, t.accent_text, t.accent);
                    ui.add_space(4.0);
                    ui.label(RichText::new(title).font(semibold(15.5)).color(t.accent_text));
                });
                for h in &hits {
                    ui.add_space(2.0);
                    ui.label(RichText::new(plain(h)).font(regular(13.0)).color(t.text_dim));
                }
            })
            .response
            .interact(Sense::click());
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            st.page = i;
            st.query.clear();
        }
    }
    let cmds: Vec<Row> = rows(app).into_iter().filter(|r| fold(&r.label).contains(&q) || fold(&r.location).contains(&q)).take(80).collect();
    if !cmds.is_empty() {
        found = true;
        subheading(ui, "Commandes");
        egui::Grid::new("help_results").num_columns(3).spacing(vec2(28.0, 7.0)).striped(true).show(ui, |ui| {
            for r in &cmds {
                ui.label(RichText::new(&r.label).font(regular(TEXT)).color(t.text));
                ui.label(RichText::new(&r.location).font(regular(12.5)).color(t.text_dim));
                keycap(ui, &r.shortcut);
                ui.end_row();
            }
        });
    }
    if !found {
        ui.add_space(8.0);
        ui.label(inline("Aucun résultat. Essayez un autre mot, par exemple « tableau », « marges » ou « PDF ».", t.text, ui.available_width()));
    }
}

/// Une ligne de guide sans ses marques Markdown, pour les extraits des résultats.
fn plain(line: &str) -> String {
    let l = line.trim_start_matches("- ");
    let l = numbered(l).map_or(l, |(_, rest)| rest);
    l.replace("**", "")
}

/// Le texte en minuscules et sans accents, pour une recherche qui ignore les deux.
fn fold(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            '’' => '\'',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{BOOK, BOOK_START, GUIDES, REFERENCE, fold, numbered, written};

    #[test]
    fn search_ignores_case_and_accents() {
        assert_eq!(fold("Créer un Tableau"), "creer un tableau");
        assert!(fold("Mise en forme").contains(&fold("FORME")));
    }

    #[test]
    fn numbered_items_are_recognised() {
        assert_eq!(numbered("2. Cliquez ici"), Some(("2", "Cliquez ici")));
        assert_eq!(numbered("Voir p. 2"), None);
    }

    #[test]
    fn every_guide_has_a_title() {
        for (_, _, title, _, text) in written() {
            assert!(text.starts_with("# "), "{title}");
        }
    }

    /// La rubrique Mode Livre suit les guides, et la référence suit la rubrique.
    #[test]
    fn the_book_pages_have_their_own_section() {
        assert_eq!(BOOK_START, GUIDES.len());
        assert_eq!(REFERENCE, GUIDES.len() + BOOK.len());
        let book: Vec<_> = written().filter(|w| w.1 == "Mode Livre").map(|w| w.0).collect();
        assert_eq!(book, (BOOK_START..REFERENCE).collect::<Vec<_>>());
        assert!(written().any(|w| w.4.contains("EPUBCheck") && w.4.contains("texte de remplacement")));
    }
}
