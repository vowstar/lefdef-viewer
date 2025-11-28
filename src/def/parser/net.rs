// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

//! NET parser implementation for DEF files

use super::common::*;
use super::{ContinuationResult, DefItemParser, ParseResult};
use crate::def::DefNet;

/// Connection in a net
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NetConnection {
    pub instance: String,
    pub pin: String,
    pub is_synthesized: bool,
}

/// Routing information for a net
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct NetRouting {
    pub layer: String,
    pub points: Vec<(f64, f64)>,
    pub width: Option<f64>,
    pub via: Option<String>,
    pub shape: String, // NEW, FIXED, COVER, ROUTED, SHIELD, NOSHIELD
}

/// Context for parsing a single NET
#[derive(Debug)]
pub struct NetContext {
    pub name: String,
    pub connections: Vec<NetConnection>,
    pub use_type: String,
    pub weight: Option<f64>,
    pub routing: Vec<NetRouting>,
    pub properties: Vec<(String, String)>,
    pub shielded: bool,
    pub source: String,
    pub pattern: String,
}

impl NetContext {
    pub fn new(name: String) -> Self {
        Self {
            name,
            connections: Vec::new(),
            use_type: String::new(),
            weight: None,
            routing: Vec::new(),
            properties: Vec::new(),
            shielded: false,
            source: String::new(),
            pattern: String::new(),
        }
    }
}

/// NET parser for DEF files
pub struct DefNetParser;

impl DefNetParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefNetParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DefItemParser for DefNetParser {
    type Item = DefNet;
    type Context = NetContext;

    fn parse_header(&self, line: &str) -> Option<Self::Context> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Check for NET header: "- NETNAME ..."
        if parts.len() >= 2 && parts[0] == "-" {
            if let Some(net_name) = parse_identifier(parts[1]) {
                // Only create context with name, don't parse attributes here
                // All attributes will be parsed in parse_continuation
                let context = NetContext::new(net_name.to_string());
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
            self.parse_line_attributes(context, trimmed);
            return ContinuationResult::Complete;
        }

        if is_item_header(trimmed) {
            return ContinuationResult::NextItem;
        }

        if is_section_end(trimmed, "NETS") {
            return ContinuationResult::NextItem;
        }

        // Parse continuation line attributes
        self.parse_line_attributes(context, trimmed);
        ContinuationResult::Continue
    }

    fn finalize(&self, context: Self::Context) -> ParseResult<Self::Item> {
        // Convert NetRouting to DefRoute
        let routes = context
            .routing
            .iter()
            .map(|r| crate::def::DefRoute {
                layer: r.layer.clone(),
                width: r.width.unwrap_or(0.0),
                routing_type: r.shape.clone(),
                shape: None, // TODO: parse STRIPE, FOLLOWPIN, etc.
                points: r
                    .points
                    .iter()
                    .map(|(x, y)| crate::def::DefRoutingPoint {
                        x: *x,
                        y: *y,
                        ext: None,
                    })
                    .collect(),
                vias: Vec::new(), // TODO: parse vias from routing
                mask: None,
                style: None,
            })
            .collect();

        Ok(DefNet {
            name: context.name,
            connections: context.connections.len(),
            pins: 0, // Will be calculated later
            use_type: context.use_type,
            weight: context.weight,
            source: context.source,
            pattern: context.pattern,
            shielded: context.shielded,
            instances: context
                .connections
                .iter()
                .map(|c| c.instance.clone())
                .collect(),
            instance_pins: context.connections.iter().map(|c| c.pin.clone()).collect(),
            routes,
        })
    }

    fn item_name() -> &'static str {
        "NET"
    }
}

