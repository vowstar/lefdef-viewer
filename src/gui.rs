// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use eframe::egui;
use egui::epaint::{PathShape, PathStroke};
use geo::{Coord, LineString, Polygon as GeoPolygon};
use rfd::FileDialog;

use crate::def::{reader::DefReader, Def};
use crate::export;
use crate::lef::{reader::LefReader, Lef};

/// Edge proximity detection result
#[derive(Debug, Clone)]
enum EdgeProximity {
    Left(f32),   // Distance to left edge
    Right(f32),  // Distance to right edge
    Top(f32),    // Distance to top edge
    Bottom(f32), // Distance to bottom edge
    None,        // Not close to any edge
}

/// Smart text positioning configuration
#[derive(Debug, Clone)]
struct TextPositioning {
    pos: egui::Pos2,
    anchor: egui::Align2,
    angle: f32, // Rotation angle in radians
}

#[derive(Default)]
pub struct LefDefViewer {
    lef_data: Option<Lef>,
    def_data: Option<Def>,
    lef_file_path: Option<String>,
    def_file_path: Option<String>,
    show_lef_details: bool,
    show_def_details: bool,
    zoom: f32,
    pan_x: f32,
    pan_y: f32,
    error_message: Option<String>,
    success_message: Option<String>,
    selected_cells: std::collections::HashSet<String>,
    visible_layers: std::collections::HashSet<String>,
    all_layers: std::collections::HashSet<String>,
    show_layers_panel: bool,
    show_pin_text: bool,
    fit_to_view_requested: bool,
    // DEF related selection states
    selected_components: std::collections::HashSet<String>,
    selected_pins: std::collections::HashSet<String>,
    selected_nets: std::collections::HashSet<String>,
    show_components: bool,
    show_pins: bool,
    show_nets: bool,
    show_diearea: bool,
}

impl LefDefViewer {
    pub fn new() -> Self {
        Self {
            lef_data: None,
            def_data: None,
            lef_file_path: None,
            def_file_path: None,
            show_lef_details: false,
            show_def_details: false,
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            error_message: None,
            success_message: None,
            selected_cells: std::collections::HashSet::new(),
            visible_layers: {
                let mut layers = std::collections::HashSet::new();
                layers.insert("OUTLINE".to_string());
                layers.insert("LABEL".to_string());
                layers
            },
            all_layers: std::collections::HashSet::new(),
            show_layers_panel: false,
            show_pin_text: true,
            fit_to_view_requested: false,
            // DEF related selection states
            selected_components: std::collections::HashSet::new(),
            selected_pins: std::collections::HashSet::new(),
            selected_nets: std::collections::HashSet::new(),
            show_components: true,
            show_pins: true,
            show_nets: true,
            show_diearea: true,
        }
    }

    fn render_text_with_outline(
        &self,
        painter: &egui::Painter,
        pos: egui::Pos2,
        anchor: egui::Align2,
        text: &str,
        font: egui::FontId,
        color: egui::Color32,
    ) {
        // Add black outline for white text
        if color == egui::Color32::WHITE {
            let outline_color = egui::Color32::BLACK;
            let outline_offset = 1.0;

            // Render outline in 8 directions
            let offsets = [
                (-outline_offset, -outline_offset), // Top-left
                (0.0, -outline_offset),             // Top
                (outline_offset, -outline_offset),  // Top-right
                (-outline_offset, 0.0),             // Left
                (outline_offset, 0.0),              // Right
                (-outline_offset, outline_offset),  // Bottom-left
                (0.0, outline_offset),              // Bottom
                (outline_offset, outline_offset),   // Bottom-right
            ];

            for (dx, dy) in offsets {
                let outline_pos = egui::pos2(pos.x + dx, pos.y + dy);
                painter.text(outline_pos, anchor, text, font.clone(), outline_color);
            }
        }

        // Render the main text on top
        painter.text(pos, anchor, text, font, color);
    }

    fn get_layer_color(&self, layer: &str) -> egui::Color32 {
        // Extract base layer name (before any '.' separator)
        let base_layer = layer.split('.').next().unwrap_or(layer);

        // Determine type-specific color adjustment
        let (base_color, type_adjustment) = match base_layer {
            "M1" | "METAL1" => (
                egui::Color32::from_rgba_unmultiplied(0, 150, 255, 180),
                self.get_type_color_adjustment(layer),
            ), // Blue
            "M2" | "METAL2" => (
                egui::Color32::from_rgba_unmultiplied(255, 100, 100, 180),
                self.get_type_color_adjustment(layer),
            ), // Red
            "M3" | "METAL3" => (
                egui::Color32::from_rgba_unmultiplied(255, 200, 0, 180),
                self.get_type_color_adjustment(layer),
            ), // Yellow
            "M4" | "METAL4" => (
                egui::Color32::from_rgba_unmultiplied(150, 255, 150, 180),
                self.get_type_color_adjustment(layer),
            ), // Light Green
            "M5" | "METAL5" => (
                egui::Color32::from_rgba_unmultiplied(255, 150, 255, 180),
                self.get_type_color_adjustment(layer),
            ), // Magenta
            "M6" | "METAL6" => (
                egui::Color32::from_rgba_unmultiplied(100, 255, 255, 180),
                self.get_type_color_adjustment(layer),
            ), // Cyan
            "M7" | "METAL7" => (
                egui::Color32::from_rgba_unmultiplied(255, 180, 100, 180),
                self.get_type_color_adjustment(layer),
            ), // Orange
            "M8" | "METAL8" => (
                egui::Color32::from_rgba_unmultiplied(180, 100, 255, 180),
                self.get_type_color_adjustment(layer),
            ), // Purple
            "POLY" | "POLY1" => (
                egui::Color32::from_rgba_unmultiplied(200, 255, 200, 180),
                self.get_type_color_adjustment(layer),
            ), // Pale Green
            "NDIFF" | "DIFF" => (
                egui::Color32::from_rgba_unmultiplied(100, 200, 255, 180),
                self.get_type_color_adjustment(layer),
            ), // Light Blue
            "PDIFF" => (
                egui::Color32::from_rgba_unmultiplied(255, 200, 200, 180),
                self.get_type_color_adjustment(layer),
            ), // Light Red
            "CONT" | "CONTACT" => (
                egui::Color32::from_rgba_unmultiplied(128, 128, 128, 200),
                self.get_type_color_adjustment(layer),
            ), // Gray
            "VIA1" => (
                egui::Color32::from_rgba_unmultiplied(200, 200, 255, 180),
                self.get_type_color_adjustment(layer),
            ), // Light Blue
            "VIA2" => (
                egui::Color32::from_rgba_unmultiplied(255, 200, 255, 180),
                self.get_type_color_adjustment(layer),
            ), // Light Magenta
            "VIA3" => (
                egui::Color32::from_rgba_unmultiplied(255, 255, 200, 180),
                self.get_type_color_adjustment(layer),
            ), // Light Yellow
            "VIA4" => (
                egui::Color32::from_rgba_unmultiplied(200, 255, 255, 180),
                self.get_type_color_adjustment(layer),
            ), // Light Cyan
            "OUTLINE" => (
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 180),
                (1.0, 1.0, 1.0, 1.0),
            ), // White for outline
            "LABEL" => (
                egui::Color32::from_rgba_unmultiplied(255, 255, 255, 255),
                (1.0, 1.0, 1.0, 1.0),
            ), // White for text labels
            _ => (
                egui::Color32::from_rgba_unmultiplied(160, 160, 160, 180),
                (1.0, 1.0, 1.0, 1.0),
            ), // Default Gray
        };

