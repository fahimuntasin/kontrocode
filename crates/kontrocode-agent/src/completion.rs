use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub file_path: String,
    pub file_content: String,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub language: String,
    pub max_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub text: String,
    pub confidence: f64,
    pub range_start_line: usize,
    pub range_end_line: usize,
}

pub struct CompletionEngine;

impl CompletionEngine {
    pub fn new() -> Self {
        Self
    }

    pub fn complete(&self, req: &CompletionRequest) -> Vec<CompletionResponse> {
        let lines: Vec<&str> = req.file_content.lines().collect();
        if req.cursor_line >= lines.len() {
            return vec![];
        }

        let current_line = lines[req.cursor_line];
        let prefix = if req.cursor_col <= current_line.len() {
            &current_line[..req.cursor_col]
        } else {
            current_line
        };

        let mut completions = Vec::new();

        if let Some(suffix) = self.complete_bracket(prefix) {
            completions.push(CompletionResponse {
                text: suffix,
                confidence: 1.0,
                range_start_line: req.cursor_line,
                range_end_line: req.cursor_line,
            });
        }

        if let Some(import_line) = self.suggest_import(current_line, &req.language, &lines) {
            completions.push(CompletionResponse {
                text: import_line,
                confidence: 0.7,
                range_start_line: req.cursor_line,
                range_end_line: req.cursor_line,
            });
        }

        if prefix.ends_with("fn ") || prefix.ends_with("func ") || prefix.ends_with("def ") {
            completions.push(CompletionResponse {
                text: self.generate_function_template(&req.language),
                confidence: 0.8,
                range_start_line: req.cursor_line,
                range_end_line: req.cursor_line + 4,
            });
        }

        if prefix.trim().is_empty() {
            if let Some(boilerplate) = self.common_patterns(&req.language, &req.file_path) {
                completions.push(CompletionResponse {
                    text: boilerplate,
                    confidence: 0.5,
                    range_start_line: req.cursor_line,
                    range_end_line: req.cursor_line + 5,
                });
            }
        }

        completions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        completions
    }

    fn complete_bracket(&self, prefix: &str) -> Option<String> {
        if prefix.ends_with('(') { return Some(")".into()); }
        if prefix.ends_with('[') { return Some("]".into()); }
        if prefix.ends_with('{') { return Some("}".into()); }
        if prefix.ends_with('<') && !prefix.contains("</") { return Some(">".into()); }
        None
    }

    fn suggest_import(&self, line: &str, language: &str, lines: &[&str]) -> Option<String> {
        if line.starts_with("use ") || line.starts_with("import ") || line.starts_with("from ") {
            return None;
        }

        let has_imports = lines.iter().any(|l| {
            l.starts_with("use ") || l.starts_with("import ") || l.starts_with("from ")
        });

        if !has_imports {
            match language {
                "rust" => Some("use std::".into()),
                "python" => Some("import ".into()),
                "javascript" | "typescript" => Some("import {  } from '';".into()),
                "go" => Some("import (".into()),
                "dart" => Some("import 'package:".into()),
                _ => None,
            }
        } else {
            None
        }
    }

    fn generate_function_template(&self, language: &str) -> String {
        match language {
            "rust" => "fn name() -> Result<()> {\n    todo!()\n}\n".into(),
            "python" => "def name():\n    pass\n".into(),
            "javascript" | "typescript" => "function name() {\n  return;\n}\n".into(),
            "go" => "func Name() error {\n\treturn nil\n}\n".into(),
            "dart" => "void name() {\n  // TODO\n}\n".into(),
            _ => "fn name() {\n  // TODO\n}\n".into(),
        }
    }

    fn common_patterns(&self, language: &str, file_path: &str) -> Option<String> {
        if file_path.ends_with("main.rs") {
            return Some("fn main() {\n    println!(\"Hello, KontroCode!\");\n}\n".into());
        }
        if file_path.ends_with("main.go") {
            return Some("package main\n\nfunc main() {\n\tprintln(\"Hello, KontroCode!\")\n}\n".into());
        }
        None
    }
}
