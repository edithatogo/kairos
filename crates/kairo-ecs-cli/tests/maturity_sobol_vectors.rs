use kairo_ecs_cli::ensemble::{LegacySobolSequence, SobolSequence};

// SciPy 1.18.0, NumPy 2.5.5, qmc.Sobol(d=16, scramble=False, bits=32).
// Reference wheel ran under CPython 3.14.10. There was no cp315 wheel and the
// 3.15.2 source build required OpenBLAS. The Rust tests run without Python.
// _sobol_direction_numbers.npz SHA-256:
// 4859931147d42ce465b8605cb277f957d98b839d03194fdf06579357906d193b
// Multiply the reference's exact dyadic coordinates by 2^32. The last point
// has a single Gray bit, so its reference is the last initialized _sv column.
const VECTORS: [(u64, [u32; 16]); 7] = [
    (0, [0; 16]),
    (1, [0x8000_0000; 16]),
    (
        2,
        [
            0xc000_0000,
            0x4000_0000,
            0x4000_0000,
            0x4000_0000,
            0xc000_0000,
            0xc000_0000,
            0x4000_0000,
            0xc000_0000,
            0xc000_0000,
            0xc000_0000,
            0xc000_0000,
            0xc000_0000,
            0x4000_0000,
            0x4000_0000,
            0xc000_0000,
            0x4000_0000,
        ],
    ),
    (
        7,
        [
            0x2000_0000,
            0xa000_0000,
            0x6000_0000,
            0x2000_0000,
            0x2000_0000,
            0x6000_0000,
            0xa000_0000,
            0xa000_0000,
            0xa000_0000,
            0xe000_0000,
            0xa000_0000,
            0x2000_0000,
            0xa000_0000,
            0x6000_0000,
            0x2000_0000,
            0x2000_0000,
        ],
    ),
    (
        57,
        [
            0xa400_0000,
            0xec00_0000,
            0xbc00_0000,
            0xd400_0000,
            0x7c00_0000,
            0xc400_0000,
            0xb400_0000,
            0x0400_0000,
            0xf400_0000,
            0xf400_0000,
            0x4c00_0000,
            0x0c00_0000,
            0xac00_0000,
            0x2400_0000,
            0xf400_0000,
            0x6400_0000,
        ],
    ),
    (
        65_551,
        [
            0x1001_8000,
            0x8fff_8000,
            0xe8d5_8000,
            0xb38a_8000,
            0xd23c_8000,
            0xf3bb_8000,
            0x921f_8000,
            0xf06d_8000,
            0x4797_8000,
            0x93cf_8000,
            0x0202_8000,
            0x4c74_8000,
            0xaab1_8000,
            0xbe0a_8000,
            0xee7d_8000,
            0x7f16_8000,
        ],
    ),
    (
        4_294_967_295,
        [
            0x0000_0001,
            0xffff_ffff,
            0xc500_5555,
            0x5005_0093,
            0xb00d_db9d,
            0x40bb_bbbb,
            0xc010_4477,
            0x8763_9641,
            0x8435_0611,
            0x82ff_78f1,
            0x81d0_0a2d,
            0x8723_9795,
            0x8247_06d7,
            0x2610_00a3,
            0xd91d_919d,
            0x72c4_d755,
        ],
    ),
];

#[test]
fn sixteen_dimensions_match_the_scipy_reference_exactly() {
    let mut sequence = SobolSequence::new(16).unwrap();
    for (index, words) in VECTORS {
        let point = words.map(|word| f64::from(word) / 4_294_967_296.0);
        assert_eq!(sequence.point_at(index).unwrap(), point, "index {index}");
        sequence.seek(index).unwrap();
        assert_eq!(sequence.next_point().unwrap(), point, "index {index}");
    }
}

#[test]
fn early_points_preserve_projection_balance() {
    let mut sobol = SobolSequence::new(2).unwrap();
    let mut quadrants = [0; 4];
    for _ in 0..4 {
        let xy = sobol.next_point().unwrap();
        quadrants[usize::from(xy[0] >= 0.5) + 2 * usize::from(xy[1] >= 0.5)] += 1;
    }
    assert_eq!(quadrants, [1; 4]);
}

#[test]
fn legacy_trajectory_retains_the_nonstandard_second_coordinate() {
    let mut legacy = LegacySobolSequence::new(2);
    for point in [[0.0, 0.0], [0.5, 0.0], [0.75, 0.5], [0.25, 0.5]] {
        assert_eq!(legacy.next_point().unwrap(), point);
    }
}
