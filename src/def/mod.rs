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
    pub instances: Vec<String>,
    pub pins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefComponent {
    pub id: String,
    pub name: String,
    pub status: String,
    pub source: String,
    pub orient: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefGCellGrid {
    pub offset: f64,
    pub num: i32,
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

pub mod parser;
pub mod reader;
