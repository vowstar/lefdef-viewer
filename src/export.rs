// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use csv::Writer;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};
use std::fs::File;
use std::io::Write;

use crate::lef::{Lef, LefMacro, LefPin};

/// Voltage configuration for Liberty export
#[derive(Debug, Clone)]
pub struct VoltageConfig {
    pub power_pins: std::collections::BTreeMap<String, f32>, // power pin name -> voltage
    pub ground_pins: std::collections::BTreeMap<String, f32>, // ground pin name -> voltage
    pub selected_related_power: String,                      // default related power pin
    pub selected_related_ground: String,                     // default related ground pin
    pub nom_voltage: f32,                                    // nominal voltage
    pub lib_name: String,                                    // library name for .lib export
    // Pin-specific related power/ground configuration
    pub pin_related_power: std::collections::BTreeMap<String, String>, // pin_name -> related_power_pin
    pub pin_related_ground: std::collections::BTreeMap<String, String>, // pin_name -> related_ground_pin
    // Pin-specific analog and pad configuration
    pub pin_is_analog: std::collections::BTreeMap<String, bool>, // pin_name -> is_analog
    pub pin_is_pad: std::collections::BTreeMap<String, bool>,    // pin_name -> is_pad
    // UI state for pin selection
    pub selected_pins: std::collections::BTreeSet<String>, // selected pins for batch operations
    pub pin_filter: String,                                // search filter for pins
}

impl Default for VoltageConfig {
    fn default() -> Self {
        Self {
            power_pins: std::collections::BTreeMap::new(),
            ground_pins: std::collections::BTreeMap::new(),
            selected_related_power: String::new(),
            selected_related_ground: String::new(),
            nom_voltage: 0.8, // Changed from 1.1 to 0.8
            lib_name: "lef_cells".to_string(),
            pin_related_power: std::collections::BTreeMap::new(),
            pin_related_ground: std::collections::BTreeMap::new(),
            pin_is_analog: std::collections::BTreeMap::new(),
            pin_is_pad: std::collections::BTreeMap::new(),
            selected_pins: std::collections::BTreeSet::new(),
            pin_filter: String::new(),
        }
    }
}

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

    // Group pins by base name (using cleaned names)
    for pin in pins {
        let cleaned_name = clean_pin_name(&pin.name);
        if let Some((base_name, _index)) = extract_bus_info(&cleaned_name) {
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

        // Extract indices and check continuity (using cleaned names)
        let mut indices: Vec<usize> = group
            .iter()
            .filter_map(|pin| {
                let cleaned_name = clean_pin_name(&pin.name);
                extract_bus_info(&cleaned_name).map(|(_, index)| index)
            })
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
            name: clean_pin_name(&pin.name),
            direction: pin.direction.clone(),
            pin_type: pin.use_type.clone(),
            width: 1,
        };
    }

    // Multi-pin bus
    let first_pin = pins[0];
    let cleaned_name = clean_pin_name(&first_pin.name);
    let base_name = extract_bus_info(&cleaned_name).unwrap().0;

    // Get all indices and find range
    let mut indices: Vec<usize> = pins
        .iter()
        .filter_map(|pin| {
            let cleaned = clean_pin_name(&pin.name);
            extract_bus_info(&cleaned).map(|(_, index)| index)
        })
        .collect();
    indices.sort();

    let min_index = indices[0];
    let max_index = indices[indices.len() - 1];
    let width = max_index - min_index + 1;

    PinCsvRecord {
        name: format!("{base_name}[{max_index}:{min_index}]"),
        direction: first_pin.direction.clone(),
        pin_type: first_pin.use_type.clone(),
        width,
    }
}

