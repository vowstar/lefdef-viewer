//! Integration tests for DEF routing geometry parsing
//!
//! Tests cover:
//! - SPECIALNETS with ROUTED, NEW, SHAPE keywords
//! - NETS with routing geometry
//! - PINS with LAYER geometry
//! - Real-world routing patterns

use lefdef_viewer::def::def_parser;

#[test]
fn test_specialnet_basic_routing() {
    let def_content = r#"
VERSION 5.8 ;
DIVIDERCHAR "/" ;
DESIGN test ;

SPECIALNETS 1 ;
- VDD ( * VDD )
  + ROUTED metal1 170 + SHAPE FOLLOWPIN ( 0 0 ) ( 22800 0 ) ;
END SPECIALNETS

END DESIGN
"#;

    let result = def_parser::parse_def(def_content);
    assert!(result.is_ok(), "Failed to parse SPECIALNETS: {:?}", result);

    let (_, def) = result.unwrap();
    assert_eq!(def.special_nets.len(), 1);

    let vdd = &def.special_nets[0];
    assert_eq!(vdd.name, "VDD");
    assert_eq!(vdd.connections.len(), 1);
    assert_eq!(vdd.connections[0].0, "*");
    assert_eq!(vdd.connections[0].1, "VDD");
    assert_eq!(vdd.routes.len(), 1);

    let route = &vdd.routes[0];
    assert_eq!(route.layer, "metal1");
    assert_eq!(route.width, 170.0);
    assert_eq!(route.shape, Some("FOLLOWPIN".to_string()));

    // Debug: print actual points
    eprintln!("Route points: {:?}", route.points);
    eprintln!("Route points count: {}", route.points.len());

    assert_eq!(route.points.len(), 2);
    assert_eq!(route.points[0].x, 0.0);
    assert_eq!(route.points[0].y, 0.0);
    assert_eq!(route.points[1].x, 22800.0);
    assert_eq!(route.points[1].y, 0.0);
}

#[test]
fn test_specialnet_multi_segment_routing() {
    let def_content = r#"
VERSION 5.8 ;
DESIGN test ;

SPECIALNETS 1 ;
- VDD ( * VDD )
  + ROUTED metal1 170 + SHAPE FOLLOWPIN ( 0 0 ) ( 22800 0 )
    NEW metal1 170 + SHAPE FOLLOWPIN ( 0 2800 ) ( 22800 2800 )
    NEW metal8 1000 + SHAPE STRIPE ( 2300 0 ) ( 2300 21000 ) ;
END SPECIALNETS

END DESIGN
"#;

    let result = def_parser::parse_def(def_content);
    assert!(
        result.is_ok(),
        "Failed to parse multi-segment SPECIALNETS: {:?}",
        result
    );

    let (_, def) = result.unwrap();
    assert_eq!(def.special_nets.len(), 1);

    let vdd = &def.special_nets[0];
    assert_eq!(vdd.routes.len(), 3);

    // First segment: metal1 FOLLOWPIN
    assert_eq!(vdd.routes[0].layer, "metal1");
    assert_eq!(vdd.routes[0].shape, Some("FOLLOWPIN".to_string()));

    // Second segment: metal1 FOLLOWPIN at different Y
    assert_eq!(vdd.routes[1].layer, "metal1");
    assert_eq!(vdd.routes[1].points[0].y, 2800.0);

    // Third segment: metal8 STRIPE (vertical power strap)
    assert_eq!(vdd.routes[2].layer, "metal8");
    assert_eq!(vdd.routes[2].width, 1000.0);
    assert_eq!(vdd.routes[2].shape, Some("STRIPE".to_string()));
    assert_eq!(vdd.routes[2].points[0].x, 2300.0);
    assert_eq!(vdd.routes[2].points[1].y, 21000.0);
}

