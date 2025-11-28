// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 Huang Rui <vowstar@gmail.com>

//! DEF/LEF Preprocessor
//!
//! Pass 1: Preprocesses DEF/LEF files by:
//! - Removing comments (# character when preceded by whitespace)
//! - Merging logical lines (statements that end with semicolon)
//! - Preserving line number mappings for error reporting

use std::fmt;

/// Mapping between logical lines and original file lines
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LineMapping {
    pub logical_line: usize,   // Index in preprocessed lines
    pub original_start: usize, // Starting line in original file (0-indexed)
    pub original_end: usize,   // Ending line in original file (0-indexed)
}

impl LineMapping {
    pub fn new(logical_line: usize, original_start: usize, original_end: usize) -> Self {
        Self {
            logical_line,
            original_start,
            original_end,
        }
    }
}

/// Preprocessed DEF/LEF content with line mappings
#[derive(Debug)]
pub struct PreprocessedDef {
    pub lines: Vec<String>,         // Logical lines (merged until semicolon)
    pub mappings: Vec<LineMapping>, // Mapping to original file lines
}

impl PreprocessedDef {
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            mappings: Vec::new(),
        }
    }

    /// Get original line range for a logical line index
    #[allow(dead_code)]
    pub fn get_original_range(&self, logical_line: usize) -> Option<(usize, usize)> {
        self.mappings
            .iter()
            .find(|m| m.logical_line == logical_line)
            .map(|m| (m.original_start, m.original_end))
    }

    /// Format error message with original line number
    #[allow(dead_code)]
    pub fn format_error(&self, logical_line: usize, message: &str) -> String {
        if let Some((start, end)) = self.get_original_range(logical_line) {
            if start == end {
                format!("Line {}: {}", start + 1, message)
            } else {
                format!("Lines {}-{}: {}", start + 1, end + 1, message)
            }
        } else {
            format!("Line {}: {}", logical_line + 1, message)
        }
    }
}

impl Default for PreprocessedDef {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PreprocessedDef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Preprocessed DEF/LEF:")?;
        writeln!(f, "  Logical lines: {}", self.lines.len())?;
        writeln!(f, "  Mappings: {}", self.mappings.len())?;
        Ok(())
    }
}

/// Preprocess DEF/LEF file content
///
/// # Arguments
/// * `content` - Raw file content as string
///
/// # Returns
/// * `PreprocessedDef` - Preprocessed content with line mappings
///
/// # Processing steps:
/// 1. Remove comments (# when preceded by whitespace/start of line)
/// 2. Merge lines until semicolon (logical line)
/// 3. Record mapping between logical and original lines
pub fn preprocess(content: &str) -> PreprocessedDef {
    let raw_lines: Vec<&str> = content.lines().collect();
    let mut result = PreprocessedDef::new();

    let mut current_line = String::new();
    let mut line_start: usize = 0;

    for (i, raw) in raw_lines.iter().enumerate() {
        // Step 1: Remove comment (everything after # when # is preceded by whitespace)
        let without_comment = remove_comment(raw);
        let trimmed = without_comment.trim();

        // Skip completely empty lines
        if trimmed.is_empty() {
            // If we have accumulated content without semicolon, finalize it
            // This handles END statements and other non-semicolon lines
            if !current_line.is_empty() {
                let logical_index = result.lines.len();
                result.lines.push(current_line.clone());
                result
                    .mappings
                    .push(LineMapping::new(logical_index, line_start, i - 1));
                current_line.clear();
            }
            continue;
        }

        // Step 2: Mark start of new logical line
        if current_line.is_empty() {
            line_start = i;
        } else {
            // Add space between merged lines
            current_line.push(' ');
        }

        // Append content to current logical line
        current_line.push_str(trimmed);

        // Step 3: Check if logical line is complete
        // Complete on: semicolon OR standalone END/DESIGN statements
        let is_end_statement = trimmed.starts_with("END ")
            || trimmed == "END"
            || trimmed.starts_with("DESIGN ")
            || (trimmed == "DESIGN" && current_line.trim() == "DESIGN");

        if trimmed.contains(';') || is_end_statement {
            let logical_index = result.lines.len();
            result.lines.push(current_line.clone());
            result
                .mappings
                .push(LineMapping::new(logical_index, line_start, i));
            current_line.clear();
        }
    }

    // Handle incomplete logical line at end of file
    if !current_line.trim().is_empty() {
        let logical_index = result.lines.len();
        result.lines.push(current_line);
        result.mappings.push(LineMapping::new(
            logical_index,
            line_start,
            raw_lines.len().saturating_sub(1),
        ));
    }

    result
}

