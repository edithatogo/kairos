//! Private exact C4.2 empirical distance kernel. Arithmetic is deliberately
//! bounded to checked u128 intermediates; overflow makes the request invalid.

use std::cmp::Ordering;

const EQUAL: &str = "empirical_equal.v1";
const WEIGHTED: &str = "weighted_descriptive.v1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MetricStatus {
    Computed,
    Empty,
    InsufficientData,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Precision {
    NotApplicable,
    ExactOffsets,
    Rejected,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MetricRequest<'a> {
    pub reference: &'a [Option<&'a str>],
    pub simulation: &'a [Option<&'a str>],
    pub reference_weights: Option<&'a [&'a str]>,
    pub simulation_weights: Option<&'a [&'a str]>,
    pub algorithm_version: &'a str,
    pub origin: Option<&'a str>,
    pub scale_ticks: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MetricResult {
    pub status: MetricStatus,
    pub w1: Option<f64>,
    pub ks_d: Option<f64>,
    pub reference_count: usize,
    pub simulation_count: usize,
    pub precision: Precision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Rat {
    negative: bool,
    numerator: u128,
    denominator: u128,
}

#[derive(Clone, Copy, Debug)]
struct Point {
    x: Rat,
    weight: Rat,
}

#[derive(Clone, Copy, Debug)]
struct Group {
    x: Rat,
    reference: Rat,
    simulation: Rat,
}

impl Rat {
    const ZERO: Self = Self {
        negative: false,
        numerator: 0,
        denominator: 1,
    };

    fn new(negative: bool, numerator: u128, denominator: u128) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        if numerator == 0 {
            return Some(Self::ZERO);
        }
        let divisor = gcd(numerator, denominator);
        Some(Self {
            negative,
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if value.is_empty() || value.eq_ignore_ascii_case("nan") || value.contains("inf") {
            return None;
        }
        let (negative, body) = match value.as_bytes().first() {
            Some(b'-') => (true, &value[1..]),
            Some(b'+') => (false, &value[1..]),
            _ => (false, value),
        };
        if body.is_empty() {
            return None;
        }
        if let Some((n, d)) = body.split_once('/') {
            if d.contains('/') || n.is_empty() || d.is_empty() {
                return None;
            }
            let numerator = n.parse::<u128>().ok()?;
            let denominator = d.parse::<u128>().ok()?;
            return Self::new(negative, numerator, denominator);
        }
        let mut parts = body.split('.');
        let whole = parts.next()?;
        let fraction = parts.next();
        if parts.next().is_some() || (whole.is_empty() && fraction.is_none()) {
            return None;
        }
        let whole = if whole.is_empty() {
            0
        } else {
            whole.parse::<u128>().ok()?
        };
        match fraction {
            None => Self::new(negative, whole, 1),
            Some(digits) if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) => {
                let denominator = 10u128.checked_pow(u32::try_from(digits.len()).ok()?)?;
                let fractional = digits.parse::<u128>().ok()?;
                let numerator = whole.checked_mul(denominator)?.checked_add(fractional)?;
                Self::new(negative, numerator, denominator)
            }
            _ => None,
        }
    }

    fn checked_add(self, other: Self) -> Option<Self> {
        let common = gcd(self.denominator, other.denominator);
        let left_factor = other.denominator / common;
        let right_factor = self.denominator / common;
        let left = self.numerator.checked_mul(left_factor)?;
        let right = other.numerator.checked_mul(right_factor)?;
        let numerator = if self.negative == other.negative {
            left.checked_add(right)?
        } else {
            left.abs_diff(right)
        };
        let negative = numerator != 0
            && (if left >= right {
                self.negative
            } else {
                other.negative
            });
        let denominator = self.denominator.checked_mul(left_factor)?;
        Self::new(negative, numerator, denominator)
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        self.checked_add(Self {
            negative: !other.negative,
            ..other
        })
    }

    fn checked_mul(self, other: Self) -> Option<Self> {
        let a = gcd(self.numerator, other.denominator);
        let b = gcd(other.numerator, self.denominator);
        Self::new(
            self.negative ^ other.negative,
            (self.numerator / a).checked_mul(other.numerator / b)?,
            (self.denominator / b).checked_mul(other.denominator / a)?,
        )
    }

    fn checked_div(self, other: Self) -> Option<Self> {
        if other.numerator == 0 {
            return None;
        }
        self.checked_mul(Self {
            negative: other.negative,
            numerator: other.denominator,
            denominator: other.numerator,
        })
    }

    fn abs(self) -> Self {
        Self {
            negative: false,
            ..self
        }
    }

    fn cmp_checked(self, other: Self) -> Option<Ordering> {
        match (self.negative, other.negative) {
            (true, false) => Some(Ordering::Less),
            (false, true) => Some(Ordering::Greater),
            (negative, _) => {
                let order = cmp_unsigned_fraction(
                    self.numerator,
                    self.denominator,
                    other.numerator,
                    other.denominator,
                )?;
                Some(if negative { order.reverse() } else { order })
            }
        }
    }

    fn to_f64(self) -> Option<f64> {
        let numerator = self.numerator as f64;
        let denominator = self.denominator as f64;
        let value = numerator / denominator;
        let value = if self.negative { -value } else { value };
        value.is_finite().then_some(value)
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a.max(1)
}

// Exact continued-fraction comparison avoids cross-product overflow.
fn cmp_unsigned_fraction(
    mut an: u128,
    mut ad: u128,
    mut bn: u128,
    mut bd: u128,
) -> Option<Ordering> {
    if ad == 0 || bd == 0 {
        return None;
    }
    let mut reverse = false;
    loop {
        let aq = an / ad;
        let bq = bn / bd;
        if aq != bq {
            let order = aq.cmp(&bq);
            return Some(if reverse { order.reverse() } else { order });
        }
        let ar = an % ad;
        let br = bn % bd;
        match (ar == 0, br == 0) {
            (true, true) => return Some(Ordering::Equal),
            (true, false) => {
                return Some(if reverse {
                    Ordering::Greater
                } else {
                    Ordering::Less
                })
            }
            (false, true) => {
                return Some(if reverse {
                    Ordering::Less
                } else {
                    Ordering::Greater
                })
            }
            (false, false) => {
                an = ad;
                ad = ar;
                bn = bd;
                bd = br;
                reverse = !reverse;
            }
        }
    }
}

fn invalid(precision: Precision) -> MetricResult {
    MetricResult {
        status: MetricStatus::Invalid,
        w1: None,
        ks_d: None,
        reference_count: 0,
        simulation_count: 0,
        precision,
    }
}

pub(crate) fn compare(request: &MetricRequest<'_>) -> MetricResult {
    let origin_requested = request.origin.is_some();
    let precision = Precision::NotApplicable;
    let Some(scale) = Rat::parse(request.scale_ticks) else {
        return invalid(precision);
    };
    if scale.negative || scale.numerator == 0 {
        return invalid(precision);
    }
    let weighted = match request.algorithm_version {
        EQUAL if request.reference_weights.is_none() && request.simulation_weights.is_none() => {
            false
        }
        WEIGHTED if request.reference_weights.is_some() && request.simulation_weights.is_some() => {
            true
        }
        _ => return invalid(precision),
    };
    let (ref_weights, sim_weights) = if weighted {
        let rw = request.reference_weights.unwrap();
        let sw = request.simulation_weights.unwrap();
        if rw.len() != request.reference.len() || sw.len() != request.simulation.len() {
            return invalid(precision);
        }
        let Some(rw) = rw.iter().map(|w| Rat::parse(w)).collect::<Option<Vec<_>>>() else {
            return invalid(precision);
        };
        let Some(sw) = sw.iter().map(|w| Rat::parse(w)).collect::<Option<Vec<_>>>() else {
            return invalid(precision);
        };
        if rw.iter().chain(&sw).any(|w| w.negative) {
            return invalid(precision);
        }
        (Some(rw), Some(sw))
    } else {
        (None, None)
    };
    let origin = if let Some(origin) = request.origin {
        let Some(origin) = origin.parse::<u128>().ok() else {
            return invalid(Precision::Rejected);
        };
        Some(origin)
    } else {
        None
    };
    let preprocessing_precision = if origin_requested {
        Precision::Rejected
    } else {
        Precision::NotApplicable
    };
    let Some(mut reference) =
        parse_points(request.reference, ref_weights.as_deref(), origin, scale)
    else {
        return invalid(preprocessing_precision);
    };
    let Some(mut simulation) =
        parse_points(request.simulation, sim_weights.as_deref(), origin, scale)
    else {
        return invalid(preprocessing_precision);
    };
    if origin_requested
        && (offsets_exact(&reference, scale).is_none()
            || offsets_exact(&simulation, scale).is_none())
    {
        return invalid(Precision::Rejected);
    }
    let result_precision = if origin_requested {
        Precision::ExactOffsets
    } else {
        Precision::NotApplicable
    };
    if sort_points(&mut reference, weighted).is_none()
        || sort_points(&mut simulation, weighted).is_none()
    {
        return invalid(result_precision);
    }
    let ref_count = reference.len();
    let sim_count = simulation.len();
    if ref_count == 0 && sim_count == 0 {
        return MetricResult {
            status: MetricStatus::Empty,
            w1: None,
            ks_d: None,
            reference_count: 0,
            simulation_count: 0,
            precision: result_precision,
        };
    }
    if weighted
        && ((!reference.is_empty() && normalize_weights(&mut reference).is_none())
            || (!simulation.is_empty() && normalize_weights(&mut simulation).is_none()))
    {
        return invalid(result_precision);
    }
    if ref_count == 0 || sim_count == 0 {
        return MetricResult {
            status: MetricStatus::InsufficientData,
            w1: None,
            ks_d: None,
            reference_count: ref_count,
            simulation_count: sim_count,
            precision: result_precision,
        };
    }
    if !weighted {
        for p in &mut reference {
            p.weight = match Rat::new(false, 1, ref_count as u128) {
                Some(weight) => weight,
                None => return invalid(result_precision),
            };
        }
        for p in &mut simulation {
            p.weight = match Rat::new(false, 1, sim_count as u128) {
                Some(weight) => weight,
                None => return invalid(result_precision),
            };
        }
    }
    let Some((w1, ks)) = reduce(reference, simulation) else {
        return invalid(result_precision);
    };
    let Some(w1) = w1.to_f64() else {
        return invalid(result_precision);
    };
    let Some(ks) = ks.to_f64() else {
        return invalid(result_precision);
    };
    if w1 < 0.0 || ks < 0.0 || ks > 1.0 {
        return invalid(result_precision);
    }
    MetricResult {
        status: MetricStatus::Computed,
        w1: Some(w1),
        ks_d: Some(ks),
        reference_count: ref_count,
        simulation_count: sim_count,
        precision: result_precision,
    }
}

fn parse_points(
    values: &[Option<&str>],
    weights: Option<&[Rat]>,
    origin: Option<u128>,
    scale: Rat,
) -> Option<Vec<Point>> {
    let mut points = Vec::new();
    for (index, value) in values.iter().enumerate() {
        let weight = weights.map_or(Rat::new(false, 1, 1), |ws| Some(ws[index]))?;
        if let Some(value) = value {
            let x = if let Some(origin) = origin {
                let tick = value.parse::<u128>().ok()?;
                let offset = tick.checked_sub(origin)?;
                Rat::new(false, offset, 1)?.checked_div(scale)?
            } else {
                Rat::parse(value)?
            };
            points.push(Point { x, weight });
        }
    }
    Some(points)
}

fn offsets_exact(points: &[Point], scale: Rat) -> Option<()> {
    for point in points {
        let offset = point.x.checked_mul(scale)?.numerator;
        if !integer_exact_as_f64(offset) {
            return None;
        }
    }
    Some(())
}

fn integer_exact_as_f64(value: u128) -> bool {
    if value == 0 {
        return true;
    }
    let bits = u128::BITS - value.leading_zeros();
    bits <= 53 || value.trailing_zeros() >= bits - 53
}

fn normalize_weights(points: &mut [Point]) -> Option<()> {
    let mut total = Rat::ZERO;
    for point in points.iter() {
        total = total.checked_add(point.weight)?;
    }
    if total.numerator == 0 || total.to_f64()?.is_infinite() {
        return None;
    }
    for point in points {
        point.weight = point.weight.checked_div(total)?;
    }
    Some(())
}

fn point_cmp(a: Point, b: Point, weight_tiebreaker: bool) -> Option<Ordering> {
    let support_order = a.x.cmp_checked(b.x)?;
    if support_order == Ordering::Equal && weight_tiebreaker {
        a.weight.cmp_checked(b.weight)
    } else {
        Some(support_order)
    }
}

fn sort_points(points: &mut [Point], weight_tiebreaker: bool) -> Option<()> {
    fn sort_range(
        points: &mut [Point],
        scratch: &mut [Point],
        weight_tiebreaker: bool,
    ) -> Option<()> {
        if points.len() < 2 {
            return Some(());
        }
        let middle = points.len() / 2;
        let (left, right) = points.split_at_mut(middle);
        let (scratch_left, scratch_right) = scratch.split_at_mut(middle);
        sort_range(left, scratch_left, weight_tiebreaker)?;
        sort_range(right, scratch_right, weight_tiebreaker)?;
        scratch.copy_from_slice(points);
        let (mut left_index, mut right_index, mut output_index) = (0, middle, 0);
        while left_index < middle && right_index < points.len() {
            let ordering = point_cmp(scratch[left_index], scratch[right_index], weight_tiebreaker)?;
            if ordering != Ordering::Greater {
                points[output_index] = scratch[left_index];
                left_index += 1;
            } else {
                points[output_index] = scratch[right_index];
                right_index += 1;
            }
            output_index += 1;
        }
        while left_index < middle {
            points[output_index] = scratch[left_index];
            left_index += 1;
            output_index += 1;
        }
        while right_index < points.len() {
            points[output_index] = scratch[right_index];
            right_index += 1;
            output_index += 1;
        }
        Some(())
    }

    let mut scratch = points.to_vec();
    sort_range(points, &mut scratch, weight_tiebreaker)
}

fn reduce(mut reference: Vec<Point>, mut simulation: Vec<Point>) -> Option<(Rat, Rat)> {
    sort_points(&mut reference, true)?;
    sort_points(&mut simulation, true)?;
    let (mut ri, mut si) = (0, 0);
    let mut groups = Vec::with_capacity(reference.len().checked_add(simulation.len())?);
    while ri < reference.len() || si < simulation.len() {
        let x = match (reference.get(ri), simulation.get(si)) {
            (Some(r), Some(s)) => {
                if r.x.cmp_checked(s.x)? != Ordering::Greater {
                    r.x
                } else {
                    s.x
                }
            }
            (Some(r), None) => r.x,
            (None, Some(s)) => s.x,
            (None, None) => break,
        };
        let mut reference_mass = Rat::ZERO;
        while let Some(point) = reference.get(ri) {
            if point.x.cmp_checked(x)? != Ordering::Equal {
                break;
            }
            reference_mass = reference_mass.checked_add(point.weight)?;
            ri += 1;
        }
        let mut simulation_mass = Rat::ZERO;
        while let Some(point) = simulation.get(si) {
            if point.x.cmp_checked(x)? != Ordering::Equal {
                break;
            }
            simulation_mass = simulation_mass.checked_add(point.weight)?;
            si += 1;
        }
        groups.push(Group {
            x,
            reference: reference_mass,
            simulation: simulation_mass,
        });
    }

    let mut ref_cdf = Rat::ZERO;
    let mut sim_cdf = Rat::ZERO;
    let mut ks = Rat::ZERO;
    let mut w1 = Rat::ZERO;
    for (index, group) in groups.iter().enumerate() {
        ref_cdf = ref_cdf.checked_add(group.reference)?;
        sim_cdf = sim_cdf.checked_add(group.simulation)?;
        let gap = ref_cdf.checked_sub(sim_cdf)?.abs();
        if gap.cmp_checked(ks)? == Ordering::Greater {
            ks = gap;
        }
        if let Some(next) = groups.get(index + 1) {
            let width = next.x.checked_sub(group.x)?;
            w1 = w1.checked_add(width.checked_mul(gap)?)?;
        }
    }
    Some((w1, ks))
}
