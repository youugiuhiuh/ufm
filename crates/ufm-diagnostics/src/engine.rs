use ufm_core::EnvironmentReport;

use crate::rule::{DiagnosticContext, DiagnosticRule};
use crate::rules::{
    BadImageFormatRule, DllNotFoundRule, DoorstopConfigRule, EntryPointNotFoundRule,
    HarmonyPatchRule, MissingMemberRule,
};
use crate::types::DiagnosticReportData;

pub struct DiagnosticEngine {
    rules: Vec<Box<dyn DiagnosticRule>>,
}

impl Default for DiagnosticEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticEngine {
    pub fn new() -> Self {
        let rules: Vec<Box<dyn DiagnosticRule>> = vec![
            Box::new(DllNotFoundRule),
            Box::new(BadImageFormatRule),
            Box::new(EntryPointNotFoundRule),
            Box::new(DoorstopConfigRule),
            Box::new(HarmonyPatchRule),
            Box::new(MissingMemberRule),
        ];

        Self { rules }
    }

    pub fn with_custom_rule(mut self, rule: Box<dyn DiagnosticRule>) -> Self {
        self.rules.push(rule);
        self
    }

    pub fn rules(&self) -> &[Box<dyn DiagnosticRule>] {
        &self.rules
    }

    pub fn analyze(&self, raw_log: &str) -> DiagnosticReportData {
        self.analyze_with_env(raw_log, None)
    }

    pub fn analyze_with_env(
        &self,
        raw_log: &str,
        env: Option<&EnvironmentReport>,
    ) -> DiagnosticReportData {
        let ctx = DiagnosticContext::new(raw_log, env);
        let mut all_issues = Vec::new();

        for rule in &self.rules {
            let issues = rule.evaluate(&ctx);
            all_issues.extend(issues);
        }

        DiagnosticReportData::new(all_issues)
    }
}
