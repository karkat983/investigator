//! Sample: a grounded investigation from finding to confirmed incident and report.
//!
//! Running this binary plays the built-in scenario and prints the rendered Markdown report —
//! which includes the investigator's grounded claims (the over-reaching causal claim having
//! been dropped by the post-checker), the incident's state and audit trail, the relationships
//! with observed and inferred edges marked distinctly, and the evidence with redaction status.

mod scenario;
mod source;

fn main() {
    let out = scenario::run();

    let grounded = match &out.response {
        investigator_ground::Response::Answer { claims } => claims.len(),
        investigator_ground::Response::Refusal { .. } => 0,
    };

    eprintln!(
        "incident {} is {:?}; {} grounded claim(s) survived the post-checker\n",
        out.incident.id, out.incident.state, grounded
    );
    println!("{}", out.report_markdown);
}
