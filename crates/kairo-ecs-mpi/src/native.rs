// SPDX-License-Identifier: Apache-2.0
//! Optional MPI API exchange for a fixed one-LP-per-rank communicator.
//!
//! All ranks must call collectives in identical order. This is a bounded preview
//! exchange primitive, not a fault-tolerant PDES/GVT implementation.
#![forbid(unsafe_code)]

use std::io::{self, ErrorKind};
use std::marker::PhantomData;
use std::rc::Rc;

use kairo_ecs_pdes::{LpId, NullMessage, PdesMessage, RemoteEvent, Tick};
use mpi::environment::Universe;
use mpi::topology::SimpleCommunicator;
use mpi::traits::*;

const MAX_BATCH_BYTES: usize = 64 * 1024;
const MAX_RANKS: usize = 128;
/// Largest opaque canonical signed record accepted by the native MPI lane.
pub const MAX_NATIVE_RECORD_BYTES: usize = 128 * 1024;
/// Maximum encoded opaque-record batch contributed by one rank.
pub const MAX_NATIVE_RECORD_BATCH_BYTES: usize = 512 * 1024;
/// Maximum records contributed by one rank in a round.
pub const MAX_NATIVE_RECORDS: usize = 64;
const RECORD_BATCH_VERSION: u8 = 1;

/// One opaque record addressed to a fixed communicator rank.
///
/// The transport supplies the source from the actual MPI rank. The bytes must
/// still be authenticated and bound to that rank by the external driver.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeMpiRecord {
    pub destination: u32,
    pub bytes: Vec<u8>,
}

/// Byte-exact record received from an actual fixed-communicator MPI rank.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeMpiInbound {
    /// Actual MPI source slot, never a value supplied by record bytes.
    pub source_rank: u32,
    /// Fixed destination rank; equals the receiving exchange's rank.
    pub destination: u32,
    /// Opaque record bytes, unchanged by framing or collective exchange.
    pub bytes: Vec<u8>,
}

fn invalid() -> io::Error {
    io::Error::new(
        ErrorKind::InvalidData,
        "invalid or oversized native MPI batch",
    )
}

/// Native MPI collective exchange with deterministic rank/FIFO delivery order.
///
/// Borrows the caller's MPI lifetime; does not initialize or finalize MPI. The
/// duplicated communicator isolates its collectives from other MPI users.
/// Every communicator rank maps to `LpId(rank)`. Maximum 128 ranks, 64 KiB per
/// rank/batch. MPI's configured error handler applies to native MPI failures;
/// process failure/restart is unsupported. Construct and use on the MPI
/// initializing thread; the caller must obey the provided MPI thread level.
/// This preview is explicitly neither Send nor Sync on every MPI implementation.
///
/// ```compile_fail
/// use kairo_ecs_mpi::native::NativeMpiExchange;
/// fn require_send<T: Send>() {}
/// require_send::<NativeMpiExchange<'static>>();
/// ```
/// ```compile_fail
/// use kairo_ecs_mpi::native::NativeMpiExchange;
/// fn require_sync<T: Sync>() {}
/// require_sync::<NativeMpiExchange<'static>>();
/// ```
pub struct NativeMpiExchange<'a> {
    communicator: SimpleCommunicator,
    _universe: PhantomData<&'a Universe>,
    _thread_affinity: PhantomData<Rc<()>>,
}

impl<'a> NativeMpiExchange<'a> {
    /// Collective constructor; all ranks in the supplied universe participate.
    pub fn new(universe: &'a Universe) -> io::Result<Self> {
        let world = universe.world();
        if world.size() <= 0 || world.size() as usize > MAX_RANKS {
            return Err(invalid());
        }
        Ok(Self {
            communicator: world.duplicate(),
            _universe: PhantomData,
            _thread_affinity: PhantomData,
        })
    }

    /// Native MPI communicator rank (also this participant's logical process).
    pub fn rank(&self) -> u32 {
        self.communicator.rank() as u32
    }

