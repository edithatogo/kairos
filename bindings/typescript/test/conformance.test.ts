import * as fs from 'fs';
import * as path from 'path';

const FIXTURES = path.join(__dirname, '..', '..', '..', 'conformance', 'fixtures');

test('deterministic ordering fixture', () => {
    const fixture = JSON.parse(fs.readFileSync(
        path.join(FIXTURES, 'deterministic_ordering.json'), 'utf8'
    ));
    expect(fixture.version).toBe(1);
    expect(fixture.expected_kind_order).toEqual([1, 2, 4, 3]);
});

test('cancellation fixture', () => {
    const fixture = JSON.parse(fs.readFileSync(
        path.join(FIXTURES, 'cancellation.json'), 'utf8'
    ));
    expect(fixture.expected_kind_order).toEqual([1, 3]);
});

test('rng replay fixture', () => {
    const fixture = JSON.parse(fs.readFileSync(
        path.join(FIXTURES, 'rng_replay.json'), 'utf8'
    ));
    expect(fixture.run_seed).toBe(7);
    expect(fixture.expected_stream).toHaveLength(4);
});
