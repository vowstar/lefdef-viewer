// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use lefdef_viewer::export::export_verilog_stub;
use lefdef_viewer::lef::{Lef, LefMacro, LefPin};
use std::fs;

fn create_test_pin(name: &str, direction: &str, use_type: &str) -> LefPin {
    LefPin {
        name: name.to_string(),
        direction: direction.to_string(),
        use_type: use_type.to_string(),
        shape: String::new(),
        ports: Vec::new(),
    }
}

fn create_test_macro(name: &str) -> LefMacro {
    LefMacro {
        name: name.to_string(),
        class: "CORE".to_string(),
        foreign: String::new(),
        origin: (0.0, 0.0),
        size_x: 1.0,
        size_y: 1.0,
        symmetry: Vec::new(),
        site: String::new(),
        pins: Vec::new(),
        obs: Vec::new(),
    }
}

#[test]
fn test_verilog_generation_single_pin() {
    let mut lef_data = Lef { macros: Vec::new() };

    // Create a simple macro with a single pin
    let mut macro_def = create_test_macro("TEST_CELL");
    let pin = create_test_pin("A", "INPUT", "SIGNAL");
    macro_def.pins.push(pin);
    lef_data.macros.push(macro_def);

    // Export to temporary file
    let temp_file = "/tmp/test_single_pin.v";
    export_verilog_stub(&lef_data, temp_file, "test_single_pin").unwrap();

    // Read the generated file
    let content = fs::read_to_string(temp_file).unwrap();

    // Verify no trailing comma in port list
    assert!(content.contains("input A       /**< A */"));
    assert!(!content.contains("input A,"));

    // Verify module declaration ends correctly
    assert!(content.contains(");"));

    // Clean up
    fs::remove_file(temp_file).unwrap();
}

#[test]
fn test_verilog_generation_only_signal_pins() {
    let mut lef_data = Lef { macros: Vec::new() };

    // Create a macro with only signal pins
    let mut macro_def = create_test_macro("TEST_SIGNAL_ONLY");

    let pin1 = create_test_pin("A1P0_IREF", "INPUT", "SIGNAL");
    let pin2 = create_test_pin("D1P0_EN", "INPUT", "SIGNAL");

    // Add bus pin for output
    for i in 0..8 {
        let pin = create_test_pin(&format!("A1P0_IOUT[{}]", i), "OUTPUT", "SIGNAL");
        macro_def.pins.push(pin);
    }

    macro_def.pins.push(pin1);
    macro_def.pins.push(pin2);

    lef_data.macros.push(macro_def);

    // Export to temporary file
    let temp_file = "/tmp/test_signal_only.v";
    export_verilog_stub(&lef_data, temp_file, "test_signal_only").unwrap();

    // Read the generated file
    let content = fs::read_to_string(temp_file).unwrap();

    // Verify proper comma placement for signal-only pins
    assert!(content.contains("    input A1P0_IREF,       /**< A1P0_IREF */"));
    assert!(content.contains("    input D1P0_EN,       /**< D1P0_EN */"));
    // Last pin should have no comma
    assert!(content.contains("    output [7:0] A1P0_IOUT  /**< A1P0_IOUT[7:0] */"));
    assert!(!content.contains("A1P0_IOUT,"));

    // Verify no PG_EXIST blocks
    assert!(!content.contains("`ifdef PG_EXIST"));
    assert!(!content.contains("`endif  /* PG_EXIST */"));

    // Clean up
    fs::remove_file(temp_file).unwrap();
}

#[test]
fn test_verilog_generation_bus_pins() {
    let mut lef_data = Lef { macros: Vec::new() };

    // Create a macro with bus pins
    let mut macro_def = create_test_macro("TEST_BUS");

    // Create bus pins DATA[0] to DATA[3]
    for i in 0..4 {
        let pin = create_test_pin(&format!("DATA[{}]", i), "INPUT", "SIGNAL");
        macro_def.pins.push(pin);
    }

    let pin_out = create_test_pin("OUT", "OUTPUT", "SIGNAL");
    macro_def.pins.push(pin_out);

    lef_data.macros.push(macro_def);

    // Export to temporary file
    let temp_file = "/tmp/test_bus.v";
    export_verilog_stub(&lef_data, temp_file, "test_bus").unwrap();

    // Read the generated file
    let content = fs::read_to_string(temp_file).unwrap();

    // Verify bus declaration
    assert!(content.contains("input [3:0] DATA"));
    assert!(content.contains("output OUT,       /**< OUT */"));
    assert!(!content.contains("output OUT /**< OUT */,"));

    // Verify module declaration ends correctly
    assert!(content.contains(");"));

    // Clean up
    fs::remove_file(temp_file).unwrap();
}