/// Remove comment from a line
///
/// Comments start with # when preceded by:
/// - Start of line
/// - Space
/// - Tab
///
/// # Arguments
/// * `line` - Input line
///
/// # Returns
/// * Line with comment removed
fn remove_comment(line: &str) -> &str {
    if let Some(pos) = find_comment_start(line) {
        &line[..pos]
    } else {
        line
    }
}

/// Find the position where a comment starts
///
/// # Arguments
/// * `line` - Input line
///
/// # Returns
/// * `Some(pos)` - Position of # that starts a comment
/// * `None` - No comment found
fn find_comment_start(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'#' {
            // # at start of line or preceded by space/tab
            if i == 0 || bytes[i - 1] == b' ' || bytes[i - 1] == b'\t' {
                return Some(i);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_line_statement() {
        let input = "VERSION 5.8 ;";
        let result = preprocess(input);
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0], "VERSION 5.8 ;");
        assert_eq!(result.mappings[0].original_start, 0);
        assert_eq!(result.mappings[0].original_end, 0);
    }

    #[test]
    fn test_two_line_statement() {
        let input = "- COMP MACRO + FIXED ( 100 200 ) N\n ;";
        let result = preprocess(input);
        assert_eq!(result.lines.len(), 1);
        assert!(result.lines[0].contains("COMP"));
        assert!(result.lines[0].contains("FIXED"));
        assert!(result.lines[0].ends_with(";"));
        assert_eq!(result.mappings[0].original_start, 0);
        assert_eq!(result.mappings[0].original_end, 1);
    }

    #[test]
    fn test_multi_line_statement() {
        let input = "- COMP MACRO\n + FIXED ( 100 200 ) N\n + SOURCE DIST\n ;";
        let result = preprocess(input);
        assert_eq!(result.lines.len(), 1);
        assert!(result.lines[0].contains("COMP"));
        assert!(result.lines[0].contains("FIXED"));
        assert!(result.lines[0].contains("SOURCE"));
        assert_eq!(result.mappings[0].original_start, 0);
        assert_eq!(result.mappings[0].original_end, 3);
    }

    #[test]
    fn test_comment_removal() {
        let input = "VERSION 5.8 ; # this is a comment";
        let result = preprocess(input);
        assert!(!result.lines[0].contains("#"));
        assert!(!result.lines[0].contains("comment"));
        assert_eq!(result.lines[0], "VERSION 5.8 ;");
    }

    #[test]
    fn test_comment_at_start() {
        let input = "# This is a comment\nVERSION 5.8 ;";
        let result = preprocess(input);
        assert_eq!(result.lines.len(), 1);
        assert_eq!(result.lines[0], "VERSION 5.8 ;");
    }

    #[test]
    fn test_multiple_statements() {
        let input = "VERSION 5.8 ;\nDESIGN test ;";
        let result = preprocess(input);
        assert_eq!(result.lines.len(), 2);
        assert_eq!(result.lines[0], "VERSION 5.8 ;");
        assert_eq!(result.lines[1], "DESIGN test ;");
        assert_eq!(result.mappings[0].original_start, 0);
        assert_eq!(result.mappings[0].original_end, 0);
        assert_eq!(result.mappings[1].original_start, 1);
        assert_eq!(result.mappings[1].original_end, 1);
    }

    #[test]
    fn test_mixed_format() {
        let input = "# Header comment\nVERSION 5.8 ;\n- C1 M1 + FIXED ( 100 200 ) N\n ;\n- C2 M2 + PLACED ( 300 400 ) S ;";
        let result = preprocess(input);
        // VERSION + 2 components = 3 statements
        assert_eq!(result.lines.len(), 3);
        assert_eq!(result.lines[0], "VERSION 5.8 ;");
        assert!(result.lines[1].contains("C1"));
        assert!(result.lines[2].contains("C2"));
    }

    #[test]
    fn test_hash_in_identifier() {
        // # not preceded by space, should NOT be treated as comment
        let input = "- net#123 PIN ;";
        let result = preprocess(input);
        assert_eq!(result.lines.len(), 1);
        assert!(result.lines[0].contains("net#123"));
    }

    #[test]
    fn test_empty_lines() {
        let input = "VERSION 5.8 ;\n\n\nDESIGN test ;";
        let result = preprocess(input);
        assert_eq!(result.lines.len(), 2);
        assert_eq!(result.lines[0], "VERSION 5.8 ;");
        assert_eq!(result.lines[1], "DESIGN test ;");
    }

    #[test]
    fn test_line_mapping_single() {
        let input = "VERSION 5.8 ;";
        let result = preprocess(input);
        assert_eq!(result.get_original_range(0), Some((0, 0)));
    }

    #[test]
    fn test_line_mapping_multi() {
        let input = "- COMP MACRO\n + FIXED ( 100 200 ) N\n ;";
        let result = preprocess(input);
        assert_eq!(result.get_original_range(0), Some((0, 2)));
    }

    #[test]
    fn test_format_error_single_line() {
        let input = "VERSION 5.8 ;";
        let result = preprocess(input);
        let msg = result.format_error(0, "Invalid version");
        assert!(msg.contains("Line 1"));
        assert!(msg.contains("Invalid version"));
    }

    #[test]
    fn test_format_error_multi_line() {
        let input = "- COMP MACRO\n + FIXED ( 100 200 ) N\n ;";
        let result = preprocess(input);
        let msg = result.format_error(0, "Invalid component");
        assert!(msg.contains("Lines 1-3"));
        assert!(msg.contains("Invalid component"));
    }

    #[test]
    fn test_fusion_compiler_format() {
        let input = r#"# Fusion Compiler write_def
VERSION 5.8 ;
COMPONENTS 2 ;
- u_io_top/comp1 MACRO1 + FIXED ( 0 100 ) N
 ;
- u_io_top/comp2 MACRO2 + PLACED ( 200 300 ) S ;
END COMPONENTS
"#;
        let result = preprocess(input);
        assert_eq!(result.lines.len(), 5); // VERSION, COMPONENTS, comp1, comp2, END
        assert!(result.lines[2].contains("u_io_top/comp1"));
        assert!(result.lines[2].contains("FIXED"));
        assert!(result.lines[3].contains("u_io_top/comp2"));
    }

    #[test]
    fn test_cadence_innovus_format() {
        let input = r#"###############################################################
#  Generated by:      Cadence Innovus 22.33-s094_1
###############################################################
VERSION 5.8 ;
DESIGN soc_top ;
COMPONENTS 2 ;
- u_io_top/u_TEST_west_9 HPDWUW0608DGP_H + FIXED ( 0 4735000 ) E
 ;
- u_io_top/u_RST_N_west_11 HPDWUW0608DGP_H + FIXED ( 0 4655000 ) E
 ;
END COMPONENTS
"#;
        let result = preprocess(input);
        assert_eq!(result.lines.len(), 6); // VERSION, DESIGN, COMPONENTS, 2 comps, END
        assert!(result.lines[3].contains("u_TEST_west_9"));
        assert!(result.lines[4].contains("u_RST_N_west_11"));
    }

    #[test]
    fn test_incomplete_statement() {
        // Statement without semicolon at end
        let input = "- COMP MACRO + FIXED ( 100 200 ) N";
        let result = preprocess(input);
        assert_eq!(result.lines.len(), 1);
        assert!(!result.lines[0].contains(";"));
    }

    #[test]
    fn test_multiple_semicolons_same_line() {
        let input = "VERSION 5.8 ; DESIGN test ;";
        let result = preprocess(input);
        // Should be treated as one logical line (first semicolon terminates)
        assert_eq!(result.lines.len(), 1);
    }
}
