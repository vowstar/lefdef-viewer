// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

//! VIA parser implementation for DEF files
//! TODO: Implement using the unified framework

use super::common::*;
use super::{ContinuationResult, DefItemParser, ParseResult};
use crate::def::{DefPolygon, DefRect, DefVia, DefViaLayer};

/// Context for parsing a single VIA
#[derive(Debug)]
#[allow(dead_code)]
pub struct ViaContext {
    pub name: String,
    pub via_rule: Option<String>,
    pub layers: Vec<DefViaLayer>,
    pub cut_size: Option<(f64, f64)>,
    pub cut_spacing: Option<(f64, f64)>,
    pub enclosure: Vec<(String, f64, f64)>, // layer, x_enclosure, y_enclosure
    pub properties: Vec<(String, String)>,
    pub pattern: String,
    pub current_layer: Option<String>,
    pub current_mask: Option<i32>,
}

#[allow(dead_code)]
impl ViaContext {
    pub fn new(name: String) -> Self {
        Self {
            name,
            via_rule: None,
            layers: Vec::new(),
            cut_size: None,
            cut_spacing: None,
            enclosure: Vec::new(),
            properties: Vec::new(),
            pattern: String::new(),
            current_layer: None,
            current_mask: None,
        }
    }

    pub fn find_or_create_layer(&mut self, layer_name: String) -> usize {
        if let Some(index) = self.layers.iter().position(|l| l.layer_name == layer_name) {
            index
        } else {
            self.layers.push(DefViaLayer {
                layer_name: layer_name.clone(),
                mask: self.current_mask,
                rects: Vec::new(),
                polygons: Vec::new(),
            });
            self.layers.len() - 1
        }
    }
}

/// VIA parser for DEF files
#[allow(dead_code)]
pub struct DefViaParser;

#[allow(dead_code)]
impl DefViaParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefViaParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DefItemParser for DefViaParser {
    type Item = DefVia;
    type Context = ViaContext;

    fn parse_header(&self, line: &str) -> Option<Self::Context> {
        let parts: Vec<&str> = line.split_whitespace().collect();

        // Check for VIA header: "- VIANAME ..."
        if parts.len() >= 2 && parts[0] == "-" {
            if let Some(via_name) = parse_identifier(parts[1]) {
                let mut context = ViaContext::new(via_name.to_string());

                // Parse any immediate attributes in header line
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

        if is_section_end(trimmed, "VIAS") {
            return ContinuationResult::NextItem;
        }

        // Parse continuation line attributes
        self.parse_line_attributes(context, trimmed);
        ContinuationResult::Continue
    }

    fn finalize(&self, context: Self::Context) -> ParseResult<Self::Item> {
        Ok(DefVia {
            name: context.name,
            layers: context.layers,
            via_rule: context.via_rule,
            cut_size: context.cut_size,
            cut_spacing: context.cut_spacing,
            enclosure: context.enclosure,
            pattern: context.pattern,
        })
    }

    fn item_name() -> &'static str {
        "VIA"
    }
}

#[allow(dead_code)]
impl DefViaParser {
    /// Parse attributes from the header line
    fn parse_header_attributes(&self, context: &mut ViaContext, parts: &[&str]) {
        for i in 2..parts.len() {
            match parts[i] {
                "VIARULE" if i + 1 < parts.len() => {
                    context.via_rule = Some(clean_semicolon(parts[i + 1]).to_string());
                }
                "CUTSIZE" if i + 2 < parts.len() => {
                    if let (Ok(w), Ok(h)) =
                        (parts[i + 1].parse::<f64>(), parts[i + 2].parse::<f64>())
                    {
                        context.cut_size = Some((w, h));
                    }
                }
                "PATTERN" if i + 1 < parts.len() => {
                    context.pattern = clean_semicolon(parts[i + 1]).to_string();
                }
                _ => {}
            }
        }
    }

