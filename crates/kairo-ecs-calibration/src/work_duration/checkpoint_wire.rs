//! Canonical bounded wire transport for the private provider checkpoint DTO.
use super::*;
use std::str;

const MAGIC: &[u8; 8] = b"KIWPV1\0\0";
const WIRE_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct IntrinsicWorkProviderWireLimits {
    pub(crate) provider: IntrinsicWorkProviderCheckpointLimits,
    pub(crate) max_wire_bytes: usize,
}

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub(crate) enum IntrinsicWorkProviderWireError {
    #[error("unsupported provider wire version")]
    UnsupportedWireVersion,
    #[error("provider wire limit exceeded")]
    LimitExceeded,
    #[error("invalid provider wire data")]
    InvalidData,
    #[error("invalid UTF-8 in provider wire data")]
    InvalidUtf8,
    #[error("truncated provider wire data")]
    Truncated,
    #[error("trailing provider wire data")]
    TrailingBytes,
    #[error("provider wire allocation failed")]
    Allocation,
    #[error("native provider checkpoint validation failed: {0}")]
    Native(IntrinsicWorkProviderCheckpointError),
}

impl IntrinsicWorkProvider {
    pub(crate) fn checkpoint_wire_v1(
        &self,
        limits: IntrinsicWorkProviderWireLimits,
    ) -> Result<Vec<u8>, IntrinsicWorkProviderWireError> {
        let expected_length = measure_provider(self, limits)?;
        let bytes = self
            .checkpoint_v1(limits.provider)
            .map_err(IntrinsicWorkProviderWireError::Native)?
            .encode_wire_v1(limits)?;
        if bytes.len() != expected_length {
            return Err(IntrinsicWorkProviderWireError::InvalidData);
        }
        Ok(bytes)
    }
}

impl IntrinsicWorkProviderCheckpointV1 {
    pub(crate) fn encode_wire_v1(
        &self,
        limits: IntrinsicWorkProviderWireLimits,
    ) -> Result<Vec<u8>, IntrinsicWorkProviderWireError> {
        let length = measure_validate(self, limits)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(length)
            .map_err(|_| IntrinsicWorkProviderWireError::Allocation)?;
        let mut w = Writer { bytes: &mut bytes };
        w.raw(MAGIC);
        w.u32(WIRE_VERSION);
        w.u32(self.version);
        w.count(self.strata.len())?;
        for stratum in &self.strata {
            w.string(&stratum.id)?;
            match &stratum.distribution {
                ProviderDistributionCheckpointV1::FixedTicks(ticks) => {
                    w.u8(0);
                    w.u128(*ticks);
                }
                ProviderDistributionCheckpointV1::WeightedTicks {
                    support,
                    cached_total,
                } => {
                    w.u8(1);
                    w.count(support.len())?;
                    for &(ticks, weight) in support {
                        w.u128(ticks);
                        w.u64(weight);
                    }
                    w.u64(*cached_total);
                }
            }
        }
        if bytes.len() != length {
            return Err(IntrinsicWorkProviderWireError::InvalidData);
        }
        Ok(bytes)
    }

    pub(crate) fn decode_wire_v1(
        bytes: &[u8],
        limits: IntrinsicWorkProviderWireLimits,
    ) -> Result<Self, IntrinsicWorkProviderWireError> {
        preflight(bytes, limits)?;
        decode(bytes)
    }

    pub(crate) fn restore_wire_v1(
        bytes: &[u8],
        limits: IntrinsicWorkProviderWireLimits,
    ) -> Result<IntrinsicWorkProvider, IntrinsicWorkProviderWireError> {
        Self::decode_wire_v1(bytes, limits)?
            .restore(limits.provider)
            .map_err(IntrinsicWorkProviderWireError::Native)
    }
}

