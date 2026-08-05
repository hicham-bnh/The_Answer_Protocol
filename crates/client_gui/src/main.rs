use eframe::egui;

struct TapClient {
    server_address: String,
}

impl TapClient {
    fn new(address: String) -> TapClient {
        TapClient {
            server_address: address,
        }
    }

    fn describe(&self) {
        println!("{}", self.server_address)
    }
}

impl eframe::App for TapClient {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("TAP client");
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "TAP",
        options,
        Box::new(|_cc| Ok(Box::new(TapClient::new("127.0.0.1:8080".to_string())))),
    )
}