/// Format pins into a compressed comma-separated string of "DIRECTION:NAME" format
fn format_pinlist_compressed(pins: &[LefPin]) -> String {
    // Sort pins by type priority before grouping
    let mut sorted_pins = pins.to_vec();
    sort_pins_by_type(&mut sorted_pins);

    let groups = group_pins_by_bus(&sorted_pins);
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

    // Sort pins by type priority before grouping
    let mut sorted_pins = macro_def.pins.clone();
    sort_pins_by_type(&mut sorted_pins);

    let groups = group_pins_by_bus(&sorted_pins);
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

/// Collect all unique bus widths from LEF data
fn collect_bus_widths(lef_data: &Lef) -> BTreeSet<usize> {
    let mut widths = BTreeSet::new();

    for macro_def in &lef_data.macros {
        let groups = group_pins_by_bus(&macro_def.pins);
        for group in groups {
            if group.len() > 1 {
                // This is a bus, calculate its width
                let mut indices: Vec<usize> = group
                    .iter()
                    .filter_map(|pin| extract_bus_info(&pin.name).map(|(_, index)| index))
                    .collect();
                if !indices.is_empty() {
                    indices.sort();
                    let width = indices[indices.len() - 1] - indices[0] + 1;
                    widths.insert(width);
                }
            }
        }
    }

    widths
}

/// Get bus type name for Liberty file (e.g., DATA8B for width 8)
fn get_bus_type_name(width: usize) -> String {
    format!("DATA{width}B")
}

/// Clean pin name by removing special characters like '!'
fn clean_pin_name(name: &str) -> String {
    name.replace('!', "")
}

/// Check if a pin is a power or ground pin
fn is_power_pin(pin: &LefPin) -> bool {
    pin.use_type == "POWER" || pin.use_type == "GROUND"
}

/// Get pin priority for sorting (POWER=0, GROUND=1, others=2)
fn get_pin_sort_priority(pin: &LefPin) -> u8 {
    match pin.use_type.as_str() {
        "POWER" => 0,
        "GROUND" => 1,
        _ => 2,
    }
}

/// Sort pins by type priority: POWER, GROUND, then others
fn sort_pins_by_type(pins: &mut [LefPin]) {
    pins.sort_by_key(|pin| (get_pin_sort_priority(pin), pin.name.clone()));
}

/// Generate Verilog signal pin declaration with comma placement before comment
fn generate_verilog_signal_port_declaration(
    pin_group: &[&LefPin],
    add_comma_prefix: bool,
    add_comma_suffix: bool,
) -> String {
    if pin_group.len() == 1 {
        // Single pin
        let pin = pin_group[0];
        let direction = match pin.direction.as_str() {
            "INPUT" => "input",
            "OUTPUT" => "output",
            "INOUT" => "inout",
            _ => "input", // default
        };

        let clean_name = clean_pin_name(&pin.name);
        let prefix = if add_comma_prefix { "," } else { "" };
        let suffix = if add_comma_suffix { "," } else { "" };
        format!("    {prefix}{direction} {clean_name}{suffix}       /**< {clean_name} */")
    } else {
        // Bus pin - use existing compression logic
        let record = compress_bus_group(pin_group);
        let direction = match record.direction.as_str() {
            "INPUT" => "input",
            "OUTPUT" => "output",
            "INOUT" => "inout",
            _ => "input", // default
        };

        let clean_name = clean_pin_name(&record.name);
        let prefix = if add_comma_prefix { "," } else { "" };
        let suffix = if add_comma_suffix { "," } else { "" };

        // Extract base name and range from compressed name
        if let Some(bracket_start) = clean_name.rfind('[') {
            let base_name = &clean_name[..bracket_start];
            let range_part = &clean_name[bracket_start..];
            format!(
                "    {prefix}{direction} {range_part} {base_name}{suffix}  /**< {clean_name} */"
            )
        } else {
            format!("    {prefix}{direction} {clean_name}{suffix}       /**< {clean_name} */")
        }
    }
}

/// Generate Verilog power pin declaration with comma placement before comment
fn generate_verilog_power_port_declaration(
    pin_group: &[&LefPin],
    add_comma_prefix: bool,
    add_comma_suffix: bool,
) -> String {
    if pin_group.len() == 1 {
        // Single pin
        let pin = pin_group[0];
        let direction = match pin.direction.as_str() {
            "INPUT" => "input",
            "OUTPUT" => "output",
            "INOUT" => "inout",
            _ => "input", // default
        };

        let clean_name = clean_pin_name(&pin.name);
        let prefix = if add_comma_prefix { "," } else { "" };
        let suffix = if add_comma_suffix { "," } else { "" };
        format!("    {prefix}{direction} {clean_name}{suffix}       /**< {clean_name} */")
    } else {
        // Bus pin - use existing compression logic
        let record = compress_bus_group(pin_group);
        let direction = match record.direction.as_str() {
            "INPUT" => "input",
            "OUTPUT" => "output",
            "INOUT" => "inout",
            _ => "input", // default
        };

        let clean_name = clean_pin_name(&record.name);
        let prefix = if add_comma_prefix { "," } else { "" };
        let suffix = if add_comma_suffix { "," } else { "" };

        // Extract base name and range from compressed name
        if let Some(bracket_start) = clean_name.rfind('[') {
            let base_name = &clean_name[..bracket_start];
            let range_part = &clean_name[bracket_start..];
            format!(
                "    {prefix}{direction} {range_part} {base_name}{suffix}  /**< {clean_name} */"
            )
        } else {
            format!("    {prefix}{direction} {clean_name}{suffix}       /**< {clean_name} */")
        }
    }
}

/// Generate Verilog port declaration for a pin group (legacy, for compatibility)
#[allow(dead_code)]
fn generate_verilog_port_declaration(pin_group: &[&LefPin]) -> String {
    if pin_group.len() == 1 {
        // Single pin
        let pin = pin_group[0];
        let direction = match pin.direction.as_str() {
            "INPUT" => "input",
            "OUTPUT" => "output",
            "INOUT" => "inout",
            _ => "input", // default
        };

        let clean_name = clean_pin_name(&pin.name);

        // Always generate power pins as regular pins for syntax compatibility
        // The `ifdef can be handled at the module level if needed
        format!("    {direction} {clean_name}       /**< {clean_name} */")
    } else {
        // Bus pin - use existing compression logic
        let record = compress_bus_group(pin_group);
        let direction = match record.direction.as_str() {
            "INPUT" => "input",
            "OUTPUT" => "output",
            "INOUT" => "inout",
            _ => "input", // default
        };

        let clean_name = clean_pin_name(&record.name);

        // Extract base name and range from compressed name
        let port_declaration = if let Some(bracket_start) = clean_name.rfind('[') {
            let base_name = &clean_name[..bracket_start];
            let range_part = &clean_name[bracket_start..];
            format!("    {direction} {range_part} {base_name}  /**< {clean_name} */")
        } else {
            format!("    {direction} {clean_name}       /**< {clean_name} */")
        };

        // Always generate as regular port declaration for syntax compatibility
        port_declaration
    }
}

/// Check if a cell only has power/ground pins
fn cell_has_only_power_pins(pins: &[LefPin]) -> bool {
    !pins.is_empty() && pins.iter().all(is_power_pin)
}

/// Collect all pg_pins that are actually used as related_power_pin or related_ground_pin in a cell
fn collect_used_pg_pins(
    pins: &[LefPin],
    voltage_config: &VoltageConfig,
) -> std::collections::HashSet<String> {
    let mut used_pg_pins = std::collections::HashSet::new();

    // First, create a set of all pin names that actually exist in this cell
    let existing_pins: std::collections::HashSet<String> =
        pins.iter().map(|pin| clean_pin_name(&pin.name)).collect();

    for pin in pins {
        let clean_name = clean_pin_name(&pin.name);

        // Skip power/ground pins - we only care about signal pins that reference power/ground pins
        if is_power_pin(pin) {
            continue;
        }

        // Check for pin-specific related power configuration first
        let related_power = voltage_config
            .pin_related_power
            .get(&clean_name)
            .map(|s| s.as_str())
            .or_else(|| {
                if !voltage_config.selected_related_power.is_empty() {
                    Some(&voltage_config.selected_related_power)
                } else {
                    None
                }
            });

        // Check for pin-specific related ground configuration first
        let related_ground = voltage_config
            .pin_related_ground
            .get(&clean_name)
            .map(|s| s.as_str())
            .or_else(|| {
                if !voltage_config.selected_related_ground.is_empty() {
                    Some(&voltage_config.selected_related_ground)
                } else {
                    None
                }
            });

        // Only add power/ground pins to the set if they actually exist in this cell
        if let Some(power) = related_power {
            if existing_pins.contains(power) {
                used_pg_pins.insert(power.to_string());
            }
        }
        if let Some(ground) = related_ground {
            if existing_pins.contains(ground) {
                used_pg_pins.insert(ground.to_string());
            }
        }
    }

    used_pg_pins
}

/// Generate Liberty pin definition for a pin group with voltage configuration
fn generate_lib_pin_definition_with_config(
    pin_group: &[&LefPin],
    voltage_config: &VoltageConfig,
    treat_power_as_signal: bool,
    used_pg_pins: &std::collections::HashSet<String>,
    existing_pins: &std::collections::HashSet<String>,
) -> String {
    if pin_group.len() == 1 {
        // Single pin
        let pin = pin_group[0];
        let clean_name = clean_pin_name(&pin.name);

        // Check if this is a power or ground pin (and not treated as signal)
        // Also check if this pg_pin is actually used as related_power_pin or related_ground_pin
        if is_power_pin(pin) && !treat_power_as_signal && used_pg_pins.contains(&clean_name) {
            // Generate pg_pin instead of regular pin
            let pg_type = match pin.use_type.as_str() {
                "POWER" => "primary_power",
                "GROUND" => "primary_ground",
                _ => "primary_power", // fallback
            };
            format!(
                "   pg_pin({clean_name})  {{\n           voltage_name : {clean_name} ;\n           pg_type : {pg_type} ;\n   }}\n"
            )
        } else {
            // Regular signal pin or power pin treated as signal or unused pg_pin
            let direction = if (treat_power_as_signal && is_power_pin(pin))
                || (is_power_pin(pin) && !used_pg_pins.contains(&clean_name))
            {
                // For power pins treated as signal or unused pg_pins, use "inout" direction
                "inout".to_string()
            } else {
                pin.direction.to_lowercase()
            };

            if (treat_power_as_signal && is_power_pin(pin))
                || (is_power_pin(pin) && !used_pg_pins.contains(&clean_name))
            {
                // Power pin treated as signal or unused pg_pin - no related power/ground pins
                format!(
                    "   pin({clean_name})  {{\n           direction : {direction};\n           capacitance : 0.02;\n   }}\n"
                )
            } else {
                // Regular signal pin
                // Check for pin-specific related power configuration first
                let related_power = voltage_config
                    .pin_related_power
                    .get(&clean_name)
                    .map(|s| s.as_str())
                    .or_else(|| {
                        if !voltage_config.selected_related_power.is_empty() {
                            Some(&voltage_config.selected_related_power)
                        } else {
                            None
                        }
                    });

                // Check for pin-specific related ground configuration first
                let related_ground = voltage_config
                    .pin_related_ground
                    .get(&clean_name)
                    .map(|s| s.as_str())
                    .or_else(|| {
                        if !voltage_config.selected_related_ground.is_empty() {
                            Some(&voltage_config.selected_related_ground)
                        } else {
                            None
                        }
                    });
                let mut pin_def = format!(
                    "   pin({clean_name})  {{\n           direction : {direction};\n           capacitance : 0.02;\n"
                );
                // Only add related_power_pin if the referenced pin actually exists in this cell
                if let Some(power) = related_power {
                    if existing_pins.contains(power) {
                        pin_def.push_str(&format!("           related_power_pin : {power} ;\n"));
                    }
                }
                // Only add related_ground_pin if the referenced pin actually exists in this cell
                if let Some(ground) = related_ground {
                    if existing_pins.contains(ground) {
                        pin_def.push_str(&format!("           related_ground_pin  : {ground} ;\n"));
                    }
                }
                // Add is_analog attribute if set
                if voltage_config
                    .pin_is_analog
                    .get(&pin.name)
                    .copied()
                    .unwrap_or(false)
                {
                    pin_def.push_str("           is_analog : true ;\n");
                }
                // Add is_pad attribute if set
                if voltage_config
                    .pin_is_pad
                    .get(&pin.name)
                    .copied()
                    .unwrap_or(false)
                {
                    pin_def.push_str("           is_pad : true ;\n");
                }
                pin_def.push_str("   }\n");
                pin_def
            }
        }
    } else {
        // Bus pin
        let record = compress_bus_group(pin_group);
        let is_power = pin_group.iter().any(|pin| is_power_pin(pin));

        if is_power && !treat_power_as_signal {
            // Check if any of the power pins in this bus are used as related pins
            let any_used = pin_group.iter().any(|pin| {
                let clean_name = clean_pin_name(&pin.name);
                used_pg_pins.contains(&clean_name)
            });

            if any_used {
                // For power/ground bus pins, generate individual pg_pins
                let mut result = String::new();
                for pin in pin_group {
                    let clean_name = clean_pin_name(&pin.name);
                    let pg_type = match pin.use_type.as_str() {
                        "POWER" => "primary_power",
                        "GROUND" => "primary_ground",
                        _ => "primary_power", // fallback
                    };
                    result.push_str(&format!(
                    "   pg_pin({clean_name})  {{\n           voltage_name : {clean_name} ;\n           pg_type : {pg_type} ;\n   }}\n"
                ));
                }
                result
            } else {
                // Power/ground bus pins not used as related pins - treat as regular pins
                let mut result = String::new();
                for pin in pin_group {
                    let clean_name = clean_pin_name(&pin.name);
                    let direction = "inout"; // Power pins as regular pins should be inout
                    result.push_str(&format!(
                        "   pin({clean_name})  {{\n           direction : {direction};\n           capacitance : 0.02;\n   }}\n"
                    ));
                }
                result
            }
        } else if is_power && treat_power_as_signal {
            // For power/ground bus pins treated as signal, generate individual pins
            let mut result = String::new();
            for pin in pin_group {
                let clean_name = clean_pin_name(&pin.name);
                result.push_str(&format!(
                    "   pin({clean_name})  {{\n           direction : inout;\n           capacitance : 0.02;\n   }}\n"
                ));
            }
            result
        } else {
            // Regular signal bus
            let direction = record.direction.to_lowercase();
            let bus_type = get_bus_type_name(record.width);

            // Extract base name from compressed name for configuration lookup
            let base_name = if let Some(bracket_start) = record.name.rfind('[') {
                &record.name[..bracket_start]
            } else {
                &record.name
            };

            // Check for bus-specific related power configuration first
            let related_power = voltage_config
                .pin_related_power
                .get(base_name)
                .map(|s| s.as_str())
                .or_else(|| {
                    if !voltage_config.selected_related_power.is_empty() {
                        Some(&voltage_config.selected_related_power)
                    } else {
                        None
                    }
                });

            // Check for bus-specific related ground configuration first
            let related_ground = voltage_config
                .pin_related_ground
                .get(base_name)
                .map(|s| s.as_str())
                .or_else(|| {
                    if !voltage_config.selected_related_ground.is_empty() {
                        Some(&voltage_config.selected_related_ground)
                    } else {
                        None
                    }
                });

            let mut result =
                format!("   bus({base_name}) {{\n        bus_type       : \"{bus_type}\";\n");

            // Only add related_power_pin if the referenced pin actually exists in this cell
            if let Some(power) = related_power {
                if existing_pins.contains(power) {
                    result.push_str(&format!("        related_power_pin : {power} ;\n"));
                }
            }
            // Only add related_ground_pin if the referenced pin actually exists in this cell
            if let Some(ground) = related_ground {
                if existing_pins.contains(ground) {
                    result.push_str(&format!("        related_ground_pin  : {ground} ;\n"));
                }
            }
            result.push('\n');

            // Generate individual pin definitions
            for i in 0..record.width {
                let mut pin_def = format!(
                    "        pin ({base_name}[{i}]) {{\n        direction      : {direction};\n        capacitance    : 0.02;\n"
                );

                // Add is_analog attribute if set (for bus pins, check the base name)
                if voltage_config
                    .pin_is_analog
                    .get(&record.name)
                    .copied()
                    .unwrap_or(false)
                {
                    pin_def.push_str("        is_analog : true ;\n");
                }
                // Add is_pad attribute if set (for bus pins, check the base name)
                if voltage_config
                    .pin_is_pad
                    .get(&record.name)
                    .copied()
                    .unwrap_or(false)
                {
                    pin_def.push_str("        is_pad : true ;\n");
                }

                pin_def.push_str("        }\n\n");
                result.push_str(&pin_def);
            }

            result.push_str(&format!("}} /* end of bus {base_name} */\n"));
            result
        }
    }
}

/// Export all LEF cells to Verilog stub file
pub fn export_verilog_stub(
    lef_data: &Lef,
    file_path: &str,
    basename: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(file_path)?;
    let guard_name = format!("DEF_{}", basename.to_uppercase());

    // Generate file header
    writeln!(file, "/**")?;
    writeln!(file, " * @file {basename}.v")?;
    writeln!(file, " * @brief Verilog stub file for LEF cells")?;
    writeln!(file, " *")?;
    writeln!(
        file,
        " * @details This file contains Verilog stub modules for all LEF cells."
    )?;
    writeln!(
        file,
        " *          Auto-generated stub file. Generated by lefdef-viewer."
    )?;
    writeln!(file, " * NOTE: Auto-generated file, do not edit manually.")?;
    writeln!(file, " */")?;
    writeln!(file, "`timescale 1ns / 1ps")?;
    writeln!(file, "`ifndef {guard_name}")?;
    writeln!(file, "`define {guard_name}")?;
    writeln!(file)?;
    writeln!(file, "`ifndef SYNTHESIS")?;

    // Generate stub for each macro
    for macro_def in &lef_data.macros {
        // Generate module header comment
        writeln!(file, "`celldefine")?;
        writeln!(file, "/**")?;
        writeln!(file, " * @brief {} module stub", macro_def.name)?;
        writeln!(file, " *")?;
        writeln!(
            file,
            " * @details Stub implementation of {} module.",
            macro_def.name
        )?;
        writeln!(file, " */")?;
        writeln!(file, "module {} (", macro_def.name)?;

        // Sort pins by type priority before generating port list
        let mut sorted_pins = macro_def.pins.clone();
        sort_pins_by_type(&mut sorted_pins);

        let groups = group_pins_by_bus(&sorted_pins);

        // Separate power pins from signal pins
        let mut signal_groups = Vec::new();
        let mut power_groups = Vec::new();

        for group in groups {
            let is_power_group = group.iter().any(|pin| is_power_pin(pin));
            if is_power_group {
                power_groups.push(group);
            } else {
                signal_groups.push(group);
            }
        }

        // Generate all ports with proper comma handling
        // New logic: PG pins first, then signal pins, only last pin has no comma
        let total_groups = power_groups.len() + signal_groups.len();

        if total_groups == 0 {
            // No ports at all
        } else if !power_groups.is_empty() {
            // Generate PG pins first (always in ifdef block)
            writeln!(file, "`ifdef PG_EXIST")?;
            for (i, group) in power_groups.iter().enumerate() {
                let is_last_pg = i == power_groups.len() - 1;
                let is_last_overall = signal_groups.is_empty() && is_last_pg;

                let port_decl =
                    generate_verilog_power_port_declaration(group, false, !is_last_overall);
                writeln!(file, "{port_decl}")?;
            }
            writeln!(file, "`endif  /* PG_EXIST */")?;

            // Generate signal pins after PG pins
            for (i, group) in signal_groups.iter().enumerate() {
                let is_last_signal = i == signal_groups.len() - 1;
                let port_decl =
                    generate_verilog_signal_port_declaration(group, false, !is_last_signal);
                writeln!(file, "{port_decl}")?;
            }
        } else {
            // Only signal pins
            for (i, group) in signal_groups.iter().enumerate() {
                let is_last = i == signal_groups.len() - 1;
                let port_decl = generate_verilog_signal_port_declaration(group, false, !is_last);
                writeln!(file, "{port_decl}")?;
            }
        }

        writeln!(file, ");")?;
        writeln!(
            file,
            "    /* It is a stub, not a complete implementation */"
        )?;
        writeln!(file, "endmodule")?;
        writeln!(file, "`endcelldefine")?;
        writeln!(file)?;
    }

    writeln!(file, "`endif  /* SYNTHESIS */")?;
    writeln!(file)?;
    writeln!(file, "`endif  /* {guard_name} */")?;
    writeln!(file)?;

    Ok(())
}

/// Export all LEF cells to Liberty stub file with voltage configuration (old implementation)
#[allow(dead_code)]
pub fn export_lib_stub_with_voltage(
    lef_data: &Lef,
    file_path: &str,
    power_voltage: f32,
    ground_voltage: f32,
    lib_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(file_path)?;

    // Generate library header
    writeln!(file, "library ({lib_name})  {{")?;
    writeln!(file)?;
    writeln!(file, "/* General Library Attributes */")?;
    writeln!(file)?;
    writeln!(file, "  technology (cmos) ;")?;
    writeln!(file, "  delay_model      : table_lookup;")?;
    writeln!(file, "  bus_naming_style : \"%s[%d]\";")?;
    writeln!(file, "  simulation  : true;")?;
    writeln!(file)?;
    writeln!(file)?;

    // Unit Definition
    writeln!(file, "/* Unit Definition */")?;
    writeln!(file)?;
    writeln!(file, "  time_unit               : \"1ns\";")?;
    writeln!(file, "  voltage_unit            : \"1V\";")?;
    writeln!(file, "  current_unit            : \"1mA\";")?;
    writeln!(file, "  capacitive_load_unit (1,pf);")?;
    writeln!(file, "  pulling_resistance_unit : \"1kohm\";")?;
    writeln!(file)?;

    // Power estimation settings
    writeln!(file, "/* Added for DesignPower (Power Estimation). */")?;
    writeln!(file, "  leakage_power_unit : 1pW;")?;
    writeln!(file, "  default_cell_leakage_power : 1;")?;
    writeln!(file)?;

    // Threshold settings
    writeln!(file, "slew_lower_threshold_pct_rise :  10 ;")?;
    writeln!(file, "slew_upper_threshold_pct_rise :  90 ;")?;
    writeln!(file, "input_threshold_pct_fall      :  50 ;")?;
    writeln!(file, "output_threshold_pct_fall     :  50 ;")?;
    writeln!(file, "input_threshold_pct_rise      :  50 ;")?;
    writeln!(file, "output_threshold_pct_rise     :  50 ;")?;
    writeln!(file, "slew_lower_threshold_pct_fall :  10 ;")?;
    writeln!(file, "slew_upper_threshold_pct_fall :  90 ;")?;
    writeln!(file, "slew_derate_from_library      :  1.0 ;")?;
    writeln!(file)?;
    writeln!(file)?;

    // Operating conditions
    writeln!(file, "/****************************/")?;
    writeln!(file, "/** user supplied nominals **/")?;
    writeln!(file, "/****************************/")?;
    writeln!(file)?;
    writeln!(file, "nom_voltage     : 1.100;")?;
    writeln!(file, "nom_temperature : 25.000;")?;
    writeln!(file, "nom_process     : 1.000;")?;
    writeln!(file)?;
    writeln!(file, "operating_conditions(\"typical\"){{")?;
    writeln!(file, "process :   1.0")?;
    writeln!(file, "temperature :  25")?;
    writeln!(file, "voltage :      1.10")?;
    writeln!(file, "tree_type : \"balanced_tree\"")?;
    writeln!(file, "}}")?;
    writeln!(file)?;
    writeln!(file, "default_operating_conditions  : typical")?;
    writeln!(file)?;
    writeln!(file)?;

    // Default values
    writeln!(file, "/****************************/")?;
    writeln!(file, "/** user supplied defaults **/")?;
    writeln!(file, "/****************************/")?;
    writeln!(file)?;
    writeln!(file, "default_inout_pin_cap           :       0.0100;")?;
    writeln!(file, "default_input_pin_cap           :       0.0100;")?;
    writeln!(file, "default_output_pin_cap          :       0.0000;")?;
    writeln!(file, "default_fanout_load             :       1.0000;")?;
    writeln!(file)?;
    writeln!(file)?;

    // Generate type declarations
    let bus_widths = collect_bus_widths(lef_data);
    if !bus_widths.is_empty() {
        writeln!(file, "/* Type declarations */")?;
        writeln!(file)?;

        for width in bus_widths {
            if width > 1 {
                writeln!(file, "  type ({})  {{", get_bus_type_name(width))?;
                writeln!(file, "    base_type : array;")?;
                writeln!(file, "    data_type : bit;")?;
                writeln!(file, "    bit_width : {width};")?;
                writeln!(file, "    bit_from  : {};", width - 1)?;
                writeln!(file, "    bit_to    : 0;")?;
                writeln!(file, "    downto    : true;")?;
                writeln!(file, "  }}")?;
                writeln!(file)?;
            }
        }
        writeln!(file)?;
    }

    // Cell descriptions
    writeln!(file, "/* **************************** */")?;
    writeln!(file, "/* ****  Cell Description  **** */")?;
    writeln!(file, "/* **************************** */")?;

    // Collect all unique power and ground pins
    let mut power_pins = std::collections::BTreeSet::new();
    let mut ground_pins = std::collections::BTreeSet::new();

    for macro_def in &lef_data.macros {
        for pin in &macro_def.pins {
            match pin.use_type.as_str() {
                "POWER" => {
                    power_pins.insert(clean_pin_name(&pin.name));
                }
                "GROUND" => {
                    ground_pins.insert(clean_pin_name(&pin.name));
                }
                _ => {}
            }
        }
    }

    // Generate dynamic voltage mapping
    writeln!(file, "/* Voltage Mapping */")?;
    for power_pin in &power_pins {
        writeln!(file, "    voltage_map({power_pin}, {power_voltage});")?;
    }
    for ground_pin in &ground_pins {
        writeln!(file, "    voltage_map({ground_pin}, {ground_voltage});")?;
    }
    writeln!(file)?;

    // Generate cell for each macro
    for macro_def in &lef_data.macros {
        writeln!(file, "cell ({})  {{", macro_def.name)?;
        writeln!(file)?;
        writeln!(file, "   area            : 100;")?;
        writeln!(file, "   dont_touch      : true;")?;
        writeln!(file, "   dont_use        : true;")?;
        writeln!(file, "   map_only        : true;")?;
        writeln!(file)?;

        // Sort pins by type priority before generating pin definitions
        let mut sorted_pins = macro_def.pins.clone();
        sort_pins_by_type(&mut sorted_pins);

        // Check if this cell only has power/ground pins
        let treat_power_as_signal = cell_has_only_power_pins(&sorted_pins);

        // Create a default voltage config for backward compatibility
        let mut default_config = VoltageConfig::default();
        default_config
            .power_pins
            .insert("VDD".to_string(), power_voltage);
        default_config
            .ground_pins
            .insert("VSS".to_string(), ground_voltage);
        default_config.selected_related_power = "VDD".to_string();
        default_config.selected_related_ground = "VSS".to_string();

        // Collect all pg_pins that are actually used as related_power_pin or related_ground_pin
        let used_pg_pins = collect_used_pg_pins(&sorted_pins, &default_config);

        // Create a set of all pin names that actually exist in this cell
        let existing_pins: std::collections::HashSet<String> = sorted_pins
            .iter()
            .map(|pin| clean_pin_name(&pin.name))
            .collect();

        let groups = group_pins_by_bus(&sorted_pins);
        for group in groups {
            let pin_def = generate_lib_pin_definition_with_config(
                &group,
                &default_config,
                treat_power_as_signal,
                &used_pg_pins,
                &existing_pins,
            );
            write!(file, "{pin_def}")?;
        }

        writeln!(file, "}}  /* end of cell {} */", macro_def.name)?;
        writeln!(file)?;
    }

    // Close library
    writeln!(file, "}}  /* end of library */")?;

    Ok(())
}

/// Export all LEF cells to Liberty stub file with voltage configuration
pub fn export_lib_stub_with_voltage_config(
    lef_data: &Lef,
    file_path: &str,
    voltage_config: &VoltageConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::create(file_path)?;

    // Generate library header
    writeln!(file, "library ({})  {{", voltage_config.lib_name)?;
    writeln!(file)?;
    writeln!(file, "/* General Library Attributes */")?;
    writeln!(file)?;
    writeln!(file, "  technology (cmos) ;")?;
    writeln!(file, "  delay_model      : table_lookup;")?;
    writeln!(file, "  bus_naming_style : \"%s[%d]\";")?;
    writeln!(file, "  simulation  : true;")?;
    writeln!(file)?;
    writeln!(file)?;

    // Unit Definition
    writeln!(file, "/* Unit Definition */")?;
    writeln!(file)?;
    writeln!(file, "  time_unit               : \"1ns\";")?;
    writeln!(file, "  voltage_unit            : \"1V\";")?;
    writeln!(file, "  current_unit            : \"1mA\";")?;
    writeln!(file, "  capacitive_load_unit (1,pf);")?;
    writeln!(file, "  pulling_resistance_unit : \"1kohm\";")?;
    writeln!(file)?;

    // Power estimation settings
    writeln!(file, "/* Added for DesignPower (Power Estimation). */")?;
    writeln!(file, "  leakage_power_unit : 1pW;")?;
    writeln!(file, "  default_cell_leakage_power : 1;")?;
    writeln!(file)?;

    // Threshold settings
    writeln!(file, "slew_lower_threshold_pct_rise :  10 ;")?;
    writeln!(file, "slew_upper_threshold_pct_rise :  90 ;")?;
    writeln!(file, "input_threshold_pct_fall      :  50 ;")?;
    writeln!(file, "output_threshold_pct_fall     :  50 ;")?;
    writeln!(file, "input_threshold_pct_rise      :  50 ;")?;
    writeln!(file, "output_threshold_pct_rise     :  50 ;")?;
    writeln!(file, "slew_lower_threshold_pct_fall :  10 ;")?;
    writeln!(file, "slew_upper_threshold_pct_fall :  90 ;")?;
    writeln!(file, "slew_derate_from_library      :  1.0 ;")?;
    writeln!(file)?;
    writeln!(file)?;

    // User supplied nominals
    writeln!(file, "/****************************/")?;
    writeln!(file, "/** user supplied nominals **/")?;
    writeln!(file, "/****************************/")?;
    writeln!(file)?;
    writeln!(file, "nom_voltage     : {:.3};", voltage_config.nom_voltage)?;
    writeln!(file, "nom_temperature : 25.000;")?;
    writeln!(file, "nom_process     : 1.000;")?;
    writeln!(file)?;

    // Operating conditions
    writeln!(file, "operating_conditions(\"typical\"){{")?;
    writeln!(file, "process :   1.0")?;
    writeln!(file, "temperature :  25")?;
    writeln!(file, "voltage :      {:.2}", voltage_config.nom_voltage)?;
    writeln!(file, "tree_type : \"balanced_tree\"")?;
    writeln!(file, "}}")?;
    writeln!(file)?;
    writeln!(file, "default_operating_conditions  : typical")?;
    writeln!(file)?;
    writeln!(file)?;

    // User supplied defaults
    writeln!(file, "/****************************/")?;
    writeln!(file, "/** user supplied defaults **/")?;
    writeln!(file, "/****************************/")?;
    writeln!(file)?;
    writeln!(file, "default_inout_pin_cap           :       0.0100;")?;
    writeln!(file, "default_input_pin_cap           :       0.0100;")?;
    writeln!(file, "default_output_pin_cap          :       0.0000;")?;
    writeln!(file, "default_fanout_load             :       1.0000;")?;
    writeln!(file)?;
    writeln!(file)?;

    // Generate type declarations
    let bus_widths = collect_bus_widths(lef_data);
    if !bus_widths.is_empty() {
        writeln!(file, "/* Type declarations */")?;
        writeln!(file)?;

        for width in bus_widths {
            if width > 1 {
                writeln!(file, "  type ({})  {{", get_bus_type_name(width))?;
                writeln!(file, "    base_type : array;")?;
                writeln!(file, "    data_type : bit;")?;
                writeln!(file, "    bit_width : {width};")?;
                writeln!(file, "    bit_from  : {};", width - 1)?;
                writeln!(file, "    bit_to    : 0;")?;
                writeln!(file, "    downto    : true;")?;
                writeln!(file, "  }}")?;
                writeln!(file)?;
            }
        }
        writeln!(file)?;
    }

    // Cell descriptions
    writeln!(file, "/* **************************** */")?;
    writeln!(file, "/* ****  Cell Description  **** */")?;
    writeln!(file, "/* **************************** */")?;

    // Generate voltage mapping
    for (power_pin, voltage) in &voltage_config.power_pins {
        writeln!(file, "    voltage_map({power_pin}, {voltage});")?;
    }
    for (ground_pin, voltage) in &voltage_config.ground_pins {
        writeln!(file, "    voltage_map({ground_pin}, {voltage});")?;
    }
    writeln!(file)?;

    // Generate cell for each macro
    for macro_def in &lef_data.macros {
        writeln!(file, "cell ({})  {{", macro_def.name)?;
        writeln!(file)?;
        writeln!(file, "   area            : 100;")?;
        writeln!(file, "   dont_touch      : true;")?;
        writeln!(file, "   dont_use        : true;")?;
        writeln!(file, "   map_only        : true;")?;

        // Check if any pin in this cell has is_pad set to true
        let has_pad_pin = macro_def.pins.iter().any(|pin| {
            voltage_config
                .pin_is_pad
                .get(&pin.name)
                .copied()
                .unwrap_or(false)
        });

        // Add pad_cell attribute if any pin is marked as pad
        if has_pad_pin {
            writeln!(file, "   pad_cell        : true;")?;
        }

        writeln!(file)?;

        // Sort pins by type priority before generating pin definitions
        let mut sorted_pins = macro_def.pins.clone();
        sort_pins_by_type(&mut sorted_pins);

        // Check if this cell only has power/ground pins
        let treat_power_as_signal = cell_has_only_power_pins(&sorted_pins);

        // Collect all pg_pins that are actually used as related_power_pin or related_ground_pin
        let used_pg_pins = collect_used_pg_pins(&sorted_pins, voltage_config);

        // Create a set of all pin names that actually exist in this cell
        let existing_pins: std::collections::HashSet<String> = sorted_pins
            .iter()
            .map(|pin| clean_pin_name(&pin.name))
            .collect();

        let groups = group_pins_by_bus(&sorted_pins);
        for group in groups {
            let pin_def = generate_lib_pin_definition_with_config(
                &group,
                voltage_config,
                treat_power_as_signal,
                &used_pg_pins,
                &existing_pins,
            );
            write!(file, "{pin_def}")?;
        }

        writeln!(file, "}}  /* end of cell {} */", macro_def.name)?;
        writeln!(file)?;
    }

    // Close library
    writeln!(file, "}}  /* end of library */")?;

    Ok(())
}

/// Export all LEF cells to Liberty stub file (backward compatibility)
#[allow(dead_code)]
pub fn export_lib_stub(lef_data: &Lef, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let default_config = VoltageConfig::default();
    export_lib_stub_with_voltage_config(lef_data, file_path, &default_config)
}