    /// Native MPI communicator size.
    pub fn size(&self) -> u32 {
        self.communicator.size() as u32
    }

    /// Exchanges one bounded batch per participant through `MPI_Allgather`.
    ///
    /// Invalid input at any rank rejects the round at every rank before payload
    /// exchange. All received batches are validated before returning any events.
    /// Calls are synchronous: every rank must participate, including empty ranks.
    /// This guarantees delivery at the exchange boundary, not durable admission,
    /// optimistic-message retirement, or a consistent cut of model state.
    pub fn exchange(&self, messages: &[PdesMessage]) -> io::Result<Vec<PdesMessage>> {
        let encoded = encode(messages, self.rank(), self.size());
        let length = encoded
            .as_ref()
            .map_or(u32::MAX, |bytes| bytes.len() as u32);
        let mut lengths = vec![0u32; self.size() as usize];
        self.communicator.all_gather_into(&length, &mut lengths[..]);
        if lengths.iter().any(|&n| n as usize > MAX_BATCH_BYTES) {
            return Err(invalid());
        }
        let encoded = encoded?;
        // Identical padded count at every rank; bound aggregate allocation to
        // 128 * 64 KiB. Empty rounds still exchange one byte per rank.
        let width = lengths.iter().copied().max().unwrap_or(0).max(1) as usize;
        let mut send = vec![0u8; width];
        send[..encoded.len()].copy_from_slice(&encoded);
        let mut received = vec![0u8; width * self.size() as usize];
        self.communicator
            .all_gather_into(&send[..], &mut received[..]);
        let mut delivered = Vec::new();
        for (rank, &length) in lengths.iter().enumerate() {
            let start = rank * width;
            let batch = decode(
                &received[start..start + length as usize],
                rank as u32,
                self.size(),
            )?;
            delivered.extend(
                batch
                    .into_iter()
                    .filter(|message| destination(message) == self.rank()),
            );
        }
        Ok(delivered)
    }

    /// Exchanges bounded opaque records in deterministic source-rank/FIFO order.
    ///
    /// This validates framing, destinations and allocation bounds only. It does
    /// not authenticate record contents or admit them to a durable engine. Use
    /// [`Self::exchange_records_validated`] to collectively enforce a pure
    /// protocol/signature/rank policy before any rank receives returned records.
    pub fn exchange_records(
        &self,
        records: &[NativeMpiRecord],
    ) -> io::Result<Vec<NativeMpiInbound>> {
        self.exchange_records_validated(records, |_, _, _| Ok(()))
    }