#[test]
fn test_verilog_generation_power_pins() {
    let mut lef_data = Lef { macros: Vec::new() };

    // Create a macro with power pins
    let mut macro_def = create_test_macro("TEST_POWER");

    let vdd_pin = create_test_pin("VDD", "INOUT", "POWER");
    let vss_pin = create_test_pin("VSS", "INOUT", "GROUND");
    let signal_pin = create_test_pin("A", "INPUT", "SIGNAL");

    macro_def.pins.push(vdd_pin);
    macro_def.pins.push(vss_pin);
    macro_def.pins.push(signal_pin);

    lef_data.macros.push(macro_def);

    // Export to temporary file
    let temp_file = "/tmp/test_power.v";
    export_verilog_stub(&lef_data, temp_file, "test_power").unwrap();

    // Read the generated file
    let content = fs::read_to_string(temp_file).unwrap();

    // Verify power pins are now properly wrapped in `ifdef PG_EXIST (first)
    assert!(content.contains("`ifdef PG_EXIST"));
    assert!(content.contains("    inout VDD,       /**< VDD */"));
    assert!(content.contains("    inout VSS,       /**< VSS */"));
    assert!(content.contains("`endif  /* PG_EXIST */"));

    // Verify signal pin comes after PG pins and has no comma (last pin)
    assert!(content.contains("    input A       /**< A */"));
    assert!(!content.contains("input A,"));

    // Verify module declaration ends correctly
    assert!(content.contains(");"));

    // Clean up
    fs::remove_file(temp_file).unwrap();
}

#[test]
fn test_verilog_generation_pg_exist_mixed_pins() {
    let mut lef_data = Lef { macros: Vec::new() };

    // Create a macro with both signal and power pins
    let mut macro_def = create_test_macro("TEST_PG_EXIST_MIXED");

    // Add signal pins
    let pin1 = create_test_pin("CLK", "INPUT", "SIGNAL");
    let pin2 = create_test_pin("DATA", "INPUT", "SIGNAL");
    let pin3 = create_test_pin("Q", "OUTPUT", "SIGNAL");

    // Add power pins
    let vdd_pin = create_test_pin("VDD", "INOUT", "POWER");
    let vss_pin = create_test_pin("VSS", "INOUT", "GROUND");

    macro_def.pins.push(vdd_pin);
    macro_def.pins.push(vss_pin);
    macro_def.pins.push(pin1);
    macro_def.pins.push(pin2);
    macro_def.pins.push(pin3);

    lef_data.macros.push(macro_def);

    // Export to temporary file
    let temp_file = "/tmp/test_pg_exist_mixed.v";
    export_verilog_stub(&lef_data, temp_file, "test_pg_exist_mixed").unwrap();

    // Read the generated file
    let content = fs::read_to_string(temp_file).unwrap();

    // Verify structure: signal pins first, then power pins in ifdef
    assert!(content.contains("input CLK,       /**< CLK */"));
    assert!(content.contains("input DATA,       /**< DATA */"));
    assert!(content.contains("output Q       /**< Q */"));

    // Verify power pins are in ifdef block (first)
    assert!(content.contains("`ifdef PG_EXIST"));
    assert!(content.contains("    inout VDD,       /**< VDD */"));
    assert!(content.contains("    inout VSS,       /**< VSS */"));
    assert!(content.contains("`endif  /* PG_EXIST */"));

    // Clean up
    fs::remove_file(temp_file).unwrap();
}

