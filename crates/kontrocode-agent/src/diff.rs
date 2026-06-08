use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub file_path: String,
    pub old_start: usize,
    pub new_start: usize,
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub enum DiffLine {
    Context(String),
    Addition(String),
    Deletion(String),
}

impl DiffLine {
    pub fn to_unified(&self) -> String {
        match self {
            DiffLine::Context(s) => format!(" {s}"),
            DiffLine::Addition(s) => format!("+{s}"),
            DiffLine::Deletion(s) => format!("-{s}"),
        }
    }
}

pub fn compute_diff(old: &str, new: &str) -> Vec<DiffHunk> {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let lcs = longest_common_subsequence(&old_lines, &new_lines);
    let mut hunks = Vec::new();
    let mut current = Vec::new();
    let mut old_idx = 0;
    let mut new_idx = 0;

    for (oi, ni) in &lcs {
        while old_idx < *oi || new_idx < *ni {
            if old_idx < *oi && new_idx < *ni {
                current.push(DiffLine::Deletion(old_lines[old_idx].to_string()));
                old_idx += 1;
                current.push(DiffLine::Addition(new_lines[new_idx].to_string()));
                new_idx += 1;
            } else if old_idx < *oi {
                current.push(DiffLine::Deletion(old_lines[old_idx].to_string()));
                old_idx += 1;
            } else if new_idx < *ni {
                current.push(DiffLine::Addition(new_lines[new_idx].to_string()));
                new_idx += 1;
            }
        }
        current.push(DiffLine::Context(old_lines[old_idx].to_string()));
        old_idx += 1;
        new_idx += 1;
    }

    while old_idx < old_lines.len() || new_idx < new_lines.len() {
        if old_idx < old_lines.len() && new_idx < new_lines.len() {
            current.push(DiffLine::Deletion(old_lines[old_idx].to_string()));
            old_idx += 1;
            current.push(DiffLine::Addition(new_lines[new_idx].to_string()));
            new_idx += 1;
        } else if old_idx < old_lines.len() {
            current.push(DiffLine::Deletion(old_lines[old_idx].to_string()));
            old_idx += 1;
        } else if new_idx < new_lines.len() {
            current.push(DiffLine::Addition(new_lines[new_idx].to_string()));
            new_idx += 1;
        }
    }

    if !current.is_empty() {
        hunks.push(DiffHunk {
            file_path: String::new(),
            old_start: 0,
            new_start: 0,
            header: String::new(),
            lines: current,
        });
    }

    hunks
}

pub fn apply_diff(original: &str, hunks: &[DiffHunk]) -> String {
    let mut result = original.to_string();
    for hunk in hunks {
        let mut applied = String::new();
        for line in &hunk.lines {
            match line {
                DiffLine::Context(s) | DiffLine::Addition(s) => {
                    applied.push_str(s);
                    applied.push('\n');
                }
                DiffLine::Deletion(_) => {}
            }
        }
        result = applied;
    }
    result
}

pub fn unified_diff(hunks: &[DiffHunk]) -> String {
    let mut output = String::new();
    for hunk in hunks {
        output.push_str(&format!(
            "--- a/{}\n+++ b/{}\n",
            hunk.file_path, hunk.file_path
        ));
        output.push_str(&format!(
            "@@ -{},{} +{},{} @@ {}\n",
            hunk.old_start,
            hunk.lines.iter().filter(|l| matches!(l, DiffLine::Context(_) | DiffLine::Deletion(_))).count(),
            hunk.new_start,
            hunk.lines.iter().filter(|l| matches!(l, DiffLine::Context(_) | DiffLine::Addition(_))).count(),
            hunk.header,
        ));
        for line in &hunk.lines {
            output.push_str(&line.to_unified());
            output.push('\n');
        }
    }
    output
}

fn longest_common_subsequence<T: PartialEq>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];

    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    let mut result = Vec::new();
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        if a[i - 1] == b[j - 1] {
            result.push((i - 1, j - 1));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            i -= 1;
        } else {
            j -= 1;
        }
    }
    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_diff() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nline2_changed\nline3\n";
        let hunks = compute_diff(old, new);
        let unified = unified_diff(&hunks);
        assert!(unified.contains("line2_changed"));
        assert!(unified.contains("line2"));
    }

    #[test]
    fn apply_diff_roundtrip() {
        let original = "hello\nworld\n";
        let modified = "hello\nkontrocode\nworld\n";
        let hunks = compute_diff(original, modified);
        let result = apply_diff(original, &hunks);
        assert_eq!(result.trim(), "hello\nkontrocode\nworld");
    }
}
