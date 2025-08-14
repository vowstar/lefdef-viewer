// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

//! COMPONENT parser implementation for DEF files
//! TODO: Implement using the unified framework

use super::{ContinuationResult, DefItemParser, ParseResult};
use crate::def::{DefComponent, DefComponentPlacement};

/// Context for parsing a single component
#[derive(Debug, Clone)]
pub struct ComponentContext {
    pub name: String,
    pub macro_name: String,
    pub placement: Option<DefComponentPlacement>,
    pub routing_halo: Option<(f64, f64, f64, f64)>,
    pub source: Option<String>,
    pub weight: Option<f64>,
    pub eeq: Option<String>,
    pub generate: Option<String>,
    pub power: Option<f64>,
    pub ground: Option<String>,
    pub properties: Vec<(String, String)>,
    pub completed: bool,
}

impl ComponentContext {
    pub fn new(name: String, macro_name: String) -> Self {
        Self {
            name,
            macro_name,
            placement: None,
            routing_halo: None,
            source: None,
            weight: None,
            eeq: None,
            generate: None,
            power: None,
            ground: None,
            properties: Vec::new(),
            completed: false,
        }
    }
}

/// Parser for DEF COMPONENTS section
pub struct DefComponentParser;

impl DefItemParser for DefComponentParser {
    type Item = DefComponent;
    type Context = ComponentContext;

    fn parse_header(&self, line: &str) -> Option<Self::Context> {
        let trimmed = line.trim();

        // Component header: "- COMP_NAME MACRO_NAME"
        if !trimmed.starts_with("- ") {
            return None;
        }

        let parts: Vec<&str> = trimmed[2..].split_whitespace().collect();
        if parts.len() < 2 {
            return None;
        }

        let component_name = parts[0].to_string();
        let macro_name = parts[1].to_string();

        let mut context = ComponentContext::new(component_name, macro_name);

        // If this is a complete line definition, parse all attributes immediately
        if trimmed.contains(';') {
            self.parse_component_line(&mut context, trimmed);
        }

        Some(context)
    }

    fn parse_continuation(&self, context: &mut Self::Context, line: &str) -> ContinuationResult {
        let trimmed = line.trim();

        // Check for completion
        if trimmed.ends_with(';') {
            self.parse_component_line(context, trimmed);
            context.completed = true;
            return ContinuationResult::Complete;
        }

        // Check for next item start
        if trimmed.starts_with("- ") {
            return ContinuationResult::NextItem;
        }

        // Parse continuation line
        self.parse_component_line(context, trimmed);
        ContinuationResult::Continue
    }

    fn finalize(&self, context: Self::Context) -> ParseResult<Self::Item> {
        Ok(DefComponent {
            name: context.name,
            macro_name: context.macro_name,
            placement: context.placement,
            routing_halo: context.routing_halo,
            source: context.source,
            weight: context.weight,
            eeq: context.eeq,
            generate: context.generate,
            power: context.power,
            ground: context.ground,
            properties: context.properties,
        })
    }

    fn item_name() -> &'static str {
        "COMPONENT"
    }
}

