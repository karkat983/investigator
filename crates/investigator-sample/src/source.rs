//! A small in-memory evidence source for the sample.
//!
//! It serves exactly the evidence the scenario needs — one finding, one agent event, one
//! gateway event, and the graph edges between them — and enforces tenant scoping, so the
//! grounding layer behaves against it just as it would against the real platform.

use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};
use lattivant_event::{ActorKind, Assertion, Event};

use investigator_ground::{EdgeRecord, EvidenceSource, FindingRecord, GraphQuery};

fn at(min: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 1, 9, min, 0).unwrap()
}

/// The sample source.
pub struct SampleSource {
    events: HashMap<(String, String), Event>,
    findings: HashMap<(String, String), FindingRecord>,
    edges: Vec<EdgeRecord>,
}

impl Default for SampleSource {
    fn default() -> Self {
        let mut events = HashMap::new();
        // the agent's observed tool call that performed the cross-tenant read
        let agent_event = Event::builder("agent.tool_call", "alpha")
            .event_id("agent-span-1")
            .source_engine("agenttrace")
            .actor(ActorKind::AiAgent, "assistant-7")
            .action("tool", "get_order")
            .target("id", "order-bravo-1")
            .outcome("read", Some(200))
            .assertion(Assertion::Observed)
            .timestamp(at(0))
            .observed_at(at(0))
            .build()
            .unwrap();
        // the gateway's observed confirmation
        let gateway_event = Event::builder("net.http", "alpha")
            .event_id("gateway-evt-1")
            .source_engine("authz")
            .actor(ActorKind::Service, "api-gateway")
            .action("operation", "GET /orders/{id}")
            .target("owner_tenant", "bravo")
            .outcome("read", Some(200))
            .assertion(Assertion::Observed)
            .timestamp(at(0))
            .observed_at(at(0))
            .build()
            .unwrap();
        events.insert(("alpha".into(), "agent-span-1".into()), agent_event);
        events.insert(("alpha".into(), "gateway-evt-1".into()), gateway_event);

        let mut findings = HashMap::new();
        findings.insert(
            ("alpha".into(), "authz-finding-1".into()),
            FindingRecord {
                id: "authz-finding-1".into(),
                tenant_id: "alpha".into(),
                title: "Verified BOLA on GET /orders/{id}".into(),
                verified: true,
                supporting_event_ids: vec!["gateway-evt-1".into()],
            },
        );

        let edges = vec![
            EdgeRecord {
                id: "ed-agent-res".into(),
                tenant_id: "alpha".into(),
                src: "assistant-7".into(),
                dst: "order-bravo-1".into(),
                relation: "accesses".into(),
                observed: true,
                causal: true,
                valid_from: at(0),
            },
            EdgeRecord {
                id: "ed-res-flow".into(),
                tenant_id: "alpha".into(),
                src: "order-bravo-1".into(),
                dst: "flow-9".into(),
                relation: "correlated_with".into(),
                observed: false,
                causal: false,
                valid_from: at(0),
            },
        ];

        SampleSource {
            events,
            findings,
            edges,
        }
    }
}

impl EvidenceSource for SampleSource {
    fn get_event(&self, tenant: &str, id: &str) -> Option<Event> {
        self.events.get(&(tenant.to_string(), id.to_string())).cloned()
    }

    fn get_finding(&self, tenant: &str, id: &str) -> Option<FindingRecord> {
        self.findings.get(&(tenant.to_string(), id.to_string())).cloned()
    }

    fn query_graph(&self, tenant: &str, query: &GraphQuery) -> Vec<EdgeRecord> {
        self.edges
            .iter()
            .filter(|e| e.tenant_id == tenant && (e.src == query.node || e.dst == query.node))
            .take(query.limit)
            .cloned()
            .collect()
    }
}
