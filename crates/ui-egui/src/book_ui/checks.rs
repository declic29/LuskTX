//! Mode Livre, phase 5 : l'onglet Qualité du panneau Livre (contrôle qualité, corrections sur
//! demande seulement) et l'onglet Versions (historique des versions sur cet ordinateur).

use std::collections::BTreeMap;

use egui::{Color32, CornerRadius, Margin, Sense, Ui, vec2};
use serde_json::{Value, json};
use wordcraft_engine::book::quality::{Issue, Level};
use wordcraft_engine::book::versions::Version;

use super::{WizardMode, open_wizard, run};
use crate::WordApp;
use crate::theme::{Tokens, regular, semibold};

const RED: Color32 = Color32::from_rgb(0xC4, 0x2B, 0x1C);
const ORANGE: Color32 = Color32::from_rgb(0xD9, 0x7A, 0x00);
const TEAL: Color32 = Color32::from_rgb(0x1D, 0x9C, 0x9B);

/// The last analysis, with the body it was made on.
#[derive(Clone, Default)]
struct QualityState {
    key: u64,
    issues: Vec<Issue>,
    ran: bool,
    hide_tips: bool,
    note: Option<String>,
}

/// The version list, as last read, and the name being typed for a new version.
#[derive(Clone, Default)]
struct VersionsState {
    list: Option<Vec<Version>>,
    label: String,
    confirm_delete: Option<(String, String)>,
    note: Option<String>,
}

fn quality_id(app: &WordApp) -> egui::Id {
    app.window_id("lusk_book_quality")
}

fn versions_id(app: &WordApp) -> egui::Id {
    app.window_id("lusk_book_versions")
}

/// A key that changes whenever the body does (blocks are shared between versions).
fn body_key(app: &WordApp) -> u64 {
    let doc = &app.session.doc;
    let mut key: u64 = doc.body.len() as u64 ^ 0x9e37_79b9;
    for b in &doc.body {
        key = key.wrapping_mul(1_000_003).wrapping_add(std::sync::Arc::as_ptr(b) as usize as u64);
    }
    key
}

pub(super) fn has_results(app: &WordApp, ctx: &egui::Context) -> bool {
    ctx.data(|d| d.get_temp::<QualityState>(quality_id(app))).is_some_and(|q| q.ran)
}

/// Run the analysis (nothing in the document changes).
pub(super) fn analyse(app: &mut WordApp, ctx: &egui::Context) {
    let id = quality_id(app);
    let mut q = ctx.data(|d| d.get_temp::<QualityState>(id)).unwrap_or_default();
    match app.run("book.check", json!({})) {
        Ok(v) => {
            q.issues = v.get("issues").cloned().and_then(|i| serde_json::from_value(i).ok()).unwrap_or_default();
            q.ran = true;
            q.key = body_key(app);
        }
        Err(e) => app.status(e),
    }
    ctx.data_mut(|d| d.insert_temp(id, q));
}

fn family(kind: &str) -> &'static str {
    match kind {
        "pageBlanche" => "Pages blanches",
        "veuve" => "Lignes veuves",
        "orpheline" => "Lignes orphelines",
        "titreIsole" => "Titres isolés en bas de page",
        "resolution" => "Résolution des images",
        "texteAlt" => "Textes de remplacement",
        "police" => "Polices",
        "marges" => "Marges",
        "format" => "Format des pages",
        "sommaire" => "Sommaire",
        "titres" => "Titres",
        "paragraphesVides" => "Paragraphes vides",
        "doubleEspace" => "Doubles espaces",
        "typographie" => "Typographie française",
        _ => "Autres",
    }
}

fn color(level: Level) -> Color32 {
    match level {
        Level::Erreur => RED,
        Level::Attention => ORANGE,
        Level::Conseil => TEAL,
    }
}

fn plural(n: usize, one: &str, many: &str) -> String {
    // En français, 0 et 1 sont au singulier.
    if n <= 1 { format!("{n} {one}") } else { format!("{n} {many}") }
}

/// Go to an issue: select what it is about, the page scrolls to it.
fn go(app: &mut WordApp, i: &Issue) {
    let focus = wordcraft_doc::Pos { off: i.pos.off.saturating_add(i.len), ..i.pos.clone() };
    let (Ok(a), Ok(f)) = (serde_json::to_value(&i.pos), serde_json::to_value(&focus)) else { return };
    run(app, "select.range", json!({"anchor": a, "focus": f}));
    app.canvas.want_focus = true;
}