    /// Exchanges complete external-driver records using native MPI collectives.
    ///
    /// Every rank, including empty ranks, must call rounds in identical order.
    /// Invalid local input rejects the round at all ranks before payload
    /// exchange. After payload exchange, every rank validates every record in
    /// source-rank/FIFO order; a rejection at any rank rejects the round for all
    /// ranks before any result is returned. A subsequent valid round is usable.
    ///
    /// `validate(source_rank, destination_rank, bytes)` must be a pure local
    /// check: no engine mutations, I/O, MPI calls or publication. A signed
    /// external protocol should verify its immutable configuration, canonical
    /// frame, signature and issuer-to-source-rank binding here, then pass the
    /// returned records to the fallible durable engine. This API is a transport
    /// boundary, not an all-rank model transaction, receipt or GVT proof.
    ///
    /// Limits:128 ranks,64 records/rank,128 KiB/record,512 KiB encoded/rank.
    /// MPI_Allgather exposes all records to every rank (no confidentiality).
    /// Native MPI errors retain the communicator's error-handler semantics;
    /// rank loss can abort/block the job. Rust errors here represent bounded
    /// framing/policy rejection, not recoverable native MPI process failure.
    pub fn exchange_records_validated<F>(
        &self,
        records: &[NativeMpiRecord],
        mut validate: F,
    ) -> io::Result<Vec<NativeMpiInbound>>
    where
        F: FnMut(u32, u32, &[u8]) -> io::Result<()>,
    {
        let encoded = encode_records(records, self.size());
        let length = encoded
            .as_ref()
            .map_or(u32::MAX, |bytes| bytes.len() as u32);
        let mut lengths = vec![0u32; self.size() as usize];
        self.communicator.all_gather_into(&length, &mut lengths[..]);
        if lengths
            .iter()
            .any(|&length| length as usize > MAX_NATIVE_RECORD_BATCH_BYTES)
        {
            return Err(invalid());
        }
        let encoded = encoded?;
        let width = lengths.iter().copied().max().unwrap_or(0).max(1) as usize;
        let mut send = vec![0u8; width];
        send[..encoded.len()].copy_from_slice(&encoded);
        let mut received = vec![0u8; width * self.size() as usize];
        self.communicator
            .all_gather_into(&send[..], &mut received[..]);
        let mut delivered = Vec::new();
        let mut accepted = 1u8;
        'ranks: for (source_rank, &length) in lengths.iter().enumerate() {
            let start = source_rank * width;
            let Ok(batch) = decode_records(&received[start..start + length as usize], self.size())
            else {
                accepted = 0;
                break;
            };
            for record in batch {
                if validate(source_rank as u32, record.destination, &record.bytes).is_err() {
                    accepted = 0;
                    break 'ranks;
                }
                if record.destination == self.rank() {
                    delivered.push(NativeMpiInbound {
                        source_rank: source_rank as u32,
                        destination: record.destination,
                        bytes: record.bytes,
                    });
                }
            }
        }
        // Always enter this collective after payload exchange, even after a
        // local decoder/policy rejection. No rank publishes a partial result.
        let mut acceptance = vec![0u8; self.size() as usize];
        self.communicator
            .all_gather_into(&accepted, &mut acceptance[..]);
        if acceptance.iter().any(|&value| value != 1) {
            return Err(invalid());
        }
        Ok(delivered)
    }

    /// Reduces full-width scalar contributions using a native MPI collective.
    ///
    /// This is NOT GVT: the caller must account for model queues, optimistic
    /// obligations and messages in flight before using a minimum for reclamation.
    pub fn minimum(&self, contribution: Tick) -> Tick {
        let ticks = contribution.ticks();
        let words = [(ticks >> 64) as u64, ticks as u64];
        let mut gathered = vec![0u64; 2 * self.size() as usize];
        self.communicator
            .all_gather_into(&words[..], &mut gathered[..]);
        gathered
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| Tick::from_ticks(((pair[0] as u128) << 64) | pair[1] as u128))
            .min()
            .unwrap_or(Tick::ZERO)
    }

    /// Native MPI barrier; not a durable commit or model-state synchronization.
    pub fn barrier(&self) {
        self.communicator.barrier();
    }
}

fn destination(message: &PdesMessage) -> u32 {
    match message {
        PdesMessage::Event(event) => event.dest_lp.0,
        PdesMessage::Null(null) => null.dest_lp.0,
    }
}

fn encode(messages: &[PdesMessage], rank: u32, size: u32) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for message in messages {
        let (tag, source, dest, tick, payload): (u8, _, _, _, &[u8]) = match message {
            PdesMessage::Event(event) => (
                0,
                event.source_lp.0,
                event.dest_lp.0,
                event.tick,
                &event.event_payload,
            ),
            PdesMessage::Null(null) => (1, null.source_lp.0, null.dest_lp.0, null.safe_time, &[]),
        };
        // 29 bytes: tag, source, destination, tick, payload length.
        if source != rank
            || dest >= size
            || payload.len() > MAX_BATCH_BYTES
            || bytes.len() + 29 + payload.len() > MAX_BATCH_BYTES
        {
            return Err(invalid());
        }
        bytes.push(tag);
        bytes.extend_from_slice(&source.to_be_bytes());
        bytes.extend_from_slice(&dest.to_be_bytes());
        bytes.extend_from_slice(&tick.ticks().to_be_bytes());
        bytes.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        bytes.extend_from_slice(payload);
    }
    Ok(bytes)
}