#[test]
fn test_verilog_generation_only_power_pins() {
    let mut lef_data = Lef { macros: Vec::new() };

    // Create a macro with only power pins (like a power switch cell)
    let mut macro_def = create_test_macro("POWER_SWITCH");

    let vdd_pin = create_test_pin("VDD", "INOUT", "POWER");
    let vss_pin = create_test_pin("VSS", "INOUT", "GROUND");
    let vddq_pin = create_test_pin("VDDQ", "INOUT", "POWER");

    macro_def.pins.push(vdd_pin);
    macro_def.pins.push(vss_pin);
    macro_def.pins.push(vddq_pin);

    lef_data.macros.push(macro_def);

    // Export to temporary file
    let temp_file = "/tmp/test_only_power.v";
    export_verilog_stub(&lef_data, temp_file, "test_only_power").unwrap();

    // Read the generated file
    let content = fs::read_to_string(temp_file).unwrap();

    // Verify all power pins are in ifdef blocks with proper comma placement
    assert!(content.contains("`ifdef PG_EXIST"));
    assert!(content.contains("    inout VDD,       /**< VDD */"));
    assert!(content.contains("    inout VDDQ,       /**< VDDQ */"));
    // Last power pin should have no comma
    assert!(content.contains("    inout VSS       /**< VSS */"));
    assert!(!content.contains("VSS,"));
    assert!(content.contains("`endif  /* PG_EXIST */"));

    // Clean up
    fs::remove_file(temp_file).unwrap();
}

