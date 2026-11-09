//! Software updates, background check at startup, and startup prompt modal.

use crate::WordApp;

#[cfg(not(target_arch = "wasm32"))]
use craft_update::updater::{Action, Status, Updater};

/// Lusk Texte : where its releases are published (GitHub Releases of `declic29/LuskTX`), instead
/// of WordCraft's. Release files are named `Lusk-Texte-<version>-<os>-<arch>.<ext>`; the id stays
/// `wordcraft`, the program's file name (`wordcraft.exe`), which the updater looks for too, and
/// it also accepts files named after it (`wordcraft-<version>-…`, the first 1.0.0 builds).
#[cfg(not(target_arch = "wasm32"))]
pub const LUSK_TEXTE: craft_update::app::App = craft_update::app::App {
    id: "wordcraft",
    name: "Lusk Texte",
    tagline: "Traitement de texte",
    repo: "declic29/LuskTX",
    former_ids: &["Lusk-Texte"],
    former_names: &[],
    icon_png: None,
};

pub(crate) struct Runtime {
    #[cfg(not(target_arch = "wasm32"))]
    updater: Updater,
    /// The desktop app turns the check at start on; tests and the web build leave it off.
    #[cfg(not(target_arch = "wasm32"))]
    startup_check: bool,
    #[cfg(not(target_arch = "wasm32"))]
    check_requested: bool,
    /// The context of the last frame, to wake the UI when a check is queued from a menu callback.
    #[cfg(not(target_arch = "wasm32"))]
    repaint: Option<egui::Context>,
    pub(crate) manual_window_open: bool,
    pub(crate) status_message: Option<String>,
    /// Lusk Texte : the outcome showed on File › Mises à jour last frame (instead of the window).
    #[cfg(not(target_arch = "wasm32"))]
    inline: bool,
}

impl Default for Runtime {
    fn default() -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            updater: Updater::new(&LUSK_TEXTE, env!("CARGO_PKG_VERSION")),
            #[cfg(not(target_arch = "wasm32"))]
            startup_check: false,
            #[cfg(not(target_arch = "wasm32"))]
            check_requested: false,
            #[cfg(not(target_arch = "wasm32"))]
            repaint: None,
            manual_window_open: false,
            status_message: None,
            #[cfg(not(target_arch = "wasm32"))]
            inline: false,
        }
    }
}

impl WordApp {
    /// Whether the app asks for a newer release when it starts (off until the desktop app turns it on).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_startup_update_check(&mut self, on: bool) {
        self.updates_rt.startup_check = on;
    }
}

pub(crate) fn check(app: &mut WordApp) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        app.updates_rt.check_requested = true;
        if let Some(ctx) = &app.updates_rt.repaint {
            ctx.request_repaint();
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        app.updates_rt.manual_window_open = true;
        app.updates_rt.status_message = Some(tl!("Update check is not supported in the web browser.").to_string());
    }
}

pub(crate) fn tick(app: &mut WordApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let rt = &mut app.updates_rt;
        rt.repaint = Some(ctx.clone());
        if std::mem::take(&mut rt.check_requested) {
            rt.updater.check_now(ctx);
        } else if rt.startup_check {
            rt.updater.check_at_start(ctx);
        }
        rt.updater.tick(ctx);
    }
    #[cfg(target_arch = "wasm32")]
    let _ = (app, ctx);
}

pub(crate) fn show(app: &mut WordApp, ctx: &egui::Context) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        // The restart helper is scheduled before the app closes, so no update starts over unsaved work.
        let clean = !(app.session.dirty || app.save_unconfirmed() || app.printing.busy() || crate::panes::has_comment_drafts(app));
        match app.updates_rt.updater.show_if(ctx, clean) {
            Action::OpenUrl(url) => ctx.open_url(egui::OpenUrl::new_tab(url)),
            Action::Close => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Action::None => {}
        }
        // The window answers Help ▸ Check for Updates, so it follows the updater's status.
        // Lusk Texte : File › Mises à jour shows the outcome on the page instead of the window.
        let inline = on_updates_page(app);
        let rt = &mut app.updates_rt;
        (rt.manual_window_open, rt.status_message) = match rt.updater.status() {
            Status::Idle => (false, None),
            Status::Checking => (true, None),
            Status::UpToDate => {
                (true, Some(tl!("Lusk Texte {version} is currently the newest version available.").replace("{version}", env!("CARGO_PKG_VERSION"))))
            }
            Status::Failed(e) => (true, Some(tl!("Could not check for updates:\n{error}").replace("{error}", e))),
        };
        if inline {
            rt.manual_window_open = false;
            rt.inline = true;
        } else if std::mem::take(&mut rt.inline) {
            // Leaving the page dismisses the outcome it showed, as closing the window does.
            rt.updater.dismiss_status();
            (rt.manual_window_open, rt.status_message) = (false, None);
        }
    }

    if app.updates_rt.manual_window_open {
        let mut open = true;
        let mut close = false;
        egui::Window::new(tl!("Check for Updates"))
            .open(&mut open)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                if let Some(msg) = &app.updates_rt.status_message {
                    ui.label(msg);
                } else {
                    ui.label(tl!("Checking for updates…"));
                }
                ui.add_space(8.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tl!("Close")).clicked() {
                        close = true;
                    }
                });
            });
        if !open || close {
            app.updates_rt.manual_window_open = false;
            app.updates_rt.status_message = None;
        }
    }

    // Closing the window dismisses the outcome it showed.
    #[cfg(not(target_arch = "wasm32"))]
    if !app.updates_rt.manual_window_open && !app.updates_rt.inline {
        app.updates_rt.updater.dismiss_status();
    }
}

