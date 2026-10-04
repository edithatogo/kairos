# C1 opt-in UTC companion codec contract candidate

Status: internally approved experimental Track 04/25 contract and dependency-free implementation direction; no external owner signature or release acceptance. No source implementation, external maintainer signature, release approval or full C1 completion. Base 4a90ebce58d8d31655d89a6c7c35680241c0dec8. This contract does not change general source adapters, legacy event_log.v1, schema wire fields, clinical applicability or temporal-helper precedence.

## Proposed public surface

Opt-in namespace `kairo_ecs_arrow::trace_time::utc_codec`, publicly reachable module; source path `crates/kairo-ecs-arrow/src/trace_time/utc_codec.rs` with only module declaration in existing trace_time.rs. No new facade, lib root reexport or package.

```
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum UtcCodecError {
    InvalidByteLength,
    NoncanonicalText,
    InvalidCalendar,
    OutOfTextRange,
    CompanionMismatch,
}
pub fn encode_utc_nanoseconds(value: i128) -> [u8; 16];
pub fn decode_utc_nanoseconds(bytes: &[u8]) -> Result<i128, UtcCodecError>;
pub fn format_canonical_utc(value: i128) -> Result<String, UtcCodecError>;
pub fn parse_canonical_utc(text: &str) -> Result<i128, UtcCodecError>;
pub fn check_utc_companion(bytes: &[u8], text: &str) -> Result<i128, UtcCodecError>;
```

No Default/conversion trait implicit coercion, panic or rounding. Errors expose std::error::Error and Display without carrying raw patient strings. Existing TemporalError/NormalizedTimestamp/ClockRole unchanged by this codec. No API aliases to generic RFC3339/calendar/source parser. UtcCodecError is non_exhaustive from first introduction: downstream matches require a wildcard; future errors may be added under governance without requiring exhaustive downstream arms. New namespace retains experimental development/release hold. Source callers opt in explicitly; existing callers need no migration. No universal stable compatibility guarantee or C ABI/binding promise.

## Exact policy and rejection ordering

Binary signed i128 two's-complement little endian, exactly16 bytes, all i128 values lossless. encode cannot fail; decode rejects other lengths. No origin calculation, timestamp unit scaling or unsigned tick decoding here.

Canonical text ASCII exactly30 bytes `YYYY-MM-DDTHH:MM:SS.nnnnnnnnnZ`; year0001..9999, Gregorian valid date, hour00..23, minute/second00..59, exactly9 fractional digits. Uppercase T/Z only. No leap-second folding, timezone/IANA/DST lookup, offsets, whitespace, Unicode digits or permissive normalization. POSIX UTC epoch nanoseconds with Euclidean negative decomposition. Explicit supported text interval: -62135596800000000000 through 253402300799999999999 inclusive. Values outside yield OutOfTextRange before date arithmetic; i128 extremes remain valid binary values but invalid text-format inputs.

parse first checks exactlength/ASCII/separators/digits (NoncanonicalText), then year0000 (OutOfTextRange), then invalid month/day/time including leap seconds (InvalidCalendar), then checked calendar-to-nanosecond result. Signed/extended years fail grammar, not a coerced narrower value. check_utc_companion first decodes bytes, then parses text, then compares exact i128; InvalidByteLength takes precedence, then parser error, then CompanionMismatch. A mismatch cannot be rescued by preferring one side.

Canonical nine-digit storage fractions never replace separate source precision/raw text/lineage/clock role. Input source timestamp adapters retain their approved formats; rejection by this codec applies only normalized companion text. Existing strict JSON time_value.utc remains date-time text; numeric companion MUST NOT appear in the JSON object. OutOfTextRange-to-trace_exclusion reason mapping is UNDECIDED and outside this codec: it emits no exclusions, manifest or count. No implied narrowing of full relative_ticks arithmetic or knowledge clock semantics.

## Reviewed implementation choice and alternatives

Approved bounded implementation direction: dependency-free checked integer Gregorian arithmetic, no optional unused foundation. Rationale: this small grammar/range is deliberately narrower than general RFC3339; current Arrow floor1.76 remains usable, no manifest/lock churn. Required arithmetic qualification: independent anchor records, leap-century and 400-year boundaries, negative nanoseconds, all month boundaries, independent generated year/day vectors, exact roundtrips and all rejection precedence; generic roundtrip alone is insufficient. No dead-code suppression. Public production codec is intended to be consumed by the later approved outer adapter; fixture success does not claim that integration exists.

Alternative maintained `time` dependency has i128 Unix nanosecond constructors/extraction and parsing/formatting support, but standard RFC3339 accepts/prints variable fractional widths and is not this exact canonical grammar. Strict wrapper still required. Current primary upstream workspace declares edition2024/Rust1.88, incompatible with unconditional legacy Arrow1.76. A reviewed optional adapter package could use current time with format/parsing features, but requires separate placement/MSRV/dependency/lock/policy gates; local-offset/macros/large-dates unnecessary for this profile. No dependency added or older pin selected merely to avoid support governance. See hash-bound dependency comparison artifact for primary sources.

## Owner and delivery boundaries

Track 04 owns two runtime paths and physical encoding; public contract ADR/disposition required. Track 25 owns public symbol/error/migration/release classification; handoffs append scope without rewriting historical closure. Track 21 consumes policy and owns later semantic exclusions/counts/mapping; Track 30 reviews any dependency/floor decision. This governance adoption contains one new contract plus append-only Track04/25 handoff references only, preserving broad contracts and existing package manifests.

After contract approval F1 one new integration fixture `tests/trace_utc_codec_v1.rs` with missing public namespace/functions/types is genuine compiler red, not an unused private module. Then F2 two owned runtime paths implements real methods; all Arrow tests1.76/current plus actual independent codec fixtures, formatter and source hashes required. No stubs, test-only public export or suppression. Existing23 tests remain immutable. No source/test dispatch authorized by governance adoption alone.

Full C1 physical schema/UTC text-numeric pairing in actual mapper, wide/long counts/raw exclusions, dataset identity/rank/partial order/occurrence pairing, stable external sort/spill/canonical hashes, occupancy/capacity/observation/censoring, standards/endpoint negative controls and independent IPC/Parquet reader remain required.
