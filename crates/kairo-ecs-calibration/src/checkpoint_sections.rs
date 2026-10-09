//! Canonical bounded section directory for the private C2 composite body.
//!
//! This module only frames opaque owner payloads. It does not interpret or
//! authorize any Flow, policy, provider, seed, or bridge content.

use kairo_ecs_types::EntityId;
use thiserror::Error;

const MAGIC: &[u8; 8] = b"KCSBODY1";
const SCHEMA_V1: u16 = 1;
const HEADER_BYTES: usize = 8 + 2 + 4;
const SINGLETON_COUNT: usize = 4;
const SINGLETON_FRAME_BYTES: usize = 1 + 8;
const RECORD_FRAME_BYTES: usize = 1 + 8 + 4 + 8;

const FLOW_TAG: u8 = 1;
const FIDELITY_TAG: u8 = 2;
const PROVIDER_TAG: u8 = 3;
const SEED_REGISTRY_TAG: u8 = 4;
const BOUND_TAG: u8 = 5;
const SUBMITTED_TAG: u8 = 6;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct C2SectionDirectoryLimitsV1 {
    pub(crate) max_wire_bytes: usize,
    pub(crate) max_sections: usize,
    pub(crate) max_bound_records: usize,
    pub(crate) max_submitted_records: usize,
    pub(crate) max_payload_bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum C2SectionDirectoryError {
    #[error("unsupported C2 section schema {0}")]
    UnsupportedSchema(u16),
    #[error("invalid C2 section body")]
    InvalidFormat,
    #[error("unknown C2 section tag")]
    InvalidTag,
    #[error("duplicate C2 singleton section")]
    DuplicateSingleton,
    #[error("required C2 singleton section is missing")]
    MissingSingleton,
    #[error("C2 section records are not in canonical full-ID order")]
    NonCanonical,
    #[error("C2 section directory limit exceeded")]
    LimitExceeded,
    #[error("truncated C2 section body")]
    Truncated,
    #[error("trailing C2 section bytes")]
    TrailingBytes,
    #[error("C2 section allocation failed")]
    AllocationFailed,
}

/// An owned per-work payload supplied by a caller such as the composite owner.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct C2RecordBytesV1 {
    pub(crate) source_work: EntityId,
    pub(crate) payload: Vec<u8>,
}

impl C2RecordBytesV1 {
    pub(crate) fn new(source_work: EntityId, payload: Vec<u8>) -> Self {
        Self {
            source_work,
            payload,
        }
    }
}

/// Borrowed inputs to the canonical body encoder. Payloads remain opaque.
pub(crate) struct C2BodyPartsV1<'a> {
    pub(crate) flow: &'a [u8],
    pub(crate) fidelity: &'a [u8],
    pub(crate) provider: &'a [u8],
    pub(crate) seed_registry: &'a [u8],
    pub(crate) bound: &'a [C2RecordBytesV1],
    pub(crate) submitted: &'a [C2RecordBytesV1],
}

