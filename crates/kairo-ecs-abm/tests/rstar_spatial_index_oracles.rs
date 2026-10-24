//! Comprehensive test oracles for rstar continuous spatial index qualification (Track 86).
//!
//! Validates:
//! 1. Brute-force neighbour oracle (verifies exact Euclidean distance match across dense coordinates).
//! 2. k-nearest neighbors oracle with deterministic distance and EntityId tie-breaking.
//! 3. Movement and removal lifecycle (proves zero ghost duplicates after multi-step movement).
//! 4. Duplicate ID idempotence (re-inserting existing entity updates location without index bloat).
//! 5. Non-finite and boundary validation (rejection of NaN, Infinity, negative radii, inverted boxes).
//! 6. Axis-aligned bounding box envelope queries against brute-force containment.
//! 7. Sparse, dense, and skewed cluster spatial distributions.
//! 8. Shuffled insertion order invariance (verifies bit-for-bit replay determinism).
//! 9. Interoperability with ContinuousSpace2D and SpatialTopology.
//! 10. GeoCoordinate tangent plane projected spatial indexing.

use kairo_ecs_abm::geometry::{GeoCoordinate, LocalTangentPlaneProjection};
use kairo_ecs_abm::spatial_index::{ContinuousSpatialIndex2D, SpatialIndexError};
use kairo_ecs_abm::ContinuousSpace2D;
use kairo_ecs_types::EntityId;

#[test]
fn test_brute_force_neighbour_oracle() {
    let mut index = ContinuousSpatialIndex2D::new();
    let mut points: Vec<(EntityId, f64, f64)> = Vec::new();

    // Deterministic synthetic population of 300 entities
    for i in 0..300 {
        let entity = EntityId::new(i as u64, 0);
        let x = ((i * 37) % 500) as f64 + (i as f64 * 0.1);
        let y = ((i * 73) % 500) as f64 - (i as f64 * 0.05);
        index.insert(entity, x, y).unwrap();
        points.push((entity, x, y));
    }

    assert_eq!(index.len(), 300);

    // Run 20 query probes against brute-force linear search
    for q in 0..20 {
        let qx = (q * 23 % 500) as f64;
        let qy = (q * 47 % 500) as f64;
        let radius = 25.0 + (q as f64 * 5.0);
        let r2 = radius * radius;

        // Brute-force oracle
        let mut expected: Vec<(EntityId, f64)> = points
            .iter()
            .filter_map(|&(e, x, y)| {
                let d2 = (x - qx).powi(2) + (y - qy).powi(2);
                if d2 <= r2 {
                    Some((e, d2.sqrt()))
                } else {
                    None
                }
            })
            .collect();
        expected.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        // Spatial index query
        let actual = index.locate_within_distance(qx, qy, radius).unwrap();

        assert_eq!(
            actual.len(),
            expected.len(),
            "Mismatch in neighbor count at probe {q} (radius {radius})"
        );
        for (act, exp) in actual.iter().zip(expected.iter()) {
            assert_eq!(act.0, exp.0);
            assert!(
                (act.1 - exp.1).abs() < 1e-9,
                "Distance mismatch for {:?}: {} vs {}",
                act.0,
                act.1,
                exp.1
            );
        }
    }
}

