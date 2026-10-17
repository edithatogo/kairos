#[allow(dead_code)]
#[path = "../src/residuals.rs"]
mod residuals;
#[allow(dead_code)]
#[path = "../src/trace_order.rs"]
mod trace_order;
#[allow(dead_code)]
mod seed_map {
    #[derive(Clone, Debug, Eq, PartialEq)]
    pub(crate) struct CalibrationStreamKey;
}
#[allow(dead_code)]
#[path = "../src/shadow.rs"]
mod shadow;
#[path = "../src/shadow_ledger.rs"]
mod shadow_ledger;

use shadow::{ObservedEvent, ResourcePolicy, ShadowError, Transition};
use shadow_ledger::{InitialClaim, InitialResource, LedgerLimits, ObservedLedger};
use std::collections::BTreeMap;
use trace_order::{EventKindRank, TraceOrderKeyV1};

fn ev(tick: u128, key: &str, available_at: Option<u128>, transition: Transition) -> ObservedEvent {
    ObservedEvent {
        order: TraceOrderKeyV1 {
            relative_ticks: tick,
            case_key: "case".into(),
            occurrence: 0,
            event_kind_rank: EventKindRank::from_canonical_decimal("1").unwrap(),
            source_event_key: key.into(),
            source_order: tick as u64,
        },
        available_at,
        source_defined: true,
        transition,
        payload: vec![tick as u8],
    }
}
fn resource() -> BTreeMap<String, InitialResource> {
    BTreeMap::from([(
        "bed".into(),
        InitialResource {
            capacity: 2,
            claims: vec![InitialClaim {
                id: "baseline".into(),
                units: 1,
            }],
        },
    )])
}
fn ledger(events: Vec<ObservedEvent>, policy: ResourcePolicy) -> ObservedLedger {
    ObservedLedger::new(
        events,
        resource(),
        vec!["baseline occupancy declared".into()],
        policy,
        LedgerLimits {
            max_events: 16,
            max_payload_bytes: 128,
            max_assumptions: 16,
            max_identifier_bytes: 512,
            max_initial_resources: 4,
            max_initial_claims: 16,
            max_assumption_bytes: 1024,
        },
    )
    .unwrap()
}

#[test]
fn canonical_order_snapshot_claims_and_digest_are_stable() {
    let a = ev(
        2,
        "acquire",
        Some(2),
        Transition::Acquire {
            resource: "bed".into(),
            claim: "second".into(),
            units: 1,
        },
    );
    let b = ev(1, "plain", Some(1), Transition::None);
    let mut subject = ledger(vec![a.clone(), b.clone()], ResourcePolicy::Strict);
    let first = subject.advance().unwrap().unwrap().clone();
    assert_eq!(first.anchor_event, "plain");
    let second = subject.advance().unwrap().unwrap().clone();
    assert_eq!(second.visible_events, vec![b.clone(), a]);
    assert_eq!(second.resources["bed"].claims["baseline"], 1);
    assert_eq!(second.resources["bed"].claims["second"], 1);
    let mut again = ledger(
        vec![
            b,
            ev(
                2,
                "acquire",
                Some(2),
                Transition::Acquire {
                    resource: "bed".into(),
                    claim: "second".into(),
                    units: 1,
                },
            ),
        ],
        ResourcePolicy::Strict,
    );
    again.advance().unwrap();
    assert_eq!(again.advance().unwrap().unwrap().digest, second.digest);
}

#[test]
fn future_knowledge_never_changes_earlier_occupancy() {
    let future = ev(
        1,
        "future-acquire",
        Some(10),
        Transition::Acquire {
            resource: "bed".into(),
            claim: "future".into(),
            units: 1,
        },
    );
    let anchor = ev(5, "anchor", Some(5), Transition::None);
    let later = ev(10, "known", Some(10), Transition::None);
    let mut inferred = ev(
        2,
        "inferred",
        Some(2),
        Transition::Acquire {
            resource: "bed".into(),
            claim: "inferred".into(),
            units: 1,
        },
    );
    inferred.source_defined = false;
    let mut subject = ledger(
        vec![future, anchor, later, inferred],
        ResourcePolicy::Strict,
    );
    assert!(matches!(
        subject.advance(),
        Err(ShadowError::UnavailableAnchor(_))
    ));
    assert!(matches!(
        subject.advance(),
        Err(ShadowError::UnavailableAnchor(_))
    ));
    let at_five = subject.advance().unwrap().unwrap().clone();
    assert!(!at_five.resources["bed"].claims.contains_key("future"));
    assert_eq!(at_five.visible_events.len(), 1);
    assert!(at_five
        .assumptions
        .iter()
        .any(|a| a.contains("future-acquire")));
    let at_ten = subject.advance().unwrap().unwrap();
    assert!(at_ten.resources["bed"].claims.contains_key("future"));
    assert!(!at_ten.resources["bed"].claims.contains_key("inferred"));
}

