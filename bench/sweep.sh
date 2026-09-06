#!/usr/bin/env bash
#
# Runs every measurement configuration several times and collects the results.
#
#   bench/sweep.sh
#   REPEATS=3 STEPS=200 bench/sweep.sh        # a quicker version
#
# Each run appends one row to the results file, so nothing is lost if the sweep
# is interrupted — and nothing is overwritten if it is run again.
#
# Close everything else first. Timing on a laptop varies by a few percent
# between identical runs, and a busy machine makes that much worse.

set -euo pipefail
cd "$(dirname "$0")/.."

REPEATS=${REPEATS:-5}
STEPS=${STEPS:-400}

# Strong scaling: the same swarm, split more and more ways.
AGENTS=${AGENTS:-4000}

# Weak scaling: the same amount of work each, however many processes there are.
PER_PROCESS=${PER_PROCESS:-500}

# This machine has 10 cores. Beyond that processes share cores and the timings
# stop measuring the simulation and start measuring the operating system.
PROCESSES=${PROCESSES:-"1 2 4 5 8 10"}

RESULTS=${RESULTS:-data/results.csv}
DIST=./target/release/swarm-dist

cores=$(sysctl -n hw.physicalcpu 2>/dev/null || nproc)
for count in $PROCESSES; do
  if [ "$count" -gt "$cores" ]; then
    echo "error: asked for $count processes but this machine has $cores cores." >&2
    echo "       Oversubscribed runs measure the scheduler, not the simulation." >&2
    exit 1
  fi
done

echo "building"
cargo build --release --quiet

mkdir -p "$(dirname "$RESULTS")"
echo "writing to $RESULTS"
echo "$REPEATS repeats, $STEPS steps, up to $cores processes"
echo

run_count=0
started=$(date +%s)

# 1. The baseline. Everything else is measured against this.
for repeat in $(seq "$REPEATS"); do
  echo "  sequential                       ($repeat/$REPEATS)"
  cargo run --release --quiet -p swarm-seq -- "$AGENTS" "$STEPS" \
    --results "$RESULTS" > /dev/null
  run_count=$((run_count + 1))
done

# 2. Strong scaling: fixed swarm, more processes. No extra barrier, so these
#    are the honest overall-speed numbers.
for count in $PROCESSES; do
  for repeat in $(seq "$REPEATS"); do
    echo "  strong  n=$count  agents=$AGENTS        ($repeat/$REPEATS)"
    mpirun -n "$count" "$DIST" "$AGENTS" "$STEPS" --results "$RESULTS" > /dev/null
    run_count=$((run_count + 1))
  done
done

# 3. The same again with the extra barrier, which is what makes the
#    compute/wait/communicate breakdown meaningful. Slightly slower, so these
#    rows are for the breakdown only, never for speedup.
for count in $PROCESSES; do
  for repeat in $(seq "$REPEATS"); do
    echo "  phases  n=$count  agents=$AGENTS        ($repeat/$REPEATS)"
    mpirun -n "$count" "$DIST" "$AGENTS" "$STEPS" \
      --results "$RESULTS" --measure-phases > /dev/null
    run_count=$((run_count + 1))
  done
done

# 4. Weak scaling: each process always holds the same number of agents, so if
#    splitting the work were free the time would not change at all.
for count in $PROCESSES; do
  agents=$((PER_PROCESS * count))
  for repeat in $(seq "$REPEATS"); do
    echo "  weak    n=$count  agents=$agents         ($repeat/$REPEATS)"
    mpirun -n "$count" "$DIST" "$agents" "$STEPS" --results "$RESULTS" > /dev/null
    run_count=$((run_count + 1))
  done
done

elapsed=$(( $(date +%s) - started ))
echo
echo "done: $run_count runs in $((elapsed / 60))m $((elapsed % 60))s"
echo "results in $RESULTS ($(( $(wc -l < "$RESULTS") - 1 )) rows)"
