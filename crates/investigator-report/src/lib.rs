//! Investigation view models and report export.
//!
//! This crate is the data contract behind Lattivant's investigation UI and its report export.
//! It renders what the UX spec insists on: observed edges kept visibly distinct from inferred
//! ones, and every item showing its evidence ids, timestamps, confidence method, missing
//! telemetry and redaction status.
//!
//! * [`view`] — [`EdgeView`] and [`EvidenceView`] with explicit provenance and redaction, so
//!   the UI (and the Markdown export) can never present an inference as an observed fact;
//! * [`report`] — [`InvestigationReport`], which assembles an incident, the investigator's
//!   grounded claims (or refusal), the evidence and the relationships into a deterministic
//!   Markdown document.
//!
//! ```
//! use investigator_report::InvestigationReport;
//! use investigator_ground::{Response, Claim};
//! use investigator_incident::Incident;
//! use chrono::{TimeZone, Utc};
//!
//! let t0 = Utc.with_ymd_and_hms(2026,1,1,0,0,0).unwrap();
//! let inc = Incident::open("inc-1", "alpha", "Cross-tenant read", "analyst-1", t0);
//! let resp = Response::Answer { claims: vec![Claim::fact("a verified finding exists", vec!["f1".into()])] };
//! let report = InvestigationReport::new(&inc, &resp, &[], &[]);
//! assert!(report.to_markdown().contains("# Incident inc-1"));
//! ```

pub mod report;
pub mod view;

pub use report::InvestigationReport;
pub use view::{ConfidenceMethod, EdgeView, EvidenceView, RedactionStatus};