    /// Parse attributes from any line (header or continuation)
    fn parse_line_attributes(&self, context: &mut ViaContext, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.is_empty() || parts[0] != "+" {
            return;
        }

        if parts.len() < 2 {
            return;
        }

        match parts[1] {
            "VIARULE" if parts.len() >= 3 => {
                context.via_rule = Some(clean_semicolon(parts[2]).to_string());
            }
            "CUTSIZE" if parts.len() >= 4 => {
                if let (Ok(w), Ok(h)) = (parts[2].parse::<f64>(), parts[3].parse::<f64>()) {
                    context.cut_size = Some((w, h));
                }
            }
            "CUTSPACING" if parts.len() >= 4 => {
                if let (Ok(x), Ok(y)) = (parts[2].parse::<f64>(), parts[3].parse::<f64>()) {
                    context.cut_spacing = Some((x, y));
                }
            }
            "ENCLOSURE" if parts.len() >= 5 => {
                let layer = parts[2].to_string();
                if let (Ok(x), Ok(y)) = (parts[3].parse::<f64>(), parts[4].parse::<f64>()) {
                    context.enclosure.push((layer, x, y));
                }
            }
            "PATTERN" if parts.len() >= 3 => {
                context.pattern = clean_semicolon(parts[2]).to_string();
            }
            "RECT" => {
                self.parse_rect(context, &parts);
            }
            "POLYGON" => {
                self.parse_polygon(context, line);
            }
            "LAYER" if parts.len() >= 3 => {
                context.current_layer = Some(parts[2].to_string());
                // Check for MASK after LAYER
                if parts.len() >= 5 && parts[3] == "MASK" {
                    if let Ok(mask) = parts[4].parse::<i32>() {
                        context.current_mask = Some(mask);
                    }
                }
            }
            "MASK" if parts.len() >= 3 => {
                if let Ok(mask) = parts[2].parse::<i32>() {
                    context.current_mask = Some(mask);
                }
            }
            _ => {}
        }
    }

    /// Parse RECT definition: + RECT layerName ( xl yl ) ( xh yh )
    fn parse_rect(&self, context: &mut ViaContext, parts: &[&str]) {
        // Check if we have enough parts for a RECT definition
        if parts.len() >= 10 && parts[0] == "+" && parts[1] == "RECT" {
            let layer_name = parts[2].to_string();

            // Find the coordinate pairs
            let mut xl = 0.0;
            let mut yl = 0.0;
            let mut xh = 0.0;
            let mut yh = 0.0;
            let mut found_coords = false;

            // Look for coordinate pairs
            for i in 3..parts.len() - 3 {
                if parts[i] == "("
                    && parts[i + 3] == ")"
                    && i + 7 < parts.len()
                    && parts[i + 4] == "("
                    && parts[i + 7] == ")"
                {
                    if let (Ok(x1), Ok(y1), Ok(x2), Ok(y2)) = (
                        parts[i + 1].parse::<f64>(),
                        parts[i + 2].parse::<f64>(),
                        parts[i + 5].parse::<f64>(),
                        parts[i + 6].parse::<f64>(),
                    ) {
                        xl = x1;
                        yl = y1;
                        xh = x2;
                        yh = y2;
                        found_coords = true;
                        break;
                    }
                }
            }

            if found_coords {
                let layer_index = context.find_or_create_layer(layer_name.clone());

                context.layers[layer_index].rects.push(DefRect {
                    layer: layer_name,
                    xl,
                    yl,
                    xh,
                    yh,
                });
            }
        }
    }

