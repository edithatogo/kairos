#![forbid(unsafe_code)]

//! Deterministic entity/component state for KairoECS.
//!
//! # Why this crate looks the way it does
//!
//! KairoECS replays runs and compares traces byte-for-byte, so *iteration order
//! is a product invariant, not an implementation detail*. Every ordered path in
//! this crate therefore derives its order from data that is stable across runs,
//! processes, and platforms:
//!
//! * entity order is `(index, generation)`, the same key [`World::snapshot`]
//!   already uses,
//! * component-type order is registration order, not hash order,
//! * [`ComponentRegistry`] keeps its columns in a `Vec` and finds types by
//!   linear scan, so no randomized hash seed can reach an ordered output, and
//! * a [`QueryPlan`] materialises its candidate list once, so execution never
//!   observes the internal dense-row order that `swap_remove` shuffles.
//!
//! # Why the columns are cache-line aligned
//!
//! [`ComponentStore`] defaults to [`CacheLineLayout`]: every column base address
//! is 64-byte aligned and every row starts on a cache-line boundary, so a SIMD
//! kernel can issue aligned loads without a scalar prologue and two entities can
//! never share a line. [`TightLayout`] keeps rows unit-stride for Arrow-style
//! buffer bodies. Rust cannot offer both properties for an arbitrary `T` without
//! `unsafe`, which this crate forbids, so both are offered explicitly instead of
//! hiding the trade-off. See [`ColumnLayout`].

pub mod hierarchy;
pub use hierarchy::*;

use std::any::{type_name, Any, TypeId};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::num::NonZeroUsize;

use kairo_ecs_types::EntityId;

/// Deterministic snapshot of one live entity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EntitySnapshot {
    /// The entity this snapshot entry describes.
    pub id: EntityId,
}

/// Deterministic world snapshot for downstream telemetry and visualization.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WorldSnapshot {
    entities: Vec<EntitySnapshot>,
}

impl WorldSnapshot {
    /// Snapshotted entities, ordered by `(index, generation)`.
    pub fn entities(&self) -> &[EntitySnapshot] {
        &self.entities
    }

    /// Number of snapshotted entities.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns `true` when no entities are snapshotted.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

fn entity_index(entity: EntityId) -> Option<usize> {
    usize::try_from(entity.index).ok()
}

/// Total, reproducible sort key for entities.
///
/// Indices are recycled by [`World::despawn`], so `(index, generation)` rather
/// than `index` alone is the key that stays meaningful after a slot is reused.
/// It is the key [`World::snapshot`] already sorts by, which keeps snapshot
/// order and query order from ever disagreeing.
fn entity_order(entity: &EntityId) -> (u64, u32) {
    (entity.index, entity.generation)
}

fn encode_dense_position(position: usize) -> NonZeroUsize {
    NonZeroUsize::new(position + 1).expect("dense positions are one-based")
}

fn decode_dense_position(position: NonZeroUsize) -> usize {
    position.get() - 1
}

#[derive(Clone, Debug, Default)]
struct EntitySlot {
    generation: u32,
    alive: bool,
}

/// Generational entity store with reusable indices and deterministic snapshots.
///
/// The chosen Track 01 storage shape is a minimal sparse-set-style allocator:
/// - entity indices are recycled through a free list,
/// - generations advance on despawn so stale handles are rejected,
/// - live entities are kept in a dense vector for cheap iteration and stable
///   snapshot construction.
#[derive(Debug, Default)]
pub struct World {
    slots: Vec<EntitySlot>,
    free_indices: Vec<u64>,
    live_entities: Vec<EntityId>,
    live_positions: Vec<Option<NonZeroUsize>>,
}

impl World {
    /// Creates an empty world.
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty world with room for `entity_capacity` entities.
    ///
    /// Reserving up front keeps allocation out of the middle of a modelled
    /// run, so allocation timing cannot become part of the replayed state.
    pub fn with_capacity(entity_capacity: usize) -> Self {
        Self {
            slots: Vec::with_capacity(entity_capacity),
            free_indices: Vec::with_capacity(entity_capacity),
            live_entities: Vec::with_capacity(entity_capacity),
            live_positions: Vec::with_capacity(entity_capacity),
        }
    }

    /// Reserves room for `additional_entities` more entities.
    pub fn reserve(&mut self, additional_entities: usize) {
        self.slots.reserve(additional_entities);
        self.free_indices.reserve(additional_entities);
        self.live_entities.reserve(additional_entities);
        self.live_positions.reserve(additional_entities);
    }

    /// Allocates an entity, recycling a freed index when one is available.
    ///
    /// The free list is a stack, so indices are reused in the exact reverse of
    /// despawn order. That is what keeps index assignment reproducible for a
    /// given sequence of spawn and despawn calls.
    pub fn spawn(&mut self) -> EntityId {
        let index = if let Some(index) = self.free_indices.pop() {
            index as usize
        } else {
            self.slots.push(EntitySlot::default());
            self.slots.len() - 1
        };

        let slot = &mut self.slots[index];
        debug_assert!(!slot.alive);

        let entity = EntityId {
            index: index as u64,
            generation: slot.generation,
        };

        slot.alive = true;

        if index >= self.live_positions.len() {
            self.live_positions.resize(index + 1, None);
        }

        self.live_positions[index] = Some(encode_dense_position(self.live_entities.len()));
        self.live_entities.push(entity);

        entity
    }

    /// Retires `id` and advances its slot's generation.
    ///
    /// Returns `false` for an unknown index or a stale generation, so both a
    /// double despawn and a handle to an already-recycled index are rejected
    /// instead of corrupting the free list.
    pub fn despawn(&mut self, id: EntityId) -> bool {
        let Some(index) = entity_index(id) else {
            return false;
        };

        let Some(slot) = self.slots.get_mut(index) else {
            return false;
        };
        if !slot.alive || slot.generation != id.generation {
            return false;
        }

        let Some(position) = self
            .live_positions
            .get_mut(index)
            .and_then(|slot| slot.take())
            .map(decode_dense_position)
        else {
            return false;
        };

        let removed = self.live_entities.swap_remove(position);
        debug_assert_eq!(removed, id);

        if let Some(swapped_entity) = self.live_entities.get(position).copied() {
            self.live_positions[swapped_entity.index as usize] =
                Some(encode_dense_position(position));
        }

        slot.alive = false;
        slot.generation = slot.generation.wrapping_add(1);
        self.free_indices.push(id.index);

        true
    }

    /// Returns `true` only when `id` names the current occupant of its slot.
    pub fn is_alive(&self, id: EntityId) -> bool {
        let Some(index) = entity_index(id) else {
            return false;
        };
        self.slots
            .get(index)
            .is_some_and(|slot| slot.alive && slot.generation == id.generation)
    }

    /// Number of live entities.
    pub fn len(&self) -> usize {
        self.live_entities.len()
    }

    /// Returns `true` when no entity is live.
    pub fn is_empty(&self) -> bool {
        self.live_entities.is_empty()
    }

    /// Captures the live set in `(index, generation)` order.
    ///
    /// The dense live list is in spawn order, which is not the order replay
    /// output must use; sorting here is what makes the snapshot independent of
    /// removal history.
    pub fn snapshot(&self) -> WorldSnapshot {
        let mut entities = self
            .live_entities
            .iter()
            .copied()
            .map(|id| EntitySnapshot { id })
            .collect::<Vec<_>>();
        entities.sort_unstable_by_key(|entity| entity_order(&entity.id));

        WorldSnapshot { entities }
    }
}

/// Cache-line size, in bytes, that [`CacheLineLayout`] guarantees.
///
/// 64 bytes is the architectural cache line on every x86-64 and AArch64 target
/// KairoECS supports, and it is also the alignment Arrow's buffer allocator
/// targets, which is what makes the two column layouts comparable.
pub const CACHE_LINE_BYTES: usize = 64;

/// One component row pinned to a cache-line boundary.
///
/// `repr(align(64))` is applied by the compiler, so no `unsafe` is required to
/// obtain the alignment; it also rounds the row size up to a multiple of 64, so
/// consecutive rows never share a line. The trade-off is memory amplification
/// for small components (`i32` costs 16x its payload); use [`TightLayout`] when
/// the column is going to be memcpy'd into an Arrow buffer instead of scanned.
#[repr(C, align(64))]
pub struct CacheLine<T> {
    value: T,
}

impl<T> CacheLine<T> {
    /// Wraps `value` in a cache-line-aligned row.
    pub const fn new(value: T) -> Self {
        Self { value }
    }

    /// Borrows the payload.
    pub fn get(&self) -> &T {
        &self.value
    }

    /// Mutably borrows the payload.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    /// Unwraps the payload, dropping the alignment padding.
    pub fn into_inner(self) -> T {
        self.value
    }
}

/// Memory placement strategy for one component column.
///
/// Implementations choose how a row is represented. Every row-based accessor on
/// [`ComponentStore`] goes through this trait, so the layout is a property of
/// the store type rather than something callers re-derive at each call site.
pub trait ColumnLayout: 'static {
    /// Storage representation of a single row under this layout.
    type Slot<T>;

    /// Base-address alignment, in bytes, guaranteed for a column of `T`.
    fn alignment<T>() -> usize;

    /// Wraps a payload into a row.
    fn from_value<T>(value: T) -> Self::Slot<T>;

    /// Unwraps a row back into its payload.
    fn into_value<T>(slot: Self::Slot<T>) -> T;

    /// Borrows the payload of a row.
    fn project<T>(slot: &Self::Slot<T>) -> &T;

    /// Mutably borrows the payload of a row.
    fn project_mut<T>(slot: &mut Self::Slot<T>) -> &mut T;
}

/// Unit-stride layout: rows are exactly `size_of::<T>()` and adjacent.
///
/// This is the layout Arrow's `Buffer` bodies and every SIMD gather/scatter loop
/// expect, but the base address is only as aligned as `T` itself requires.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TightLayout;

impl ColumnLayout for TightLayout {
    type Slot<T> = T;

    fn alignment<T>() -> usize {
        std::mem::align_of::<T>()
    }

    fn from_value<T>(value: T) -> Self::Slot<T> {
        value
    }

    fn into_value<T>(slot: Self::Slot<T>) -> T {
        slot
    }

    fn project<T>(slot: &Self::Slot<T>) -> &T {
        slot
    }

    fn project_mut<T>(slot: &mut Self::Slot<T>) -> &mut T {
        slot
    }
}

