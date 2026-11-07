//! L'écran d'accueil de Lusk Texte : une fenêtre rectangulaire sans bordure, centrée, qui apparaît
//! en fondu, montre le logo (en fondu lui aussi), le slogan, les mentions légales et une barre de
//! chargement, puis disparaît en fondu et laisse place à l'application.
//!
//! La barre suit le vrai chargement : l'interface, le catalogue des polices installées (lancé en
//! arrière-plan dès la première image) et la première mise en page du document. Elle reste au
//! moins [`MIN_SECS`] à l'écran pour qu'on la voie ; si les polices tardent, l'écran n'attend
//! jamais plus de [`MAX_WAIT_SECS`].
//!
//! L'application de bureau ouvre sa première fenêtre à la taille de cet écran
//! ([`SIZE`]) ; quand [`Splash::ui`] dit qu'il a fini, elle redonne à la fenêtre ses bordures et
//! sa taille.

use egui::{Align2, Color32, CornerRadius, FontId, Rect, Sense, pos2, vec2};

use crate::WordApp;
use crate::theme::{regular, semibold};

/// Taille de l'écran d'accueil (points), au format de son image (16:9).
pub const SIZE: [f32; 2] = [720.0, 405.0];
/// Durée minimale d'affichage de la barre.
pub const MIN_SECS: f64 = 1.5;
/// Au-delà, l'écran n'attend plus le catalogue des polices.
pub const MAX_WAIT_SECS: f64 = 8.0;
const FADE_IN: f64 = 0.45;
const FADE_OUT: f64 = 0.45;
const LOGO_DELAY: f64 = 0.25;
const LOGO_FADE: f64 = 0.7;
/// Un court instant barre pleine avant le fondu de sortie.
const HOLD: f64 = 0.15;

// ─── Textes de l'écran d'accueil : à modifier ici ──────────────────────────────────────────
// `{version}` est remplacé par le numéro de version de Lusk Texte. Après une modification,
// recompilez (`cargo run --release -p wordcraft`) pour voir le résultat.

/// Le nom, à droite du logo.
pub const TITLE: &str = "Lusk Texte";
/// Le slogan, sous le nom.
pub const SLOGAN: &str = "« Ce que vous écrivez vous appartient. »";
/// Mentions légales, première ligne, à gauche.
pub const LEGAL_LEFT: &str = "Version {version}  ·  Portée par une association loi 1901";
/// Mentions légales, première ligne, à droite.
pub const LEGAL_RIGHT: &str = "© 2026 Lusk Texte";
/// Mentions légales, seconde ligne, à gauche (laisser vide pour n'avoir qu'une ligne).
pub const LEGAL_SECOND: &str = "Logiciel libre sous licence MIT ou Apache 2.0";

// ──────────────────────────────────────────────────────────────────────────────────────────

const NAVY: Color32 = Color32::from_rgb(0x0B, 0x3B, 0x66);
const INK: Color32 = Color32::from_rgb(0x1B, 0x4F, 0x7A);

/// Où en est l'écran d'accueil.
#[derive(Debug, Default)]
pub struct Splash {
    start: Option<f64>,
    fonts_ready: bool,
    laid_out: bool,
    shown: f32,
    full_at: Option<f64>,
    /// Effets d'animation coupés (réglage ou système) : pas de fondu, la barre seule.
    still: bool,
}

impl Splash {
    pub fn new(animations: bool) -> Self {
        Splash { still: !animations, ..Splash::default() }
    }

