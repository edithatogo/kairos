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

#[cfg(test)]
mod tests {
    use super::*;

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
