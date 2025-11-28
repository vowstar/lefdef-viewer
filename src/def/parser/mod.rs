// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

//! Unified parsing framework for DEF files
//!
//! This module provides a generic, safe framework for parsing multi-line items
//! in DEF files, eliminating infinite loop risks and providing consistent
//! error handling.

pub mod common;
pub mod component;
pub mod net;
pub mod pin;
pub mod specialnet;
pub mod via;

use std::collections::HashMap;
use std::default::Default;
use std::fmt;

/// Result type for parsing operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Error types that can occur during parsing
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ParseError {
    UnexpectedEof,
    InvalidFormat(String),
    InfiniteLoop(String), // Changed to String to support detailed error messages
    UnknownKeyword(String),
    MaxIterationsExceeded(usize),
    NoProgress {
        items_parsed: usize,
        stuck_at_line: usize,
    },
    ItemTooLong {
        item_type: &'static str,
        lines: usize,
        limit: usize,
    },
    Timeout {
        duration: std::time::Duration,
        items_parsed: usize,
    },
}

/// LoopDetector, used to prevent infinite loops
#[derive(Debug)]
#[allow(dead_code)]
pub struct LoopDetector {
    line_counts: HashMap<String, usize>,
    max_repeats: usize,
}

impl LoopDetector {
    /// Create a new loop detector with specified maximum repeats
    pub fn new(max_repeats: usize) -> Self {
        Self {
            line_counts: HashMap::new(),
            max_repeats,
        }
    }

    /// Check if a line has been processed too many times (potential infinite loop)
    #[allow(dead_code)]
    pub fn check_infinite_loop(&mut self, line_index: &str) -> bool {
        let count = self.line_counts.entry(line_index.to_string()).or_insert(0);
        *count += 1;
        *count > self.max_repeats
    }
}

impl Default for LoopDetector {
    fn default() -> Self {
        Self::new(10)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedEof => write!(f, "Unexpected end of file"),
            ParseError::InvalidFormat(msg) => write!(f, "Invalid format: {msg}"),
            ParseError::InfiniteLoop(msg) => write!(f, "Infinite loop detected: {msg}"),
            ParseError::UnknownKeyword(kw) => write!(f, "Unknown keyword: {kw}"),
            ParseError::MaxIterationsExceeded(max) => {
                write!(f, "Maximum iterations exceeded: {max}")
            }
            ParseError::NoProgress {
                items_parsed,
                stuck_at_line,
            } => {
                write!(
                    f,
                    "No progress: parsed {items_parsed} items, stuck at line {stuck_at_line}"
                )
            }
            ParseError::ItemTooLong {
                item_type,
                lines,
                limit,
            } => {
                write!(
                    f,
                    "Item too long: {item_type} has {lines} lines, limit is {limit}"
                )
            }
            ParseError::Timeout {
                duration,
                items_parsed,
            } => {
                write!(
                    f,
                    "Timeout after {:.2}s: parsed {} items",
                    duration.as_secs_f64(),
                    items_parsed
                )
            }
        }
    }
}

/// Result of handling a continuation line
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
pub enum ContinuationResult {
    /// Continue processing more lines
    Continue,
    /// Current item is complete, move to next
    Complete,
    /// Hit the start of next item, should backtrack
    NextItem,
    /// Error occurred
    Error(String),
}

/// Context for DEF parsing operations
#[derive(Debug)]
#[allow(dead_code)]
pub struct ParseContext {
    pub current_line: usize,
    pub section_name: String,
    pub item_count: usize,
    pub debug_info: Vec<String>,
    pub processed_lines: std::collections::HashSet<usize>,
}

#[allow(dead_code)]
impl ParseContext {
    pub fn new(section_name: String, start_line: usize) -> Self {
        Self {
            current_line: start_line,
            section_name,
            item_count: 0,
            debug_info: Vec::new(),
            processed_lines: std::collections::HashSet::new(),
        }
    }

    pub fn add_debug(&mut self, message: String) {
        self.debug_info
            .push(format!("Line {}: {}", self.current_line, message));
    }

    pub fn check_infinite_loop(&mut self, line_index: usize) -> ParseResult<()> {
        if self.processed_lines.contains(&line_index) {
            return Err(ParseError::InfiniteLoop(format!(
                "Line {line_index} repeated"
            )));
        }
        self.processed_lines.insert(line_index);
        Ok(())
    }
}

/// Generic trait for parsing specific item types in DEF files
pub trait DefItemParser {
    /// The type of item this parser produces
    type Item;
    /// The context type used during parsing
    type Context;

    /// Parse the header line of an item (e.g., "- PINNAME + NET ...")
    fn parse_header(&self, line: &str) -> Option<Self::Context>;

    /// Process a continuation line for the current item
    fn parse_continuation(&self, context: &mut Self::Context, line: &str) -> ContinuationResult;