#[test]
fn test_k_nearest_neighbors_oracle() {
    let mut index = ContinuousSpatialIndex2D::new();
    let mut points: Vec<(EntityId, f64, f64)> = Vec::new();

    for i in 0..200 {
        let entity = EntityId::new(i as u64, 0);
        let x = (i as f64) * 2.0;
        let y = ((i * 17) % 100) as f64;
        index.insert(entity, x, y).unwrap();
        points.push((entity, x, y));
    }

    for k in [1, 5, 10, 25] {
        let qx = 100.0;
        let qy = 50.0;

        // Brute-force sort all points
        let mut all: Vec<(EntityId, f64)> = points
            .iter()
            .map(|&(e, x, y)| {
                let dist = ((x - qx).powi(2) + (y - qy).powi(2)).sqrt();
                (e, dist)
            })
            .collect();
        all.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });
        let expected: Vec<(EntityId, f64)> = all.into_iter().take(k).collect();

        let actual = index.k_nearest_neighbors(qx, qy, k).unwrap();
        assert_eq!(actual.len(), k);
        for (act, exp) in actual.iter().zip(expected.iter()) {
            assert_eq!(act.0, exp.0);
            assert!((act.1 - exp.1).abs() < 1e-9);
        }

        // Nearest neighbor single query matches k=1
        if k == 1 {
            let single = index.nearest_neighbor(qx, qy).unwrap().unwrap();
            assert_eq!(single.0, expected[0].0);
            assert!((single.1 - expected[0].1).abs() < 1e-9);
        }
    }
}

#[test]
fn test_movement_and_removal_lifecycle() {
    let mut index = ContinuousSpatialIndex2D::new();

    let e1 = EntityId::new(1, 0);
    let e2 = EntityId::new(2, 0);
    let e3 = EntityId::new(3, 0);

    index.insert(e1, 10.0, 10.0).unwrap();
    index.insert(e2, 20.0, 20.0).unwrap();
    index.insert(e3, 30.0, 30.0).unwrap();
    assert_eq!(index.len(), 3);

    // Initial query at (10, 10) radius 5 returns only e1
    let q1 = index.locate_within_distance(10.0, 10.0, 5.0).unwrap();
    assert_eq!(q1.len(), 1);
    assert_eq!(q1[0].0, e1);

    // Move e1 to (100.0, 100.0)
    index.insert(e1, 100.0, 100.0).unwrap();
    assert_eq!(index.len(), 3, "Move must not increase entity count");

    // Old location now returns empty
    let q_old = index.locate_within_distance(10.0, 10.0, 5.0).unwrap();
    assert!(
        q_old.is_empty(),
        "Old location must not contain moved entity"
    );

    // New location returns e1
    let q_new = index.locate_within_distance(100.0, 100.0, 5.0).unwrap();
    assert_eq!(q_new.len(), 1);
    assert_eq!(q_new[0].0, e1);

    // Position inquiry
    assert_eq!(index.position(e1), Some((100.0, 100.0)));
    assert_eq!(index.position(e2), Some((20.0, 20.0)));

    // Remove e2
    let removed = index.remove(e2);
    assert_eq!(removed, Some((20.0, 20.0)));
    assert_eq!(index.len(), 2);
    assert!(!index.contains(e2));

    // Non-existent removal returns None
    assert_eq!(index.remove(EntityId::new(999, 0)), None);

    // e2 no longer returned in queries
    let q_e2 = index.locate_within_distance(20.0, 20.0, 10.0).unwrap();
    assert!(q_e2.is_empty());
}

#[test]
fn test_duplicate_id_idempotence() {
    let mut index = ContinuousSpatialIndex2D::new();
    let entity = EntityId::new(42, 0);

    // Repeated insertion of same entity
    for step in 0..10 {
        let coord = (step as f64) * 10.0;
        index.insert(entity, coord, coord).unwrap();
        assert_eq!(index.len(), 1);
        assert_eq!(index.position(entity), Some((coord, coord)));
    }

    // Envelope query returns only 1 instance
    let in_box = index.locate_in_envelope(0.0, 0.0, 100.0, 100.0).unwrap();
    assert_eq!(in_box, vec![entity]);
}

