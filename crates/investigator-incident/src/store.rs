//! A tenant-scoped incident store with optimistic concurrency and dedup.
//!
//! Two analysts editing the same case must not silently clobber each other, so commits are
//! optimistic: a writer declares the version it started from, and a commit is rejected if the
//! stored version has moved on since. Access is tenant-scoped — a case is invisible outside its
//! tenant — and new incidents can be deduplicated against existing ones that rest on the same
//! evidence, so one real event does not spawn a pile of duplicate cases.

use std::collections::HashMap;

use thiserror::Error;

use crate::incident::Incident;

/// Why a store operation failed.
#[derive(Debug, Error, PartialEq)]
pub enum StoreError {
    /// The incident was not found in the tenant.
    #[error("incident {id:?} not found in tenant {tenant:?}")]
    NotFound {
        /// Tenant.
        tenant: String,
        /// Incident id.
        id: String,
    },
    /// A concurrent edit moved the stored version past what the writer started from.
    #[error("version conflict on {id:?}: expected {expected}, stored {stored}")]
    VersionConflict {
        /// Incident id.
        id: String,
        /// The version the writer started from.
        expected: u64,
        /// The version currently stored.
        stored: u64,
    },
    /// The updated incident's tenant did not match the stored one's.
    #[error("tenant mismatch committing {id:?}")]
    TenantMismatch {
        /// Incident id.
        id: String,
    },
}

/// The result of a deduplicating create.
#[derive(Debug, PartialEq)]
pub enum CreateOutcome {
    /// A new incident was stored.
    Created,
    /// An existing incident covers the same evidence; its id is returned.
    Duplicate {
        /// The existing incident id.
        existing_id: String,
    },
}

/// A tenant-scoped incident store.
#[derive(Default)]
pub struct IncidentStore {
    incidents: HashMap<(String, String), Incident>,
}

impl IncidentStore {
    /// An empty store.
    pub fn new() -> Self {
        IncidentStore::default()
    }

    fn key(incident: &Incident) -> (String, String) {
        (incident.tenant_id.clone(), incident.id.clone())
    }

    /// Store a new incident unconditionally.
    pub fn create(&mut self, incident: Incident) {
        self.incidents.insert(Self::key(&incident), incident);
    }

    /// The dedup fingerprint of an incident: its sorted evidence set.
    fn fingerprint(incident: &Incident) -> String {
        let mut ids = incident.evidence_ids.clone();
        ids.sort();
        ids.join(",")
    }

    /// Store a new incident, unless one in the same tenant already rests on the same evidence.
    pub fn create_deduplicated(&mut self, incident: Incident) -> CreateOutcome {
        let fp = Self::fingerprint(&incident);
        if !fp.is_empty() {
            if let Some(existing) = self
                .incidents
                .values()
                .find(|i| i.tenant_id == incident.tenant_id && Self::fingerprint(i) == fp)
            {
                return CreateOutcome::Duplicate {
                    existing_id: existing.id.clone(),
                };
            }
        }
        let id = incident.id.clone();
        self.create(incident);
        let _ = id;
        CreateOutcome::Created
    }

    /// Fetch a tenant's incident.
    pub fn get(&self, tenant: &str, id: &str) -> Option<&Incident> {
        self.incidents.get(&(tenant.to_string(), id.to_string()))
    }

    /// Commit an edited incident, rejecting if the stored version has moved past
    /// `expected_version` (a concurrent edit).
    pub fn commit(&mut self, updated: Incident, expected_version: u64) -> Result<(), StoreError> {
        let key = Self::key(&updated);
        let stored = self.incidents.get(&key).ok_or_else(|| StoreError::NotFound {
            tenant: updated.tenant_id.clone(),
            id: updated.id.clone(),
        })?;
        if stored.tenant_id != updated.tenant_id {
            return Err(StoreError::TenantMismatch { id: updated.id.clone() });
        }
        if stored.version != expected_version {
            return Err(StoreError::VersionConflict {
                id: updated.id.clone(),
                expected: expected_version,
                stored: stored.version,
            });
        }
        self.incidents.insert(key, updated);
        Ok(())
    }

    /// Count of incidents in a tenant.
    pub fn count(&self, tenant: &str) -> usize {
        self.incidents.keys().filter(|(t, _)| t == tenant).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::IncidentState;
    use chrono::{DateTime, TimeZone, Utc};

    fn at(s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, s).unwrap()
    }

    fn incident(id: &str, tenant: &str) -> Incident {
        Incident::open(id, tenant, "case", "analyst", at(0))
    }

    #[test]
    fn get_is_tenant_scoped() {
        let mut store = IncidentStore::new();
        store.create(incident("inc-1", "alpha"));
        assert!(store.get("alpha", "inc-1").is_some());
        // unauthorized cross-tenant access sees nothing
        assert!(store.get("bravo", "inc-1").is_none());
    }

    #[test]
    fn concurrent_edit_is_rejected_by_version_conflict() {
        let mut store = IncidentStore::new();
        store.create(incident("inc-1", "alpha"));

        // two analysts both read at version 1
        let mut a = store.get("alpha", "inc-1").unwrap().clone();
        let mut b = store.get("alpha", "inc-1").unwrap().clone();
        a.transition(IncidentState::Investigating, "a", "x", at(1)).unwrap();
        b.transition(IncidentState::Dismissed, "b", "y", at(1)).unwrap();

        // first commit (from version 1) succeeds
        assert!(store.commit(a, 1).is_ok());
        // second commit, also from version 1, now conflicts
        assert!(matches!(
            store.commit(b, 1),
            Err(StoreError::VersionConflict { expected: 1, stored: 2, .. })
        ));
    }

    #[test]
    fn commit_to_missing_incident_is_not_found() {
        let mut store = IncidentStore::new();
        let inc = incident("ghost", "alpha");
        assert!(matches!(store.commit(inc, 1), Err(StoreError::NotFound { .. })));
    }

    #[test]
    fn dedup_detects_same_evidence_set() {
        let mut store = IncidentStore::new();
        let mut first = incident("inc-1", "alpha");
        first.attach_evidence("ev-1", "a", at(1));
        first.attach_evidence("ev-2", "a", at(2));
        store.create(first);

        let mut dup = incident("inc-2", "alpha");
        dup.attach_evidence("ev-2", "a", at(3));
        dup.attach_evidence("ev-1", "a", at(4)); // same set, different order
        let outcome = store.create_deduplicated(dup);
        assert_eq!(outcome, CreateOutcome::Duplicate { existing_id: "inc-1".into() });
        assert_eq!(store.count("alpha"), 1);
    }

    #[test]
    fn dedup_allows_distinct_evidence() {
        let mut store = IncidentStore::new();
        let mut a = incident("inc-1", "alpha");
        a.attach_evidence("ev-1", "a", at(1));
        store.create(a);
        let mut b = incident("inc-2", "alpha");
        b.attach_evidence("ev-9", "a", at(1));
        assert_eq!(store.create_deduplicated(b), CreateOutcome::Created);
        assert_eq!(store.count("alpha"), 2);
    }
}
