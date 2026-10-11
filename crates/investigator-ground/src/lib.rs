//! Evidence-grounded AI investigation.
//!
//! This crate helps an analyst interpret evidence **without inventing facts**. It gives an
//! investigator exactly three read-only, tenant-scoped, audited tools, constrains every
//! statement to a cited [`Claim`], and runs a [`PostChecker`] that rejects anything the
//! evidence does not support — refusing outright when nothing can be grounded.
//!
//! * [`evidence`] — the read-only [`EvidenceSource`] and the record kinds it returns (there are
//!   no write tools anywhere: no blocking, no policy change, no response actions);
//! * [`tools`] — the three-tool allowlist (`get_event`, `get_finding`, `query_graph`), each
//!   call tenant-scoped and recorded in an audit log;
//! * [`claim`] — the constrained response shape: `Fact` / `Causal` / `Hypothesis`, each with
//!   evidence ids and stated uncertainty;
//! * [`checker`] — the post-checker that verifies citations exist, belong to the tenant, and
//!   fit the claim kind (correlation can never pass as causation), and refuses when evidence is
//!   insufficient.
//!
//! ```
//! use investigator_ground::{PostChecker, Claim, Response, Evidence, FindingRecord};
//!
//! let finding = Evidence::Finding(FindingRecord {
//!     id: "f1".into(), tenant_id: "alpha".into(), title: "BOLA".into(),
//!     verified: true, supporting_event_ids: vec!["e1".into()],
//! });
//! let checker = PostChecker::new("alpha", &[finding]);
//!
//! // a well-cited fact is kept; an uncited one is dropped, and if nothing survives we refuse.
//! let resp = checker.respond(vec![Claim::fact("a verified BOLA finding exists", vec!["f1".into()])]);
//! assert!(matches!(resp, Response::Answer { .. }));
//! ```

pub mod checker;
pub mod claim;
pub mod evidence;
pub mod tools;

pub use checker::{CheckOutcome, PostChecker};
pub use claim::{Claim, ClaimKind, Response, Uncertainty};
pub use evidence::{
    EdgeRecord, Evidence, EvidenceKind, EvidenceSource, FindingRecord, GraphQuery,
};
pub use tools::{AuditEntry, Tool, ToolSession};
