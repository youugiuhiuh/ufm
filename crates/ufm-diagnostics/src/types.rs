use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Info = 0,
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl std::fmt::Display for RiskLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RiskLevel::Info => write!(f, "INFO"),
            RiskLevel::Low => write!(f, "LOW"),
            RiskLevel::Medium => write!(f, "MEDIUM"),
            RiskLevel::High => write!(f, "HIGH"),
            RiskLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticIssue {
    pub rule_id: String,
    pub title: String,
    pub risk_level: RiskLevel,
    pub description: String,
    pub matched_lines: Vec<String>,
    pub suggested_fix: String,
    pub references: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticReportData {
    pub total_issues: usize,
    pub max_risk: RiskLevel,
    pub issues: Vec<DiagnosticIssue>,
    pub summary: String,
}

impl DiagnosticReportData {
    pub fn new(mut issues: Vec<DiagnosticIssue>) -> Self {
        // Sort by risk level descending
        issues.sort_by_key(|a| std::cmp::Reverse(a.risk_level));

        let max_risk = issues
            .iter()
            .map(|i| i.risk_level)
            .max()
            .unwrap_or(RiskLevel::Info);

        let total_issues = issues.len();
        let summary = if total_issues == 0 {
            "No known issues or error patterns detected in log.".to_string()
        } else {
            format!(
                "Detected {} issue(s) with highest risk level: {}.",
                total_issues, max_risk
            )
        };

        Self {
            total_issues,
            max_risk,
            issues,
            summary,
        }
    }
}
