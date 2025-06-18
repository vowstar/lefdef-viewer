// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use eframe::egui;
use egui::epaint::{PathShape, PathStroke};
use geo::{Coord, LineString, Polygon as GeoPolygon};
use rfd::FileDialog;

use crate::def::{reader::DefReader, Def};
use crate::lef::{reader::LefReader, Lef};

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
    selected_cells: std::collections::HashSet<String>,
    visible_layers: std::collections::HashSet<String>,
    all_layers: std::collections::HashSet<String>,
    show_layers_panel: bool,
    show_pin_text: bool,
    fit_to_view_requested: bool,
}

impl LefDefViewer {
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            show_layers_panel: true,
            show_pin_text: true,
            fit_to_view_requested: false,
            ..Default::default()
        }
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
            // Draw die area outline
            for (i, point) in def.die_area_points.iter().enumerate() {
                if i > 0 {
                    let prev_point = &def.die_area_points[i - 1];
                    let start = egui::pos2(
                        center.x + self.pan_x + (prev_point.0 as f32 * self.zoom * 0.001), // Scale down from microns
                        center.y + self.pan_y + (prev_point.1 as f32 * self.zoom * 0.001),
                    );
                    let end = egui::pos2(
                        center.x + self.pan_x + (point.0 as f32 * self.zoom * 0.001),
                        center.y + self.pan_y + (point.1 as f32 * self.zoom * 0.001),
                    );
                    painter.line_segment([start, end], egui::Stroke::new(3.0, egui::Color32::RED));
                }
            }

            if !def.die_area_points.is_empty() && def.die_area_points.len() > 2 {
                let first = &def.die_area_points[0];
                let last = &def.die_area_points[def.die_area_points.len() - 1];
                let start = egui::pos2(
                    center.x + self.pan_x + (last.0 as f32 * self.zoom * 0.001),
                    center.y + self.pan_y + (last.1 as f32 * self.zoom * 0.001),
                );
                let end = egui::pos2(
                    center.x + self.pan_x + (first.0 as f32 * self.zoom * 0.001),
                    center.y + self.pan_y + (first.1 as f32 * self.zoom * 0.001),
                );
                painter.line_segment([start, end], egui::Stroke::new(3.0, egui::Color32::RED));
            }

            // Draw components
            for component in &def.components {
                let comp_x = center.x + self.pan_x + (component.x as f32 * self.zoom * 0.001);
                let comp_y = center.y + self.pan_y + (component.y as f32 * self.zoom * 0.001);

                // Draw a small rectangle for each component
                let comp_size = 5.0 * self.zoom;
                let comp_rect = egui::Rect::from_center_size(
                    egui::pos2(comp_x, comp_y),
                    egui::vec2(comp_size.max(2.0), comp_size.max(2.0)),
                );

                painter.rect_filled(comp_rect, 0.0, egui::Color32::from_rgb(0, 200, 100));
                painter.rect_stroke(comp_rect, 0.0, egui::Stroke::new(1.0, egui::Color32::WHITE));

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

            // Draw pins
            for pin in &def.pins {
                let pin_x = center.x + self.pan_x + (pin.x as f32 * self.zoom * 0.001);
                let pin_y = center.y + self.pan_y + (pin.y as f32 * self.zoom * 0.001);

                // Draw a small circle for each pin
                let pin_radius = 3.0 * self.zoom;
                painter.circle_filled(
                    egui::pos2(pin_x, pin_y),
                    pin_radius.max(1.0),
                    egui::Color32::LIGHT_BLUE,
                );
            }
        }

        // Render all text on top of everything
        for (pos, text, font, color) in texts_to_render {
            painter.text(pos, egui::Align2::CENTER_CENTER, &text, font, color);
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
}

impl eframe::App for LefDefViewer {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(error) = &self.error_message.clone() {
            egui::Window::new("Error")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(error);
                    if ui.button("OK").clicked() {
                        self.error_message = None;
                    }
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
