#[path = "../src/metric_cohorts.rs"]
mod metric_cohorts;
#[path = "../src/metrics.rs"]
mod metrics;

use metric_cohorts::{compare_groups, Identity, Outcome, Row, Spec, Status};

fn identity() -> Identity {
    Identity {
        dataset: "synthetic".into(),
        endpoint: "finish".into(),
        unit: "minute".into(),
        mapping: "mapping.v1".into(),
        seed_schedule: "schedule.v1".into(),
        seed_map: "map.v1".into(),
    }
}
fn spec() -> Spec {
    Spec {
        identity: identity(),
        groups: vec!["a".into(), "b".into()],
        window: Some((10, 20)),
        algorithm_version: "empirical_equal.v1".into(),
        origin: None,
        scale_ticks: "60".into(),
        provenance_verified: true,
    }
}
fn point(key: &str, time: Option<u128>, value: &str) -> Row {
    Row {
        key: key.into(),
        group: "a".into(),
        selection_time: time,
        value: Some(value.into()),
        weight: None,
        outcome: Outcome::Point,
        excluded: false,
        censored: false,
        missing: false,
        failed: false,
        infeasible: false,
    }
}
#[test]
fn source_window_and_empty_stratum_preserve_coverage() {
    let r = vec![
        point("start", Some(10), "0"),
        point("end", Some(20), "1000"),
        point("missing-time", None, "500"),
    ];
    let mut simulation = point("prediction", Some(10), "1");
    simulation.infeasible = true;
    let s = vec![simulation];
    let id = identity();
    let got = compare_groups(&spec(), &id, &id, &r, &s);
    assert_eq!(got[0].status, Status::Computed);
    assert_eq!(got[0].w1, Some(1.0));
    assert_eq!(got[0].reference_count, 1);
    assert_eq!(got[0].reference_diagnostics.raw, 3);
    assert_eq!(got[0].reference_diagnostics.excluded, 2);
    assert_eq!(got[0].reference_diagnostics.missing, 1);
    assert_eq!(got[0].simulation_diagnostics.infeasible, 1);
    assert_eq!(got[0].simulation_count, 1);
    assert_eq!(got[0].unmatched_count, 0);
    assert_eq!(got[1].status, Status::Empty);
    assert_eq!(got[1].w1, None);
}
#[test]
fn identities_duplicates_and_unknown_groups_fail_without_losing_attempts() {
    let id = identity();
    let mut other = id.clone();
    other.unit = "second".into();
    let r = vec![point("a", Some(10), "0")];
    let s = vec![point("b", Some(10), "1")];
    let got = compare_groups(&spec(), &id, &other, &r, &s);
    assert_eq!(got[0].status, Status::Invalid);
    assert_eq!(got[0].reference_count, 0);
    assert_eq!(got[0].w1, None);
    let duplicate = vec![r[0].clone(), r[0].clone()];
    assert_eq!(
        compare_groups(&spec(), &id, &id, &duplicate, &s)[0].status,
        Status::Invalid
    );
    let mut unknown = r[0].clone();
    unknown.group = "unplanned".into();
    let got = compare_groups(&spec(), &id, &id, &[unknown], &s);
    assert!(got.iter().all(|g| g.status == Status::Invalid));
    assert_eq!(
        got.iter()
            .find(|g| g.group == "unplanned")
            .unwrap()
            .reference_diagnostics
            .raw,
        1
    );
    assert_eq!(got[0].batch_reference_count, 1);
}
#[test]
fn permutation_provenance_weight_and_window_negatives() {
    let id = identity();
    let r = vec![point("z", Some(11), "2"), point("a", Some(10), "0")];
    let s = vec![point("q", Some(11), "3"), point("b", Some(10), "1")];
    let a = compare_groups(&spec(), &id, &id, &r, &s);
    let mut reversed = r.clone();
    reversed.reverse();
    assert_eq!(a, compare_groups(&spec(), &id, &id, &reversed, &s));
    let mut cfg = spec();
    cfg.provenance_verified = false;
    assert_eq!(
        compare_groups(&cfg, &id, &id, &r, &s)[0].status,
        Status::Unverified
    );
    cfg.provenance_verified = true;
    cfg.window = Some((20, 10));
    assert_eq!(
        compare_groups(&cfg, &id, &id, &r, &s)[0].status,
        Status::Invalid
    );
    let mut explicit = r.clone();
    explicit[0].weight = Some("1".into());
    assert_eq!(
        compare_groups(&spec(), &id, &id, &explicit, &s)[0].status,
        Status::Invalid
    );
}
#[test]
fn nonpoint_outcomes_remain_counted_with_overlap() {
    let id = identity();
    let mut r = Vec::new();
    for (i, outcome) in [
        Outcome::Missing,
        Outcome::Censored,
        Outcome::Failed,
        Outcome::Infeasible,
    ]
    .into_iter()
    .enumerate()
    {
        let mut row = point(&i.to_string(), Some(10), "0");
        row.value = None;
        row.outcome = outcome;
        row.excluded = true;
        row.censored = true;
        r.push(row);
    }
    let got = compare_groups(&spec(), &id, &id, &r, &[]);
    assert_eq!(got[0].status, Status::Empty);
    assert_eq!(got[0].reference_diagnostics.raw, 4);
    assert_eq!(got[0].reference_diagnostics.excluded, 4);
    assert_eq!(got[0].reference_diagnostics.censored, 4);
    assert_eq!(got[0].reference_diagnostics.failed, 1);
    assert_eq!(got[0].reference_diagnostics.infeasible, 1);
    assert_eq!(
        got[0].coverage_warnings,
        vec![
            "censored_observations_present",
            "excluded_observations_present",
            "failed_outcomes_present",
            "infeasible_outcomes_present",
            "missing_outcomes_present",
            "uncensored_subset_no_survival_correction",
        ]
    );
    r[0].value = Some("0".into());
    assert_eq!(
        compare_groups(&spec(), &id, &id, &r, &[])[0].status,
        Status::Invalid
    );
}

