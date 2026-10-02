#!/usr/bin/env bash
#
# Strong scaling at several swarm sizes, holding the swarm's crowding fixed.
#
#   bench/sweep-density.sh
#   SIZES="1000 10000" bench/sweep-density.sh     # a quicker version
#
# The difference from sweep-sizes.sh is one flag. That script leaves the world
# at a fixed 1000x1000, so a bigger swarm is also a more crowded one: every
# agent gets more neighbours to look at, the work grows with the square of the
# swarm size, and "bigger problem" cannot be told apart from "denser flock".
# This script passes --constant-density, which grows the world along with the
# swarm so each agent keeps roughly the same number of neighbours.
#
# What that costs, measured sequentially at 10 steps each:
#
#     agents      fixed world      constant density
#       1000         0.7 ms/step        0.7 ms/step
#      10000        ~20   ms/step       4.4 ms/step
#     100000     ~1900   ms/step       37.7 ms/step
#    1000000   ~190000   ms/step      890   ms/step
#
# So a million agents is about seven minutes a run here, against an estimated
# twenty-six hours with a fixed world. That is what makes this sweep possible
# at all.
#
# The sizes from 1000 to 16000 are the ones sweep-sizes.sh uses, so the report
# can compare the same swarm with and without the crowding. The larger sizes
# only make sense here: the fixed world is far too slow to reach them.
#
# Rough wall clock for the whole sweep with the defaults below: a few minutes
# for everything up to 100000, plus about three hours for 1000000 on its own.
# Drop the last size from SIZES if that is too long.
#
# Close everything else first. Timing on a laptop varies by a few percent
# between identical runs, and a busy machine makes that much worse.

set -euo pipefail
cd "$(dirname "$0")/.."

REPEATS=${REPEATS:-5}
STEPS=${STEPS:-500}
SIZES=${SIZES:-"1000 2000 4000 8000 10000 16000 100000 1000000"}
PROCESSES=${PROCESSES:-"1 2 4 5 8 10"}
PHASE_REPEATS=${PHASE_REPEATS:-3}
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
echo "$REPEATS repeats, $STEPS steps, sizes: $SIZES (constant density)"
echo

started=$(date +%s)
run_count=0

for agents in $SIZES; do
  # The baseline for this size. It needs the same flag as everything else, or
  # the speedup would be measured against a different simulation.
  for repeat in $(seq "$REPEATS"); do
    echo "  sequential  agents=$agents            ($repeat/$REPEATS)"
    ./target/release/swarm-seq "$agents" "$STEPS" \
      --constant-density --results "$RESULTS" > /dev/null
    run_count=$((run_count + 1))
  done

  for count in $PROCESSES; do
    for repeat in $(seq "$REPEATS"); do
      echo "  strong  n=$count  agents=$agents        ($repeat/$REPEATS)"
      mpirun -n "$count" "$DIST" "$agents" "$STEPS" \
        --constant-density --results "$RESULTS" > /dev/null
      run_count=$((run_count + 1))
    done
    # Fewer repeats for the breakdown: it is about proportions, which are far
    # less noisy than absolute times.
    for repeat in $(seq "$PHASE_REPEATS"); do
      echo "  phases  n=$count  agents=$agents        ($repeat/$PHASE_REPEATS)"
      mpirun -n "$count" "$DIST" "$agents" "$STEPS" \
        --constant-density --results "$RESULTS" --measure-phases > /dev/null
      run_count=$((run_count + 1))
    done
  done
done

elapsed=$(( $(date +%s) - started ))
echo
echo "done: $run_count runs in $((elapsed / 60))m $((elapsed % 60))s"
echo "results in $RESULTS ($(( $(wc -l < "$RESULTS") - 1 )) rows)"