impl C2BodyPartsV1<'_> {
    /// Measure and validate the entire directory before its single output reserve.
    pub(crate) fn encode(
        &self,
        limits: C2SectionDirectoryLimitsV1,
    ) -> Result<Vec<u8>, C2SectionDirectoryError> {
        let record_count = self
            .bound
            .len()
            .checked_add(self.submitted.len())
            .ok_or(C2SectionDirectoryError::LimitExceeded)?;
        let section_count = SINGLETON_COUNT
            .checked_add(record_count)
            .ok_or(C2SectionDirectoryError::LimitExceeded)?;
        if self.bound.len() > limits.max_bound_records
            || self.submitted.len() > limits.max_submitted_records
            || section_count > limits.max_sections
            || u32::try_from(section_count).is_err()
        {
            return Err(C2SectionDirectoryError::LimitExceeded);
        }

        let mut payload_bytes = 0usize;
        let mut body_len = HEADER_BYTES;
        for payload in [self.flow, self.fidelity, self.provider, self.seed_registry] {
            u64::try_from(payload.len()).map_err(|_| C2SectionDirectoryError::LimitExceeded)?;
            payload_bytes = add_cap(payload_bytes, payload.len(), limits.max_payload_bytes)?;
            body_len = add_cap(body_len, SINGLETON_FRAME_BYTES, limits.max_wire_bytes)?;
            body_len = add_cap(body_len, payload.len(), limits.max_wire_bytes)?;
        }

        // The two already-sorted owner slices are merged without a scratch
        // vector. Strict global order also rejects cross-kind duplicate IDs.
        validate_merged_records(self.bound, self.submitted)?;
        let mut bound_index = 0usize;
        let mut submitted_index = 0usize;
        while bound_index < self.bound.len() || submitted_index < self.submitted.len() {
            let (record, _) = next_merged(
                self.bound,
                self.submitted,
                &mut bound_index,
                &mut submitted_index,
            );
            u64::try_from(record.payload.len())
                .map_err(|_| C2SectionDirectoryError::LimitExceeded)?;
            payload_bytes = add_cap(
                payload_bytes,
                record.payload.len(),
                limits.max_payload_bytes,
            )?;
            body_len = add_cap(body_len, RECORD_FRAME_BYTES, limits.max_wire_bytes)?;
            body_len = add_cap(body_len, record.payload.len(), limits.max_wire_bytes)?;
        }

        let mut out = Vec::new();
        out.try_reserve_exact(body_len)
            .map_err(|_| C2SectionDirectoryError::AllocationFailed)?;
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&SCHEMA_V1.to_le_bytes());
        out.extend_from_slice(
            &u32::try_from(section_count)
                .map_err(|_| C2SectionDirectoryError::LimitExceeded)?
                .to_le_bytes(),
        );
        write_singleton(&mut out, FLOW_TAG, self.flow)?;
        write_singleton(&mut out, FIDELITY_TAG, self.fidelity)?;
        write_singleton(&mut out, PROVIDER_TAG, self.provider)?;
        write_singleton(&mut out, SEED_REGISTRY_TAG, self.seed_registry)?;

        bound_index = 0;
        submitted_index = 0;
        while bound_index < self.bound.len() || submitted_index < self.submitted.len() {
            let (record, tag) = next_merged(
                self.bound,
                self.submitted,
                &mut bound_index,
                &mut submitted_index,
            );
            write_record(&mut out, tag, record)?;
        }
        if out.len() != body_len {
            return Err(C2SectionDirectoryError::InvalidFormat);
        }
        Ok(out)
    }
}

fn next_merged<'a>(
    bound: &'a [C2RecordBytesV1],
    submitted: &'a [C2RecordBytesV1],
    bound_index: &mut usize,
    submitted_index: &mut usize,
) -> (&'a C2RecordBytesV1, u8) {
    if *submitted_index >= submitted.len()
        || (*bound_index < bound.len()
            && bound[*bound_index].source_work < submitted[*submitted_index].source_work)
    {
        let record = &bound[*bound_index];
        *bound_index += 1;
        (record, BOUND_TAG)
    } else {
        let record = &submitted[*submitted_index];
        *submitted_index += 1;
        (record, SUBMITTED_TAG)
    }
}

fn validate_merged_records(
    bound: &[C2RecordBytesV1],
    submitted: &[C2RecordBytesV1],
) -> Result<(), C2SectionDirectoryError> {
    if bound
        .windows(2)
        .any(|pair| pair[0].source_work >= pair[1].source_work)
        || submitted
            .windows(2)
            .any(|pair| pair[0].source_work >= pair[1].source_work)
    {
        return Err(C2SectionDirectoryError::NonCanonical);
    }
    let mut bound_index = 0usize;
    let mut submitted_index = 0usize;
    let mut previous = None;
    while bound_index < bound.len() || submitted_index < submitted.len() {
        let (record, _) = next_merged(bound, submitted, &mut bound_index, &mut submitted_index);
        if previous.is_some_and(|id| id >= record.source_work) {
            return Err(C2SectionDirectoryError::NonCanonical);
        }
        previous = Some(record.source_work);
    }
    Ok(())
}