/// Cache-line-aligned layout: rows start on a 64-byte boundary and are padded
/// to a multiple of 64 bytes.
///
/// This is the default for [`ComponentStore`] because it is what a scan-heavy
/// kernel wants: aligned vector loads, and no false sharing when two component
/// columns for different entities land on the same line.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CacheLineLayout;

impl ColumnLayout for CacheLineLayout {
    type Slot<T> = CacheLine<T>;

    fn alignment<T>() -> usize {
        CACHE_LINE_BYTES
    }

    fn from_value<T>(value: T) -> Self::Slot<T> {
        CacheLine::new(value)
    }

    fn into_value<T>(slot: Self::Slot<T>) -> T {
        slot.into_inner()
    }

    fn project<T>(slot: &Self::Slot<T>) -> &T {
        slot.get()
    }

    fn project_mut<T>(slot: &mut Self::Slot<T>) -> &mut T {
        slot.get_mut()
    }
}

/// A sparse set storing components of a single type.
/// Dense array: contiguous storage of components for alive entities.
/// Sparse array: maps EntityId index -> position in dense array.
///
/// The dense array is a Structure-of-Arrays column: every component of type `T`
/// sits at the same offset within its row, so a kernel can walk the column with
/// unit stride instead of chasing a per-entity allocation.
///
/// Dense row order is *storage* order, not simulation order: [`Self::remove`]
/// uses `swap_remove`, which moves the last row into the freed slot. Anything
/// whose order is observable must go through [`ComponentRegistry::query`], which
/// materialises a sorted [`QueryPlan`] first.
pub struct ComponentStore<T, L: ColumnLayout = CacheLineLayout> {
    dense: Vec<L::Slot<T>>,
    sparse: Vec<Option<SparseEntry>>,
    entities: Vec<EntityId>,
}

/// Sparse-set bookkeeping: the generation that owns a dense slot, plus the
/// one-based dense position (one-based so `None` can mean "absent").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct SparseEntry {
    generation: u32,
    position: NonZeroUsize,
}

impl<T> ComponentStore<T, CacheLineLayout> {
    /// Creates an empty cache-line-aligned column.
    pub fn new() -> Self {
        Self::from_parts(Vec::new(), Vec::new(), Vec::new())
    }

    /// Creates an empty cache-line-aligned column with room for
    /// `entity_capacity` rows, avoiding reallocation for the common
    /// "spawn N entities then attach one component" sequence.
    pub fn with_capacity(entity_capacity: usize) -> Self {
        Self::from_parts(
            Vec::with_capacity(entity_capacity),
            Vec::with_capacity(entity_capacity),
            Vec::with_capacity(entity_capacity),
        )
    }
}

impl<T> ComponentStore<T, TightLayout> {
    /// Creates an empty unit-stride column.
    ///
    /// Named differently from [`ComponentStore::new`] on purpose: a constructor
    /// shared by both layouts would be ambiguous at an un-annotated
    /// `ComponentStore::new()` call site, because a defaulted type parameter
    /// does not participate in inference.
    pub fn new_tight() -> Self {
        Self::from_parts(Vec::new(), Vec::new(), Vec::new())
    }

    /// Creates an empty unit-stride column with room for `entity_capacity` rows.
    pub fn with_capacity_tight(entity_capacity: usize) -> Self {
        Self::from_parts(
            Vec::with_capacity(entity_capacity),
            Vec::with_capacity(entity_capacity),
            Vec::with_capacity(entity_capacity),
        )
    }
}

impl<T, L: ColumnLayout> ComponentStore<T, L> {
    fn from_parts(
        dense: Vec<L::Slot<T>>,
        sparse: Vec<Option<SparseEntry>>,
        entities: Vec<EntityId>,
    ) -> Self {
        Self {
            dense,
            sparse,
            entities,
        }
    }

    /// Reserves room for `additional_entities` more rows.
    pub fn reserve(&mut self, additional_entities: usize) {
        self.dense.reserve(additional_entities);
        self.sparse.reserve(additional_entities);
        self.entities.reserve(additional_entities);
    }

    fn sparse_position(&self, entity: EntityId) -> Option<usize> {
        let index = entity_index(entity)?;
        let entry = self.sparse.get(index).copied().flatten()?;
        if entry.generation != entity.generation {
            return None;
        }
        Some(decode_dense_position(entry.position))
    }

    fn sparse_entry_for_index(&self, index: usize) -> Option<SparseEntry> {
        self.sparse.get(index).copied().flatten()
    }

    /// Inserts or updates the component for a live entity.
    ///
    /// Returns `false` when `entity` is stale for the same index, which keeps
    /// older generations from overwriting the current row.
    #[must_use]
    pub fn insert(&mut self, entity: EntityId, component: T) -> bool {
        let idx = entity_index(entity).expect("entity index exceeds platform usize");

        if idx >= self.sparse.len() {
            self.sparse.resize(idx + 1, None);
        }

        if let Some(entry) = self.sparse_entry_for_index(idx) {
            if entity.generation < entry.generation {
                return false;
            }

            let pos = decode_dense_position(entry.position);
            debug_assert_eq!(self.entities[pos].index, entity.index);
            self.entities[pos] = entity;
            self.dense[pos] = L::from_value(component);
            self.sparse[idx] = Some(SparseEntry {
                generation: entity.generation,
                position: entry.position,
            });
            return true;
        }

        let dense_position = self.dense.len();
        self.sparse[idx] = Some(SparseEntry {
            generation: entity.generation,
            position: encode_dense_position(dense_position),
        });
        self.dense.push(L::from_value(component));
        self.entities.push(entity);
        true
    }

    /// Drops the row for `entity`, returning the component it held.
    ///
    /// `None` for an unknown index, a stale generation, or a row already
    /// removed, so a recycled index can never be cleared twice.
    pub fn remove(&mut self, entity: EntityId) -> Option<T> {
        let pos = self.sparse_position(entity)?;
        if self.entities[pos] != entity {
            return None;
        }
        Some(self.remove_at(pos))
    }

    fn remove_at(&mut self, pos: usize) -> T {
        let removed_entity = self.entities.swap_remove(pos);
        let removed = L::into_value(self.dense.swap_remove(pos));

        self.sparse[removed_entity.index as usize] = None;

        if pos < self.entities.len() {
            let swapped_entity_idx = self.entities[pos].index as usize;
            self.sparse[swapped_entity_idx] = Some(SparseEntry {
                generation: self.entities[pos].generation,
                position: encode_dense_position(pos),
            });
        }

        removed
    }

    /// Borrows the component for `entity`.
    ///
    /// `None` unless `entity` matches both the index and the generation that
    /// currently own the row.
    pub fn get(&self, entity: EntityId) -> Option<&T> {
        let pos = self.sparse_position(entity)?;
        if self.entities.get(pos).copied()? != entity {
            return None;
        }
        Some(L::project(self.dense.get(pos)?))
    }

    /// Mutably borrows the component for `entity`, under the same
    /// index-and-generation check as [`Self::get`].
    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut T> {
        let pos = self.sparse_position(entity)?;
        if self.entities.get(pos).copied()? != entity {
            return None;
        }
        Some(L::project_mut(self.dense.get_mut(pos)?))
    }

    /// Returns `true` when `entity` owns a row in this column.
    pub fn contains(&self, entity: EntityId) -> bool {
        self.get(entity).is_some()
    }

    /// Number of populated rows.
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    /// Returns `true` when the column holds no rows.
    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    /// Borrows the whole column without going through the sparse index.
    ///
    /// This is the interop seam: the slice is contiguous and starts at the
    /// alignment reported by [`Self::column_alignment`], which is exactly the
    /// shape Arrow's primitive-array bodies and aligned SIMD loads expect.
    pub fn column(&self) -> &[L::Slot<T>] {
        &self.dense
    }

    /// Mutable counterpart of [`Self::column`].
    pub fn column_mut(&mut self) -> &mut [L::Slot<T>] {
        &mut self.dense
    }

    /// Base-address alignment guaranteed for this column.
    pub fn column_alignment(&self) -> usize {
        L::alignment::<T>()
    }

    /// Distance in bytes between the start of consecutive rows.
    pub fn row_stride_bytes(&self) -> usize {
        std::mem::size_of::<L::Slot<T>>()
    }

    /// Bytes occupied by component payloads, excluding any layout padding.
    ///
    /// This is the value to hand to an Arrow buffer builder;
    /// [`Self::column_bytes`] is the value that is actually resident.
    pub fn payload_bytes(&self) -> usize {
        self.dense.len().saturating_mul(std::mem::size_of::<T>())
    }

    /// Bytes actually resident in the column, including alignment padding.
    pub fn column_bytes(&self) -> usize {
        self.dense.len().saturating_mul(self.row_stride_bytes())
    }

    /// Iterates rows in *storage* order.
    ///
    /// Fast, but `swap_remove` makes the order depend on removal history. Use
    /// [`Self::iter_ordered`] whenever the order is observable.
    pub fn iter(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.entities
            .iter()
            .copied()
            .zip(self.dense.iter().map(L::project))
    }

    /// Mutable counterpart of [`Self::iter`], in storage order.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (EntityId, &mut T)> {
        self.entities
            .iter()
            .copied()
            .zip(self.dense.iter_mut().map(L::project_mut))
    }

    /// Iterates rows in `(index, generation)` order.
    ///
    /// Unlike [`Self::iter`] this is a total, reproducible order that does not
    /// depend on insertion or removal history, at the cost of one `O(n log n)`
    /// sort per call.
    pub fn iter_ordered(&self) -> impl Iterator<Item = (EntityId, &T)> {
        let mut order: Vec<usize> = (0..self.dense.len()).collect();
        order.sort_unstable_by_key(|&position| entity_order(&self.entities[position]));
        order
            .into_iter()
            .map(|position| (self.entities[position], L::project(&self.dense[position])))
    }
}

impl<T> Default for ComponentStore<T, CacheLineLayout> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "numa")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Row geometry and NUMA hint for a component column, used by the `numa`
/// feature to reason about where a column physically lives.
#[cfg(feature = "numa")]
pub struct ComponentLocalityPlan {
    /// NUMA node the caller intends to place this column on.
    pub numa_node_hint: Option<u32>,
    /// Number of rows in the column.
    pub rows: usize,
    /// Bytes occupied by component payloads.
    pub dense_bytes: usize,
    /// Bytes actually resident, including layout padding.
    pub column_bytes: usize,
    /// Distance between consecutive rows, in bytes.
    pub row_stride_bytes: usize,
    /// Base-address alignment the column guarantees.
    pub column_alignment: usize,
    /// Whether the column is expected to stay resident on one node.
    pub cache_local: bool,
}

