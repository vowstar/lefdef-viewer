// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

//! Enhanced Voltage Configuration Dialog
//!
//! This module provides a comprehensive voltage configuration dialog for Liberty export
//! with support for thousands of pins, batch selection, and per-pin power/ground configuration.

use crate::export::{PinCsvRecord, VoltageConfig};
use crate::lef::{Lef, LefPin};
use eframe::egui;
use egui_extras::{Column, TableBuilder};
use std::collections::BTreeMap;

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

/// Clean pin name by removing special characters like '!'
fn clean_pin_name(name: &str) -> String {
    name.replace('!', "")
}

/// Check if a pin is a power or ground pin
#[allow(dead_code)]
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

/// Group pins by bus base name and validate bus constraints
fn group_pins_by_bus(pins: &[LefPin]) -> Vec<Vec<&LefPin>> {
    let mut base_name_groups: BTreeMap<String, Vec<&LefPin>> = BTreeMap::new();
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
        name: format!("{}[{}:{}]", base_name, max_index, min_index),
        direction: first_pin.direction.clone(),
        pin_type: first_pin.use_type.clone(),
        width,
    }
}

/// Enhanced voltage configuration dialog state and rendering
#[derive(Default)]
pub struct VoltageDialog {
    /// Whether the dialog is currently shown
    pub visible: bool,
}

impl VoltageDialog {
    /// Create a new voltage dialog
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the voltage configuration dialog
    pub fn show(&mut self) {
        self.visible = true;
    }

    /// Hide the voltage configuration dialog
    #[allow(dead_code)]
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Initialize voltage configuration from LEF data
    pub fn initialize_config(lef_data: &Lef, voltage_config: &mut VoltageConfig) {
        // Collect all unique power, ground pins (for voltage configuration)
        let mut power_pins = std::collections::BTreeSet::new();
        let mut ground_pins = std::collections::BTreeSet::new();

        for macro_def in &lef_data.macros {
            for pin in &macro_def.pins {
                let clean_name = pin.name.replace('!', "");
                match pin.use_type.as_str() {
                    "POWER" => {
                        power_pins.insert(clean_name);
                    }
                    "GROUND" => {
                        ground_pins.insert(clean_name);
                    }
                    _ => {}
                }
            }
        }

        // Initialize voltage configuration
        voltage_config.power_pins.clear();
        voltage_config.ground_pins.clear();
        voltage_config.selected_pins.clear();
        voltage_config.pin_filter.clear();

        // Set default voltages for all power pins
        for power_pin in power_pins {
            voltage_config.power_pins.insert(power_pin.clone(), 0.8);
            if voltage_config.selected_related_power.is_empty() {
                voltage_config.selected_related_power = power_pin;
            }
        }

        // Set default voltages for all ground pins
        for ground_pin in ground_pins {
            voltage_config.ground_pins.insert(ground_pin.clone(), 0.0);
            if voltage_config.selected_related_ground.is_empty() {
                voltage_config.selected_related_ground = ground_pin;
            }
        }

        // Generate compressed pin groups (same logic as in render) to get the actual pin names shown in the table
        let mut all_pins: Vec<LefPin> = Vec::new();
        for macro_def in &lef_data.macros {
            all_pins.extend(macro_def.pins.clone());
        }
        sort_pins_by_type(&mut all_pins);

        // Group pins by bus and compress to get the final pin names
        let groups = group_pins_by_bus(&all_pins);
        let compressed_pin_groups: Vec<PinCsvRecord> = groups
            .iter()
            .map(|group| compress_bus_group(group))
            .collect();

        // Select all compressed pin groups by default (this includes both individual pins and buses)
        for pin_group in compressed_pin_groups {
            voltage_config.selected_pins.insert(pin_group.name);
        }
    }

