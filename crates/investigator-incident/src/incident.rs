//! An incident and its immutable audit history.
//!
//! Every change to an incident — a status transition, an attached evidence id, an assignment, a
//! note, a recorded decision — appends an [`AuditEvent`] carrying who did it, when, and why. The
//! audit vector is append-only: there is no API to edit or remove a past entry, so the history
//! is a faithful record. Each mutation also bumps a version, which the store uses to detect
//! concurrent edits.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::state::IncidentState;

/// A single, immutable audit record.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditEvent {
    /// When it happened.
    pub at: DateTime<Utc>,
    /// Who did it (an analyst or service principal id).
    pub actor: String,
    /// What happened.
    pub action: AuditAction,
    /// The stated reason.
    pub reason: String,
}

/// The kinds of change recorded in the audit trail.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum AuditAction {
    /// The incident was created.
    Created,
    /// The state changed.
    StatusChanged {
        /// Previous state.
        from: IncidentState,
        /// New state.
        to: IncidentState,
    },
    /// An evidence id was attached.
    EvidenceAttached {
        /// The evidence id.
        evidence_id: String,
    },
    /// An owner was assigned.
    OwnerAssigned {
        /// The new owner.
        owner: String,
    },
    /// A note was added.
    NoteAdded,
    /// A decision was recorded.
    DecisionRecorded {
        /// The decision text.
        decision: String,
    },
}

/// Why an incident operation failed.
#[derive(Debug, Error, PartialEq)]
pub enum IncidentError {
    /// The requested state transition is not allowed.
    #[error("cannot transition from {from:?} to {to:?}")]
    InvalidTransition {
        /// Current state.
        from: IncidentState,
        /// Attempted state.
        to: IncidentState,
    },
    /// A reason was required but empty.
    #[error("a reason is required for this action")]
    ReasonRequired,
}

/// A security incident / case.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Incident {
    /// Stable id.
    pub id: String,
    /// Owning tenant.
    pub tenant_id: String,
    /// A short title.
    pub title: String,
    /// Current state.
    pub state: IncidentState,
    /// The assigned owner, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// Attached evidence ids.
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    /// Free-text notes.
    #[serde(default)]
    pub notes: Vec<String>,
    /// Monotonic version, bumped on every mutation (for optimistic concurrency).
    pub version: u64,
    /// When created.
    pub created_at: DateTime<Utc>,
    /// When last updated.
    pub updated_at: DateTime<Utc>,
    /// Append-only audit history.
    pub audit: Vec<AuditEvent>,
}

impl Incident {
    /// Open a new incident in state `New`.
    pub fn open(
        id: impl Into<String>,
        tenant_id: impl Into<String>,
        title: impl Into<String>,
        actor: impl Into<String>,
        at: DateTime<Utc>,
    ) -> Self {
        let mut inc = Incident {
            id: id.into(),
            tenant_id: tenant_id.into(),
            title: title.into(),
            state: IncidentState::New,
            owner: None,
            evidence_ids: Vec::new(),
            notes: Vec::new(),
            version: 1,
            created_at: at,
            updated_at: at,
            audit: Vec::new(),
        };
        inc.audit.push(AuditEvent {
            at,
            actor: actor.into(),
            action: AuditAction::Created,
            reason: "incident opened".into(),
        });
        inc
    }

    fn touch(&mut self, at: DateTime<Utc>, actor: String, action: AuditAction, reason: String) {
        self.version += 1;
        self.updated_at = at;
        self.audit.push(AuditEvent {
            at,
            actor,
            action,
            reason,
        });
    }