#[test]
fn test_specialnet_power_and_ground() {
    let def_content = r#"
VERSION 5.8 ;
DESIGN test ;

SPECIALNETS 2 ;
- VDD ( * VDD )
  + USE POWER
  + VOLTAGE 1.8
  + ROUTED metal1 170 ( 0 0 ) ( 1000 0 ) ;
- VSS ( * VSS )
  + USE GROUND
  + ROUTED metal1 170 ( 0 1400 ) ( 1000 1400 ) ;
END SPECIALNETS

END DESIGN
"#;

    let result = def_parser::parse_def(def_content);
    assert!(
        result.is_ok(),
        "Failed to parse power/ground nets: {:?}",
        result
    );

    let (_, def) = result.unwrap();
    assert_eq!(def.special_nets.len(), 2);

    // Check VDD
    let vdd = &def.special_nets[0];
    assert_eq!(vdd.name, "VDD");
    assert_eq!(vdd.use_type, Some("POWER".to_string()));
    assert_eq!(vdd.voltage, Some(1.8));

    // Check VSS
    let vss = &def.special_nets[1];
    assert_eq!(vss.name, "VSS");
    assert_eq!(vss.use_type, Some("GROUND".to_string()));
}

#[test]
fn test_pin_with_layer_geometry() {
    let def_content = r#"
VERSION 5.8 ;
DESIGN test ;

DIEAREA ( 0 0 ) ( 100000 100000 ) ;

PINS 2 ;
- clk + NET clk + DIRECTION INPUT + USE SIGNAL
  + LAYER metal2 ( -35 0 ) ( 35 70 )
  + PLACED ( 10545 21000 ) S ;
- req_msg[31] + NET req_msg[31] + DIRECTION INPUT + USE SIGNAL
  + LAYER metal2 ( -35 0 ) ( 35 70 )
  + PLACED ( 13585 0 ) N ;
END PINS

END DESIGN
"#;

    let result = def_parser::parse_def(def_content);
    assert!(
        result.is_ok(),
        "Failed to parse pins with geometry: {:?}",
        result
    );

    let (_, def) = result.unwrap();
    assert_eq!(def.pins.len(), 2);

    // Check first pin
    let clk_pin = &def.pins[0];
    assert_eq!(clk_pin.name, "clk");
    assert_eq!(clk_pin.direction, "INPUT");
    assert_eq!(clk_pin.rects.len(), 1);

    let rect = &clk_pin.rects[0];
    assert_eq!(rect.layer, "metal2");
    assert_eq!(rect.xl, -35.0);
    assert_eq!(rect.yl, 0.0);
    assert_eq!(rect.xh, 35.0);
    assert_eq!(rect.yh, 70.0);
}

#[test]
fn test_nets_with_routing() {
    let def_content = r#"
VERSION 5.8 ;
DESIGN test ;

COMPONENTS 2 ;
- inv1 INVX1 + PLACED ( 1000 2000 ) N ;
- nand1 NAND2X1 + PLACED ( 3000 2000 ) N ;
END COMPONENTS

NETS 1 ;
- net1 ( PIN IN1 ) ( inv1 A ) ( nand1 B )
  + USE SIGNAL
  + ROUTED metal2 ( 1000 2000 ) ( 1500 2000 ) ( 1500 2500 ) ;
END NETS

END DESIGN
"#;

    let result = def_parser::parse_def(def_content);
    assert!(
        result.is_ok(),
        "Failed to parse nets with routing: {:?}",
        result
    );

    let (_, def) = result.unwrap();
    assert_eq!(def.nets.len(), 1);

    let net = &def.nets[0];
    assert_eq!(net.name, "net1");
    assert_eq!(net.use_type, "SIGNAL");

    // Debug: print actual connections
    eprintln!("Net connections: {}", net.connections);
    eprintln!("Net instances: {:?}", net.instances);
    eprintln!("Net instance_pins: {:?}", net.instance_pins);

    assert_eq!(net.connections, 3);

    // Check routing - should have parsed the route
    // Note: Current implementation has basic routing parsing
    // Full implementation will capture all routing points
}

