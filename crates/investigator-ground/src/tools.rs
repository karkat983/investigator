//! The read-only tool allowlist, retrieval, and audit log.
//!
//! The investigator can call exactly three tools — `get_event`, `get_finding`, `query_graph` —
//! and every call is tenant-scoped and audited. There is deliberately no fourth tool: nothing
//! here can write, block, or change policy. Each retrieval records an [`AuditEntry`] so an
//! investigation can be replayed and reviewed.

use std::cell::RefCell;

use crate::evidence::{EdgeRecord, Evidence, EvidenceSource, FindingRecord, GraphQuery};

/// The allowlisted read-only tools.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    /// Fetch a single event.
    GetEvent,
    /// Fetch a single finding.
    GetFinding,
    /// Query graph edges around a node.
    QueryGraph,
}

impl Tool {
    /// The tool's stable name.
    pub fn name(self) -> &'static str {
        match self {
            Tool::GetEvent => "get_event",
            Tool::GetFinding => "get_finding",
            Tool::QueryGraph => "query_graph",
        }
    }

    /// Whether a tool name is on the allowlist.
    pub fn is_allowed(name: &str) -> bool {
        matches!(name, "get_event" | "get_finding" | "query_graph")
    }
}

/// One recorded tool invocation.
#[derive(Clone, Debug, PartialEq)]
pub struct AuditEntry {
    /// The tenant the call was scoped to.
    pub tenant_id: String,
    /// The tool called.
    pub tool: Tool,
    /// The argument (id or node).
    pub argument: String,
    /// How many evidence items were returned.
    pub returned: usize,
}

/// A tenant-scoped session over an evidence source, recording every retrieval.
pub struct ToolSession<'s, S: EvidenceSource> {
    source: &'s S,
    tenant_id: String,
    audit: RefCell<Vec<AuditEntry>>,
}

impl<'s, S: EvidenceSource> ToolSession<'s, S> {
    /// Open a session for a tenant.
    pub fn new(source: &'s S, tenant_id: impl Into<String>) -> Self {
        ToolSession {
            source,
            tenant_id: tenant_id.into(),
            audit: RefCell::new(Vec::new()),
        }
    }

    fn record(&self, tool: Tool, argument: &str, returned: usize) {
        self.audit.borrow_mut().push(AuditEntry {
            tenant_id: self.tenant_id.clone(),
            tool,
            argument: argument.to_string(),
            returned,
        });
    }

    /// `get_event` — always scoped to this session's tenant.
    pub fn get_event(&self, id: &str) -> Option<Evidence> {
        let ev = self.source.get_event(&self.tenant_id, id);
        self.record(Tool::GetEvent, id, ev.is_some() as usize);
        ev.map(|e| Evidence::Event(Box::new(e)))
    }

    /// `get_finding`.
    pub fn get_finding(&self, id: &str) -> Option<Evidence> {
        let f: Option<FindingRecord> = self.source.get_finding(&self.tenant_id, id);
        self.record(Tool::GetFinding, id, f.is_some() as usize);
        f.map(Evidence::Finding)
    }

    /// `query_graph` around a node.
    pub fn query_graph(&self, node: &str, limit: usize) -> Vec<Evidence> {
        let edges: Vec<EdgeRecord> = self
            .source
            .query_graph(&self.tenant_id, &GraphQuery { node: node.into(), limit });
        self.record(Tool::QueryGraph, node, edges.len());
        edges.into_iter().map(Evidence::Edge).collect()
    }

    /// The audit log of this session.
    pub fn audit(&self) -> Vec<AuditEntry> {
        self.audit.borrow().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::EdgeRecord;
    use chrono::{TimeZone, Utc};
    use lattivant_event::{ActorKind, Assertion, Event};

    #[derive(Default)]
    struct FakeSource;

    impl EvidenceSource for FakeSource {
        fn get_event(&self, tenant: &str, id: &str) -> Option<Event> {
            if tenant == "alpha" && id == "e1" {
                Some(
                    Event::builder("agent.tool_call", tenant)
                        .event_id(id)
                        .source_engine("agenttrace")
                        .actor(ActorKind::AiAgent, "a")
                        .outcome("read", Some(200))
                        .assertion(Assertion::Observed)
                        .build()
                        .unwrap(),
                )
            } else {
                None
            }
        }
        fn get_finding(&self, _tenant: &str, _id: &str) -> Option<FindingRecord> {
            None
        }
        fn query_graph(&self, tenant: &str, query: &GraphQuery) -> Vec<EdgeRecord> {
            if tenant == "alpha" && query.node == "agent-1" {
                vec![EdgeRecord {
                    id: "ed1".into(),
                    tenant_id: "alpha".into(),
                    src: "agent-1".into(),
                    dst: "res-1".into(),
                    relation: "accesses".into(),
                    observed: true,
                    causal: true,
                    valid_from: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
                }]
            } else {
                Vec::new()
            }
        }
    }

    #[test]
    fn allowlist_only_recognizes_three_tools() {
        assert!(Tool::is_allowed("get_event"));
        assert!(Tool::is_allowed("query_graph"));
        assert!(!Tool::is_allowed("delete_event"));
        assert!(!Tool::is_allowed("block_ip"));
    }

    #[test]
    fn retrieval_is_tenant_scoped_and_audited() {
        let src = FakeSource;
        let session = ToolSession::new(&src, "alpha");
        assert!(session.get_event("e1").is_some());
        assert_eq!(session.query_graph("agent-1", 10).len(), 1);
        let audit = session.audit();
        assert_eq!(audit.len(), 2);
        assert_eq!(audit[0].tool, Tool::GetEvent);
        assert_eq!(audit[1].returned, 1);
    }

    #[test]
    fn a_different_tenant_session_sees_nothing() {
        let src = FakeSource;
        let session = ToolSession::new(&src, "bravo");
        assert!(session.get_event("e1").is_none());
        assert!(session.query_graph("agent-1", 10).is_empty());
    }
}
