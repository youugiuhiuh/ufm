use ufm_core::EnvironmentReport;
use ufm_diagnostics::DiagnosticReportData;

pub fn render_environment_markdown(report: &EnvironmentReport) -> String {
    let mut md = String::new();

    md.push_str("# Environment Detection Report\n\n");
    md.push_str("## System Summary\n\n");
    md.push_str("| Property | Value |\n");
    md.push_str("| :--- | :--- |\n");
    md.push_str(&format!("| **Game Path** | `{}` |\n", report.game_path));

    let data_dir_str = report
        .data_dir
        .as_deref()
        .map(|d| format!("`{}`", d))
        .unwrap_or_else(|| "*(None)*".into());
    md.push_str(&format!("| **Data Directory** | {} |\n", data_dir_str));

    let unity_ver = report
        .unity_version
        .as_ref()
        .map(|v| v.raw.as_str())
        .unwrap_or("Unknown");
    md.push_str(&format!("| **Unity Version** | `{}` |\n", unity_ver));
    md.push_str(&format!(
        "| **Runtime Type** | `{:?}` |\n",
        report.runtime_type
    ));
    md.push_str(&format!(
        "| **Architecture** | `{:?}` |\n",
        report.architecture
    ));
    md.push_str(&format!(
        "| **Platform** | `{:?}` |\n",
        report.target_platform
    ));

    let frameworks_str = if report.installed_frameworks.is_empty() {
        "*(None)*".to_string()
    } else {
        report.installed_frameworks.join(", ")
    };
    md.push_str(&format!(
        "| **Installed Frameworks** | {} |\n",
        frameworks_str
    ));
    md.push_str(&format!(
        "| **Overall Confidence** | `{:?}` |\n\n",
        report.overall_confidence
    ));

    md.push_str("## Detection Evidence\n\n");
    if report.evidences.is_empty() {
        md.push_str("*No evidence items recorded.*\n\n");
    } else {
        md.push_str("| Key | Confidence | Description | Path |\n");
        md.push_str("| :--- | :--- | :--- | :--- |\n");
        for ev in &report.evidences {
            let p = ev.path.as_deref().unwrap_or("-");
            md.push_str(&format!(
                "| `{}` | `{:?}` | {} | `{}` |\n",
                ev.key, ev.confidence, ev.description, p
            ));
        }
        md.push('\n');
    }

    md
}

pub fn render_diagnostics_markdown(diag: &DiagnosticReportData) -> String {
    let mut md = String::new();

    md.push_str("# Diagnostics & Log Analysis Report\n\n");
    md.push_str("## Summary\n\n");
    md.push_str(&format!(
        "- **Total Issues Detected:** {}\n",
        diag.total_issues
    ));
    md.push_str(&format!("- **Highest Risk Level:** `{}`\n", diag.max_risk));
    md.push_str(&format!("- **Assessment:** {}\n\n", diag.summary));

    if diag.issues.is_empty() {
        md.push_str("✅ **No known error patterns detected in log.**\n\n");
    } else {
        md.push_str("## Issues & Remediation\n\n");
        for (idx, issue) in diag.issues.iter().enumerate() {
            md.push_str(&format!(
                "### {}. [{}] {}\n\n",
                idx + 1,
                issue.risk_level,
                issue.title
            ));
            md.push_str(&format!("**Description:** {}\n\n", issue.description));

            if !issue.matched_lines.is_empty() {
                md.push_str("**Matched Log Snippet:**\n```text\n");
                for line in &issue.matched_lines {
                    md.push_str(line);
                    md.push('\n');
                }
                md.push_str("```\n\n");
            }

            md.push_str("**Suggested Fix:**\n");
            md.push_str(&issue.suggested_fix);
            md.push_str("\n\n");

            if !issue.references.is_empty() {
                md.push_str("**References:**\n");
                for r in &issue.references {
                    md.push_str(&format!("- <{}>\n", r));
                }
                md.push('\n');
            }
            md.push_str("---\n\n");
        }
    }

    md
}

pub fn render_combined_markdown(
    env: Option<&EnvironmentReport>,
    diag: Option<&DiagnosticReportData>,
) -> String {
    let mut md = String::new();
    md.push_str("# Unity Framework Manager (UFM) Comprehensive Report\n\n");

    if let Some(e) = env {
        md.push_str(&render_environment_markdown(e));
        md.push_str("\n---\n\n");
    }

    if let Some(d) = diag {
        md.push_str(&render_diagnostics_markdown(d));
    }

    md
}
