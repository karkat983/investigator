//! Incident and case management.
//!
//! The analyst workflow from alert to documented case: open an incident, assign it, attach
//! evidence, add notes, move it through its lifecycle, record decisions, and mark false
//! positives — all on a strict [state machine](state::IncidentState) with an **immutable audit
//! trail** of who did what, when, and why.
//!
//! * [`state`] — the lifecycle and its allowed transitions (confirmation and dismissal are
//!   analyst decisions, never automatic);
//! * [`incident`] — the [`Incident`] with its append-only audit history and monotonic version;
//! * [`store`] — a tenant-scoped [`IncidentStore`] with optimistic-concurrency commits and
//!   evidence-based deduplication.
//!
//! ```
//! use investigator_incident::{Incident, IncidentState};
//! use chrono::{TimeZone, Utc};
//!
//! let t0 = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
//! let mut inc = Incident::open("inc-1", "alpha", "Cross-tenant read", "analyst-1", t0);
//! inc.transition(IncidentState::Investigating, "analyst-1", "triaging", t0).unwrap();
//! inc.attach_evidence("authz-finding-1", "analyst-1", t0);
//! inc.transition(IncidentState::Confirmed, "analyst-1", "reproduced", t0).unwrap();
//! assert!(inc.audit_is_consistent());
//! ```

pub mod incident;
pub mod state;
pub mod store;

pub use incident::{AuditAction, AuditEvent, Incident, IncidentError};
pub use state::IncidentState;
pub use store::{CreateOutcome, IncidentStore, StoreError};
