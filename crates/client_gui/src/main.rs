use crate::app::TapClient;
use eframe::egui;
use std::sync::mpsc;
mod app;
mod net;
mod state;

fn main() -> eframe::Result {
    let (tx, rx) = mpsc::channel::<String>();
    net::start("127.0.0.1:8080".to_string(), tx);
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "TAP",
        options,
        Box::new(|_cc| Ok(Box::new(TapClient::fake(rx)))),
    )
}
