// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use csv::Writer;
use serde::Serialize;
use std::fs::File;

use crate::lef::{Lef, LefMacro, LefPin};

#[derive(Debug, Serialize)]
pub struct MacroCsvRecord {
    #[serde(rename = "Micro")]
    pub macro_name: String,
    #[serde(rename = "Class")]
    pub class: String,
    #[serde(rename = "Size")]
    pub size: String,
    #[serde(rename = "Pins")]
    pub pins: usize,
    #[serde(rename = "Area")]
    pub area: f64,
    #[serde(rename = "Pinlist")]
    pub pinlist: String,
}

/// Format pins into a comma-separated string of "DIRECTION:NAME" format
fn format_pinlist(pins: &[LefPin]) -> String {
    pins.iter()
        .map(|pin| format!("{}:{}", pin.direction, pin.name))
        .collect::<Vec<String>>()
        .join(",")
}

/// Convert a LefMacro to a MacroCsvRecord
fn macro_to_csv_record(macro_def: &LefMacro) -> MacroCsvRecord {
    MacroCsvRecord {
        macro_name: macro_def.name.clone(),
        class: macro_def.class.clone(),
        size: format!("{:.3} x {:.3}", macro_def.size_x, macro_def.size_y),
        pins: macro_def.pins.len(),
        area: macro_def.size_x * macro_def.size_y,
        pinlist: format_pinlist(&macro_def.pins),
    }
}

/// Export LEF data to CSV file
pub fn export_lef_to_csv(
    lef_data: &Lef,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(file_path)?;
    let mut writer = Writer::from_writer(file);

    for macro_def in &lef_data.macros {
        let record = macro_to_csv_record(macro_def);
        writer.serialize(record)?;
    }

    writer.flush()?;
    Ok(())
}
