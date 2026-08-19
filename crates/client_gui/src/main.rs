use crate::app::TapClient;
use eframe::egui;
use std::env;
use std::net::SocketAddr;
use std::sync::mpsc;
mod app;
mod net;
mod protocol;
mod state;

fn main() -> eframe::Result {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 || args[1].parse::<SocketAddr>().is_err() {
        eprintln!("Bad arguments, usage: client_gui <ip:port>, no localhost is allowed.");
        std::process::exit(1);
    }
    let addr = args[1].clone();
    let (tx, rx) = mpsc::channel::<String>();
    let (tx_out, rx_out) = mpsc::channel::<String>();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "TAP",
        options,
        Box::new(|cc| {
            net::start(addr.clone().to_string(), tx, rx_out, cc.egui_ctx.clone());
            Ok(Box::new(TapClient::new(addr, rx, tx_out)))
        }),
    )
}
