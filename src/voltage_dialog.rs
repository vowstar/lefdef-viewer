// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

//! Enhanced Voltage Configuration Dialog
//!
//! This module provides a comprehensive voltage configuration dialog for Liberty export
//! with support for thousands of pins, batch selection, and per-pin power/ground configuration.

use crate::export::VoltageConfig;
use crate::lef::Lef;
use eframe::egui;

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
        // Collect all unique power, ground, and regular pins
        let mut power_pins = std::collections::BTreeSet::new();
        let mut ground_pins = std::collections::BTreeSet::new();
        let mut all_pins = std::collections::BTreeSet::new();

        for macro_def in &lef_data.macros {
            for pin in &macro_def.pins {
                let clean_name = pin.name.replace('!', "");
                all_pins.insert(clean_name.clone());
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

        // Select all pins by default
        voltage_config.selected_pins = all_pins;
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
            .default_height(600.0)
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
                        egui::ComboBox::from_id_source("related_power")
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
                        egui::ComboBox::from_id_source("related_ground")
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

                // Collect all pins from LEF data for display
                let mut all_pin_names: Vec<String> = Vec::new();
                if let Some(lef_data) = lef_data {
                    let mut pins_set = std::collections::BTreeSet::new();
                    for macro_def in &lef_data.macros {
                        for pin in &macro_def.pins {
                            pins_set.insert(pin.name.replace('!', ""));
                        }
                    }
                    all_pin_names = pins_set.into_iter().collect();
                }

                // Filter pins based on search
                let filtered_pins: Vec<&String> = all_pin_names
                    .iter()
                    .filter(|pin_name| {
                        voltage_config.pin_filter.is_empty()
                            || pin_name
                                .to_lowercase()
                                .contains(&voltage_config.pin_filter.to_lowercase())
                    })
                    .collect();

                // Batch selection controls
                ui.horizontal(|ui| {
                    if ui.button("Select All").clicked() {
                        for pin_name in &all_pin_names {
                            voltage_config.selected_pins.insert(pin_name.clone());
                        }
                    }
                    if ui.button("Deselect All").clicked() {
                        voltage_config.selected_pins.clear();
                    }
                    if ui.button("Select Filtered").clicked() {
                        for pin_name in &filtered_pins {
                            voltage_config.selected_pins.insert((*pin_name).clone());
                        }
                    }
                    if ui.button("Deselect Filtered").clicked() {
                        for pin_name in &filtered_pins {
                            voltage_config.selected_pins.remove(*pin_name);
                        }
                    }
                    ui.label(format!(
                        "Selected: {}/{}",
                        voltage_config.selected_pins.len(),
                        all_pin_names.len()
                    ));
                });

                // Batch assignment controls for selected pins
                if !voltage_config.selected_pins.is_empty() {
                    ui.separator();
                    ui.label("Batch Assignment for Selected Pins:");
                    ui.horizontal(|ui| {
                        ui.label("Set Related Power:");
                        egui::ComboBox::from_id_source("batch_power")
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
                        egui::ComboBox::from_id_source("batch_ground")
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

                // Pin list with scrollable area
                ui.label(format!(
                    "Pin List (showing {} of {}):",
                    filtered_pins.len(),
                    all_pin_names.len()
                ));
                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width());
                        for pin_name in &filtered_pins {
                            ui.horizontal(|ui| {
                                // Pin selection checkbox
                                let mut selected = voltage_config.selected_pins.contains(*pin_name);
                                if ui.checkbox(&mut selected, "").changed() {
                                    if selected {
                                        voltage_config.selected_pins.insert((*pin_name).clone());
                                    } else {
                                        voltage_config.selected_pins.remove(*pin_name);
                                    }
                                }

                                // Pin name
                                ui.label(*pin_name);

                                // Individual related power selection
                                ui.label("Power:");
                                let current_power = voltage_config
                                    .pin_related_power
                                    .get(*pin_name)
                                    .cloned()
                                    .unwrap_or_else(|| {
                                        voltage_config.selected_related_power.clone()
                                    });
                                egui::ComboBox::from_id_source(format!("power_{}", pin_name))
                                    .selected_text(&current_power)
                                    .width(80.0)
                                    .show_ui(ui, |ui| {
                                        for power_pin in voltage_config.power_pins.keys() {
                                            if ui
                                                .selectable_label(
                                                    current_power == *power_pin,
                                                    power_pin,
                                                )
                                                .clicked()
                                            {
                                                voltage_config
                                                    .pin_related_power
                                                    .insert((*pin_name).clone(), power_pin.clone());
                                            }
                                        }
                                    });

                                // Individual related ground selection
                                ui.label("Ground:");
                                let current_ground = voltage_config
                                    .pin_related_ground
                                    .get(*pin_name)
                                    .cloned()
                                    .unwrap_or_else(|| {
                                        voltage_config.selected_related_ground.clone()
                                    });
                                egui::ComboBox::from_id_source(format!("ground_{}", pin_name))
                                    .selected_text(&current_ground)
                                    .width(80.0)
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
                                                    (*pin_name).clone(),
                                                    ground_pin.clone(),
                                                );
                                            }
                                        }
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
