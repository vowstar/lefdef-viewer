// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

mod def;
mod export;
mod gui;
mod lef;

use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    env_logger::init();

    let creator = Box::new(|cc: &eframe::CreationContext<'_>| {
        egui_extras::install_image_loaders(&cc.egui_ctx);
        Ok(Box::new(gui::LefDefViewer::new()) as Box<dyn eframe::App>)
    });

    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    match eframe::run_native("LEF/DEF Viewer", options.clone(), creator.clone()) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("wgpu failed: {e}, falling back to glow...");
            options.renderer = eframe::Renderer::Glow;
            eframe::run_native("LEF/DEF Viewer (OpenGL)", options, creator)
        }
    }
}
