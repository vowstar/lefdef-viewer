// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until, take_while1},
    character::complete::{char, i32 as parse_i32, multispace0, space1},
    combinator::opt,
    error::ParseError,
    multi::separated_list0,
    number::complete::double,
    sequence::{delimited, preceded, terminated, tuple},
    IResult,
};

use super::{Def, DefGCellGrid, DefPolygon, DefVia, DefViaLayer};

fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_' || c == '.' || c == '/' || c == '-')(
        input,
    )
}

fn string_literal(input: &str) -> IResult<&str, &str> {
    alt((
        delimited(char('"'), take_until("\""), char('"')),
        identifier,
    ))(input)
}

fn parse_die_area(input: &str) -> IResult<&str, Vec<(f64, f64)>> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("DIEAREA")(input)?;
    let (input, _) = space1(input)?;
    let (input, points) = separated_list0(
        space1,
        tuple((
            preceded(tag("("), double),
            preceded(space1, terminated(double, tag(")"))),
        )),
    )(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((input, points))
}

fn parse_gcell_grid(input: &str) -> IResult<&str, (Vec<DefGCellGrid>, Vec<DefGCellGrid>)> {
    let (input, _) = multispace0(input)?;

    let mut gcell_x = Vec::new();
    let mut gcell_y = Vec::new();
    let mut remaining = input;

    while let Ok((rest, _)) = preceded(
        multispace0::<&str, nom::error::Error<&str>>,
        tag("GCELLGRID"),
    )(remaining)
    {
        let (rest, _) = space1(rest)?;
        let (rest, direction) = identifier(rest)?;
        let (rest, _) = space1(rest)?;
        let (rest, offset) = double(rest)?;
        let (rest, _) = space1(rest)?;
        let (rest, _) = tag("DO")(rest)?;
        let (rest, _) = space1(rest)?;
        let (rest, num) = parse_i32(rest)?;
        let (rest, _) = space1(rest)?;
        let (rest, _) = tag("STEP")(rest)?;
        let (rest, _) = space1(rest)?;
        let (rest, step) = double(rest)?;
        let (rest, _) = multispace0(rest)?;
        let (rest, _) = tag(";")(rest)?;

        let grid = DefGCellGrid { offset, num, step };

        if direction.to_uppercase() == "X" {
            gcell_x.push(grid);
        } else if direction.to_uppercase() == "Y" {
            gcell_y.push(grid);
        }

        remaining = rest;
    }

    Ok((remaining, (gcell_x, gcell_y)))
}