impl DefNetParser {
    /// Parse connections from any line that might contain them
    /// Only parse connections before routing keywords (ROUTED, NEW, etc.)
    fn parse_connections_in_line(&self, context: &mut NetContext, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut i = 0;

        while i < parts.len() {
            // Stop parsing connections when we hit routing keywords or USE
            if parts[i] == "ROUTED"
                || parts[i] == "NEW"
                || parts[i] == "FIXED"
                || parts[i] == "COVER"
                || parts[i] == "USE"
                || parts[i] == "+"
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

                    context.connections.push(NetConnection {
                        instance,
                        pin,
                        is_synthesized: false, // TODO: detect synthesized connections
                    });
                }
                i += 4; // Skip past this pattern
            } else {
                i += 1;
            }
        }
    }

    /// Parse routing information from a line
    /// Format: ROUTED layer ( x y ) ( x y ) vianame
    ///         NEW layer ( x y ) ( x y )
    /// Supports wildcard coordinates: ( * y ) or ( x * )
    fn parse_routing_in_line(&self, context: &mut NetContext, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut i = 0;
        let mut last_x = 0.0;
        let mut last_y = 0.0;

        while i < parts.len() {
            match parts[i] {
                "ROUTED" | "NEW" | "FIXED" | "COVER" => {
                    if i + 1 < parts.len() {
                        let routing_type = parts[i].to_string();
                        let layer = parts[i + 1].to_string();

                        // NETS typically don't have explicit width, use default
                        let width = None; // Will default to minimal width in renderer

                        // Collect points and vias for this route segment
                        let mut points = Vec::new();
                        let mut via = None;

                        i += 2; // Skip routing_type and layer

                        // Parse coordinates and vias until we hit next routing keyword
                        while i < parts.len() {
                            match parts[i] {
                                "NEW" | "ROUTED" | "FIXED" | "COVER" | "+" => {
                                    // Stop at next routing keyword
                                    break;
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

                                        points.push((x, y));
                                        last_x = x;
                                        last_y = y;

                                        i += 4; // Skip ( x y )
                                    } else {
                                        i += 1;
                                    }
                                }
                                _ => {
                                    // Check if this is a via name (single word, not a number)
                                    if !parts[i].contains('(')
                                        && !parts[i].contains(')')
                                        && parts[i].parse::<f64>().is_err()
                                        && via.is_none()
                                    {
                                        // This looks like a via name
                                        via = Some(parts[i].to_string());
                                    }
                                    i += 1;
                                }
                            }
                        }

                        // Only create route if we have points
                        if !points.is_empty() {
                            context.routing.push(NetRouting {
                                layer,
                                points,
                                width,
                                via,
                                shape: routing_type,
                            });
                        }
                    } else {
                        i += 1;
                    }
                }
                _ => i += 1,
            }
        }
    }

    /// Parse attributes from any line (header or continuation)
    fn parse_line_attributes(&self, context: &mut NetContext, line: &str) {
        // Parse connections
        self.parse_connections_in_line(context, line);

        // Parse routing information
        self.parse_routing_in_line(context, line);

        // Extract USE value
        if let Some(use_type) = extract_keyword_value(line, "USE") {
            context.use_type = use_type;
        }

        // Extract WEIGHT value
        if let Some(weight_str) = extract_keyword_value(line, "WEIGHT") {
            if let Ok(weight) = weight_str.parse::<f64>() {
                context.weight = Some(weight);
            }
        }

        // Extract SOURCE value
        if let Some(source) = extract_keyword_value(line, "SOURCE") {
            context.source = source;
        }

        // Extract PATTERN value
        if let Some(pattern) = extract_keyword_value(line, "PATTERN") {
            context.pattern = pattern;
        }

        // Check for SHIELDED
        if contains_keyword(line, "SHIELDED") {
            context.shielded = true;
        }

        // Extract PROPERTY values
        if let Some(prop_name) = extract_keyword_value(line, "PROPERTY") {
            // TODO: Handle property values which might be on next line
            context.properties.push((prop_name, String::new()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_net_header() {
        let parser = DefNetParser::new();
        let mut context = parser.parse_header("- N1 ( I1 A ) ( I2 B )").unwrap();

        assert_eq!(context.name, "N1");

        // Parse the full line to get connections (now done in parse_continuation)
        parser.parse_line_attributes(&mut context, "- N1 ( I1 A ) ( I2 B )");

        assert_eq!(context.connections.len(), 2);
        assert_eq!(context.connections[0].instance, "I1");
        assert_eq!(context.connections[0].pin, "A");
        assert_eq!(context.connections[1].instance, "I2");
        assert_eq!(context.connections[1].pin, "B");
    }

    #[test]
    fn test_parse_net_with_use() {
        let parser = DefNetParser::new();
        let mut context = NetContext::new("testnet".to_string());

        let result = parser.parse_continuation(&mut context, "+ USE SIGNAL ;");
        assert_eq!(result, ContinuationResult::Complete);
        assert_eq!(context.use_type, "SIGNAL");
    }

    #[test]
    fn test_parse_net_connections() {
        let parser = DefNetParser::new();
        let mut context = NetContext::new("testnet".to_string());

        parser.parse_connections_in_line(&mut context, "( INST1 PIN1 ) ( INST2 PIN2 )");
        assert_eq!(context.connections.len(), 2);
        assert_eq!(context.connections[0].instance, "INST1");
        assert_eq!(context.connections[1].pin, "PIN2");
    }

    #[test]
    fn test_parse_routing_with_wildcard_coordinates() {
        let parser = DefNetParser::new();
        let mut context = NetContext::new("testnet".to_string());

        // Test wildcard coordinates: ( * y ) and ( x * )
        parser.parse_routing_in_line(
            &mut context,
            "+ ROUTED metal2 ( 1000 2000 ) ( * 3000 ) ( 1500 * )",
        );

        assert_eq!(context.routing.len(), 1);
        assert_eq!(context.routing[0].layer, "metal2");
        assert_eq!(context.routing[0].points.len(), 3);
        // First point: explicit coordinates
        assert_eq!(context.routing[0].points[0], (1000.0, 2000.0));
        // Second point: wildcard x inherits from previous (1000), y is explicit (3000)
        assert_eq!(context.routing[0].points[1], (1000.0, 3000.0));
        // Third point: x is explicit (1500), wildcard y inherits from previous (3000)
        assert_eq!(context.routing[0].points[2], (1500.0, 3000.0));
    }

    #[test]
    fn test_net_weight_parsing() {
        let parser = DefNetParser::new();
        let mut context = NetContext::new("testnet".to_string());

        parser.parse_line_attributes(&mut context, "+ WEIGHT 5");
        assert_eq!(context.weight, Some(5.0));
    }

    #[test]
    fn test_next_item_detection() {
        let parser = DefNetParser::new();
        let mut context = NetContext::new("N1".to_string());

        let result = parser.parse_continuation(&mut context, "- N2 ( I3 C )");
        assert_eq!(result, ContinuationResult::NextItem);
    }

    #[test]
    fn test_section_end_detection() {
        let parser = DefNetParser::new();
        let mut context = NetContext::new("N1".to_string());

        let result = parser.parse_continuation(&mut context, "END NETS");
        assert_eq!(result, ContinuationResult::NextItem);
    }
}