#[test]
fn test_diearea_rectangle() {
    let def_content = r#"
VERSION 5.8 ;
DESIGN test ;

DIEAREA ( 0 0 ) ( 22800 21000 ) ;

END DESIGN
"#;

    let result = def_parser::parse_def(def_content);
    assert!(result.is_ok(), "Failed to parse DIEAREA: {:?}", result);

    let (_, def) = result.unwrap();
    assert_eq!(def.die_area_points.len(), 2);
    assert_eq!(def.die_area_points[0], (0.0, 0.0));
    assert_eq!(def.die_area_points[1], (22800.0, 21000.0));
}

#[test]
fn test_diearea_polygon() {
    let def_content = r#"
VERSION 5.8 ;
DESIGN test ;

DIEAREA ( 0 0 ) ( 0 1000 ) ( 500 1000 ) ( 500 500 ) ( 1000 500 ) ( 1000 0 ) ;

END DESIGN
"#;

    let result = def_parser::parse_def(def_content);
    assert!(
        result.is_ok(),
        "Failed to parse polygon DIEAREA: {:?}",
        result
    );

    let (_, def) = result.unwrap();
    assert_eq!(def.die_area_points.len(), 6);
    // L-shaped die area
    assert_eq!(def.die_area_points[0], (0.0, 0.0));
    assert_eq!(def.die_area_points[1], (0.0, 1000.0));
    assert_eq!(def.die_area_points[2], (500.0, 1000.0));
    assert_eq!(def.die_area_points[5], (1000.0, 0.0));
}

#[test]
fn test_complete_minimal_design() {
    let def_content = r#"
VERSION 5.8 ;
DIVIDERCHAR "/" ;
BUSBITCHARS "[]" ;
DESIGN minimal_gcd ;
UNITS DISTANCE MICRONS 1000 ;

DIEAREA ( 0 0 ) ( 10000 10000 ) ;

COMPONENTS 1 ;
- inv1 INVX1 + PLACED ( 1000 2000 ) N ;
END COMPONENTS

PINS 1 ;
- clk + NET clk + DIRECTION INPUT + USE SIGNAL
  + LAYER metal2 ( -35 0 ) ( 35 70 )
  + PLACED ( 5000 10000 ) S ;
END PINS

SPECIALNETS 1 ;
- VDD ( * VDD )
  + USE POWER
  + ROUTED metal1 170 + SHAPE FOLLOWPIN ( 0 0 ) ( 10000 0 ) ;
END SPECIALNETS

NETS 1 ;
- clk ( PIN clk ) ( inv1 A )
  + USE SIGNAL ;
END NETS

END DESIGN
"#;

    let result = def_parser::parse_def(def_content);
    assert!(
        result.is_ok(),
        "Failed to parse complete design: {:?}",
        result
    );

    let (_, def) = result.unwrap();

    // Verify all sections parsed
    assert_eq!(def.die_area_points.len(), 2);
    assert_eq!(def.components.len(), 1);
    assert_eq!(def.pins.len(), 1);
    assert_eq!(def.special_nets.len(), 1);
    assert_eq!(def.nets.len(), 1);

    // Verify DIEAREA
    assert_eq!(def.die_area_points[0], (0.0, 0.0));
    assert_eq!(def.die_area_points[1], (10000.0, 10000.0));

    // Verify COMPONENTS
    assert_eq!(def.components[0].name, "inv1");

    // Verify PINS
    assert_eq!(def.pins[0].name, "clk");
    assert_eq!(def.pins[0].rects.len(), 1);

    // Verify SPECIALNETS
    assert_eq!(def.special_nets[0].name, "VDD");
    assert_eq!(def.special_nets[0].use_type, Some("POWER".to_string()));

    // Verify NETS
    assert_eq!(def.nets[0].name, "clk");
}

