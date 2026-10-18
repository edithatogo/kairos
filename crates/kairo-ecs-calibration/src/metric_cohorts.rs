//! Private fixed-cohort adapter. Selection never uses a prediction as a clock.
use crate::metrics::{compare, MetricRequest, MetricStatus, Precision};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    pub dataset: String,
    pub endpoint: String,
    pub unit: String,
    pub mapping: String,
    pub seed_schedule: String,
    pub seed_map: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    Point,
    Missing,
    Censored,
    Failed,
    Infeasible,
}

#[derive(Clone, Debug)]
pub(crate) struct Row {
    pub key: String,
    pub group: String,
    pub selection_time: Option<u128>,
    pub value: Option<String>,
    pub weight: Option<String>,
    pub outcome: Outcome,
    pub excluded: bool,
    pub censored: bool,
    pub missing: bool,
    pub failed: bool,
    pub infeasible: bool,
}

#[derive(Debug)]
pub(crate) struct Spec {
    pub identity: Identity,
    pub groups: Vec<String>,
    pub window: Option<(u128, u128)>,
    pub algorithm_version: String,
    pub origin: Option<String>,
    pub scale_ticks: String,
    pub provenance_verified: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counts {
    pub raw: usize,
    pub excluded: usize,
    pub censored: usize,
    pub missing: usize,
    pub failed: usize,
    pub infeasible: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Computed,
    Empty,
    InsufficientData,
    Invalid,
    Unverified,
}

#[derive(Debug, PartialEq)]
pub(crate) struct GroupMetric {
    pub group: String,
    pub status: Status,
    pub w1: Option<f64>,
    pub ks_d: Option<f64>,
    pub reference_count: usize,
    pub simulation_count: usize,
    pub reference_tie_count: Option<usize>,
    pub simulation_tie_count: Option<usize>,
    pub coverage_warnings: Vec<String>,
    pub reference_diagnostics: Counts,
    pub simulation_diagnostics: Counts,
    pub unmatched_count: usize,
    /// Whole-batch attempted rows; never infer this from overlapping diagnostics.
    pub batch_reference_count: usize,
    pub batch_simulation_count: usize,
    pub precision: Precision,
}

type Selected<'a> = (Counts, Vec<&'a Row>);

fn select<'a>(rows: &'a [Row], group: &str, window: Option<(u128, u128)>) -> Selected<'a> {
    let mut counts = Counts::default();
    let mut selected = Vec::new();
    for row in rows.iter().filter(|r| r.group == group) {
        counts.raw += 1;
        let outside = window.is_some_and(|(start, end)| {
            row.selection_time
                .is_none_or(|time| time < start || time >= end)
        });
        counts.excluded += usize::from(row.excluded || outside);
        counts.censored += usize::from(row.censored || row.outcome == Outcome::Censored);
        counts.missing += usize::from(
            row.missing
                || row.outcome == Outcome::Missing
                || (window.is_some() && row.selection_time.is_none()),
        );
        counts.failed += usize::from(row.failed || row.outcome == Outcome::Failed);
        counts.infeasible += usize::from(row.infeasible || row.outcome == Outcome::Infeasible);
        if !row.excluded && !outside && row.outcome == Outcome::Point {
            selected.push(row);
        }
    }
    selected.sort_by(|a, b| a.key.cmp(&b.key));
    (counts, selected)
}

fn valid_rows(rows: &[Row], groups: &BTreeSet<&str>) -> bool {
    let mut keys = BTreeSet::new();
    rows.iter().all(|row| {
        !row.key.is_empty()
            && keys.insert(row.key.as_str())
            && groups.contains(row.group.as_str())
            && ((row.outcome == Outcome::Point) == row.value.is_some())
    })
}