fn fix_value(i: &Issue) -> Option<Value> {
    let fix = i.fix.as_ref()?;
    Some(json!({"pos": serde_json::to_value(&i.pos).ok()?, "len": i.len, "was": i.was, "fix": fix}))
}

/// Apply corrections the user asked for, then analyse again (the positions moved).
fn apply(app: &mut WordApp, ctx: &egui::Context, issues: &[&Issue]) {
    let fixes: Vec<Value> = issues.iter().filter_map(|i| fix_value(i)).collect();
    if fixes.is_empty() {
        return;
    }
    let n = match app.run("book.fix", json!({ "fixes": fixes })) {
        Ok(v) => v.get("fixed").and_then(Value::as_u64).unwrap_or(0),
        Err(e) => {
            app.status(e);
            return;
        }
    };
    analyse(app, ctx);
    let id = quality_id(app);
    let mut q = ctx.data(|d| d.get_temp::<QualityState>(id)).unwrap_or_default();
    q.note = Some(format!("{} — Ctrl+Z pour annuler.", plural(n as usize, "correction faite", "corrections faites")));
    ctx.data_mut(|d| d.insert_temp(id, q));
}

/// The Qualité tab.
pub(super) fn quality_tab(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let ctx = ui.ctx().clone();
    let id = quality_id(app);
    let mut q = ctx.data(|d| d.get_temp::<QualityState>(id)).unwrap_or_default();
    let stale = q.ran && q.key != body_key(app);
    let mut rerun = false;
    ui.horizontal_wrapped(|ui| {
        let label = if q.ran { "Relancer l'analyse" } else { "Analyser le livre" };
        if ui.button(label).on_hover_text("Relit tout le livre et sa mise en page ; rien n'est modifié").clicked() {
            rerun = true;
        }
        ui.checkbox(&mut q.hide_tips, "Masquer les conseils");
    });
    ui.label(
        egui::RichText::new(
            "Rien n'est modifié sans votre accord : cliquez un problème pour y aller, « Corriger » pour accepter la correction proposée.",
        )
        .font(regular(11.0))
        .color(t.text_dim),
    );
    if stale {
        ui.label(egui::RichText::new("Le texte a changé depuis l'analyse : relancez-la avant de corriger.").font(regular(11.5)).color(ORANGE));
    }
    if let Some(n) = &q.note {
        ui.label(egui::RichText::new(n).font(regular(11.5)).color(t.text));
    }
    ui.add_space(4.0);
    let mut action: Option<(&'static str, Vec<Issue>)> = None;
    if q.ran {
        let count = |l: Level| q.issues.iter().filter(|i| i.level == l).count();
        let (e, a, c) = (count(Level::Erreur), count(Level::Attention), count(Level::Conseil));
        if e + a + c == 0 {
            ui.label(egui::RichText::new("✓ Aucun problème trouvé.").font(semibold(13.0)).color(TEAL));
        } else {
            ui.horizontal_wrapped(|ui| {
                for (n, one, many, col) in [(e, "erreur", "erreurs", RED), (a, "à vérifier", "à vérifier", ORANGE), (c, "conseil", "conseils", TEAL)]
                {
                    let (r, p) = ui.allocate_painter(vec2(10.0, 10.0), Sense::hover());
                    p.circle_filled(r.rect.center(), 4.5, col);
                    ui.label(egui::RichText::new(plural(n, one, many)).font(regular(12.0)).color(t.text));
                    ui.add_space(6.0);
                }
            });
        }
        if q.issues.len() >= wordcraft_engine::book::quality::MAX_ISSUES {
            ui.label(
                egui::RichText::new("Analyse arrêtée après 2 000 problèmes : corrigez les premiers puis relancez.")
                    .font(regular(11.0))
                    .color(t.text_dim),
            );
        }
        ui.separator();
        let mut groups: BTreeMap<(Level, &'static str), Vec<&Issue>> = BTreeMap::new();
        for i in q.issues.iter().filter(|i| !(q.hide_tips && i.level == Level::Conseil)) {
            groups.entry((i.level, family(&i.kind))).or_default().push(i);
        }
        egui::ScrollArea::vertical().id_salt("lusk_quality_scroll").auto_shrink([false, false]).show(ui, |ui| {
            for ((level, name), items) in &groups {
                let fixable: Vec<&Issue> = items.iter().copied().filter(|i| i.fix.is_some()).collect();
                let header = egui::RichText::new(format!("{name} ({})", items.len())).font(semibold(12.5)).color(color(*level));
                egui::CollapsingHeader::new(header).id_salt(("lusk_q", name, *level as u8)).default_open(items.len() <= 12).show(ui, |ui| {
                    if fixable.len() > 1 && !stale && ui.small_button(format!("Tout corriger ({})", fixable.len())).clicked() {
                        action = Some(("fix", fixable.iter().map(|i| (*i).clone()).collect()));
                    }
                    for i in items.iter().take(300) {
                        issue_row(ui, &t, i, stale, &mut action);
                    }
                    if items.len() > 300 {
                        ui.label(egui::RichText::new(format!("… et {} autres.", items.len() - 300)).font(regular(11.0)).color(t.text_dim));
                    }
                });
            }
        });
    }
    ctx.data_mut(|d| d.insert_temp(id, q));
    if rerun {
        analyse(app, &ctx);
        return;
    }
    match action {
        Some(("go", v)) => {
            if let Some(i) = v.first() {
                go(app, i);
            }
        }
        Some(("fix", v)) => {
            let refs: Vec<&Issue> = v.iter().collect();
            apply(app, &ctx, &refs);
        }
        Some(("command", v)) => {
            let Some(cmd) = v.first().and_then(|i| i.command.clone()) else { return };
            if cmd == "book.settings" {
                open_wizard(app, &ctx, WizardMode::Settings);
            } else {
                run(app, &cmd, json!({}));
                analyse(app, &ctx);
            }
        }
        _ => {}
    }
}

fn issue_row(ui: &mut Ui, t: &Tokens, i: &Issue, stale: bool, action: &mut Option<(&'static str, Vec<Issue>)>) {
    egui::Frame::NONE.inner_margin(Margin::symmetric(4, 3)).show(ui, |ui| {
        ui.horizontal(|ui| {
            let (r, p) = ui.allocate_painter(vec2(8.0, 14.0), Sense::hover());
            p.circle_filled(r.rect.center(), 3.5, color(i.level));
            ui.vertical(|ui| {
                let mut text = i.message.clone();
                if let Some(page) = i.page {
                    text.push_str(&format!("  · p. {page}"));
                }
                // The message is the link to the place in the book; the buttons stay apart.
                let msg = ui
                    .add(egui::Label::new(egui::RichText::new(text).font(regular(11.5)).color(t.text)).wrap().sense(Sense::click()))
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Aller à cet endroit du livre");
                if msg.hovered() {
                    ui.painter().rect_stroke(
                        msg.rect.expand(2.0),
                        CornerRadius::same(3),
                        egui::Stroke::new(1.0, t.text_dim),
                        egui::StrokeKind::Outside,
                    );
                }
                if msg.clicked() {
                    *action = Some(("go", vec![i.clone()]));
                }
                ui.horizontal(|ui| {
                    if let Some(fix) = i.fix.as_ref().filter(|_| !stale) {
                        let what = if fix.is_empty() { "rien (supprimer)".to_string() } else { format!("« {} »", shown(fix)) };
                        if ui.small_button("Corriger").on_hover_text(format!("Remplacer « {} » par {what}", shown(&i.was))).clicked() {
                            *action = Some(("fix", vec![i.clone()]));
                        }
                    }
                    if let Some(cmd) = &i.command {
                        let label = if cmd == "book.settings" { "Réglages du livre…" } else { "Mettre à jour" };
                        if ui.small_button(label).clicked() {
                            *action = Some(("command", vec![i.clone()]));
                        }
                    }
                });
            });
        });
    });
}

/// Invisible characters made visible in a tooltip.
fn shown(s: &str) -> String {
    s.replace(' ', "[espace]").replace('\u{a0}', "[espace insécable]").replace('\u{202f}', "[espace fine insécable]")
}

/// Keep a version of the book after a save the user asked for (the latest 30 are kept).
pub fn auto_version(app: &mut WordApp) {
    let Some(dir) = app.services.versions_dir.clone() else { return };
    if !wordcraft_engine::book::is_book(&app.session.doc) {
        return;
    }
    if let Err(e) = app.session.run("book.versionSave", &json!({"dir": dir.to_string_lossy(), "auto": true})) {
        log::warn!("book version: {e}");
    }
    if let Some(ctx) = app.ctx.clone() {
        let id = versions_id(app);
        ctx.data_mut(|d| d.remove::<VersionsState>(id));
    }
}

/// The Versions tab.
pub(super) fn versions_tab(app: &mut WordApp, ui: &mut Ui) {
    let t = Tokens::get(ui.ctx());
    let ctx = ui.ctx().clone();
    let Some(dir) = app.services.versions_dir.clone() else {
        ui.label("L'historique des versions n'est pas disponible ici.");
        return;
    };
    let dir_s = dir.to_string_lossy().to_string();
    let id = versions_id(app);
    let mut st = ctx.data(|d| d.get_temp::<VersionsState>(id)).unwrap_or_default();
    if st.list.is_none() {
        st.list = Some(
            app.session
                .run("book.versionList", &json!({"dir": dir_s}))
                .ok()
                .and_then(|v| v.get("versions").cloned())
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default(),
        );
    }
    ui.label(egui::RichText::new("Nom de la version").font(regular(11.5)).color(t.text_dim));
    let mut save = false;
    ui.horizontal(|ui| {
        let edit =
            ui.add(egui::TextEdit::singleline(&mut st.label).hint_text("Par exemple : avant relecture").desired_width(ui.available_width() - 90.0));
        save |= edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        save |= ui.button("Enregistrer").on_hover_text("Garder une copie complète du livre tel qu'il est maintenant").clicked();
    });
    ui.label(
        egui::RichText::new("Une version est aussi gardée à chaque enregistrement du livre (les 30 dernières). Tout reste sur cet ordinateur.")
            .font(regular(11.0))
            .color(t.text_dim),
    );
    if let Some(n) = &st.note {
        ui.label(egui::RichText::new(n).font(regular(11.5)).color(t.text));
    }
    ui.separator();
    let mut open: Option<String> = None;
    let list = st.list.clone().unwrap_or_default();
    if list.is_empty() {
        ui.label(egui::RichText::new("Aucune version pour l'instant.").font(regular(12.0)).color(t.text_dim));
    }
    egui::ScrollArea::vertical().id_salt("lusk_versions_scroll").auto_shrink([false, false]).show(ui, |ui| {
        for v in &list {
            egui::Frame::NONE.inner_margin(Margin::symmetric(4, 4)).show(ui, |ui| {
                let name = if v.auto {
                    egui::RichText::new(&v.label).font(regular(12.0)).color(t.text_dim)
                } else {
                    egui::RichText::new(&v.label).font(semibold(12.5)).color(t.text)
                };
                ui.add(egui::Label::new(name).wrap());
                ui.label(
                    egui::RichText::new(format!("{} · {} mots", french_date(&v.date), super::group_digits(v.words)))
                        .font(regular(11.0))
                        .color(t.text_dim),
                );
                ui.horizontal(|ui| {
                    if ui
                        .small_button("Ouvrir")
                        .on_hover_text("Ouvrir cette version dans un nouveau document ; le livre en cours ne change pas")
                        .clicked()
                    {
                        open = Some(v.id.clone());
                    }
                    if ui.small_button("Supprimer…").on_hover_text("Supprimer cette version de cet ordinateur").clicked() {
                        st.confirm_delete = Some((v.id.clone(), v.label.clone()));
                    }
                });
            });
            ui.separator();
        }
    });
    if save {
        match app.session.run("book.versionSave", &json!({"dir": dir_s, "label": st.label})) {
            Ok(v) => {
                st.note = Some(format!("Version « {} » enregistrée.", v.get("label").and_then(Value::as_str).unwrap_or_default()));
                st.label.clear();
                st.list = None;
            }
            Err(e) => app.status(e.to_string()),
        }
    }
    ctx.data_mut(|d| d.insert_temp(id, st));
    if let Some(vid) = open {
        run(app, "book.versionOpen", json!({"dir": dir_s, "id": vid}));
    }
}

/// `2026-11-06T08:30:00Z` → `06/11/2026 à 09:30` (heure de l'ordinateur).
fn french_date(utc: &str) -> String {
    let local = crate::local_time::utc_time(utc);
    match (local.get(0..4), local.get(5..7), local.get(8..10), local.get(11..16)) {
        (Some(y), Some(m), Some(d), Some(hm)) => format!("{d}/{m}/{y} à {hm}"),
        _ => utc.to_string(),
    }
}

/// Confirm before a version is deleted (it can't be undone).
pub(super) fn confirm_version_delete(app: &mut WordApp, ctx: &egui::Context) {
    let id = versions_id(app);
    let mut st = ctx.data(|d| d.get_temp::<VersionsState>(id)).unwrap_or_default();
    let Some((vid, label)) = st.confirm_delete.clone() else { return };
    let t = Tokens::get(ctx);
    let mut done = false;
    let mut delete = false;
    let modal = egui::Modal::new(id.with("confirm"))
        .frame(egui::Frame::NONE.fill(t.panel).corner_radius(CornerRadius::same(10)).inner_margin(Margin::same(20)))
        .show(ctx, |ui| {
            ui.set_width(400.0);
            ui.label(egui::RichText::new("Supprimer cette version ?").font(semibold(16.0)).color(t.text));
            ui.add_space(6.0);
            ui.label(egui::RichText::new(format!("La version « {label} » sera supprimée de cet ordinateur.")).font(regular(13.0)).color(t.text));
            ui.label(
                egui::RichText::new("Cette suppression ne peut pas être annulée. Le livre en cours ne change pas.")
                    .font(regular(12.0))
                    .color(t.text_dim),
            );
            ui.add_space(14.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let danger = egui::Button::new(egui::RichText::new("Supprimer").font(semibold(13.0)).color(Color32::WHITE)).fill(RED);
                if ui.add(danger.min_size(vec2(110.0, 30.0))).clicked() {
                    delete = true;
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
    if delete && let Some(dir) = app.services.versions_dir.clone() {
        match app.session.run("book.versionDelete", &json!({"dir": dir.to_string_lossy(), "id": vid})) {
            Ok(_) => st.note = Some(format!("Version « {label} » supprimée.")),
            Err(e) => app.status(e.to_string()),
        }
        st.list = None;
    }
    if done {
        st.confirm_delete = None;
        ctx.data_mut(|d| d.insert_temp(id, st));
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::kittest::Queryable;
    use serde_json::json;
    use wordcraft_engine::Session;
    use wordcraft_engine::book::{self, BookSettings};

    use crate::{Services, WordApp};

    fn book_app(versions: Option<std::path::PathBuf>) -> WordApp {
        let mut base = wordcraft_doc::Document::new();
        base.styles.default_chr.lang = Some("fr-FR".into());
        let doc = book::new_document(base, &BookSettings { title: "Essai".into(), author: "Jeanne".into(), ..BookSettings::default() });
        let mut a = WordApp::new(Session::new(doc), Services { versions_dir: versions, ..Services::default() });
        a.session.run("book.insert", &json!({"kind": "chapter", "title": "Un"})).unwrap();
        a.session.run("text.insert", &json!({"text": "Bonjour  le monde"})).unwrap();
        a
    }

    fn harness(a: WordApp) -> egui_kittest::Harness<'static, WordApp> {
        egui_kittest::Harness::builder().with_size(egui::vec2(1440.0, 900.0)).build_ui_state(
            |ui, app: &mut WordApp| {
                let ctx = ui.ctx().clone();
                app.logic(&ctx);
                app.ui(ui);
            },
            a,
        )
    }

    /// The Qualité tab lists what the analysis found, changes nothing by itself, and applies a
    /// correction only when « Corriger » is clicked (one undo step).
    #[test]
    fn the_quality_tab_fixes_only_on_request() {
        let mut h = harness(book_app(None));
        h.run_steps(3);
        h.state_mut().run("ui.bookPaneTab", json!({"tab": "qualite"})).unwrap();
        h.run_steps(4);
        assert!(h.query_by_label("Doubles espaces (1)").is_some(), "the double space is listed");
        assert!(h.state().session.doc.plain_text(wordcraft_doc::StoryRef::Body).contains("Bonjour  le monde"), "nothing changed yet");
        let fix = h.query_all_by_label("Corriger").count();
        assert_eq!(fix, 1);
        h.get_by_label("Corriger").click();
        h.run_steps(4);
        let text = h.state().session.doc.plain_text(wordcraft_doc::StoryRef::Body);
        assert!(text.contains("Bonjour le monde"), "{text}");
        assert!(h.query_by_label("1 correction faite — Ctrl+Z pour annuler.").is_some());
        assert!(h.query_by_label("Doubles espaces (1)").is_none(), "analysed again");
        h.state_mut().run("edit.undo", json!({})).unwrap();
        assert!(h.state().session.doc.plain_text(wordcraft_doc::StoryRef::Body).contains("Bonjour  le monde"), "one undo step");
    }

    /// The Versions tab keeps a version on request and opens it as a new document.
    #[test]
    fn the_versions_tab_saves_and_opens_a_copy() {
        let dir = std::env::temp_dir().join(format!("lusk-ui-versions-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut h = harness(book_app(Some(dir.clone())));
        h.run_steps(3);
        h.state_mut().run("ui.bookPaneTab", json!({"tab": "versions"})).unwrap();
        h.run_steps(3);
        assert!(h.query_by_label("Aucune version pour l'instant.").is_some());
        h.get_by_label("Enregistrer").click();
        h.run_steps(4);
        assert_eq!(h.query_all_by_label("Ouvrir").count(), 1);
        let docs = h.state().session.documents().len();
        h.get_by_label("Ouvrir").click();
        h.run_steps(4);
        assert_eq!(h.state().session.documents().len(), docs + 1, "a new document");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
