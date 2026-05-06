import json
import os

FIXTURES = os.path.join(os.path.dirname(__file__), '..', '..', '..', 'conformance', 'fixtures')


def test_deterministic_ordering():
    with open(os.path.join(FIXTURES, 'deterministic_ordering.json')) as f:
        fixture = json.load(f)
    assert fixture['version'] == 1
    assert fixture['expected_kind_order'] == [1, 2, 4, 3]


def test_cancellation():
    with open(os.path.join(FIXTURES, 'cancellation.json')) as f:
        fixture = json.load(f)
    assert fixture['expected_kind_order'] == [1, 3]


def test_rng_replay():
    with open(os.path.join(FIXTURES, 'rng_replay.json')) as f:
        fixture = json.load(f)
    assert fixture['run_seed'] == 7
    assert len(fixture['expected_stream']) == 4