/// Compare externally fixed groups with independently retained coverage counts.
pub(crate) fn compare_groups(
    spec: &Spec,
    reference_identity: &Identity,
    simulation_identity: &Identity,
    reference: &[Row],
    simulation: &[Row],
) -> Vec<GroupMetric> {
    let groups: BTreeSet<&str> = spec.groups.iter().map(String::as_str).collect();
    let identity_valid = [
        &spec.identity.dataset,
        &spec.identity.endpoint,
        &spec.identity.unit,
        &spec.identity.mapping,
        &spec.identity.seed_schedule,
        &spec.identity.seed_map,
    ]
    .iter()
    .all(|s| !s.is_empty());
    let invalid = !identity_valid
        || reference_identity != &spec.identity
        || simulation_identity != &spec.identity
        || groups.len() != spec.groups.len()
        || groups.is_empty()
        || groups.contains("")
        || spec.window.is_some_and(|(start, end)| start >= end)
        || !valid_rows(reference, &groups)
        || !valid_rows(simulation, &groups);
    // Unknown input groups remain visible as invalid diagnostic groups.
    // They never acquire authority to become computed strata.
    let mut output_groups = groups;
    output_groups.extend(
        reference
            .iter()
            .chain(simulation)
            .map(|row| row.group.as_str()),
    );
    if output_groups.is_empty() {
        output_groups.insert("");
    }
    let mut output = BTreeMap::new();
    for group in output_groups {
        let (reference_diagnostics, r) = select(reference, group, spec.window);
        let (simulation_diagnostics, s) = select(simulation, group, spec.window);
        let rv: Vec<_> = r.iter().map(|row| row.value.as_deref()).collect();
        let sv: Vec<_> = s.iter().map(|row| row.value.as_deref()).collect();
        let rw: Option<Vec<&str>> = r.iter().map(|row| row.weight.as_deref()).collect();
        let sw: Option<Vec<&str>> = s.iter().map(|row| row.weight.as_deref()).collect();
        let weighted = spec.algorithm_version == "weighted_descriptive.v1";
        let incompatible_weights = if weighted {
            rw.is_none() || sw.is_none()
        } else {
            r.iter().chain(&s).any(|row| row.weight.is_some())
        };
        let failed_preconditions = invalid || incompatible_weights;
        let result = if failed_preconditions || !spec.provenance_verified {
            None
        } else {
            Some(compare(&MetricRequest {
                reference: &rv,
                simulation: &sv,
                reference_weights: if weighted { rw.as_deref() } else { None },
                simulation_weights: if weighted { sw.as_deref() } else { None },
                algorithm_version: &spec.algorithm_version,
                origin: spec.origin.as_deref(),
                scale_ticks: &spec.scale_ticks,
            }))
        };
        let status = if failed_preconditions {
            Status::Invalid
        } else if !spec.provenance_verified {
            Status::Unverified
        } else {
            match result.as_ref().expect("preconditions checked").status {
                MetricStatus::Computed => Status::Computed,
                MetricStatus::Empty => Status::Empty,
                MetricStatus::InsufficientData => Status::InsufficientData,
                MetricStatus::Invalid => Status::Invalid,
            }
        };
        let reference_tie_count = result.as_ref().and_then(|r| r.reference_tie_count);
        let simulation_tie_count = result.as_ref().and_then(|r| r.simulation_tie_count);
        let mut coverage_warnings = BTreeSet::new();
        if reference_diagnostics.censored > 0 || simulation_diagnostics.censored > 0 {
            coverage_warnings.insert("censored_observations_present".to_owned());
        }
        if reference_diagnostics.excluded > 0 || simulation_diagnostics.excluded > 0 {
            coverage_warnings.insert("excluded_observations_present".to_owned());
        }
        if reference_diagnostics.failed > 0 || simulation_diagnostics.failed > 0 {
            coverage_warnings.insert("failed_outcomes_present".to_owned());
        }
        if reference_diagnostics.infeasible > 0 || simulation_diagnostics.infeasible > 0 {
            coverage_warnings.insert("infeasible_outcomes_present".to_owned());
        }
        if reference_diagnostics.missing > 0 || simulation_diagnostics.missing > 0 {
            coverage_warnings.insert("missing_outcomes_present".to_owned());
        }
        if reference_tie_count.is_some_and(|count| count > 0)
            || simulation_tie_count.is_some_and(|count| count > 0)
        {
            coverage_warnings.insert("tied_observations_present".to_owned());
        }
        if reference
            .iter()
            .chain(simulation)
            .any(|row| row.group == group && row.outcome == Outcome::Censored)
        {
            coverage_warnings.insert("uncensored_subset_no_survival_correction".to_owned());
        }
        output.insert(
            group,
            GroupMetric {
                group: group.to_owned(),
                status,
                w1: result.as_ref().and_then(|r| r.w1),
                ks_d: result.as_ref().and_then(|r| r.ks_d),
                reference_count: result.as_ref().map_or(0, |r| r.reference_count),
                simulation_count: result.as_ref().map_or(0, |r| r.simulation_count),
                reference_tie_count,
                simulation_tie_count,
                coverage_warnings: coverage_warnings.into_iter().collect(),
                reference_diagnostics,
                simulation_diagnostics,
                unmatched_count: 0,
                batch_reference_count: reference.len(),
                batch_simulation_count: simulation.len(),
                precision: result.map_or(Precision::NotApplicable, |r| r.precision),
            },
        );
    }
    output.into_values().collect()
}