    /// Render the enhanced voltage configuration dialog
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        voltage_config: &mut VoltageConfig,
        lef_data: Option<&Lef>,
        export_callback: &mut bool,
    ) {
        if !self.visible {
            return;
        }

        egui::Window::new("Enhanced Voltage Configuration")
            .collapsible(false)
            .resizable(true)
            .default_width(800.0)
            .default_height(450.0)
            .max_height(ctx.screen_rect().height() - 100.0)
            .show(ctx, |ui| {
                ui.label("Configure voltage values and pin settings for Liberty export:");
                ui.separator();

                // Nominal voltage configuration
                ui.horizontal(|ui| {
                    ui.label("Nominal Voltage (V):");
                    ui.add(
                        egui::DragValue::new(&mut voltage_config.nom_voltage)
                            .speed(0.01)
                            .range(0.0..=5.0)
                            .fixed_decimals(2),
                    );
                });

                // Library name configuration
                ui.horizontal(|ui| {
                    ui.label("Library Name:");
                    ui.text_edit_singleline(&mut voltage_config.lib_name);
                });
                ui.separator();

                // Power pins configuration
                if !voltage_config.power_pins.is_empty() {
                    ui.label("Power Pins:");
                    for (pin_name, voltage) in &mut voltage_config.power_pins {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", pin_name));
                            ui.add(
                                egui::DragValue::new(voltage)
                                    .speed(0.01)
                                    .range(0.0..=5.0)
                                    .fixed_decimals(2)
                                    .suffix(" V"),
                            );
                        });
                    }
                    ui.separator();
                }

                // Ground pins configuration
                if !voltage_config.ground_pins.is_empty() {
                    ui.label("Ground Pins:");
                    for (pin_name, voltage) in &mut voltage_config.ground_pins {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}:", pin_name));
                            ui.add(
                                egui::DragValue::new(voltage)
                                    .speed(0.01)
                                    .range(-2.0..=2.0)
                                    .fixed_decimals(2)
                                    .suffix(" V"),
                            );
                        });
                    }
                    ui.separator();
                }

                // Related power/ground pin selection
                ui.horizontal(|ui| {
                    if voltage_config.power_pins.len() > 1 {
                        ui.label("Default Related Power Pin:");
                        egui::ComboBox::from_id_salt("related_power")
                            .selected_text(&voltage_config.selected_related_power)
                            .show_ui(ui, |ui| {
                                for pin_name in voltage_config.power_pins.keys() {
                                    ui.selectable_value(
                                        &mut voltage_config.selected_related_power,
                                        pin_name.clone(),
                                        pin_name,
                                    );
                                }
                            });
                    }

                    if voltage_config.ground_pins.len() > 1 {
                        ui.label("Default Related Ground Pin:");
                        egui::ComboBox::from_id_salt("related_ground")
                            .selected_text(&voltage_config.selected_related_ground)
                            .show_ui(ui, |ui| {
                                for pin_name in voltage_config.ground_pins.keys() {
                                    ui.selectable_value(
                                        &mut voltage_config.selected_related_ground,
                                        pin_name.clone(),
                                        pin_name,
                                    );
                                }
                            });
                    }
                });
                ui.separator();

                // Pin selection and configuration section
                ui.label("Pin Selection and Configuration:");

                // Search filter
                ui.horizontal(|ui| {
                    ui.label("Filter pins:");
                    ui.text_edit_singleline(&mut voltage_config.pin_filter);
                });

                // Collect all pins from LEF data and group them by bus
                let mut compressed_pin_groups: Vec<PinCsvRecord> = Vec::new();
                if let Some(lef_data) = lef_data {
                    // Collect all pins and sort them
                    let mut all_pins: Vec<LefPin> = Vec::new();
                    for macro_def in &lef_data.macros {
                        all_pins.extend(macro_def.pins.clone());
                    }
                    sort_pins_by_type(&mut all_pins);

                    // Group pins by bus and compress
                    let groups = group_pins_by_bus(&all_pins);
                    compressed_pin_groups = groups
                        .iter()
                        .map(|group| compress_bus_group(group))
                        .collect();

                    // Sort the compressed pin groups to ensure stable ordering
                    compressed_pin_groups.sort_by(|a, b| {
                        // First sort by pin type priority (POWER, GROUND, others)
                        let a_priority = match a.pin_type.as_str() {
                            "POWER" => 0,
                            "GROUND" => 1,
                            _ => 2,
                        };
                        let b_priority = match b.pin_type.as_str() {
                            "POWER" => 0,
                            "GROUND" => 1,
                            _ => 2,
                        };

                        match a_priority.cmp(&b_priority) {
                            std::cmp::Ordering::Equal => {
                                // If same priority, sort by name alphabetically
                                a.name.cmp(&b.name)
                            }
                            other => other,
                        }
                    });
                }

                // Filter compressed pins based on search
                let filtered_pin_groups: Vec<&PinCsvRecord> = compressed_pin_groups
                    .iter()
                    .filter(|pin_group| {
                        voltage_config.pin_filter.is_empty()
                            || pin_group
                                .name
                                .to_lowercase()
                                .contains(&voltage_config.pin_filter.to_lowercase())
                    })
                    .collect();

                // Batch selection controls
                ui.horizontal(|ui| {
                    if ui.button("Select All").clicked() {
                        for pin_group in &compressed_pin_groups {
                            voltage_config.selected_pins.insert(pin_group.name.clone());
                        }
                    }
                    if ui.button("Deselect All").clicked() {
                        voltage_config.selected_pins.clear();
                    }
                    if ui.button("Select Filtered").clicked() {
                        for pin_group in &filtered_pin_groups {
                            voltage_config.selected_pins.insert(pin_group.name.clone());
                        }
                    }
                    if ui.button("Deselect Filtered").clicked() {
                        for pin_group in &filtered_pin_groups {
                            voltage_config.selected_pins.remove(&pin_group.name);
                        }
                    }
                    ui.label(format!(
                        "Selected: {}/{}",
                        voltage_config.selected_pins.len(),
                        compressed_pin_groups.len()
                    ));
                });

                // Batch assignment controls for selected pins
                if !voltage_config.selected_pins.is_empty() {
                    ui.separator();
                    ui.label("Batch Assignment for Selected Pins:");
                    ui.horizontal(|ui| {
                        ui.label("Set Related Power:");
                        egui::ComboBox::from_id_salt("batch_power")
                            .selected_text("Select Power Pin")
                            .show_ui(ui, |ui| {
                                for pin_name in voltage_config.power_pins.keys() {
                                    if ui.selectable_label(false, pin_name).clicked() {
                                        for selected_pin in &voltage_config.selected_pins {
                                            voltage_config
                                                .pin_related_power
                                                .insert(selected_pin.clone(), pin_name.clone());
                                        }
                                    }
                                }
                            });

                        ui.label("Set Related Ground:");
                        egui::ComboBox::from_id_salt("batch_ground")
                            .selected_text("Select Ground Pin")
                            .show_ui(ui, |ui| {
                                for pin_name in voltage_config.ground_pins.keys() {
                                    if ui.selectable_label(false, pin_name).clicked() {
                                        for selected_pin in &voltage_config.selected_pins {
                                            voltage_config
                                                .pin_related_ground
                                                .insert(selected_pin.clone(), pin_name.clone());
                                        }
                                    }
                                }
                            });
                    });
                }

                ui.separator();

                // Pin list with TableBuilder for perfect alignment
                ui.label(format!(
                    "Pin List (showing {} of {}):",
                    filtered_pin_groups.len(),
                    compressed_pin_groups.len()
                ));

                // Use TableBuilder for professional table layout with fixed header
                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(false)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::exact(30.0)) // Checkbox
                    .column(Column::remainder().at_least(150.0)) // Signal Name - takes remaining space
                    .column(Column::exact(80.0)) // Type
                    .column(Column::exact(50.0)) // Width
                    .column(Column::exact(50.0)) // Power label
                    .column(Column::exact(100.0)) // Power selection
                    .column(Column::exact(50.0)) // Ground label
                    .column(Column::exact(100.0)) // Ground selection
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("✓");
                        });
                        header.col(|ui| {
                            ui.strong("Signal Name");
                        });
                        header.col(|ui| {
                            ui.strong("Type");
                        });
                        header.col(|ui| {
                            ui.strong("Width");
                        });
                        header.col(|ui| {
                            ui.strong("Power");
                        });
                        header.col(|ui| {
                            ui.strong("Power Pin");
                        });
                        header.col(|ui| {
                            ui.strong("Ground");
                        });
                        header.col(|ui| {
                            ui.strong("Ground Pin");
                        });
                    })
                    .body(|mut body| {
                        for pin_group in &filtered_pin_groups {
                            body.row(18.0, |mut row| {
                                // Checkbox column
                                row.col(|ui| {
                                    let mut selected =
                                        voltage_config.selected_pins.contains(&pin_group.name);
                                    if ui.checkbox(&mut selected, "").changed() {
                                        if selected {
                                            voltage_config
                                                .selected_pins
                                                .insert(pin_group.name.clone());
                                        } else {
                                            voltage_config.selected_pins.remove(&pin_group.name);
                                        }
                                    }
                                });

                                // Signal name column
                                row.col(|ui| {
                                    ui.label(&pin_group.name);
                                });

                                // Type column
                                row.col(|ui| {
                                    ui.label(&pin_group.pin_type);
                                });

                                // Width column
                                row.col(|ui| {
                                    ui.label(pin_group.width.to_string());
                                });

                                // Power label column
                                row.col(|ui| {
                                    ui.label("Power:");
                                });

                                // Power selection column
                                row.col(|ui| {
                                    let current_power = voltage_config
                                        .pin_related_power
                                        .get(&pin_group.name)
                                        .cloned()
                                        .unwrap_or_else(|| {
                                            voltage_config.selected_related_power.clone()
                                        });
                                    egui::ComboBox::from_id_salt(format!(
                                        "power_{}",
                                        pin_group.name
                                    ))
                                    .selected_text(&current_power)
                                    .width(90.0)
                                    .show_ui(ui, |ui| {
                                        for power_pin in voltage_config.power_pins.keys() {
                                            if ui
                                                .selectable_label(
                                                    current_power == *power_pin,
                                                    power_pin,
                                                )
                                                .clicked()
                                            {
                                                voltage_config.pin_related_power.insert(
                                                    pin_group.name.clone(),
                                                    power_pin.clone(),
                                                );
                                            }
                                        }
                                    });
                                });

                                // Ground label column
                                row.col(|ui| {
                                    ui.label("Ground:");
                                });

                                // Ground selection column
                                row.col(|ui| {
                                    let current_ground = voltage_config
                                        .pin_related_ground
                                        .get(&pin_group.name)
                                        .cloned()
                                        .unwrap_or_else(|| {
                                            voltage_config.selected_related_ground.clone()
                                        });
                                    egui::ComboBox::from_id_salt(format!(
                                        "ground_{}",
                                        pin_group.name
                                    ))
                                    .selected_text(&current_ground)
                                    .width(90.0)
                                    .show_ui(ui, |ui| {
                                        for ground_pin in voltage_config.ground_pins.keys() {
                                            if ui
                                                .selectable_label(
                                                    current_ground == *ground_pin,
                                                    ground_pin,
                                                )
                                                .clicked()
                                            {
                                                voltage_config.pin_related_ground.insert(
                                                    pin_group.name.clone(),
                                                    ground_pin.clone(),
                                                );
                                            }
                                        }
                                    });
                                });
                            });
                        }
                    });

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Export").clicked() {
                        self.visible = false;
                        *export_callback = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.visible = false;
                    }
                });
            });
    }
}
