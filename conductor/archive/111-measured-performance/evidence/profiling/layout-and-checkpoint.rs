use kairo_ecs_state::{ColumnLayout, ComponentStore, World};
use std::hint::black_box;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::time::Instant;

fn elapsed<T>(run: impl FnOnce() -> T) -> (T, u128) {
    let start = Instant::now();
    let result = black_box(run());
    (result, start.elapsed().as_nanos())
}

fn layout<L: ColumnLayout>(entities: usize, mut store: ComponentStore<u64, L>) {
    let mut world = World::new();
    let (expected, load_ns) = elapsed(|| {
        (0..entities as u64)
            .map(|index| {
                let entity = world.spawn();
                let value = index.wrapping_mul(0x9e37_79b9_7f4a_7c15);
                assert!(store.insert(entity, value));
                value
            })
            .fold(0u64, u64::wrapping_add)
    });
    let scan = || {
        black_box(&store)
            .iter()
            .map(|(_, value)| *value)
            .fold(0u64, u64::wrapping_add)
    };
    assert_eq!(scan(), expected);
    let (checksum, scan_ns) = elapsed(|| (0..64).map(|_| black_box(scan())).last().unwrap());
    assert_eq!(checksum, expected);
    println!(
        "{entities},{},{},{load_ns},{scan_ns},{checksum}",
        store.row_stride_bytes(),
        store.column_bytes(),
    );
}

fn checkpoint(entities: usize) {
    let mut world = World::new();
    let ids: Vec<_> = (0..entities).map(|_| world.spawn()).collect();
    for &entity in ids.iter().step_by(3) {
        assert!(world.despawn(entity));
    }
    for _ in 0..entities / 6 {
        world.spawn();
    }

    let (snapshot, snapshot_ns) = elapsed(|| world.snapshot());
    let (bytes, encode_ns) = elapsed(|| world.checkpoint_json().unwrap());
    let (mut writer, mut reader) = UnixStream::pair().unwrap();
    let mut received = vec![0; bytes.len()];
    let ((), transfer_ns) = std::thread::scope(|scope| {
        let handle = scope.spawn(|| reader.read_exact(&mut received).unwrap());
        elapsed(|| {
            writer.write_all(&bytes).unwrap();
            handle.join().unwrap();
        })
    });
    assert_eq!(bytes, received);
    let (mut restored, decode_ns) = elapsed(|| World::from_checkpoint_json(&received).unwrap());
    assert_eq!(world.snapshot(), snapshot);
    assert_eq!(restored.snapshot(), snapshot);
    assert_eq!(restored.spawn(), world.spawn());
    println!(
        "{entities},{},{snapshot_ns},{encode_ns},{transfer_ns},{decode_ns}",
        bytes.len()
    );
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mode = args.next().unwrap();
    let entities = args.next().unwrap().parse::<usize>().unwrap();
    assert!(args.next().is_none());
    match mode.as_str() {
        "aligned" => layout(entities, ComponentStore::with_capacity(entities)),
        "tight" => layout(entities, ComponentStore::with_capacity_tight(entities)),
        "checkpoint" => checkpoint(entities),
        _ => panic!("expected aligned, tight, or checkpoint"),
    }
}
