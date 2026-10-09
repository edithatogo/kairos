//! Private, bounded canonical wire for calibration route metadata and receipts.
use super::*;
use std::error::Error;
use std::fmt::{Display, Formatter};

const META_MAGIC: &[u8; 8] = b"KMETV1\0\0";
const RECEIPT_MAGIC: &[u8; 8] = b"KRRCV1\0\0";
const SCHEMA: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct RouteReceiptWireLimits {
    pub(crate) native: RouteReceiptCheckpointLimits,
    pub(crate) max_wire_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RouteReceiptWireError {
    UnsupportedSchema(u16),
    InvalidTag,
    InvalidUtf8,
    Truncated,
    TrailingBytes,
    LimitExceeded,
    AllocationFailed,
    Native(RouteReceiptError),
}
impl Display for RouteReceiptWireError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedSchema(v) => write!(f, "unsupported route receipt wire schema {v}"),
            Self::InvalidTag => f.write_str("invalid route receipt wire tag"),
            Self::InvalidUtf8 => f.write_str("invalid route receipt UTF-8"),
            Self::Truncated => f.write_str("truncated route receipt wire data"),
            Self::TrailingBytes => f.write_str("trailing route receipt wire data"),
            Self::LimitExceeded => f.write_str("route receipt wire limit exceeded"),
            Self::AllocationFailed => f.write_str("route receipt wire allocation failed"),
            Self::Native(error) => write!(f, "{error:?}"),
        }
    }
}
impl Error for RouteReceiptWireError {}
type R<T = ()> = Result<T, RouteReceiptWireError>;

fn add(a: usize, b: usize) -> R<usize> {
    a.checked_add(b).ok_or(RouteReceiptWireError::LimitExceeded)
}
fn metadata_valid(image: &RouteMetadataCheckpointV1) -> R {
    if image.version != ROUTE_METADATA_VERSION_V1 {
        return Err(RouteReceiptWireError::Native(
            RouteReceiptError::UnsupportedMetadataVersion,
        ));
    }
    validate_purpose(&image.trip_purpose).map_err(RouteReceiptWireError::Native)?;
    if image.distance_provenance != DistanceProvenance::ConfiguredGeometry {
        return Err(RouteReceiptWireError::Native(
            RouteReceiptError::SensorObservationOnly,
        ));
    }
    Ok(())
}

impl RouteMetadataCheckpointV1 {
    pub(crate) fn wire_len_v1(&self, limits: RouteReceiptWireLimits) -> R<usize> {
        metadata_valid(self)?;
        let ids = self.trip_purpose.len();
        if ids > limits.native.max_identifier_bytes {
            return Err(RouteReceiptWireError::LimitExceeded);
        }
        let len = add(23, ids)?;
        if len > limits.max_wire_bytes {
            return Err(RouteReceiptWireError::LimitExceeded);
        }
        Ok(len)
    }
    pub(crate) fn encode_wire_v1(&self, limits: RouteReceiptWireLimits) -> R<Vec<u8>> {
        let length = self.wire_len_v1(limits)?;
        let mut out = Vec::new();
        out.try_reserve_exact(length)
            .map_err(|_| RouteReceiptWireError::AllocationFailed)?;
        out.extend_from_slice(META_MAGIC);
        out.extend_from_slice(&SCHEMA.to_le_bytes());
        out.extend_from_slice(&self.version.to_le_bytes());
        out.push(0); // ConfiguredGeometry.
        out.extend_from_slice(&(self.trip_purpose.len() as u64).to_le_bytes());
        out.extend_from_slice(self.trip_purpose.as_bytes());
        debug_assert_eq!(out.len(), length);
        Ok(out)
    }
    pub(crate) fn preflight_wire_v1(bytes: &[u8], limits: RouteReceiptWireLimits) -> R<usize> {
        if bytes.len() > limits.max_wire_bytes {
            return Err(RouteReceiptWireError::LimitExceeded);
        }
        let mut r = Reader::new(bytes);
        r.header(META_MAGIC)?;
        if r.u32()? != ROUTE_METADATA_VERSION_V1 {
            return Err(RouteReceiptWireError::Native(
                RouteReceiptError::UnsupportedMetadataVersion,
            ));
        }
        match r.u8()? {
            0 => {}
            1 => {
                return Err(RouteReceiptWireError::Native(
                    RouteReceiptError::SensorObservationOnly,
                ))
            }
            _ => return Err(RouteReceiptWireError::InvalidTag),
        }
        let n = r.count()?;
        if n > limits.native.max_identifier_bytes {
            return Err(RouteReceiptWireError::LimitExceeded);
        }
        let text =
            std::str::from_utf8(r.raw(n)?).map_err(|_| RouteReceiptWireError::InvalidUtf8)?;
        validate_purpose(text).map_err(RouteReceiptWireError::Native)?;
        r.finish()?;
        Ok(n)
    }
    pub(crate) fn decode_wire_v1(bytes: &[u8], limits: RouteReceiptWireLimits) -> R<Self> {
        Self::preflight_wire_v1(bytes, limits)?;
        let mut r = Reader::new(bytes);
        r.header(META_MAGIC)?;
        let version = r.u32()?;
        let distance_provenance = match r.u8()? {
            0 => DistanceProvenance::ConfiguredGeometry,
            _ => return Err(RouteReceiptWireError::InvalidTag),
        };
        let trip_purpose = r.string(limits.native.max_identifier_bytes)?;
        r.finish()?;
        Ok(Self {
            version,
            trip_purpose,
            distance_provenance,
        })
    }
}