/// Lusk Texte : whether File › Mises à jour is showing.
#[cfg(not(target_arch = "wasm32"))]
fn on_updates_page(app: &WordApp) -> bool {
    app.ui.backstage && app.ui.backstage_page == "updates"
}

/// Lusk Texte : File › Mises à jour. The version installed, a button that looks for a newer one
/// on the association's GitHub releases, and the check at start. A newer version opens the
/// updater's own window, which downloads it, checks its fingerprint and installs it once every
/// document is saved.
pub(crate) fn page(app: &mut WordApp, ui: &mut egui::Ui) {
    ui.label(egui::RichText::new("Mises à jour").font(crate::theme::semibold(26.0)));
    ui.add_space(16.0);
    ui.label(egui::RichText::new(format!("Lusk Texte {}", env!("CARGO_PKG_VERSION"))).font(crate::theme::semibold(16.0)));
    ui.label("Version installée sur cet ordinateur.");
    ui.add_space(20.0);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let checking = matches!(app.updates_rt.updater.status(), Status::Checking);
        ui.horizontal(|ui| {
            if ui.add_enabled(!checking, egui::Button::new("Rechercher des mises à jour").min_size(egui::vec2(0.0, 30.0))).clicked() {
                check(app);
            }
            if checking {
                ui.add(egui::Spinner::new());
                ui.label("Recherche en cours…");
            }
        });
        // The outcome, in the page's words rather than the window's.
        let outcome = match app.updates_rt.updater.status() {
            Status::UpToDate => Some(format!("Vous avez la dernière version de Lusk Texte ({}).", env!("CARGO_PKG_VERSION"))),
            Status::Failed(e) if e == "no releases yet" => Some("Aucune version n’a encore été publiée sur GitHub.".to_string()),
            Status::Failed(e) => Some(format!("La recherche n’a pas abouti. Vérifiez la connexion à Internet, puis réessayez.\n(Détail : {e})")),
            Status::Idle | Status::Checking => None,
        };
        if let Some(outcome) = outcome {
            ui.add_space(8.0);
            ui.label(outcome);
        }
        ui.add_space(20.0);
        let mut at_start = app.ui.check_updates_at_start;
        if ui.checkbox(&mut at_start, "Rechercher automatiquement les mises à jour au démarrage").changed() {
            // Saved with the other preferences when the app closes; it applies at the next start.
            app.ui.check_updates_at_start = at_start;
        }
        ui.add_space(20.0);
        ui.label(
            "Les nouvelles versions sont publiées par l’association Lusk. Quand une version plus récente \
             existe, Lusk Texte propose de la télécharger et vérifie son empreinte avant de l’installer. \
             L’installation attend que tous vos documents soient enregistrés.",
        );
        ui.add_space(8.0);
        ui.label("Seul le numéro de la dernière version est demandé : aucun document ni aucune donnée personnelle n’est envoyé.");
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = app;
        ui.label("Dans le navigateur, Lusk Texte est toujours à jour : la page charge la dernière version.");
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod lusk_tests {
    use super::LUSK_TEXTE;
    use craft_update::platform::parse_asset_name;

    /// Lusk Texte : the updater recognizes the release files named `Lusk-Texte-<version>-…`, and
    /// still the first builds' `wordcraft-<version>-…`, but not the copies without a version.
    #[test]
    fn release_files_named_lusk_texte_are_found() {
        let stems: Vec<&str> = LUSK_TEXTE.stems().collect();
        let a = parse_asset_name("Lusk-Texte-1.1.0-windows-x64.msi", &stems).unwrap();
        assert_eq!((a.version.as_str(), a.token.as_str()), ("1.1.0", "windows-x64"));
        assert!(parse_asset_name("Lusk-Texte-1.1.0-macos-universal.dmg", &stems).is_some());
        assert!(parse_asset_name("Lusk-Texte-1.1.0-linux-x86_64.AppImage", &stems).is_some());
        assert!(parse_asset_name("wordcraft-1.0.0-windows-x64.msi", &stems).is_some());
        assert!(parse_asset_name("Lusk-Texte-Windows.msi", &stems).is_none());
        assert!(parse_asset_name("Lusk-Texte-cli-1.1.0-macos-universal.zip", &stems).is_none());
    }
}
