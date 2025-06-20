// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use csv::Writer;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;

use crate::lef::{Lef, LefMacro, LefPin};

#[derive(Debug, Serialize)]
pub struct PinCsvRecord {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Direction")]
    pub direction: String,
    #[serde(rename = "Type")]
    pub pin_type: String,
    #[serde(rename = "Width")]
    pub width: usize,
}

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

/// Extract bus information from pin name
/// Returns Some((base_name, index)) if pin follows bus pattern, None otherwise
fn extract_bus_info(pin_name: &str) -> Option<(String, usize)> {
    // Try pattern with square brackets: base_name[index]
    if let Some(bracket_start) = pin_name.rfind('[') {
        if let Some(bracket_end) = pin_name.rfind(']') {
            if bracket_start < bracket_end {
                let base_name = pin_name[..bracket_start].to_string();
                let index_str = &pin_name[bracket_start + 1..bracket_end];
                if let Ok(index) = index_str.parse::<usize>() {
                    return Some((base_name, index));
                }
            }
        }
    }

    // Try pattern with angle brackets: base_name<index>
    if let Some(angle_start) = pin_name.rfind('<') {
        if let Some(angle_end) = pin_name.rfind('>') {
            if angle_start < angle_end {
                let base_name = pin_name[..angle_start].to_string();
                let index_str = &pin_name[angle_start + 1..angle_end];
                if let Ok(index) = index_str.parse::<usize>() {
                    return Some((base_name, index));
                }
            }
        }
    }

    None
}

/// Group pins by bus base name and validate bus constraints
fn group_pins_by_bus(pins: &[LefPin]) -> Vec<Vec<&LefPin>> {
    let mut base_name_groups: HashMap<String, Vec<&LefPin>> = HashMap::new();
    let mut single_pins: Vec<&LefPin> = Vec::new();

    // Group pins by base name
    for pin in pins {
        if let Some((base_name, _index)) = extract_bus_info(&pin.name) {
            base_name_groups.entry(base_name).or_default().push(pin);
        } else {
            single_pins.push(pin);
        }
    }

    let mut result = Vec::new();

    // Add single pins as individual groups
    for pin in single_pins {
        result.push(vec![pin]);
    }

    // Process bus groups
    for (_base_name, group) in base_name_groups {
        if group.len() < 2 {
            // Single pin, add as individual group
            result.extend(group.into_iter().map(|pin| vec![pin]));
            continue;
        }

        // Check if all pins in group have same direction and use_type
        let first_direction = &group[0].direction;
        let first_use_type = &group[0].use_type;

        let attributes_consistent = group
            .iter()
            .all(|pin| pin.direction == *first_direction && pin.use_type == *first_use_type);

        if !attributes_consistent {
            // Split into individual pins
            result.extend(group.into_iter().map(|pin| vec![pin]));
            continue;
        }

        // Extract indices and check continuity
        let mut indices: Vec<usize> = group
            .iter()
            .filter_map(|pin| extract_bus_info(&pin.name).map(|(_, index)| index))
            .collect();

        if indices.len() != group.len() {
            // Some pins couldn't be parsed, split into individual pins
            result.extend(group.into_iter().map(|pin| vec![pin]));
            continue;
        }

        indices.sort();
        let min_index = indices[0];
        let max_index = indices[indices.len() - 1];
        let expected_count = max_index - min_index + 1;

        if indices.len() == expected_count {
            // Continuous bus, add as single group
            result.push(group);
        } else {
            // Not continuous, split into individual pins
            result.extend(group.into_iter().map(|pin| vec![pin]));
        }
    }

    result
}

/// Compress a bus group into a single PinCsvRecord
fn compress_bus_group(pins: &[&LefPin]) -> PinCsvRecord {
    if pins.len() == 1 {
        // Single pin
        let pin = pins[0];
        return PinCsvRecord {
            name: pin.name.clone(),
            direction: pin.direction.clone(),
            pin_type: pin.use_type.clone(),
            width: 1,
        };
    }

    // Multi-pin bus
    let first_pin = pins[0];
    let base_name = extract_bus_info(&first_pin.name).unwrap().0;

    // Get all indices and find range
    let mut indices: Vec<usize> = pins
        .iter()
        .filter_map(|pin| extract_bus_info(&pin.name).map(|(_, index)| index))
        .collect();
    indices.sort();

    let min_index = indices[0];
    let max_index = indices[indices.len() - 1];
    let width = max_index - min_index + 1;

    PinCsvRecord {
        name: format!("{}[{}:{}]", base_name, max_index, min_index),
        direction: first_pin.direction.clone(),
        pin_type: first_pin.use_type.clone(),
        width,
    }
}

/// Format pins into a compressed comma-separated string of "DIRECTION:NAME" format
fn format_pinlist_compressed(pins: &[LefPin]) -> String {
    let groups = group_pins_by_bus(pins);
    let records: Vec<PinCsvRecord> = groups
        .iter()
        .map(|group| compress_bus_group(group))
        .collect();

    records
        .iter()
        .map(|record| format!("{}:{}", record.direction, record.name))
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
        pinlist: format_pinlist_compressed(&macro_def.pins),
    }
}

/// Export single cell's pinlist to CSV file
pub fn export_cell_pinlist_to_csv(
    macro_def: &LefMacro,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(file_path)?;
    let mut writer = Writer::from_writer(file);

    let groups = group_pins_by_bus(&macro_def.pins);
    for group in groups {
        let record = compress_bus_group(&group);
        writer.serialize(record)?;
    }

    writer.flush()?;
    Ok(())
}

/// Export multiple cells' pinlists to separate CSV files in output directory
pub fn export_multiple_cells_pinlist(
    macros: &[&LefMacro],
    output_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(output_dir)?;

    for macro_def in macros {
        let file_name = format!("{}.csv", macro_def.name);
        let file_path = std::path::Path::new(output_dir).join(file_name);
        export_cell_pinlist_to_csv(macro_def, &file_path.to_string_lossy())?;
    }

    Ok(())
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
