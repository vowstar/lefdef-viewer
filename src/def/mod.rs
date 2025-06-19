// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefRect {
    pub layer: String,
    pub xl: f64,
    pub yl: f64,
    pub xh: f64,
    pub yh: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefPort {
    pub rects: Vec<DefRect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefPin {
    pub name: String,
    pub net: String,
    pub use_type: String,
    pub status: String,
    pub direction: String,
    pub orient: String,
    pub x: f64,
    pub y: f64,
    pub rects: Vec<DefRect>,
    pub ports: Vec<DefPort>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefNet {
    pub name: String,
    pub connections: usize,
    pub pins: usize,
    pub use_type: String,
    pub weight: Option<f64>,
    pub source: String,
    pub pattern: String,
    pub shielded: bool,
    pub instances: Vec<String>,
    pub instance_pins: Vec<String>,
    pub routing: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefPlacement {
    pub placement_type: String, // PLACED, FIXED, COVER, UNPLACED
    pub x: f64,
    pub y: f64,
    pub orientation: String, // N, S, E, W, FN, FS, FE, FW
}

// Alias for component placement to maintain compatibility
pub type DefComponentPlacement = DefPlacement;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefComponent {
    pub name: String,
    pub macro_name: String,
    pub placement: Option<DefPlacement>,
    pub routing_halo: Option<(f64, f64, f64, f64)>, // left, bottom, right, top
    pub source: Option<String>,
    pub weight: Option<f64>,
    pub eeq: Option<String>,
    pub generate: Option<String>,
    pub power: Option<f64>,
    pub ground: Option<String>,
    pub properties: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefGCellGrid {
    pub offset: f64,
    pub count: usize,
    pub step: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefTrack {
    pub layer: String,
    pub offset: f64,
    pub num: i32,
    pub step: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefRow {
    pub name: String,
    pub macro_name: String,
    pub x: f64,
    pub y: f64,
    pub num_x: i32,
    pub num_y: i32,
    pub step_x: f64,
    pub step_y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefPolygon {
    pub points: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefViaLayer {
    pub layer_name: String,
    pub mask: Option<i32>,
    pub rects: Vec<DefRect>,
    pub polygons: Vec<DefPolygon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefVia {
    pub name: String,
    pub layers: Vec<DefViaLayer>,
    pub via_rule: Option<String>,
    pub cut_size: Option<(f64, f64)>,
    pub cut_spacing: Option<(f64, f64)>,
    pub enclosure: Vec<(String, f64, f64)>,
    pub pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Def {
    pub die_area_points: Vec<(f64, f64)>,
    pub g_cell_grid_x: Vec<DefGCellGrid>,
    pub g_cell_grid_y: Vec<DefGCellGrid>,
    pub pins: Vec<DefPin>,
    pub nets: Vec<DefNet>,
    pub components: Vec<DefComponent>,
    pub rows: Vec<DefRow>,
    pub tracks_x: Vec<DefTrack>,
    pub tracks_y: Vec<DefTrack>,
    pub vias: Vec<DefVia>,
}

pub mod def_parser;
pub mod parser;
pub mod reader;
