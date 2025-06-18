// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LefRect {
    pub layer: String,
    pub xl: f64,
    pub yl: f64,
    pub xh: f64,
    pub yh: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LefPolygon {
    pub layer: String,
    pub points: Vec<(f64, f64)>,
    pub is_hole: bool, // true if clockwise (subtractive/hole), false if counterclockwise (additive)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LefPort {
    pub rects: Vec<LefRect>,
    pub polygons: Vec<LefPolygon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LefPin {
    pub name: String,
    pub direction: String,
    pub use_type: String,
    pub shape: String,
    pub ports: Vec<LefPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LefObstruction {
    pub rects: Vec<LefRect>,
    pub polygons: Vec<LefPolygon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LefMacro {
    pub name: String,
    pub class: String,
    pub source: String,
    pub site_name: String,
    pub origin_x: f64,
    pub origin_y: f64,
    pub size_x: f64,
    pub size_y: f64,
    pub foreign_name: String,
    pub foreign_x: f64,
    pub foreign_y: f64,
    pub pins: Vec<LefPin>,
    pub obstruction: Option<LefObstruction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lef {
    pub macros: Vec<LefMacro>,
}

pub mod parser;
pub mod reader;
