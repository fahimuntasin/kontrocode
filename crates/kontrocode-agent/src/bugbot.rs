use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewComment {
    pub file: String,
    pub line: Option<u32>,
    pub text: String,
    pub severity: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReviewResult {
    pub comments: Vec<ReviewComment>,
}

pub async fn review_diff(diff_content: &str) -> anyhow::Result<ReviewResult> {
    if diff_content.trim().is_empty() {
        return Ok(ReviewResult { comments: vec![] });
    }

    let mut comments = Vec::new();
    let lines: Vec<&str> = diff_content.lines().collect();
    let mut current_file = String::new();
    let mut current_line: u32 = 0;

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("diff --git") {
            if let Some(path) = line.split(' ').nth(3) {
                current_file = path.trim_start_matches("b/").to_string();
            }
            continue;
        }
        if line.starts_with("---") {
            continue;
        }

        if line.starts_with('+') && !line.starts_with("+++") {
            let code = &line[1..];

            if code.contains("TODO") || code.contains("FIXME") {
                comments.push(ReviewComment {
                    file: current_file.clone(),
                    line: Some(current_line),
                    text: format!("Found TODO/FIXME: `{}`", code.trim()),
                    severity: "info".into(),
                });
            }

            if code.contains("unwrap()") {
                comments.push(ReviewComment {
                    file: current_file.clone(),
                    line: Some(current_line),
                    text: "Consider using proper error handling instead of `unwrap()`".into(),
                    severity: "warning".into(),
                });
            }

            if code.contains("println!") || code.contains("console.log") {
                comments.push(ReviewComment {
                    file: current_file.clone(),
                    line: Some(current_line),
                    text: "Debug print statement found — remove before merge".into(),
                    severity: "warning".into(),
                });
            }

            current_line += 1;
        } else if !line.starts_with('-') {
            current_line += 1;
        }
    }

    Ok(ReviewResult { comments })
}
