//! Lossless UTC binary storage and a strict normalized Gregorian text companion.
//! Source precision, raw values, lineage and clock roles remain separate.

const NS: i128 = 1_000_000_000;
const MIN_TEXT: i128 = -62_135_596_800_000_000_000;
const MAX_TEXT: i128 = 253_402_300_799_999_999_999;

/// Failure in normalized storage validation. Downstream matches require a wildcard.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum UtcCodecError {
    #[error("UTC binary value must contain exactly sixteen bytes")]
    InvalidByteLength,
    #[error("UTC text is not in canonical storage form")]
    NoncanonicalText,
    #[error("UTC text contains an invalid Gregorian calendar value")]
    InvalidCalendar,
    #[error("UTC value is outside the canonical text range")]
    OutOfTextRange,
    #[error("UTC binary and text companions disagree")]
    CompanionMismatch,
}

/// Encode every signed i128 nanosecond value without narrowing.
pub fn encode_utc_nanoseconds(value: i128) -> [u8; 16] {
    value.to_le_bytes()
}

/// Decode exactly sixteen little-endian two's-complement bytes.
pub fn decode_utc_nanoseconds(bytes: &[u8]) -> Result<i128, UtcCodecError> {
    let bytes: [u8; 16] = bytes
        .try_into()
        .map_err(|_| UtcCodecError::InvalidByteLength)?;
    Ok(i128::from_le_bytes(bytes))
}

fn leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn month_days(year: i64, month: i64) -> i64 {
    match month {
        2 if leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => 0,
    }
}

// March-based Gregorian eras. Inputs are validated years 1..=9999,
// hence every intermediate is bounded to a few million days.
fn civil_days(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

fn civil_date(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month, day)
}

/// Format ASCII30 UTC text for years 0001 through 9999, without leap-second folding.
pub fn format_canonical_utc(value: i128) -> Result<String, UtcCodecError> {
    if !(MIN_TEXT..=MAX_TEXT).contains(&value) {
        return Err(UtcCodecError::OutOfTextRange);
    }
    let seconds = value.div_euclid(NS);
    let fraction = value.rem_euclid(NS);
    let days =
        i64::try_from(seconds.div_euclid(86400)).map_err(|_| UtcCodecError::OutOfTextRange)?;
    let sod = seconds.rem_euclid(86400);
    let (year, month, day) = civil_date(days);
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{fraction:09}Z",
        sod / 3600,
        (sod / 60) % 60,
        sod % 60
    ))
}

fn decimal(bytes: &[u8]) -> i64 {
    bytes
        .iter()
        .fold(0, |value, digit| value * 10 + i64::from(digit - b'0'))
}

/// Parse only the canonical storage grammar; this is not a source timestamp parser.
pub fn parse_canonical_utc(text: &str) -> Result<i128, UtcCodecError> {
    let b = text.as_bytes();
    if b.len() != 30 || !b.is_ascii() {
        return Err(UtcCodecError::NoncanonicalText);
    }
    for (index, byte) in b.iter().enumerate() {
        let separator = match index {
            4 | 7 => Some(b'-'),
            10 => Some(b'T'),
            13 | 16 => Some(b':'),
            19 => Some(b'.'),
            29 => Some(b'Z'),
            _ => None,
        };
        if separator.map_or(!byte.is_ascii_digit(), |expected| *byte != expected) {
            return Err(UtcCodecError::NoncanonicalText);
        }
    }
    let year = decimal(&b[0..4]);
    if year == 0 {
        return Err(UtcCodecError::OutOfTextRange);
    }
    let month = decimal(&b[5..7]);
    let day = decimal(&b[8..10]);
    let hour = decimal(&b[11..13]);
    let minute = decimal(&b[14..16]);
    let second = decimal(&b[17..19]);
    if !(1..=12).contains(&month)
        || day < 1
        || day > month_days(year, month)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return Err(UtcCodecError::InvalidCalendar);
    }
    i128::from(civil_days(year, month, day))
        .checked_mul(86400)
        .and_then(|v| v.checked_add(i128::from(hour * 3600 + minute * 60 + second)))
        .and_then(|v| v.checked_mul(NS))
        .and_then(|v| v.checked_add(i128::from(decimal(&b[20..29]))))
        .ok_or(UtcCodecError::OutOfTextRange)
}

/// Validate both companions in byte-length, text-validity, then equality order.
pub fn check_utc_companion(bytes: &[u8], text: &str) -> Result<i128, UtcCodecError> {
    let value = decode_utc_nanoseconds(bytes)?;
    if value != parse_canonical_utc(text)? {
        return Err(UtcCodecError::CompanionMismatch);
    }
    Ok(value)
}
