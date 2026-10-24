#![forbid(unsafe_code)]

use kairo_ecs_types::EntityId;
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use std::collections::HashMap;

/// Errors produced during continuous spatial indexing operations.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum SpatialIndexError {
    #[error("non-finite coordinate supplied: x={x}, y={y}")]
    NonFiniteCoordinate { x: f64, y: f64 },

    #[error("non-finite query radius: {0}")]
    NonFiniteRadius(f64),

    #[error("negative query radius: {0}")]
    NegativeRadius(f64),

    #[error("invalid bounding box: min_x ({min_x}) > max_x ({max_x}) or min_y ({min_y}) > max_y ({max_y})")]
    InvalidBoundingBox {
        min_x: f64,
        max_x: f64,
        min_y: f64,
        max_y: f64,
    },
}

/// An indexed entity located at a 2D Cartesian coordinate `[x, y]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpatialEntity2D {
    pub entity: EntityId,
    pub point: [f64; 2],
}

impl SpatialEntity2D {
    pub fn new(entity: EntityId, x: f64, y: f64) -> Self {
        Self {
            entity,
            point: [x, y],
        }
    }
}

impl RTreeObject for SpatialEntity2D {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point(self.point)
    }
}

impl PointDistance for SpatialEntity2D {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        let dx = self.point[0] - point[0];
        let dy = self.point[1] - point[1];
        dx * dx + dy * dy
    }
}

/// A continuous 2D spatial index backed by an R-tree (`rstar`) with $O(\log N)$
/// updates, nearest-neighbor queries, and deterministic tie-breaking.
#[derive(Clone, Debug)]
pub struct ContinuousSpatialIndex2D {
    tree: RTree<SpatialEntity2D>,
    entities: HashMap<EntityId, [f64; 2]>,
}

impl Default for ContinuousSpatialIndex2D {
    fn default() -> Self {
        Self::new()
    }
}

impl ContinuousSpatialIndex2D {
    /// Creates an empty continuous 2D spatial index.
    pub fn new() -> Self {
        Self {
            tree: RTree::new(),
            entities: HashMap::new(),
        }
    }

