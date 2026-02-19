#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod serial_port;
mod settings;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("UART Console")
            .with_inner_size([1200.0, 720.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "UART Console",
        native_options,
        Box::new(|cc| Ok(Box::new(app::UartConsoleApp::new(cc)))),
    )
}
