#![windows_subsystem = "windows"]
pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");
pub mod lang;
pub mod logging;
pub use logging::{log_info, log_warn, log_error};
pub mod modules;
pub mod utils;
pub mod app_state;
pub mod settings;
pub mod jobs;
pub mod ui;
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
    log_info(&format!("=== OXYTOOLS v{} START ===", VERSION));
    let _ = modules::binaries::extraire_deps();
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
        &format!("Oxytools v{}", VERSION),
        options,
        Box::new(|cc| {
            let mut app = OxytoolsApp::default();
            app.load_config();
            app.apply_theme(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    );
    log_info("=== OXYTOOLS FERMETURE ===");
    modules::binaries::cleanup();
    result
}