#[cfg(feature = "numa")]
impl ComponentLocalityPlan {
    /// Derives a locality plan from a column's live geometry.
    pub fn for_store<T, L: ColumnLayout>(
        store: &ComponentStore<T, L>,
        numa_node_hint: u32,
    ) -> Self {
        Self {
            numa_node_hint: Some(numa_node_hint),
            rows: store.len(),
            dense_bytes: store.payload_bytes(),
            column_bytes: store.column_bytes(),
            row_stride_bytes: store.row_stride_bytes(),
            column_alignment: store.column_alignment(),
            cache_local: true,
        }
    }
}

/// Type-erased view of one component column.
///
/// [`ComponentRegistry`] keeps columns as `Box<dyn ErasedColumn>` so it can
/// answer "which entities carry a `Position`" without knowing `Position`. It is
/// `Any`, so [`ComponentRegistry::store`] can additionally verify that a column
/// really holds the type the caller asked for instead of trusting its label.
pub trait ErasedColumn: Any {
    /// `TypeId` of the component this column stores.
    fn component_type(&self) -> TypeId;

    /// Stable diagnostic name of the stored component type.
    fn component_name(&self) -> &'static str;

    /// Number of populated rows.
    fn row_count(&self) -> usize;

    /// Entities owning a row, in storage order.
    fn entities(&self) -> &[EntityId];

    /// Returns `true` when `entity` owns a row with a matching generation.
    fn contains(&self, entity: EntityId) -> bool;

    /// Drops the row for `entity`, returning `true` when one existed.
    fn remove_entity(&mut self, entity: EntityId) -> bool;
}

impl<T: 'static, L: ColumnLayout> ErasedColumn for ComponentStore<T, L> {
    fn component_type(&self) -> TypeId {
        TypeId::of::<T>()
    }

    fn component_name(&self) -> &'static str {
        type_name::<T>()
    }

    fn row_count(&self) -> usize {
        self.dense.len()
    }

    fn entities(&self) -> &[EntityId] {
        &self.entities
    }

    fn contains(&self, entity: EntityId) -> bool {
        ComponentStore::<T, L>::contains(self, entity)
    }

    fn remove_entity(&mut self, entity: EntityId) -> bool {
        self.remove(entity).is_some()
    }
}

/// Registration ordinal of a component type.
///
/// Deliberately *not* a [`TypeId`]: `TypeId` values are compiler-assigned and
/// are not comparable or reproducible across builds, whereas registration order
/// is defined by the order the caller first touched the type and is therefore
/// stable for a given run configuration.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ComponentTypeId(u32);

impl ComponentTypeId {
    fn from_ordinal(ordinal: usize) -> Self {
        Self(u32::try_from(ordinal).expect("component type ordinal exceeds u32"))
    }

    /// Position of this type in [`ComponentRegistry::registered_types`].
    pub fn ordinal(self) -> usize {
        self.0 as usize
    }

    /// Compact ordinal for wire formats and diagnostics.
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl Display for ComponentTypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "component#{}", self.0)
    }
}

struct RegisteredColumn {
    component_type: TypeId,
    column: Box<dyn ErasedColumn>,
}

/// Type-erased component storage registry.
///
/// Columns live in a `Vec` in registration order and are found by linear scan
/// over `TypeId`. A registry holds on the order of tens of component types, so
/// the scan is a handful of cache-resident comparisons; in exchange, *no*
/// randomized hash seed can influence anything this type emits, which is the
/// property the replay conformance fixtures depend on.
pub struct ComponentRegistry {
    columns: Vec<RegisteredColumn>,
}

