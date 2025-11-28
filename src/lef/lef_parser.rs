// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use nom::{
    branch::alt,
    bytes::complete::{tag, take_until},
    character::complete::{alpha1, alphanumeric1, char, multispace0, space0, space1},
    combinator::recognize,
    multi::many0,
    number::complete::double,
    sequence::{delimited, pair},
    IResult, Parser,
};

use super::{Lef, LefMacro, LefObstruction, LefPin, LefPolygon, LefPort, LefRect};
use crate::def::preprocessor::preprocess;

fn calculate_polygon_winding(points: &[(f64, f64)]) -> bool {
    if points.len() < 3 {
        return false;
    }

    let mut sum = 0.0;
    for i in 0..points.len() {
        let j = (i + 1) % points.len();
        sum += (points[j].0 - points[i].0) * (points[j].1 + points[i].1);
    }

    sum > 0.0 // clockwise (hole) if positive, counterclockwise (solid) if negative
}

fn identifier(input: &str) -> IResult<&str, &str> {
    recognize(pair(
        alt((alpha1, tag("_"))),
        many0(alt((alphanumeric1, tag("_")))),
    ))
    .parse(input)
}

#[allow(dead_code)]
fn string_literal(input: &str) -> IResult<&str, &str> {
    alt((
        delimited(char('"'), take_until("\""), char('"')),
        identifier,
    ))
    .parse(input)
}

#[allow(dead_code)]
fn parse_rect(input: &str) -> IResult<&str, LefRect> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("LAYER")(input)?;
    let (input, _) = space0(input)?;
    let (input, layer) = identifier(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("RECT")(input)?;
    let (input, _) = space0(input)?;
    let (input, xl) = double(input)?;
    let (input, _) = space0(input)?;
    let (input, yl) = double(input)?;
    let (input, _) = space0(input)?;
    let (input, xh) = double(input)?;
    let (input, _) = space0(input)?;
    let (input, yh) = double(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag(";")(input)?;

    Ok((
        input,
        LefRect {
            layer: layer.to_string(),
            xl,
            yl,
            xh,
            yh,
        },
    ))
}

#[allow(dead_code)]
fn parse_polygon(input: &str) -> IResult<&str, LefPolygon> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("LAYER")(input)?;
    let (input, _) = space0(input)?;
    let (input, _) = space1(input)?;
    let (input, layer) = identifier(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("POLYGON")(input)?;
    let (input, _) = multispace0(input)?;

    // Parse coordinate pairs
    let mut points = Vec::new();
    let mut remaining = input;

    loop {
        let (rest, _) = multispace0(remaining)?;

        // Check for end marker
        if rest.starts_with(';') {
            let (rest, _) = tag(";")(rest)?;
            let is_hole = calculate_polygon_winding(&points);
            return Ok((
                rest,
                LefPolygon {
                    layer: layer.to_string(),
                    points,
                    is_hole,
                },
            ));
        }

        // Try to parse a coordinate pair
        if let Ok((rest, x)) = double::<&str, nom::error::Error<&str>>(rest) {
            let (rest, _) = space1(rest)?;
            if let Ok((rest, y)) = double::<&str, nom::error::Error<&str>>(rest) {
                points.push((x, y));
                remaining = rest;
                continue;
            }
        }

        break;
    }

    // If we get here, parsing failed
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Tag,
    )))
}

// Similar to parse_pin, this is handled manually in parse_simple_macro
#[allow(dead_code)]
fn parse_port(_input: &str) -> IResult<&str, LefPort> {
    unimplemented!("PORT parsing is handled by parse_simple_macro")
}

// This parser is used for the parts that the manual parser doesn't handle
// But our manual parser already handles the full PIN parsing, so this is mainly a stub
#[allow(dead_code)]
fn parse_pin(_input: &str) -> IResult<&str, LefPin> {
    // This function is not actually used since parse_simple_macro handles all PIN parsing manually
    // Keeping it as a stub to satisfy the interface
    unimplemented!("PIN parsing is handled by parse_simple_macro")
}

#[allow(dead_code)]
fn parse_obstruction(input: &str) -> IResult<&str, LefObstruction> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("OBS")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, rects) = many0(parse_rect).parse(input)?;
    let (input, polygons) = many0(parse_polygon).parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("END")(input)?;

    Ok((input, LefObstruction { rects, polygons }))
}

