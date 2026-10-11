//! View models for the investigation UI.
//!
//! The UX spec requires that the interface render observed edges distinctly from inferred ones
//! and always show evidence ids, timestamps, the confidence method, missing telemetry, and
//! redaction status. These view models are the data contract behind that: the frontend renders
//! them, and because they carry the provenance explicitly, the UI can never accidentally present
//! an inference as an observed fact.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// How much of a piece of evidence was redacted before the investigator (and UI) saw it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionStatus {
    /// Nothing redacted.
    None,
    /// Some fields redacted.
    Partial,
    /// Content fully withheld; only metadata shown.
    Full,
}

/// How a relationship's confidence was established — shown next to every edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceMethod {
    /// Directly observed.
    Observed,
    /// Inferred by a rule.
    InferredRule,
    /// Inferred statistically.
    InferredStat,
}

impl ConfidenceMethod {
    /// Whether this is an observed (not inferred) method — the UI renders the two distinctly.
    pub fn is_observed(self) -> bool {
        matches!(self, ConfidenceMethod::Observed)
    }

    /// A glyph the UI can use to mark observed vs inferred at a glance.
    pub fn glyph(self) -> &'static str {
        match self {
            ConfidenceMethod::Observed => "●",      // solid: observed
            ConfidenceMethod::InferredRule => "◆",  // rule inference
            ConfidenceMethod::InferredStat => "◇",  // statistical inference
        }
    }
}

/// A rendered edge in the entity/graph view.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EdgeView {
    /// Source label.
    pub src: String,
    /// Destination label.
    pub dst: String,
    /// The relation label.
    pub relation: String,
    /// How the edge's confidence was established.
    pub confidence_method: ConfidenceMethod,
    /// Whether the relation is causal (a correlation is not).
    pub causal: bool,
    /// Evidence ids backing the edge.
    pub evidence_ids: Vec<String>,
    /// When the edge was valid from.
    pub timestamp: DateTime<Utc>,
}

/// A rendered evidence item.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EvidenceView {
    /// The evidence id.
    pub id: String,
    /// A short kind label (`event`, `finding`, `graph_edge`).
    pub kind: String,
    /// When the evidence was observed.
    pub timestamp: DateTime<Utc>,
    /// Redaction status.
    pub redaction: RedactionStatus,
    /// Telemetry known to be missing for this item (surfaced, not hidden).
    pub missing_telemetry: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observed_and_inferred_are_distinguishable() {
        assert!(ConfidenceMethod::Observed.is_observed());
        assert!(!ConfidenceMethod::InferredRule.is_observed());
        assert_ne!(
            ConfidenceMethod::Observed.glyph(),
            ConfidenceMethod::InferredStat.glyph()
        );
    }
}
