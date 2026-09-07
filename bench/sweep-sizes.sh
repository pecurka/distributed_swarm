#!/usr/bin/env bash
#
# Strong scaling at several swarm sizes.
#
#   bench/sweep-sizes.sh
#
# The main sweep measures one swarm size, which answers "does splitting help?"
# but not "when does splitting start to help?" — and that is the question the
# thesis is actually asking. This runs the same strong-scaling curve at several
# sizes so the two can be compared.
#
# Note what "bigger" means here: the world stays the same size, so more agents
# means a denser swarm, not a larger one. Each agent then has more neighbours,
# and the work grows roughly with the square of the swarm size rather than
# linearly. That is worth saying plainly when reporting these numbers.

set -euo pipefail
cd "$(dirname "$0")/.."

REPEATS=${REPEATS:-5}
STEPS=${STEPS:-500}
SIZES=${SIZES:-"1000 2000 4000 8000 16000"}
PROCESSES=${PROCESSES:-"1 2 4 5 8 10"}
PHASE_REPEATS=${PHASE_REPEATS:-3}
RESULTS=${RESULTS:-data/results.csv}
DIST=./target/release/swarm-dist

cores=$(sysctl -n hw.physicalcpu 2>/dev/null || nproc)
for count in $PROCESSES; do
  if [ "$count" -gt "$cores" ]; then
    echo "error: asked for $count processes but this machine has $cores cores." >&2
    exit 1
  fi
done

echo "building"
cargo build --release --quiet
mkdir -p "$(dirname "$RESULTS")"
echo "writing to $RESULTS"
echo "$REPEATS repeats, $STEPS steps, sizes: $SIZES"
echo

started=$(date +%s)
run_count=0

for agents in $SIZES; do
  # The baseline for this size.
  for repeat in $(seq "$REPEATS"); do
    echo "  sequential  agents=$agents            ($repeat/$REPEATS)"
    cargo run --release --quiet -p swarm-seq -- "$agents" "$STEPS" \
      --results "$RESULTS" > /dev/null
    run_count=$((run_count + 1))
  done

  for count in $PROCESSES; do
    for repeat in $(seq "$REPEATS"); do
      echo "  strong  n=$count  agents=$agents        ($repeat/$REPEATS)"
      mpirun -n "$count" "$DIST" "$agents" "$STEPS" --results "$RESULTS" > /dev/null
      run_count=$((run_count + 1))
    done
    # Fewer repeats for the breakdown: it is about proportions, which are far
    # less noisy than absolute times.
    for repeat in $(seq "$PHASE_REPEATS"); do
      echo "  phases  n=$count  agents=$agents        ($repeat/$PHASE_REPEATS)"
      mpirun -n "$count" "$DIST" "$agents" "$STEPS" \
        --results "$RESULTS" --measure-phases > /dev/null
      run_count=$((run_count + 1))
    done
  done
done

elapsed=$(( $(date +%s) - started ))
echo
echo "done: $run_count runs in $((elapsed / 60))m $((elapsed % 60))s"
echo "results in $RESULTS ($(( $(wc -l < "$RESULTS") - 1 )) rows)"