#[test]
fn test_nonfinite_and_boundary_rejection() {
    let mut index = ContinuousSpatialIndex2D::new();
    let e = EntityId::new(1, 0);

    // NaN / Infinity coordinates rejected
    assert!(matches!(
        index.insert(e, f64::NAN, 10.0),
        Err(SpatialIndexError::NonFiniteCoordinate { .. })
    ));
    assert!(matches!(
        index.insert(e, 10.0, f64::INFINITY),
        Err(SpatialIndexError::NonFiniteCoordinate { .. })
    ));

    index.insert(e, 10.0, 10.0).unwrap();

    // Query non-finite parameters rejected
    assert!(matches!(
        index.locate_within_distance(f64::NAN, 10.0, 5.0),
        Err(SpatialIndexError::NonFiniteCoordinate { .. })
    ));
    assert!(matches!(
        index.locate_within_distance(10.0, 10.0, f64::NAN),
        Err(SpatialIndexError::NonFiniteRadius(_))
    ));
    assert!(matches!(
        index.locate_within_distance(10.0, 10.0, -1.0),
        Err(SpatialIndexError::NegativeRadius(r)) if r == -1.0
    ));

    // Invalid bounding boxes rejected
    assert!(matches!(
        index.locate_in_envelope(20.0, 10.0, 10.0, 20.0),
        Err(SpatialIndexError::InvalidBoundingBox { .. })
    ));
    assert!(matches!(
        index.locate_in_envelope(10.0, 30.0, 20.0, 20.0),
        Err(SpatialIndexError::InvalidBoundingBox { .. })
    ));
}

#[test]
fn test_bounding_box_envelope_oracle() {
    let mut index = ContinuousSpatialIndex2D::new();
    let mut points: Vec<(EntityId, f64, f64)> = Vec::new();

    for i in 0..150 {
        let e = EntityId::new(i as u64, 0);
        let x = (i as f64) * 3.0;
        let y = ((i * 31) % 150) as f64;
        index.insert(e, x, y).unwrap();
        points.push((e, x, y));
    }

    let min_x = 50.0;
    let min_y = 20.0;
    let max_x = 250.0;
    let max_y = 100.0;

    let mut expected: Vec<EntityId> = points
        .iter()
        .filter_map(|&(e, x, y)| {
            if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
                Some(e)
            } else {
                None
            }
        })
        .collect();
    expected.sort();

    let actual = index
        .locate_in_envelope(min_x, min_y, max_x, max_y)
        .unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn test_sparse_dense_skewed_workloads() {
    let mut index = ContinuousSpatialIndex2D::new();

    // 100 dense cluster entities in [0..5] x [0..5]
    for i in 0..100 {
        let e = EntityId::new(i, 0);
        let x = (i as f64 % 10.0) * 0.5;
        let y = ((i / 10) as f64) * 0.5;
        index.insert(e, x, y).unwrap();
    }

    // 10 sparse outlier entities in [10_000..100_000]
    for i in 100..110 {
        let e = EntityId::new(i, 0);
        let x = (i as f64) * 1_000.0;
        let y = (i as f64) * 1_000.0;
        index.insert(e, x, y).unwrap();
    }

    assert_eq!(index.len(), 110);

    // Query inside dense cluster
    let dense_hits = index.locate_within_distance(2.5, 2.5, 1.0).unwrap();
    assert!(!dense_hits.is_empty());
    assert!(dense_hits.iter().all(|(e, _)| e.index < 100));

    // Nearest neighbor to far outlier
    let far_nn = index
        .nearest_neighbor(100_000.0, 100_000.0)
        .unwrap()
        .unwrap();
    assert_eq!(far_nn.0, EntityId::new(100, 0));
    assert!(far_nn.1 < 1e-9);
}