fn parse_def_simple(input: &str) -> IResult<&str, Def> {
    println!("🔧 Starting DEF parsing...");

    let mut die_area_points = Vec::new();
    let mut components = Vec::new();
    let mut pins = Vec::new();
    let mut nets = Vec::new();
    let mut vias = Vec::new();

    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            i += 1;
            continue;
        }

        match parts[0] {
            "DIEAREA" => {
                println!("🔧   Found DIEAREA");

                // Collect all DIEAREA content across multiple lines until we find the semicolon
                let mut diearea_content = String::new();
                let mut line_idx = i;

                // Add current line content (starting from DIEAREA)
                diearea_content.push_str(line);

                // Continue collecting until we find a semicolon
                while !diearea_content.contains(';') && line_idx + 1 < lines.len() {
                    line_idx += 1;
                    diearea_content.push(' ');
                    diearea_content.push_str(lines[line_idx].trim());
                }

                // Update the main loop index
                i = line_idx;

                // Parse all points from the collected content
                let content_parts: Vec<&str> = diearea_content.split_whitespace().collect();
                let mut j = 1; // Skip "DIEAREA"

                while j < content_parts.len() {
                    if content_parts[j] == "("
                        && j + 3 < content_parts.len()
                        && content_parts[j + 3] == ")"
                    {
                        if let (Ok(x), Ok(y)) = (
                            content_parts[j + 1].parse::<f64>(),
                            content_parts[j + 2].parse::<f64>(),
                        ) {
                            die_area_points.push((x, y));
                            println!("🔧     Die area point: ({:.1}, {:.1})", x, y);
                        }
                        j += 4; // Move past ( x y )
                    } else if content_parts[j] == ";" {
                        break; // End of DIEAREA statement
                    } else {
                        j += 1;
                    }
                }
            }
            "COMPONENTS" if parts.len() > 1 => {
                if let Ok(num_components) = parts[1].parse::<usize>() {
                    println!(
                        "🔧   Found COMPONENTS section with {} components",
                        num_components
                    );
                    i += 1;

                    // Use the new unified parsing framework
                    let component_parser = crate::def::parser::component::DefComponentParser;
                    let multi_parser = crate::def::parser::MultiLineParser::new(component_parser)
                        .with_debug(true)
                        .with_max_iterations(50000)
                        .with_timeout(std::time::Duration::from_secs(120))
                        .with_max_repeated_lines(10)
                        .with_max_lines_per_item(500);

                    match multi_parser.parse_section(&lines, i, "END COMPONENTS") {
                        Ok((parsed_components, next_index)) => {
                            for component in parsed_components {
                                let placement_info =
                                    if let Some(ref placement) = component.placement {
                                        format!(
                                            "{} at ({:.1}, {:.1}) {}",
                                            placement.placement_type,
                                            placement.x,
                                            placement.y,
                                            placement.orientation
                                        )
                                    } else {
                                        "no placement".to_string()
                                    };
                                println!(
                                    "🔧     Component: {} ({}) {}",
                                    component.name, component.macro_name, placement_info
                                );
                                components.push(component);
                            }
                            i = next_index;
                        }
                        Err(e) => {
                            println!("🔧   Error parsing COMPONENTS section: {}", e);
                            // Fallback: skip to END COMPONENTS
                            while i < lines.len() && !lines[i].trim().starts_with("END COMPONENTS")
                            {
                                i += 1;
                            }
                        }
                    }
                }
            }
            "PINS" if parts.len() > 1 => {
                if let Ok(num_pins) = parts[1].parse::<usize>() {
                    println!("🔧   Found PINS section with {} pins", num_pins);
                    i += 1;

                    // Use the new unified parsing framework
                    let pin_parser = crate::def::parser::pin::DefPinParser::new();
                    let multi_parser = crate::def::parser::MultiLineParser::new(pin_parser)
                        .with_debug(true)
                        .with_max_iterations(50000)
                        .with_timeout(std::time::Duration::from_secs(120))
                        .with_max_repeated_lines(10)
                        .with_max_lines_per_item(200);

                    match multi_parser.parse_section(&lines, i, "END PINS") {
                        Ok((parsed_pins, next_index)) => {
                            for pin in parsed_pins {
                                println!(
                                    "🔧     Pin: {} at ({:.1}, {:.1}) dir={} use={}",
                                    pin.name, pin.x, pin.y, pin.direction, pin.use_type
                                );
                                pins.push(pin);
                            }
                            i = next_index;
                        }
                        Err(e) => {
                            println!("🔧   Error parsing PINS section: {}", e);
                            // Fallback: skip to END PINS
                            while i < lines.len() && !lines[i].trim().starts_with("END PINS") {
                                i += 1;
                            }
                        }
                    }
                }
            }
            "NETS" if parts.len() > 1 => {
                if let Ok(num_nets) = parts[1].parse::<usize>() {
                    println!("🔧   Found NETS section with {} nets", num_nets);
                    i += 1;

                    // Use the new unified parsing framework
                    let net_parser = crate::def::parser::net::DefNetParser::new();
                    let multi_parser = crate::def::parser::MultiLineParser::new(net_parser)
                        .with_debug(true)
                        .with_max_iterations(50000)
                        .with_timeout(std::time::Duration::from_secs(120))
                        .with_max_repeated_lines(10)
                        .with_max_lines_per_item(1000);

                    match multi_parser.parse_section(&lines, i, "END NETS") {
                        Ok((parsed_nets, next_index)) => {
                            for net in parsed_nets {
                                println!(
                                    "🔧     Net: {} with {} instances, {} pins",
                                    net.name, net.connections, net.pins
                                );
                                nets.push(net);
                            }
                            i = next_index;
                        }
                        Err(e) => {
                            println!("🔧   Error parsing NETS section: {}", e);
                            // Fallback: skip to END NETS
                            while i < lines.len() && !lines[i].trim().starts_with("END NETS") {
                                i += 1;
                            }
                        }
                    }
                }
            }
            "VIAS" if parts.len() > 1 => {
                if let Ok(num_vias) = parts[1].parse::<usize>() {
                    println!("🔧   Found VIAS section with {} vias", num_vias);
                    i += 1;

                    // Parse vias until END VIAS
                    while i < lines.len() {
                        let via_line = lines[i].trim();
                        if via_line.starts_with("END VIAS") {
                            break;
                        }

                        let via_parts: Vec<&str> = via_line.split_whitespace().collect();
                        if via_parts.len() >= 2 && via_parts[0] == "-" {
                            // Via definition: - viaName
                            let via_name = via_parts[1].to_string();
                            println!("🔧     Parsing VIA: {}", via_name);

                            let mut layers = Vec::new();

                            i += 1;
                            // Parse via content until next via or END VIAS
                            while i < lines.len() {
                                let via_content_line = lines[i].trim();
                                if via_content_line.starts_with("END VIAS")
                                    || (via_content_line.starts_with('-')
                                        && via_content_line.len() > 1)
                                {
                                    break;
                                }

                                let content_parts: Vec<&str> =
                                    via_content_line.split_whitespace().collect();
                                if !content_parts.is_empty() && content_parts[0] == "+" {
                                    // Layer-specific definition
                                    if content_parts.len() >= 2 {
                                        match content_parts[1] {
                                            "RECT" => {
                                                // + RECT layerName ( xl yl ) ( xh yh )
                                                if content_parts.len() >= 8 {
                                                    let layer_name = content_parts[2].to_string();
                                                    if let (Ok(xl), Ok(yl), Ok(xh), Ok(yh)) = (
                                                        content_parts[4].parse::<f64>(),
                                                        content_parts[5].parse::<f64>(),
                                                        content_parts[7].parse::<f64>(),
                                                        content_parts[8].parse::<f64>(),
                                                    ) {
                                                        // Find or create layer
                                                        let layer_index = layers.iter().position(
                                                            |l: &DefViaLayer| {
                                                                l.layer_name == layer_name
                                                            },
                                                        );

                                                        if let Some(idx) = layer_index {
                                                            layers[idx].rects.push(
                                                                crate::def::DefRect {
                                                                    layer: layer_name.clone(),
                                                                    xl,
                                                                    yl,
                                                                    xh,
                                                                    yh,
                                                                },
                                                            );
                                                        } else {
                                                            let mut new_layer = DefViaLayer {
                                                                layer_name: layer_name.clone(),
                                                                mask: None,
                                                                rects: Vec::new(),
                                                                polygons: Vec::new(),
                                                            };
                                                            new_layer.rects.push(
                                                                crate::def::DefRect {
                                                                    layer: layer_name,
                                                                    xl,
                                                                    yl,
                                                                    xh,
                                                                    yh,
                                                                },
                                                            );
                                                            layers.push(new_layer);
                                                        }

                                                        println!("🔧       Added RECT on layer {} at ({:.1},{:.1}) -> ({:.1},{:.1})", 
                                                               content_parts[2], xl, yl, xh, yh);
                                                    }
                                                }
                                            }
                                            "POLYGON" => {
                                                // + POLYGON layerName [+ MASK maskNum] ( x1 y1 ) ( x2 y2 ) ...
                                                if content_parts.len() >= 3 {
                                                    let layer_name = content_parts[2].to_string();
                                                    let mut mask_num: Option<i32> = None;

                                                    // Collect all POLYGON content across multiple lines until semicolon
                                                    let mut polygon_content = String::new();
                                                    polygon_content.push_str(via_content_line);

                                                    // Continue collecting until we find a semicolon
                                                    let mut poly_i = i + 1;
                                                    while !polygon_content.contains(';')
                                                        && poly_i < lines.len()
                                                    {
                                                        let next_line = lines[poly_i].trim();
                                                        // Stop if we hit next via definition or END VIAS
                                                        if next_line.starts_with('-')
                                                            || next_line.starts_with("END VIAS")
                                                        {
                                                            break;
                                                        }
                                                        polygon_content.push(' ');
                                                        polygon_content.push_str(next_line);
                                                        poly_i += 1;
                                                    }

                                                    // Update main loop index
                                                    i = poly_i - 1;

                                                    // Parse all content
                                                    let poly_parts: Vec<&str> = polygon_content
                                                        .split_whitespace()
                                                        .collect();
                                                    let mut part_idx = 3; // Skip "+ POLYGON layerName"

                                                    // Check for MASK
                                                    if part_idx < poly_parts.len()
                                                        && poly_parts[part_idx] == "+"
                                                        && part_idx + 1 < poly_parts.len()
                                                        && poly_parts[part_idx + 1] == "MASK"
                                                    {
                                                        part_idx += 2;
                                                        if part_idx < poly_parts.len() {
                                                            if let Ok(mask) =
                                                                poly_parts[part_idx].parse::<i32>()
                                                            {
                                                                mask_num = Some(mask);
                                                            }
                                                            part_idx += 1;
                                                        }
                                                    }

                                                    // Parse coordinate pairs within parentheses
                                                    let mut points = Vec::new();
                                                    while part_idx < poly_parts.len() {
                                                        if poly_parts[part_idx] == "("
                                                            && part_idx + 3 < poly_parts.len()
                                                            && poly_parts[part_idx + 3] == ")"
                                                        {
                                                            if let (Ok(x), Ok(y)) = (
                                                                poly_parts[part_idx + 1]
                                                                    .parse::<f64>(),
                                                                poly_parts[part_idx + 2]
                                                                    .parse::<f64>(),
                                                            ) {
                                                                points.push((x, y));
                                                                part_idx += 4; // Move past ( x y )
                                                            } else {
                                                                break;
                                                            }
                                                        } else if poly_parts[part_idx] == ";" {
                                                            break;
                                                        } else {
                                                            part_idx += 1;
                                                        }
                                                    }

                                                    if !points.is_empty() {
                                                        // Find or create layer
                                                        let layer_index = layers.iter().position(
                                                            |l: &DefViaLayer| {
                                                                l.layer_name == layer_name
                                                            },
                                                        );

                                                        if let Some(idx) = layer_index {
                                                            layers[idx].polygons.push(DefPolygon {
                                                                points: points.clone(),
                                                            });
                                                            if mask_num.is_some() {
                                                                layers[idx].mask = mask_num;
                                                            }
                                                        } else {
                                                            let mut new_layer = DefViaLayer {
                                                                layer_name: layer_name.clone(),
                                                                mask: mask_num,
                                                                rects: Vec::new(),
                                                                polygons: Vec::new(),
                                                            };
                                                            new_layer.polygons.push(DefPolygon {
                                                                points: points.clone(),
                                                            });
                                                            layers.push(new_layer);
                                                        }

                                                        println!("🔧       Added POLYGON on layer {} with {} points{}: {:?}", 
                                                               layer_name, points.len(),
                                                               if let Some(mask) = mask_num { format!(" MASK {}", mask) } else { String::new() },
                                                               points);
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                i += 1;
                            }

                            vias.push(DefVia {
                                name: via_name,
                                layers,
                                via_rule: None,
                                cut_size: None,
                                cut_spacing: None,
                                enclosure: Vec::new(),
                                pattern: String::new(),
                            });
                            continue; // Don't increment i again
                        }
                        i += 1;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }

    println!(
        "✅ DEF parsed: {} die points, {} components, {} pins",
        die_area_points.len(),
        components.len(),
        pins.len()
    );

    Ok((
        "",
        Def {
            die_area_points,
            g_cell_grid_x: Vec::new(),
            g_cell_grid_y: Vec::new(),
            pins,
            nets,
            components,
            rows: Vec::new(),
            tracks_x: Vec::new(),
            tracks_y: Vec::new(),
            vias,
        },
    ))
}

pub fn parse_def(input: &str) -> IResult<&str, Def> {
    parse_def_simple(input)
}