fn measure_validate(
    image: &IntrinsicWorkProviderCheckpointV1,
    limits: IntrinsicWorkProviderWireLimits,
) -> Result<usize, IntrinsicWorkProviderWireError> {
    if image.version != INTRINSIC_WORK_PROVIDER_VERSION_V1 {
        return Err(IntrinsicWorkProviderWireError::Native(
            IntrinsicWorkProviderCheckpointError::UnsupportedVersion,
        ));
    }
    let (strata, supports, identifiers) = aggregate(image)?;
    if strata == 0
        || strata > limits.provider.max_strata
        || supports > limits.provider.max_total_support
        || identifiers > limits.provider.max_identifier_bytes
    {
        return Err(IntrinsicWorkProviderWireError::LimitExceeded);
    }

    // Compute exact wire size before any semantic validation allocates scratch.
    let mut length = 24usize;
    for stratum in &image.strata {
        length = add(length, add(8, stratum.id.len())?)?;
        match &stratum.distribution {
            ProviderDistributionCheckpointV1::FixedTicks(_) => length = add(length, 17)?,
            ProviderDistributionCheckpointV1::WeightedTicks { support, .. } => {
                let support_bytes = support
                    .len()
                    .checked_mul(24)
                    .ok_or(IntrinsicWorkProviderWireError::LimitExceeded)?;
                length = add(length, add(17, support_bytes)?)?;
            }
        }
    }
    if length > limits.max_wire_bytes {
        return Err(IntrinsicWorkProviderWireError::LimitExceeded);
    }

    // All aggregate count and byte caps now precede duplicate-support scratch.
    for (index, stratum) in image.strata.iter().enumerate() {
        validate_id("stratum_id", &stratum.id).map_err(|_| native_invalid_id())?;
        if index > 0 && image.strata[index - 1].id >= stratum.id {
            return Err(native_invalid_id());
        }
        match &stratum.distribution {
            ProviderDistributionCheckpointV1::FixedTicks(ticks) if *ticks > 0 => {}
            ProviderDistributionCheckpointV1::FixedTicks(_) => {
                return Err(native_invalid_distribution());
            }
            ProviderDistributionCheckpointV1::WeightedTicks {
                support,
                cached_total,
            } => {
                validate_support(support, Some(*cached_total))
                    .map_err(IntrinsicWorkProviderWireError::Native)?;
            }
        }
    }
    Ok(length)
}

fn measure_provider(
    provider: &IntrinsicWorkProvider,
    limits: IntrinsicWorkProviderWireLimits,
) -> Result<usize, IntrinsicWorkProviderWireError> {
    let mut supports = 0usize;
    let mut identifiers = 0usize;
    for (id, distribution) in &provider.strata {
        identifiers = identifiers
            .checked_add(id.len())
            .ok_or(IntrinsicWorkProviderWireError::LimitExceeded)?;
        if let Distribution::Weighted { support, .. } = &distribution.distribution {
            supports = supports
                .checked_add(support.len())
                .ok_or(IntrinsicWorkProviderWireError::LimitExceeded)?;
        }
    }
    if provider.strata.is_empty()
        || provider.strata.len() > limits.provider.max_strata
        || supports > limits.provider.max_total_support
        || identifiers > limits.provider.max_identifier_bytes
    {
        return Err(IntrinsicWorkProviderWireError::LimitExceeded);
    }
    let mut length = 24usize;
    for (id, distribution) in &provider.strata {
        length = add(length, add(8, id.len())?)?;
        match &distribution.distribution {
            Distribution::Fixed(_) => length = add(length, 17)?,
            Distribution::Weighted { support, .. } => {
                let support_bytes = support
                    .len()
                    .checked_mul(24)
                    .ok_or(IntrinsicWorkProviderWireError::LimitExceeded)?;
                length = add(length, add(17, support_bytes)?)?;
            }
        }
    }
    if length > limits.max_wire_bytes {
        return Err(IntrinsicWorkProviderWireError::LimitExceeded);
    }
    Ok(length)
}

fn aggregate(
    image: &IntrinsicWorkProviderCheckpointV1,
) -> Result<(usize, usize, usize), IntrinsicWorkProviderWireError> {
    let mut supports = 0usize;
    let mut identifiers = 0usize;
    for stratum in &image.strata {
        identifiers = identifiers
            .checked_add(stratum.id.len())
            .ok_or(IntrinsicWorkProviderWireError::LimitExceeded)?;
        if let ProviderDistributionCheckpointV1::WeightedTicks { support, .. } =
            &stratum.distribution
        {
            supports = supports
                .checked_add(support.len())
                .ok_or(IntrinsicWorkProviderWireError::LimitExceeded)?;
        }
    }
    Ok((image.strata.len(), supports, identifiers))
}

fn native_invalid_id() -> IntrinsicWorkProviderWireError {
    IntrinsicWorkProviderWireError::Native(
        IntrinsicWorkProviderCheckpointError::InvalidIdentifierOrOrder,
    )
}
fn native_invalid_distribution() -> IntrinsicWorkProviderWireError {
    IntrinsicWorkProviderWireError::Native(
        IntrinsicWorkProviderCheckpointError::InvalidDistribution,
    )
}
fn add(a: usize, b: usize) -> Result<usize, IntrinsicWorkProviderWireError> {
    a.checked_add(b)
        .ok_or(IntrinsicWorkProviderWireError::LimitExceeded)
}

