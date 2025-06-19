# LEF/DEF Viewer (Rust)

[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/vowstar/lefdef-viewer/actions/workflows/ci.yml/badge.svg)](https://github.com/vowstar/lefdef-viewer/actions/workflows/ci.yml)

A Rust-based LEF (Library Exchange Format) and DEF (Design Exchange Format) file viewer with a modern GUI built using egui.

## Features

- **LEF File Support**: Parse and visualize LEF files containing macro definitions, pins, and layout information
- **DEF File Support**: Parse and visualize DEF files with die area, components, nets, and routing information
- **Interactive GUI**: Modern interface with file browsing, zoom/pan controls, and detailed data inspection
- **Real-time Visualization**: Dynamic rendering of layout elements with proper scaling and positioning
- **Cross-platform**: Built with Rust and egui for Windows, macOS, and Linux support
- **Static Binary**: Linux version available with musl static linking for portable deployment

## Installation

### Prerequisites

- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))

### Building from Source

```bash
git clone <repository-url>
cd lefdef-viewer
cargo build --release
```

### Building Static Binary (Linux)

For a fully static binary on Linux that doesn't depend on system libraries:

```bash
# Install musl target
rustup target add x86_64-unknown-linux-musl

# Install musl tools (on Debian/Ubuntu)
sudo apt-get install musl-tools

# Build with musl target
cargo build --release --target x86_64-unknown-linux-musl
```

The resulting binary will be at `target/x86_64-unknown-linux-musl/release/lefdef-viewer`.

### Running

```bash
cargo run --release
```

## Usage

1. **Open Files**: Use File → Open LEF File or Open DEF File to load your files
2. **Navigate**: Use mouse drag to pan around the visualization
3. **Zoom**: Use the zoom slider in the left panel or mouse wheel
4. **View Details**: Enable "Show LEF Details" or "Show DEF Details" from the View menu for detailed information
5. **Reset View**: Click "Reset View" to return to the original zoom and pan settings

## Architecture

The project is structured into several modules:

- `lef/`: LEF file parsing and data structures
  - `mod.rs`: Core LEF data structures (LefMacro, LefPin, LefRect, etc.)
  - `parser.rs`: Nom-based parser for LEF files
  - `reader.rs`: High-level reader interface
- `def/`: DEF file parsing and data structures
  - `mod.rs`: Core DEF data structures (DefComponent, DefNet, DefPin, etc.)
  - `parser.rs`: Nom-based parser for DEF files
  - `reader.rs`: High-level reader interface
- `gui.rs`: egui-based graphical user interface
- `main.rs`: Application entry point

## Supported Features

### LEF Files

- MACRO definitions with class, source, and site information
- PIN definitions with direction, use, and shape
- PORT definitions with layer rectangles
- OBS (obstruction) definitions
- Size and origin information

### DEF Files

- DIEAREA definitions
- GCELLGRID definitions (X and Y)
- Component placement information
- Pin definitions and locations
- Net connectivity (basic parsing)
- Row and track definitions

## Dependencies

- `egui`: Immediate mode GUI framework
- `eframe`: Application framework for egui
- `nom`: Parser combinator library for file parsing
- `serde`: Serialization framework
- `rfd`: Native file dialog
- `log` & `env_logger`: Logging support

## License

Licensed under either of

- MIT License ([LICENSE-MIT](LICENSE))

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Roadmap

- [ ] Enhanced DEF parsing (complete COMPONENTS, NETS sections)
- [x] Layer management and visualization
- [ ] Export functionality (PNG, SVG)
- [ ] Advanced measurement tools
- [ ] Design rule checking visualization
- [ ] Technology file support
- [ ] Performance optimizations for large files
