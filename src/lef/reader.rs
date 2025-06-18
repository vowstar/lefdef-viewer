// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use std::fs;
use std::path::Path;

use super::{parser::parse_lef, Lef};

pub struct LefReader;

impl LefReader {
    pub fn new() -> Self {
        Self
    }

    pub fn read<P: AsRef<Path>>(&self, path: P) -> Result<Lef, Box<dyn std::error::Error>> {
        let path_str = path.as_ref().display().to_string();
        println!("🔍 Loading LEF file: {}", path_str);

        let content = fs::read_to_string(path)?;
        println!("📄 LEF file size: {} bytes", content.len());

        // Print first few lines for debugging
        let lines: Vec<&str> = content.lines().take(10).collect();
        println!("📋 First 10 lines:");
        for (i, line) in lines.iter().enumerate() {
            println!("  {}: {}", i + 1, line);
        }

        match parse_lef(&content) {
            Ok((remaining, lef)) => {
                println!("✅ LEF parsed successfully!");
                println!("📊 Found {} macros", lef.macros.len());
                for (i, macro_def) in lef.macros.iter().enumerate().take(5) {
                    println!(
                        "  Macro {}: {} (size: {:.3}x{:.3})",
                        i + 1,
                        macro_def.name,
                        macro_def.size_x,
                        macro_def.size_y
                    );
                }
                if !remaining.trim().is_empty() {
                    println!("⚠️  Unparsed content remaining: {} chars", remaining.len());
                }
                Ok(lef)
            }
            Err(e) => {
                println!("❌ Failed to parse LEF file: {:?}", e);
                Err(format!("Failed to parse LEF file: {:?}", e).into())
            }
        }
    }
}

impl Default for LefReader {
    fn default() -> Self {
        Self::new()
    }
}