// This pass validates all byte/count budgets and fixed-format semantics without
// allocating any strings, support vectors, or duplicate-tracking structures.
fn preflight(
    bytes: &[u8],
    limits: IntrinsicWorkProviderWireLimits,
) -> Result<(), IntrinsicWorkProviderWireError> {
    if bytes.len() > limits.max_wire_bytes {
        return Err(IntrinsicWorkProviderWireError::LimitExceeded);
    }
    let mut r = Reader::new(bytes);
    if r.raw(8)? != MAGIC {
        return Err(IntrinsicWorkProviderWireError::InvalidData);
    }
    if r.u32()? != WIRE_VERSION {
        return Err(IntrinsicWorkProviderWireError::UnsupportedWireVersion);
    }
    if r.u32()? != INTRINSIC_WORK_PROVIDER_VERSION_V1 {
        return Err(IntrinsicWorkProviderWireError::Native(
            IntrinsicWorkProviderCheckpointError::UnsupportedVersion,
        ));
    }
    let count = r.count(limits.provider.max_strata)?;
    if count == 0 {
        return Err(IntrinsicWorkProviderWireError::InvalidData);
    }
    let mut support_total = 0usize;
    let mut identifier_total = 0usize;
    let mut previous: Option<&str> = None;
    for _ in 0..count {
        let len = r.usize()?;
        identifier_total = identifier_total
            .checked_add(len)
            .ok_or(IntrinsicWorkProviderWireError::LimitExceeded)?;
        if len == 0 {
            return Err(IntrinsicWorkProviderWireError::InvalidData);
        }
        if identifier_total > limits.provider.max_identifier_bytes {
            return Err(IntrinsicWorkProviderWireError::LimitExceeded);
        }
        let id =
            str::from_utf8(r.raw(len)?).map_err(|_| IntrinsicWorkProviderWireError::InvalidUtf8)?;
        validate_id("stratum_id", id).map_err(|_| IntrinsicWorkProviderWireError::InvalidData)?;
        if previous.is_some_and(|old| old >= id) {
            return Err(IntrinsicWorkProviderWireError::InvalidData);
        }
        previous = Some(id);
        match r.u8()? {
            0 => {
                if r.u128()? == 0 {
                    return Err(IntrinsicWorkProviderWireError::InvalidData);
                }
            }
            1 => {
                let n = r.count(limits.provider.max_total_support)?;
                support_total = support_total
                    .checked_add(n)
                    .ok_or(IntrinsicWorkProviderWireError::LimitExceeded)?;
                if n == 0 {
                    return Err(IntrinsicWorkProviderWireError::InvalidData);
                }
                if support_total > limits.provider.max_total_support {
                    return Err(IntrinsicWorkProviderWireError::LimitExceeded);
                }
                let mut total = 0u64;
                for _ in 0..n {
                    if r.u128()? == 0 {
                        return Err(IntrinsicWorkProviderWireError::InvalidData);
                    }
                    let weight = r.u64()?;
                    if weight == 0 {
                        return Err(IntrinsicWorkProviderWireError::InvalidData);
                    }
                    total = total
                        .checked_add(weight)
                        .ok_or(IntrinsicWorkProviderWireError::InvalidData)?;
                }
                if r.u64()? != total {
                    return Err(IntrinsicWorkProviderWireError::InvalidData);
                }
            }
            _ => return Err(IntrinsicWorkProviderWireError::InvalidData),
        }
    }
    r.finish()
}

fn decode(
    bytes: &[u8],
) -> Result<IntrinsicWorkProviderCheckpointV1, IntrinsicWorkProviderWireError> {
    let mut r = Reader::new(bytes);
    r.raw(8)?;
    r.u32()?;
    let version = r.u32()?;
    let count = r.usize()?;
    let mut strata = Vec::new();
    strata
        .try_reserve_exact(count)
        .map_err(|_| IntrinsicWorkProviderWireError::Allocation)?;
    for _ in 0..count {
        let id = r.string()?;
        let distribution = match r.u8()? {
            0 => ProviderDistributionCheckpointV1::FixedTicks(r.u128()?),
            1 => {
                let count = r.usize()?;
                let mut support = Vec::new();
                support
                    .try_reserve_exact(count)
                    .map_err(|_| IntrinsicWorkProviderWireError::Allocation)?;
                for _ in 0..count {
                    support.push((r.u128()?, r.u64()?));
                }
                ProviderDistributionCheckpointV1::WeightedTicks {
                    support,
                    cached_total: r.u64()?,
                }
            }
            _ => return Err(IntrinsicWorkProviderWireError::InvalidData),
        };
        strata.push(ProviderStratumCheckpointV1 { id, distribution });
    }
    r.finish()?;
    Ok(IntrinsicWorkProviderCheckpointV1 { version, strata })
}

