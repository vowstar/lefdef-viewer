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

use super::{Def, DefGCellGrid};

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

                    // Parse components until END COMPONENTS
                    while i < lines.len() {
                        let comp_line = lines[i].trim();
                        if comp_line.starts_with("END COMPONENTS") {
                            break;
                        }

                        let comp_parts: Vec<&str> = comp_line.split_whitespace().collect();
                        if comp_parts.len() >= 2 && comp_parts[0] == "-" {
                            // Component line: - componentName macroName + PLACED ( x y ) orientation ;
                            let comp_id = comp_parts[1].to_string();
                            let comp_name = if comp_parts.len() > 2 {
                                comp_parts[2].to_string()
                            } else {
                                "unknown".to_string()
                            };

                            let mut x = 0.0;
                            let mut y = 0.0;
                            let mut orient = String::new();

                            // Look for PLACED ( x y ) orientation
                            for j in 3..comp_parts.len() {
                                if comp_parts[j] == "PLACED"
                                    && j + 4 < comp_parts.len()
                                    && comp_parts[j + 1] == "("
                                    && comp_parts[j + 4] == ")"
                                {
                                    if let (Ok(px), Ok(py)) = (
                                        comp_parts[j + 2].parse::<f64>(),
                                        comp_parts[j + 3].parse::<f64>(),
                                    ) {
                                        x = px;
                                        y = py;
                                        if j + 5 < comp_parts.len() {
                                            orient =
                                                comp_parts[j + 5].trim_end_matches(';').to_string();
                                        }
                                        break;
                                    }
                                }
                            }

                            components.push(crate::def::DefComponent {
                                id: comp_id.clone(),
                                name: comp_name,
                                status: "PLACED".to_string(),
                                source: "USER".to_string(),
                                orient,
                                x,
                                y,
                            });

                            println!("🔧     Component: {} at ({:.1}, {:.1})", comp_id, x, y);
                        }
                        i += 1;
                    }
                }
            }
            "PINS" if parts.len() > 1 => {
                if let Ok(num_pins) = parts[1].parse::<usize>() {
                    println!("🔧   Found PINS section with {} pins", num_pins);
                    i += 1;

                    // Parse pins until END PINS
                    while i < lines.len() {
                        let pin_line = lines[i].trim();
                        if pin_line.starts_with("END PINS") {
                            break;
                        }

                        let pin_parts: Vec<&str> = pin_line.split_whitespace().collect();
                        if pin_parts.len() >= 2 && pin_parts[0] == "-" {
                            let pin_name = pin_parts[1].to_string();

                            pins.push(crate::def::DefPin {
                                name: pin_name.clone(),
                                net: "".to_string(),
                                use_type: "".to_string(),
                                status: "".to_string(),
                                direction: "".to_string(),
                                orient: "".to_string(),
                                x: 0.0,
                                y: 0.0,
                                rects: Vec::new(),
                                ports: Vec::new(),
                            });

                            println!("🔧     Pin: {}", pin_name);
                        }
                        i += 1;
                    }
                }
            }
            "NETS" if parts.len() > 1 => {
                if let Ok(num_nets) = parts[1].parse::<usize>() {
                    println!("🔧   Found NETS section with {} nets", num_nets);
                    // Skip detailed net parsing for now
                    while i < lines.len() && !lines[i].trim().starts_with("END NETS") {
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
        },
    ))
}

pub fn parse_def(input: &str) -> IResult<&str, Def> {
    parse_def_simple(input)
}