fn add_cap(total: usize, next: usize, cap: usize) -> Result<usize, C2SectionDirectoryError> {
    let total = total
        .checked_add(next)
        .ok_or(C2SectionDirectoryError::LimitExceeded)?;
    if total > cap {
        return Err(C2SectionDirectoryError::LimitExceeded);
    }
    Ok(total)
}

fn write_len(out: &mut Vec<u8>, len: usize) -> Result<(), C2SectionDirectoryError> {
    out.extend_from_slice(
        &u64::try_from(len)
            .map_err(|_| C2SectionDirectoryError::LimitExceeded)?
            .to_le_bytes(),
    );
    Ok(())
}

fn write_singleton(
    out: &mut Vec<u8>,
    tag: u8,
    payload: &[u8],
) -> Result<(), C2SectionDirectoryError> {
    out.push(tag);
    write_len(out, payload.len())?;
    out.extend_from_slice(payload);
    Ok(())
}

fn write_record(
    out: &mut Vec<u8>,
    tag: u8,
    record: &C2RecordBytesV1,
) -> Result<(), C2SectionDirectoryError> {
    out.push(tag);
    out.extend_from_slice(&record.source_work.index.to_le_bytes());
    out.extend_from_slice(&record.source_work.generation.to_le_bytes());
    write_len(out, record.payload.len())?;
    out.extend_from_slice(&record.payload);
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct C2SectionDirectoryViewV1<'a> {
    bytes: &'a [u8],
    flow: (usize, usize),
    fidelity: (usize, usize),
    provider: (usize, usize),
    seed_registry: (usize, usize),
    record_start: usize,
    section_count: usize,
    bound_count: usize,
    submitted_count: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct C2SectionRecordRefV1<'a> {
    pub(crate) source_work: EntityId,
    pub(crate) payload: &'a [u8],
}