    /// Number of distinct entities currently indexed.
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// Whether the spatial index contains no entities.
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// Inserts or updates an entity at continuous coordinate `(x, y)`.
    ///
    /// If the entity was already indexed at a different position, its old position
    /// is atomically removed from the R-tree to avoid ghost duplicates.
    pub fn insert(&mut self, entity: EntityId, x: f64, y: f64) -> Result<(), SpatialIndexError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(SpatialIndexError::NonFiniteCoordinate { x, y });
        }

        // If entity already exists, remove old position from R-tree
        if let Some(&old_point) = self.entities.get(&entity) {
            let old_item = SpatialEntity2D {
                entity,
                point: old_point,
            };
            self.tree.remove(&old_item);
        }

        let new_point = [x, y];
        let item = SpatialEntity2D {
            entity,
            point: new_point,
        };
        self.tree.insert(item);
        self.entities.insert(entity, new_point);
        Ok(())
    }

    /// Removes an entity from the spatial index.
    ///
    /// Returns the previous continuous position `(x, y)` if the entity was present.
    pub fn remove(&mut self, entity: EntityId) -> Option<(f64, f64)> {
        if let Some(old_point) = self.entities.remove(&entity) {
            let item = SpatialEntity2D {
                entity,
                point: old_point,
            };
            self.tree.remove(&item);
            Some((old_point[0], old_point[1]))
        } else {
            None
        }
    }

    /// Checks if an entity is indexed.
    pub fn contains(&self, entity: EntityId) -> bool {
        self.entities.contains_key(&entity)
    }

    /// Retrieves the current continuous position `(x, y)` of an entity.
    pub fn position(&self, entity: EntityId) -> Option<(f64, f64)> {
        self.entities.get(&entity).map(|p| (p[0], p[1]))
    }

    /// Finds all entities located within Euclidean `radius` of `(x, y)`.
    ///
    /// Returns a deterministic vector of `(EntityId, distance)` pairs sorted by:
    /// 1. Ascending Euclidean distance
    /// 2. EntityId (tie-breaker for simulation determinism)
    pub fn locate_within_distance(
        &self,
        x: f64,
        y: f64,
        radius: f64,
    ) -> Result<Vec<(EntityId, f64)>, SpatialIndexError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(SpatialIndexError::NonFiniteCoordinate { x, y });
        }
        if !radius.is_finite() {
            return Err(SpatialIndexError::NonFiniteRadius(radius));
        }
        if radius < 0.0 {
            return Err(SpatialIndexError::NegativeRadius(radius));
        }

        let query_point = [x, y];
        let r2 = radius * radius;
        let mut results: Vec<(EntityId, f64)> = self
            .tree
            .locate_within_distance(query_point, r2)
            .map(|item| {
                let d2 = item.distance_2(&query_point);
                (item.entity, d2.sqrt())
            })
            .collect();

        // Sort deterministically by distance, then EntityId
        results.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        Ok(results)
    }

    /// Finds the single nearest neighbor to `(x, y)` if any entity is indexed,
    /// breaking ties deterministically with the smaller EntityId.
    ///
    /// Returns `(EntityId, distance)`.
    pub fn nearest_neighbor(
        &self,
        x: f64,
        y: f64,
    ) -> Result<Option<(EntityId, f64)>, SpatialIndexError> {
        let knn = self.k_nearest_neighbors(x, y, 1)?;
        Ok(knn.into_iter().next())
    }

    /// Finds the `k` nearest neighbors to `(x, y)`.
    ///
    /// Returns a deterministic vector of `(EntityId, distance)` pairs sorted by distance,
    /// then EntityId (guaranteeing bit-for-bit replay determinism even when points are equidistant).
    pub fn k_nearest_neighbors(
        &self,
        x: f64,
        y: f64,
        k: usize,
    ) -> Result<Vec<(EntityId, f64)>, SpatialIndexError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(SpatialIndexError::NonFiniteCoordinate { x, y });
        }
        if k == 0 || self.is_empty() {
            return Ok(Vec::new());
        }

        let query_point = [x, y];
        let mut candidates: Vec<(EntityId, f64)> = Vec::new();
        let mut cutoff_dist: Option<f64> = None;

        for item in self.tree.nearest_neighbor_iter(&query_point) {
            let dist = item.distance_2(&query_point).sqrt();
            if let Some(cutoff) = cutoff_dist {
                // Keep collecting as long as items have the same distance (within floating point precision)
                // so tie-breaking by EntityId remains strictly invariant to R-tree internal ordering.
                if dist > cutoff + 1e-12 {
                    break;
                }
            }
            candidates.push((item.entity, dist));
            if candidates.len() >= k && cutoff_dist.is_none() {
                cutoff_dist = Some(dist);
            }
        }

        candidates.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        candidates.truncate(k);

        Ok(candidates)
    }

    /// Finds all entities situated within an axis-aligned bounding box `[min_x, min_y]` to `[max_x, max_y]`.
    ///
    /// Results are sorted deterministically by EntityId.
    pub fn locate_in_envelope(
        &self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
    ) -> Result<Vec<EntityId>, SpatialIndexError> {
        if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
            return Err(SpatialIndexError::NonFiniteCoordinate { x: min_x, y: min_y });
        }
        if min_x > max_x || min_y > max_y {
            return Err(SpatialIndexError::InvalidBoundingBox {
                min_x,
                max_x,
                min_y,
                max_y,
            });
        }

        let aabb = AABB::from_corners([min_x, min_y], [max_x, max_y]);
        let mut results: Vec<EntityId> = self
            .tree
            .locate_in_envelope(&aabb)
            .map(|item| item.entity)
            .collect();

        results.sort();
        Ok(results)
    }
}
