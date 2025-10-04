use regex::Regex;
use std::sync::OnceLock;

/// ANSI color codes for terminal output
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BLUE: &str = "\x1b[34m";           // Keywords (definition, kind, model)
    pub const GREEN: &str = "\x1b[32m";          // Strings
    pub const CYAN: &str = "\x1b[36m";           // Node names (id, string, integer)
    pub const YELLOW: &str = "\x1b[33m";         // Numbers
    pub const GRAY: &str = "\x1b[90m";           // Comments
    pub const MAGENTA: &str = "\x1b[35m";        // Special values (null, true, false)
}

struct SyntaxHighlighter {
    comment_line: Regex,
    string: Regex,
    number: Regex,
    boolean: Regex,
    null: Regex,
    keyword: Regex,
    node_name: Regex,
}

fn highlighter() -> &'static SyntaxHighlighter {
    static HIGHLIGHTER: OnceLock<SyntaxHighlighter> = OnceLock::new();
    HIGHLIGHTER.get_or_init(|| SyntaxHighlighter {
        comment_line: Regex::new(r"//.*$").unwrap(),
        string: Regex::new(r#""([^"\\]|\\.)*""#).unwrap(),
        number: Regex::new(r"\b-?\d+(\.\d+)?([eE][+-]?\d+)?\b").unwrap(),
        boolean: Regex::new(r"\b(true|false)\b").unwrap(),
        null: Regex::new(r"\bnull\b").unwrap(),
        keyword: Regex::new(r"\b(definition|kind|model|name|description)\b").unwrap(),
        node_name: Regex::new(r"\b(id|string|integer|boolean|float|object|array)\b").unwrap(),
    })
}

pub fn cat_text_ansi(text: &str) {
    let h = highlighter();
    
    for line in text.lines() {
        let current_line = line.to_string();
        let mut highlights: Vec<(usize, usize, &str)> = Vec::new();
        
        // Find all matches and their positions
        // Comments first (they have precedence)
        if let Some(m) = h.comment_line.find(&current_line) {
            highlights.push((m.start(), m.end(), colors::GRAY));
        }
        
        // Only process other patterns if not a comment
        if highlights.is_empty() {
            // Strings
            for m in h.string.find_iter(&current_line) {
                highlights.push((m.start(), m.end(), colors::GREEN));
            }
            
            // Keywords
            for m in h.keyword.find_iter(&current_line) {
                if !overlaps(&highlights, m.start(), m.end()) {
                    highlights.push((m.start(), m.end(), colors::BLUE));
                }
            }
            
            // Node names (type names)
            for m in h.node_name.find_iter(&current_line) {
                if !overlaps(&highlights, m.start(), m.end()) {
                    highlights.push((m.start(), m.end(), colors::CYAN));
                }
            }
            
            // Booleans
            for m in h.boolean.find_iter(&current_line) {
                if !overlaps(&highlights, m.start(), m.end()) {
                    highlights.push((m.start(), m.end(), colors::MAGENTA));
                }
            }
            
            // Null
            for m in h.null.find_iter(&current_line) {
                if !overlaps(&highlights, m.start(), m.end()) {
                    highlights.push((m.start(), m.end(), colors::MAGENTA));
                }
            }
            
            // Numbers
            for m in h.number.find_iter(&current_line) {
                if !overlaps(&highlights, m.start(), m.end()) {
                    highlights.push((m.start(), m.end(), colors::YELLOW));
                }
            }
        }
        
        // Sort highlights by position
        highlights.sort_by_key(|(start, _, _)| *start);
        
        // Apply highlights
        if highlights.is_empty() {
            println!("{}", line);
        } else {
            let mut result = String::new();
            let mut pos = 0;
            
            for (start, end, color) in highlights {
                // Add unhighlighted text before this match
                if pos < start {
                    result.push_str(&current_line[pos..start]);
                }
                // Add highlighted text
                result.push_str(color);
                result.push_str(&current_line[start..end]);
                result.push_str(colors::RESET);
                pos = end;
            }
            
            // Add remaining unhighlighted text
            if pos < current_line.len() {
                result.push_str(&current_line[pos..]);
            }
            
            println!("{}", result);
        }
    }
}

fn overlaps(highlights: &[(usize, usize, &str)], start: usize, end: usize) -> bool {
    highlights.iter().any(|(s, e, _)| {
        (start >= *s && start < *e) || (end > *s && end <= *e) || (start <= *s && end >= *e)
    })
}