#[test]
#[ignore] // Ignored by default to avoid CI failures on systems without iverilog
fn test_iverilog_syntax_validation() {
    use std::process::Command;

    let mut lef_data = Lef { macros: Vec::new() };

    // Create a comprehensive test case
    let mut macro_def = create_test_macro("SYNTAX_TEST");

    // Add various pin types
    let signal_pin = create_test_pin("A", "INPUT", "SIGNAL");
    let bus_pins: Vec<LefPin> = (0..4)
        .map(|i| create_test_pin(&format!("DATA[{}]", i), "INPUT", "SIGNAL"))
        .collect();
    let output_pin = create_test_pin("Y", "OUTPUT", "SIGNAL");
    let vdd_pin = create_test_pin("VDD", "INOUT", "POWER");
    let vss_pin = create_test_pin("VSS", "INOUT", "GROUND");

    macro_def.pins.push(vdd_pin);
    macro_def.pins.push(vss_pin);
    macro_def.pins.push(signal_pin);
    for pin in bus_pins {
        macro_def.pins.push(pin);
    }
    macro_def.pins.push(output_pin);

    lef_data.macros.push(macro_def);

    // Export to temporary file
    let temp_file = "/tmp/test_iverilog_syntax.v";
    export_verilog_stub(&lef_data, temp_file, "test_iverilog_syntax").unwrap();

    // Test with iverilog (without PG_EXIST defined)
    let output = Command::new("iverilog")
        .args(["-Wall", temp_file, "-o", "/tmp/test_syntax_no_pg.vvp"])
        .output();

    match output {
        Ok(result) => {
            assert!(
                result.status.success(),
                "iverilog failed without PG_EXIST: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Err(e) => {
            panic!("Failed to run iverilog (not installed?): {}", e);
        }
    }

    // Test with iverilog (with PG_EXIST defined)
    let output = Command::new("iverilog")
        .args([
            "-Wall",
            "-DPG_EXIST",
            temp_file,
            "-o",
            "/tmp/test_syntax_with_pg.vvp",
        ])
        .output();

    match output {
        Ok(result) => {
            assert!(
                result.status.success(),
                "iverilog failed with PG_EXIST: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Err(e) => {
            panic!("Failed to run iverilog (not installed?): {}", e);
        }
    }

    // Clean up
    fs::remove_file(temp_file).unwrap();
    let _ = fs::remove_file("/tmp/test_syntax_no_pg.vvp");
    let _ = fs::remove_file("/tmp/test_syntax_with_pg.vvp");
}

#[test]
#[ignore] // Ignored by default for CI compatibility
fn test_iverilog_validation_only_power() {
    use std::process::Command;

    let mut lef_data = Lef { macros: Vec::new() };

    // Create test case: only PG pins
    let mut macro_def = create_test_macro("POWER_SWITCH");
    let vdd_pin = create_test_pin("VDD", "INOUT", "POWER");
    let vss_pin = create_test_pin("VSS", "INOUT", "GROUND");
    let vddq_pin = create_test_pin("VDDQ", "INOUT", "POWER");

    macro_def.pins.push(vdd_pin);
    macro_def.pins.push(vss_pin);
    macro_def.pins.push(vddq_pin);
    lef_data.macros.push(macro_def);

    // Export to temporary file
    let temp_file = "/tmp/test_iverilog_only_power.v";
    export_verilog_stub(&lef_data, temp_file, "test_iverilog_only_power").unwrap();

    // Test with iverilog (without PG_EXIST defined) - should have empty port list
    let output = Command::new("iverilog")
        .args(["-Wall", temp_file, "-o", "/tmp/test_only_power_no_pg.vvp"])
        .output();

    match output {
        Ok(result) => {
            assert!(
                result.status.success(),
                "iverilog failed without PG_EXIST: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Err(e) => {
            panic!("Failed to run iverilog (not installed?): {}", e);
        }
    }

    // Test with iverilog (with PG_EXIST defined)
    let output = Command::new("iverilog")
        .args([
            "-Wall",
            "-DPG_EXIST",
            temp_file,
            "-o",
            "/tmp/test_only_power_with_pg.vvp",
        ])
        .output();

    match output {
        Ok(result) => {
            assert!(
                result.status.success(),
                "iverilog failed with PG_EXIST: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Err(e) => {
            panic!("Failed to run iverilog (not installed?): {}", e);
        }
    }

    // Clean up
    fs::remove_file(temp_file).unwrap();
    let _ = fs::remove_file("/tmp/test_only_power_no_pg.vvp");
    let _ = fs::remove_file("/tmp/test_only_power_with_pg.vvp");
}

#[test]
#[ignore] // Ignored by default for CI compatibility
fn test_iverilog_validation_only_signal() {
    use std::process::Command;

    let mut lef_data = Lef { macros: Vec::new() };

    // Create test case: only signal pins
    let mut macro_def = create_test_macro("TEST_SIGNAL_ONLY");
    let pin1 = create_test_pin("A1P0_IREF", "INPUT", "SIGNAL");
    let pin2 = create_test_pin("D1P0_EN", "INPUT", "SIGNAL");

    for i in 0..8 {
        let pin = create_test_pin(&format!("A1P0_IOUT[{}]", i), "OUTPUT", "SIGNAL");
        macro_def.pins.push(pin);
    }

    macro_def.pins.push(pin1);
    macro_def.pins.push(pin2);
    lef_data.macros.push(macro_def);

    // Export to temporary file
    let temp_file = "/tmp/test_iverilog_only_signal.v";
    export_verilog_stub(&lef_data, temp_file, "test_iverilog_only_signal").unwrap();

    // Test with iverilog (both cases should work the same)
    let output = Command::new("iverilog")
        .args(["-Wall", temp_file, "-o", "/tmp/test_only_signal.vvp"])
        .output();

    match output {
        Ok(result) => {
            assert!(
                result.status.success(),
                "iverilog failed: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Err(e) => {
            panic!("Failed to run iverilog (not installed?): {}", e);
        }
    }

    // Clean up
    fs::remove_file(temp_file).unwrap();
    let _ = fs::remove_file("/tmp/test_only_signal.vvp");
}

#[test]
fn test_verilog_generation_pg_and_signal_pins() {
    let mut lef_data = Lef { macros: Vec::new() };

    // Create a macro with both PG and signal pins (PG first, then signal)
    let mut macro_def = create_test_macro("current_source_1to8");

    // Add power pins first (they should appear first in output)
    let avdd_pin = create_test_pin("AVDD1P0", "INOUT", "POWER");
    let avss_pin = create_test_pin("AVSS", "INOUT", "GROUND");

    // Add signal pins
    let iref_pin = create_test_pin("A1P0_IREF", "INPUT", "SIGNAL");
    let en_pin = create_test_pin("D1P0_EN", "INPUT", "SIGNAL");

    // Add bus pin for output
    for i in 0..8 {
        let pin = create_test_pin(&format!("A1P0_IOUT[{}]", i), "OUTPUT", "SIGNAL");
        macro_def.pins.push(pin);
    }

    macro_def.pins.push(avdd_pin);
    macro_def.pins.push(avss_pin);
    macro_def.pins.push(iref_pin);
    macro_def.pins.push(en_pin);

    lef_data.macros.push(macro_def);

    // Export to temporary file
    let temp_file = "/tmp/test_pg_and_signal.v";
    export_verilog_stub(&lef_data, temp_file, "test_pg_and_signal").unwrap();

    // Read the generated file
    let content = fs::read_to_string(temp_file).unwrap();

    // Verify structure: PG pins first in ifdef, then signal pins
    assert!(content.contains("`ifdef PG_EXIST"));
    assert!(content.contains("    inout AVDD1P0,       /**< AVDD1P0 */"));
    assert!(content.contains("    inout AVSS,       /**< AVSS */"));
    assert!(content.contains("`endif  /* PG_EXIST */"));

    // Verify signal pins after PG pins
    assert!(content.contains("    input A1P0_IREF,       /**< A1P0_IREF */"));
    assert!(content.contains("    input D1P0_EN,       /**< D1P0_EN */"));

    // Verify last pin has no comma
    assert!(content.contains("    output [7:0] A1P0_IOUT  /**< A1P0_IOUT[7:0] */"));
    assert!(!content.contains("A1P0_IOUT,"));

    // Clean up
    fs::remove_file(temp_file).unwrap();
}

#[test]
#[ignore] // Ignored by default for CI compatibility
fn test_iverilog_validation_pg_and_signal() {
    use std::process::Command;

    let mut lef_data = Lef { macros: Vec::new() };

    // Create test case: PG pins + signal pins
    let mut macro_def = create_test_macro("current_source_1to8");
    let avdd_pin = create_test_pin("AVDD1P0", "INOUT", "POWER");
    let avss_pin = create_test_pin("AVSS", "INOUT", "GROUND");
    let iref_pin = create_test_pin("A1P0_IREF", "INPUT", "SIGNAL");
    let en_pin = create_test_pin("D1P0_EN", "INPUT", "SIGNAL");

    for i in 0..8 {
        let pin = create_test_pin(&format!("A1P0_IOUT[{}]", i), "OUTPUT", "SIGNAL");
        macro_def.pins.push(pin);
    }

    macro_def.pins.push(avdd_pin);
    macro_def.pins.push(avss_pin);
    macro_def.pins.push(iref_pin);
    macro_def.pins.push(en_pin);

    lef_data.macros.push(macro_def);

    // Export to temporary file
    let temp_file = "/tmp/test_iverilog_pg_signal.v";
    export_verilog_stub(&lef_data, temp_file, "test_iverilog_pg_signal").unwrap();

    // Test with iverilog (without PG_EXIST defined)
    let output = Command::new("iverilog")
        .args(["-Wall", temp_file, "-o", "/tmp/test_pg_signal_no_pg.vvp"])
        .output();

    match output {
        Ok(result) => {
            assert!(
                result.status.success(),
                "iverilog failed without PG_EXIST: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Err(e) => {
            panic!("Failed to run iverilog (not installed?): {}", e);
        }
    }

    // Test with iverilog (with PG_EXIST defined)
    let output = Command::new("iverilog")
        .args([
            "-Wall",
            "-DPG_EXIST",
            temp_file,
            "-o",
            "/tmp/test_pg_signal_with_pg.vvp",
        ])
        .output();

    match output {
        Ok(result) => {
            assert!(
                result.status.success(),
                "iverilog failed with PG_EXIST: {}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        Err(e) => {
            panic!("Failed to run iverilog (not installed?): {}", e);
        }
    }

    // Clean up
    fs::remove_file(temp_file).unwrap();
    let _ = fs::remove_file("/tmp/test_pg_signal_no_pg.vvp");
    let _ = fs::remove_file("/tmp/test_pg_signal_with_pg.vvp");
}
