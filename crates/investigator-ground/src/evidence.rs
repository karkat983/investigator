//! The read-only evidence the investigator may consult.
//!
//! The investigator never reaches into databases directly; it goes through an
//! [`EvidenceSource`] that returns **tenant-scoped, redacted** records and nothing else. The
//! three record kinds — events, findings, and graph edges — mirror the only three read-only
//! tools the investigator is allowed to call. There are no write tools anywhere in this crate:
//! the investigator cannot block, change policy, or take any response action.

use chrono::{DateTime, Utc};
use lattivant_event::Event;
use serde::{Deserialize, Serialize};

/// The kind of evidence a record is — used by the post-checker to judge whether a claim's
/// citations actually support the *kind* of claim being made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// A raw observed event.
    Event,
    /// A security finding (itself may be observed or inferred).
    Finding,
    /// A graph edge connecting two entities.
    GraphEdge,
}

/// A finding record, reduced to what the investigator needs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FindingRecord {
    /// The finding id.
    pub id: String,
    /// The owning tenant.
    pub tenant_id: String,
    /// A short title.
    pub title: String,
    /// Whether the finding is verified (reproducible) as opposed to probable/speculative.
    pub verified: bool,
    /// Event ids that back the finding.
    pub supporting_event_ids: Vec<String>,
}

/// A graph edge record, reduced to what the investigator needs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgeRecord {
    /// The edge id.
    pub id: String,
    /// The owning tenant.
    pub tenant_id: String,
    /// Source node.
    pub src: String,
    /// Destination node.
    pub dst: String,
    /// The relationship label (e.g. `accesses`, `correlated_with`).
    pub relation: String,
    /// Whether the edge is observed (true) or inferred (false).
    pub observed: bool,
    /// Whether this relation is causal (a `correlated_with` edge is not).
    pub causal: bool,
    /// When the edge was valid from.
    pub valid_from: DateTime<Utc>,
}

/// A piece of retrieved evidence, tagged with its kind and tenant.
#[derive(Clone, Debug, PartialEq)]
pub enum Evidence {
    /// An event.
    Event(Box<Event>),
    /// A finding.
    Finding(FindingRecord),
    /// A graph edge.
    Edge(EdgeRecord),
}

impl Evidence {
    /// The evidence id.
    pub fn id(&self) -> &str {
        match self {
            Evidence::Event(e) => e.event_id.as_str(),
            Evidence::Finding(f) => &f.id,
            Evidence::Edge(e) => &e.id,
        }
    }

    /// The owning tenant.
    pub fn tenant_id(&self) -> &str {
        match self {
            Evidence::Event(e) => e.tenant_id.as_str(),
            Evidence::Finding(f) => &f.tenant_id,
            Evidence::Edge(e) => &e.tenant_id,
        }
    }

    /// The evidence kind.
    pub fn kind(&self) -> EvidenceKind {
        match self {
            Evidence::Event(_) => EvidenceKind::Event,
            Evidence::Finding(_) => EvidenceKind::Finding,
            Evidence::Edge(_) => EvidenceKind::GraphEdge,
        }
    }
}

/// A simple graph query the investigator may run.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphQuery {
    /// The node to search around.
    pub node: String,
    /// Maximum edges to return.
    pub limit: usize,
}

/// The read-only, tenant-scoped evidence source. Implementations must return only the named
/// tenant's data and must already have applied redaction.
pub trait EvidenceSource {
    /// Fetch an event by id within a tenant.
    fn get_event(&self, tenant: &str, id: &str) -> Option<Event>;
    /// Fetch a finding by id within a tenant.
    fn get_finding(&self, tenant: &str, id: &str) -> Option<FindingRecord>;
    /// Query graph edges around a node within a tenant.
    fn query_graph(&self, tenant: &str, query: &GraphQuery) -> Vec<EdgeRecord>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattivant_event::{ActorKind, Assertion};

    fn event(id: &str, tenant: &str) -> Event {
        Event::builder("agent.tool_call", tenant)
            .event_id(id)
            .source_engine("agenttrace")
            .actor(ActorKind::AiAgent, "a")
            .outcome("read", Some(200))
            .assertion(Assertion::Observed)
            .build()
            .unwrap()
    }

    #[test]
    fn evidence_exposes_id_tenant_and_kind() {
        let ev = Evidence::Event(Box::new(event("e1", "alpha")));
        assert_eq!(ev.id(), "e1");
        assert_eq!(ev.tenant_id(), "alpha");
        assert_eq!(ev.kind(), EvidenceKind::Event);

        let f = Evidence::Finding(FindingRecord {
            id: "f1".into(),
            tenant_id: "alpha".into(),
            title: "BOLA".into(),
            verified: true,
            supporting_event_ids: vec!["e1".into()],
        });
        assert_eq!(f.kind(), EvidenceKind::Finding);
    }
}
