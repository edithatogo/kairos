import sys
from collections import Counter
from pathlib import Path


def cpu_ids(value):
    result = []
    for part in filter(None, value.split(",")):
        first, separator, last = part.partition("-")
        start = int(first)
        result.extend(range(start, int(last) + 1) if separator else [start])
    return tuple(sorted(result))


def read_topology(path):
    result = Counter()
    for line in Path(path).read_text().splitlines():
        *attributes, cpus = line.split("\t")
        result[(*attributes, cpu_ids(cpus))] += 1
    return result


observed = read_topology(sys.argv[1])
expected = read_topology(sys.argv[2])
assert observed == expected, {"observed_only": observed - expected, "expected_only": expected - observed}
print(Counter(row[0] for row in observed.elements()))
