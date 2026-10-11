//! A deterministic end-to-end investigation.
//!
//! An analyst starts from a verified AuthZ finding and, in a few steps, follows it to the agent
//! event that triggered it and the gateway event that confirms it — exactly the path the UX
//! definition of done describes. The investigator proposes four claims; the post-checker keeps
//! the three that are grounded and drops the one that tries to assert causation from a mere
//! correlation. The surviving claims, an opened incident, and the rendered report are returned.

use chrono::{DateTime, TimeZone, Utc};

use investigator_ground::{Claim, PostChecker, Response, ToolSession};
use investigator_incident::{Incident, IncidentState};
use investigator_report::{ConfidenceMethod, EdgeView, EvidenceView, InvestigationReport, RedactionStatus};

use crate::source::SampleSource;

fn at(min: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 1, 9, min, 0).unwrap()
}

/// The scenario output.
pub struct ScenarioOutput {
    /// The checked investigator response.
    pub response: Response,
    /// The incident opened from the finding.
    pub incident: Incident,
    /// The rendered Markdown report.
    pub report_markdown: String,
}

/// Run the end-to-end investigation deterministically.
pub fn run() -> ScenarioOutput {
    let source = SampleSource::default();
    let session = ToolSession::new(&source, "alpha");

    // Step 1: pull the finding. Step 2: the agent event. Step 3: the gateway confirmation.
    // Step 4: the graph edges around the agent.
    let mut retrieved = Vec::new();
    if let Some(f) = session.get_finding("authz-finding-1") {
        retrieved.push(f);
    }
    if let Some(e) = session.get_event("agent-span-1") {
        retrieved.push(e);
    }
    if let Some(e) = session.get_event("gateway-evt-1") {
        retrieved.push(e);
    }
    retrieved.extend(session.query_graph("order-bravo-1", 10));

    // The investigator proposes claims; the last one over-reaches (causation from correlation).
    let proposed = vec![
        Claim::fact(
            "A verified BOLA finding exists on GET /orders/{id}.",
            vec!["authz-finding-1".into()],
        ),
        Claim::fact(
            "Agent assistant-7 read order-bravo-1, owned by tenant bravo.",
            vec!["agent-span-1".into(), "ed-agent-res".into()],
        ),
        Claim::causal(
            "The agent's access caused the cross-tenant disclosure.",
            vec!["ed-agent-res".into()],
        ),
        // over-reach: asserting the network flow was the exfiltration, from a correlation only
        Claim::causal(
            "The data was exfiltrated over network flow-9.",
            vec!["ed-res-flow".into()],
        ),
    ];

    let checker = PostChecker::new("alpha", &retrieved);
    let response = checker.respond(proposed);

    // Open and advance an incident from the finding.
    let mut incident = Incident::open(
        "inc-7",
        "alpha",
        "Agent cross-tenant order read",
        "analyst-1",
        at(10),
    );
    incident.assign("analyst-1", "analyst-1", at(10));
    incident
        .transition(IncidentState::Investigating, "analyst-1", "following the finding", at(11))
        .unwrap();
    incident.attach_evidence("authz-finding-1", "analyst-1", at(12));
    incident.attach_evidence("agent-span-1", "analyst-1", at(12));
    incident.attach_evidence("gateway-evt-1", "analyst-1", at(12));
    incident
        .transition(IncidentState::Confirmed, "analyst-1", "reproduced from evidence", at(13))
        .unwrap();

    // Build the report view models.
    let edges = vec![
        EdgeView {
            src: "assistant-7".into(),
            dst: "order-bravo-1".into(),
            relation: "accesses".into(),
            confidence_method: ConfidenceMethod::Observed,
            causal: true,
            evidence_ids: vec!["agent-span-1".into(), "ed-agent-res".into()],
            timestamp: at(0),
        },
        EdgeView {
            src: "order-bravo-1".into(),
            dst: "flow-9".into(),
            relation: "correlated_with".into(),
            confidence_method: ConfidenceMethod::InferredStat,
            causal: false,
            evidence_ids: vec!["ed-res-flow".into()],
            timestamp: at(0),
        },
    ];
    let evidence = vec![
        EvidenceView {
            id: "authz-finding-1".into(),
            kind: "finding".into(),
            timestamp: at(0),
            redaction: RedactionStatus::None,
            missing_telemetry: vec![],
        },
        EvidenceView {
            id: "gateway-evt-1".into(),
            kind: "event".into(),
            timestamp: at(0),
            redaction: RedactionStatus::Partial,
            missing_telemetry: vec!["response_body".into()],
        },
    ];

    let report_markdown = InvestigationReport::new(&incident, &response, &evidence, &edges).to_markdown();

    ScenarioOutput {
        response,
        incident,
        report_markdown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grounded_claims_survive_and_the_overreach_is_dropped() {
        let out = run();
        match &out.response {
            Response::Answer { claims } => {
                // 2 facts + 1 valid causal (on the observed accesses edge) = 3; the
                // correlation-based causal claim is rejected.
                assert_eq!(claims.len(), 3);
                assert!(claims.iter().all(|c| !c.claim_text.contains("exfiltrated")));
            }
            Response::Refusal { .. } => panic!("should have grounded answers"),
        }
    }

    #[test]
    fn incident_is_confirmed_with_a_consistent_audit_trail() {
        let out = run();
        assert_eq!(out.incident.state, IncidentState::Confirmed);
        assert!(out.incident.audit_is_consistent());
        assert_eq!(out.incident.evidence_ids.len(), 3);
    }

    #[test]
    fn report_renders_and_distinguishes_observed_from_inferred() {
        let out = run();
        let md = &out.report_markdown;
        assert!(md.contains("# Incident inc-7"));
        assert!(md.contains("● assistant-7"));      // observed
        assert!(md.contains("◇ order-bravo-1"));    // stat-inferred
        assert!(md.contains("association only"));
        assert!(md.contains("[FACT]"));
        assert!(md.contains("[CAUSAL]"));
        assert!(!md.contains("exfiltrated"));        // the rejected claim never appears
    }
}
