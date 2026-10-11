//! Assembling and exporting an investigation report.
//!
//! A report ties a case together for the "reports" screen and for export: the incident and its
//! state, the investigator's grounded claims (or its refusal), the evidence that was consulted
//! with redaction and missing-telemetry noted, and the relationships rendered with observed and
//! inferred edges kept visibly distinct. The Markdown export is deterministic so two runs over
//! the same inputs produce the same document.

use investigator_ground::{Claim, ClaimKind, Response};
use investigator_incident::Incident;

use crate::view::{EdgeView, EvidenceView};

/// A complete investigation report.
pub struct InvestigationReport<'a> {
    /// The incident the report documents.
    pub incident: &'a Incident,
    /// The investigator's grounded response.
    pub response: &'a Response,
    /// The evidence consulted.
    pub evidence: &'a [EvidenceView],
    /// The relationships, observed and inferred.
    pub edges: &'a [EdgeView],
}

impl<'a> InvestigationReport<'a> {
    /// Build a report.
    pub fn new(
        incident: &'a Incident,
        response: &'a Response,
        evidence: &'a [EvidenceView],
        edges: &'a [EdgeView],
    ) -> Self {
        InvestigationReport {
            incident,
            response,
            evidence,
            edges,
        }
    }

    /// Whether the report rests on at least one grounded (non-refusal) answer.
    pub fn has_grounded_answer(&self) -> bool {
        matches!(self.response, Response::Answer { claims } if !claims.is_empty())
    }

    /// Render the report as deterministic Markdown.
    pub fn to_markdown(&self) -> String {
        let inc = self.incident;
        let mut out = String::new();

        out.push_str(&format!("# Incident {}: {}\n\n", inc.id, inc.title));
        out.push_str(&format!("- Tenant: {}\n", inc.tenant_id));
        out.push_str(&format!("- State: {:?}\n", inc.state));
        out.push_str(&format!("- Owner: {}\n", inc.owner.as_deref().unwrap_or("unassigned")));
        out.push_str(&format!("- Opened: {}\n", inc.created_at.to_rfc3339()));
        out.push_str(&format!("- Last updated: {}\n\n", inc.updated_at.to_rfc3339()));

        out.push_str("## Investigator assessment\n\n");
        match self.response {
            Response::Refusal { reason } => {
                out.push_str(&format!("> Refused: {reason}\n\n"));
            }
            Response::Answer { claims } => {
                for claim in claims {
                    out.push_str(&render_claim(claim));
                }
                out.push('\n');
            }
        }

        out.push_str("## Relationships\n\n");
        out.push_str("Legend: ● observed, ◆ rule-inferred, ◇ stat-inferred.\n\n");
        for e in self.edges {
            let kind = if e.causal { "" } else { " (association only)" };
            out.push_str(&format!(
                "- {} {} —{}→ {}{}  [evidence: {}]\n",
                e.confidence_method.glyph(),
                e.src,
                e.relation,
                e.dst,
                kind,
                join_or_none(&e.evidence_ids),
            ));
        }
        out.push('\n');

        out.push_str("## Evidence\n\n");
        for ev in self.evidence {
            out.push_str(&format!(
                "- `{}` ({}) @ {} — redaction: {:?}{}\n",
                ev.id,
                ev.kind,
                ev.timestamp.to_rfc3339(),
                ev.redaction,
                if ev.missing_telemetry.is_empty() {
                    String::new()
                } else {
                    format!(" — missing: {}", ev.missing_telemetry.join(", "))
                },
            ));
        }
        out.push('\n');

        out.push_str("## Audit trail\n\n");
        for a in &inc.audit {
            out.push_str(&format!(
                "- {} — {} — {:?} — {}\n",
                a.at.to_rfc3339(),
                a.actor,
                a.action,
                a.reason
            ));
        }

        out
    }
}

fn render_claim(claim: &Claim) -> String {
    let marker = match claim.claim_kind {
        ClaimKind::Fact => "FACT",
        ClaimKind::Causal => "CAUSAL",
        ClaimKind::Hypothesis => "HYPOTHESIS",
    };
    format!(
        "- **[{marker}]** {} _(certainty: {:?}; cites: {})_\n",
        claim.claim_text,
        claim.uncertainty,
        join_or_none(&claim.evidence_ids),
    )
}

fn join_or_none(ids: &[String]) -> String {
    if ids.is_empty() {
        "none".to_string()
    } else {
        ids.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{ConfidenceMethod, RedactionStatus};
    use chrono::{TimeZone, Utc};
    use investigator_incident::{Incident, IncidentState};

    fn at() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
    }

    fn incident() -> Incident {
        let mut inc = Incident::open("inc-1", "alpha", "Cross-tenant read", "analyst-1", at());
        inc.transition(IncidentState::Investigating, "analyst-1", "triage", at()).unwrap();
        inc.attach_evidence("authz-finding-1", "analyst-1", at());
        inc
    }

    fn edges() -> Vec<EdgeView> {
        vec![
            EdgeView {
                src: "agent-7".into(),
                dst: "res-bravo".into(),
                relation: "accesses".into(),
                confidence_method: ConfidenceMethod::Observed,
                causal: true,
                evidence_ids: vec!["gw-1".into()],
                timestamp: at(),
            },
            EdgeView {
                src: "res-bravo".into(),
                dst: "flow-9".into(),
                relation: "correlated_with".into(),
                confidence_method: ConfidenceMethod::InferredStat,
                causal: false,
                evidence_ids: vec!["flow-9".into()],
                timestamp: at(),
            },
        ]
    }

    fn evidence() -> Vec<EvidenceView> {
        vec![EvidenceView {
            id: "authz-finding-1".into(),
            kind: "finding".into(),
            timestamp: at(),
            redaction: RedactionStatus::Partial,
            missing_telemetry: vec!["response_body".into()],
        }]
    }

    #[test]
    fn answer_report_renders_claims_edges_and_evidence() {
        let inc = incident();
        let resp = Response::Answer {
            claims: vec![Claim::fact("the agent read a cross-tenant order", vec!["gw-1".into()])],
        };
        let edges = edges();
        let evidence = evidence();
        let report = InvestigationReport::new(&inc, &resp, &evidence, &edges);
        assert!(report.has_grounded_answer());
        let md = report.to_markdown();
        assert!(md.contains("# Incident inc-1"));
        assert!(md.contains("[FACT]"));
        // observed and inferred edges are marked differently
        assert!(md.contains("● agent-7"));
        assert!(md.contains("◇ res-bravo"));
        assert!(md.contains("association only"));
        // evidence shows redaction and missing telemetry
        assert!(md.contains("redaction: Partial"));
        assert!(md.contains("missing: response_body"));
        // audit trail present
        assert!(md.contains("## Audit trail"));
    }

    #[test]
    fn refusal_report_states_the_refusal() {
        let inc = incident();
        let resp = Response::Refusal {
            reason: "insufficient evidence".into(),
        };
        let report = InvestigationReport::new(&inc, &resp, &[], &[]);
        assert!(!report.has_grounded_answer());
        assert!(report.to_markdown().contains("Refused: insufficient evidence"));
    }

    #[test]
    fn markdown_is_deterministic() {
        let inc = incident();
        let resp = Response::Answer {
            claims: vec![Claim::hypothesis("might be lateral movement")],
        };
        let report = InvestigationReport::new(&inc, &resp, &[], &[]);
        assert_eq!(report.to_markdown(), report.to_markdown());
    }
}