#[test]
fn unavailable_anchor_is_never_exposed_as_eligible_snapshot() {
    let mut unknown = ev(1, "unknown", None, Transition::None);
    unknown.source_defined = false;
    let mut subject = ledger(vec![unknown], ResourcePolicy::Strict);
    assert!(matches!(
        subject.advance(),
        Err(ShadowError::UnavailableAnchor(_))
    ));
    assert!(matches!(
        subject.current(),
        Err(ShadowError::UnavailableAnchor(_))
    ));
    assert_eq!(subject.frontier(), 1);
}

#[test]
fn strict_policy_checks_late_known_historical_occupancy() {
    let impossible = ev(
        1,
        "late-impossible",
        Some(10),
        Transition::Acquire {
            resource: "bed".into(),
            claim: "too-many".into(),
            units: 2,
        },
    );
    let mut subject = ledger(vec![impossible], ResourcePolicy::Strict);
    assert!(matches!(
        subject.advance(),
        Err(ShadowError::ResourceInfeasible(_))
    ));
    assert_eq!(subject.frontier(), 0);
    assert_eq!(subject.diagnostics()[0].event, "late-impossible");
}

#[test]
fn diagnostic_policy_keeps_late_historical_failure_without_leaking_claim() {
    let impossible = ev(
        1,
        "late-impossible",
        Some(10),
        Transition::Acquire {
            resource: "bed".into(),
            claim: "too-many".into(),
            units: 2,
        },
    );
    let anchor = ev(5, "anchor", Some(5), Transition::None);
    let mut subject = ledger(vec![impossible, anchor], ResourcePolicy::Diagnostic);
    assert!(matches!(
        subject.advance(),
        Err(ShadowError::UnavailableAnchor(_))
    ));
    let snapshot = subject.advance().unwrap().unwrap();
    assert!(!snapshot.resources["bed"].claims.contains_key("too-many"));
    assert!(subject
        .diagnostics()
        .iter()
        .any(|d| d.event == "late-impossible"));
}

#[test]
fn strict_infeasible_transition_does_not_commit_frontier() {
    let over = ev(
        1,
        "over",
        Some(1),
        Transition::Acquire {
            resource: "bed".into(),
            claim: "extra".into(),
            units: 2,
        },
    );
    let mut subject = ledger(vec![over], ResourcePolicy::Strict);
    assert!(matches!(
        subject.advance(),
        Err(ShadowError::ResourceInfeasible(_))
    ));
    assert_eq!(subject.frontier(), 0);
    assert!(subject.current().unwrap().is_none());
    assert_eq!(subject.diagnostics().len(), 1);
}

#[test]
fn diagnostic_mode_keeps_invalid_row_and_reports_capacity_violation() {
    let over = ev(
        1,
        "over",
        Some(1),
        Transition::Acquire {
            resource: "bed".into(),
            claim: "extra".into(),
            units: 2,
        },
    );
    let mut subject = ledger(vec![over.clone()], ResourcePolicy::Diagnostic);
    let snapshot = subject.advance().unwrap().unwrap();
    assert!(!snapshot.resource_feasible);
    assert_eq!(snapshot.visible_events, vec![over]);
    assert_eq!(
        subject.diagnostics()[0].reason,
        "occupancy exceeds capacity"
    );
}

