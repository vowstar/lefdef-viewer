// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use std::fs;
use std::path::Path;

use super::Lef;

pub struct LefReader;

impl LefReader {
    pub fn new() -> Self {
        Self
    }

    pub fn read<P: AsRef<Path>>(&self, path: P) -> Result<Lef, Box<dyn std::error::Error>> {
        let path_str = path.as_ref().display().to_string();
        println!("[LOAD] Loading LEF file: {path_str}");

        let content = fs::read_to_string(path)?;
        println!("[FILE] LEF file size: {} bytes", content.len());

        // Print first few lines for debugging
        let lines: Vec<&str> = content.lines().take(10).collect();
        println!("[FILE] First 10 lines:");
        for (i, line) in lines.iter().enumerate() {
            println!("  {}: {}", i + 1, line);
        }

        // Use proven nom-based parser
        println!("[DBG] Using proven nom-based LEF parser...");
        match super::lef_parser::parse_lef(&content) {
            Ok((_, lef)) => {
                println!("[PASS] LEF parsed successfully!");
                println!(
                    "[INFO] Found {} macros with complete PIN geometry data",
                    lef.macros.len()
                );

                // Detailed statistics
                let mut total_pins = 0;
                let mut total_rects = 0;
                let mut total_polygons = 0;

                for macro_def in &lef.macros {
                    total_pins += macro_def.pins.len();
                    for pin in &macro_def.pins {
                        for port in &pin.ports {
                            total_rects += port.rects.len();
                            total_polygons += port.polygons.len();
                        }
                    }
                }

                println!(
                    "[INFO] Statistics: {total_pins} pins, {total_rects} rects, {total_polygons} polygons"
                );

                Ok(lef)
            }
            Err(e) => {
                println!("[FAIL] Failed to parse LEF file: {e:?}");
                Err(format!("Failed to parse LEF file: {e:?}").into())
            }
        }
    }
}

impl Default for LefReader {
    fn default() -> Self {
        Self::new()
    }
}
