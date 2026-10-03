//! Internal order key for normalized calibration trace records.
//!
//! The comparator follows the C0 tuple. Text components use unsigned UTF-8
//! bytewise order; this makes cross-language ordering explicit without
//! changing the tuple or applying Unicode normalization. Dataset-wide mapping
//! checks (rank membership and uniqueness) belong to the validator layer.

use std::cmp::Ordering;

/// A canonical, unbounded, nonnegative integer rank.
///
/// C0 rank values have no fixed-width maximum. The decimal string is canonical
/// (ASCII digits, no leading zero except `0`) so numeric comparison never
/// converts through a bounded integer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EventKindRank(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct InvalidEventKindRank;

impl EventKindRank {
    /// Construct from an already-normalized C0 integer value. JSON adapters
    /// must parse C0 numeric tokens exactly and normalize mathematically before
    /// calling this constructor.
    pub(crate) fn from_canonical_decimal(value: &str) -> Result<Self, InvalidEventKindRank> {
        let bytes = value.as_bytes();
        if bytes.is_empty()
            || !bytes.iter().all(|byte| byte.is_ascii_digit())
            || (bytes.len() > 1 && bytes[0] == b'0')
        {
            return Err(InvalidEventKindRank);
        }
        Ok(Self(value.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl Ord for EventKindRank {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.as_bytes().cmp(other.0.as_bytes()))
    }
}

impl PartialOrd for EventKindRank {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The typed six-field projection used for C0 trace presentation order.
///
/// This is not the complete `trace_event.v1` record and does not perform
/// dataset-wide validation. Inputs are assumed to have passed the owning
/// Track 21 validator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TraceOrderKeyV1 {
    pub(crate) relative_ticks: u128,
    pub(crate) case_key: String,
    pub(crate) occurrence: u32,
    pub(crate) event_kind_rank: EventKindRank,
    pub(crate) source_event_key: String,
    pub(crate) source_order: u64,
}

impl Ord for TraceOrderKeyV1 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.relative_ticks
            .cmp(&other.relative_ticks)
            .then_with(|| self.case_key.as_bytes().cmp(other.case_key.as_bytes()))
            .then_with(|| self.occurrence.cmp(&other.occurrence))
            .then_with(|| self.event_kind_rank.cmp(&other.event_kind_rank))
            .then_with(|| {
                self.source_event_key
                    .as_bytes()
                    .cmp(other.source_event_key.as_bytes())
            })
            .then_with(|| self.source_order.cmp(&other.source_order))
    }
}

impl PartialOrd for TraceOrderKeyV1 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::{EventKindRank, TraceOrderKeyV1};
    use std::cmp::Ordering;

    fn rank(value: &str) -> EventKindRank {
        EventKindRank::from_canonical_decimal(value).expect("test rank is canonical")
    }

    fn key(
        relative_ticks: u128,
        case_key: &str,
        occurrence: u32,
        event_kind_rank: &str,
        source_event_key: &str,
        source_order: u64,
    ) -> TraceOrderKeyV1 {
        TraceOrderKeyV1 {
            relative_ticks,
            case_key: case_key.to_owned(),
            occurrence,
            event_kind_rank: rank(event_kind_rank),
            source_event_key: source_event_key.to_owned(),
            source_order,
        }
    }

    #[test]
    fn rank_requires_canonical_ascii_unsigned_decimal() {
        for value in ["", "00", "01", "+1", "-1", " 1", "1 ", "１", "1e3", "1\n"] {
            assert!(
                EventKindRank::from_canonical_decimal(value).is_err(),
                "{value:?}"
            );
        }
        assert_eq!(rank("0").as_str(), "0");
        assert_eq!(rank("1").as_str(), "1");
        assert_eq!(
            rank("18446744073709551616").as_str(),
            "18446744073709551616"
        );
        assert_eq!(rank(&format!("1{}", "0".repeat(200))).as_str().len(), 201);
    }

    #[test]
    fn rank_comparison_is_numeric_without_fixed_width_conversion() {
        let values = [
            rank("0"),
            rank("2"),
            rank("10"),
            rank("18446744073709551616"),
            rank(&format!("1{}", "0".repeat(200))),
        ];
        assert!(values.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(EventKindRank::from_canonical_decimal("000").is_err());
        assert!(rank("23") < rank("24"));
    }

    #[test]
    fn order_tuple_uses_each_component_in_declared_precedence() {
        assert!(key(0, "z", 0, "0", "z", 0) < key(1, "a", 0, "0", "a", 0));
        assert!(key(1, "a", 0, "0", "z", 0) < key(1, "b", 0, "0", "a", 0));
        assert!(key(1, "a", 0, "0", "z", 0) < key(1, "a", 1, "0", "a", 0));
        assert!(key(1, "a", 1, "2", "z", 0) < key(1, "a", 1, "10", "a", 0));
        assert!(key(1, "a", 1, "2", "a", 0) < key(1, "a", 1, "2", "b", 0));
        assert!(key(1, "a", 1, "2", "a", 0) < key(1, "a", 1, "2", "a", 1));
    }

    #[test]
    fn text_order_is_unsigned_utf8_bytewise_without_normalization() {
        assert_eq!("A".as_bytes().cmp("z".as_bytes()), Ordering::Less);
        assert_eq!("z".as_bytes().cmp("é".as_bytes()), Ordering::Less);
        assert_ne!("é", "e\u{301}");
        assert!(key(0, "z", 0, "0", "a", 0) < key(0, "é", 0, "0", "a", 0));
        assert!(key(0, "same", 0, "0", "e", 0) < key(0, "same", 0, "0", "é", 0));
        assert!(key(0, "same", 0, "0", "é", 0) < key(0, "same", 0, "0", "中", 0));
    }

    #[test]
    fn equal_keys_compare_equal_and_sorting_is_independent_of_all_permutations() {
        let a = key(0, "case", 0, "2", "a", 0);
        let b = key(0, "case", 0, "10", "a", 0);
        let c = key(u128::MAX, "case", u32::MAX, "1", "z", u64::MAX);
        assert_eq!(a.cmp(&a.clone()), Ordering::Equal);
        let expected = vec![a.clone(), b.clone(), c.clone()];
        let permutations = [
            vec![a.clone(), b.clone(), c.clone()],
            vec![a.clone(), c.clone(), b.clone()],
            vec![b.clone(), a.clone(), c.clone()],
            vec![b.clone(), c.clone(), a.clone()],
            vec![c.clone(), a.clone(), b.clone()],
            vec![c, b, a],
        ];
        for mut permutation in permutations {
            permutation.sort();
            assert_eq!(permutation, expected);
        }
        assert_eq!(expected[0].relative_ticks, 0);
        assert!(
            key(u128::MAX - 1, "case", 0, "0", "a", 0) < key(u128::MAX, "case", 0, "0", "a", 0)
        );
        assert!(
            key(0, "case", u32::MAX - 1, "0", "a", u64::MAX)
                < key(0, "case", u32::MAX, "0", "a", 0)
        );
        assert!(key(0, "case", 0, "0", "a", u64::MAX - 1) < key(0, "case", 0, "0", "a", u64::MAX));
        assert_eq!(expected[2].relative_ticks, u128::MAX);
    }
}