impl ComponentRegistry {
    /// Creates a registry with no component columns.
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
        }
    }

    /// Registers `T` if it is new and returns its stable ordinal.
    ///
    /// Registration is idempotent: repeating a registration returns the original
    /// ordinal instead of consuming a new one.
    pub fn register<T: 'static>(&mut self) -> ComponentTypeId {
        ComponentTypeId::from_ordinal(self.ensure_registered::<T>())
    }

    fn column_position(&self, component_type: TypeId) -> Option<usize> {
        self.columns
            .iter()
            .position(|column| column.component_type == component_type)
    }

    fn column_at<T: 'static>(&self) -> Option<usize> {
        self.column_position(TypeId::of::<T>())
    }

    fn ensure_registered<T: 'static>(&mut self) -> usize {
        if let Some(position) = self.column_at::<T>() {
            return position;
        }
        self.columns.push(RegisteredColumn {
            component_type: TypeId::of::<T>(),
            column: Box::new(ComponentStore::<T>::new()),
        });
        self.columns.len() - 1
    }

    /// Number of registered component types.
    /// Number of registered component types.
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    /// Returns `true` when no component type has been registered.
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    /// Registered component types in registration order.
    ///
    /// This is the only order the registry exposes for component types, and it
    /// is reproducible for a given sequence of registrations.
    pub fn registered_types(&self) -> impl Iterator<Item = ComponentTypeId> + '_ {
        (0..self.columns.len()).map(ComponentTypeId::from_ordinal)
    }

    /// Returns `true` when a column for `T` exists.
    pub fn is_registered<T: 'static>(&self) -> bool {
        self.column_at::<T>().is_some()
    }

    /// Borrows the cache-line-aligned column for `T`.
    pub fn store<T: 'static>(&self) -> Option<&ComponentStore<T>> {
        self.store_in::<T, CacheLineLayout>()
    }

    /// Mutably borrows the cache-line-aligned column for `T`.
    pub fn store_mut<T: 'static>(&mut self) -> Option<&mut ComponentStore<T>> {
        self.store_in_mut::<T, CacheLineLayout>()
    }

    /// Borrows the column for `T` when it was built with layout `L`.
    ///
    /// The downcast is what keeps a mislabelled column from being read as the
    /// wrong type, which is why it is checked on every access rather than once
    /// at registration.
    pub fn store_in<T: 'static, L: ColumnLayout>(&self) -> Option<&ComponentStore<T, L>> {
        let position = self.column_at::<T>()?;
        let column: &dyn Any = &*self.columns[position].column;
        column.downcast_ref::<ComponentStore<T, L>>()
    }

    /// Mutable counterpart of [`Self::store_in`].
    pub fn store_in_mut<T: 'static, L: ColumnLayout>(
        &mut self,
    ) -> Option<&mut ComponentStore<T, L>> {
        let position = self.column_at::<T>()?;
        let column: &mut dyn Any = &mut *self.columns[position].column;
        column.downcast_mut::<ComponentStore<T, L>>()
    }

    /// Inserts a component into its type column, registering the column on demand.
    ///
    /// Returns `false` when the entity is stale or the typed column is unavailable.
    #[must_use]
    pub fn insert<T: 'static>(&mut self, entity: EntityId, component: T) -> bool {
        let position = self.ensure_registered::<T>();
        let column: &mut dyn Any = &mut *self.columns[position].column;
        let Some(column) = column.downcast_mut::<ComponentStore<T>>() else {
            return false;
        };
        column.insert(entity, component)
    }

    /// Removes `entity`'s component of type `T`, registering nothing if the
    /// column does not exist yet.
    pub fn remove<T: 'static>(&mut self, entity: EntityId) -> Option<T> {
        self.store_mut::<T>()?.remove(entity)
    }

    /// Borrows `entity`'s component of type `T`.
    pub fn get<T: 'static>(&self, entity: EntityId) -> Option<&T> {
        self.store::<T>()?.get(entity)
    }

    /// Mutably borrows `entity`'s component of type `T`.
    pub fn get_mut<T: 'static>(&mut self, entity: EntityId) -> Option<&mut T> {
        self.store_mut::<T>()?.get_mut(entity)
    }

    /// Returns `true` when `entity` carries a component of type `T`.
    pub fn contains<T: 'static>(&self, entity: EntityId) -> bool {
        self.store::<T>()
            .is_some_and(|store| store.contains(entity))
    }

    /// Number of rows in the column for `T`, or `None` when it is unregistered.
    pub fn row_count<T: 'static>(&self) -> Option<usize> {
        self.store::<T>().map(|store| store.len())
    }

    /// Removes `entity` from every column, returning `true` when any row went away.
    ///
    /// Sparse sets have no liveness of their own, so a despawn that only touched
    /// the [`World`] would leave ghost rows behind for every component type.
    /// Doing this is what lets a recycled index be reused without the new
    /// occupant inheriting the previous occupant's components.
    pub fn despawn_entity(&mut self, entity: EntityId) -> bool {
        let mut removed_any = false;
        for column in &mut self.columns {
            removed_any |= column.column.remove_entity(entity);
        }
        removed_any
    }

    /// Despawns `entity` from `world` and drops its component rows.
    ///
    /// Returns `false` when the handle was already stale, in which case no
    /// component is touched.
    pub fn despawn(&mut self, world: &mut World, entity: EntityId) -> bool {
        if !world.despawn(entity) {
            return false;
        }
        self.despawn_entity(entity);
        true
    }

    /// Component types attached to `entity`, in ascending registration order.
    ///
    /// Empty for an entity that carries no components, because the registry
    /// alone cannot distinguish "no components" from "never seen".
    pub fn component_types(&self, entity: EntityId) -> Vec<ComponentTypeId> {
        self.columns
            .iter()
            .enumerate()
            .filter(|(_, column)| column.column.contains(entity))
            .map(|(ordinal, _)| ComponentTypeId::from_ordinal(ordinal))
            .collect()
    }

    /// Groups every entity carrying components by its component-type set.
    ///
    /// Groups are ordered by their sorted type lists and entities within a group
    /// are ordered by `(index, generation)`, so the whole structure is a
    /// reproducible snapshot. Group ids are scoped to the returned vector; they
    /// are not persistent handles.
    pub fn archetypes(&self) -> Vec<ArchetypeGroup> {
        let mut entities: Vec<EntityId> = self
            .columns
            .iter()
            .flat_map(|column| column.column.entities().iter().copied())
            .collect();
        entities.sort_unstable_by_key(entity_order);
        entities.dedup();

        let mut groups: BTreeMap<Vec<ComponentTypeId>, Vec<EntityId>> = BTreeMap::new();
        for entity in entities {
            let components = self.component_types(entity);
            groups.entry(components).or_default().push(entity);
        }

        groups
            .into_iter()
            .enumerate()
            .map(|(ordinal, (components, entities))| ArchetypeGroup {
                id: ArchetypeId::from_ordinal(ordinal),
                components,
                entities,
            })
            .collect()
    }

    /// Plans a deterministic iteration over the entities matching `D` and `F`.
    ///
    /// The returned plan carries a fully sorted candidate list, so no downstream
    /// executor can accidentally surface sparse-set dense order. The candidate
    /// list is drawn from the smallest matching column, which minimises the
    /// number of probes; ties are broken by registration ordinal so the choice
    /// is total rather than arbitrary.
    ///
    /// Returns [`QueryError::NoSourceColumn`] when `D` names no component,
    /// because the registry cannot enumerate entities on its own, and
    /// [`QueryError::ConflictingAccess`] when `D` would alias a type.
    ///
    /// ```
    /// use kairo_ecs_state::{ComponentRegistry, Mut, QueryAll, Ref};
    /// use kairo_ecs_types::EntityId;
    ///
    /// let mut registry = ComponentRegistry::new();
    /// let entity = EntityId::new(0, 0);
    /// registry.insert(entity, 7_i64);
    /// registry.insert(entity, 3_i32);
    ///
    /// let plan = registry.query::<(&Ref<i64>, &Mut<i32>), QueryAll>()?;
    /// assert_eq!(plan.entities(), &[entity]);
    /// # Ok::<(), kairo_ecs_state::QueryError>(())
    /// ```
    pub fn query<D: QueryData, F: QueryFilter>(&self) -> Result<QueryPlan, QueryError> {
        let accesses = D::accesses();
        if accesses.is_empty() {
            return Err(QueryError::NoSourceColumn);
        }
        self.plan_for::<F>(&accesses)
    }

    /// [`Self::query`] with the match-everything filter, for the common
    /// "everything with these components" case.
    pub fn query_all<D: QueryData>(&self) -> Result<QueryPlan, QueryError> {
        self.query::<D, QueryAll>()
    }

    fn plan_for<F: QueryFilter>(
        &self,
        accesses: &[ComponentAccess],
    ) -> Result<QueryPlan, QueryError> {
        for (offset, access) in accesses.iter().enumerate() {
            for other in &accesses[offset + 1..] {
                if access.component_type == other.component_type
                    && (access.mutable || other.mutable)
                {
                    return Err(QueryError::ConflictingAccess {
                        component: access.component_name,
                    });
                }
            }
        }

        let source = self
            .columns
            .iter()
            .enumerate()
            .filter(|(_, column)| {
                accesses
                    .iter()
                    .any(|access| access.component_type == column.component_type)
            })
            .min_by_key(|(ordinal, column)| (column.column.row_count(), *ordinal))
            .map(|(ordinal, _)| ComponentTypeId::from_ordinal(ordinal))
            .ok_or(QueryError::SourceColumnMissing {
                component: accesses[0].component_name,
            })?;

        let source_column = &self.columns[source.ordinal()].column;
        let mut entities = source_column.entities().to_vec();
        entities.sort_unstable_by_key(entity_order);
        entities.retain(|&entity| {
            F::matches(self, entity)
                && accesses
                    .iter()
                    .all(|access| self.column_matches(access, entity))
        });

        Ok(QueryPlan { entities, source })
    }

    fn column_matches(&self, access: &ComponentAccess, entity: EntityId) -> bool {
        self.columns
            .iter()
            .find(|column| column.component_type == access.component_type)
            .is_some_and(|column| column.column.contains(entity))
    }

    /// Copies the `C` column for the planned entities into a private buffer.
    ///
    /// Read-side components have to be staged because the borrow checker cannot
    /// prove that two columns of one registry are disjoint without `unsafe`,
    /// which this crate forbids. See [`ComponentRegistry::for_each_mut2`].
    fn stage<C: 'static + Clone>(&self, plan: &QueryPlan) -> Vec<(EntityId, C)> {
        let Some(store) = self.store::<C>() else {
            return Vec::new();
        };
        let mut staged = Vec::with_capacity(plan.entities.len());
        for &entity in &plan.entities {
            if let Some(value) = store.get(entity) {
                staged.push((entity, value.clone()));
            }
        }
        staged
    }

    /// Visits `(entity, &A)` for every planned entity in plan order.
    pub fn for_each1<F, A: 'static>(&self, plan: &QueryPlan, mut visit: F)
    where
        F: FnMut(EntityId, &A),
    {
        let Some(store) = self.store::<A>() else {
            return;
        };
        for &entity in &plan.entities {
            if let Some(value) = store.get(entity) {
                visit(entity, value);
            }
        }
    }

    /// Visits `(entity, &A, &B)` for every planned entity in plan order.
    pub fn for_each2<F, A, B>(&self, plan: &QueryPlan, mut visit: F)
    where
        A: 'static,
        B: 'static,
        F: FnMut(EntityId, &A, &B),
    {
        let (Some(store_a), Some(store_b)) = (self.store::<A>(), self.store::<B>()) else {
            return;
        };
        for &entity in &plan.entities {
            if let (Some(a), Some(b)) = (store_a.get(entity), store_b.get(entity)) {
                visit(entity, a, b);
            }
        }
    }

    /// Visits `(entity, &A, &B, &C)` for every planned entity in plan order.
    pub fn for_each3<F, A, B, C>(&self, plan: &QueryPlan, mut visit: F)
    where
        A: 'static,
        B: 'static,
        C: 'static,
        F: FnMut(EntityId, &A, &B, &C),
    {
        let (Some(store_a), Some(store_b), Some(store_c)) =
            (self.store::<A>(), self.store::<B>(), self.store::<C>())
        else {
            return;
        };
        for &entity in &plan.entities {
            if let (Some(a), Some(b), Some(c)) = (
                store_a.get(entity),
                store_b.get(entity),
                store_c.get(entity),
            ) {
                visit(entity, a, b, c);
            }
        }
    }

    /// Visits `(entity, &A, &B, &C, &D)` for every planned entity in plan order.
    pub fn for_each4<F, A, B, C, D>(&self, plan: &QueryPlan, mut visit: F)
    where
        A: 'static,
        B: 'static,
        C: 'static,
        D: 'static,
        F: FnMut(EntityId, &A, &B, &C, &D),
    {
        let (Some(store_a), Some(store_b), Some(store_c), Some(store_d)) = (
            self.store::<A>(),
            self.store::<B>(),
            self.store::<C>(),
            self.store::<D>(),
        ) else {
            return;
        };
        for &entity in &plan.entities {
            if let (Some(a), Some(b), Some(c), Some(d)) = (
                store_a.get(entity),
                store_b.get(entity),
                store_c.get(entity),
                store_d.get(entity),
            ) {
                visit(entity, a, b, c, d);
            }
        }
    }

    /// Visits `(entity, &mut A)` for every planned entity in plan order.
    pub fn for_each_mut1<F, A: 'static>(&mut self, plan: &QueryPlan, mut visit: F)
    where
        F: FnMut(EntityId, &mut A),
    {
        let Some(store) = self.store_mut::<A>() else {
            return;
        };
        for &entity in &plan.entities {
            if let Some(value) = store.get_mut(entity) {
                visit(entity, value);
            }
        }
    }

    /// Visits `(entity, &A, &mut B)` for every planned entity in plan order.
    ///
    /// This is the safe-Rust spelling of `Query<(&A, &mut B)>`. The `&A` handed
    /// to `visit` borrows a staged copy rather than the column itself: two
    /// columns of one registry cannot be handed out as one shared and one
    /// exclusive borrow without `unsafe`, and this crate forbids `unsafe`.
    /// Staging also makes the borrow trivially sound if `visit` re-enters the
    /// registry or panics.
    pub fn for_each_mut2<F, A, B>(&mut self, plan: &QueryPlan, mut visit: F)
    where
        A: 'static + Clone,
        B: 'static,
        F: FnMut(EntityId, &A, &mut B),
    {
        let staged = self.stage::<A>(plan);
        let Some(store) = self.store_mut::<B>() else {
            return;
        };
        for (entity, a) in staged {
            if let Some(b) = store.get_mut(entity) {
                visit(entity, &a, b);
            }
        }
    }

    /// Visits `(entity, &A, &B, &mut C)` for every planned entity in plan order.
    pub fn for_each_mut3<F, A, B, C>(&mut self, plan: &QueryPlan, mut visit: F)
    where
        A: 'static + Clone,
        B: 'static + Clone,
        C: 'static,
        F: FnMut(EntityId, &A, &B, &mut C),
    {
        let staged_a = self.stage::<A>(plan);
        let staged_b = self.stage::<B>(plan);
        let Some(store) = self.store_mut::<C>() else {
            return;
        };
        for ((entity, a), (_, b)) in staged_a.into_iter().zip(staged_b) {
            if let Some(c) = store.get_mut(entity) {
                visit(entity, &a, &b, c);
            }
        }
    }

    /// Visits `(entity, &A, &B, &C, &mut D)` for every planned entity in plan order.
    pub fn for_each_mut4<F, A, B, C, D>(&mut self, plan: &QueryPlan, mut visit: F)
    where
        A: 'static + Clone,
        B: 'static + Clone,
        C: 'static + Clone,
        D: 'static,
        F: FnMut(EntityId, &A, &B, &C, &mut D),
    {
        let staged_a = self.stage::<A>(plan);
        let staged_b = self.stage::<B>(plan);
        let staged_c = self.stage::<C>(plan);
        let Some(store) = self.store_mut::<D>() else {
            return;
        };
        for (((entity, a), (_, b)), (_, c)) in staged_a.into_iter().zip(staged_b).zip(staged_c) {
            if let Some(d) = store.get_mut(entity) {
                visit(entity, &a, &b, &c, d);
            }
        }
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// One access slot of a query descriptor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentAccess {
    component_type: TypeId,
    component_name: &'static str,
    mutable: bool,
}

impl ComponentAccess {
    fn read<C: 'static>() -> Self {
        Self {
            component_type: TypeId::of::<C>(),
            component_name: type_name::<C>(),
            mutable: false,
        }
    }

    fn write<C: 'static>() -> Self {
        Self {
            component_type: TypeId::of::<C>(),
            component_name: type_name::<C>(),
            mutable: true,
        }
    }

    /// `TypeId` of the accessed component.
    pub fn component_type(&self) -> TypeId {
        self.component_type
    }

    /// Diagnostic name of the accessed component.
    pub fn component_name(&self) -> &'static str {
        self.component_name
    }

    /// Returns `true` when the slot asks for exclusive access.
    pub fn is_mutable(&self) -> bool {
        self.mutable
    }
}