#[test]
fn test_nets_routing_layer_parsing() {
    let def_content = r#"
VERSION 5.8 ;
DESIGN test ;

NETS 3 ;
- net1 ( INST1 A ) ( INST2 B )
  + ROUTED metal2 ( 1000 2000 ) ( 1500 2000 ) ( 1500 2500 ) ;
- net2 ( INST3 C ) ( INST4 D )
  + ROUTED metal3 ( 2000 3000 ) ( 2500 3000 )
    NEW metal4 ( 2500 3000 ) ( 2500 3500 ) ;
- net3 ( INST5 E ) ( INST6 F )
  + ROUTED metal1 ( 500 1000 ) ( 1000 1000 ) via1_4
    NEW metal2 ( 1000 1000 ) ( 1000 1500 ) ;
END NETS

END DESIGN
"#;

    let result = def_parser::parse_def(def_content);
    assert!(result.is_ok(), "Failed to parse NETS routing: {:?}", result);

    let (_, def) = result.unwrap();
    assert_eq!(def.nets.len(), 3);

    // Check net1: single metal2 route
    let net1 = &def.nets[0];
    assert_eq!(net1.name, "net1");
    assert_eq!(net1.routes.len(), 1);
    assert_eq!(net1.routes[0].layer, "metal2");
    assert_eq!(net1.routes[0].points.len(), 3);
    assert_eq!(net1.routes[0].routing_type, "ROUTED");

    // Check net2: metal3 + metal4
    let net2 = &def.nets[1];
    assert_eq!(net2.name, "net2");
    assert_eq!(net2.routes.len(), 2);
    assert_eq!(net2.routes[0].layer, "metal3");
    assert_eq!(net2.routes[0].points.len(), 2);
    assert_eq!(net2.routes[1].layer, "metal4");
    assert_eq!(net2.routes[1].points.len(), 2);
    assert_eq!(net2.routes[1].routing_type, "NEW");

    // Check net3: metal1 + metal2 with via
    let net3 = &def.nets[2];
    assert_eq!(net3.name, "net3");
    assert_eq!(net3.routes.len(), 2);
    assert_eq!(net3.routes[0].layer, "metal1");
    assert_eq!(net3.routes[0].points.len(), 2);
    assert_eq!(net3.routes[1].layer, "metal2");
    assert_eq!(net3.routes[1].points.len(), 2);
}

#[test]
fn test_specialnets_routing_layer_parsing() {
    let def_content = r#"
VERSION 5.8 ;
DESIGN test ;

SPECIALNETS 2 ;
- VDD ( * VDD )
  + USE POWER
  + ROUTED metal1 170 ( 0 0 ) ( 1000 0 )
    NEW metal2 200 ( 1000 0 ) ( 1000 1000 )
    NEW metal8 1000 + SHAPE STRIPE ( 500 0 ) ( 500 5000 ) ;
- VSS ( * VSS )
  + USE GROUND
  + ROUTED metal1 170 ( 0 1400 ) ( 1000 1400 )
    NEW metal3 250 ( 1000 1400 ) ( 2000 1400 ) ;
END SPECIALNETS

END DESIGN
"#;

    let result = def_parser::parse_def(def_content);
    assert!(
        result.is_ok(),
        "Failed to parse SPECIALNETS routing: {:?}",
        result
    );

    let (_, def) = result.unwrap();
    assert_eq!(def.special_nets.len(), 2);

    // Check VDD: metal1 + metal2 + metal8
    let vdd = &def.special_nets[0];
    assert_eq!(vdd.name, "VDD");
    assert_eq!(vdd.use_type, Some("POWER".to_string()));
    assert_eq!(vdd.routes.len(), 3);

    assert_eq!(vdd.routes[0].layer, "metal1");
    assert_eq!(vdd.routes[0].width, 170.0);
    assert_eq!(vdd.routes[0].points.len(), 2);

    assert_eq!(vdd.routes[1].layer, "metal2");
    assert_eq!(vdd.routes[1].width, 200.0);
    assert_eq!(vdd.routes[1].points.len(), 2);

    assert_eq!(vdd.routes[2].layer, "metal8");
    assert_eq!(vdd.routes[2].width, 1000.0);
    assert_eq!(vdd.routes[2].shape, Some("STRIPE".to_string()));
    assert_eq!(vdd.routes[2].points.len(), 2);

    // Check VSS: metal1 + metal3
    let vss = &def.special_nets[1];
    assert_eq!(vss.name, "VSS");
    assert_eq!(vss.use_type, Some("GROUND".to_string()));
    assert_eq!(vss.routes.len(), 2);

    assert_eq!(vss.routes[0].layer, "metal1");
    assert_eq!(vss.routes[0].width, 170.0);

    assert_eq!(vss.routes[1].layer, "metal3");
    assert_eq!(vss.routes[1].width, 250.0);
}

