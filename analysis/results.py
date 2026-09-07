#!/usr/bin/env python3
"""Reads a results file and prints the measurement tables.

    python3 analysis/results.py
    python3 analysis/results.py data/other-results.csv

Uses nothing outside Python's standard library.

Every number is a median across repeats. Timing on a laptop varies by a few
percent between identical runs, so a single run is not a measurement.

The charts live in `report.py`, which imports this file for its numbers.
"""

import collections
import csv
import statistics
import sys
from pathlib import Path

RESULTS = Path("data/results.csv")

# The four parts of a step, in the order they happen.
PHASE_COLUMNS = [
    "computing_seconds",
    "waiting_seconds",
    "communicating_seconds",
    "finishing_seconds",
]

# The names of the four parts, for the terminal tables. The report page has its
# own copies, translated.
PHASE_NAMES = ["computing", "waiting for others", "communicating",
               "finishing together"]


def load(path=RESULTS):
    """Groups runs by configuration, keeping only the longest step count."""
    rows = list(csv.DictReader(open(path)))
    steps = max(int(row["steps"]) for row in rows)
    grouped = collections.defaultdict(list)
    for row in rows:
        if int(row["steps"]) != steps:
            continue
        key = (row["runner"], int(row["processes"]), int(row["agents"]),
               row["measured_phases"] == "true")
        grouped[key].append(row)
    return grouped, steps


def middle(runs, column):
    """The median, which is what a handful of noisy runs can honestly give."""
    return statistics.median(float(run[column]) for run in runs)


def sizes_and_processes(grouped):
    sizes = sorted({key[2] for key in grouped if key[0] == "sequential"})
    processes = sorted({key[1] for key in grouped if key[0] == "distributed"})
    return sizes, processes


def speedup_table(grouped, sizes, processes):
    """How many times faster than one process, for each swarm size."""
    table = {}
    for agents in sizes:
        baseline = grouped.get(("sequential", 1, agents, False))
        if not baseline:
            continue
        base = middle(baseline, "simulating_seconds")
        for count in processes:
            runs = grouped.get(("distributed", count, agents, False))
            if runs:
                table[(agents, count)] = base / middle(runs, "simulating_seconds")
    return table


def phase_table(grouped, sizes, count):
    """What share of a step went where, as percentages."""
    table = {}
    for agents in sizes:
        runs = grouped.get(("distributed", count, agents, True))
        if not runs:
            continue
        parts = [middle(runs, column) for column in PHASE_COLUMNS]
        total = sum(parts)
        table[agents] = [100 * part / total for part in parts] if total else [0] * 4
    return table


def imbalance_table(grouped, sizes, processes):
    """How uneven the work was, averaged across the whole run."""
    table = {}
    for agents in sizes:
        for count in processes:
            runs = grouped.get(("distributed", count, agents, False))
            if runs:
                table[(agents, count)] = middle(runs, "average_imbalance")
    return table


# ---------------------------------------------------------------- tables ----

def print_tables(grouped, steps, sizes, processes):
    speedups = speedup_table(grouped, sizes, processes)
    imbalance = imbalance_table(grouped, sizes, processes)
    biggest = max(processes)

    print(f"\nAll numbers are medians across repeats, {steps} steps.\n")

    print("SPEEDUP  (times faster than one process)")
    print("  agents  " + "".join(f"{count:>9}" for count in processes))
    for agents in sizes:
        row = "".join(f"{speedups.get((agents, c), float('nan')):>8.2f}x"
                      for c in processes)
        print(f"{agents:>8}  {row}")

    print("\nEFFICIENCY  (speedup divided by process count)")
    print("  agents  " + "".join(f"{count:>9}" for count in processes))
    for agents in sizes:
        row = "".join(f"{100 * speedups.get((agents, c), 0) / c:>8.0f}%"
                      for c in processes)
        print(f"{agents:>8}  {row}")

    print("\nAVERAGE IMBALANCE  (busiest process over the average, 1.00 is even)")
    print("  agents  " + "".join(f"{count:>9}" for count in processes))
    for agents in sizes:
        row = "".join(f"{imbalance.get((agents, c), float('nan')):>8.2f}x"
                      for c in processes)
        print(f"{agents:>8}  {row}")

    print(f"\nWHERE THE TIME WENT at {biggest} processes  (share of a step)")
    phases = phase_table(grouped, sizes, biggest)
    print("  agents  " + "".join(f"{name:>20}" for name in PHASE_NAMES))
    for agents in sizes:
        if agents in phases:
            row = "".join(f"{value:>19.1f}%" for value in phases[agents])
            print(f"{agents:>8}  {row}")
    print()


def main():
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else RESULTS
    grouped, steps = load(path)
    sizes, processes = sizes_and_processes(grouped)
    print_tables(grouped, steps, sizes, processes)


if __name__ == "__main__":
    main()