impl DefComponentParser {
    fn parse_component_line(&self, context: &mut ComponentContext, line: &str) {
        let trimmed = line.trim_end_matches(';').trim();
        let mut parts = trimmed.split_whitespace().peekable();

        while let Some(part) = parts.next() {
            match part {
                "+" => {
                    // Continuation marker, skip
                }
                "PLACED" | "FIXED" | "COVER" | "UNPLACED" => {
                    if let Some(placement) = self.parse_placement(&mut parts, part) {
                        context.placement = Some(placement);
                    }
                }
                "HALO" => {
                    if let Some(halo) = self.parse_halo(&mut parts) {
                        context.routing_halo = Some(halo);
                    }
                }
                "ROUTINGHALO" => {
                    if let Some(halo) = self.parse_halo(&mut parts) {
                        context.routing_halo = Some(halo);
                    }
                }
                "SOURCE" => {
                    if let Some(source) = parts.next() {
                        context.source = Some(source.to_string());
                    }
                }
                "WEIGHT" => {
                    if let Some(weight_str) = parts.next() {
                        if let Ok(weight) = weight_str.parse::<f64>() {
                            context.weight = Some(weight);
                        }
                    }
                }
                "EEQ" => {
                    if let Some(eeq) = parts.next() {
                        context.eeq = Some(eeq.to_string());
                    }
                }
                "GENERATE" => {
                    if let Some(generate) = parts.next() {
                        context.generate = Some(generate.to_string());
                    }
                }
                "POWER" => {
                    if let Some(power_str) = parts.next() {
                        if let Ok(power) = power_str.parse::<f64>() {
                            context.power = Some(power);
                        }
                    }
                }
                "GROUND" => {
                    if let Some(ground) = parts.next() {
                        context.ground = Some(ground.to_string());
                    }
                }
                "PROPERTY" => {
                    if let Some(prop_name) = parts.next() {
                        if let Some(prop_value) = parts.next() {
                            context
                                .properties
                                .push((prop_name.to_string(), prop_value.to_string()));
                        }
                    }
                }
                _ => {
                    // Skip unknown keywords
                }
            }
        }
    }

    fn parse_placement(
        &self,
        parts: &mut std::iter::Peekable<std::str::SplitWhitespace>,
        placement_type: &str,
    ) -> Option<DefComponentPlacement> {
        // PLACED ( x y ) orientation
        if parts.peek() == Some(&"(") {
            parts.next(); // consume '('
            if let Some(x_str) = parts.next() {
                if let Some(y_str) = parts.next() {
                    if parts.next() == Some(")") {
                        // consume ')'
                        if let (Ok(x), Ok(y)) = (x_str.parse::<f64>(), y_str.parse::<f64>()) {
                            let orientation = parts.next().unwrap_or("N").to_string();
                            return Some(DefComponentPlacement {
                                placement_type: placement_type.to_string(),
                                x,
                                y,
                                orientation,
                            });
                        }
                    }
                }
            }
        }
        None
    }

