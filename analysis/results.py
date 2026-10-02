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
import math
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


# The two ways a run can size its world. See `setups_of`.
FIXED_WORLD = "fixed_world"
CONSTANT_DENSITY = "constant_density"

# The default setup the runners start from: 1000 agents in a 1000x1000 world.
# Must match DEFAULT_SWARM_SIZE and DEFAULT_WORLD in crates/core.
DEFAULT_SWARM_SIZE = 1000
DEFAULT_WORLD_SIDE = 1000.0

# The results file stores the world to four decimal places, so a world worked
# out here can differ from the stored one in the last digit.
WORLD_TOLERANCE = 0.001


def setups_of(row):
    """Which setups a run belongs to: the fixed world, constant density, both or neither.

    - Fixed world: the default 1000x1000 box whatever the swarm size, so more
      agents means a more crowded swarm.
    - Constant density: the world grown with the swarm (`--constant-density`),
      so every swarm size is as crowded as 1000 agents in 1000x1000.

    At 1000 agents the two are the same simulation, so those runs belong to
    both. A world set by hand with `--world` usually belongs to neither, and
    is left out of the tables.
    """
    agents = int(row["agents"])
    width, height = float(row["world_x"]), float(row["world_y"])
    if abs(width - height) > WORLD_TOLERANCE:
        return set()
    setups = set()
    if abs(width - DEFAULT_WORLD_SIDE) < WORLD_TOLERANCE:
        setups.add(FIXED_WORLD)
    # Same sum as world_for_constant_density in crates/core: the area grows in
    # step with the swarm, so the side grows with the square root.
    constant_density_side = DEFAULT_WORLD_SIDE * math.sqrt(agents / DEFAULT_SWARM_SIZE)
    if abs(width - constant_density_side) < WORLD_TOLERANCE:
        setups.add(CONSTANT_DENSITY)
    return setups


def load(path=RESULTS, setup=FIXED_WORLD):
    """Groups runs by configuration, keeping only the longest step count.

    Runs from one setup only (see `setups_of`). The same number of agents in a
    smaller world is a more crowded swarm doing more work, so putting both in
    one group would take the median across two different simulations and
    report it as a single measurement.
    """
    rows = list(csv.DictReader(open(path)))
    left_out = [row for row in rows if not setups_of(row)]
    if left_out:
        print(f"note: left out {len(left_out)} runs whose world was set by hand",
              file=sys.stderr)
    rows = [row for row in rows if setup in setups_of(row)]
    if not rows:
        return {}, 0

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


def summary_at(grouped, steps, agents, count):
    """One swarm size in one setup, boiled down to the numbers the comparison uses.

    Nothing if any of the runs it needs are missing.
    """
    baseline = grouped.get(("sequential", 1, agents, False))
    split = grouped.get(("distributed", count, agents, False))
    phases = phase_table(grouped, [agents], count).get(agents)
    if not (baseline and split and phases):
        return None
    sequential = middle(baseline, "simulating_seconds")
    return {
        "milliseconds_per_step": 1000 * sequential / steps,
        "speedup": sequential / middle(split, "simulating_seconds"),
        "imbalance": middle(split, "average_imbalance"),
        "waiting": phases[1],
        "communicating": phases[2],
    }


def crowding_table(fixed_world, constant_density):
    """The same swarm sizes measured both ways, side by side.

    Each argument is what `load` returns for that setup. The result maps each
    swarm size measured in both to a (fixed world, constant density) pair of
    summaries, all at the largest process count both setups were run with.

    1000 agents is left out: both setups are the same simulation there, so it
    shows nothing about crowding.
    """
    (crowded_grouped, crowded_steps), (roomy_grouped, roomy_steps) = fixed_world, constant_density
    crowded_sizes, crowded_processes = sizes_and_processes(crowded_grouped)
    roomy_sizes, roomy_processes = sizes_and_processes(roomy_grouped)
    shared_processes = set(crowded_processes) & set(roomy_processes)
    if not shared_processes:
        return {}, None
    count = max(shared_processes)
    table = {}
    for agents in sorted(set(crowded_sizes) & set(roomy_sizes)):
        if agents == DEFAULT_SWARM_SIZE:
            continue
        crowded = summary_at(crowded_grouped, crowded_steps, agents, count)
        roomy = summary_at(roomy_grouped, roomy_steps, agents, count)
        if crowded and roomy:
            table[agents] = (crowded, roomy)
    return table, count


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


def print_crowding(table, count):
    print(f"FIXED WORLD AGAINST CONSTANT DENSITY  (at {count} processes)")
    print("  The same swarm in a 1000x1000 world, and in a world grown so it is")
    print("  no more crowded than 1000 agents. 'fixed' first, 'grown' second.\n")
    columns = [("ms per step, 1 process", "milliseconds_per_step", "{:.1f}"),
               ("speedup", "speedup", "{:.2f}x"),
               ("imbalance", "imbalance", "{:.2f}x"),
               ("waiting", "waiting", "{:.1f}%"),
               ("communicating", "communicating", "{:.1f}%")]
    print("  agents  " + "".join(f"{heading:>24}" for heading, _, _ in columns))
    for agents, (crowded, roomy) in table.items():
        cells = "".join(
            f"{shape.format(crowded[key]) + ' / ' + shape.format(roomy[key]):>24}"
            for _, key, shape in columns)
        print(f"{agents:>8}  {cells}")
    print()


def main():
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else RESULTS
    loaded = {}
    for setup, heading in [(FIXED_WORLD, "FIXED 1000x1000 WORLD"),
                           (CONSTANT_DENSITY, "CONSTANT DENSITY")]:
        grouped, steps = load(path, setup)
        loaded[setup] = (grouped, steps)
        if not grouped:
            continue
        print(f"\n======== {heading} ========")
        sizes, processes = sizes_and_processes(grouped)
        print_tables(grouped, steps, sizes, processes)

    table, count = crowding_table(loaded[FIXED_WORLD], loaded[CONSTANT_DENSITY])
    if table:
        print_crowding(table, count)


if __name__ == "__main__":
    main()
