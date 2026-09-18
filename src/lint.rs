use std::fmt;

/// Functions that force a spreadsheet to recalculate on every edit,
/// not just when their own inputs change. A sheet with a few hundred
/// of these gets noticeably slow, and the cause is rarely obvious
/// from the symptom alone.
const VOLATILE_FUNCTIONS: [&str; 6] = ["NOW(", "TODAY(", "RAND(", "RANDBETWEEN(", "OFFSET(", "INDIRECT("];

pub struct Finding {
    pub line: usize,
    pub rule: &'static str,
    pub message: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: [{}] {}", self.line, self.rule, self.message)
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
            message: "formula has no expression after '='".to_string(),
        });
        return findings;
    }

    check_parens(line_number, body, &mut findings);
    check_volatile(line_number, body, &mut findings);

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
                message: format!("uses {} which recalculates on every sheet edit", &name[..name.len() - 1]),
            });
        }
    }
}