    fn parse_halo(
        &self,
        parts: &mut std::iter::Peekable<std::str::SplitWhitespace>,
    ) -> Option<(f64, f64, f64, f64)> {
        // HALO left bottom right top
        if let (Some(left_str), Some(bottom_str), Some(right_str), Some(top_str)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        {
            if let (Ok(left), Ok(bottom), Ok(right), Ok(top)) = (
                left_str.parse::<f64>(),
                bottom_str.parse::<f64>(),
                right_str.parse::<f64>(),
                top_str.parse::<f64>(),
            ) {
                return Some((left, bottom, right, top));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_component() {
        let parser = DefComponentParser;
        let context = parser.parse_header("- COMP1 MACRO1").unwrap();
        assert_eq!(context.name, "COMP1");
        assert_eq!(context.macro_name, "MACRO1");
    }

    #[test]
    fn test_parse_component_with_placement() {
        let parser = DefComponentParser;
        let mut context = ComponentContext::new("COMP1".to_string(), "MACRO1".to_string());

        let result = parser.parse_continuation(&mut context, "+ PLACED ( 100 200 ) N ;");
        assert_eq!(result, ContinuationResult::Complete);

        let placement = context.placement.unwrap();
        assert_eq!(placement.placement_type, "PLACED");
        assert_eq!(placement.x, 100.0);
        assert_eq!(placement.y, 200.0);
        assert_eq!(placement.orientation, "N");
    }

    #[test]
    fn test_parse_component_with_source() {
        let parser = DefComponentParser;
        let mut context = ComponentContext::new("COMP1".to_string(), "MACRO1".to_string());

        parser.parse_continuation(&mut context, "+ SOURCE USER ;");
        assert_eq!(context.source, Some("USER".to_string()));
    }

    #[test]
    fn test_parse_component_with_weight() {
        let parser = DefComponentParser;
        let mut context = ComponentContext::new("COMP1".to_string(), "MACRO1".to_string());

        parser.parse_continuation(&mut context, "+ WEIGHT 1.5 ;");
        assert_eq!(context.weight, Some(1.5));
    }

    #[test]
    fn test_parse_component_with_halo() {
        let parser = DefComponentParser;
        let mut context = ComponentContext::new("COMP1".to_string(), "MACRO1".to_string());

        parser.parse_continuation(&mut context, "+ HALO 10 20 30 40 ;");
        assert_eq!(context.routing_halo, Some((10.0, 20.0, 30.0, 40.0)));
    }

    #[test]
    fn test_parse_component_multiline() {
        let parser = DefComponentParser;
        let mut context = ComponentContext::new("COMP1".to_string(), "MACRO1".to_string());

        assert_eq!(
            parser.parse_continuation(&mut context, "+ PLACED ( 100 200 ) N"),
            ContinuationResult::Continue
        );
        assert_eq!(
            parser.parse_continuation(&mut context, "+ SOURCE USER"),
            ContinuationResult::Continue
        );
        assert_eq!(
            parser.parse_continuation(&mut context, "+ WEIGHT 1.5 ;"),
            ContinuationResult::Complete
        );

        assert!(context.placement.is_some());
        assert_eq!(context.source, Some("USER".to_string()));
        assert_eq!(context.weight, Some(1.5));
    }

    #[test]
    fn test_next_item_detection() {
        let parser = DefComponentParser;
        let mut context = ComponentContext::new("COMP1".to_string(), "MACRO1".to_string());

        let result = parser.parse_continuation(&mut context, "- COMP2 MACRO2");
        assert_eq!(result, ContinuationResult::NextItem);
    }

    #[test]
    fn test_single_line_component_parsing() {
        let parser = DefComponentParser;

        // Real world example from gcd.def
        let test_line =
            "- PHY_0 sky130_fd_sc_hd__decap_3 + SOURCE DIST + FIXED ( 10120 10880 ) N ;";

        // Parse header - after our fix, this should now include placement info
        let context = parser.parse_header(test_line).unwrap();
        assert_eq!(context.name, "PHY_0");
        assert_eq!(context.macro_name, "sky130_fd_sc_hd__decap_3");

        // After our fix, placement should be parsed in parse_header for complete lines
        if let Some(ref placement) = context.placement {
            assert_eq!(placement.placement_type, "FIXED");
            assert_eq!(placement.x, 10120.0);
            assert_eq!(placement.y, 10880.0);
            assert_eq!(placement.orientation, "N");
            println!(
                "[PASS] Placement parsed correctly in header: {} ({}, {}) {}",
                placement.placement_type, placement.x, placement.y, placement.orientation
            );
        } else {
            panic!("Placement should be parsed in parse_header for complete lines after our fix!");
        }
    }

    #[test]
    fn test_single_line_component_full_flow() {
        use super::super::MultiLineParser;

        let parser = MultiLineParser::new(DefComponentParser);
        let lines = vec![
            "COMPONENTS 2 ;",
            "    - PHY_0 sky130_fd_sc_hd__decap_3 + SOURCE DIST + FIXED ( 10120 10880 ) N ;",
            "    - PHY_1 sky130_fd_sc_hd__decap_3 + SOURCE DIST + PLACED ( 20240 30360 ) S ;",
            "END COMPONENTS",
        ];

        let line_refs: Vec<&str> = lines.iter().map(|s| s.as_ref()).collect();
        let (components, _) = parser
            .parse_section(&line_refs, 1, "END COMPONENTS")
            .unwrap();

        assert_eq!(components.len(), 2);

        // Verify first component
        assert_eq!(components[0].name, "PHY_0");
        let placement1 = components[0].placement.as_ref().unwrap();
        assert_eq!(placement1.placement_type, "FIXED");
        assert_eq!(placement1.x, 10120.0);
        assert_eq!(placement1.y, 10880.0);
        assert_eq!(placement1.orientation, "N");

        // Verify second component
        assert_eq!(components[1].name, "PHY_1");
        let placement2 = components[1].placement.as_ref().unwrap();
        assert_eq!(placement2.placement_type, "PLACED");
        assert_eq!(placement2.x, 20240.0);
        assert_eq!(placement2.y, 30360.0);
        assert_eq!(placement2.orientation, "S");
    }
}