    /// Finalize the item from the accumulated context
    fn finalize(&self, context: Self::Context) -> ParseResult<Self::Item>;

    /// Get the name of this item type for debugging
    fn item_name() -> &'static str;
}

/// Enhanced universal multi-line parser engine
pub struct MultiLineParser<P: DefItemParser> {
    parser: P,
    #[allow(dead_code)]
    max_iterations: usize,
    debug_mode: bool,
    #[allow(dead_code)]
    timeout_duration: std::time::Duration,
    #[allow(dead_code)]
    max_repeated_line_count: usize,
    #[allow(dead_code)]
    max_lines_per_item: usize,
}

impl<P: DefItemParser> MultiLineParser<P> {
    pub fn new(parser: P) -> Self {
        Self {
            parser,
            max_iterations: 50000, // Higher default for large files
            debug_mode: false,
            timeout_duration: std::time::Duration::from_secs(120), // 2 minutes default
            max_repeated_line_count: 10,                           // Detect line repetition
            max_lines_per_item: 2000,                              // Single item limit
        }
    }

    /// Create parser with preprocessed mode enabled (recommended)
    pub fn with_preprocessed(parser: P) -> Self {
        Self::new(parser)
    }

    #[allow(dead_code)]
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug_mode = debug;
        self
    }

    #[allow(dead_code)]
    pub fn with_timeout(mut self, duration: std::time::Duration) -> Self {
        self.timeout_duration = duration;
        self
    }

    #[allow(dead_code)]
    pub fn with_max_repeated_lines(mut self, count: usize) -> Self {
        self.max_repeated_line_count = count;
        self
    }

    #[allow(dead_code)]
    pub fn with_max_lines_per_item(mut self, lines: usize) -> Self {
        self.max_lines_per_item = lines;
        self
    }

    /// Parse a section containing multiple items with enhanced loop detection
    ///
    /// NOTE: This method expects raw (non-preprocessed) lines.
    /// For better handling of multi-line statements, use parse_section_preprocessed instead.
    #[allow(dead_code)]
    pub fn parse_section(
        &self,
        lines: &[&str],
        start_index: usize,
        end_pattern: &str,
    ) -> ParseResult<(Vec<P::Item>, usize)> {
        let mut items = Vec::new();
        let mut context = ParseContext::new(P::item_name().to_string(), start_index);
        let mut i = start_index;
        let mut iterations = 0;
        let start_time = std::time::Instant::now();
        let mut last_progress_line = start_index;
        let mut repeated_line_count = 0;

        if self.debug_mode {
            println!(
                "[DBG] Starting {} section parsing at line {}",
                P::item_name(),
                start_index
            );
        }

        while i < lines.len() && iterations < self.max_iterations {
            iterations += 1;
            context.current_line = i;

            // Check timeout
            if start_time.elapsed() > self.timeout_duration {
                return Err(ParseError::Timeout {
                    duration: start_time.elapsed(),
                    items_parsed: items.len(),
                });
            }

            // Check for repeated line processing
            if i == last_progress_line {
                repeated_line_count += 1;
                if repeated_line_count > self.max_repeated_line_count {
                    return Err(ParseError::NoProgress {
                        items_parsed: items.len(),
                        stuck_at_line: i,
                    });
                }
            } else {
                last_progress_line = i;
                repeated_line_count = 0;
            }

            let line = lines[i].trim();

            // Check for section end
            if line.starts_with(end_pattern) {
                if self.debug_mode {
                    println!("[DBG] Found section end: {end_pattern}");
                }
                break;
            }

            // Skip empty lines
            if line.is_empty() {
                i += 1;
                continue;
            }

            // Try to parse as new item header
            if let Some(item_context) = self.parser.parse_header(line) {
                context.item_count += 1;
                if self.debug_mode {
                    println!(
                        "[DBG] Parsing {} #{}: {}",
                        P::item_name(),
                        context.item_count,
                        line
                    );
                }

                let (item, next_index) = self.parse_single_item(lines, i, item_context)?;
                items.push(item);
                i = next_index;
            } else {
                // Not a valid item header, skip line
                if self.debug_mode {
                    println!("[DBG] Skipping non-item line: {line}");
                }
                i += 1;
            }
        }

        if iterations >= self.max_iterations {
            return Err(ParseError::MaxIterationsExceeded(self.max_iterations));
        }

        if self.debug_mode {
            println!(
                "[DBG] Completed {} section: {} items parsed",
                P::item_name(),
                items.len()
            );
        }

        Ok((items, i))
    }

    /// Parse preprocessed lines (recommended method for multi-line statement support)
    ///
    /// This method works with preprocessed logical lines where:
    /// - Comments are already removed
    /// - Multi-line statements are merged until semicolon
    /// - Each line represents a complete logical statement
    pub fn parse_section_preprocessed(
        &self,
        lines: &[String],
        start_index: usize,
        end_pattern: &str,
    ) -> ParseResult<(Vec<P::Item>, usize)> {
        let line_refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        self.parse_section_preprocessed_refs(&line_refs, start_index, end_pattern)
    }

    /// Parse preprocessed lines using string references
    pub fn parse_section_preprocessed_refs(
        &self,
        lines: &[&str],
        start_index: usize,
        end_pattern: &str,
    ) -> ParseResult<(Vec<P::Item>, usize)> {
        let mut items = Vec::new();
        let mut i = start_index;

        if self.debug_mode {
            println!(
                "[DBG] Starting {} section parsing (preprocessed) at line {}",
                P::item_name(),
                start_index
            );
        }

        while i < lines.len() {
            let line = lines[i].trim();

            // Check for section end
            if line.starts_with(end_pattern) {
                if self.debug_mode {
                    println!("[DBG] Found section end: {end_pattern}");
                }
                break;
            }

            // Skip empty lines
            if line.is_empty() {
                i += 1;
                continue;
            }

            // Try to parse as new item
            if let Some(mut item_context) = self.parser.parse_header(line) {
                if self.debug_mode {
                    println!(
                        "[DBG] Parsing {} #{}: {}",
                        P::item_name(),
                        items.len() + 1,
                        line
                    );
                }

                // For preprocessed lines, each line is complete
                // Parse the entire line as continuation to extract all attributes
                let result = self.parser.parse_continuation(&mut item_context, line);

                match result {
                    ContinuationResult::Complete | ContinuationResult::Continue => {
                        // Finalize and add item
                        items.push(self.parser.finalize(item_context)?);
                        i += 1;
                    }
                    ContinuationResult::NextItem => {
                        // Should not happen with preprocessed lines
                        items.push(self.parser.finalize(item_context)?);
                        i += 1;
                    }
                    ContinuationResult::Error(msg) => {
                        return Err(ParseError::InvalidFormat(msg));
                    }
                }
            } else {
                // Not a valid item header, skip line
                if self.debug_mode {
                    println!("[DBG] Skipping non-item line: {line}");
                }
                i += 1;
            }
        }

        if self.debug_mode {
            println!(
                "[DBG] Completed {} section: {} items parsed",
                P::item_name(),
                items.len()
            );
        }

        Ok((items, i))
    }

    /// Parse a single item starting from the header line with length limits
    #[allow(dead_code)]
    fn parse_single_item(
        &self,
        lines: &[&str],
        start_index: usize,
        mut item_context: P::Context,
    ) -> ParseResult<(P::Item, usize)> {
        let mut i = start_index + 1; // Start from next line after header
        let mut iterations = 0;
        let header_line = lines[start_index].trim();

        // Check if header line already contains semicolon (complete in one line)
        if header_line.contains(';') {
            if self.debug_mode {
                println!("[DBG]   Single-line item detected");
            }
            // For single-line definitions, parse the complete line first
            let result = self
                .parser
                .parse_continuation(&mut item_context, header_line);
            match result {
                ContinuationResult::Complete => {
                    return Ok((self.parser.finalize(item_context)?, start_index + 1));
                }
                _ => {
                    // If parse_continuation didn't mark as complete, finalize anyway
                    return Ok((self.parser.finalize(item_context)?, start_index + 1));
                }
            }
        }

        while i < lines.len() && iterations < self.max_iterations {
            iterations += 1;

            // Check if single item is becoming too long
            let lines_processed = i - start_index;
            if lines_processed > self.max_lines_per_item {
                return Err(ParseError::ItemTooLong {
                    item_type: P::item_name(),
                    lines: lines_processed,
                    limit: self.max_lines_per_item,
                });
            }

            let line = lines[i].trim();

            if self.debug_mode {
                println!("[DBG]   Processing continuation: {line}");
            }

            match self.parser.parse_continuation(&mut item_context, line) {
                ContinuationResult::Continue => {
                    i += 1;
                }
                ContinuationResult::Complete => {
                    if self.debug_mode {
                        println!("[DBG]   Item completed");
                    }
                    return Ok((self.parser.finalize(item_context)?, i + 1));
                }
                ContinuationResult::NextItem => {
                    if self.debug_mode {
                        println!("[DBG]   Hit next item, backtracking");
                    }
                    return Ok((self.parser.finalize(item_context)?, i));
                }
                ContinuationResult::Error(msg) => {
                    return Err(ParseError::InvalidFormat(msg));
                }
            }
        }

        if iterations >= self.max_iterations {
            return Err(ParseError::MaxIterationsExceeded(self.max_iterations));
        }

        // Reached end of file
        Ok((self.parser.finalize(item_context)?, i))
    }
}
