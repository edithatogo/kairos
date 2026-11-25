import argparse
import csv
import re
import statistics
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("binary")
parser.add_argument("output")
args = parser.parse_args()

columns = {
    "state": ("entities", "payload_stride", "column_bytes", "load_ns", "scan_ns", "checksum"),
    "checkpoint": (
        "entities", "checkpoint_bytes", "snapshot_ns", "encode_ns", "transfer_ns", "decode_ns"
    ),
}


def measure(*command):
    run = subprocess.run(
        ["/usr/bin/time", "-l", *command], capture_output=True, check=True, text=True
    )
    peak = int(re.search(r"(\d+)\s+maximum resident set size", run.stderr)[1])
    return list(map(int, run.stdout.strip().split(","))), peak


for name, fields in columns.items():
    path = f"{args.output}/{name}.csv"
    with open(path, "w", newline="") as output:
        writer = csv.writer(output)
        writer.writerow(["pair", "mode", *fields, "rss_bytes"])
        for entities in (4096, 262144):
            modes = ("aligned", "tight") if name == "state" else ("checkpoint",)
            for mode in modes:
                measure(args.binary, mode, str(entities))
            samples = {mode: [] for mode in modes}
            for pair in range(10):
                checksums = set()
                order = modes if pair % 2 == 0 else modes[::-1]
                for mode in order:
                    row, peak = measure(args.binary, mode, str(entities))
                    writer.writerow([pair, mode, *row, peak])
                    samples[mode].append([*row, peak])
                    if name == "state":
                        checksums.add(row[-1])
                assert name != "state" or len(checksums) == 1
            for mode in modes:
                medians = map(statistics.median, zip(*samples[mode], strict=True))
                print(mode, dict(zip((*fields, "rss_bytes"), medians, strict=True)))
            if name == "state":
                for field in ("load_ns", "scan_ns", "rss_bytes"):
                    index = (*fields, "rss_bytes").index(field)
                    ratios = (
                        tight[index] / aligned[index]
                        for tight, aligned in zip(samples["tight"], samples["aligned"], strict=True)
                    )
                    print(entities, field, "tight/aligned median", statistics.median(ratios))
