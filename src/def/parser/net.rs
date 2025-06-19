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
                let mut context = NetContext::new(net_name.to_string());

                // Parse any immediate connections in header: ( INSTANCE PIN )
                self.parse_connections_in_line(&mut context, line);

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
            routing: context.routing.len(),
        })
    }

    fn item_name() -> &'static str {
        "NET"
    }
}

impl DefNetParser {
    /// Parse connections from any line that might contain them
    fn parse_connections_in_line(&self, context: &mut NetContext, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut i = 0;

        while i < parts.len() {
            // Look for connection pattern: ( INSTANCE PIN )
            if i + 3 < parts.len() && parts[i] == "(" && parts[i + 3] == ")" {
                let instance = parts[i + 1].to_string();
                let pin = parts[i + 2].to_string();

                context.connections.push(NetConnection {
                    instance,
                    pin,
                    is_synthesized: false, // TODO: detect synthesized connections
                });
                i += 4; // Skip past this connection
            } else {
                i += 1;
            }
        }
    }

    /// Parse routing information from a line
    fn parse_routing_in_line(&self, context: &mut NetContext, line: &str) {
        // Handle different routing types: NEW, ROUTED, FIXED, COVER, SHIELD
        if let Some(shape) = self.extract_routing_shape(line) {
            if let Some(layer) = extract_keyword_value(line, "LAYER") {
                let points = self.extract_routing_points(line);
                let width = self.extract_routing_width(line);
                let via = extract_keyword_value(line, "VIA");

                context.routing.push(NetRouting {
                    layer,
                    points,
                    width,
                    via,
                    shape,
                });
            }
        }
    }

    /// Extract routing shape (NEW, ROUTED, etc.)
    fn extract_routing_shape(&self, line: &str) -> Option<String> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        for part in parts {
            match part {
                "NEW" | "ROUTED" | "FIXED" | "COVER" | "SHIELD" | "NOSHIELD" => {
                    return Some(part.to_string());
                }
                _ => continue,
            }
        }
        None
    }

    /// Extract coordinate points from routing line
    fn extract_routing_points(&self, line: &str) -> Vec<(f64, f64)> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let mut points = Vec::new();
        let mut i = 0;

        while i < parts.len() {
            if parts[i] == "(" && i + 3 < parts.len() && parts[i + 3] == ")" {
                if let (Ok(x), Ok(y)) = (parts[i + 1].parse::<f64>(), parts[i + 2].parse::<f64>()) {
                    points.push((x, y));
                    i += 4; // Skip past coordinate pair
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        points
    }

    /// Extract routing width
    fn extract_routing_width(&self, line: &str) -> Option<f64> {
        extract_keyword_value(line, "WIDTH").and_then(|w| w.parse().ok())
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
        let context = parser.parse_header("- N1 ( I1 A ) ( I2 B )").unwrap();

        assert_eq!(context.name, "N1");
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
    fn test_parse_routing_points() {
        let parser = DefNetParser::new();
        let points = parser.extract_routing_points("+ ROUTED METAL1 ( 100 200 ) ( 300 400 )");

        assert_eq!(points.len(), 2);
        assert_eq!(points[0], (100.0, 200.0));
        assert_eq!(points[1], (300.0, 400.0));
    }

    #[test]
    fn test_routing_shape_detection() {
        let parser = DefNetParser::new();

        assert_eq!(
            parser.extract_routing_shape("+ NEW METAL1"),
            Some("NEW".to_string())
        );
        assert_eq!(
            parser.extract_routing_shape("+ ROUTED METAL2"),
            Some("ROUTED".to_string())
        );
        assert_eq!(
            parser.extract_routing_shape("+ FIXED"),
            Some("FIXED".to_string())
        );
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
