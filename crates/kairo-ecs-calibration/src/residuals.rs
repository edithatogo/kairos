//! Private paired point-residual summaries for the C4.2 runtime contract.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct LogicalKey {
    pub study_id: String,
    pub dataset_id: String,
    pub scenario_id: String,
    pub seed_schedule_id: String,
    pub replication_id: String,
    pub case_key: String,
    pub task_key: String,
    pub occurrence: u32,
    pub endpoint: String,
    pub seed_purpose: String,
    pub seed_map_ref: String,
    pub mapping_version: String,
}
impl LogicalKey {
    fn valid(&self) -> bool {
        [
            &self.study_id,
            &self.dataset_id,
            &self.scenario_id,
            &self.seed_schedule_id,
            &self.replication_id,
            &self.case_key,
            &self.task_key,
            &self.endpoint,
            &self.seed_purpose,
            &self.seed_map_ref,
            &self.mapping_version,
        ]
        .iter()
        .all(|s| !s.trim().is_empty())
            && matches!(
                self.seed_purpose.as_str(),
                "service" | "transit" | "behavior" | "calibration"
            )
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct GroupKey {
    pub strata: Vec<(String, String)>,
}
impl GroupKey {
    fn valid(&self) -> bool {
        let mut last: Option<&str> = None;
        for (key, _) in &self.strata {
            if key.trim().is_empty() || last.is_some_and(|prev| prev >= key.as_str()) {
                return false;
            }
            last = Some(key);
        }
        true
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum Side {
    Reference,
    Simulation,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum OutcomeStatus {
    Observed,
    Predicted,
    Missing,
    Censored,
    Failed,
    Infeasible,
}
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct Row {
    pub key: LogicalKey,
    pub side: Side,
    pub group: GroupKey,
    /// Externally selected source time, independent of predicted ticks.
    pub source_time: Option<u128>,
    pub status: OutcomeStatus,
    pub observed_ticks: Option<u128>,
    pub predicted_ticks: Option<u128>,
    pub prediction_unclamped: bool,
    pub tick_unit: String,
    pub provenance_supported: bool,
    pub excluded: bool,
    /// Independent feasibility diagnostic; a point prediction may still be used.
    pub infeasible: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResidualSign {
    Negative,
    Zero,
    Positive,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Residual {
    pub sign: ResidualSign,
    pub magnitude: u128,
}
impl Residual {
    pub(crate) fn between(predicted: u128, observed: u128) -> Self {
        if predicted > observed {
            Self {
                sign: ResidualSign::Positive,
                magnitude: predicted - observed,
            }
        } else if predicted < observed {
            Self {
                sign: ResidualSign::Negative,
                magnitude: observed - predicted,
            }
        } else {
            Self {
                sign: ResidualSign::Zero,
                magnitude: 0,
            }
        }
    }
    fn as_f64(self) -> f64 {
        let m = self.magnitude as f64;
        match self.sign {
            ResidualSign::Negative => -m,
            ResidualSign::Zero => 0.0,
            ResidualSign::Positive => m,
        }
    }
    fn loses_precision(self) -> bool {
        let bits = 128 - self.magnitude.leading_zeros();
        bits > 53 && self.magnitude.trailing_zeros() < bits - 53
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Window {
    pub start: u128,
    pub end_exclusive: u128,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct Counts {
    /// Valid point observations actually used in each population.
    pub reference: usize,
    pub simulation: usize,
    pub matched: usize,
    pub unmatched: usize,
    pub excluded: usize,
    pub censored: usize,
    pub missing: usize,
    pub failed: usize,
    pub infeasible: usize,
    pub missing_time: usize,
    /// All raw input record instances, retained even when validation fails.
    pub raw_reference: usize,
    pub raw_simulation: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SummaryStatus {
    Computed,
    Empty,
    InsufficientData,
    Invalid,
    Unverified,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PairedRow {
    pub key: LogicalKey,
    /// Vectors preserve duplicate and contradictory source rows for diagnosis.
    pub reference: Vec<Row>,
    pub simulation: Vec<Row>,
    pub residual: Option<Residual>,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Summary {
    pub algorithm_version: &'static str,
    pub group: GroupKey,
    pub status: SummaryStatus,
    pub counts: Counts,
    pub rows: Vec<PairedRow>,
    /// Descriptive binary64 values; division, compensation, and sqrt can round.
    pub bias: Option<f64>,
    pub mae: Option<f64>,
    pub rmse: Option<f64>,
    /// True when any exact residual magnitude loses information on f64 conversion.
    pub approximate: bool,
}
#[derive(Clone, Copy, Debug, Default)]
struct Compensated {
    sum: f64,
    correction: f64,
}
impl Compensated {
    fn add(&mut self, x: f64) {
        let t = self.sum + x;
        if self.sum.abs() >= x.abs() {
            self.correction += (self.sum - t) + x;
        } else {
            self.correction += (x - t) + self.sum;
        }
        self.sum = t;
    }
    fn value(self) -> f64 {
        self.sum + self.correction
    }
}

type PairMap = BTreeMap<LogicalKey, (Vec<Row>, Vec<Row>)>;
type InvalidGroupMap = BTreeMap<GroupKey, (Counts, PairMap)>;
type Cohort = (String, String, String, String, String, String, String);

pub(crate) fn summarize(
    rows: &[Row],
    declared_groups: &[GroupKey],
    window: Option<Window>,
) -> Vec<Summary> {
    let groups: BTreeSet<GroupKey> = if declared_groups.is_empty() {
        [GroupKey::default()].into_iter().collect()
    } else {
        declared_groups.iter().cloned().collect()
    };
    let mut invalid = (!declared_groups.is_empty() && declared_groups.len() != groups.len())
        || groups.iter().any(|g| !g.valid())
        || (declared_groups.is_empty() && rows.iter().any(|r| r.group != GroupKey::default()))
        || rows
            .iter()
            .any(|r| !groups.contains(&r.group) || !r.group.valid())
        || window.is_some_and(|w| w.start >= w.end_exclusive);

    // Build pairs once in logical-key order. Every input row remains in the
    // pair until batch validation has finished, including duplicate sides.
    let mut by_key: PairMap = BTreeMap::new();
    let mut cohort: Option<Cohort> = None;
    for row in rows {
        let c = (
            row.key.study_id.clone(),
            row.key.dataset_id.clone(),
            row.key.scenario_id.clone(),
            row.key.endpoint.clone(),
            row.key.mapping_version.clone(),
            row.key.seed_schedule_id.clone(),
            row.key.seed_map_ref.clone(),
        );
        if cohort.as_ref().is_some_and(|previous| previous != &c) {
            invalid = true;
        } else {
            cohort = Some(c);
        }
        if !row.key.valid() || row.tick_unit != "nanosecond" || !row_coherent(row) {
            invalid = true;
        }
        let pair = by_key.entry(row.key.clone()).or_default();
        let (own, other) = match row.side {
            Side::Reference => (&mut pair.0, &pair.1),
            Side::Simulation => (&mut pair.1, &pair.0),
        };
        if !own.is_empty() {
            invalid = true;
        }
        if other.first().is_some_and(|existing| {
            existing.group != row.group || existing.source_time != row.source_time
        }) {
            invalid = true;
        }
        own.push(row.clone());
    }
    if invalid {
        return invalid_summaries(rows, &groups, window);
    }

    let mut grouped: BTreeMap<GroupKey, PairMap> = groups
        .iter()
        .cloned()
        .map(|g| (g, BTreeMap::new()))
        .collect();
    for (key, (reference, simulation)) in by_key {
        let group = reference
            .first()
            .or_else(|| simulation.first())
            .unwrap()
            .group
            .clone();
        grouped
            .get_mut(&group)
            .unwrap()
            .insert(key, (reference, simulation));
    }
    grouped
        .into_iter()
        .map(|(group, pairs)| summarize_group(group, pairs, window))
        .collect()
}

fn row_coherent(row: &Row) -> bool {
    match (
        row.side,
        row.status,
        row.observed_ticks,
        row.predicted_ticks,
    ) {
        (Side::Reference, OutcomeStatus::Observed, Some(_), None) => true,
        (Side::Reference, OutcomeStatus::Missing | OutcomeStatus::Censored, None, None) => true,
        (Side::Reference, OutcomeStatus::Failed, _, None) => true,
        (Side::Simulation, OutcomeStatus::Predicted, None, Some(_)) => row.prediction_unclamped,
        (Side::Simulation, OutcomeStatus::Infeasible, None, Some(_)) => row.prediction_unclamped,
        (
            Side::Simulation,
            OutcomeStatus::Infeasible
            | OutcomeStatus::Failed
            | OutcomeStatus::Missing
            | OutcomeStatus::Censored,
            None,
            None,
        ) => !row.prediction_unclamped,
        _ => false,
    }
}

fn empty_summary(group: GroupKey, status: SummaryStatus) -> Summary {
    Summary {
        algorithm_version: "residual_summary.canonical_float.v1",
        group,
        status,
        counts: Counts::default(),
        rows: vec![],
        bias: None,
        mae: None,
        rmse: None,
        approximate: false,
    }
}
fn invalid_summaries(
    rows: &[Row],
    groups: &BTreeSet<GroupKey>,
    window: Option<Window>,
) -> Vec<Summary> {
    let mut targets = groups.clone();
    for row in rows {
        targets.insert(row.group.clone());
    }
    if targets.is_empty() {
        targets.insert(GroupKey::default());
    }
    let mut grouped: InvalidGroupMap = targets
        .into_iter()
        .map(|group| (group, (Counts::default(), BTreeMap::new())))
        .collect();
    for row in rows {
        let (counts, map) = grouped.get_mut(&row.group).unwrap();
        let pair = map.entry(row.key.clone()).or_default();
        match row.side {
            Side::Reference => {
                pair.0.push(row.clone());
                counts.raw_reference += 1;
            }
            Side::Simulation => {
                pair.1.push(row.clone());
                counts.raw_simulation += 1;
            }
        }
        if row.excluded
            || window.is_some_and(|w| {
                row.source_time
                    .is_none_or(|t| t < w.start || t >= w.end_exclusive)
            })
        {
            counts.excluded += 1;
        }
        if window.is_some() && row.source_time.is_none() {
            counts.missing_time += 1;
        }
        match row.status {
            OutcomeStatus::Censored => counts.censored += 1,
            OutcomeStatus::Missing => counts.missing += 1,
            OutcomeStatus::Failed => counts.failed += 1,
            OutcomeStatus::Infeasible => counts.infeasible += 1,
            _ => {}
        }
        if row.infeasible && row.status != OutcomeStatus::Infeasible {
            counts.infeasible += 1;
        }
        counts.unmatched += 1;
    }
    grouped
        .into_iter()
        .map(|(group, (counts, map))| {
            let rows = map
                .into_iter()
                .map(|(key, (mut reference, mut simulation))| {
                    reference.sort();
                    simulation.sort();
                    PairedRow {
                        key,
                        reference,
                        simulation,
                        residual: None,
                    }
                })
                .collect();
            Summary {
                counts,
                rows,
                ..empty_summary(group, SummaryStatus::Invalid)
            }
        })
        .collect()
}

fn summarize_group(group: GroupKey, pairs: PairMap, window: Option<Window>) -> Summary {
    let mut counts = Counts::default();
    let mut output = Vec::new();
    let mut errors = Vec::new();
    let mut unverified = false;
    for (key, (reference, simulation)) in pairs {
        let r = reference.first();
        let s = simulation.first();
        counts.raw_reference += reference.len();
        counts.raw_simulation += simulation.len();
        let mut eligible = [false; 2];
        for (index, records) in [&reference, &simulation].into_iter().enumerate() {
            if let Some(row) = records.first() {
                if !row.provenance_supported {
                    unverified = true;
                }
                let time_eligible = window.is_none_or(|w| {
                    row.source_time
                        .is_some_and(|t| w.start <= t && t < w.end_exclusive)
                });
                if row.excluded || !time_eligible {
                    counts.excluded += 1;
                }
                if window.is_some() && row.source_time.is_none() {
                    counts.missing_time += 1;
                }
                if row.infeasible || row.status == OutcomeStatus::Infeasible {
                    counts.infeasible += 1;
                }
                match row.status {
                    OutcomeStatus::Censored => counts.censored += 1,
                    OutcomeStatus::Missing => counts.missing += 1,
                    OutcomeStatus::Failed => counts.failed += 1,
                    _ => {}
                }
                let has_point = match (row.side, row.status) {
                    (Side::Reference, OutcomeStatus::Observed) => true,
                    (Side::Simulation, OutcomeStatus::Predicted | OutcomeStatus::Infeasible) => {
                        row.predicted_ticks.is_some()
                    }
                    _ => false,
                };
                eligible[index] =
                    has_point && !row.excluded && time_eligible && row.provenance_supported;
            }
        }
        if eligible[0] {
            counts.reference += 1;
        }
        if eligible[1] {
            counts.simulation += 1;
        }
        let residual = if eligible[0] && eligible[1] {
            let error = Residual::between(
                s.unwrap().predicted_ticks.unwrap(),
                r.unwrap().observed_ticks.unwrap(),
            );
            errors.push((key.clone(), error));
            counts.matched += 1;
            Some(error)
        } else {
            counts.unmatched += reference.len() + simulation.len();
            None
        };
        output.push(PairedRow {
            key,
            reference,
            simulation,
            residual,
        });
    }
    let status = if unverified {
        SummaryStatus::Unverified
    } else if counts.reference == 0 && counts.simulation == 0 {
        SummaryStatus::Empty
    } else if errors.is_empty() {
        SummaryStatus::InsufficientData
    } else {
        SummaryStatus::Computed
    };
    let mut summary = Summary {
        algorithm_version: "residual_summary.canonical_float.v1",
        group,
        status,
        counts,
        rows: output,
        bias: None,
        mae: None,
        rmse: None,
        approximate: false,
    };
    if summary.status == SummaryStatus::Computed {
        let (mut signed, mut absolute, mut squares) = (
            Compensated::default(),
            Compensated::default(),
            Compensated::default(),
        );
        for (_, error) in &errors {
            let x = error.as_f64();
            summary.approximate |= error.loses_precision();
            signed.add(x);
            absolute.add(x.abs());
            squares.add(x * x);
        }
        let n = errors.len() as f64;
        let (bias, mae, rmse) = (
            signed.value() / n,
            absolute.value() / n,
            (squares.value() / n).sqrt(),
        );
        if bias.is_finite() && mae.is_finite() && rmse.is_finite() {
            summary.bias = Some(bias);
            summary.mae = Some(mae);
            summary.rmse = Some(rmse);
        } else {
            summary.status = SummaryStatus::Invalid;
        }
    }
    summary
}