    /// Parse POLYGON definition which may span multiple lines
    fn parse_polygon(&self, context: &mut ViaContext, line: &str) {
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 3 || parts[0] != "+" || parts[1] != "POLYGON" {
            return;
        }

        let layer_name = parts[2].to_string();
        let mut part_idx = 3;
        let mut mask_num = None;

        // 修复可折叠的if语句
        if part_idx < parts.len() && parts[part_idx] == "MASK" && part_idx + 1 < parts.len() {
            if let Ok(mask) = parts[part_idx + 1].parse::<i32>() {
                mask_num = Some(mask);
                part_idx += 2;
            }
        }

        // Parse coordinate pairs
        let mut points = Vec::new();
        while part_idx + 3 < parts.len() {
            if parts[part_idx] == "(" && parts[part_idx + 3] == ")" {
                if let (Ok(x), Ok(y)) = (
                    parts[part_idx + 1].parse::<f64>(),
                    parts[part_idx + 2].parse::<f64>(),
                ) {
                    points.push((x, y));
                    part_idx += 4; // Skip past ( x y )
                } else {
                    part_idx += 1;
                }
            } else {
                part_idx += 1;
            }
        }

        if !points.is_empty() {
            let layer_index = context.find_or_create_layer(layer_name);

            if let Some(mask) = mask_num {
                context.layers[layer_index].mask = Some(mask);
            }

            context.layers[layer_index]
                .polygons
                .push(DefPolygon { points });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_via_header() {
        let parser = DefViaParser::new();
        let context = parser.parse_header("- VIA1").unwrap();

        assert_eq!(context.name, "VIA1");
    }

    #[test]
    fn test_parse_via_with_viarule() {
        let parser = DefViaParser::new();
        let context = parser.parse_header("- VIA1 + VIARULE RULE1").unwrap();

        assert_eq!(context.name, "VIA1");
        assert_eq!(context.via_rule, Some("RULE1".to_string()));
    }

    #[test]
    fn test_parse_via_cutsize() {
        let parser = DefViaParser::new();
        let mut context = ViaContext::new("VIA1".to_string());

        parser.parse_line_attributes(&mut context, "+ CUTSIZE 100 200");
        assert_eq!(context.cut_size, Some((100.0, 200.0)));
    }

    #[test]
    fn test_parse_via_rect() {
        let parser = DefViaParser::new();
        let mut context = ViaContext::new("VIA1".to_string());

        parser.parse_line_attributes(&mut context, "+ RECT METAL1 ( 0 0 ) ( 100 100 )");
        assert_eq!(context.layers.len(), 1);
        assert_eq!(context.layers[0].layer_name, "METAL1");
        assert_eq!(context.layers[0].rects.len(), 1);

        let rect = &context.layers[0].rects[0];
        assert_eq!(rect.xl, 0.0);
        assert_eq!(rect.yl, 0.0);
        assert_eq!(rect.xh, 100.0);
        assert_eq!(rect.yh, 100.0);
    }

    #[test]
    fn test_parse_via_polygon() {
        let parser = DefViaParser::new();
        let mut context = ViaContext::new("VIA1".to_string());

        parser.parse_line_attributes(
            &mut context,
            "+ POLYGON METAL1 ( 0 0 ) ( 100 0 ) ( 100 100 ) ( 0 100 )",
        );
        assert_eq!(context.layers.len(), 1);
        assert_eq!(context.layers[0].layer_name, "METAL1");
        assert_eq!(context.layers[0].polygons.len(), 1);

        let polygon = &context.layers[0].polygons[0];
        assert_eq!(polygon.points.len(), 4);
        assert_eq!(polygon.points[0], (0.0, 0.0));
        assert_eq!(polygon.points[3], (0.0, 100.0));
    }

    #[test]
    fn test_parse_via_polygon_with_mask() {
        let parser = DefViaParser::new();
        let mut context = ViaContext::new("VIA1".to_string());

        parser.parse_line_attributes(&mut context, "+ POLYGON METAL1 MASK 2 ( 0 0 ) ( 100 100 )");
        assert_eq!(context.layers.len(), 1);
        assert_eq!(context.layers[0].mask, Some(2));
        assert_eq!(context.layers[0].polygons.len(), 1);
    }

    #[test]
    fn test_parse_via_enclosure() {
        let parser = DefViaParser::new();
        let mut context = ViaContext::new("VIA1".to_string());

        parser.parse_line_attributes(&mut context, "+ ENCLOSURE METAL1 10 20");
        assert_eq!(context.enclosure.len(), 1);
        assert_eq!(context.enclosure[0], ("METAL1".to_string(), 10.0, 20.0));
    }

    #[test]
    fn test_next_item_detection() {
        let parser = DefViaParser::new();
        let mut context = ViaContext::new("VIA1".to_string());

        let result = parser.parse_continuation(&mut context, "- VIA2");
        assert_eq!(result, ContinuationResult::NextItem);
    }

    #[test]
    fn test_section_end_detection() {
        let parser = DefViaParser::new();
        let mut context = ViaContext::new("VIA1".to_string());

        let result = parser.parse_continuation(&mut context, "END VIAS");
        assert_eq!(result, ContinuationResult::NextItem);
    }
}
