//! The constrained shape of anything the investigator says.
//!
//! The investigator may not emit free prose. Every statement is a [`Claim`] with a kind, the
//! evidence ids it rests on, and a stated uncertainty. This is what makes grounding checkable:
//! a `Fact` with no surviving citation is rejected, a `Causal` claim resting only on a
//! correlation is rejected, and anything the evidence cannot support must be downgraded to
//! `Hypothesis` or dropped.

use serde::{Deserialize, Serialize};

/// What sort of statement a claim is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimKind {
    /// A factual assertion; must be backed by compatible, existing evidence.
    Fact,
    /// A causal assertion (X caused Y); needs observed, causal evidence — never correlation alone.
    Causal,
    /// An interpretation the evidence does not establish; allowed only if labelled as such.
    Hypothesis,
}

/// How confident the claim is presented as being.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Uncertainty {
    /// Presented as certain.
    Certain,
    /// Presented as likely.
    Likely,
    /// Presented as uncertain/open.
    Uncertain,
}

/// A single grounded claim.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    /// The human-readable statement.
    pub claim_text: String,
    /// Evidence ids cited in support.
    pub evidence_ids: Vec<String>,
    /// The kind of claim.
    pub claim_kind: ClaimKind,
    /// The stated uncertainty.
    pub uncertainty: Uncertainty,
}

impl Claim {
    /// A factual claim.
    pub fn fact(text: impl Into<String>, evidence_ids: Vec<String>) -> Self {
        Claim {
            claim_text: text.into(),
            evidence_ids,
            claim_kind: ClaimKind::Fact,
            uncertainty: Uncertainty::Certain,
        }
    }

    /// A causal claim.
    pub fn causal(text: impl Into<String>, evidence_ids: Vec<String>) -> Self {
        Claim {
            claim_text: text.into(),
            evidence_ids,
            claim_kind: ClaimKind::Causal,
            uncertainty: Uncertainty::Likely,
        }
    }

    /// A hypothesis.
    pub fn hypothesis(text: impl Into<String>) -> Self {
        Claim {
            claim_text: text.into(),
            evidence_ids: Vec::new(),
            claim_kind: ClaimKind::Hypothesis,
            uncertainty: Uncertainty::Uncertain,
        }
    }
}

/// A response is either a set of checked claims or an explicit refusal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Response {
    /// Grounded claims that passed the checker.
    Answer {
        /// The claims.
        claims: Vec<Claim>,
    },
    /// A refusal because evidence was insufficient.
    Refusal {
        /// Why the investigator refused.
        reason: String,
    },
}

impl Response {
    /// Whether this is a refusal.
    pub fn is_refusal(&self) -> bool {
        matches!(self, Response::Refusal { .. })
    }
}