fn parse_simple_macro(input: &str) -> IResult<&str, LefMacro> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("MACRO")(input)?;
    let (input, _) = space1(input)?;
    let (input, name) = identifier(input)?;
    let (input, _) = multispace0(input)?;

    println!("[DBG] Parsing MACRO: {name}");

    // Parse macro content with PIN extraction
    let remaining = input;
    let mut pins: Vec<LefPin> = Vec::new();
    let mut obstructions: Vec<LefObstruction> = Vec::new();
    let mut size_x = 0.0;
    let mut size_y = 0.0;
    let mut origin_x = 0.0;
    let mut origin_y = 0.0;
    let mut class = String::new();
    let mut site = String::new();
    let mut _source = String::new();
    let mut _foreign_x = 0.0;
    let mut _foreign_y = 0.0;
    let symmetry = Vec::new();

    let end_pattern = format!("END {name}");

    // Use preprocessor for multi-line statement support
    let preprocessed = preprocess(remaining);
    let lines = &preprocessed.lines;
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }

        if line.starts_with(&end_pattern) {
            println!(
                "[PASS] Found macro: {} (size: {:.3}x{:.3}, pins: {})",
                name,
                size_x,
                size_y,
                pins.len()
            );

            // Find where this line ends in the original input
            if let Some(pos) = remaining.find(&end_pattern) {
                let rest = &remaining[pos + end_pattern.len()..];
                return Ok((
                    rest,
                    LefMacro {
                        name: name.to_string(),
                        class,
                        foreign: _source,
                        origin: (origin_x, origin_y),
                        size_x,
                        size_y,
                        symmetry,
                        site,
                        pins,
                        obs: if !obstructions.is_empty() {
                            obstructions
                        } else {
                            vec![]
                        },
                    },
                ));
            }
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            i += 1;
            continue;
        }

        match parts[0] {
            "CLASS" if parts.len() > 1 => {
                class = parts[1].trim_end_matches(';').to_string();
            }
            "SOURCE" if parts.len() > 1 => {
                _source = parts[1].trim_end_matches(';').to_string();
            }
            "SITE" if parts.len() > 1 => {
                site = parts[1].trim_end_matches(';').to_string();
            }
            "ORIGIN" if parts.len() > 2 => {
                if let (Ok(x), Ok(y)) = (parts[1].parse::<f64>(), parts[2].parse::<f64>()) {
                    origin_x = x;
                    origin_y = y;
                }
            }
            "SIZE" if parts.len() > 3 && parts[2] == "BY" => {
                if let (Ok(x), Ok(y)) = (parts[1].parse::<f64>(), parts[3].parse::<f64>()) {
                    size_x = x;
                    size_y = y;
                }
            }
            "FOREIGN" if parts.len() > 3 => {
                _source = parts[1].to_string();
                if let (Ok(x), Ok(y)) = (parts[2].parse::<f64>(), parts[3].parse::<f64>()) {
                    _foreign_x = x;
                    _foreign_y = y;
                }
            }
            "PIN" if parts.len() > 1 => {
                // Parse PIN section
                let pin_name = parts[1].to_string();
                println!("[DBG]   Parsing PIN: {pin_name}");

                let mut direction = String::new();
                let mut use_type = String::new();
                let mut shape = String::new();
                let mut ports = Vec::new();

                // Check if DIRECTION/USE/SHAPE are on the same line (preprocessed)
                // Format: PIN name DIRECTION dir ; or PIN name DIRECTION dir USE type ;
                for j in 2..parts.len() {
                    match parts[j] {
                        "DIRECTION" if j + 1 < parts.len() => {
                            direction = parts[j + 1].trim_end_matches(';').to_string();
                        }
                        "USE" if j + 1 < parts.len() => {
                            use_type = parts[j + 1].trim_end_matches(';').to_string();
                            if use_type == "POWER" || use_type == "GROUND" {
                                println!(
                                    "[DBG]     Found POWER/GROUND pin: {pin_name} (USE: {use_type})"
                                );
                            }
                        }
                        "SHAPE" if j + 1 < parts.len() => {
                            shape = parts[j + 1].trim_end_matches(';').to_string();
                        }
                        _ => {}
                    }
                }

                // Look for PORT sections within this PIN
                i += 1;
                while i < lines.len() {
                    let pin_line = lines[i].trim();
                    if pin_line.starts_with("END") || pin_line.starts_with("PIN ") {
                        break;
                    }

                    let pin_parts: Vec<&str> = pin_line.split_whitespace().collect();
                    if !pin_parts.is_empty() {
                        match pin_parts[0] {
                            "DIRECTION" if pin_parts.len() > 1 => {
                                direction = pin_parts[1].trim_end_matches(';').to_string();
                            }
                            "USE" if pin_parts.len() > 1 => {
                                use_type = pin_parts[1].trim_end_matches(';').to_string();
                                if use_type == "POWER" || use_type == "GROUND" {
                                    println!(
                                        "[DBG]     Found POWER/GROUND pin: {pin_name} (USE: {use_type})"
                                    );
                                }
                            }
                            "SHAPE" if pin_parts.len() > 1 => {
                                shape = pin_parts[1].trim_end_matches(';').to_string();
                            }
                            "PORT" => {
                                // Parse PORT content
                                println!(
                                    "[DBG]     Found PORT in pin {pin_name} (USE: {use_type})"
                                );
                                let mut rects = Vec::new();
                                let mut polygons = Vec::new();
                                let mut current_layer = String::new();

                                // Check if LAYER is on the same line (preprocessed)
                                // Format: PORT LAYER M1 ;
                                for j in 1..pin_parts.len() {
                                    if pin_parts[j] == "LAYER" && j + 1 < pin_parts.len() {
                                        current_layer =
                                            pin_parts[j + 1].trim_end_matches(';').to_string();
                                        if use_type == "POWER" || use_type == "GROUND" {
                                            println!("[DBG]       POWER/GROUND pin {pin_name} using layer: {current_layer}");
                                        }
                                        break;
                                    }
                                }

                                i += 1;
                                while i < lines.len() {
                                    let port_line = lines[i].trim();
                                    if port_line.starts_with("END")
                                        || port_line.starts_with("PORT")
                                        || port_line.starts_with("PIN ")
                                    {
                                        if port_line.starts_with("END") {
                                            i += 1; // Skip the END line
                                        }
                                        break;
                                    }

                                    let port_parts: Vec<&str> =
                                        port_line.split_whitespace().collect();
                                    if !port_parts.is_empty() {
                                        println!("[DBG]       Processing port line: {port_line}");
                                        match port_parts[0] {
                                            "LAYER" if port_parts.len() > 1 => {
                                                current_layer =
                                                    port_parts[1].trim_end_matches(';').to_string();
                                                if use_type == "POWER" || use_type == "GROUND" {
                                                    println!("[DBG]       POWER/GROUND pin {pin_name} using layer: {current_layer}");
                                                }
                                            }
                                            "RECT" if port_parts.len() >= 5 => {
                                                if let (Ok(xl), Ok(yl), Ok(xh), Ok(yh)) = (
                                                    port_parts[1].parse::<f64>(),
                                                    port_parts[2].parse::<f64>(),
                                                    port_parts[3].parse::<f64>(),
                                                    port_parts[4].parse::<f64>(),
                                                ) {
                                                    rects.push(LefRect {
                                                        layer: current_layer.clone(),
                                                        xl,
                                                        yl,
                                                        xh,
                                                        yh,
                                                    });
                                                    if use_type == "POWER" || use_type == "GROUND" {
                                                        println!("[DBG]       Added POWER/GROUND rect on {current_layer}: ({xl:.1},{yl:.1}) -> ({xh:.1},{yh:.1})");
                                                    } else {
                                                        println!("[DBG]       Added rect on {current_layer}: ({xl:.1},{yl:.1}) -> ({xh:.1},{yh:.1})");
                                                    }
                                                }
                                            }
                                            "POLYGON" => {
                                                // Parse polygon coordinates - may span multiple lines
                                                let mut points = Vec::new();
                                                let mut mask_num: Option<i32> = None;

                                                // Collect all POLYGON content across multiple lines until semicolon
                                                let mut polygon_content = String::new();
                                                polygon_content.push_str(port_line);

                                                // Continue collecting until we find a semicolon
                                                let mut poly_i = i + 1;
                                                while !polygon_content.contains(';')
                                                    && poly_i < lines.len()
                                                {
                                                    polygon_content.push(' ');
                                                    polygon_content.push_str(lines[poly_i].trim());
                                                    poly_i += 1;
                                                }

                                                // Update main loop index
                                                i = poly_i - 1;

                                                // Parse all content
                                                let poly_parts: Vec<&str> =
                                                    polygon_content.split_whitespace().collect();
                                                let mut part_idx = 1; // Skip "POLYGON"

                                                // Check for MASK
                                                if part_idx < poly_parts.len()
                                                    && poly_parts[part_idx] == "MASK"
                                                {
                                                    part_idx += 1;
                                                    if part_idx < poly_parts.len() {
                                                        if let Ok(mask) =
                                                            poly_parts[part_idx].parse::<i32>()
                                                        {
                                                            mask_num = Some(mask);
                                                        }
                                                        part_idx += 1;
                                                    }
                                                }

                                                // Parse coordinate pairs
                                                while part_idx + 1 < poly_parts.len() {
                                                    let x_str =
                                                        poly_parts[part_idx].trim_end_matches(';');
                                                    let y_str = poly_parts[part_idx + 1]
                                                        .trim_end_matches(';');

                                                    if let (Ok(x), Ok(y)) =
                                                        (x_str.parse::<f64>(), y_str.parse::<f64>())
                                                    {
                                                        points.push((x, y));
                                                        part_idx += 2;
                                                    } else {
                                                        break;
                                                    }
                                                }

                                                if !points.is_empty() {
                                                    let is_hole =
                                                        calculate_polygon_winding(&points);
                                                    polygons.push(LefPolygon {
                                                        layer: current_layer.clone(),
                                                        points,
                                                        is_hole,
                                                    });
                                                    println!("[DBG]       Added polygon on {} with {} points ({}){}: {:?}",
                                                           current_layer, polygons.last().unwrap().points.len(),
                                                           if is_hole { "hole" } else { "solid" },
                                                           if let Some(mask) = mask_num { format!(" MASK {mask}") } else { String::new() },
                                                           polygons.last().unwrap().points);
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                    i += 1;
                                }

                                if !rects.is_empty() || !polygons.is_empty() {
                                    ports.push(LefPort { rects, polygons });
                                }
                                continue; // Don't increment i again
                            }
                            _ => {}
                        }
                    }
                    i += 1;
                }

                pins.push(LefPin {
                    name: pin_name,
                    direction,
                    use_type,
                    shape,
                    ports,
                });
                continue; // Don't increment i again since we already processed PIN content
            }
            "OBS" => {
                // Parse OBS section
                println!("[DBG]   Parsing OBS");
                let mut rects = Vec::new();
                let mut polygons = Vec::new();
                let mut current_layer = String::new();

                i += 1;
                while i < lines.len() {
                    let obs_line = lines[i].trim();
                    if obs_line.starts_with("END") && obs_line != "END OBS" {
                        break;
                    }

                    let obs_parts: Vec<&str> = obs_line.split_whitespace().collect();
                    if !obs_parts.is_empty() {
                        println!("[DBG]     Processing OBS line: {obs_line}");
                        match obs_parts[0] {
                            "LAYER" if obs_parts.len() > 1 => {
                                current_layer = obs_parts[1].to_string();
                            }
                            "RECT" if obs_parts.len() >= 5 => {
                                if let (Ok(xl), Ok(yl), Ok(xh), Ok(yh)) = (
                                    obs_parts[1].parse::<f64>(),
                                    obs_parts[2].parse::<f64>(),
                                    obs_parts[3].parse::<f64>(),
                                    obs_parts[4].parse::<f64>(),
                                ) {
                                    rects.push(LefRect {
                                        layer: current_layer.clone(),
                                        xl,
                                        yl,
                                        xh,
                                        yh,
                                    });
                                    println!("[DBG]     Added OBS rect on {current_layer}: ({xl:.1},{yl:.1}) -> ({xh:.1},{yh:.1})");
                                }
                            }
                            "POLYGON" => {
                                // Parse polygon coordinates - may span multiple lines
                                let mut points = Vec::new();
                                let mut mask_num: Option<i32> = None;

                                // Collect all POLYGON content across multiple lines until semicolon
                                let mut polygon_content = String::new();
                                polygon_content.push_str(obs_line);

                                // Continue collecting until we find a semicolon
                                let mut poly_i = i + 1;
                                while !polygon_content.contains(';') && poly_i < lines.len() {
                                    polygon_content.push(' ');
                                    polygon_content.push_str(lines[poly_i].trim());
                                    poly_i += 1;
                                }

                                // Update main loop index
                                i = poly_i - 1;

                                // Parse all content
                                let poly_parts: Vec<&str> =
                                    polygon_content.split_whitespace().collect();
                                let mut part_idx = 1; // Skip "POLYGON"

                                // Check for MASK
                                if part_idx < poly_parts.len() && poly_parts[part_idx] == "MASK" {
                                    part_idx += 1;
                                    if part_idx < poly_parts.len() {
                                        if let Ok(mask) = poly_parts[part_idx].parse::<i32>() {
                                            mask_num = Some(mask);
                                        }
                                        part_idx += 1;
                                    }
                                }

                                // Parse coordinate pairs
                                while part_idx + 1 < poly_parts.len() {
                                    let x_str = poly_parts[part_idx].trim_end_matches(';');
                                    let y_str = poly_parts[part_idx + 1].trim_end_matches(';');

                                    if let (Ok(x), Ok(y)) =
                                        (x_str.parse::<f64>(), y_str.parse::<f64>())
                                    {
                                        points.push((x, y));
                                        part_idx += 2;
                                    } else {
                                        break;
                                    }
                                }

                                if !points.is_empty() {
                                    let is_hole = calculate_polygon_winding(&points);
                                    polygons.push(LefPolygon {
                                        layer: current_layer.clone(),
                                        points,
                                        is_hole,
                                    });
                                    println!(
                                        "[DBG]     Added OBS polygon on {} with {} points ({}){}: {:?}",
                                        current_layer,
                                        polygons.last().unwrap().points.len(),
                                        if is_hole { "hole" } else { "solid" },
                                        if let Some(mask) = mask_num { format!(" MASK {mask}") } else { String::new() },
                                        polygons.last().unwrap().points
                                    );
                                }
                            }
                            "END" => {
                                break; // End of OBS section
                            }
                            _ => {}
                        }
                    }
                    i += 1;
                }

                // Store the obstruction data in the macro
                obstructions.push(LefObstruction { rects, polygons });

                println!(
                    "[DBG]   OBS parsing complete: {} rects, {} polygons",
                    obstructions.last().unwrap().rects.len(),
                    obstructions.last().unwrap().polygons.len()
                );
                continue;
            }
            _ => {}
        }
        i += 1;
    }

    // If we get here, we didn't find the END
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Tag,
    )))
}

