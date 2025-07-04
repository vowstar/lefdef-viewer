// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use lefdef_viewer::lef::{Lef, LefMacro};

fn create_test_lef_data() -> Lef {
    Lef {
        macros: vec![
            LefMacro {
                name: "INVERTER_X1".to_string(),
                class: "CORE".to_string(),
                foreign: String::new(),
                origin: (0.0, 0.0),
                size_x: 10.0,
                size_y: 20.0,
                symmetry: Vec::new(),
                site: String::new(),
                pins: Vec::new(),
                obs: Vec::new(),
            },
            LefMacro {
                name: "NAND_X2".to_string(),
                class: "CORE".to_string(),
                foreign: String::new(),
                origin: (0.0, 0.0),
                size_x: 15.0,
                size_y: 20.0,
                symmetry: Vec::new(),
                site: String::new(),
                pins: Vec::new(),
                obs: Vec::new(),
            },
            LefMacro {
                name: "BUFFER_X4".to_string(),
                class: "CORE".to_string(),
                foreign: String::new(),
                origin: (0.0, 0.0),
                size_x: 20.0,
                size_y: 20.0,
                symmetry: Vec::new(),
                site: String::new(),
                pins: Vec::new(),
                obs: Vec::new(),
            },
            LefMacro {
                name: "AND_X1".to_string(),
                class: "CORE".to_string(),
                foreign: String::new(),
                origin: (0.0, 0.0),
                size_x: 12.0,
                size_y: 20.0,
                symmetry: Vec::new(),
                site: String::new(),
                pins: Vec::new(),
                obs: Vec::new(),
            },
            LefMacro {
                name: "CURRENT_SOURCE_1TO8".to_string(),
                class: "CORE".to_string(),
                foreign: String::new(),
                origin: (0.0, 0.0),
                size_x: 25.0,
                size_y: 30.0,
                symmetry: Vec::new(),
                site: String::new(),
                pins: Vec::new(),
                obs: Vec::new(),
            },
        ],
    }
}

#[test]
fn test_macro_filter_empty() {
    let lef_data = create_test_lef_data();
    let filter = "";

    let filtered: Vec<&LefMacro> = lef_data
        .macros
        .iter()
        .filter(|macro_def| {
            if filter.is_empty() {
                true
            } else {
                macro_def
                    .name
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
            }
        })
        .collect();

    assert_eq!(filtered.len(), 5);
    assert_eq!(filtered[0].name, "INVERTER_X1");
    assert_eq!(filtered[1].name, "NAND_X2");
    assert_eq!(filtered[2].name, "BUFFER_X4");
    assert_eq!(filtered[3].name, "AND_X1");
    assert_eq!(filtered[4].name, "CURRENT_SOURCE_1TO8");
}

#[test]
fn test_macro_filter_inverter() {
    let lef_data = create_test_lef_data();
    let filter = "INVERTER";

    let filtered: Vec<&LefMacro> = lef_data
        .macros
        .iter()
        .filter(|macro_def| {
            if filter.is_empty() {
                true
            } else {
                macro_def
                    .name
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
            }
        })
        .collect();

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "INVERTER_X1");
}

#[test]
fn test_macro_filter_case_insensitive() {
    let lef_data = create_test_lef_data();
    let filter = "current";

    let filtered: Vec<&LefMacro> = lef_data
        .macros
        .iter()
        .filter(|macro_def| {
            if filter.is_empty() {
                true
            } else {
                macro_def
                    .name
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
            }
        })
        .collect();

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "CURRENT_SOURCE_1TO8");
}

#[test]
fn test_macro_filter_x1() {
    let lef_data = create_test_lef_data();
    let filter = "X1";

    let filtered: Vec<&LefMacro> = lef_data
        .macros
        .iter()
        .filter(|macro_def| {
            if filter.is_empty() {
                true
            } else {
                macro_def
                    .name
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
            }
        })
        .collect();

    assert_eq!(filtered.len(), 2);
    assert_eq!(filtered[0].name, "INVERTER_X1");
    assert_eq!(filtered[1].name, "AND_X1");
}

#[test]
fn test_macro_filter_no_match() {
    let lef_data = create_test_lef_data();
    let filter = "NONEXISTENT";

    let filtered: Vec<&LefMacro> = lef_data
        .macros
        .iter()
        .filter(|macro_def| {
            if filter.is_empty() {
                true
            } else {
                macro_def
                    .name
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
            }
        })
        .collect();

    assert_eq!(filtered.len(), 0);
}

#[test]
fn test_macro_filter_partial_match() {
    let lef_data = create_test_lef_data();
    let filter = "X";

    let filtered: Vec<&LefMacro> = lef_data
        .macros
        .iter()
        .filter(|macro_def| {
            if filter.is_empty() {
                true
            } else {
                macro_def
                    .name
                    .to_lowercase()
                    .contains(&filter.to_lowercase())
            }
        })
        .collect();

    // Should match INVERTER_X1, NAND_X2, BUFFER_X4, AND_X1
    assert_eq!(filtered.len(), 4);
    assert_eq!(filtered[0].name, "INVERTER_X1");
    assert_eq!(filtered[1].name, "NAND_X2");
    assert_eq!(filtered[2].name, "BUFFER_X4");
    assert_eq!(filtered[3].name, "AND_X1");
}
