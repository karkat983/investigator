# investigator

The analyst-facing engine of [Lattivant](https://github.com/karkat983), an evidence-first
security platform. It helps an analyst interpret evidence **without inventing facts**, manages
the case from alert to documented incident, and renders the whole investigation so observed facts
and inferences are never confused.

The non-negotiable rule: **no unsupported factual sentence passes**. The AI investigator may only
read evidence through a tiny allowlist of tools, every statement it makes is a cited claim, and a
post-checker rejects anything whose citations don't exist, belong to another tenant, or don't fit
the claim — so correlation can never be dressed up as causation, and when nothing can be grounded
the investigator refuses rather than guess. It takes **no autonomous action**: there is no tool
to block, change policy, or respond.

Evidence is read as the platform's canonical events from
[`lattivant-platform`](https://github.com/karkat983/lattivant-platform).

## Crates

| Crate | What it provides |
|-------|------------------|
| [`investigator-ground`](crates/investigator-ground) | The evidence-grounded core: a read-only, tenant-scoped, audited tool allowlist (`get_event`, `get_finding`, `query_graph`), a constrained claim shape (fact / causal / hypothesis with citations and uncertainty), and a **post-checker** that verifies every citation and refuses when evidence is insufficient. No write tools exist anywhere. |
| [`investigator-incident`](crates/investigator-incident) | Case management: an explicit incident state machine (confirmation and dismissal are analyst decisions), an **immutable audit trail** of who/when/why, and a tenant-scoped store with optimistic-concurrency commits and evidence-based deduplication. |
| [`investigator-report`](crates/investigator-report) | The UI/report data contract: view models that render observed edges distinctly from inferred ones and surface evidence ids, timestamps, confidence method, missing telemetry and redaction status, plus a deterministic Markdown report exporter. |
| [`investigator-sample`](crates/investigator-sample) | A worked end-to-end investigation: from a verified finding to the agent event to the gateway confirmation, through the post-checker, into a confirmed incident and a rendered report. |

## The post-checker, in one line

A claim survives only if its citations **exist**, are in the **investigation's tenant**, and
**fit the claim kind** — and a `Causal` claim must rest on observed, causal evidence, never on a
`correlated_with` edge. Everything else is dropped; if nothing survives, the investigator refuses.

## Try it

```sh
cargo run -p investigator-sample
```

The scenario starts from a verified BOLA finding and follows it, in a few steps, to the agent
tool call and the gateway event that confirm it. The investigator proposes four claims; the
post-checker keeps the three that are grounded and **drops the one that asserts exfiltration from
a mere correlation**. An incident is opened, advanced to *confirmed* with a consistent audit
trail, and a Markdown report is rendered with observed and inferred edges marked distinctly.

```sh
cargo test              # all crates
cargo clippy --all-targets
```

Requires a recent stable Rust toolchain (edition 2021, rustc ≥ 1.80).

## License

MIT. See [LICENSE](LICENSE).

## Authorized testing only

All tenants, identities, resources and events in this repository are synthetic. Use these tools
only against systems you own or are explicitly authorized to test.