impl RouteReceiptCheckpointV1 {
    pub(crate) fn canonical_bytes_len(&self) -> usize {
        self.canonical_bytes.len()
    }
    pub(crate) fn wire_len_v1(&self, limits: RouteReceiptWireLimits) -> R<usize> {
        if self.version != 1 {
            return Err(RouteReceiptWireError::Native(
                RouteReceiptError::UnsupportedMetadataVersion,
            ));
        }
        if self.canonical_bytes.len() > limits.native.max_canonical_bytes {
            return Err(RouteReceiptWireError::LimitExceeded);
        }
        let meta_len = self.metadata.wire_len_v1(limits)?;
        let digest: [u8; 32] = Sha256::digest(&self.canonical_bytes).into();
        if digest != self.sha256 {
            return Err(RouteReceiptWireError::Native(
                RouteReceiptError::RouteMismatch,
            ));
        }
        let len = add(8 + 2 + 4 + 8, meta_len)?;
        let len = add(add(len, 8)?, self.canonical_bytes.len())?;
        let len = add(len, 32)?;
        if len > limits.max_wire_bytes {
            return Err(RouteReceiptWireError::LimitExceeded);
        }
        Ok(len)
    }
    pub(crate) fn encode_wire_v1(&self, limits: RouteReceiptWireLimits) -> R<Vec<u8>> {
        let length = self.wire_len_v1(limits)?;
        let metadata = self.metadata.encode_wire_v1(limits)?;
        let mut out = Vec::new();
        out.try_reserve_exact(length)
            .map_err(|_| RouteReceiptWireError::AllocationFailed)?;
        out.extend_from_slice(RECEIPT_MAGIC);
        out.extend_from_slice(&SCHEMA.to_le_bytes());
        out.extend_from_slice(&self.version.to_le_bytes());
        out.extend_from_slice(&(metadata.len() as u64).to_le_bytes());
        out.extend_from_slice(&metadata);
        out.extend_from_slice(&(self.canonical_bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(&self.canonical_bytes);
        out.extend_from_slice(&self.sha256);
        debug_assert_eq!(out.len(), length);
        Ok(out)
    }
    pub(crate) fn preflight_wire_v1(
        bytes: &[u8],
        limits: RouteReceiptWireLimits,
    ) -> R<(usize, usize)> {
        if bytes.len() > limits.max_wire_bytes {
            return Err(RouteReceiptWireError::LimitExceeded);
        }
        let mut r = Reader::new(bytes);
        r.header(RECEIPT_MAGIC)?;
        if r.u32()? != 1 {
            return Err(RouteReceiptWireError::Native(
                RouteReceiptError::UnsupportedMetadataVersion,
            ));
        }
        let meta_len = r.count()?;
        if meta_len > r.remaining() {
            return Err(RouteReceiptWireError::Truncated);
        }
        let meta = r.raw(meta_len)?;
        let ids = RouteMetadataCheckpointV1::preflight_wire_v1(
            meta,
            RouteReceiptWireLimits {
                max_wire_bytes: meta_len,
                ..limits
            },
        )?;
        let canonical_len = r.count()?;
        if canonical_len > limits.native.max_canonical_bytes {
            return Err(RouteReceiptWireError::LimitExceeded);
        }
        let canonical = r.raw(canonical_len)?;
        let digest = r.raw(32)?;
        let expected: [u8; 32] = Sha256::digest(canonical).into();
        if digest != expected {
            return Err(RouteReceiptWireError::Native(
                RouteReceiptError::RouteMismatch,
            ));
        }
        r.finish()?;
        Ok((ids, canonical_len))
    }
    pub(crate) fn decode_wire_v1(bytes: &[u8], limits: RouteReceiptWireLimits) -> R<Self> {
        Self::preflight_wire_v1(bytes, limits)?;
        let mut r = Reader::new(bytes);
        r.header(RECEIPT_MAGIC)?;
        let version = r.u32()?;
        let meta_len = r.count()?;
        let metadata = RouteMetadataCheckpointV1::decode_wire_v1(
            r.raw(meta_len)?,
            RouteReceiptWireLimits {
                max_wire_bytes: meta_len,
                ..limits
            },
        )?;
        let canonical_len = r.count()?;
        let canonical_bytes = r.owned_bytes(canonical_len, limits.native.max_canonical_bytes)?;
        let sha256 = r
            .raw(32)?
            .try_into()
            .map_err(|_| RouteReceiptWireError::Truncated)?;
        r.finish()?;
        Ok(Self {
            version,
            metadata,
            canonical_bytes,
            sha256,
        })
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
    fn raw(&mut self, n: usize) -> R<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .ok_or(RouteReceiptWireError::LimitExceeded)?;
        let slice = self
            .bytes
            .get(self.at..end)
            .ok_or(RouteReceiptWireError::Truncated)?;
        self.at = end;
        Ok(slice)
    }
    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.at)
    }
    fn u8(&mut self) -> R<u8> {
        Ok(self.raw(1)?[0])
    }
    fn u16(&mut self) -> R<u16> {
        Ok(u16::from_le_bytes(self.raw(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> R<u32> {
        Ok(u32::from_le_bytes(self.raw(4)?.try_into().unwrap()))
    }
    fn count(&mut self) -> R<usize> {
        usize::try_from(u64::from_le_bytes(self.raw(8)?.try_into().unwrap()))
            .map_err(|_| RouteReceiptWireError::LimitExceeded)
    }
    fn string(&mut self, cap: usize) -> R<String> {
        let n = self.count()?;
        if n > cap {
            return Err(RouteReceiptWireError::LimitExceeded);
        }
        let value =
            std::str::from_utf8(self.raw(n)?).map_err(|_| RouteReceiptWireError::InvalidUtf8)?;
        let mut out = String::new();
        out.try_reserve_exact(n)
            .map_err(|_| RouteReceiptWireError::AllocationFailed)?;
        out.push_str(value);
        Ok(out)
    }
    fn owned_bytes(&mut self, n: usize, cap: usize) -> R<Vec<u8>> {
        if n > cap {
            return Err(RouteReceiptWireError::LimitExceeded);
        }
        let data = self.raw(n)?;
        let mut out = Vec::new();
        out.try_reserve_exact(n)
            .map_err(|_| RouteReceiptWireError::AllocationFailed)?;
        out.extend_from_slice(data);
        Ok(out)
    }
    fn header(&mut self, magic: &[u8; 8]) -> R {
        if self.raw(8)? != magic {
            return Err(RouteReceiptWireError::InvalidTag);
        }
        let schema = self.u16()?;
        if schema != SCHEMA {
            return Err(RouteReceiptWireError::UnsupportedSchema(schema));
        }
        Ok(())
    }
    fn finish(&self) -> R {
        if self.at == self.bytes.len() {
            Ok(())
        } else {
            Err(RouteReceiptWireError::TrailingBytes)
        }
    }
}