fn parse_macro(input: &str) -> IResult<&str, LefMacro> {
    parse_simple_macro(input)
}

#[allow(dead_code)]
fn skip_to_macro(input: &str) -> IResult<&str, &str> {
    let mut remaining = input;

    loop {
        let (rest, _) = multispace0(remaining)?;
        if rest.is_empty() {
            break;
        }

        if let Ok((_rest, _)) = tag::<&str, &str, nom::error::Error<&str>>("MACRO")(rest) {
            return Ok((remaining, remaining));
        }

        // Skip to next line
        if let Some(newline_pos) = rest.find('\n') {
            remaining = &rest[newline_pos + 1..];
        } else {
            break;
        }
    }

    Ok((remaining, remaining))
}

pub fn parse_lef(input: &str) -> IResult<&str, Lef> {
    println!("[DBG] Starting LEF parsing...");
    println!("[DBG] Preprocessing LEF file...");

    // Preprocess entire file first
    let preprocessed = preprocess(input);
    println!(
        "[DBG] Preprocessed: {} logical lines from {} raw lines",
        preprocessed.lines.len(),
        input.lines().count()
    );

    let (mut input, _) = multispace0(input)?;
    let mut macros = Vec::new();

    // Skip header content and find MACROs
    loop {
        let (rest, _) = multispace0(input)?;
        if rest.is_empty() {
            break;
        }

        // Try to parse a MACRO
        if let Ok((rest, macro_def)) = parse_macro(rest) {
            macros.push(macro_def);
            input = rest;
        } else {
            // Skip to next line
            if let Some(newline_pos) = rest.find('\n') {
                input = &rest[newline_pos + 1..];
            } else {
                break;
            }
        }
    }

    println!("[PASS] LEF parsed: {} macros", macros.len());
    Ok((input, Lef { macros }))
}