/// Read-only component access in a query descriptor; stands for `&C`.
///
/// The payload is `PhantomData<fn() -> C>` rather than `PhantomData<C>` so the
/// marker stays `Send + Sync` regardless of `C` and stays covariant in `C`.
pub struct Ref<C>(PhantomData<fn() -> C>);

/// Read-write component access in a query descriptor; stands for `&mut C`.
pub struct Mut<C>(PhantomData<fn() -> C>);

/// Describes which components a query reads and which it writes.
///
/// Implemented for [`Ref`], [`Mut`], and tuples of arity one through four, so a
/// query reads the way its call site does:
/// `Query<(&Ref<Position>, &Mut<Velocity>)>`.
pub trait QueryData {
    /// Flattens the descriptor into a left-to-right list of access slots.
    fn accesses() -> Vec<ComponentAccess>;
}

impl<C: 'static> QueryData for Ref<C> {
    fn accesses() -> Vec<ComponentAccess> {
        vec![ComponentAccess::read::<C>()]
    }
}

impl<C: 'static> QueryData for Mut<C> {
    fn accesses() -> Vec<ComponentAccess> {
        vec![ComponentAccess::write::<C>()]
    }
}

/// The empty descriptor: names no component, so it cannot enumerate entities.
///
/// Present mainly so [`QueryError::NoSourceColumn`] is reachable rather than
/// merely theoretical.
impl QueryData for () {
    fn accesses() -> Vec<ComponentAccess> {
        Vec::new()
    }
}

/// Lets a call site mirror the borrow shape its closure receives, so the plan
/// reads as `Query<(&Ref<Position>, &Mut<Velocity>)>`.
impl<C: QueryData> QueryData for &C {
    fn accesses() -> Vec<ComponentAccess> {
        C::accesses()
    }
}

macro_rules! impl_query_data_tuple {
    ($($name:ident),+) => {
        impl<$($name: QueryData),+> QueryData for ($($name,)+) {
            fn accesses() -> Vec<ComponentAccess> {
                let mut accesses = Vec::new();
                $( accesses.extend(<$name as QueryData>::accesses()); )+
                accesses
            }
        }
    };
}

impl_query_data_tuple!(A);
impl_query_data_tuple!(A, B);
impl_query_data_tuple!(A, B, C);
impl_query_data_tuple!(A, B, C, D);

/// Requires a component to be present.
pub struct With<C>(PhantomData<fn() -> C>);

/// Requires a component to be absent.
pub struct Without<C>(PhantomData<fn() -> C>);

/// Match-everything filter, used when a query has no predicates.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct QueryAll;

/// Predicate over candidate entities.
///
/// Filters are type-level, so they are selected by turbofish rather than by
/// value: a plan therefore cannot depend on when its predicate was built, and
/// two runs that mention the same filter set always produce the same plan.
/// Tuples compose with logical AND, which is what makes
/// `(With<Player>, Without<Dead>)` read naturally.
pub trait QueryFilter {
    /// Returns `true` when `entity` passes this predicate.
    fn matches(registry: &ComponentRegistry, entity: EntityId) -> bool;
}

impl QueryFilter for () {
    fn matches(_registry: &ComponentRegistry, _entity: EntityId) -> bool {
        true
    }
}

impl QueryFilter for QueryAll {
    fn matches(_registry: &ComponentRegistry, _entity: EntityId) -> bool {
        true
    }
}

impl<C: 'static> QueryFilter for With<C> {
    fn matches(registry: &ComponentRegistry, entity: EntityId) -> bool {
        registry.column_matches(&ComponentAccess::read::<C>(), entity)
    }
}

impl<C: 'static> QueryFilter for Without<C> {
    fn matches(registry: &ComponentRegistry, entity: EntityId) -> bool {
        !registry.column_matches(&ComponentAccess::read::<C>(), entity)
    }
}

macro_rules! impl_query_filter_tuple {
    ($($name:ident),+) => {
        impl<$($name: QueryFilter),+> QueryFilter for ($($name,)+) {
            fn matches(registry: &ComponentRegistry, entity: EntityId) -> bool {
                $( <$name as QueryFilter>::matches(registry, entity) && )+ true
            }
        }
    };
}

impl_query_filter_tuple!(A);
impl_query_filter_tuple!(A, B);
impl_query_filter_tuple!(A, B, C);
impl_query_filter_tuple!(A, B, C, D);

/// Failure to build a [`QueryPlan`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QueryError {
    /// The same component was requested more than once with exclusive access.
    ConflictingAccess { component: &'static str },
    /// The descriptor named no component, so there is no column to enumerate.
    NoSourceColumn,
    /// A referenced component type has no registered column.
    SourceColumnMissing { component: &'static str },
}

impl Display for QueryError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConflictingAccess { component } => {
                write!(f, "{component} is accessed more than once in one query")
            }
            Self::NoSourceColumn => {
                f.write_str("a query must name at least one component to enumerate entities")
            }
            Self::SourceColumnMissing { component } => {
                write!(f, "no registered component column for {component}")
            }
        }
    }
}

impl std::error::Error for QueryError {}

/// Immutable, reproducible candidate list for a query.
///
/// Built once and reused, which is what keeps execution order independent of any
/// later mutation and of sparse-set dense layout.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryPlan {
    entities: Vec<EntityId>,
    source: ComponentTypeId,
}

impl QueryPlan {
    /// Matching entities in `(index, generation)` order.
    pub fn entities(&self) -> &[EntityId] {
        &self.entities
    }

    /// Component type the candidate list was drawn from.
    ///
    /// Recorded because it explains the plan's cost: every remaining component
    /// was probed once per candidate.
    pub fn source(&self) -> ComponentTypeId {
        self.source
    }

    /// Number of matching entities.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns `true` when nothing matched.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

/// Snapshot-scoped identifier of an archetype group.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ArchetypeId(u32);

impl ArchetypeId {
    fn from_ordinal(ordinal: usize) -> Self {
        Self(u32::try_from(ordinal).expect("archetype ordinal exceeds u32"))
    }

    /// Position of this group in the vector returned by
    /// [`ComponentRegistry::archetypes`].
    pub fn ordinal(self) -> usize {
        self.0 as usize
    }

    /// Compact ordinal for wire formats and diagnostics.
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl Display for ArchetypeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "archetype#{}", self.0)
    }
}

/// All entities sharing one component-type set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchetypeGroup {
    id: ArchetypeId,
    components: Vec<ComponentTypeId>,
    entities: Vec<EntityId>,
}

impl ArchetypeGroup {
    /// Identifier of this group within the snapshot it came from.
    pub fn id(&self) -> ArchetypeId {
        self.id
    }

    /// Component types in this archetype, ascending by registration order.
    pub fn components(&self) -> &[ComponentTypeId] {
        &self.components
    }

    /// Members in `(index, generation)` order.
    pub fn entities(&self) -> &[EntityId] {
        &self.entities
    }

    /// Number of members.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Returns `true` when the group has no members.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }
}

#[cfg(all(test, feature = "numa"))]
mod numa_tests {
    use super::*;

    #[cfg(feature = "numa")]
    #[test]
    fn component_locality_plan_preserves_dense_order_and_node_hint() {
        let mut store = ComponentStore::<i32>::with_capacity(4);
        let first = EntityId::new(0, 0);
        let second = EntityId::new(1, 0);
        assert!(store.insert(first, 10_i32));
        assert!(store.insert(second, 20_i32));

        let plan = ComponentLocalityPlan::for_store(&store, 2);

        assert_eq!(plan.numa_node_hint, Some(2));
        assert_eq!(plan.rows, 2);
        assert_eq!(plan.dense_bytes, 2 * std::mem::size_of::<i32>());
        assert!(plan.cache_local);
    }

    #[test]
    fn component_locality_plan_reports_cache_line_geometry() {
        let mut store = ComponentStore::<f64>::with_capacity(2);
        assert!(store.insert(EntityId::new(0, 0), 1.0));
        assert!(store.insert(EntityId::new(1, 0), 2.0));

        let plan = ComponentLocalityPlan::for_store(&store, 0);

        assert_eq!(plan.row_stride_bytes, CACHE_LINE_BYTES);
        assert_eq!(plan.column_alignment, CACHE_LINE_BYTES);
        assert_eq!(plan.dense_bytes, 2 * std::mem::size_of::<f64>());
        assert_eq!(plan.column_bytes, 2 * CACHE_LINE_BYTES);
    }
}

#[cfg(test)]
mod registry_insert_tests {
    use super::*;

    /// A column deliberately filed under the wrong `TypeId`.
    ///
    /// Mirrors a corrupted or hand-built registry: `ComponentRegistry::insert`
    /// must refuse to write through a column whose contents do not match its
    /// label instead of reinterpreting memory.
    struct MislabeledColumn {
        inner: ComponentStore<u64>,
    }

    impl ErasedColumn for MislabeledColumn {
        fn component_type(&self) -> TypeId {
            TypeId::of::<u32>()
        }

        fn component_name(&self) -> &'static str {
            type_name::<u32>()
        }

        fn row_count(&self) -> usize {
            self.inner.len()
        }

        fn entities(&self) -> &[EntityId] {
            &self.inner.entities
        }

        fn contains(&self, entity: EntityId) -> bool {
            self.inner.contains(entity)
        }