    /// Dessine l'écran d'accueil pour cette image. `true` quand il a fini (fondu de sortie
    /// terminé) : l'application prend alors la fenêtre.
    pub fn ui(&mut self, app: &mut WordApp, ui: &mut egui::Ui) -> bool {
        let ctx = ui.ctx().clone();
        let now = ctx.input(|i| i.time);
        let start = *self.start.get_or_insert_with(|| {
            wordcraft_fonts::FontDb::global().scan_in_background();
            now
        });
        let t = now - start;
        // L'étape en cours et la part du chargement qu'elle représente.
        if !self.fonts_ready && (wordcraft_fonts::FontDb::global().catalog_ready() || t > MAX_WAIT_SECS) {
            self.fonts_ready = true;
        }
        let (target, status) = if !self.fonts_ready {
            // Le catalogue avance sans dire où il en est : la barre s'en approche doucement.
            (0.2 + 0.5 * (1.0 - (-t / 1.2).exp()) as f32, "Chargement des polices…")
        } else if !self.laid_out {
            if app.fonts_frames >= 2 {
                let _ = app.session.layout();
                self.laid_out = true;
            }
            (0.8, "Mise en page du document…")
        } else {
            (1.0, "Prêt")
        };
        let target = if app.fonts_frames < 2 { 0.1 } else { target };
        // Jamais plus vite que la durée minimale, jamais en arrière.
        let paced = target.min((t / MIN_SECS) as f32);
        self.shown = self.shown.max(paced).min(1.0);
        if self.shown >= 1.0 && self.full_at.is_none() {
            self.full_at = Some(now);
        }
        let out = self.full_at.map_or(0.0, |f| ((now - f - HOLD) / FADE_OUT).clamp(0.0, 1.0));
        let alpha = if self.still {
            1.0
        } else {
            let inn = (t / FADE_IN).clamp(0.0, 1.0);
            (ease(inn) * (1.0 - ease(out))) as f32
        };
        let logo = if self.still { 1.0 } else { ease(((t - LOGO_DELAY) / LOGO_FADE).clamp(0.0, 1.0)) as f32 * alpha };
        // Chargé, la barre finit de se remplir à son rythme.
        let status = if target >= 1.0 && self.shown < 1.0 { "Démarrage…" } else { status };
        paint(ui, app, Paint { alpha, logo, progress: self.shown, status });
        // Les polices de l'interface sont installées par `logic` et servent à l'image suivante.
        if app.fonts_frames < 2 {
            app.fonts_frames += 1;
        }
        let done = self.full_at.is_some_and(|f| now - f >= HOLD + if self.still { 0.0 } else { FADE_OUT });
        if !done {
            ctx.request_repaint();
        }
        done
    }
}

/// Un texte de l'écran avec le numéro de version à la place de `{version}`.
fn with_version(text: &str) -> String {
    text.replace("{version}", env!("CARGO_PKG_VERSION"))
}

fn ease(p: f64) -> f64 {
    // Doux au départ et à l'arrivée.
    p * p * (3.0 - 2.0 * p)
}

struct Paint<'a> {
    alpha: f32,
    logo: f32,
    progress: f32,
    status: &'a str,
}

/// La texture de l'image de fond, chargée une fois.
fn background(ctx: &egui::Context) -> egui::TextureHandle {
    let id = egui::Id::unique("lusk_splash_background");
    if let Some(h) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return h;
    }
    // Une image illisible (impossible avec le fichier intégré) donne un fond bleu, jamais un plantage.
    let img = image::load_from_memory(include_bytes!("../../../assets/splash/accueil-lusk-texte.jpg"))
        .map(|i| i.to_rgba8())
        .unwrap_or_else(|_| image::RgbaImage::from_pixel(1, 1, image::Rgba([0x1E, 0x88, 0xE5, 255])));
    let h = ctx.load_texture(
        "lusk_splash_background",
        egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw()),
        egui::TextureOptions::LINEAR,
    );
    ctx.data_mut(|d| d.insert_temp(id, h.clone()));
    h
}

