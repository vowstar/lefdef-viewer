// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

//! LEF (Library Exchange Format) file parsing and data structures
//!
//! This module provides comprehensive LEF file parsing using proven nom-based parser
//! that supports all LEF features including multi-line POLYGON definitions.

pub mod lef_parser;
pub mod reader;

pub use reader::LefReader;

/// Main LEF file structure
#[derive(Debug, Clone)]
pub struct Lef {
    pub macros: Vec<LefMacro>,
}

/// LEF MACRO definition
#[derive(Debug, Clone)]
pub struct LefMacro {
    pub name: String,
    pub class: String,
    pub foreign: String,
    pub origin: (f64, f64),
    pub size_x: f64,
    pub size_y: f64,
    pub symmetry: Vec<String>,
    pub site: String,
    pub pins: Vec<LefPin>,
    pub obs: Vec<LefObstruction>,
}

/// LEF PIN definition with complete geometry support
#[derive(Debug, Clone)]
pub struct LefPin {
    pub name: String,
    pub direction: String,
    pub use_type: String,
    pub shape: String,
    pub ports: Vec<LefPort>,
}

/// LEF PORT containing geometric shapes
#[derive(Debug, Clone)]
pub struct LefPort {
    pub rects: Vec<LefRect>,
    pub polygons: Vec<LefPolygon>,
}

/// LEF RECT geometry
#[derive(Debug, Clone)]
pub struct LefRect {
    pub layer: String,
    pub xl: f64,
    pub yl: f64,
    pub xh: f64,
    pub yh: f64,
}

/// LEF POLYGON geometry with multi-line support
#[derive(Debug, Clone)]
pub struct LefPolygon {
    pub layer: String,
    pub points: Vec<(f64, f64)>,
    pub is_hole: bool,
}

/// LEF OBSTRUCTION (OBS)
#[derive(Debug, Clone)]
pub struct LefObstruction {
    pub rects: Vec<LefRect>,
    pub polygons: Vec<LefPolygon>,
}
