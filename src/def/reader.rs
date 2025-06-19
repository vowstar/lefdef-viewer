// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use std::fs;
use std::path::Path;

use super::{def_parser::parse_def, Def};

pub struct DefReader;

impl DefReader {
    pub fn new() -> Self {
        Self
    }

    pub fn read<P: AsRef<Path>>(&self, path: P) -> Result<Def, Box<dyn std::error::Error>> {
        let path_str = path.as_ref().display().to_string();
        println!("🔍 Loading DEF file: {}", path_str);

        let content = fs::read_to_string(path)?;
        println!("📄 DEF file size: {} bytes", content.len());

        // Print first few lines for debugging
        let lines: Vec<&str> = content.lines().take(10).collect();
        println!("📋 First 10 lines:");
        for (i, line) in lines.iter().enumerate() {
            println!("  {}: {}", i + 1, line);
        }

        match parse_def(&content) {
            Ok((remaining, def)) => {
                println!("✅ DEF parsed successfully!");
                println!("📊 Die area points: {}", def.die_area_points.len());
                println!("📊 Components: {}", def.components.len());
                println!("📊 Pins: {}", def.pins.len());
                println!("📊 Nets: {}", def.nets.len());
                if !remaining.trim().is_empty() {
                    println!("⚠️  Unparsed content remaining: {} chars", remaining.len());
                }
                Ok(def)
            }
            Err(e) => {
                println!("❌ Failed to parse DEF file: {:?}", e);
                Err(format!("Failed to parse DEF file: {:?}", e).into())
            }
        }
    }
}

impl Default for DefReader {
    fn default() -> Self {
        Self::new()
    }
}
