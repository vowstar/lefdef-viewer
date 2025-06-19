// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

//! PIN parser implementation for DEF files

use super::common::*;
use super::{ContinuationResult, DefItemParser, ParseResult};
use crate::def::DefPin;

/// Context for parsing a single PIN
#[derive(Debug)]
pub struct PinContext {
    pub name: String,
    pub net: String,
    pub direction: String,
    pub use_type: String,
    pub x: f64,
    pub y: f64,
    pub orient: String,
    pub status: String,
}

impl PinContext {
    pub fn new(name: String) -> Self {
        Self {
            name,
            net: String::new(),
            direction: String::new(),
            use_type: String::new(),
            x: 0.0,
            y: 0.0,
            orient: String::new(),
            status: "PLACED".to_string(),
        }
    }
}

/// PIN parser for DEF files
pub struct DefPinParser;

impl DefPinParser {
    pub fn new() -> Self {
        Self
    }
}

impl DefItemParser for DefPinParser {
    type Item = DefPin;
    type Context = PinContext;

    fn parse_header(&self, line: &str) -> Option<Self::Context> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Check for PIN header: "- PINNAME + ..."
        if parts.len() >= 2 && parts[0] == "-" {
            if let Some(pin_name) = parse_identifier(parts[1]) {
                let mut context = PinContext::new(pin_name.to_string());

                // Parse header line for immediate attributes
                self.parse_header_attributes(&mut context, &parts);

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

        if is_section_end(trimmed, "PINS") {
            return ContinuationResult::NextItem;
        }

        // Parse continuation line attributes
        self.parse_line_attributes(context, trimmed);
        ContinuationResult::Continue
    }

    fn finalize(&self, context: Self::Context) -> ParseResult<Self::Item> {
        Ok(DefPin {
            name: context.name,
            net: context.net,
            use_type: context.use_type,
            status: context.status,
            direction: context.direction,
            orient: context.orient,
            x: context.x,
            y: context.y,
            rects: Vec::new(),
            ports: Vec::new(),
        })
    }

    fn item_name() -> &'static str {
        "PIN"
    }
}

impl DefPinParser {
    /// Parse attributes from the header line
    fn parse_header_attributes(&self, context: &mut PinContext, parts: &[&str]) {
        for i in 2..parts.len() {
            match parts[i] {
                "NET" if i + 1 < parts.len() => {
                    context.net = clean_semicolon(parts[i + 1]).to_string();
                }
                "DIRECTION" if i + 1 < parts.len() => {
                    context.direction = clean_semicolon(parts[i + 1]).to_string();
                }
                "USE" if i + 1 < parts.len() => {
                    context.use_type = clean_semicolon(parts[i + 1]).to_string();
                }
                "PLACED" | "FIXED" => {
                    if let Some((status, x, y, orient)) = parse_placement(&parts[i..].join(" ")) {
                        context.status = status;
                        context.x = x;
                        context.y = y;
                        context.orient = orient;
                    }
                }
                _ => {}
            }
        }
    }

    /// Parse attributes from any line (header or continuation)
    fn parse_line_attributes(&self, context: &mut PinContext, line: &str) {
        // Extract NET value
        if let Some(net) = extract_keyword_value(line, "NET") {
            context.net = net;
        }

        // Extract DIRECTION value
        if let Some(direction) = extract_keyword_value(line, "DIRECTION") {
            context.direction = direction;
        }

        // Extract USE value
        if let Some(use_type) = extract_keyword_value(line, "USE") {
            context.use_type = use_type;
        }

        // Extract placement information
        if let Some((status, x, y, orient)) = parse_placement(line) {
            context.status = status;
            context.x = x;
            context.y = y;
            context.orient = orient;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_pin_header() {
        let parser = DefPinParser::new();
        let context = parser.parse_header("- OUTBUS<1> + NET OUTBUS<1>").unwrap();

        assert_eq!(context.name, "OUTBUS<1>");
        assert_eq!(context.net, "OUTBUS<1>");
    }

    #[test]
    fn test_parse_complete_pin_header() {
        let parser = DefPinParser::new();
        let context = parser
            .parse_header("- P0 + NET N0 + DIRECTION INPUT + USE SIGNAL")
            .unwrap();

        assert_eq!(context.name, "P0");
        assert_eq!(context.net, "N0");
        assert_eq!(context.direction, "INPUT");
        assert_eq!(context.use_type, "SIGNAL");
    }

    #[test]
    fn test_parse_pin_with_semicolon() {
        let parser = DefPinParser::new();
        let mut context = PinContext::new("OUTBUS<1>".to_string());

        let result = parser.parse_continuation(&mut context, "- OUTBUS<1> + NET OUTBUS<1> ;");
        assert_eq!(result, ContinuationResult::Complete);
    }

    #[test]
    fn test_parse_placement_continuation() {
        let parser = DefPinParser::new();
        let mut context = PinContext::new("P0".to_string());

        let result = parser.parse_continuation(&mut context, "+ PLACED ( 45 -2160 ) N ;");
        assert_eq!(result, ContinuationResult::Complete);
        assert_eq!(context.status, "PLACED");
        assert_eq!(context.x, 45.0);
        assert_eq!(context.y, -2160.0);
        assert_eq!(context.orient, "N");
    }

    #[test]
    fn test_next_item_detection() {
        let parser = DefPinParser::new();
        let mut context = PinContext::new("P0".to_string());

        let result = parser.parse_continuation(&mut context, "- P1 + NET N1");
        assert_eq!(result, ContinuationResult::NextItem);
    }

    #[test]
    fn test_section_end_detection() {
        let parser = DefPinParser::new();
        let mut context = PinContext::new("P0".to_string());

        let result = parser.parse_continuation(&mut context, "END PINS");
        assert_eq!(result, ContinuationResult::NextItem);
    }
}
