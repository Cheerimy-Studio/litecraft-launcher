//! LiteCraft Launther: a small PCL2-shaped launcher for LiteCraft.

mod app;
mod cli;
mod events;
mod job;
mod launch;
mod minecraft;
mod net;
mod paths;
mod prepare;
mod settings;
mod ui;

fn main() -> eframe::Result {
    if let Some(code) = cli::run() {
        std::process::exit(code);
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            // Logical points: 150% displays turn these into 1770x990 physical, which fits
            // a 1080p screen. The previous size overflowed it and cut the right column.
            .with_inner_size([1180.0, 660.0])
            .with_min_inner_size([900.0, 560.0])
            .with_title("LiteCraft Launther"),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "LiteCraft Launther",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
