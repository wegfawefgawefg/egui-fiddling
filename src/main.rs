pub mod sketch;

use eframe::{NativeOptions, Renderer, Result};

fn main() -> Result<()> {
    let opts = NativeOptions {
        renderer: Renderer::Wgpu,
        ..Default::default()
    };

    // run_native expects Result<Box<dyn App>, _> in 0.31
    eframe::run_native(
        "Scene Tree (egui + wgpu)",
        opts,
        Box::new(|cc| Ok(Box::new(sketch::AppState::new(cc)))),
    )
}
