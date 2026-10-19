#![windows_subsystem = "windows"]
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const APP_NAME: &str = if cfg!(feature = "full") { "Oxytools" } else { "Oxytools Office" };
pub mod lang;
pub mod logging;
pub use logging::{log_info, log_warn, log_error};
pub mod modules;
pub mod utils;
pub mod app_state;
pub mod settings;
pub mod jobs;
pub mod ui;
#[cfg(feature = "full")]
pub mod shell;
use app_state::OxytoolsApp;
#[cfg(test)]
#[path = "test.rs"]
mod test;
use eframe::egui;

impl OxytoolsApp {
    fn apply_theme(&self, ctx: &egui::Context) {
        match self.current_theme.as_str() {
            "Light" => ctx.set_visuals(egui::Visuals::light()),
            "Dark" => ctx.set_visuals(egui::Visuals::dark()),
            _ => ctx.set_visuals(egui::Visuals::default()),
        }
    }
}
fn main() -> eframe::Result {
    #[cfg(feature = "full")]
    let launch = shell::launch_from_args();
    #[cfg(feature = "full")]
    if matches!(launch, shell::Launch::Exit) {
        return Ok(());
    }
    log_info(&format!("=== OXYTOOLS {} START ===", VERSION));
    let _ = modules::binaries::extraire_deps();
    #[cfg(feature = "full")]
    if let shell::Launch::Tag(action, files) = &launch {
        let mut app = OxytoolsApp::default();
        app.load_config();
        shell::run_tag(*action, files, app.lang, app.lang_id);
        log_info("=== OXYTOOLS FERMETURE ===");
        modules::binaries::cleanup();
        return Ok(());
    }
    let mut options = eframe::NativeOptions::default();
    #[cfg(target_os = "windows")]
    let icon_bytes: &[u8] = include_bytes!("../assets/Oxytools_icon.ico");
    #[cfg(not(target_os = "windows"))]
    let icon_bytes: &[u8] = include_bytes!("../assets/Oxytools_icon.png");
    if let Ok(icon_data) = image::load_from_memory(icon_bytes) {
        let icon_rgba = icon_data.to_rgba8();
        let (width, height) = icon_rgba.dimensions();
        options.viewport.icon = Some(std::sync::Arc::new(egui::IconData { rgba: icon_rgba.into_raw(), width, height }));
    }
    let result = eframe::run_native(
        &format!("{} {}", APP_NAME, VERSION),
        options,
        Box::new(|cc| {
            let mut app = OxytoolsApp::default();
            app.load_config();
            #[cfg(feature = "full")]
            if let shell::Launch::Scrap(files) = launch {
                app.open_scrap(files);
            }
            app.apply_theme(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    );
    log_info("=== OXYTOOLS FERMETURE ===");
    modules::binaries::cleanup();
    result
}