#[test]
fn test_nets_wildcard_coordinates() {
    let def_content = r#"
VERSION 5.8 ;
DIVIDERCHAR "/" ;
BUSBITCHARS "[]" ;
DESIGN test_wildcard ;
UNITS DISTANCE MICRONS 1000 ;

DIEAREA ( 0 0 ) ( 10000 10000 ) ;

NETS 2 ;
- net1 ( INST1 A ) ( INST2 B )
  + ROUTED metal2 ( 1000 2000 ) ( * 3000 ) ( 1500 * ) ;
- net2 ( INST3 C ) ( INST4 D )
  + ROUTED metal3 ( 500 1000 ) ( * 2000 )
    NEW metal4 ( 1000 * ) ( 1500 2500 ) ;
END NETS

END DESIGN
"#;

    let (_, def) = def_parser::parse_def(def_content).unwrap();

    assert_eq!(def.nets.len(), 2);

    // Test net1: single route with wildcard coordinates
    let net1 = &def.nets[0];
    assert_eq!(net1.name, "net1");
    assert_eq!(net1.routes.len(), 1);
    assert_eq!(net1.routes[0].layer, "metal2");
    assert_eq!(net1.routes[0].points.len(), 3);
    // First point: explicit (1000, 2000)
    assert_eq!(net1.routes[0].points[0].x, 1000.0);
    assert_eq!(net1.routes[0].points[0].y, 2000.0);
    // Second point: wildcard x=*, explicit y=3000 -> (1000, 3000)
    assert_eq!(net1.routes[0].points[1].x, 1000.0);
    assert_eq!(net1.routes[0].points[1].y, 3000.0);
    // Third point: explicit x=1500, wildcard y=* -> (1500, 3000)
    assert_eq!(net1.routes[0].points[2].x, 1500.0);
    assert_eq!(net1.routes[0].points[2].y, 3000.0);

    // Test net2: multi-segment route with wildcard in NEW segment
    let net2 = &def.nets[1];
    assert_eq!(net2.name, "net2");
    assert_eq!(net2.routes.len(), 2);

    // First segment: metal3
    assert_eq!(net2.routes[0].layer, "metal3");
    assert_eq!(net2.routes[0].points.len(), 2);
    assert_eq!(net2.routes[0].points[0].x, 500.0);
    assert_eq!(net2.routes[0].points[0].y, 1000.0);
    assert_eq!(net2.routes[0].points[1].x, 500.0);
    assert_eq!(net2.routes[0].points[1].y, 2000.0);

    // Second segment: metal4 (NEW continues from previous segment)
    assert_eq!(net2.routes[1].layer, "metal4");
    assert_eq!(net2.routes[1].routing_type, "NEW");
    assert_eq!(net2.routes[1].points.len(), 2);
    // First point of NEW: explicit x=1000, wildcard y=* inherits from previous segment (2000)
    assert_eq!(net2.routes[1].points[0].x, 1000.0);
    assert_eq!(net2.routes[1].points[0].y, 2000.0);
    // Second point: explicit (1500, 2500)
    assert_eq!(net2.routes[1].points[1].x, 1500.0);
    assert_eq!(net2.routes[1].points[1].y, 2500.0);
}