    /// Transition to a new state, recording actor and reason.
    pub fn transition(
        &mut self,
        to: IncidentState,
        actor: impl Into<String>,
        reason: impl Into<String>,
        at: DateTime<Utc>,
    ) -> Result<(), IncidentError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(IncidentError::ReasonRequired);
        }
        if !self.state.can_transition_to(to) {
            return Err(IncidentError::InvalidTransition { from: self.state, to });
        }
        let from = self.state;
        self.state = to;
        self.touch(at, actor.into(), AuditAction::StatusChanged { from, to }, reason);
        Ok(())
    }

    /// Attach an evidence id (idempotent — attaching the same id twice is a no-op).
    pub fn attach_evidence(&mut self, evidence_id: impl Into<String>, actor: impl Into<String>, at: DateTime<Utc>) {
        let evidence_id = evidence_id.into();
        if self.evidence_ids.contains(&evidence_id) {
            return;
        }
        self.evidence_ids.push(evidence_id.clone());
        self.touch(
            at,
            actor.into(),
            AuditAction::EvidenceAttached { evidence_id },
            "evidence attached".into(),
        );
    }

    /// Assign an owner.
    pub fn assign(&mut self, owner: impl Into<String>, actor: impl Into<String>, at: DateTime<Utc>) {
        let owner = owner.into();
        self.owner = Some(owner.clone());
        self.touch(at, actor.into(), AuditAction::OwnerAssigned { owner }, "owner assigned".into());
    }

    /// Add a note.
    pub fn add_note(&mut self, note: impl Into<String>, actor: impl Into<String>, at: DateTime<Utc>) {
        let note = note.into();
        self.notes.push(note);
        self.touch(at, actor.into(), AuditAction::NoteAdded, "note added".into());
    }

    /// Record a decision.
    pub fn record_decision(&mut self, decision: impl Into<String>, actor: impl Into<String>, at: DateTime<Utc>) {
        let decision = decision.into();
        self.touch(
            at,
            actor.into(),
            AuditAction::DecisionRecorded { decision: decision.clone() },
            decision,
        );
    }

    /// Mark the incident a false positive: dismiss it with the reason recorded.
    pub fn mark_false_positive(&mut self, actor: impl Into<String>, reason: impl Into<String>, at: DateTime<Utc>) -> Result<(), IncidentError> {
        self.transition(IncidentState::Dismissed, actor, reason, at)
    }

    /// Whether the audit history is internally consistent: created first, versions monotonic.
    pub fn audit_is_consistent(&self) -> bool {
        matches!(self.audit.first().map(|e| &e.action), Some(AuditAction::Created))
            && self.audit.len() as u64 == self.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, s).unwrap()
    }

    fn incident() -> Incident {
        Incident::open("inc-1", "alpha", "Cross-tenant read", "analyst-1", at(0))
    }

    #[test]
    fn opening_records_a_created_audit_entry() {
        let inc = incident();
        assert_eq!(inc.state, IncidentState::New);
        assert_eq!(inc.version, 1);
        assert!(matches!(inc.audit[0].action, AuditAction::Created));
        assert!(inc.audit_is_consistent());
    }

    #[test]
    fn valid_transition_updates_state_and_audits() {
        let mut inc = incident();
        inc.transition(IncidentState::Investigating, "analyst-1", "triaging", at(1)).unwrap();
        assert_eq!(inc.state, IncidentState::Investigating);
        assert_eq!(inc.version, 2);
        assert!(matches!(
            inc.audit.last().unwrap().action,
            AuditAction::StatusChanged { from: IncidentState::New, to: IncidentState::Investigating }
        ));
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let mut inc = incident();
        let err = inc.transition(IncidentState::Resolved, "a", "x", at(1)).unwrap_err();
        assert!(matches!(err, IncidentError::InvalidTransition { .. }));
        // state unchanged, version unchanged
        assert_eq!(inc.state, IncidentState::New);
        assert_eq!(inc.version, 1);
    }

    #[test]
    fn transition_requires_a_reason() {
        let mut inc = incident();
        assert!(matches!(
            inc.transition(IncidentState::Investigating, "a", "  ", at(1)),
            Err(IncidentError::ReasonRequired)
        ));
    }

    #[test]
    fn evidence_attachment_is_idempotent() {
        let mut inc = incident();
        inc.attach_evidence("ev-1", "a", at(1));
        inc.attach_evidence("ev-1", "a", at(2));
        assert_eq!(inc.evidence_ids, vec!["ev-1"]);
        // only one attach audit entry (plus created)
        assert_eq!(inc.version, 2);
    }

    #[test]
    fn full_lifecycle_keeps_a_consistent_audit_trail() {
        let mut inc = incident();
        inc.assign("analyst-2", "analyst-1", at(1));
        inc.transition(IncidentState::Investigating, "analyst-2", "looking", at(2)).unwrap();
        inc.attach_evidence("ev-1", "analyst-2", at(3));
        inc.transition(IncidentState::Confirmed, "analyst-2", "reproduced", at(4)).unwrap();
        inc.record_decision("notify data owner", "analyst-2", at(5));
        inc.transition(IncidentState::Resolved, "analyst-2", "remediated", at(6)).unwrap();
        assert_eq!(inc.state, IncidentState::Resolved);
        assert!(inc.audit_is_consistent());
    }

    #[test]
    fn mark_false_positive_dismisses_with_reason() {
        let mut inc = incident();
        inc.transition(IncidentState::Investigating, "a", "triage", at(1)).unwrap();
        inc.mark_false_positive("a", "legitimate admin action", at(2)).unwrap();
        assert_eq!(inc.state, IncidentState::Dismissed);
    }
}