        // Apply type-specific color adjustment
        egui::Color32::from_rgba_unmultiplied(
            ((base_color.r() as f32 * type_adjustment.0) as u8).min(255),
            ((base_color.g() as f32 * type_adjustment.1) as u8).min(255),
            ((base_color.b() as f32 * type_adjustment.2) as u8).min(255),
            ((base_color.a() as f32 * type_adjustment.3) as u8).min(255),
        )
    }

    fn get_type_color_adjustment(&self, layer: &str) -> (f32, f32, f32, f32) {
        // Adjust color based on layer type suffix
        if layer.contains(".LABEL") {
            (1.2, 1.2, 1.2, 0.9) // Brighter, slightly transparent for labels
        } else if layer.contains(".PIN") {
            (1.0, 1.0, 1.0, 1.0) // Normal color for pins
        } else if layer.contains(".OBS") {
            (0.7, 0.7, 0.7, 0.8) // Darker, more transparent for obstructions
        } else {
            (1.0, 1.0, 1.0, 1.0) // Default unchanged
        }
    }

    fn get_layer_order(&self, layer: &str) -> i32 {
        // Extract base layer name and type
        let base_layer = layer.split('.').next().unwrap_or(layer);
        let layer_type = if layer.contains('.') {
            layer.split('.').nth(1).unwrap_or("")
        } else {
            ""
        };

        // Base layer ordering (multiply by 10 to leave room for type ordering)
        let base_order = match base_layer {
            "OUTLINE" => 5,
            "POLY" | "POLY1" => 10,
            "NDIFF" | "DIFF" | "PDIFF" => 20,
            "CONT" | "CONTACT" => 30,
            "M1" | "METAL1" => 40,
            "VIA1" => 50,
            "M2" | "METAL2" => 60,
            "VIA2" => 70,
            "M3" | "METAL3" => 80,
            "VIA3" => 90,
            "M4" | "METAL4" => 100,
            "VIA4" => 110,
            "M5" | "METAL5" => 120,
            "M6" | "METAL6" => 130,
            "M7" | "METAL7" => 140,
            "M8" | "METAL8" => 150,
            _ => 0, // Default bottom layer
        } * 10;

        // Type-specific ordering within each base layer
        let type_order = match layer_type {
            "OBS" => 1,   // Obstructions render first (bottom)
            "PIN" => 2,   // Pins render second
            "LABEL" => 3, // Labels render on top
            _ => 0,       // Default/base layer
        };

        base_order + type_order
    }

    // Utility function to calculate polygon area (shoelace formula)
    fn polygon_area(points: &[egui::Pos2]) -> f32 {
        if points.len() < 3 {
            return 0.0;
        }

        let mut area = 0.0;
        let n = points.len();
        for i in 0..n {
            let j = (i + 1) % n;
            area += (points[j].x - points[i].x) * (points[j].y + points[i].y);
        }
        area.abs() * 0.5
    }

    // Utility function to check if a polygon is convex
    fn is_convex(points: &[egui::Pos2]) -> bool {
        if points.len() < 3 {
            return true;
        }

        let n = points.len();
        let mut sign = 0;

        for i in 0..n {
            let p1 = points[i];
            let p2 = points[(i + 1) % n];
            let p3 = points[(i + 2) % n];

            // Cross product to determine turn direction
            let cross = (p2.x - p1.x) * (p3.y - p2.y) - (p2.y - p1.y) * (p3.x - p2.x);

            if cross.abs() > 1e-6 {
                // Avoid floating point precision issues
                let current_sign = if cross > 0.0 { 1 } else { -1 };
                if sign == 0 {
                    sign = current_sign;
                } else if sign != current_sign {
                    return false; // Direction change means non-convex
                }
            }
        }

        true
    }

    // Remove duplicate consecutive vertices
    fn deduplicate_vertices(points: &[egui::Pos2]) -> Vec<egui::Pos2> {
        if points.is_empty() {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut last_point = points[0];
        result.push(last_point);

        for &point in points.iter().skip(1) {
            // Only add if different from last point (with small tolerance)
            if (point.x - last_point.x).abs() > 1e-6 || (point.y - last_point.y).abs() > 1e-6 {
                result.push(point);
                last_point = point;
            }
        }

        result
    }

    fn compute_final_polygons(
        &self,
        additive_polygons: &[&crate::lef::LefPolygon],
        subtractive_polygons: &[&crate::lef::LefPolygon],
        offset_x: f32,
        offset_y: f32,
    ) -> Vec<Vec<egui::Pos2>> {
        use geo::{BooleanOps, Coord, LineString, Polygon as GeoPolygon};
        let mut final_polygons = Vec::new();

        // Robustness: if no additive polygons but we have subtractive polygons,
        // render the subtractive polygons as outlines to show their contours
        if additive_polygons.is_empty() {
            if !subtractive_polygons.is_empty() {
                // Fallback: render subtractive polygons as visible shapes
                for lef_polygon in subtractive_polygons {
                    if lef_polygon.points.len() >= 3 {
                        let mut screen_points = Vec::new();
                        for (px, py) in &lef_polygon.points {
                            let screen_x = offset_x + (*px as f32 * self.zoom);
                            let screen_y = offset_y + (*py as f32 * self.zoom);
                            screen_points.push(egui::pos2(screen_x, screen_y));
                        }

                        if screen_points.len() >= 3 {
                            let deduplicated = Self::deduplicate_vertices(&screen_points);
                            if deduplicated.len() >= 3 {
                                let area = Self::polygon_area(&deduplicated);
                                if area > 1e-6 {
                                    final_polygons.push(deduplicated);
                                }
                            }
                        }
                    }
                }
            }
            return final_polygons;
        }

        // Start with union of all additive polygons
        let mut result: Option<geo::MultiPolygon<f64>> = None;

        // Union all additive polygons first
        for lef_polygon in additive_polygons {
            if lef_polygon.points.len() >= 3 {
                // Convert LEF polygon to geo polygon
                let coords: Vec<Coord<f64>> = lef_polygon
                    .points
                    .iter()
                    .map(|(x, y)| Coord { x: *x, y: *y })
                    .collect();

                // Ensure the polygon is closed
                let mut line_coords = coords.clone();
                if line_coords.first() != line_coords.last() {
                    if let Some(first) = line_coords.first().cloned() {
                        line_coords.push(first);
                    }
                }

                if line_coords.len() >= 4 {
                    // At least 3 unique points + closing point
                    if let Ok(line_string) = LineString::try_from(line_coords) {
                        let geo_polygon = GeoPolygon::new(line_string, vec![]);

                        if let Some(existing_result) = result {
                            // Union with existing result
                            result = Some(existing_result.union(&geo_polygon.into()));
                        } else {
                            // First polygon
                            result = Some(geo_polygon.into());
                        }
                    }
                }
            }
        }

        // Subtract all subtractive polygons from the result
        if let Some(mut current_result) = result {
            for lef_polygon in subtractive_polygons {
                if lef_polygon.points.len() >= 3 {
                    // Convert LEF polygon to geo polygon
                    let coords: Vec<Coord<f64>> = lef_polygon
                        .points
                        .iter()
                        .map(|(x, y)| Coord { x: *x, y: *y })
                        .collect();

                    // Ensure the polygon is closed
                    let mut line_coords = coords.clone();
                    if line_coords.first() != line_coords.last() {
                        if let Some(first) = line_coords.first().cloned() {
                            line_coords.push(first);
                        }
                    }

                    if line_coords.len() >= 4 {
                        // At least 3 unique points + closing point
                        if let Ok(line_string) = LineString::try_from(line_coords) {
                            let geo_polygon = GeoPolygon::new(line_string, vec![]);

                            // Subtract from current result
                            current_result = current_result.difference(&geo_polygon.into());
                        }
                    }
                }
            }

            result = Some(current_result);
        }

        // Convert result back to screen coordinates for rendering
        if let Some(multi_polygon) = result {
            for polygon in multi_polygon {
                let exterior = polygon.exterior();
                let mut screen_points = Vec::new();

                for coord in exterior.coords() {
                    let screen_x = offset_x + (coord.x as f32 * self.zoom);
                    let screen_y = offset_y + (coord.y as f32 * self.zoom);
                    screen_points.push(egui::pos2(screen_x, screen_y));
                }

                if screen_points.len() >= 3 {
                    // Apply vertex deduplication
                    let deduplicated = Self::deduplicate_vertices(&screen_points);

                    // Filter out dust (microscopic polygons)
                    if deduplicated.len() >= 3 {
                        let area = Self::polygon_area(&deduplicated);
                        if area > 1e-6 {
                            // Minimum area threshold
                            final_polygons.push(deduplicated);
                        }
                    }
                }

                // Skip holes for now - they should be represented as empty space, not filled polygons
                // The boolean operations already handle holes correctly by creating the exterior ring
                // with the proper shape. Adding interior rings as separate filled polygons creates
                // visual artifacts.
                //
                // TODO: If hole visualization is needed, render them as outlines or with background color
                /*
                for interior in polygon.interiors() {
                    let mut hole_points = Vec::new();
                    for coord in interior.coords() {
                        let screen_x = offset_x + (coord.x as f32 * self.zoom);
                        let screen_y = offset_y + (coord.y as f32 * self.zoom);
                        hole_points.push(egui::pos2(screen_x, screen_y));
                    }
                    if hole_points.len() >= 3 {
                        // Render holes as outlines only to show their boundaries
                        final_polygons.push(hole_points);
                    }
                }
                */
            }
        }

        final_polygons
    }

    fn calculate_bounds(&self) -> Option<(f32, f32, f32, f32)> {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut found_any = false;

        if let Some(lef) = &self.lef_data {
            for macro_def in &lef.macros {
                if !self.selected_cells.is_empty() && !self.selected_cells.contains(&macro_def.name)
                {
                    continue;
                }

                let mut macro_has_content = false;
                let macro_x = macro_def.origin_x as f32;
                let macro_y = macro_def.origin_y as f32;

                // Include macro size bounds
                let left = macro_x;
                let bottom = macro_y;
                let right = left + macro_def.size_x as f32;
                let top = bottom + macro_def.size_y as f32;

                min_x = min_x.min(left);
                min_y = min_y.min(bottom);
                max_x = max_x.max(right);
                max_y = max_y.max(top);
                macro_has_content = true;

                // Include pin shapes in bounds calculation
                for pin in &macro_def.pins {
                    for port in &pin.ports {
                        // Include rectangles
                        for rect in &port.rects {
                            let detailed_layer = format!("{}.PIN", rect.layer);
                            if self.visible_layers.contains(&detailed_layer) {
                                let rect_left = macro_x + rect.xl as f32;
                                let rect_bottom = macro_y + rect.yl as f32;
                                let rect_right = macro_x + rect.xh as f32;
                                let rect_top = macro_y + rect.yh as f32;

                                min_x = min_x.min(rect_left);
                                min_y = min_y.min(rect_bottom);
                                max_x = max_x.max(rect_right);
                                max_y = max_y.max(rect_top);
                                macro_has_content = true;
                            }
                        }

                        // Include polygons
                        for polygon in &port.polygons {
                            let detailed_layer = format!("{}.PIN", polygon.layer);
                            if self.visible_layers.contains(&detailed_layer) {
                                for (px, py) in &polygon.points {
                                    let point_x = macro_x + *px as f32;
                                    let point_y = macro_y + *py as f32;

                                    min_x = min_x.min(point_x);
                                    min_y = min_y.min(point_y);
                                    max_x = max_x.max(point_x);
                                    max_y = max_y.max(point_y);
                                    macro_has_content = true;
                                }
                            }
                        }
                    }
                }

                // Include obstruction shapes in bounds calculation
                if let Some(obs) = &macro_def.obstruction {
                    // Include obstruction rectangles
                    for rect in &obs.rects {
                        let detailed_layer = format!("{}.OBS", rect.layer);
                        if self.visible_layers.contains(&detailed_layer) {
                            let rect_left = macro_x + rect.xl as f32;
                            let rect_bottom = macro_y + rect.yl as f32;
                            let rect_right = macro_x + rect.xh as f32;
                            let rect_top = macro_y + rect.yh as f32;

                            min_x = min_x.min(rect_left);
                            min_y = min_y.min(rect_bottom);
                            max_x = max_x.max(rect_right);
                            max_y = max_y.max(rect_top);
                            macro_has_content = true;
                        }
                    }

                    // Include obstruction polygons
                    for polygon in &obs.polygons {
                        let detailed_layer = format!("{}.OBS", polygon.layer);
                        if self.visible_layers.contains(&detailed_layer) {
                            for (px, py) in &polygon.points {
                                let point_x = macro_x + *px as f32;
                                let point_y = macro_y + *py as f32;

                                min_x = min_x.min(point_x);
                                min_y = min_y.min(point_y);
                                max_x = max_x.max(point_x);
                                max_y = max_y.max(point_y);
                                macro_has_content = true;
                            }
                        }
                    }
                }

                if macro_has_content {
                    found_any = true;
                }
            }
        }

        if let Some(def) = &self.def_data {
            for point in &def.die_area_points {
                let x = point.0 as f32 * 0.001; // Scale from microns
                let y = point.1 as f32 * 0.001;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
                found_any = true;
            }
        }

        if found_any && max_x > min_x && max_y > min_y {
            Some((min_x, min_y, max_x, max_y))
        } else {
            None
        }
    }

    fn calculate_outline_bounds(&self) -> Option<(f32, f32, f32, f32)> {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut found_any = false;

        // Only consider visible OUTLINE layers from selected macros
        if let Some(lef) = &self.lef_data {
            for macro_def in &lef.macros {
                if !self.selected_cells.is_empty() && !self.selected_cells.contains(&macro_def.name)
                {
                    continue;
                }

                // Only use macro size bounds (OUTLINE)
                if self.visible_layers.contains("OUTLINE") {
                    let macro_x = macro_def.origin_x as f32;
                    let macro_y = macro_def.origin_y as f32;
                    let left = macro_x;
                    let bottom = macro_y;
                    let right = left + macro_def.size_x as f32;
                    let top = bottom + macro_def.size_y as f32;

                    min_x = min_x.min(left);
                    min_y = min_y.min(bottom);
                    max_x = max_x.max(right);
                    max_y = max_y.max(top);
                    found_any = true;
                }
            }
        }

        // Also consider DEF die area if no LEF macros or OUTLINE not visible
        if !found_any {
            if let Some(def) = &self.def_data {
                for point in &def.die_area_points {
                    let x = point.0 as f32 * 0.001; // Scale from microns
                    let y = point.1 as f32 * 0.001;
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                    found_any = true;
                }
            }
        }

        if found_any && max_x > min_x && max_y > min_y {
            Some((min_x, min_y, max_x, max_y))
        } else {
            None
        }
    }

    fn fit_to_view(&mut self, available_size: egui::Vec2) {
        if let Some((min_x, min_y, max_x, max_y)) = self.calculate_outline_bounds() {
            let content_width = max_x - min_x;
            let content_height = max_y - min_y;

            if content_width > 0.0 && content_height > 0.0 {
                // Use 90% of available space for content, 10% for margin
                let target_width = available_size.x * 0.9;
                let target_height = available_size.y * 0.9;

                let scale_x = target_width / content_width;
                let scale_y = target_height / content_height;

                // Use the smaller scale to ensure everything fits
                self.zoom = scale_x.min(scale_y).max(0.1);

                // Center the content properly - use the outside boundary center
                let center_x = (min_x + max_x) * 0.5;
                let center_y = (min_y + max_y) * 0.5;

                // Reset pan to center the content in the available space
                // Corrected formula: pan = -world_center * zoom
                self.pan_x = -center_x * self.zoom;
                self.pan_y = -center_y * self.zoom;
            }
        }
    }

    fn load_lef_file(&mut self, path: String) {
        let reader = LefReader::new();
        match reader.read(&path) {
            Ok(lef) => {
                // Update layer lists - collect all available layers with detailed type information
                self.all_layers.clear();
                self.visible_layers.clear();

                // Add virtual layers
                self.all_layers.insert("OUTLINE".to_string());
                self.visible_layers.insert("OUTLINE".to_string());

                // Add LABEL virtual layer for PIN text control
                self.all_layers.insert("LABEL".to_string());
                if self.show_pin_text {
                    self.visible_layers.insert("LABEL".to_string());
                }

                for macro_def in &lef.macros {
                    for pin in &macro_def.pins {
                        for port in &pin.ports {
                            for rect in &port.rects {
                                let detailed_layer = format!("{}.PIN", rect.layer);
                                self.all_layers.insert(detailed_layer.clone());
                                self.visible_layers.insert(detailed_layer);
                            }
                            for polygon in &port.polygons {
                                let detailed_layer = format!("{}.PIN", polygon.layer);
                                self.all_layers.insert(detailed_layer.clone());
                                self.visible_layers.insert(detailed_layer);
                            }
                        }
                    }
                    if let Some(obs) = &macro_def.obstruction {
                        for rect in &obs.rects {
                            let detailed_layer = format!("{}.OBS", rect.layer);
                            self.all_layers.insert(detailed_layer.clone());
                            // OBS layers are added to all_layers but not visible_layers (default hidden)
                        }
                        for polygon in &obs.polygons {
                            let detailed_layer = format!("{}.OBS", polygon.layer);
                            self.all_layers.insert(detailed_layer.clone());
                            // OBS layers are added to all_layers but not visible_layers (default hidden)
                        }
                    }
                }

                // Debug: Count obstructions and layers
                let mut obs_count = 0;
                let mut obs_macros = Vec::new();
                for macro_def in &lef.macros {
                    if let Some(obs) = &macro_def.obstruction {
                        obs_count += obs.rects.len() + obs.polygons.len();
                        obs_macros.push(&macro_def.name);
                    }
                }

                if obs_count > 0 {
                    println!(
                        "DEBUG: Found {} OBS shapes in {} macros",
                        obs_count,
                        obs_macros.len()
                    );
                } else {
                    println!("DEBUG: No OBS data found in any macro");
                }

                // Count OBS layers
                let obs_layers: Vec<&String> = self
                    .all_layers
                    .iter()
                    .filter(|layer| layer.contains(".OBS"))
                    .collect();
                println!(
                    "DEBUG: Added {} OBS layers (default hidden)",
                    obs_layers.len()
                );

                self.lef_data = Some(lef);
                self.lef_file_path = Some(path);
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load LEF file: {}", e));
            }
        }
    }

    fn load_def_file(&mut self, path: String) {
        let reader = DefReader::new();
        match reader.read(&path) {
            Ok(def) => {
                self.def_data = Some(def);
                self.def_file_path = Some(path);
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to load DEF file: {}", e));
            }
        }
    }

    fn handle_export_lef_csv(&mut self) {
        if let Some(lef_data) = &self.lef_data {
            if let Some(file_path) = FileDialog::new()
                .set_file_name("lef_macros.csv")
                .add_filter("CSV files", &["csv"])
                .save_file()
            {
                match export::export_lef_to_csv(lef_data, &file_path.to_string_lossy()) {
                    Ok(()) => {
                        self.success_message = Some(format!(
                            "Successfully exported {} macros to CSV file: {}",
                            lef_data.macros.len(),
                            file_path.display()
                        ));
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to export CSV: {}", e));
                    }
                }
            }
        }
    }

    fn render_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open LEF File").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("LEF files", &["lef"])
                        .pick_file()
                    {
                        self.load_lef_file(path.to_string_lossy().to_string());
                    }
                    ui.close_menu();
                }

                if ui.button("Open DEF File").clicked() {
                    if let Some(path) = FileDialog::new()
                        .add_filter("DEF files", &["def"])
                        .pick_file()
                    {
                        self.load_def_file(path.to_string_lossy().to_string());
                    }
                    ui.close_menu();
                }

                ui.separator();

                if ui
                    .add_enabled(
                        self.lef_data.is_some(),
                        egui::Button::new("Export LEF to CSV"),
                    )
                    .clicked()
                {
                    self.handle_export_lef_csv();
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Close LEF File").clicked() {
                    self.lef_data = None;
                    self.lef_file_path = None;
                    self.selected_cells.clear();
                    self.all_layers.clear();
                    self.visible_layers.clear();
                    ui.close_menu();
                }

                if ui.button("Close DEF File").clicked() {
                    self.def_data = None;
                    self.def_file_path = None;
                    ui.close_menu();
                }

                ui.separator();

                if ui.button("Exit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("View", |ui| {
                ui.checkbox(&mut self.show_lef_details, "Show LEF Details");
                ui.checkbox(&mut self.show_def_details, "Show DEF Details");
                ui.checkbox(&mut self.show_layers_panel, "Show Layers Panel");
                ui.separator();
                // Sync show_pin_text with LABEL layer visibility
                let mut label_visible = self.visible_layers.contains("LABEL");
                if ui.checkbox(&mut label_visible, "Show PIN Text").clicked() {
                    if label_visible {
                        self.visible_layers.insert("LABEL".to_string());
                    } else {
                        self.visible_layers.remove("LABEL");
                    }
                    self.show_pin_text = label_visible;
                }
            });
        });
    }

    fn render_left_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("Files");

            if let Some(path) = &self.lef_file_path {
                ui.label(format!("LEF: {}", path));
            } else {
                ui.label("No LEF file loaded");
            }

            if let Some(path) = &self.def_file_path {
                ui.label(format!("DEF: {}", path));
            } else {
                ui.label("No DEF file loaded");
            }

            ui.separator();

            ui.heading("Controls");

            // Zoom controls
            ui.horizontal(|ui| {
                ui.label("Zoom:");
                if ui.button("−").clicked() {
                    self.zoom = (self.zoom * 0.8).max(0.01);
                }
                ui.add(egui::Slider::new(&mut self.zoom, 0.01..=20.0).logarithmic(true));
                if ui.button("+").clicked() {
                    self.zoom = (self.zoom * 1.25).min(20.0);
                }
            });

            ui.horizontal(|ui| {
                if ui.button("Fit to View").clicked() {
                    self.fit_to_view_requested = true;
                }
                if ui.button("Reset View").clicked() {
                    self.zoom = 1.0;
                    self.pan_x = 0.0;
                    self.pan_y = 0.0;
                }
            });

            ui.label("💡 Fit to View uses OUTLINE layers only");

            ui.separator();

            if let Some(lef) = &self.lef_data {
                ui.heading("LEF Macros (Cells)");
                ui.label("Select cells to display:");
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for macro_def in &lef.macros {
                            let mut is_selected = self.selected_cells.contains(&macro_def.name);
                            if ui.checkbox(&mut is_selected, &macro_def.name).clicked() {
                                if is_selected {
                                    self.selected_cells.insert(macro_def.name.clone());
                                } else {
                                    self.selected_cells.remove(&macro_def.name);
                                }
                            }

                            ui.collapsing(format!("Details: {}", &macro_def.name), |ui| {
                                ui.label(format!("Class: {}", macro_def.class));
                                ui.label(format!(
                                    "Size: {:.3} x {:.3}",
                                    macro_def.size_x, macro_def.size_y
                                ));
                                ui.label(format!("Pins: {}", macro_def.pins.len()));
                                if let Some(obs) = &macro_def.obstruction {
                                    ui.label(format!("Obstructions: {}", obs.rects.len()));
                                }
                            });
                        }
                    });

                ui.separator();
                if ui.button("Select All Cells").clicked() {
                    for macro_def in &lef.macros {
                        self.selected_cells.insert(macro_def.name.clone());
                    }
                }
                if ui.button("Clear Selection").clicked() {
                    self.selected_cells.clear();
                }
            }

            // DEF Structure Section
            if let Some(def) = &self.def_data {
                ui.separator();
                ui.heading("📊 DEF Structure");

                // DESIGN information
                ui.label("📐 DESIGN");
                ui.indent("design_info", |ui| {
                    ui.label("Design loaded successfully");
                });

                ui.separator();

                // DIEAREA section
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_diearea, "");
                    if def.die_area_points.len() == 2 {
                        ui.label(format!(
                            "🔲 DIEAREA (Rectangle: {} points)",
                            def.die_area_points.len()
                        ));
                    } else {
                        ui.label(format!(
                            "🔲 DIEAREA (Polygon: {} points)",
                            def.die_area_points.len()
                        ));
                    }
                });

                // Show DIEAREA details
                if !def.die_area_points.is_empty() {
                    ui.indent("diearea_details", |ui| {
                        if def.die_area_points.len() == 2 {
                            let p1 = &def.die_area_points[0];
                            let p2 = &def.die_area_points[1];
                            let width = (p2.0 - p1.0).abs();
                            let height = (p2.1 - p1.1).abs();
                            ui.label(format!(
                                "  📐 Size: {:.1} × {:.1} μm",
                                width / 1000.0,
                                height / 1000.0
                            ));
                            ui.label(format!(
                                "  📍 Bottom-left: ({:.1}, {:.1})",
                                p1.0 / 1000.0,
                                p1.1 / 1000.0
                            ));
                            ui.label(format!(
                                "  📍 Top-right: ({:.1}, {:.1})",
                                p2.0 / 1000.0,
                                p2.1 / 1000.0
                            ));
                        } else {
                            ui.label("  📐 Custom polygon shape");
                            ui.label(format!("  📍 {} vertices", def.die_area_points.len()));

                            // Calculate bounding box
                            let min_x = def
                                .die_area_points
                                .iter()
                                .map(|p| p.0)
                                .fold(f64::INFINITY, f64::min);
                            let max_x = def
                                .die_area_points
                                .iter()
                                .map(|p| p.0)
                                .fold(f64::NEG_INFINITY, f64::max);
                            let min_y = def
                                .die_area_points
                                .iter()
                                .map(|p| p.1)
                                .fold(f64::INFINITY, f64::min);
                            let max_y = def
                                .die_area_points
                                .iter()
                                .map(|p| p.1)
                                .fold(f64::NEG_INFINITY, f64::max);

                            ui.label(format!(
                                "  📦 Bounds: ({:.1}, {:.1}) to ({:.1}, {:.1})",
                                min_x / 1000.0,
                                min_y / 1000.0,
                                max_x / 1000.0,
                                max_y / 1000.0
                            ));
                        }
                    });
                }

                ui.separator();

                // COMPONENTS section
                egui::CollapsingHeader::new(format!("📦 COMPONENTS ({})", def.components.len()))
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.show_components, "Show Components");
                            ui.label(format!("Total: {}", def.components.len()));
                        });

                        if !def.components.is_empty() {
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Select All").clicked() {
                                    for component in &def.components {
                                        self.selected_components.insert(component.id.clone());
                                    }
                                }
                                if ui.button("Clear Selection").clicked() {
                                    self.selected_components.clear();
                                }
                            });

                            egui::ScrollArea::vertical()
                                .auto_shrink([false, true])
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    for component in &def.components {
                                        let mut is_selected =
                                            self.selected_components.contains(&component.id);
                                        let response = ui.checkbox(&mut is_selected, &component.id);
                                        if response.clicked() {
                                            if is_selected {
                                                self.selected_components
                                                    .insert(component.id.clone());
                                            } else {
                                                self.selected_components.remove(&component.id);
                                            }
                                        }

                                        // Show component details on hover
                                        if response.hovered() {
                                            response.on_hover_text(format!(
                                                "  {} at ({:.1}, {:.1}) {}",
                                                component.name,
                                                component.x,
                                                component.y,
                                                component.orient
                                            ));
                                        }
                                    }
                                });
                        }
                    });

                ui.separator();

                // PINS section
                egui::CollapsingHeader::new(format!("📌 PINS ({})", def.pins.len()))
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.show_pins, "Show Pins");
                            ui.label(format!("Total: {}", def.pins.len()));
                        });

                        if !def.pins.is_empty() {
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Select All").clicked() {
                                    for pin in &def.pins {
                                        self.selected_pins.insert(pin.name.clone());
                                    }
                                }
                                if ui.button("Clear Selection").clicked() {
                                    self.selected_pins.clear();
                                }
                            });

                            egui::ScrollArea::vertical()
                                .auto_shrink([false, true])
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    for pin in &def.pins {
                                        let mut is_selected =
                                            self.selected_pins.contains(&pin.name);
                                        let response = ui.checkbox(&mut is_selected, &pin.name);
                                        if response.clicked() {
                                            if is_selected {
                                                self.selected_pins.insert(pin.name.clone());
                                            } else {
                                                self.selected_pins.remove(&pin.name);
                                            }
                                        }

                                        // Show pin details on hover
                                        if response.hovered() {
                                            response.on_hover_text(format!(
                                                "  {} {} {} at ({:.1}, {:.1})",
                                                pin.direction, pin.use_type, pin.net, pin.x, pin.y
                                            ));
                                        }
                                    }
                                });
                        }
                    });

                ui.separator();

                // NETS section
                egui::CollapsingHeader::new(format!("🔗 NETS ({})", def.nets.len()))
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut self.show_nets, "Show Nets");
                            ui.label(format!("Total: {}", def.nets.len()));
                        });

                        if !def.nets.is_empty() {
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Select All").clicked() {
                                    for net in &def.nets {
                                        self.selected_nets.insert(net.name.clone());
                                    }
                                }
                                if ui.button("Clear Selection").clicked() {
                                    self.selected_nets.clear();
                                }
                            });

                            egui::ScrollArea::vertical()
                                .auto_shrink([false, true])
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    for net in &def.nets {
                                        let mut is_selected =
                                            self.selected_nets.contains(&net.name);
                                        let response = ui.checkbox(&mut is_selected, &net.name);
                                        if response.clicked() {
                                            if is_selected {
                                                self.selected_nets.insert(net.name.clone());
                                            } else {
                                                self.selected_nets.remove(&net.name);
                                            }
                                        }

                                        // Show net details on hover
                                        if response.hovered() {
                                            response.on_hover_text(format!(
                                                "  {} instances, {} pins",
                                                net.instances.len(),
                                                net.pins.len()
                                            ));
                                        }
                                    }
                                });
                        }
                    });
            }
        });
    }

    fn render_visualization(&mut self, ui: &mut egui::Ui) {
        // First record the remaining available space
        let available_size = ui.available_size();

        // Then allocate this entire space at once
        let (response, painter) = ui.allocate_painter(available_size, egui::Sense::drag());

        // Use the previously recorded `available_size` for fit-to-view
        // Handle fit to view request
        if self.fit_to_view_requested {
            self.fit_to_view(available_size);
            self.fit_to_view_requested = false;
        }

        // Handle F key for fit to view
        if ui.input(|i| i.key_pressed(egui::Key::F)) {
            self.fit_to_view(available_size);
        }

        // Handle mouse interactions
        if response.dragged() {
            let delta = response.drag_delta();
            self.pan_x += delta.x;
            self.pan_y += delta.y;
        }

        // Handle mouse wheel zoom
        if let Some(hover_pos) = response.hover_pos() {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta.y);
            if scroll_delta != 0.0 {
                let zoom_factor = if scroll_delta > 0.0 { 1.1 } else { 0.9 };
                let old_zoom = self.zoom;

                // Get drawing area center
                let rect = response.rect;
                let center = rect.center();

                // Convert mouse screen position to world coordinates before zoom
                let world_x = (hover_pos.x - center.x - self.pan_x) / old_zoom;
                let world_y = (hover_pos.y - center.y - self.pan_y) / old_zoom;

                // Update zoom
                self.zoom = (self.zoom * zoom_factor).clamp(0.01, 20.0);

                // Adjust pan so that the world point under mouse stays at the same screen position
                self.pan_x = hover_pos.x - center.x - (world_x * self.zoom);
                self.pan_y = hover_pos.y - center.y - (world_y * self.zoom);
            }
        }

        let rect = response.rect;
        let center = rect.center();

        painter.rect_filled(rect, 0.0, egui::Color32::BLACK);

        // Store text to render on top
        let mut texts_to_render = Vec::new();
        let mut smart_texts_to_render = Vec::new();

        if let Some(lef) = &self.lef_data {
            for macro_def in &lef.macros {
                // Only render selected cells (or all if none selected)
                if !self.selected_cells.is_empty() && !self.selected_cells.contains(&macro_def.name)
                {
                    continue;
                }

                let x = center.x + self.pan_x + (macro_def.origin_x as f32 * self.zoom);
                let y = center.y + self.pan_y + (macro_def.origin_y as f32 * self.zoom);
                let w = macro_def.size_x as f32 * self.zoom;
                let h = macro_def.size_y as f32 * self.zoom;

                let macro_rect =
                    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w.max(1.0), h.max(1.0)));

                // Render macro outline if OUTLINE layer is visible
                if self.visible_layers.contains("OUTLINE") {
                    let outline_color = self.get_layer_color("OUTLINE");
                    painter.rect_stroke(macro_rect, 0.0, egui::Stroke::new(2.0, outline_color));
                }

                // Render pins with layer visibility
                for pin in &macro_def.pins {
                    let mut pin_bounds: Option<(f32, f32, f32, f32)> = None; // min_x, min_y, max_x, max_y
                    let mut has_visible_shapes = false;

                    for port in &pin.ports {
                        // Render rectangles
                        for rect_data in &port.rects {
                            let detailed_layer = format!("{}.PIN", rect_data.layer);
                            if !self.visible_layers.contains(&detailed_layer) {
                                continue;
                            }

                            has_visible_shapes = true;

                            let pin_rect = egui::Rect::from_min_max(
                                egui::pos2(
                                    x + (rect_data.xl as f32 * self.zoom),
                                    y + (rect_data.yl as f32 * self.zoom),
                                ),
                                egui::pos2(
                                    x + (rect_data.xh as f32 * self.zoom),
                                    y + (rect_data.yh as f32 * self.zoom),
                                ),
                            );

                            let color = self.get_layer_color(&detailed_layer);
                            painter.rect_filled(pin_rect, 0.0, color);

                            // Update pin bounds for text positioning
                            let rect_min_x = x + (rect_data.xl as f32 * self.zoom);
                            let rect_min_y = y + (rect_data.yl as f32 * self.zoom);
                            let rect_max_x = x + (rect_data.xh as f32 * self.zoom);
                            let rect_max_y = y + (rect_data.yh as f32 * self.zoom);

                            if let Some((min_x, min_y, max_x, max_y)) = pin_bounds {
                                pin_bounds = Some((
                                    min_x.min(rect_min_x),
                                    min_y.min(rect_min_y),
                                    max_x.max(rect_max_x),
                                    max_y.max(rect_max_y),
                                ));
                            } else {
                                pin_bounds = Some((rect_min_x, rect_min_y, rect_max_x, rect_max_y));
                            }
                        }

                        // Group polygons by layer for this specific PORT (within this PIN)
                        // Boolean operations only apply within the same pin and same layer
                        let mut layer_polygons: std::collections::HashMap<
                            String,
                            Vec<&crate::lef::LefPolygon>,
                        > = std::collections::HashMap::new();
                        for polygon_data in &port.polygons {
                            let detailed_layer = format!("{}.PIN", polygon_data.layer);
                            if !self.visible_layers.contains(&detailed_layer) {
                                continue;
                            }
                            layer_polygons
                                .entry(detailed_layer.clone())
                                .or_default()
                                .push(polygon_data);
                        }

                        // Sort layers by z-order to prevent flickering
                        let mut sorted_layers: Vec<_> = layer_polygons.into_iter().collect();
                        sorted_layers
                            .sort_by_key(|(layer_name, _)| self.get_layer_order(layer_name));

                        // Process each layer's polygons with proper boolean operations
                        for (layer_name, polygons) in sorted_layers {
                            has_visible_shapes = true;
                            let color = self.get_layer_color(&layer_name);

                            // Separate counterclockwise (additive) and clockwise (subtractive) polygons
                            let mut additive_polygons = Vec::new();
                            let mut subtractive_polygons = Vec::new();

                            for polygon_data in &polygons {
                                if polygon_data.points.len() >= 3 {
                                    // LEF specification: counterclockwise = solid areas, clockwise = holes
                                    if polygon_data.is_hole {
                                        // Clockwise → hole/void
                                        subtractive_polygons.push(polygon_data);
                                    } else {
                                        // Counterclockwise → solid
                                        additive_polygons.push(polygon_data);
                                    }
                                }
                            }

                            // Compute the final polygons after boolean operations
                            let additive_refs: Vec<&crate::lef::LefPolygon> =
                                additive_polygons.iter().map(|&&p| p).collect();
                            let subtractive_refs: Vec<&crate::lef::LefPolygon> =
                                subtractive_polygons.iter().map(|&&p| p).collect();
                            let final_polygons = self.compute_final_polygons(
                                &additive_refs[..],
                                &subtractive_refs[..],
                                x,
                                y,
                            );

                            // Render the final computed polygons
                            for (i, screen_points) in final_polygons.iter().enumerate() {
                                if screen_points.len() >= 3 {
                                    // Calculate bounds for text positioning
                                    let mut poly_min_x = f32::INFINITY;
                                    let mut poly_min_y = f32::INFINITY;
                                    let mut poly_max_x = f32::NEG_INFINITY;
                                    let mut poly_max_y = f32::NEG_INFINITY;

                                    for point in screen_points {
                                        poly_min_x = poly_min_x.min(point.x);
                                        poly_min_y = poly_min_y.min(point.y);
                                        poly_max_x = poly_max_x.max(point.x);
                                        poly_max_y = poly_max_y.max(point.y);
                                    }

                                    // --- draw filled polygon, irrespective of convexity ---
                                    painter.add(egui::Shape::Path(PathShape {
                                        points: screen_points.clone(), // already deduped
                                        closed: true,
                                        fill: color,
                                        stroke: PathStroke::NONE,
                                    }));

                                    // Update pin bounds for text positioning
                                    if let Some((min_x, min_y, max_x, max_y)) = pin_bounds {
                                        pin_bounds = Some((
                                            min_x.min(poly_min_x),
                                            min_y.min(poly_min_y),
                                            max_x.max(poly_max_x),
                                            max_y.max(poly_max_y),
                                        ));
                                    } else {
                                        pin_bounds =
                                            Some((poly_min_x, poly_min_y, poly_max_x, poly_max_y));
                                    }
                                }
                            }
                        }
                    }

                    // Add PIN text once per pin if LABEL layer is visible, zoom is high enough, and pin has visible shapes
                    if self.visible_layers.contains("LABEL")
                        && self.zoom > 0.5
                        && has_visible_shapes
                    {
                        if let Some((min_x, min_y, max_x, max_y)) = pin_bounds {
                            let pin_center =
                                egui::pos2((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);
                            texts_to_render.push((
                                pin_center,
                                pin.name.clone(),
                                egui::FontId::monospace(8.0),
                                egui::Color32::WHITE,
                            ));
                        }
                    }
                }

                // Render obstructions
                if let Some(obs) = &macro_def.obstruction {
                    // Render obstruction rectangles
                    for rect_data in &obs.rects {
                        let detailed_layer = format!("{}.OBS", rect_data.layer);

                        if !self.visible_layers.contains(&detailed_layer) {
                            continue;
                        }

                        let obs_rect = egui::Rect::from_min_max(
                            egui::pos2(
                                x + (rect_data.xl as f32 * self.zoom),
                                y + (rect_data.yl as f32 * self.zoom),
                            ),
                            egui::pos2(
                                x + (rect_data.xh as f32 * self.zoom),
                                y + (rect_data.yh as f32 * self.zoom),
                            ),
                        );
                        let color = self.get_layer_color(&detailed_layer);
                        // Render OBS as dashed outline instead of filled rectangle
                        let stroke = egui::Stroke::new(1.0, color);
                        painter.rect_stroke(obs_rect, 0.0, stroke);

                        // Add dashed pattern by drawing additional lines
                        let dash_length = 3.0;
                        let gap_length = 2.0;
                        let pattern_length = dash_length + gap_length;

                        // Top edge dashes
                        let mut x = obs_rect.min.x;
                        while x < obs_rect.max.x {
                            let end_x = (x + dash_length).min(obs_rect.max.x);
                            painter.line_segment(
                                [
                                    egui::pos2(x, obs_rect.min.y),
                                    egui::pos2(end_x, obs_rect.min.y),
                                ],
                                stroke,
                            );
                            x += pattern_length;
                        }

                        // Bottom edge dashes
                        let mut x = obs_rect.min.x;
                        while x < obs_rect.max.x {
                            let end_x = (x + dash_length).min(obs_rect.max.x);
                            painter.line_segment(
                                [
                                    egui::pos2(x, obs_rect.max.y),
                                    egui::pos2(end_x, obs_rect.max.y),
                                ],
                                stroke,
                            );
                            x += pattern_length;
                        }

                        // Left edge dashes
                        let mut y = obs_rect.min.y;
                        while y < obs_rect.max.y {
                            let end_y = (y + dash_length).min(obs_rect.max.y);
                            painter.line_segment(
                                [
                                    egui::pos2(obs_rect.min.x, y),
                                    egui::pos2(obs_rect.min.x, end_y),
                                ],
                                stroke,
                            );
                            y += pattern_length;
                        }

                        // Right edge dashes
                        let mut y = obs_rect.min.y;
                        while y < obs_rect.max.y {
                            let end_y = (y + dash_length).min(obs_rect.max.y);
                            painter.line_segment(
                                [
                                    egui::pos2(obs_rect.max.x, y),
                                    egui::pos2(obs_rect.max.x, end_y),
                                ],
                                stroke,
                            );
                            y += pattern_length;
                        }
                    }

                    // Group obstruction polygons by layer to avoid z-fighting
                    let mut obs_layer_polygons: std::collections::HashMap<
                        String,
                        Vec<&crate::lef::LefPolygon>,
                    > = std::collections::HashMap::new();
                    for polygon_data in &obs.polygons {
                        let detailed_layer = format!("{}.OBS", polygon_data.layer);
                        if !self.visible_layers.contains(&detailed_layer) {
                            continue;
                        }
                        obs_layer_polygons
                            .entry(detailed_layer.clone())
                            .or_default()
                            .push(polygon_data);
                    }

                    // Sort obstruction layers by z-order to prevent flickering
                    let mut sorted_obs_layers: Vec<_> = obs_layer_polygons.into_iter().collect();
                    sorted_obs_layers
                        .sort_by_key(|(layer_name, _)| self.get_layer_order(layer_name));

                    // Render obstruction polygons by layer
                    for (layer_name, polygons) in sorted_obs_layers {
                        let color = self.get_layer_color(&layer_name);

                        // Separate counterclockwise (additive) and clockwise (subtractive) polygons
                        let mut additive_polygons = Vec::new();
                        let mut subtractive_polygons = Vec::new();

                        for polygon_data in &polygons {
                            if polygon_data.points.len() >= 3 {
                                // LEF specification: counterclockwise = solid areas, clockwise = holes
                                if polygon_data.is_hole {
                                    // Clockwise → hole/void
                                    subtractive_polygons.push(polygon_data);
                                } else {
                                    // Counterclockwise → solid
                                    additive_polygons.push(polygon_data);
                                }
                            }
                        }

                        // Compute the final polygons after boolean operations
                        let additive_refs: Vec<&crate::lef::LefPolygon> =
                            additive_polygons.iter().map(|&&p| p).collect();
                        let subtractive_refs: Vec<&crate::lef::LefPolygon> =
                            subtractive_polygons.iter().map(|&&p| p).collect();
                        let final_polygons = self.compute_final_polygons(
                            &additive_refs[..],
                            &subtractive_refs[..],
                            x,
                            y,
                        );

                        // Render the final computed polygons as dashed outlines
                        for screen_points in final_polygons {
                            if screen_points.len() >= 3 {
                                // Draw dashed outline for OBS polygons
                                let stroke = egui::Stroke::new(1.0, color);

                                // Draw dashed lines between consecutive points
                                for i in 0..screen_points.len() {
                                    let start = screen_points[i];
                                    let end = screen_points[(i + 1) % screen_points.len()];

                                    // Calculate line direction and length
                                    let dx = end.x - start.x;
                                    let dy = end.y - start.y;
                                    let line_length = (dx * dx + dy * dy).sqrt();

                                    if line_length > 0.0 {
                                        let dash_length = 3.0;
                                        let gap_length = 2.0;
                                        let pattern_length = dash_length + gap_length;

                                        // Normalize direction
                                        let dir_x = dx / line_length;
                                        let dir_y = dy / line_length;

                                        // Draw dashes along the line
                                        let mut t = 0.0;
                                        while t < line_length {
                                            let dash_end = (t + dash_length).min(line_length);
                                            let dash_start_pos = egui::pos2(
                                                start.x + dir_x * t,
                                                start.y + dir_y * t,
                                            );
                                            let dash_end_pos = egui::pos2(
                                                start.x + dir_x * dash_end,
                                                start.y + dir_y * dash_end,
                                            );

                                            painter.line_segment(
                                                [dash_start_pos, dash_end_pos],
                                                stroke,
                                            );
                                            t += pattern_length;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Store text for later rendering (on top)
                if self.zoom > 0.3 {
                    texts_to_render.push((
                        macro_rect.center(),
                        macro_def.name.clone(),
                        egui::FontId::default(),
                        egui::Color32::WHITE,
                    ));
                }
            }
        }

        if let Some(def) = &self.def_data {
            // Draw die area outline (if enabled)
            if self.show_diearea && !def.die_area_points.is_empty() {
                if def.die_area_points.len() == 2 {
                    // Handle 2-point rectangle: (x1,y1) (x2,y2) defines a rectangle
                    let p1 = &def.die_area_points[0];
                    let p2 = &def.die_area_points[1];

                    // Convert to screen coordinates (keep Y axis consistent with multi-point)
                    let screen_p1 = egui::pos2(
                        center.x + self.pan_x + (p1.0 as f32 * self.zoom * 0.001),
                        center.y + self.pan_y + (p1.1 as f32 * self.zoom * 0.001), // Same as components
                    );
                    let screen_p2 = egui::pos2(
                        center.x + self.pan_x + (p2.0 as f32 * self.zoom * 0.001),
                        center.y + self.pan_y + (p2.1 as f32 * self.zoom * 0.001), // Same as components
                    );

                    // Create rectangle from min/max of both points
                    let rect = egui::Rect::from_two_pos(screen_p1, screen_p2);

                    // Draw rectangle outline
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(3.0, egui::Color32::RED));

                    // Draw corner markers
                    painter.circle_filled(screen_p1, 3.0, egui::Color32::RED);
                    painter.circle_filled(screen_p2, 3.0, egui::Color32::RED);
                } else {
                    // Handle multi-point polygon: connect all points and close the polygon
                    let mut screen_points: Vec<egui::Pos2> = Vec::new();

                    // Convert all points to screen coordinates (same as components)
                    for point in &def.die_area_points {
                        let screen_point = egui::pos2(
                            center.x + self.pan_x + (point.0 as f32 * self.zoom * 0.001),
                            center.y + self.pan_y + (point.1 as f32 * self.zoom * 0.001), // Same as components
                        );
                        screen_points.push(screen_point);
                    }

                    // First draw a subtle fill
                    if screen_points.len() >= 3 {
                        painter.add(egui::epaint::Shape::convex_polygon(
                            screen_points.clone(),
                            egui::Color32::from_rgba_unmultiplied(255, 0, 0, 15), // Very light red fill
                            egui::Stroke::NONE,
                        ));
                    }

                    // Then draw thick outline lines between consecutive points
                    for i in 0..screen_points.len() {
                        let current = screen_points[i];
                        let next = screen_points[(i + 1) % screen_points.len()];

                        // Draw thick red outline
                        painter.line_segment(
                            [current, next],
                            egui::Stroke::new(4.0, egui::Color32::from_rgb(255, 0, 0)), // Thick red line
                        );
                    }
                }
            }

            // Draw components (if enabled and selected)
            if self.show_components {
                for component in &def.components {
                    // Only draw if this component is selected (or all if none are selected)
                    if !self.selected_components.is_empty()
                        && !self.selected_components.contains(&component.id)
                    {
                        continue;
                    }

                    let comp_x = center.x + self.pan_x + (component.x as f32 * self.zoom * 0.001);
                    let comp_y = center.y + self.pan_y + (component.y as f32 * self.zoom * 0.001);

                    // Draw a small rectangle for each component
                    let comp_size = 5.0 * self.zoom;
                    let comp_rect = egui::Rect::from_center_size(
                        egui::pos2(comp_x, comp_y),
                        egui::vec2(comp_size.max(2.0), comp_size.max(2.0)),
                    );

                    // Use different colors based on selection
                    let is_selected = self.selected_components.contains(&component.id);
                    let fill_color = if is_selected {
                        egui::Color32::from_rgb(0, 255, 150) // Brighter green for selected
                    } else {
                        egui::Color32::from_rgb(0, 200, 100) // Normal green
                    };

                    painter.rect_filled(comp_rect, 0.0, fill_color);
                    painter.rect_stroke(
                        comp_rect,
                        0.0,
                        egui::Stroke::new(1.0, egui::Color32::WHITE),
                    );

                    // Draw component name if zoom is high enough
                    // Store component text for later rendering
                    if self.zoom > 2.0 {
                        texts_to_render.push((
                            egui::pos2(comp_x, comp_y - comp_size - 10.0),
                            component.id.clone(),
                            egui::FontId::monospace(8.0),
                            egui::Color32::YELLOW,
                        ));
                    }
                }
            }

            // Draw pins (if enabled and selected)
            if self.show_pins {
                for pin in &def.pins {
                    // Only draw if this pin is selected (or all if none are selected)
                    if !self.selected_pins.is_empty() && !self.selected_pins.contains(&pin.name) {
                        continue;
                    }

                    let pin_x = center.x + self.pan_x + (pin.x as f32 * self.zoom * 0.001);
                    let pin_y = center.y + self.pan_y + (pin.y as f32 * self.zoom * 0.001);

                    // Draw a small circle for each pin
                    let pin_radius = 3.0 * self.zoom;

                    // Use different colors based on selection and pin type
                    let is_selected = self.selected_pins.contains(&pin.name);
                    let fill_color = if is_selected {
                        egui::Color32::from_rgb(150, 150, 255) // Brighter blue for selected
                    } else {
                        match pin.direction.as_str() {
                            "INPUT" => egui::Color32::from_rgb(100, 255, 100), // Green for input
                            "OUTPUT" => egui::Color32::from_rgb(255, 100, 100), // Red for output
                            "INOUT" => egui::Color32::from_rgb(255, 255, 100), // Yellow for bidirectional
                            _ => egui::Color32::LIGHT_BLUE,                    // Default blue
                        }
                    };

                    painter.circle_filled(
                        egui::pos2(pin_x, pin_y),
                        pin_radius.max(1.0),
                        fill_color,
                    );
                    painter.circle_stroke(
                        egui::pos2(pin_x, pin_y),
                        pin_radius.max(1.0),
                        egui::Stroke::new(1.0, egui::Color32::WHITE),
                    );

                    // Draw pin name with smart positioning if zoom is high enough
                    if self.zoom > 1.0 {
                        // Reduced threshold from 3.0 to 1.0
                        let pin_screen_pos = egui::pos2(pin_x, pin_y);

                        // Calculate edge proximity with 8% threshold
                        let edge_proximity = Self::calculate_pin_edge_proximity(
                            (pin.x as f32, pin.y as f32),
                            &def.die_area_points,
                            self.zoom,
                            center,
                            self.pan_x,
                            self.pan_y,
                            0.08, // 8% threshold ratio
                        );

                        // Calculate DIEAREA screen bounds for positioning calculation
                        let screen_bounds: Vec<egui::Pos2> = def
                            .die_area_points
                            .iter()
                            .map(|(x, y)| {
                                egui::pos2(
                                    center.x + self.pan_x + (*x as f32 * self.zoom * 0.001),
                                    center.y + self.pan_y + (*y as f32 * self.zoom * 0.001),
                                )
                            })
                            .collect();

                        let text_positioning = Self::calculate_smart_text_position(
                            pin_screen_pos,
                            edge_proximity,
                            &screen_bounds,
                        );

                        // Store smart text positioning info for later rendering
                        smart_texts_to_render.push((
                            text_positioning,
                            pin.name.clone(),
                            egui::FontId::monospace(14.0),
                            egui::Color32::WHITE,
                        ));
                    }
                }
            }
        }

        // Render all text on top of everything with outline for white text
        for (pos, text, font, color) in texts_to_render {
            self.render_text_with_outline(
                &painter,
                pos,
                egui::Align2::CENTER_CENTER,
                &text,
                font,
                color,
            );
        }

        // Render smart positioned text using the new rendering system
        for (positioning, text, font, color) in smart_texts_to_render {
            self.render_smart_text_with_outline(&painter, &positioning, &text, font, color);
        }

        ui.ctx().request_repaint();
    }

    fn render_layers_panel(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("Layers");

            if self.lef_data.is_some() {
                ui.label("Toggle layer visibility:");

                // Get all unique layers from the complete list, not just visible ones
                let mut all_layers: Vec<String> = self.all_layers.iter().cloned().collect();
                all_layers.sort();

                // Ensure OUTLINE is always first and LABEL is second
                if let Some(outline_pos) = all_layers.iter().position(|layer| layer == "OUTLINE") {
                    let outline = all_layers.remove(outline_pos);
                    all_layers.insert(0, outline);
                }
                if let Some(label_pos) = all_layers.iter().position(|layer| layer == "LABEL") {
                    let label = all_layers.remove(label_pos);
                    all_layers.insert(1, label);
                }

                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for layer in &all_layers {
                            let mut is_visible = self.visible_layers.contains(layer);

                            // Color indicator using our layer color system
                            let color = self.get_layer_color(layer);

                            ui.horizontal(|ui| {
                                // Color square
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::Vec2::splat(12.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, 2.0, color);

                                if ui.checkbox(&mut is_visible, layer).clicked() {
                                    if is_visible {
                                        self.visible_layers.insert(layer.clone());
                                    } else {
                                        self.visible_layers.remove(layer);
                                    }

                                    // Sync show_pin_text when LABEL layer visibility changes
                                    if layer == "LABEL" {
                                        self.show_pin_text = is_visible;
                                    }
                                }
                            });
                        }
                    });

                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Show All").clicked() {
                        for layer in &all_layers {
                            self.visible_layers.insert(layer.clone());
                        }
                        // Sync show_pin_text when showing all layers
                        self.show_pin_text = true;
                    }
                    if ui.button("Hide All").clicked() {
                        self.visible_layers.clear();
                        // Sync show_pin_text when hiding all layers
                        self.show_pin_text = false;
                    }
                });

                ui.separator();
                ui.label(format!("Total layers: {}", all_layers.len()));
                ui.label(format!("Visible: {}", self.visible_layers.len()));

                // Debug info
                ui.separator();
                ui.label("DEBUG - All layers:");
                for layer in &all_layers {
                    ui.monospace(layer);
                }
            } else {
                ui.label("No LEF file loaded");
            }
        });
    }

    /// Calculate pin proximity to DIEAREA edges
    fn calculate_pin_edge_proximity(
        pin_pos: (f32, f32),
        diearea_bounds: &[(f64, f64)],
        zoom: f32,
        center: egui::Pos2,
        pan_x: f32,
        pan_y: f32,
        threshold_ratio: f32,
    ) -> EdgeProximity {
        if diearea_bounds.is_empty() {
            return EdgeProximity::None;
        }

        // Convert pin to screen coordinates (same as DEF pins)
        let pin_screen_x = center.x + pan_x + (pin_pos.0 as f32 * zoom * 0.001);
        let pin_screen_y = center.y + pan_y + (pin_pos.1 as f32 * zoom * 0.001);

        // Convert DIEAREA to screen coordinates
        let screen_bounds: Vec<egui::Pos2> = diearea_bounds
            .iter()
            .map(|(x, y)| {
                egui::pos2(
                    center.x + pan_x + (*x as f32 * zoom * 0.001),
                    center.y + pan_y + (*y as f32 * zoom * 0.001),
                )
            })
            .collect();

        if screen_bounds.len() < 2 {
            return EdgeProximity::None;
        }

        // Calculate bounding box for threshold calculation
        let mut min_x = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for point in &screen_bounds {
            min_x = min_x.min(point.x);
            max_x = max_x.max(point.x);
            min_y = min_y.min(point.y);
            max_y = max_y.max(point.y);
        }

        let width = max_x - min_x;
        let height = max_y - min_y;
        let threshold = (width.min(height) * threshold_ratio).max(10.0); // Minimum 10 pixels

        if screen_bounds.len() == 2 {
            // Rectangle case: simple distance to edges
            let left_dist = (pin_screen_x - min_x).abs();
            let right_dist = (pin_screen_x - max_x).abs();
            let top_dist = (pin_screen_y - min_y).abs();
            let bottom_dist = (pin_screen_y - max_y).abs();

            let min_dist = left_dist.min(right_dist).min(top_dist).min(bottom_dist);

            if min_dist <= threshold {
                if min_dist == left_dist {
                    EdgeProximity::Left(left_dist)
                } else if min_dist == right_dist {
                    EdgeProximity::Right(right_dist)
                } else if min_dist == top_dist {
                    EdgeProximity::Top(top_dist)
                } else {
                    EdgeProximity::Bottom(bottom_dist)
                }
            } else {
                EdgeProximity::None
            }
        } else {
            // Polygon case: distance to polygon edges
            let pin_point = egui::pos2(pin_screen_x, pin_screen_y);
            let mut min_distance = f32::INFINITY;
            let mut closest_edge_type = EdgeProximity::None;

            for i in 0..screen_bounds.len() {
                let p1 = screen_bounds[i];
                let p2 = screen_bounds[(i + 1) % screen_bounds.len()];

                let dist = Self::point_to_line_distance(pin_point, p1, p2);

                if dist < min_distance && dist <= threshold {
                    min_distance = dist;

                    // Determine edge type based on line orientation and position
                    let line_center_x = (p1.x + p2.x) * 0.5;
                    let line_center_y = (p1.y + p2.y) * 0.5;
                    let dx = (p2.x - p1.x).abs();
                    let dy = (p2.y - p1.y).abs();

                    if dx > dy {
                        // Horizontal-ish line
                        if line_center_y <= min_y + height * 0.3 {
                            closest_edge_type = EdgeProximity::Top(dist);
                        } else if line_center_y >= max_y - height * 0.3 {
                            closest_edge_type = EdgeProximity::Bottom(dist);
                        }
                    } else {
                        // Vertical-ish line
                        if line_center_x <= min_x + width * 0.3 {
                            closest_edge_type = EdgeProximity::Left(dist);
                        } else if line_center_x >= max_x - width * 0.3 {
                            closest_edge_type = EdgeProximity::Right(dist);
                        }
                    }
                }
            }

            closest_edge_type
        }
    }

    /// Calculate distance from point to line segment
    fn point_to_line_distance(
        point: egui::Pos2,
        line_start: egui::Pos2,
        line_end: egui::Pos2,
    ) -> f32 {
        let line_vec = line_end - line_start;
        let point_vec = point - line_start;

        let line_length_sq = line_vec.x * line_vec.x + line_vec.y * line_vec.y;

        if line_length_sq < 1e-6 {
            // Line is essentially a point
            return (point - line_start).length();
        }

        let t = ((point_vec.x * line_vec.x + point_vec.y * line_vec.y) / line_length_sq)
            .clamp(0.0, 1.0);
        let projection = line_start + line_vec * t;

        (point - projection).length()
    }

    /// Calculate smart text positioning based on edge proximity
    fn calculate_smart_text_position(
        pin_screen_pos: egui::Pos2,
        edge_proximity: EdgeProximity,
        diearea_screen_bounds: &[egui::Pos2],
    ) -> TextPositioning {
        match edge_proximity {
            EdgeProximity::Left(_) => {
                // Pin near left edge: place text to the left of pin, right-aligned to edge
                let left_edge_x = diearea_screen_bounds
                    .iter()
                    .map(|p| p.x)
                    .fold(f32::INFINITY, f32::min);
                TextPositioning {
                    pos: egui::pos2(left_edge_x, pin_screen_pos.y),
                    anchor: egui::Align2::RIGHT_CENTER,
                    angle: 0.0,
                }
            }
            EdgeProximity::Right(_) => {
                // Pin near right edge: place text to the right of pin, left-aligned to edge
                let right_edge_x = diearea_screen_bounds
                    .iter()
                    .map(|p| p.x)
                    .fold(f32::NEG_INFINITY, f32::max);
                TextPositioning {
                    pos: egui::pos2(right_edge_x, pin_screen_pos.y),
                    anchor: egui::Align2::LEFT_CENTER,
                    angle: 0.0,
                }
            }
            EdgeProximity::Top(_) => {
                // Pin near top edge: place text above pin, rotated 90° counterclockwise
                // When rotated -90°, text grows upward from the rotation point
                TextPositioning {
                    pos: egui::pos2(pin_screen_pos.x - 10.0, pin_screen_pos.y - 5.0), // More left offset to align with pin center
                    anchor: egui::Align2::LEFT_TOP,
                    angle: -std::f32::consts::FRAC_PI_2, // 90 degrees counterclockwise
                }
            }
            EdgeProximity::Bottom(_) => {
                // Pin near bottom edge: place text below pin, rotated 90° counterclockwise
                // Back to working configuration: LEFT_TOP anchor with large Y offset to ensure text is below pin
                TextPositioning {
                    pos: egui::pos2(pin_screen_pos.x - 10.0, pin_screen_pos.y + 100.0), // Large offset to put text below pin
                    anchor: egui::Align2::LEFT_TOP,
                    angle: -std::f32::consts::FRAC_PI_2, // 90 degrees counterclockwise
                }
            }
            EdgeProximity::None => {
                // Not near any edge: use default positioning
                TextPositioning {
                    pos: egui::pos2(pin_screen_pos.x, pin_screen_pos.y - 8.0),
                    anchor: egui::Align2::CENTER_CENTER,
                    angle: 0.0,
                }
            }
        }
    }

    /// Enhanced text rendering with rotation support and outline
    fn render_smart_text_with_outline(
        &self,
        painter: &egui::Painter,
        positioning: &TextPositioning,
        text: &str,
        font: egui::FontId,
        color: egui::Color32,
    ) {
        // Create TextShape with rotation using egui's API
        let mut text_shape = egui::Shape::text(
            &painter.fonts(|f| f.clone()),
            positioning.pos,
            positioning.anchor,
            text,
            font.clone(),
            color,
        );

        // Apply rotation if needed
        if positioning.angle != 0.0 {
            if let egui::Shape::Text(text_shape) = &mut text_shape {
                text_shape.angle = positioning.angle;
            }
        }

        // Add outline effect for white text by rendering multiple offset copies
        if color == egui::Color32::WHITE {
            let outline_color = egui::Color32::BLACK;
            let outline_offsets = [
                (-1.0, -1.0),
                (0.0, -1.0),
                (1.0, -1.0),
                (-1.0, 0.0),
                (1.0, 0.0),
                (-1.0, 1.0),
                (0.0, 1.0),
                (1.0, 1.0),
            ];

            for (dx, dy) in outline_offsets {
                let outline_pos = egui::pos2(positioning.pos.x + dx, positioning.pos.y + dy);
                let mut outline_shape = egui::Shape::text(
                    &painter.fonts(|f| f.clone()),
                    outline_pos,
                    positioning.anchor,
                    text,
                    font.clone(),
                    outline_color,
                );

                // Apply rotation to outline if needed
                if positioning.angle != 0.0 {
                    if let egui::Shape::Text(outline_shape) = &mut outline_shape {
                        outline_shape.angle = positioning.angle;
                    }
                }
                painter.add(outline_shape);
            }
        }

        // Render main text on top
        painter.add(text_shape);
    }
}