impl<'a> C2SectionDirectoryViewV1<'a> {
    /// Validate the whole body without allocating; the returned view borrows all
    /// payload bytes and can be passed directly to staged owner decoders.
    pub(crate) fn parse(
        bytes: &'a [u8],
        limits: C2SectionDirectoryLimitsV1,
    ) -> Result<Self, C2SectionDirectoryError> {
        let mut r = Reader::new(bytes);
        if bytes.len() > limits.max_wire_bytes {
            return Err(C2SectionDirectoryError::LimitExceeded);
        }
        if r.take(MAGIC.len())? != MAGIC {
            return Err(C2SectionDirectoryError::InvalidFormat);
        }
        let schema = r.u16()?;
        if schema != SCHEMA_V1 {
            return Err(C2SectionDirectoryError::UnsupportedSchema(schema));
        }
        let section_count =
            usize::try_from(r.u32()?).map_err(|_| C2SectionDirectoryError::LimitExceeded)?;
        if section_count < SINGLETON_COUNT {
            return Err(C2SectionDirectoryError::MissingSingleton);
        }
        if section_count > limits.max_sections {
            return Err(C2SectionDirectoryError::LimitExceeded);
        }

        let mut payload_total = 0usize;
        let mut singleton_ranges = [(0usize, 0usize); SINGLETON_COUNT];
        for (index, expected_tag) in [FLOW_TAG, FIDELITY_TAG, PROVIDER_TAG, SEED_REGISTRY_TAG]
            .into_iter()
            .enumerate()
        {
            let tag = r.u8()?;
            if tag != expected_tag {
                if (FLOW_TAG..=SEED_REGISTRY_TAG).contains(&tag) {
                    return Err(C2SectionDirectoryError::DuplicateSingleton);
                }
                if (BOUND_TAG..=SUBMITTED_TAG).contains(&tag) {
                    return Err(C2SectionDirectoryError::MissingSingleton);
                }
                return Err(C2SectionDirectoryError::InvalidTag);
            }
            let len = r.length()?;
            payload_total = add_cap(payload_total, len, limits.max_payload_bytes)?;
            let start = r.position();
            r.take(len)?;
            singleton_ranges[index] = (start, len);
        }

        let record_start = r.position();
        let mut bound_count = 0usize;
        let mut submitted_count = 0usize;
        let mut previous = None;
        for _ in SINGLETON_COUNT..section_count {
            let tag = r.u8()?;
            if tag != BOUND_TAG && tag != SUBMITTED_TAG {
                return Err(if (FLOW_TAG..=SEED_REGISTRY_TAG).contains(&tag) {
                    C2SectionDirectoryError::DuplicateSingleton
                } else {
                    C2SectionDirectoryError::InvalidTag
                });
            }
            let source_work = r.entity_id()?;
            if previous.is_some_and(|id| id >= source_work) {
                return Err(C2SectionDirectoryError::NonCanonical);
            }
            previous = Some(source_work);
            if tag == BOUND_TAG {
                bound_count = bound_count
                    .checked_add(1)
                    .ok_or(C2SectionDirectoryError::LimitExceeded)?;
                if bound_count > limits.max_bound_records {
                    return Err(C2SectionDirectoryError::LimitExceeded);
                }
            } else {
                submitted_count = submitted_count
                    .checked_add(1)
                    .ok_or(C2SectionDirectoryError::LimitExceeded)?;
                if submitted_count > limits.max_submitted_records {
                    return Err(C2SectionDirectoryError::LimitExceeded);
                }
            }
            let len = r.length()?;
            payload_total = add_cap(payload_total, len, limits.max_payload_bytes)?;
            r.take(len)?;
        }
        r.finish()?;
        Ok(Self {
            bytes,
            flow: singleton_ranges[0],
            fidelity: singleton_ranges[1],
            provider: singleton_ranges[2],
            seed_registry: singleton_ranges[3],
            record_start,
            section_count,
            bound_count,
            submitted_count,
        })
    }

    pub(crate) fn flow(&self) -> &'a [u8] {
        self.slice(self.flow)
    }
    pub(crate) fn fidelity(&self) -> &'a [u8] {
        self.slice(self.fidelity)
    }
    pub(crate) fn provider(&self) -> &'a [u8] {
        self.slice(self.provider)
    }
    pub(crate) fn seed_registry(&self) -> &'a [u8] {
        self.slice(self.seed_registry)
    }
    pub(crate) fn section_count(&self) -> usize {
        self.section_count
    }
    pub(crate) fn bound_count(&self) -> usize {
        self.bound_count
    }
    pub(crate) fn submitted_count(&self) -> usize {
        self.submitted_count
    }
    pub(crate) fn bound_records(&self) -> C2RecordIterV1<'a> {
        C2RecordIterV1::new(self.bytes, self.record_start, self.section_count, BOUND_TAG)
    }
    pub(crate) fn submitted_records(&self) -> C2RecordIterV1<'a> {
        C2RecordIterV1::new(
            self.bytes,
            self.record_start,
            self.section_count,
            SUBMITTED_TAG,
        )
    }
    pub(crate) fn record_frame_offsets(&self) -> impl Iterator<Item = usize> + 'a {
        let start = self.record_start;
        let count = self.section_count - SINGLETON_COUNT;
        RecordOffsetIterV1::new(self.bytes, start, count)
    }

    fn slice(&self, (start, len): (usize, usize)) -> &'a [u8] {
        &self.bytes[start..start + len]
    }
}

#[derive(Clone, Copy)]
pub(crate) struct C2RecordIterV1<'a> {
    reader: Reader<'a>,
    remaining: usize,
    wanted_tag: u8,
}