fn paint(ui: &mut egui::Ui, app: &WordApp, p: Paint<'_>) {
    let ctx = ui.ctx().clone();
    let r = ctx.content_rect();
    let (_, resp) = ui.allocate_exact_size(r.size(), Sense::hover());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Image, true, format!("Lusk Texte — {}", p.status)));
    let painter = ui.painter();
    let fade = |c: Color32, a: f32| c.gamma_multiply(a.clamp(0.0, 1.0));
    let corner = CornerRadius::same(10);
    let bg = background(&ctx);
    painter.add(
        egui::epaint::RectShape::filled(r, corner, fade(Color32::WHITE, p.alpha))
            .with_texture(bg.id(), Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))),
    );
    // Le texte attend les polices de l'interface (installées par `logic` à la première image).
    if app.fonts_frames < 2 {
        return;
    }
    let (w, h) = (r.width(), r.height());
    let text = |pos: egui::Pos2, anchor: Align2, s: &str, font: FontId, c: Color32| {
        painter.text(pos, anchor, s, font, c);
    };
    // Le logo et le nom, sur la partie claire de l'image.
    let logo = crate::chrome::account_logo(&ctx);
    let size = logo.size_vec2();
    let lh = 92.0;
    let lw = lh * size.x / size.y.max(1.0);
    let center = pos2(r.min.x + w * 0.56, r.min.y + h * 0.40);
    let title_w = 236.0;
    let left = center.x - (lw + 18.0 + title_w) / 2.0;
    let logo_rect = Rect::from_min_size(pos2(left, center.y - lh / 2.0 - 6.0), vec2(lw, lh));
    painter.image(logo.id(), logo_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), fade(Color32::WHITE, p.logo));
    let tx = logo_rect.max.x + 18.0;
    text(pos2(tx, center.y - 14.0), Align2::LEFT_CENTER, TITLE, semibold(42.0), fade(NAVY, p.logo));
    text(pos2(tx + 2.0, center.y + 26.0), Align2::LEFT_CENTER, SLOGAN, regular(15.5), fade(INK, p.logo));
    // La barre de chargement et l'étape en cours, sur les vagues sombres du bas.
    let white = |a: f32| fade(Color32::WHITE, a * p.alpha);
    let bar = Rect::from_min_size(pos2(r.min.x + 40.0, r.max.y - 66.0), vec2(w - 80.0, 4.0));
    painter.rect_filled(bar, CornerRadius::same(2), white(0.28));
    let fill = Rect::from_min_size(bar.min, vec2(bar.width() * p.progress.clamp(0.0, 1.0), bar.height()));
    painter.rect_filled(fill, CornerRadius::same(2), white(0.95));
    text(pos2(bar.min.x, bar.min.y - 10.0), Align2::LEFT_BOTTOM, p.status, regular(12.0), white(0.9));
    // Les mentions légales, discrètes, sur deux lignes.
    let legal = regular(10.5);
    let (y1, y2) = (r.max.y - 34.0, r.max.y - 18.0);
    text(pos2(bar.min.x, y1), Align2::LEFT_CENTER, &with_version(LEGAL_LEFT), legal.clone(), white(0.85));
    text(pos2(bar.max.x, y1), Align2::RIGHT_CENTER, &with_version(LEGAL_RIGHT), legal.clone(), white(0.85));
    text(pos2(bar.min.x, y2), Align2::LEFT_CENTER, &with_version(LEGAL_SECOND), legal, white(0.7));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> WordApp {
        WordApp::new(wordcraft_engine::Session::new(wordcraft_doc::Document::new()), crate::Services::default())
    }

    /// Runs the screen at 60 images a second; the time it says it has finished.
    fn run(splash: &mut Splash, app: &mut WordApp) -> Option<f64> {
        let ctx = egui::Context::default();
        for frame in 0..900 {
            let time = f64::from(frame) / 60.0;
            let input = egui::RawInput {
                time: Some(time),
                screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(SIZE[0], SIZE[1]))),
                ..Default::default()
            };
            let mut done = false;
            let mut out = ctx.run_ui(input, |ui| {
                app.logic(&ui.ctx().clone());
                done = splash.ui(app, ui);
            });
            out.textures_delta.clear();
            if done {
                return Some(time);
            }
        }
        None
    }

    /// The screen runs its course: it fills, fades out and says it has finished, after at least
    /// the minimum time and well before the longest wait; the document is laid out meanwhile.
    #[test]
    fn the_start_screen_fills_then_finishes() {
        let mut app = app();
        let mut splash = Splash::new(true);
        let at = run(&mut splash, &mut app).expect("the start screen finishes");
        assert!(at >= MIN_SECS + FADE_OUT, "shown at least {MIN_SECS} s then faded, finished at {at}");
        assert!(at < MAX_WAIT_SECS + 1.5, "never waits past the longest wait, finished at {at}");
        assert!(splash.laid_out, "the document was laid out while it showed");
        assert!((splash.shown - 1.0).abs() < f32::EPSILON);
    }

    /// The texts show the version and no longer name WordCraft.
    #[test]
    fn the_legal_lines_show_the_version() {
        assert_eq!(with_version(LEGAL_LEFT), format!("Version {}  ·  Portée par une association loi 1901", env!("CARGO_PKG_VERSION")));
        for t in [TITLE, SLOGAN, LEGAL_LEFT, LEGAL_RIGHT, LEGAL_SECOND] {
            assert!(!t.contains("WordCraft"), "{t}");
        }
    }

    /// With animation effects off there is no fade: it ends as soon as the bar has been full a
    /// moment.
    #[test]
    fn without_animations_it_does_not_fade() {
        let mut app = app();
        let mut splash = Splash::new(false);
        let at = run(&mut splash, &mut app).expect("the start screen finishes");
        let full = splash.full_at.unwrap_or_default();
        assert!(at - full < FADE_OUT, "no fade-out: {at} after full at {full}");
    }
}
