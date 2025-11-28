//! Test parsing real gcd.def file without GUI
//!
//! This test verifies that the parser can handle a real-world DEF file
//! with all features: DIEAREA, VIAS, COMPONENTS, PINS, SPECIALNETS, NETS

use lefdef_viewer::def::reader::DefReader;
use std::path::Path;

#[test]
#[ignore] // Only run when real file is present
fn test_parse_gcd_def() {
    let def_path = "/home/vowstar/Sync/Project/Software/2025/RUST/lefdef-smurf/gcd.def";

    // Skip test if file doesn't exist
    if !Path::new(def_path).exists() {
        eprintln!("Skipping test: {} not found", def_path);
        return;
    }

    eprintln!("[TEST] Testing real GCD DEF file: {}", def_path);
    eprintln!("======================================");

    let reader = DefReader::new();
    let result = reader.read(def_path);

    assert!(result.is_ok(), "Failed to parse gcd.def: {:?}", result);

    let def = result.unwrap();

    eprintln!("======================================");
    eprintln!("[SUCCESS] GCD DEF file parsed successfully!");
    eprintln!("======================================");
    eprintln!("[RESULT] Summary:");
    eprintln!("  DIEAREA points: {}", def.die_area_points.len());
    eprintln!("  Components: {}", def.components.len());
    eprintln!("  Pins: {}", def.pins.len());
    eprintln!("  Special nets: {}", def.special_nets.len());
    eprintln!("  Nets: {}", def.nets.len());
    eprintln!("  Vias: {}", def.vias.len());
    eprintln!("======================================");

    // Verify expected structure
    assert_eq!(def.die_area_points.len(), 2, "DIEAREA should have 2 points");
    assert_eq!(def.die_area_points[0], (0.0, 0.0));
    assert_eq!(def.die_area_points[1], (22800.0, 21000.0));

    assert_eq!(def.components.len(), 182, "Should have 182 components");
    // Note: File declares 66 pins but actually contains 56
    assert_eq!(
        def.pins.len(),
        56,
        "Should have 56 pins (actual count in file)"
    );
    assert_eq!(def.vias.len(), 7, "Should have 7 vias");

    // Expected based on the file
    assert_eq!(
        def.special_nets.len(),
        2,
        "Should have 2 special nets (VDD, VSS)"
    );
    assert_eq!(def.nets.len(), 266, "Should have 266 nets");

    eprintln!("======================================");
    eprintln!("[SAMPLE] First 5 components:");
    for (i, comp) in def.components.iter().take(5).enumerate() {
        eprintln!("  {}. {} ({})", i + 1, comp.name, comp.macro_name);
        if let Some(placement) = &comp.placement {
            eprintln!(
                "     {} at ({:.1}, {:.1}) {}",
                placement.placement_type, placement.x, placement.y, placement.orientation
            );
        }
    }

    eprintln!("======================================");
    eprintln!("[SAMPLE] First 5 pins:");
    for (i, pin) in def.pins.iter().take(5).enumerate() {
        eprintln!(
            "  {}. {} (net: {}, direction: {})",
            i + 1,
            pin.name,
            pin.net,
            pin.direction
        );
        eprintln!(
            "     {} at ({:.1}, {:.1}) {}",
            pin.status, pin.x, pin.y, pin.orient
        );
        eprintln!("     {} LAYER geometries", pin.rects.len());
    }

    eprintln!("======================================");
    eprintln!("[SAMPLE] Special nets:");
    for (i, snet) in def.special_nets.iter().enumerate() {
        eprintln!(
            "  {}. {} (use: {:?}, connections: {}, routes: {})",
            i + 1,
            snet.name,
            snet.use_type,
            snet.connections.len(),
            snet.routes.len()
        );

        // Show first route details
        if let Some(route) = snet.routes.first() {
            eprintln!(
                "     First route: {} layer {}, width {:.1}, {} points",
                route.routing_type,
                route.layer,
                route.width,
                route.points.len()
            );
        }
    }

    eprintln!("======================================");
    eprintln!("[SAMPLE] First 3 nets:");
    for (i, net) in def.nets.iter().take(3).enumerate() {
        eprintln!(
            "  {}. {} (use: {}, connections: {}, routes: {})",
            i + 1,
            net.name,
            net.use_type,
            net.connections,
            net.routes.len()
        );
    }

    eprintln!("======================================");
    eprintln!("[PASS] All checks passed!");
}
