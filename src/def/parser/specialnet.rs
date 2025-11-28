// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

//! SPECIALNET parser implementation for DEF files
//!
//! SPECIALNETS are used for power distribution networks (VDD, VSS)
//! and other non-rippable routing (clock trees, critical analog paths)

use super::common::*;
use super::{ContinuationResult, DefItemParser, ParseResult};
use crate::def::{DefRoute, DefRoutingPoint, DefSpecialNet};

/// Context for parsing a single SPECIALNET
#[derive(Debug)]
pub struct SpecialNetContext {
    pub name: String,
    pub connections: Vec<(String, String)>, // (instance, pin)
    pub routes: Vec<DefRoute>,
    pub use_type: Option<String>,
    pub weight: Option<f64>,
    pub voltage: Option<f64>,
    pub original_net: Option<String>,
    pub current_route: Option<DefRoute>, // Accumulator for building current route
}

impl SpecialNetContext {
    pub fn new(name: String) -> Self {
        Self {
            name,
            connections: Vec::new(),
            routes: Vec::new(),
            use_type: None,
            weight: None,
            voltage: None,
            original_net: None,
            current_route: None,
        }
    }

    /// Start a new route segment
    fn start_route(
        &mut self,
        layer: String,
        width: f64,
        routing_type: String,
        shape: Option<String>,
    ) {
        // Finalize any existing route
        if let Some(route) = self.current_route.take() {
            self.routes.push(route);
        }

        // Create new route
        self.current_route = Some(DefRoute {
            layer,
            width,
            routing_type,
            shape,
            points: Vec::new(),
            vias: Vec::new(),
            mask: None,
            style: None,
        });
    }

    /// Add a point to the current route
    fn add_point(&mut self, x: f64, y: f64) {
        if let Some(route) = &mut self.current_route {
            route.points.push(DefRoutingPoint { x, y, ext: None });
        }
    }

    /// Add a via to the current route
    fn add_via(&mut self, via_name: String, x: f64, y: f64) {
        if let Some(route) = &mut self.current_route {
            route.vias.push((via_name, x, y));
        }
    }

    /// Finalize the current route
    fn finalize_route(&mut self) {
        if let Some(route) = self.current_route.take() {
            self.routes.push(route);
        }
    }
}

/// SPECIALNET parser for DEF files
pub struct DefSpecialNetParser;

impl DefSpecialNetParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefSpecialNetParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DefItemParser for DefSpecialNetParser {
    type Item = DefSpecialNet;
    type Context = SpecialNetContext;

    fn parse_header(&self, line: &str) -> Option<Self::Context> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Check for SPECIALNET header: "- NETNAME ..."
        if parts.len() >= 2 && parts[0] == "-" {
            if let Some(net_name) = parse_identifier(parts[1]) {
                // Only create context with name, don't parse attributes here
                // All attributes will be parsed in parse_continuation
                let context = SpecialNetContext::new(net_name.to_string());
                return Some(context);
            }
        }

        None
    }

    fn parse_continuation(&self, context: &mut Self::Context, line: &str) -> ContinuationResult {
        let trimmed = line.trim();

        // Check for end conditions
        if trimmed.contains(';') {
            // Parse any final attributes in this line before ending
            // Note: finalize_route is called inside parse_routing_in_line when it sees ';'
            self.parse_line_attributes(context, trimmed);
            return ContinuationResult::Complete;
        }

        if is_item_header(trimmed) {
            context.finalize_route();
            return ContinuationResult::NextItem;
        }

        if is_section_end(trimmed, "SPECIALNETS") {
            context.finalize_route();
            return ContinuationResult::NextItem;
        }

        // Parse continuation line attributes
        self.parse_line_attributes(context, trimmed);
        ContinuationResult::Continue
    }

    fn finalize(&self, context: Self::Context) -> ParseResult<Self::Item> {
        Ok(DefSpecialNet {
            name: context.name,
            connections: context.connections,
            routes: context.routes,
            use_type: context.use_type,
            weight: context.weight,
            voltage: context.voltage,
            original_net: context.original_net,
        })
    }

    fn item_name() -> &'static str {
        "SPECIALNET"
    }
}

