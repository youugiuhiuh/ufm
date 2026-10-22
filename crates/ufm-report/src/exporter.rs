use serde_json::json;
use ufm_core::EnvironmentReport;
use ufm_diagnostics::DiagnosticReportData;

use crate::error::ReportError;
use crate::markdown::{
    render_combined_markdown, render_diagnostics_markdown, render_environment_markdown,
};
use crate::redactor::Redactor;

#[derive(Default)]
pub struct ReportExporter {
    redactor: Redactor,
}

impl ReportExporter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_redactor(redactor: Redactor) -> Self {
        Self { redactor }
    }

    pub fn export_environment_json(
        &self,
        env: &EnvironmentReport,
        redact: bool,
    ) -> Result<String, ReportError> {
        let value = serde_json::to_value(env)?;
        let final_value = if redact {
            self.redactor.redact_value(&value)
        } else {
            value
        };
        Ok(serde_json::to_string_pretty(&final_value)?)
    }

    pub fn export_environment_markdown(
        &self,
        env: &EnvironmentReport,
        redact: bool,
    ) -> Result<String, ReportError> {
        let md = render_environment_markdown(env);
        if redact {
            Ok(self.redactor.redact_text(&md))
        } else {
            Ok(md)
        }
    }

    pub fn export_diagnostic_json(
        &self,
        diag: &DiagnosticReportData,
        redact: bool,
    ) -> Result<String, ReportError> {
        let value = serde_json::to_value(diag)?;
        let final_value = if redact {
            self.redactor.redact_value(&value)
        } else {
            value
        };
        Ok(serde_json::to_string_pretty(&final_value)?)
    }

    pub fn export_diagnostic_markdown(
        &self,
        diag: &DiagnosticReportData,
        redact: bool,
    ) -> Result<String, ReportError> {
        let md = render_diagnostics_markdown(diag);
        if redact {
            Ok(self.redactor.redact_text(&md))
        } else {
            Ok(md)
        }
    }

    pub fn export_combined_json(
        &self,
        env: Option<&EnvironmentReport>,
        diag: Option<&DiagnosticReportData>,
        redact: bool,
    ) -> Result<String, ReportError> {
        let root = json!({
            "environment": env,
            "diagnostics": diag,
        });
        let final_value = if redact {
            self.redactor.redact_value(&root)
        } else {
            root
        };
        Ok(serde_json::to_string_pretty(&final_value)?)
    }

    pub fn export_combined_markdown(
        &self,
        env: Option<&EnvironmentReport>,
        diag: Option<&DiagnosticReportData>,
        redact: bool,
    ) -> Result<String, ReportError> {
        let md = render_combined_markdown(env, diag);
        if redact {
            Ok(self.redactor.redact_text(&md))
        } else {
            Ok(md)
        }
    }
}