        fn remove_entity(&mut self, entity: EntityId) -> bool {
            self.inner.remove(entity).is_some()
        }
    }

    #[test]
    fn insert_registers_missing_store_and_keeps_normal_behavior() {
        let mut registry = ComponentRegistry::new();
        let mut world = World::new();
        let entity = world.spawn();

        assert!(registry.insert(entity, 42_u32));
        assert_eq!(registry.get::<u32>(entity), Some(&42));
    }

    #[test]
    fn insert_returns_false_for_an_inconsistent_typed_store() {
        let mut registry = ComponentRegistry::new();
        registry.columns.push(RegisteredColumn {
            component_type: TypeId::of::<u32>(),
            column: Box::new(MislabeledColumn {
                inner: ComponentStore::<u64>::new(),
            }),
        });
        let mut world = World::new();
        let entity = world.spawn();

        assert!(!registry.insert(entity, 42_u32));
        assert!(registry.store::<u32>().is_none());
    }

    #[test]
    fn register_assigns_stable_ordinals_in_first_touch_order() {
        let mut registry = ComponentRegistry::new();

        let position = registry.register::<u64>();
        let velocity = registry.register::<i32>();

        assert_eq!(position.as_u32(), 0);
        assert_eq!(velocity.as_u32(), 1);
        assert_eq!(position.ordinal(), 0);
        assert_eq!(
            registry.registered_types().collect::<Vec<_>>(),
            vec![position, velocity]
        );
        // Re-registering is idempotent and must not consume an ordinal.
        assert_eq!(registry.register::<u64>(), position);
        assert_eq!(registry.len(), 2);
        assert!(!registry.is_empty());
        assert_eq!(position.to_string(), "component#0");
    }

    #[test]
    fn store_in_rejects_a_layout_the_column_was_not_built_with() {
        let mut registry = ComponentRegistry::new();
        assert!(registry.insert(EntityId::new(0, 0), 1_u32));

        assert!(registry.store::<u32>().is_some());
        assert!(registry.store_in::<u32, TightLayout>().is_none());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
    struct Payload(u64);

    #[test]
    fn test_world_despawn() {
        let mut world = World::new();

        // Spawn a couple of entities
        let entity1 = world.spawn();
        let entity2 = world.spawn();
        let entity3 = world.spawn();

        assert_eq!(world.len(), 3);
        assert!(world.is_alive(entity1));
        assert!(world.is_alive(entity2));
        assert!(world.is_alive(entity3));

        // Despawn entity2
        let despawned = world.despawn(entity2);
        assert!(despawned);

        // Verify entity2 is no longer alive
        assert!(!world.is_alive(entity2));
        assert_eq!(world.len(), 2);

        // Despawning again should fail
        assert!(!world.despawn(entity2));

        // Other entities should still be alive
        assert!(world.is_alive(entity1));
        assert!(world.is_alive(entity3));

        // Despawn an entity with an invalid index
        let invalid_entity = EntityId::new(999, 0);
        assert!(!world.despawn(invalid_entity));

        // Despawn an entity with a stale generation
        let stale_entity = EntityId::new(entity1.index, entity1.generation.wrapping_add(1));
        assert!(!world.despawn(stale_entity));
    }

    #[test]
    fn test_world_with_capacity() {
        let capacity = 42;
        let world = World::with_capacity(capacity);

        assert!(world.slots.capacity() >= capacity);
        assert!(world.free_indices.capacity() >= capacity);
        assert!(world.live_entities.capacity() >= capacity);
        assert!(world.live_positions.capacity() >= capacity);
    }

    #[test]
    fn component_store_with_capacity_reserves_all_storage_vectors() {
        let capacity = 42;
        let store = ComponentStore::<u32>::with_capacity(capacity);

        assert!(store.dense.capacity() >= capacity);
        assert!(store.sparse.capacity() >= capacity);
        assert!(store.entities.capacity() >= capacity);
    }

    #[test]
    fn world_reserve_grows_all_storage_vectors_from_empty() {
        let additional = 10;
        let mut world = World::new();
        world.reserve(additional);

        assert!(world.slots.capacity() >= additional);
        assert!(world.free_indices.capacity() >= additional);
        assert!(world.live_entities.capacity() >= additional);
        assert!(world.live_positions.capacity() >= additional);
    }

    #[test]
    fn component_store_reserve_grows_all_storage_vectors_from_empty() {
        let additional = 10;
        let mut store = ComponentStore::<u32>::new();
        store.reserve(additional);

        assert!(store.dense.capacity() >= additional);
        assert!(store.sparse.capacity() >= additional);
        assert!(store.entities.capacity() >= additional);
    }

    #[test]
    fn world_reserve_adds_capacity_after_entities_exist() {
        let mut world = World::new();
        world.spawn();
        world.spawn();
        let additional = 10;

        let slots_required = world.slots.len() + additional;
        let free_indices_required = world.free_indices.len() + additional;
        let live_entities_required = world.live_entities.len() + additional;
        let live_positions_required = world.live_positions.len() + additional;

        world.reserve(additional);

        assert!(world.slots.capacity() >= slots_required);
        assert!(world.free_indices.capacity() >= free_indices_required);
        assert!(world.live_entities.capacity() >= live_entities_required);
        assert!(world.live_positions.capacity() >= live_positions_required);
    }

    #[test]
    fn component_store_reserve_adds_capacity_after_components_exist() {
        let mut world = World::new();
        let entities = [world.spawn(), world.spawn(), world.spawn()];
        let mut store = ComponentStore::<u32>::new();
        for (value, entity) in entities.into_iter().enumerate() {
            assert!(store.insert(entity, value as u32));
        }
        let additional = 10;

        let dense_required = store.dense.len() + additional;
        let sparse_required = store.sparse.len() + additional;
        let entities_required = store.entities.len() + additional;

        store.reserve(additional);

        assert!(store.dense.capacity() >= dense_required);
        assert!(store.sparse.capacity() >= sparse_required);
        assert!(store.entities.capacity() >= entities_required);
    }

    #[test]
    fn cache_line_layout_aligns_the_column_base_address() {
        let mut store = ComponentStore::<Payload>::new();
        for index in 0..64 {
            assert!(store.insert(EntityId::new(index, 0), Payload(index)));
        }

        let base = store.column().as_ptr() as usize;

        assert_eq!(base % CACHE_LINE_BYTES, 0);
        assert_eq!(std::mem::align_of_val(&store.column()[0]), CACHE_LINE_BYTES);
        assert_eq!(store.column_alignment(), CACHE_LINE_BYTES);
    }

    #[test]
    fn cache_line_layout_pads_rows_to_a_whole_number_of_lines() {
        let store = ComponentStore::<Payload>::with_capacity(4);

        assert_eq!(store.row_stride_bytes(), CACHE_LINE_BYTES);
        assert_eq!(store.row_stride_bytes() % CACHE_LINE_BYTES, 0);
        assert_eq!(store.payload_bytes(), 0);
        assert_eq!(store.column_bytes(), 0);
    }

    #[test]
    fn cache_line_layout_grows_resident_bytes_by_the_padding() {
        let mut store = ComponentStore::<Payload>::new();
        for index in 0..5 {
            assert!(store.insert(EntityId::new(index, 0), Payload(index)));
        }

        assert_eq!(store.len(), 5);
        assert_eq!(store.payload_bytes(), 5 * std::mem::size_of::<Payload>());
        assert_eq!(store.column_bytes(), 5 * CACHE_LINE_BYTES);
        assert!(store.column_bytes() > store.payload_bytes());
    }

    #[test]
    fn tight_layout_keeps_rows_unit_stride_for_arrow_style_buffers() {
        let mut store = ComponentStore::<Payload, TightLayout>::new_tight();
        for index in 0..5 {
            assert!(store.insert(EntityId::new(index, 0), Payload(index)));
        }

        assert_eq!(store.row_stride_bytes(), std::mem::size_of::<Payload>());
        assert_eq!(store.column_alignment(), std::mem::align_of::<Payload>());
        assert_eq!(store.column_bytes(), store.payload_bytes());
    }

    #[test]
    fn large_components_still_get_a_line_aligned_base_and_whole_line_rows() {
        let mut store = ComponentStore::<[f64; 32]>::new();
        assert!(store.insert(EntityId::new(0, 0), [0.0; 32]));

        assert_eq!(store.column().as_ptr() as usize % CACHE_LINE_BYTES, 0);
        assert_eq!(store.row_stride_bytes(), 32 * 8);
        assert_eq!(store.row_stride_bytes() % CACHE_LINE_BYTES, 0);
        assert_eq!(store.column_alignment(), CACHE_LINE_BYTES);
    }

    #[test]
    fn both_layouts_agree_on_reads_writes_and_iteration() {
        let mut aligned = ComponentStore::<Payload>::new();
        let mut tight = ComponentStore::<Payload, TightLayout>::new_tight();
        for index in [3_u64, 1, 2] {
            let payload = Payload(index);
            assert!(aligned.insert(EntityId::new(index, 0), payload));
            assert!(tight.insert(EntityId::new(index, 0), payload));
        }

        assert_eq!(
            aligned.iter_ordered().collect::<Vec<_>>(),
            tight.iter_ordered().collect::<Vec<_>>()
        );

        aligned
            .get_mut(EntityId::new(2, 0))
            .expect("row 2 present")
            .0 = 42;
        tight.get_mut(EntityId::new(2, 0)).expect("row 2 present").0 = 42;

        assert_eq!(
            aligned.get(EntityId::new(2, 0)),
            tight.get(EntityId::new(2, 0))
        );
        assert_eq!(aligned.remove(EntityId::new(2, 0)).map(|v| v.0), Some(42));
        assert_eq!(tight.remove(EntityId::new(2, 0)).map(|v| v.0), Some(42));
    }

    #[test]
    fn iter_ordered_is_independent_of_insertion_and_removal_history() {
        let mut forward = ComponentStore::<Payload>::new();
        let mut shuffled = ComponentStore::<Payload>::new();

        for index in 0..16_u64 {
            assert!(forward.insert(EntityId::new(index, 0), Payload(index)));
        }
        for index in (0..16_u64).rev() {
            assert!(shuffled.insert(EntityId::new(index, 0), Payload(index)));
        }

        // Storage order differs, deterministic order does not.
        let forward_storage: Vec<EntityId> = forward.iter().map(|(e, _)| e).collect();
        let shuffled_storage: Vec<EntityId> = shuffled.iter().map(|(e, _)| e).collect();
        assert_ne!(forward_storage, shuffled_storage);

        assert_eq!(
            forward.iter_ordered().collect::<Vec<_>>(),
            shuffled.iter_ordered().collect::<Vec<_>>()
        );
    }
}