impl DefSpecialNetParser {
    /// Parse connections from any line that might contain them
    /// Format: ( * PIN ) or ( INSTANCE PIN )
    /// Only parse connections before routing keywords (ROUTED, FIXED, etc.)
    fn parse_connections_in_line(&self, context: &mut SpecialNetContext, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut i = 0;

        while i < parts.len() {
            // Stop parsing connections when we hit routing keywords
            if parts[i] == "ROUTED" || parts[i] == "FIXED" || parts[i] == "COVER" || parts[i] == "+"
            {
                break;
            }

            // Look for connection pattern: ( INSTANCE PIN )
            // Check that the next two tokens are NOT numbers (to avoid parsing coordinates)
            if i + 3 < parts.len() && parts[i] == "(" && parts[i + 3] == ")" {
                // Check if this looks like a coordinate (both parts are numbers)
                let is_coordinate =
                    parts[i + 1].parse::<f64>().is_ok() && parts[i + 2].parse::<f64>().is_ok();

                if !is_coordinate {
                    let instance = parts[i + 1].to_string();
                    let pin = parts[i + 2].to_string();
                    context.connections.push((instance, pin));
                }
                i += 4; // Skip past this pattern
            } else {
                i += 1;
            }
        }
    }

    /// Parse routing information from a line
    /// Special nets support: ROUTED, FIXED, COVER with optional SHAPE (STRIPE, FOLLOWPIN, etc.)
    fn parse_routing_in_line(&self, context: &mut SpecialNetContext, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut i = 0;
        let mut last_x = 0.0;
        let mut last_y = 0.0;

        while i < parts.len() {
            match parts[i] {
                "+" => {
                    // Skip + separators
                    i += 1;
                }
                "ROUTED" | "FIXED" | "COVER" => {
                    // Finalize previous route before starting new one
                    context.finalize_route();

                    if i + 1 < parts.len() {
                        let routing_type = parts[i].to_string();
                        let layer = parts[i + 1].to_string();

                        // Extract width (next number after layer)
                        let mut width = 0.0;
                        let mut skip_count = 2; // routing_type + layer
                        if i + 2 < parts.len() {
                            if let Ok(w) = parts[i + 2].parse::<f64>() {
                                width = w;
                                skip_count = 3; // routing_type + layer + width
                            }
                        }

                        // Check for SHAPE keyword (no need to extract shape separately)
                        let shape = None; // Will be set when we encounter SHAPE keyword

                        // Start new route
                        context.start_route(layer, width, routing_type, shape);

                        i += skip_count;
                    } else {
                        i += 1;
                    }
                }
                "NEW" => {
                    // Finalize previous route before starting new one
                    context.finalize_route();

                    if i + 1 < parts.len() {
                        let layer = parts[i + 1].to_string();

                        let mut width = 0.0;
                        let mut skip_count = 2; // NEW + layer
                        if i + 2 < parts.len() {
                            if let Ok(w) = parts[i + 2].parse::<f64>() {
                                width = w;
                                skip_count = 3; // NEW + layer + width
                            }
                        }

                        // NEW continues with ROUTED as default routing_type
                        context.start_route(layer, width, "ROUTED".to_string(), None);

                        i += skip_count;
                    } else {
                        i += 1;
                    }
                }
                "SHAPE" => {
                    // Set shape for current route
                    if i + 1 < parts.len() {
                        if let Some(route) = &mut context.current_route {
                            route.shape = Some(parts[i + 1].to_string());
                        }
                        i += 2; // Skip SHAPE + value
                    } else {
                        i += 1;
                    }
                }
                "(" => {
                    // Parse coordinate: ( x y ) or ( x * ) or ( * y )
                    if i + 3 < parts.len() && parts[i + 3] == ")" {
                        let x_str = parts[i + 1];
                        let y_str = parts[i + 2];

                        // Handle wildcard coordinates (inherit from previous point)
                        let x = if x_str == "*" {
                            last_x
                        } else {
                            x_str.parse().unwrap_or(0.0)
                        };
                        let y = if y_str == "*" {
                            last_y
                        } else {
                            y_str.parse().unwrap_or(0.0)
                        };

                        context.add_point(x, y);
                        last_x = x;
                        last_y = y;

                        i += 4; // Skip ( x y )
                    } else {
                        i += 1;
                    }
                }
                via_name if self.is_via_name(via_name) => {
                    // Via reference - might be followed by coordinates
                    if i + 1 < parts.len() && parts[i + 1] == "(" {
                        if i + 4 < parts.len() && parts[i + 4] == ")" {
                            let x = parts[i + 2].parse().unwrap_or(0.0);
                            let y = parts[i + 3].parse().unwrap_or(0.0);
                            context.add_via(via_name.to_string(), x, y);
                            last_x = x;
                            last_y = y;
                            i += 5; // Skip via ( x y )
                        } else {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                }
                ";" => {
                    // End of statement, finalize any pending route
                    context.finalize_route();
                    i += 1;
                }
                _ => i += 1,
            }
        }
    }

    /// Check if a token looks like a via name (heuristic: contains "via" or "Via")
    fn is_via_name(&self, token: &str) -> bool {
        token.to_lowercase().contains("via")
            || token.chars().next().is_some_and(|c| c.is_uppercase())
    }

    /// Parse attributes from any line (header or continuation)
    fn parse_line_attributes(&self, context: &mut SpecialNetContext, line: &str) {
        // Parse connections
        self.parse_connections_in_line(context, line);

        // Parse routing information
        self.parse_routing_in_line(context, line);

        // Extract USE value (POWER, GROUND, CLOCK, etc.)
        if let Some(use_type) = extract_keyword_value(line, "USE") {
            context.use_type = Some(use_type);
        }

        // Extract WEIGHT value
        if let Some(weight_str) = extract_keyword_value(line, "WEIGHT") {
            if let Ok(weight) = weight_str.parse::<f64>() {
                context.weight = Some(weight);
            }
        }

        // Extract VOLTAGE value
        if let Some(voltage_str) = extract_keyword_value(line, "VOLTAGE") {
            if let Ok(voltage) = voltage_str.parse::<f64>() {
                context.voltage = Some(voltage);
            }
        }

        // Extract ORIGINAL value
        if let Some(original) = extract_keyword_value(line, "ORIGINAL") {
            context.original_net = Some(original);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_specialnet_header() {
        let parser = DefSpecialNetParser::new();
        let mut context = parser.parse_header("- VDD ( * VDD )").unwrap();

        assert_eq!(context.name, "VDD");

        // Parse the full line to get connections (now done in parse_continuation)
        parser.parse_line_attributes(&mut context, "- VDD ( * VDD )");

        assert_eq!(context.connections.len(), 1);
        assert_eq!(context.connections[0].0, "*");
        assert_eq!(context.connections[0].1, "VDD");
    }

    #[test]
    fn test_parse_specialnet_with_routing() {
        let parser = DefSpecialNetParser::new();
        let line = "- VDD ( * VDD ) + ROUTED metal1 170 + SHAPE FOLLOWPIN ( 0 0 ) ( 22800 0 ) ;";
        let mut context = parser.parse_header("- VDD ( * VDD )").unwrap();

        parser.parse_line_attributes(&mut context, line);
        context.finalize_route();

        assert_eq!(context.routes.len(), 1);
        assert_eq!(context.routes[0].layer, "metal1");
        assert_eq!(context.routes[0].width, 170.0);
        assert_eq!(context.routes[0].shape, Some("FOLLOWPIN".to_string()));
    }

    #[test]
    fn test_parse_new_segment() {
        let parser = DefSpecialNetParser::new();
        let mut context = SpecialNetContext::new("VDD".to_string());

        parser.parse_routing_in_line(
            &mut context,
            "+ ROUTED metal1 170 ( 0 0 ) ( 100 0 ) NEW metal2 200 ( 100 0 ) ( 100 100 )",
        );

        assert_eq!(context.routes.len(), 1); // First route completed
        context.finalize_route(); // Finalize second route

        assert_eq!(context.routes.len(), 2);
        assert_eq!(context.routes[0].layer, "metal1");
        assert_eq!(context.routes[1].layer, "metal2");
    }

    #[test]
    fn test_parse_stripe_shape() {
        let parser = DefSpecialNetParser::new();
        let mut context = SpecialNetContext::new("VDD".to_string());

        parser.parse_routing_in_line(
            &mut context,
            "+ ROUTED metal8 1000 + SHAPE STRIPE ( 2300 0 ) ( 2300 21000 )",
        );
        context.finalize_route();

        assert_eq!(context.routes.len(), 1);
        assert_eq!(context.routes[0].shape, Some("STRIPE".to_string()));
        assert_eq!(context.routes[0].width, 1000.0);
    }

    #[test]
    fn test_parse_with_use_type() {
        let parser = DefSpecialNetParser::new();
        let mut context = SpecialNetContext::new("VDD".to_string());

        parser.parse_line_attributes(&mut context, "+ USE POWER");
        assert_eq!(context.use_type, Some("POWER".to_string()));
    }

    #[test]
    fn test_wildcard_coordinates() {
        let parser = DefSpecialNetParser::new();
        let mut context = SpecialNetContext::new("VDD".to_string());

        parser.parse_routing_in_line(
            &mut context,
            "+ ROUTED metal1 170 ( 0 0 ) ( 100 * ) ( * 200 )",
        );
        context.finalize_route();

        assert_eq!(context.routes[0].points.len(), 3);
        // Wildcard handling: currently uses 0.0, should be improved to inherit from previous point
    }
}