#[test]
fn tied_warning_uses_selected_supports_and_unverified_ties_stay_unknown() {
    let id = identity();
    let r = vec![point("a", Some(10), "1/2"), point("b", Some(11), "0.5")];
    let s = vec![point("c", Some(10), "1/2")];
    let got = compare_groups(&spec(), &id, &id, &r, &s);
    assert_eq!(got[0].reference_tie_count, Some(1));
    assert_eq!(got[0].simulation_tie_count, Some(0));
    assert!(got[0]
        .coverage_warnings
        .iter()
        .any(|w| w == "tied_observations_present"));
    let mut cfg = spec();
    cfg.provenance_verified = false;
    let got = compare_groups(&cfg, &id, &id, &r, &s);
    assert_eq!(got[0].reference_tie_count, None);
    assert_eq!(got[0].simulation_tie_count, None);
    assert!(!got[0]
        .coverage_warnings
        .iter()
        .any(|w| w == "tied_observations_present"));
    let mut attempted = r.clone();
    let mut censored = point("censored", Some(10), "0");
    censored.value = None;
    censored.outcome = Outcome::Censored;
    attempted.push(censored);
    let got = compare_groups(&cfg, &id, &id, &attempted, &s);
    assert_eq!(got[0].status, Status::Unverified);
    assert_eq!(got[0].reference_tie_count, None);
    assert!(got[0]
        .coverage_warnings
        .contains(&"censored_observations_present".into()));
    assert!(got[0]
        .coverage_warnings
        .contains(&"uncensored_subset_no_survival_correction".into()));
}

#[test]
fn excluded_censor_warns_without_inventing_ties_for_empty_declared_group() {
    let id = identity();
    let mut censored = point("c", Some(10), "0");
    censored.value = None;
    censored.outcome = Outcome::Censored;
    censored.excluded = true;
    let got = compare_groups(&spec(), &id, &id, &[censored], &[]);
    assert_eq!(got[0].status, Status::Empty);
    assert_eq!(got[0].reference_tie_count, Some(0));
    assert!(got[0]
        .coverage_warnings
        .contains(&"uncensored_subset_no_survival_correction".into()));
    assert!(got[0]
        .coverage_warnings
        .contains(&"excluded_observations_present".into()));
    assert_eq!(got[1].coverage_warnings, Vec::<String>::new());
}