#[test]
fn duplicate_source_key_and_unknown_release_reject_or_diagnose() {
    let duplicate = ev(2, "same", Some(2), Transition::None);
    assert!(matches!(
        ObservedLedger::new(
            vec![duplicate.clone(), duplicate],
            resource(),
            vec![],
            ResourcePolicy::Strict,
            LedgerLimits {
                max_events: 4,
                max_payload_bytes: 20,
                max_assumptions: 2,
                max_identifier_bytes: 64,
                max_initial_resources: 2,
                max_initial_claims: 4,
                max_assumption_bytes: 64,
            }
        ),
        Err(ShadowError::DuplicateIdentity(_))
    ));
    let release = ev(
        1,
        "release",
        Some(1),
        Transition::Release {
            resource: "bed".into(),
            claim: "absent".into(),
        },
    );
    let mut subject = ledger(vec![release], ResourcePolicy::Diagnostic);
    subject.advance().unwrap();
    assert_eq!(
        subject.diagnostics()[0].reason,
        "release names no active claim"
    );
}

#[test]
fn bounds_and_initial_claim_validation_fail_closed() {
    assert!(matches!(
        ObservedLedger::new(
            vec![ev(1, "e", Some(1), Transition::None)],
            resource(),
            vec![],
            ResourcePolicy::Strict,
            LedgerLimits {
                max_events: 0,
                max_payload_bytes: 0,
                max_assumptions: 0,
                max_identifier_bytes: 0,
                max_initial_resources: 0,
                max_initial_claims: 0,
                max_assumption_bytes: 0,
            }
        ),
        Err(ShadowError::LimitExceeded)
    ));
    let bad = BTreeMap::from([(
        "bed".into(),
        InitialResource {
            capacity: 1,
            claims: vec![InitialClaim {
                id: "x".into(),
                units: 2,
            }],
        },
    )]);
    assert!(ObservedLedger::new(
        vec![],
        bad,
        vec![],
        ResourcePolicy::Strict,
        LedgerLimits {
            max_events: 1,
            max_payload_bytes: 1,
            max_assumptions: 1,
            max_identifier_bytes: 16,
            max_initial_resources: 1,
            max_initial_claims: 1,
            max_assumption_bytes: 16,
        }
    )
    .is_err());
}

#[test]
fn copied_identifiers_assumptions_and_initial_inventory_are_bounded() {
    let event = ev(1, "long-event-key", Some(1), Transition::None);
    let limits = LedgerLimits {
        max_events: 4,
        max_payload_bytes: 32,
        max_assumptions: 4,
        max_identifier_bytes: 8,
        max_initial_resources: 4,
        max_initial_claims: 4,
        max_assumption_bytes: 64,
    };
    assert!(matches!(
        ObservedLedger::new(
            vec![event],
            BTreeMap::new(),
            vec![],
            ResourcePolicy::Strict,
            limits
        ),
        Err(ShadowError::LimitExceeded)
    ));

    let limits = LedgerLimits {
        max_identifier_bytes: 256,
        ..limits
    };
    let too_many_resources = BTreeMap::from([
        (
            "one".into(),
            InitialResource {
                capacity: 1,
                claims: vec![],
            },
        ),
        (
            "two".into(),
            InitialResource {
                capacity: 1,
                claims: vec![],
            },
        ),
    ]);
    assert!(matches!(
        ObservedLedger::new(
            vec![],
            too_many_resources,
            vec![],
            ResourcePolicy::Strict,
            LedgerLimits {
                max_initial_resources: 1,
                ..limits
            }
        ),
        Err(ShadowError::LimitExceeded)
    ));
    let too_many_claims = BTreeMap::from([(
        "bed".into(),
        InitialResource {
            capacity: 2,
            claims: vec![
                InitialClaim {
                    id: "a".into(),
                    units: 1,
                },
                InitialClaim {
                    id: "b".into(),
                    units: 1,
                },
            ],
        },
    )]);
    assert!(matches!(
        ObservedLedger::new(
            vec![],
            too_many_claims,
            vec![],
            ResourcePolicy::Strict,
            LedgerLimits {
                max_initial_claims: 1,
                ..limits
            }
        ),
        Err(ShadowError::LimitExceeded)
    ));
    assert!(matches!(
        ObservedLedger::new(
            vec![],
            BTreeMap::new(),
            vec!["long assumption".into()],
            ResourcePolicy::Strict,
            LedgerLimits {
                max_assumption_bytes: 4,
                ..limits
            }
        ),
        Err(ShadowError::LimitExceeded)
    ));
}
