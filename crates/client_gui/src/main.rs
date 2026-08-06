use eframe::egui;
use crate::app::TapClient;
mod app;
mod state;


fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "TAP",
        options,
        Box::new(|_cc| Ok(Box::new(TapClient::fake()))),
    )
}