impl eframe::App for LefDefViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(error) = &self.error_message.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.colored_label(egui::Color32::from_rgb(244, 67, 54), error);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.allocate_space(egui::Vec2::new(ui.available_width() / 2.0 - 25.0, 0.0));
                        if ui.button("OK").clicked() {
                            self.error_message = None;
                        }
                    });
                });
        }

        if let Some(success) = &self.success_message.clone() {
            egui::Window::new("Success")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.colored_label(egui::Color32::from_rgb(76, 175, 80), success);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.allocate_space(egui::Vec2::new(ui.available_width() / 2.0 - 25.0, 0.0));
                        if ui.button("OK").clicked() {
                            self.success_message = None;
                        }
                    });
                });
        }

        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.render_menu_bar(ui);
        });

        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                self.render_left_panel(ui);
            });

        if self.show_layers_panel {
            egui::SidePanel::right("layers_panel")
                .resizable(true)
                .default_width(250.0)
                .show(ctx, |ui| {
                    self.render_layers_panel(ui);
                });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("LEF/DEF Visualization");
            self.render_visualization(ui);
        });

        if self.show_lef_details {
            egui::Window::new("LEF Details")
                .resizable(true)
                .default_size([400.0, 300.0])
                .show(ctx, |ui| {
                    if let Some(lef) = &self.lef_data {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                ui.label(format!("Total Macros: {}", lef.macros.len()));
                                ui.separator();
                                for macro_def in &lef.macros {
                                    ui.collapsing(&macro_def.name, |ui| {
                                        ui.monospace(format!("Class: {}", macro_def.class));
                                        ui.monospace(format!("Source: {}", macro_def.source));
                                        ui.monospace(format!("Site: {}", macro_def.site_name));
                                        ui.monospace(format!(
                                            "Origin: ({:.3}, {:.3})",
                                            macro_def.origin_x, macro_def.origin_y
                                        ));
                                        ui.monospace(format!(
                                            "Size: {:.3} x {:.3}",
                                            macro_def.size_x, macro_def.size_y
                                        ));
                                        ui.monospace(format!(
                                            "Foreign: {} ({:.3}, {:.3})",
                                            macro_def.foreign_name,
                                            macro_def.foreign_x,
                                            macro_def.foreign_y
                                        ));
                                        ui.monospace(format!("Pins: {}", macro_def.pins.len()));
                                    });
                                }
                            });
                    } else {
                        ui.label("No LEF data loaded");
                    }
                });
        }

        if self.show_def_details {
            egui::Window::new("DEF Details")
                .resizable(true)
                .default_size([400.0, 300.0])
                .show(ctx, |ui| {
                    if let Some(def) = &self.def_data {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, true])
                            .show(ui, |ui| {
                                ui.label(format!("Die Area Points: {}", def.die_area_points.len()));
                                ui.label(format!("Components: {}", def.components.len()));
                                ui.label(format!("Pins: {}", def.pins.len()));
                                ui.label(format!("Nets: {}", def.nets.len()));
                                ui.separator();

                                if !def.die_area_points.is_empty() {
                                    ui.collapsing("Die Area", |ui| {
                                        for (i, point) in def.die_area_points.iter().enumerate() {
                                            ui.monospace(format!(
                                                "Point {}: ({:.3}, {:.3})",
                                                i, point.0, point.1
                                            ));
                                        }
                                    });
                                }
                            });
                    } else {
                        ui.label("No DEF data loaded");
                    }
                });
        }
    }
}