fn decode(mut bytes: &[u8], rank: u32, size: u32) -> io::Result<Vec<PdesMessage>> {
    let mut messages = Vec::new();
    while !bytes.is_empty() {
        if bytes.len() < 29 {
            return Err(invalid());
        }
        let tag = bytes[0];
        let source = u32::from_be_bytes(bytes[1..5].try_into().unwrap());
        let dest = u32::from_be_bytes(bytes[5..9].try_into().unwrap());
        let tick = Tick::from_ticks(u128::from_be_bytes(bytes[9..25].try_into().unwrap()));
        let length = u32::from_be_bytes(bytes[25..29].try_into().unwrap()) as usize;
        bytes = &bytes[29..];
        if source != rank || dest >= size || length > bytes.len() {
            return Err(invalid());
        }
        let message = match tag {
            0 => PdesMessage::Event(RemoteEvent {
                source_lp: LpId(source),
                dest_lp: LpId(dest),
                tick,
                event_payload: bytes[..length].to_vec(),
            }),
            1 if length == 0 => PdesMessage::Null(NullMessage {
                source_lp: LpId(source),
                dest_lp: LpId(dest),
                safe_time: tick,
            }),
            _ => return Err(invalid()),
        };
        messages.push(message);
        bytes = &bytes[length..];
    }
    Ok(messages)
}

fn encode_records(records: &[NativeMpiRecord], size: u32) -> io::Result<Vec<u8>> {
    if records.len() > MAX_NATIVE_RECORDS || size == 0 || size as usize > MAX_RANKS {
        return Err(invalid());
    }
    let mut bytes = vec![RECORD_BATCH_VERSION];
    bytes.extend_from_slice(&(records.len() as u16).to_be_bytes());
    for record in records {
        if record.destination >= size
            || record.bytes.is_empty()
            || record.bytes.len() > MAX_NATIVE_RECORD_BYTES
            || bytes.len() + 8 + record.bytes.len() > MAX_NATIVE_RECORD_BATCH_BYTES
        {
            return Err(invalid());
        }
        bytes.extend_from_slice(&record.destination.to_be_bytes());
        bytes.extend_from_slice(&(record.bytes.len() as u32).to_be_bytes());
        bytes.extend_from_slice(&record.bytes);
    }
    Ok(bytes)
}