impl<'a> C2RecordIterV1<'a> {
    fn new(bytes: &'a [u8], start: usize, section_count: usize, wanted_tag: u8) -> Self {
        Self {
            reader: Reader { bytes, at: start },
            remaining: section_count - SINGLETON_COUNT,
            wanted_tag,
        }
    }
}

impl<'a> Iterator for C2RecordIterV1<'a> {
    type Item = C2SectionRecordRefV1<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.remaining > 0 {
            self.remaining -= 1;
            let tag = self.reader.u8().ok()?;
            let source_work = self.reader.entity_id().ok()?;
            let len = self.reader.length().ok()?;
            let payload = self.reader.take(len).ok()?;
            if tag == self.wanted_tag {
                return Some(C2SectionRecordRefV1 {
                    source_work,
                    payload,
                });
            }
        }
        None
    }
}

struct RecordOffsetIterV1<'a> {
    reader: Reader<'a>,
    remaining: usize,
}

impl<'a> RecordOffsetIterV1<'a> {
    fn new(bytes: &'a [u8], start: usize, remaining: usize) -> Self {
        Self {
            reader: Reader { bytes, at: start },
            remaining,
        }
    }
}

impl Iterator for RecordOffsetIterV1<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        let offset = self.reader.position();
        self.reader.u8().ok()?;
        self.reader.entity_id().ok()?;
        let len = self.reader.length().ok()?;
        self.reader.take(len).ok()?;
        Some(offset)
    }
}

#[derive(Clone, Copy)]
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    fn position(&self) -> usize {
        self.at
    }
    fn take(&mut self, len: usize) -> Result<&'a [u8], C2SectionDirectoryError> {
        let end = self
            .at
            .checked_add(len)
            .ok_or(C2SectionDirectoryError::LimitExceeded)?;
        let bytes = self
            .bytes
            .get(self.at..end)
            .ok_or(C2SectionDirectoryError::Truncated)?;
        self.at = end;
        Ok(bytes)
    }
    fn u8(&mut self) -> Result<u8, C2SectionDirectoryError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, C2SectionDirectoryError> {
        Ok(u16::from_le_bytes(
            self.take(2)?
                .try_into()
                .map_err(|_| C2SectionDirectoryError::InvalidFormat)?,
        ))
    }
    fn u32(&mut self) -> Result<u32, C2SectionDirectoryError> {
        Ok(u32::from_le_bytes(
            self.take(4)?
                .try_into()
                .map_err(|_| C2SectionDirectoryError::InvalidFormat)?,
        ))
    }
    fn u64(&mut self) -> Result<u64, C2SectionDirectoryError> {
        Ok(u64::from_le_bytes(
            self.take(8)?
                .try_into()
                .map_err(|_| C2SectionDirectoryError::InvalidFormat)?,
        ))
    }
    fn length(&mut self) -> Result<usize, C2SectionDirectoryError> {
        usize::try_from(self.u64()?).map_err(|_| C2SectionDirectoryError::LimitExceeded)
    }
    fn entity_id(&mut self) -> Result<EntityId, C2SectionDirectoryError> {
        Ok(EntityId::new(
            u64::from_le_bytes(
                self.take(8)?
                    .try_into()
                    .map_err(|_| C2SectionDirectoryError::InvalidFormat)?,
            ),
            u32::from_le_bytes(
                self.take(4)?
                    .try_into()
                    .map_err(|_| C2SectionDirectoryError::InvalidFormat)?,
            ),
        ))
    }
    fn finish(&self) -> Result<(), C2SectionDirectoryError> {
        if self.at == self.bytes.len() {
            Ok(())
        } else if self.at < self.bytes.len() {
            Err(C2SectionDirectoryError::TrailingBytes)
        } else {
            Err(C2SectionDirectoryError::InvalidFormat)
        }
    }
}
