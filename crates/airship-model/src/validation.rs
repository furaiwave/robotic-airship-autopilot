//! Structured validation results shared by every configuration section.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// The configuration cannot be simulated.
    Error,
    /// The configuration can be simulated but is outside the range the model was designed for.
    Warning,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Dotted path of the offending field, e.g. `airship.length_m`.
    pub field: String,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn error(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.issues.push(ValidationIssue {
            field: field.into(),
            message: message.into(),
            severity: Severity::Error,
        });
    }

    pub fn warning(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.issues.push(ValidationIssue {
            field: field.into(),
            message: message.into(),
            severity: Severity::Warning,
        });
    }

    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    pub fn errors(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues.iter().filter(|i| i.severity == Severity::Error)
    }

    pub fn warnings(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
    }

    /// Append all issues of `other`, prefixing their field paths with `prefix.`.
    pub fn merge(&mut self, prefix: &str, other: ValidationReport) {
        for mut issue in other.issues {
            if !prefix.is_empty() {
                issue.field = format!("{prefix}.{}", issue.field);
            }
            self.issues.push(issue);
        }
    }

    /// Require `value` to be finite and inside `[min, max]`.
    pub fn range(&mut self, field: &str, value: f64, min: f64, max: f64, unit: &str) -> bool {
        if !value.is_finite() {
            self.error(field, "value must be a finite number");
            return false;
        }
        if value < min || value > max {
            self.error(
                field,
                format!("must be between {min} and {max} {unit} (got {value})"),
            );
            return false;
        }
        true
    }

    /// Emit a warning when `value` is outside the recommended `[min, max]` band.
    pub fn recommend(&mut self, field: &str, value: f64, min: f64, max: f64, why: &str) {
        if value.is_finite() && (value < min || value > max) {
            self.warning(
                field,
                format!("{value} is outside the recommended range [{min}, {max}]: {why}"),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_check_and_merge() {
        let mut r = ValidationReport::new();
        assert!(r.range("a", 1.0, 0.0, 2.0, "m"));
        assert!(!r.range("b", 3.0, 0.0, 2.0, "m"));
        assert!(!r.range("c", f64::NAN, 0.0, 2.0, "m"));
        let mut top = ValidationReport::new();
        top.merge("airship", r);
        assert!(top.has_errors());
        assert_eq!(top.errors().count(), 2);
        assert_eq!(top.issues[0].field, "airship.b");
    }
}