fn decode_records(mut bytes: &[u8], size: u32) -> io::Result<Vec<NativeMpiRecord>> {
    if bytes.len() < 3
        || bytes.len() > MAX_NATIVE_RECORD_BATCH_BYTES
        || bytes[0] != RECORD_BATCH_VERSION
        || size == 0
        || size as usize > MAX_RANKS
    {
        return Err(invalid());
    }
    let count = u16::from_be_bytes(bytes[1..3].try_into().unwrap()) as usize;
    if count > MAX_NATIVE_RECORDS {
        return Err(invalid());
    }
    bytes = &bytes[3..];
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        if bytes.len() < 8 {
            return Err(invalid());
        }
        let destination = u32::from_be_bytes(bytes[..4].try_into().unwrap());
        let length = u32::from_be_bytes(bytes[4..8].try_into().unwrap()) as usize;
        bytes = &bytes[8..];
        if destination >= size
            || length == 0
            || length > MAX_NATIVE_RECORD_BYTES
            || length > bytes.len()
        {
            return Err(invalid());
        }
        records.push(NativeMpiRecord {
            destination,
            bytes: bytes[..length].to_vec(),
        });
        bytes = &bytes[length..];
    }
    if !bytes.is_empty() {
        return Err(invalid());
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opaque_codec_rejects_noncanonical_and_out_of_bound_batches() {
        let records = vec![NativeMpiRecord {
            destination: 1,
            bytes: vec![0, 255, 42],
        }];
        let encoded = encode_records(&records, 2).unwrap();
        assert_eq!(decode_records(&encoded, 2).unwrap(), records);
        for length in 0..encoded.len() {
            assert!(decode_records(&encoded[..length], 2).is_err());
        }
        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(decode_records(&trailing, 2).is_err());
        let mut version = encoded.clone();
        version[0] += 1;
        assert!(decode_records(&version, 2).is_err());
        let mut excess_count = encoded.clone();
        excess_count[1..3].copy_from_slice(&65u16.to_be_bytes());
        assert!(decode_records(&excess_count, 2).is_err());
        assert!(decode_records(&encoded, 1).is_err());
        assert!(encode_records(&records, 1).is_err());
        assert!(encode_records(&[], 0).is_err());
        assert!(decode_records(&encode_records(&[], 2).unwrap(), 0).is_err());
        assert!(encode_records(&vec![records[0].clone(); 65], 2).is_err());
        assert!(encode_records(
            &[NativeMpiRecord {
                destination: 0,
                bytes: vec![]
            }],
            2
        )
        .is_err());
        let mut empty_payload = encoded.clone();
        empty_payload[7..11].copy_from_slice(&0u32.to_be_bytes());
        assert!(decode_records(&empty_payload, 2).is_err());
        let mut oversized_payload = encoded.clone();
        oversized_payload[7..11]
            .copy_from_slice(&((MAX_NATIVE_RECORD_BYTES + 1) as u32).to_be_bytes());
        assert!(decode_records(&oversized_payload, 2).is_err());
    }

    #[test]
    fn opaque_codec_exact_packet_batch_and_count_boundaries() {
        let record = NativeMpiRecord {
            destination: 0,
            bytes: vec![0x5a; MAX_NATIVE_RECORD_BYTES],
        };
        assert_eq!(
            decode_records(
                &encode_records(std::slice::from_ref(&record), 1).unwrap(),
                1
            )
            .unwrap(),
            vec![record.clone()]
        );
        let mut oversized = record.clone();
        oversized.bytes.push(0);
        assert!(encode_records(&[oversized], 1).is_err());
        let mut batch = vec![record; 4];
        batch[3].bytes.truncate(MAX_NATIVE_RECORD_BYTES - 35);
        let encoded = encode_records(&batch, 1).unwrap();
        assert_eq!(encoded.len(), MAX_NATIVE_RECORD_BATCH_BYTES);
        assert_eq!(decode_records(&encoded, 1).unwrap(), batch);
        batch[3].bytes.push(0);
        assert!(encode_records(&batch, 1).is_err());
        let count_boundary = vec![
            NativeMpiRecord {
                destination: 0,
                bytes: vec![1]
            };
            MAX_NATIVE_RECORDS
        ];
        assert_eq!(
            decode_records(&encode_records(&count_boundary, 1).unwrap(), 1).unwrap(),
            count_boundary
        );
    }

    #[test]
    fn codec_rejects_truncation_spoofed_rank_and_unknown_tag() {
        let message = PdesMessage::Event(RemoteEvent {
            source_lp: LpId(0),
            dest_lp: LpId(1),
            tick: Tick::from_ticks(u128::MAX),
            event_payload: vec![1, 2, 3],
        });
        let bytes = encode(std::slice::from_ref(&message), 0, 2).unwrap();
        assert_eq!(decode(&bytes, 0, 2).unwrap(), vec![message]);
        for length in 1..bytes.len() {
            assert!(decode(&bytes[..length], 0, 2).is_err());
        }
        assert!(decode(&bytes, 1, 2).is_err());
        assert!(decode(&bytes, 0, 1).is_err());
        let mut invalid_tag = bytes.clone();
        invalid_tag[0] = 2;
        assert!(decode(&invalid_tag, 0, 2).is_err());
        let mut invalid_null = bytes.clone();
        invalid_null[0] = 1;
        assert!(decode(&invalid_null, 0, 2).is_err());
    }
}