struct Writer<'a> {
    bytes: &'a mut Vec<u8>,
}
impl Writer<'_> {
    fn raw(&mut self, b: &[u8]) {
        self.bytes.extend_from_slice(b);
    }
    fn u8(&mut self, n: u8) {
        self.bytes.push(n);
    }
    fn u32(&mut self, n: u32) {
        self.raw(&n.to_le_bytes());
    }
    fn u64(&mut self, n: u64) {
        self.raw(&n.to_le_bytes());
    }
    fn u128(&mut self, n: u128) {
        self.raw(&n.to_le_bytes());
    }
    fn count(&mut self, n: usize) -> Result<(), IntrinsicWorkProviderWireError> {
        self.u64(u64::try_from(n).map_err(|_| IntrinsicWorkProviderWireError::LimitExceeded)?);
        Ok(())
    }
    fn string(&mut self, s: &str) -> Result<(), IntrinsicWorkProviderWireError> {
        self.count(s.len())?;
        self.raw(s.as_bytes());
        Ok(())
    }
}
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    fn raw(&mut self, n: usize) -> Result<&'a [u8], IntrinsicWorkProviderWireError> {
        let end = self
            .at
            .checked_add(n)
            .ok_or(IntrinsicWorkProviderWireError::LimitExceeded)?;
        let value = self
            .bytes
            .get(self.at..end)
            .ok_or(IntrinsicWorkProviderWireError::Truncated)?;
        self.at = end;
        Ok(value)
    }
    fn u8(&mut self) -> Result<u8, IntrinsicWorkProviderWireError> {
        Ok(self.raw(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, IntrinsicWorkProviderWireError> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, IntrinsicWorkProviderWireError> {
        Ok(u64::from_le_bytes(self.raw(8)?.try_into().unwrap()))
    }
    fn u128(&mut self) -> Result<u128, IntrinsicWorkProviderWireError> {
        Ok(u128::from_le_bytes(self.raw(16)?.try_into().unwrap()))
    }
    fn usize(&mut self) -> Result<usize, IntrinsicWorkProviderWireError> {
        usize::try_from(self.u64()?).map_err(|_| IntrinsicWorkProviderWireError::LimitExceeded)
    }
    fn count(&mut self, cap: usize) -> Result<usize, IntrinsicWorkProviderWireError> {
        let n = self.usize()?;
        if n > cap {
            Err(IntrinsicWorkProviderWireError::LimitExceeded)
        } else {
            Ok(n)
        }
    }
    fn string(&mut self) -> Result<String, IntrinsicWorkProviderWireError> {
        let n = self.usize()?;
        let s = str::from_utf8(self.raw(n)?)
            .map_err(|_| IntrinsicWorkProviderWireError::InvalidUtf8)?;
        let mut out = String::new();
        out.try_reserve_exact(n)
            .map_err(|_| IntrinsicWorkProviderWireError::Allocation)?;
        out.push_str(s);
        Ok(out)
    }
    fn finish(&self) -> Result<(), IntrinsicWorkProviderWireError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(IntrinsicWorkProviderWireError::TrailingBytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed_map::{CalibrationSeedMap, SeedPurpose};

    fn limits() -> IntrinsicWorkProviderWireLimits {
        IntrinsicWorkProviderWireLimits {
            provider: IntrinsicWorkProviderCheckpointLimits {
                max_strata: 8,
                max_total_support: 32,
                max_identifier_bytes: 128,
            },
            max_wire_bytes: 4096,
        }
    }
    fn provider() -> IntrinsicWorkProvider {
        IntrinsicWorkProvider::new(
            INTRINSIC_WORK_PROVIDER_VERSION_V1,
            vec![
                (
                    "z-fixed".into(),
                    IntrinsicDurationDistribution::fixed(11).unwrap(),
                ),
                (
                    "a-weighted".into(),
                    IntrinsicDurationDistribution::weighted_ticks(vec![(31, 2), (17, 5), (23, 1)])
                        .unwrap(),
                ),
            ],
        )
        .unwrap()
    }
    fn service() -> (CalibrationStreamKey, CalibrationStream) {
        let mut map = CalibrationSeedMap::new(1, "provider-wire", 991).unwrap();
        let key = map
            .key_for("schedule", 2, "case-4", "task-8", SeedPurpose::Service)
            .unwrap();
        let stream = map
            .stream_for("schedule", 2, "case-4", "task-8", SeedPurpose::Service)
            .unwrap();
        (key, stream)
    }
    fn first_distribution_tag(bytes: &[u8]) -> usize {
        let len = u64::from_le_bytes(bytes[24..32].try_into().unwrap()) as usize;
        32 + len
    }

    #[test]
    fn wire_roundtrip_preserves_distribution_order_and_advanced_service_sampling() {
        let original = provider();
        let image = original.checkpoint_v1(limits().provider).unwrap();
        let bytes = image.encode_wire_v1(limits()).unwrap();
        assert_eq!(original.checkpoint_wire_v1(limits()).unwrap(), bytes);
        let decoded = IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bytes, limits()).unwrap();
        assert_eq!(decoded, image);
        assert_eq!(decoded.strata[0].id, "a-weighted");
        match &decoded.strata[0].distribution {
            ProviderDistributionCheckpointV1::WeightedTicks {
                support,
                cached_total,
            } => {
                assert_eq!(support, &[(31, 2), (17, 5), (23, 1)]);
                assert_eq!(*cached_total, 8);
            }
            _ => panic!("weighted branch expected"),
        }
        assert!(matches!(
            decoded.strata[1].distribution,
            ProviderDistributionCheckpointV1::FixedTicks(11)
        ));
        let restored =
            IntrinsicWorkProviderCheckpointV1::restore_wire_v1(&bytes, limits()).unwrap();
        for id in ["a-weighted", "z-fixed", "a-weighted", "z-fixed"] {
            let (key_a, mut a) = service();
            let (key_b, mut b) = service();
            for _ in 0..2 {
                a.next_u64().unwrap();
                b.next_u64().unwrap();
            }
            let x = original.sample(id, &mut a, &key_a).unwrap();
            let y = restored.sample(id, &mut b, &key_b).unwrap();
            assert_eq!(
                (x.duration(), x.draw_before(), x.draw_after()),
                (y.duration(), y.draw_before(), y.draw_after())
            );
            assert_eq!(a.draw_position(), b.draw_position());
        }
        let mut transit_map = CalibrationSeedMap::new(1, "provider-wire", 991).unwrap();
        let transit_key = transit_map
            .key_for("schedule", 2, "case-4", "task-8", SeedPurpose::Transit)
            .unwrap();
        let mut transit = transit_map
            .stream_for("schedule", 2, "case-4", "task-8", SeedPurpose::Transit)
            .unwrap();
        assert_eq!(
            restored.sample("a-weighted", &mut transit, &transit_key),
            Err(WorkDurationError::WrongPurpose)
        );
    }

    #[test]
    fn wire_caps_are_symmetric_at_exact_boundaries() {
        let image = provider().checkpoint_v1(limits().provider).unwrap();
        let bytes = image.encode_wire_v1(limits()).unwrap();
        let mut exact = limits();
        exact.provider.max_strata = 2;
        exact.provider.max_total_support = 3;
        exact.provider.max_identifier_bytes = "a-weighted".len() + "z-fixed".len();
        exact.max_wire_bytes = bytes.len();
        assert_eq!(image.encode_wire_v1(exact).unwrap(), bytes);
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bytes, exact).unwrap(),
            image
        );
        let mut too_small = exact;
        too_small.provider.max_strata = 1;
        assert_eq!(
            image.encode_wire_v1(too_small),
            Err(IntrinsicWorkProviderWireError::LimitExceeded)
        );
        let mut wrong_order = image.clone();
        wrong_order.strata.swap(0, 1);
        assert_eq!(
            wrong_order.encode_wire_v1(limits()),
            Err(IntrinsicWorkProviderWireError::Native(
                IntrinsicWorkProviderCheckpointError::InvalidIdentifierOrOrder
            ))
        );
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bytes, too_small),
            Err(IntrinsicWorkProviderWireError::LimitExceeded)
        );
        let mut too_small = exact;
        too_small.provider.max_total_support = 2;
        assert_eq!(
            image.encode_wire_v1(too_small),
            Err(IntrinsicWorkProviderWireError::LimitExceeded)
        );
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bytes, too_small),
            Err(IntrinsicWorkProviderWireError::LimitExceeded)
        );
        let mut too_small = exact;
        too_small.provider.max_identifier_bytes -= 1;
        assert_eq!(
            image.encode_wire_v1(too_small),
            Err(IntrinsicWorkProviderWireError::LimitExceeded)
        );
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bytes, too_small),
            Err(IntrinsicWorkProviderWireError::LimitExceeded)
        );
        let mut too_small = exact;
        too_small.max_wire_bytes -= 1;
        assert_eq!(
            image.encode_wire_v1(too_small),
            Err(IntrinsicWorkProviderWireError::LimitExceeded)
        );
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bytes, too_small),
            Err(IntrinsicWorkProviderWireError::LimitExceeded)
        );
    }

    #[test]
    fn wire_preflight_rejects_header_counts_utf8_tags_truncation_and_trailing() {
        let image = provider().checkpoint_v1(limits().provider).unwrap();
        let bytes = image.encode_wire_v1(limits()).unwrap();
        let mut bad = bytes.clone();
        bad[8..12].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bad, limits()),
            Err(IntrinsicWorkProviderWireError::UnsupportedWireVersion)
        );
        let mut bad = bytes.clone();
        bad[16..24].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bad, limits()),
            Err(IntrinsicWorkProviderWireError::LimitExceeded)
        );
        let mut bad = bytes.clone();
        bad[24..32].copy_from_slice(&0u64.to_le_bytes());
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bad, limits()),
            Err(IntrinsicWorkProviderWireError::InvalidData)
        );
        let mut bad = bytes.clone();
        bad[32] = 0xff;
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bad, limits()),
            Err(IntrinsicWorkProviderWireError::InvalidUtf8)
        );
        let mut bad = bytes.clone();
        let tag_at = first_distribution_tag(&bad);
        bad[tag_at] = 8;
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bad, limits()),
            Err(IntrinsicWorkProviderWireError::InvalidData)
        );
        let mut bad = bytes.clone();
        let at = first_distribution_tag(&bad) + 1;
        bad[at..at + 8].copy_from_slice(&0u64.to_le_bytes());
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bad, limits()),
            Err(IntrinsicWorkProviderWireError::InvalidData)
        );
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bytes[..bytes.len() - 1], limits()),
            Err(IntrinsicWorkProviderWireError::Truncated)
        );
        let mut bad = bytes;
        bad.push(0);
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bad, limits()),
            Err(IntrinsicWorkProviderWireError::TrailingBytes)
        );
    }

    #[test]
    fn wire_restore_uses_native_distribution_validation() {
        let mut image = provider().checkpoint_v1(limits().provider).unwrap();
        if let ProviderDistributionCheckpointV1::WeightedTicks { cached_total, .. } =
            &mut image.strata[0].distribution
        {
            *cached_total += 1;
        }
        assert!(matches!(
            image.encode_wire_v1(limits()),
            Err(IntrinsicWorkProviderWireError::Native(
                IntrinsicWorkProviderCheckpointError::InvalidDistribution
            ))
        ));
        let image = provider().checkpoint_v1(limits().provider).unwrap();
        let mut bytes = image.encode_wire_v1(limits()).unwrap();
        let at = first_distribution_tag(&bytes) + 1 + 8;
        bytes[at..at + 16].copy_from_slice(&0u128.to_le_bytes());
        assert_eq!(
            IntrinsicWorkProviderCheckpointV1::decode_wire_v1(&bytes, limits()),
            Err(IntrinsicWorkProviderWireError::InvalidData)
        );

        let image = provider().checkpoint_v1(limits().provider).unwrap();
        let mut bytes = image.encode_wire_v1(limits()).unwrap();
        let first = first_distribution_tag(&bytes) + 1 + 8;
        let second = first + 24;
        let first_ticks = bytes[first..first + 16].to_vec();
        bytes[second..second + 16].copy_from_slice(&first_ticks);
        assert!(matches!(
            IntrinsicWorkProviderCheckpointV1::restore_wire_v1(&bytes, limits()),
            Err(IntrinsicWorkProviderWireError::Native(
                IntrinsicWorkProviderCheckpointError::InvalidDistribution
            ))
        ));
    }
}