#[cfg(test)]
mod query_determinism_tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    struct Position {
        x: i64,
        y: i64,
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    struct Velocity {
        dx: i64,
        dy: i64,
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    struct Health(i32);

    #[derive(Clone, Copy, Debug, Default, PartialEq)]
    struct Frozen;

    fn entity(index: u64, generation: u32) -> EntityId {
        EntityId { index, generation }
    }

    type Row = (EntityId, i64, i64, i64);

    fn rows(count: u64) -> Vec<Row> {
        (0..count)
            .map(|index| (entity(index, 0), index as i64, index as i64, index as i64))
            .collect()
    }

    /// Builds one logical world, inserting components in either forward or
    /// reverse order so that only a storage-order leak could differ.
    fn build_registry(rows: &[Row], reverse: bool) -> ComponentRegistry {
        let mut registry = ComponentRegistry::new();
        for &(target, x, vx, health) in rows {
            if reverse {
                assert!(registry.insert(target, Health(health as i32)));
                assert!(registry.insert(target, Velocity { dx: vx, dy: 0 }));
                assert!(registry.insert(target, Position { x, y: 0 }));
            } else {
                assert!(registry.insert(target, Position { x, y: 0 }));
                assert!(registry.insert(target, Velocity { dx: vx, dy: 0 }));
                assert!(registry.insert(target, Health(health as i32)));
            }
        }
        registry
    }

    fn observe(registry: &ComponentRegistry) -> Vec<(EntityId, i64, i64)> {
        let plan = registry
            .query::<(&Ref<Position>, &Mut<Velocity>), ()>()
            .expect("plan builds");
        let mut observed = Vec::new();
        registry.for_each2(&plan, |target, position: &Position, velocity: &Velocity| {
            observed.push((target, position.x, velocity.dx));
        });
        observed
    }

    #[test]
    fn query_order_is_independent_of_component_insertion_order() {
        let rows = rows(32);
        let forward = build_registry(&rows, false);
        let reversed = build_registry(&rows, true);

        assert_eq!(observe(&forward), observe(&reversed));
    }

    #[test]
    fn query_order_follows_index_order_not_registration_order() {
        let rows = vec![
            (entity(2, 0), 2, -2, 20),
            (entity(0, 0), 0, 0, 0),
            (entity(1, 0), 1, -1, 10),
        ];

        // Velocity is registered before Position, so registration order is the
        // reverse of index order; query order must still be index order.
        let registry = build_registry(&rows, true);

        assert_eq!(
            observe(&registry),
            vec![
                (entity(0, 0), 0, 0),
                (entity(1, 0), 1, -1),
                (entity(2, 0), 2, -2),
            ]
        );
    }

    #[test]
    fn repeated_planning_of_an_unchanged_registry_is_identical() {
        let registry = build_registry(&rows(8), false);

        let first = registry.query_all::<(&Ref<Position>, &Ref<Velocity>)>();
        let second = registry.query_all::<(&Ref<Position>, &Ref<Velocity>)>();

        assert_eq!(first, second);
        assert_eq!(observe(&registry), observe(&registry));
    }

    #[test]
    fn plan_survives_removal_history_that_shuffles_dense_rows() {
        let mut registry = build_registry(&rows(16), false);

        // Removing rows forces `swap_remove` to move the tail row forward, which
        // destroys the dense-order invariant the plan was built against.
        for index in [0_u64, 5, 9] {
            assert!(registry.remove::<Position>(entity(index, 0)).is_some());
        }

        let plan = registry
            .query::<(&Ref<Position>, &Ref<Velocity>), ()>()
            .expect("plan builds");

        let survivors: Vec<EntityId> = (0..16_u64)
            .filter(|index| ![0, 5, 9].contains(index))
            .map(|index| entity(index, 0))
            .collect();
        assert_eq!(plan.entities(), survivors.as_slice());

        let mut visited = Vec::new();
        registry.for_each2(&plan, |target, position: &Position, _: &Velocity| {
            visited.push((target, position.x));
        });
        assert_eq!(
            visited,
            survivors
                .iter()
                .map(|&target| (target, target.index as i64))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn generation_rollover_orders_by_index_and_honours_the_live_generation() {
        let mut world = World::new();
        let mut registry = ComponentRegistry::new();

        let first = world.spawn();
        let reused = world.spawn();
        let last = world.spawn();
        for target in [first, reused, last] {
            assert!(registry.insert(target, Position { x: 1, y: 1 }));
            assert!(registry.insert(target, Health(1)));
        }

        // Drop `reused` from both the world and the registry, then take the slot
        // back. The index is reused, the generation is not.
        assert!(registry.despawn(&mut world, reused));
        assert!(!registry.contains::<Position>(reused));
        assert_eq!(
            registry
                .query_all::<Ref<Position>>()
                .expect("plan builds")
                .entities(),
            &[first, last]
        );

        let reborn = world.spawn();
        assert_eq!(reborn.index, reused.index);
        assert_eq!(reborn.generation, reused.generation.wrapping_add(1));
        assert!(registry.insert(reborn, Position { x: 7, y: 7 }));

        let plan = registry.query_all::<Ref<Position>>().expect("plan builds");

        // Index order is the primary key, so the reborn entity sits exactly where
        // its predecessor sat, and it must carry the new generation.
        assert_eq!(plan.entities(), &[first, reborn, last]);
        assert_eq!(plan.entities()[1].generation, reborn.generation);
        assert_eq!(plan.len(), 3);
        assert!(!plan.is_empty());

        // The stale handle must never reappear through any accessor.
        assert_eq!(registry.get::<Position>(reused), None);
        assert!(!registry.contains::<Health>(reused));
        assert!(registry.component_types(reused).is_empty());
        assert_eq!(registry.component_types(reborn).len(), 1);
        assert_eq!(registry.query_all::<Ref<Health>>().expect("plan").len(), 2);

        let mut observed = Vec::new();
        registry.for_each1(&plan, |target, position: &Position| {
            observed.push((target, position.x));
        });
        assert_eq!(observed, vec![(first, 1), (reborn, 7), (last, 1)]);
    }

    #[test]
    fn despawning_through_the_registry_drops_every_column() {
        let mut world = World::new();
        let mut registry = ComponentRegistry::new();

        let target = world.spawn();
        assert!(registry.insert(target, Position { x: 1, y: 1 }));
        assert!(registry.insert(target, Velocity { dx: 1, dy: 1 }));
        assert!(registry.insert(target, Health(5)));

        assert!(registry.despawn(&mut world, target));
        assert_eq!(registry.row_count::<Position>(), Some(0));
        assert_eq!(registry.row_count::<Velocity>(), Some(0));
        assert_eq!(registry.row_count::<Health>(), Some(0));
        assert!(registry.archetypes().is_empty());

        // A stale despawn must not touch anything.
        assert!(!registry.despawn(&mut world, target));
        assert!(!registry.despawn_entity(target));
    }

    #[test]
    fn with_and_without_filters_are_applied_before_the_executor_runs() {
        let mut registry = build_registry(&rows(6), true);
        let plan = registry
            .query::<Ref<Position>, (With<Health>, Without<Frozen>)>()
            .expect("plan builds");
        assert_eq!(plan.entities().len(), 6);

        assert!(registry.insert(entity(2, 0), Frozen));
        assert!(registry.remove::<Position>(entity(4, 0)).is_some());

        let filtered = registry
            .query::<Ref<Position>, (With<Health>, Without<Frozen>)>()
            .expect("plan builds");

        assert_eq!(
            filtered.entities(),
            &[entity(0, 0), entity(1, 0), entity(3, 0), entity(5, 0)]
        );

        // `()` and `QueryAll` are the same match-everything filter.
        let bare = registry.query::<Ref<Position>, ()>().expect("plan builds");
        let all = registry
            .query::<Ref<Position>, QueryAll>()
            .expect("plan builds");
        assert_eq!(bare.entities(), all.entities());
        assert_eq!(bare.source(), all.source());
    }

    #[test]
    fn queries_reject_conflicting_access_and_descriptors_without_a_source() {
        let mut registry = ComponentRegistry::new();
        assert!(registry.insert(entity(0, 0), Position { x: 0, y: 0 }));

        let conflict = registry
            .query::<(&Ref<Position>, &Mut<Position>), ()>()
            .expect_err("double exclusive access must be rejected");
        assert!(matches!(conflict, QueryError::ConflictingAccess { .. }));
        assert!(conflict.to_string().contains("more than once"));

        let no_source = registry
            .query_all::<()>()
            .expect_err("descriptor must name a component");
        assert_eq!(no_source, QueryError::NoSourceColumn);
        assert!(no_source.to_string().contains("at least one component"));

        let missing = ComponentRegistry::new()
            .query_all::<Ref<Position>>()
            .expect_err("unregistered column");
        assert!(matches!(missing, QueryError::SourceColumnMissing { .. }));
        assert!(missing
            .to_string()
            .contains("no registered component column"));

        // The same type read twice is not a conflict.
        assert!(registry
            .query_all::<(&Ref<Position>, &Ref<Position>)>()
            .is_ok());
    }

    #[test]
    fn the_source_column_is_the_smallest_match_and_ties_break_on_ordinal() {
        let mut registry = ComponentRegistry::new();
        for index in 0..8_u64 {
            assert!(registry.insert(entity(index, 0), Position { x: 0, y: 0 }));
        }
        // Velocity has fewer rows, so the plan must be drawn from it even though
        // Position was registered first.
        for index in 4..6_u64 {
            assert!(registry.insert(entity(index, 0), Velocity { dx: 0, dy: 0 }));
        }

        let plan = registry
            .query::<(&Ref<Position>, &Ref<Velocity>), ()>()
            .expect("plan builds");
        assert_eq!(plan.source().as_u32(), 1);
        assert_eq!(plan.entities(), &[entity(4, 0), entity(5, 0)]);

        // Equal row counts: the lowest registration ordinal wins, so the choice
        // is total rather than dependent on scan order.
        for index in [0_u64, 1, 2, 3, 6, 7] {
            assert!(registry.insert(entity(index, 0), Velocity { dx: 0, dy: 0 }));
        }
        assert_eq!(registry.row_count::<Velocity>(), Some(8));
        let tied = registry
            .query::<(&Ref<Position>, &Ref<Velocity>), ()>()
            .expect("plan builds");
        assert_eq!(tied.source().as_u32(), 0);
    }

    #[test]
    fn executors_visit_in_plan_order_and_mutation_is_visible() {
        let mut registry = build_registry(&rows(4), false);
        let plan = registry
            .query::<(&Ref<Position>, &Mut<Velocity>), ()>()
            .expect("plan builds");

        let mut order = Vec::new();
        registry.for_each_mut2(
            &plan,
            |target, position: &Position, velocity: &mut Velocity| {
                order.push(target);
                velocity.dx += position.x;
            },
        );

        assert_eq!(
            order,
            vec![entity(0, 0), entity(1, 0), entity(2, 0), entity(3, 0)]
        );
        for index in 0..4_u64 {
            assert_eq!(
                registry.get::<Velocity>(entity(index, 0)).map(|v| v.dx),
                Some(index as i64 * 2)
            );
        }
    }

    #[test]
    fn mutable_executors_cover_every_documented_arity() {
        let mut registry = ComponentRegistry::new();
        let target = entity(0, 0);
        assert!(registry.insert(target, Position { x: 1, y: 2 }));
        assert!(registry.insert(target, Velocity { dx: 3, dy: 4 }));
        assert!(registry.insert(target, Health(5)));
        assert!(registry.insert(target, Frozen));

        let position_plan = registry.query_all::<Mut<Position>>().expect("plan");
        registry.for_each_mut1(&position_plan, |_, position: &mut Position| {
            position.x += 10;
        });
        assert_eq!(registry.get::<Position>(target).map(|p| p.x), Some(11));

        let velocity_plan = registry
            .query_all::<(&Ref<Position>, &Ref<Velocity>, &Mut<Health>)>()
            .expect("plan");
        registry.for_each_mut3(
            &velocity_plan,
            |_, position: &Position, velocity: &Velocity, health: &mut Health| {
                health.0 += 1 + (position.x + velocity.dx) as i32;
            },
        );
        assert_eq!(registry.get::<Velocity>(target).map(|v| v.dx), Some(3));
        assert_eq!(registry.get::<Health>(target).map(|h| h.0), Some(20));

        let frozen_plan = registry
            .query_all::<(&Ref<Position>, &Ref<Velocity>, &Ref<Health>, &Mut<Frozen>)>()
            .expect("plan");
        let mut health_count = 0_i32;
        registry.for_each_mut4(
            &frozen_plan,
            |_, _: &Position, _: &Velocity, _: &Health, frozen: &mut Frozen| {
                // `Frozen` is a marker with no payload, so the visit only proves
                // that the fourth mutable slot was reachable in plan order.
                let _ = frozen;
                health_count += 10;
            },
        );
        assert_eq!(health_count, 10);
        assert_eq!(registry.get::<Health>(target).map(|h| h.0), Some(20));
    }

    #[test]
    fn read_only_executors_cover_every_documented_arity() {
        let mut registry = ComponentRegistry::new();
        let target = entity(0, 0);
        assert!(registry.insert(target, Position { x: 1, y: 2 }));
        assert!(registry.insert(target, Velocity { dx: 3, dy: 4 }));
        assert!(registry.insert(target, Health(5)));
        assert!(registry.insert(target, 9_i32));
        let mut seen = Vec::new();
        let mut health = 0_i32;

        registry.for_each1(
            &registry.query_all::<Ref<Position>>().expect("plan"),
            |e, p: &Position| seen.push((e, p.x)),
        );
        registry.for_each2(
            &registry
                .query_all::<(&Ref<Position>, &Ref<Velocity>)>()
                .expect("plan"),
            |e, p: &Position, _v: &Velocity| seen.push((e, p.x)),
        );
        registry.for_each3(
            &registry
                .query_all::<(&Ref<Position>, &Ref<Velocity>, &Ref<Health>)>()
                .expect("plan"),
            |e, p: &Position, _v: &Velocity, h: &Health| {
                health = h.0;
                seen.push((e, p.x));
            },
        );
        registry.for_each4(
            &registry
                .query_all::<(&Ref<Position>, &Ref<Velocity>, &Ref<Health>, &Ref<i32>)>()
                .expect("plan"),
            |e, p: &Position, _v: &Velocity, _h: &Health, _: &i32| seen.push((e, p.x)),
        );

        assert_eq!(seen, vec![(target, 1); 4]);
        assert_eq!(health, 5);
    }

    #[test]
    fn executors_are_no_ops_for_columns_they_cannot_resolve() {
        let mut registry = ComponentRegistry::new();
        assert!(registry.insert(entity(0, 0), Position { x: 1, y: 1 }));
        assert!(registry.insert(entity(1, 0), Position { x: 2, y: 2 }));

        let single = registry.query_all::<Ref<Position>>().expect("plan builds");
        let mut calls = 0_u32;

        // `Velocity` is unregistered, so every one of these must visit nothing
        // rather than misbehave.
        registry.for_each2(&single, |_: EntityId, _: &Position, _: &Velocity| {
            calls += 1;
        });
        registry.for_each3(
            &single,
            |_: EntityId, _: &Position, _: &Velocity, _: &Health| calls += 1,
        );
        registry.for_each4(
            &single,
            |_: EntityId, _: &Position, _: &Velocity, _: &Health, _: &Frozen| calls += 1,
        );
        registry.for_each_mut1(&single, |_: EntityId, _: &mut Velocity| calls += 1);
        registry.for_each_mut2(&single, |_: EntityId, _: &Position, _: &mut Velocity| {
            calls += 1
        });
        registry.for_each_mut3(
            &single,
            |_: EntityId, _: &Position, _: &Velocity, _: &mut Health| calls += 1,
        );
        registry.for_each_mut4(
            &single,
            |_: EntityId, _: &Position, _: &Velocity, _: &Health, _: &mut Frozen| {
                calls += 1;
            },
        );

        assert_eq!(calls, 0);
        assert_eq!(registry.get::<Position>(entity(0, 0)).map(|p| p.x), Some(1));
    }

    #[test]
    fn an_empty_plan_visits_nothing() {
        let mut registry = ComponentRegistry::new();
        assert!(registry.insert(entity(0, 0), Position { x: 1, y: 1 }));
        let plan = registry
            .query::<Ref<Position>, With<Health>>()
            .expect("plan builds");
        assert!(plan.is_empty());

        let mut calls = 0_u32;
        registry.for_each_mut1(&plan, |_: EntityId, _: &mut Position| calls += 1);
        assert_eq!(calls, 0);
    }

    #[test]
    fn archetypes_are_ordered_by_type_list_and_by_entity_key() {
        let mut registry = ComponentRegistry::new();
        assert!(registry.insert(entity(0, 0), Position { x: 0, y: 0 }));
        assert!(registry.insert(entity(1, 0), Position { x: 0, y: 0 }));
        assert!(registry.insert(entity(1, 0), Health(1)));
        assert!(registry.insert(entity(2, 0), Health(2)));

        let groups = registry.archetypes();
        assert_eq!(groups.len(), 3);

        let rendered: Vec<(Vec<u32>, Vec<EntityId>)> = groups
            .iter()
            .map(|group| {
                (
                    group
                        .components()
                        .iter()
                        .map(|id| id.as_u32())
                        .collect::<Vec<_>>(),
                    group.entities().to_vec(),
                )
            })
            .collect();

        assert_eq!(
            rendered,
            vec![
                (vec![0], vec![entity(0, 0)]),
                (vec![0, 1], vec![entity(1, 0)]),
                (vec![1], vec![entity(2, 0)]),
            ]
        );
        assert_eq!(groups[0].id().as_u32(), 0);
        assert_eq!(groups[0].len(), 1);
        assert!(!groups[0].is_empty());
        assert_eq!(groups[0].id().to_string(), "archetype#0");
        assert_eq!(registry.archetypes(), groups);
    }

    #[test]
    fn archetypes_are_independent_of_registration_and_insertion_order() {
        fn seed(registry: &mut ComponentRegistry, indices: &[u64], reverse: bool) {
            for &index in indices {
                let target = entity(index, 0);
                if reverse {
                    if index % 2 == 0 {
                        assert!(registry.insert(target, Health(index as i32)));
                    }
                    if index % 2 == 1 {
                        assert!(registry.insert(
                            target,
                            Velocity {
                                dx: index as i64,
                                dy: 0,
                            },
                        ));
                    }
                    assert!(registry.insert(target, Position { x: 0, y: 0 }));
                } else {
                    assert!(registry.insert(target, Position { x: 0, y: 0 }));
                    if index % 2 == 0 {
                        assert!(registry.insert(target, Health(index as i32)));
                    }
                    if index % 2 == 1 {
                        assert!(registry.insert(
                            target,
                            Velocity {
                                dx: index as i64,
                                dy: 0,
                            },
                        ));
                    }
                }
            }
        }

        let mut forward = ComponentRegistry::new();
        seed(&mut forward, &[0, 1, 2, 3], false);
        let mut reversed = ComponentRegistry::new();
        seed(&mut reversed, &[3, 2, 1, 0], true);

        // The two registries registered the same three types in opposite orders,
        // so the ordinal labels differ...
        assert_eq!(forward.component_types(entity(0, 0)).len(), 2);
        assert_eq!(reversed.component_types(entity(0, 0)).len(), 2);
        assert_ne!(
            forward.component_types(entity(0, 0)),
            reversed.component_types(entity(0, 0))
        );

        // ...but the partitioning of entities into archetypes must agree, once the
        // group order that follows from the differing ordinals is normalised out.
        let membership = |registry: &ComponentRegistry| -> Vec<Vec<EntityId>> {
            let mut groups: Vec<Vec<EntityId>> = registry
                .archetypes()
                .into_iter()
                .map(|group| group.entities().to_vec())
                .collect();
            groups.sort();
            groups
        };
        assert_eq!(membership(&forward), membership(&reversed));
        assert_eq!(
            membership(&forward),
            vec![
                vec![entity(0, 0), entity(2, 0)],
                vec![entity(1, 0), entity(3, 0)],
            ]
        );
    }

    #[test]
    fn component_types_are_reported_in_registration_order() {
        let mut registry = ComponentRegistry::new();
        assert!(registry.insert(entity(0, 0), Health(1)));
        assert!(registry.insert(entity(0, 0), Position { x: 0, y: 0 }));

        let health = registry.register::<Health>();
        let position = registry.register::<Position>();

        assert_eq!(
            registry.component_types(entity(0, 0)),
            vec![health, position]
        );
        assert!(registry.component_types(entity(9, 0)).is_empty());
    }

    #[test]
    fn registry_emits_no_hash_derived_ordering() {
        let mut registry = ComponentRegistry::new();
        for index in 0..64_u64 {
            assert!(registry.insert(
                entity(index, 0),
                Position {
                    x: index as i64,
                    y: 0,
                },
            ));
            assert!(registry.insert(entity(index, 0), Health(index as i32)));
        }

        // Ordinals are a pure append of first-touch order.
        let ordinals: Vec<u32> = registry.registered_types().map(|id| id.as_u32()).collect();
        assert_eq!(ordinals, vec![0, 1]);
        assert_eq!(
            registry.component_types(entity(5, 0)),
            vec![
                ComponentTypeId::from_ordinal(0),
                ComponentTypeId::from_ordinal(1)
            ]
        );

        assert!(registry.is_registered::<Position>());
        assert!(registry.is_registered::<Health>());
        assert!(!registry.is_registered::<Velocity>());
        assert_eq!(registry.row_count::<Position>(), Some(64));
        assert_eq!(registry.row_count::<Velocity>(), None);

        let plan = registry.query_all::<Ref<Position>>().expect("plan builds");
        let mut described = Vec::new();
        registry.for_each1(&plan, |target, _: &Position| described.push(target));
        assert_eq!(
            described,
            (0..64_u64)
                .map(|index| entity(index, 0))
                .collect::<Vec<_>>()
        );
    }
}
