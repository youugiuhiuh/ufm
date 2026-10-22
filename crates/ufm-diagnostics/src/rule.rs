use ufm_core::EnvironmentReport;

use crate::types::DiagnosticIssue;

#[derive(Debug, Clone)]
pub struct DiagnosticContext<'a> {
    pub raw_log: &'a str,
    pub lines: Vec<&'a str>,
    pub environment: Option<&'a EnvironmentReport>,
}

impl<'a> DiagnosticContext<'a> {
    pub fn new(raw_log: &'a str, environment: Option<&'a EnvironmentReport>) -> Self {
        let lines: Vec<&'a str> = raw_log.lines().collect();
        Self {
            raw_log,
            lines,
            environment,
        }
    }
}

pub trait DiagnosticRule: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn evaluate(&self, ctx: &DiagnosticContext) -> Vec<DiagnosticIssue>;
}
