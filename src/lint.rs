use std::fmt;

/// Functions that force a spreadsheet to recalculate on every edit,
/// not just when their own inputs change. A sheet with a few hundred
/// of these gets noticeably slow, and the cause is rarely obvious
/// from the symptom alone.
const VOLATILE_FUNCTIONS: [&str; 6] = ["NOW(", "TODAY(", "RAND(", "RANDBETWEEN(", "OFFSET(", "INDIRECT("];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    // The formula is broken or will evaluate to something other than intended.
    Error,
    // The formula works but is worth a second look (performance, fragility).
    Warning,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
        }
    }
}

pub struct Finding {
    pub line: usize,
    pub rule: &'static str,
    pub severity: Severity,
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}: [{}] {}", self.line, self.severity, self.rule, self.message)
    }
}

/// Runs every rule against a single line of input and returns whatever
/// it finds. Kept to one line at a time so the caller never has to hold
/// more than the current line in memory.
pub fn check_line(line_number: usize, raw: &str) -> Vec<Finding> {
    let text = raw.trim();
    let mut findings = Vec::new();

    if text.is_empty() || text.starts_with('#') {
        return findings;
    }

    let body = text.strip_prefix('=').unwrap_or(text);

    if body.trim().is_empty() {
        findings.push(Finding {
            line: line_number,
            rule: "empty-formula",
            severity: Severity::Error,
            message: "formula has no expression after '='".to_string(),
        });
        return findings;
    }

    check_parens(line_number, body, &mut findings);
    check_volatile(line_number, body, &mut findings);
    check_cross_sheet(line_number, body, &mut findings);

    findings
}

fn check_parens(line_number: usize, body: &str, findings: &mut Vec<Finding>) {
    let mut depth: i32 = 0;
    let mut in_string = false;

    for ch in body.chars() {
        match ch {
            '"' => in_string = !in_string,
            '(' if !in_string => depth += 1,
            ')' if !in_string => {
                depth -= 1;
                if depth < 0 {
                    findings.push(Finding {
                        line: line_number,
                        rule: "unbalanced-parens",
                        severity: Severity::Error,
                        message: "unexpected ')' with no matching '('".to_string(),
                    });
                    return;
                }
            }
            _ => {}
        }
    }

    if depth > 0 {
        findings.push(Finding {
            line: line_number,
            rule: "unbalanced-parens",
            severity: Severity::Error,
            message: format!("missing {} closing ')'", depth),
        });
    }
}

fn check_volatile(line_number: usize, body: &str, findings: &mut Vec<Finding>) {
    let upper = body.to_ascii_uppercase();
    for name in VOLATILE_FUNCTIONS {
        if upper.contains(name) {
            findings.push(Finding {
                line: line_number,
                rule: "volatile-function",
                severity: Severity::Warning,
                message: format!("uses {} which recalculates on every sheet edit", &name[..name.len() - 1]),
            });
        }
    }
}

/// Flags references to another sheet by name, e.g. `Sheet2!A1` or
/// `'Q3 Actuals'!B2`. These aren't wrong, but a hardcoded sheet name
/// breaks silently the moment someone renames or reorders the sheet,
/// and the formula that used it just starts returning #REF! with no
/// hint of why. Reports each distinct sheet name once per line.
fn check_cross_sheet(line_number: usize, body: &str, findings: &mut Vec<Finding>) {
    let chars: Vec<char> = body.chars().collect();
    let mut seen: Vec<String> = Vec::new();
    let mut in_string = false;
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '"' {
            in_string = !in_string;
            i += 1;
            continue;
        }
        if in_string {
            i += 1;
            continue;
        }

        if ch == '\'' {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && chars[j] != '\'' {
                j += 1;
            }
            if j < chars.len() && j + 1 < chars.len() && chars[j + 1] == '!' {
                let name: String = chars[start..j].iter().collect();
                record_sheet_reference(line_number, name, &mut seen, findings);
                i = j + 2;
                continue;
            }
            i += 1;
            continue;
        }

        if ch.is_alphabetic() || ch == '_' {
            let start = i;
            let mut j = i;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_' || chars[j] == '.') {
                j += 1;
            }
            if j < chars.len() && chars[j] == '!' {
                let name: String = chars[start..j].iter().collect();
                record_sheet_reference(line_number, name, &mut seen, findings);
                i = j + 1;
                continue;
            }
            i = j;
            continue;
        }

        i += 1;
    }
}

fn record_sheet_reference(
    line_number: usize,
    name: String,
    seen: &mut Vec<String>,
    findings: &mut Vec<Finding>,
) {
    if name.is_empty() || seen.contains(&name) {
        return;
    }
    seen.push(name.clone());
    findings.push(Finding {
        line: line_number,
        rule: "cross-sheet-reference",
        severity: Severity::Warning,
        message: format!(
            "hardcoded reference to sheet '{}'; renaming or reordering that sheet will silently break this formula",
            name
        ),
    });
}
