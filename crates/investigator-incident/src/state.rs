//! The incident state machine.
//!
//! An incident moves through a small, explicit set of states, and only along allowed edges.
//! Confirmation and dismissal are analyst decisions, so they are transitions a person drives —
//! never something the system flips on its own. Terminal-ish states (`Dismissed`, `Resolved`)
//! can be reopened to `Investigating`, because real triage is not always a straight line.

use serde::{Deserialize, Serialize};

/// The lifecycle state of an incident.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentState {
    /// Just created, not yet triaged.
    New,
    /// Under active investigation.
    Investigating,
    /// Confirmed a real incident (analyst decision).
    Confirmed,
    /// Judged not an incident (analyst decision).
    Dismissed,
    /// Closed out after handling.
    Resolved,
}

impl IncidentState {
    /// Whether a transition to `next` is allowed from this state.
    pub fn can_transition_to(self, next: IncidentState) -> bool {
        use IncidentState::*;
        matches!(
            (self, next),
            (New, Investigating)
                | (New, Dismissed)
                | (Investigating, Confirmed)
                | (Investigating, Dismissed)
                | (Investigating, Resolved)
                | (Confirmed, Resolved)
                | (Confirmed, Investigating)   // reopen for more work
                | (Dismissed, Investigating)   // reopen on new evidence
                | (Resolved, Investigating)    // reopen if it recurs
        )
    }

    /// Whether this state is one a human must have chosen (not auto-reachable).
    pub fn requires_analyst_decision(self) -> bool {
        matches!(self, IncidentState::Confirmed | IncidentState::Dismissed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use IncidentState::*;

    #[test]
    fn happy_path_transitions_are_allowed() {
        assert!(New.can_transition_to(Investigating));
        assert!(Investigating.can_transition_to(Confirmed));
        assert!(Confirmed.can_transition_to(Resolved));
    }

    #[test]
    fn invalid_transitions_are_rejected() {
        assert!(!New.can_transition_to(Resolved));
        assert!(!New.can_transition_to(Confirmed));
        assert!(!Resolved.can_transition_to(Confirmed));
    }

    #[test]
    fn terminal_states_can_reopen_to_investigating() {
        assert!(Dismissed.can_transition_to(Investigating));
        assert!(Resolved.can_transition_to(Investigating));
    }

    #[test]
    fn confirmation_and_dismissal_need_a_person() {
        assert!(Confirmed.requires_analyst_decision());
        assert!(Dismissed.requires_analyst_decision());
        assert!(!Resolved.requires_analyst_decision());
    }
}