#[test]
fn test_shuffled_insertion_order_invariance() {
    // Verifies bit-for-bit replay determinism across different insertion orders
    let mut original: Vec<(EntityId, f64, f64)> = (0..50)
        .map(|i| {
            let e = EntityId::new(i as u64, 0);
            let x = ((i * 19) % 100) as f64;
            let y = ((i * 37) % 100) as f64;
            (e, x, y)
        })
        .collect();

    let mut index1 = ContinuousSpatialIndex2D::new();
    for &(e, x, y) in &original {
        index1.insert(e, x, y).unwrap();
    }

    // Pseudo-random shuffle without external non-deterministic sources
    original.sort_by_key(|item| (item.0.index * 41) % 97);

    let mut index2 = ContinuousSpatialIndex2D::new();
    for &(e, x, y) in &original {
        index2.insert(e, x, y).unwrap();
    }

    // Both indexes must produce identical results for queries
    let res1 = index1.locate_within_distance(50.0, 50.0, 30.0).unwrap();
    let res2 = index2.locate_within_distance(50.0, 50.0, 30.0).unwrap();
    assert_eq!(res1, res2);

    let knn1 = index1.k_nearest_neighbors(50.0, 50.0, 10).unwrap();
    let knn2 = index2.k_nearest_neighbors(50.0, 50.0, 10).unwrap();
    assert_eq!(knn1, knn2);
}

#[test]
fn test_continuous_space_2d_interoperability() {
    let mut cspace = ContinuousSpace2D::new(10.0);
    let e1 = EntityId::new(1, 0);
    let e2 = EntityId::new(2, 0);
    let e3 = EntityId::new(3, 0);

    cspace.place_agent(e1, (5.0, 5.0));
    cspace.place_agent(e2, (12.0, 5.0));
    cspace.place_agent(e3, (50.0, 50.0));

    // Move e1 to (14.0, 5.0)
    cspace.place_agent(e1, (14.0, 5.0));

    // Remove e3
    assert_eq!(cspace.remove_agent(e3), Some((50.0, 50.0)));

    // Query cspace
    let within_cspace = cspace.neighbors_within((12.0, 5.0), 3.0);
    assert_eq!(within_cspace, vec![e1, e2]);

    // Convert to spatial index
    let s_index = cspace.to_spatial_index().unwrap();
    assert_eq!(s_index.len(), 2);

    let within_index = s_index.locate_within_distance(12.0, 5.0, 3.0).unwrap();
    let index_entities: Vec<EntityId> = within_index.into_iter().map(|(e, _)| e).collect();
    assert_eq!(index_entities, vec![e2, e1]); // e2 distance is 0.0, e1 distance is 2.0
}

#[test]
fn test_geocoordinate_tangent_plane_spatial_indexing() {
    // Integrates GeoCoordinate and LocalTangentPlaneProjection with ContinuousSpatialIndex2D
    let hospital = GeoCoordinate::try_new(51.5074, -0.1278).unwrap(); // London central
    let ltp = LocalTangentPlaneProjection::new(hospital);

    let mut spatial_index = ContinuousSpatialIndex2D::new();

    // Clinics around London
    let c1 = GeoCoordinate::try_new(51.5150, -0.1200).unwrap(); // ~1.0 km NE
    let c2 = GeoCoordinate::try_new(51.5000, -0.1400).unwrap(); // ~1.2 km SW
    let c3 = GeoCoordinate::try_new(51.6000, -0.1278).unwrap(); // ~10.3 km N

    let id1 = EntityId::new(101, 0);
    let id2 = EntityId::new(102, 0);
    let id3 = EntityId::new(103, 0);

    let p1 = ltp.project(&c1);
    let p2 = ltp.project(&c2);
    let p3 = ltp.project(&c3);

    spatial_index.insert(id1, p1.x(), p1.y()).unwrap();
    spatial_index.insert(id2, p2.x(), p2.y()).unwrap();
    spatial_index.insert(id3, p3.x(), p3.y()).unwrap();

    // Find clinics within 2 km of hospital origin (0, 0)
    let nearby = spatial_index
        .locate_within_distance(0.0, 0.0, 2_000.0)
        .unwrap();
    assert_eq!(nearby.len(), 2);
    assert_eq!(nearby[0].0, id1);
    assert_eq!(nearby[1].0, id2);

    // Nearest clinic to hospital
    let nearest = spatial_index.nearest_neighbor(0.0, 0.0).unwrap().unwrap();
    assert_eq!(nearest.0, id1);
    assert!(nearest.1 > 900.0 && nearest.1 < 1_200.0);
}
