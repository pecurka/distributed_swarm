#!/usr/bin/env bash
#
# Weak scaling: every process always holds the same number of agents.
#
#   bench/sweep-weak.sh
#   PER_PROCESS="500" bench/sweep-weak.sh       # a quicker version
#
# If splitting the work were free, a step would take the same time at every
# process count, and the number of agents moved per second would grow in step
# with the processes. How far the measurements fall short of that is the cost
# of splitting.
#
# Runs in both setups. In the fixed 1000x1000 world, more processes means more
# agents in the same space, so each process's agents also get more crowded and
# its work grows even though its agent count does not. With --constant-density
# the world grows with the swarm, so each process's work really does stay the
# same. The fixed-world runs are kept to show how much that difference matters.
#
# Uses 500 steps, the same as the other sweeps, because the analysis only
# compares runs of the same length.
#
# Close everything else first. Timing on a laptop varies by a few percent
# between identical runs, and a busy machine makes that much worse.

set -euo pipefail
cd "$(dirname "$0")/.."

REPEATS=${REPEATS:-5}
STEPS=${STEPS:-500}
PER_PROCESS=${PER_PROCESS:-"500 2000"}
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
echo "$REPEATS repeats, $STEPS steps, per process: $PER_PROCESS"
echo

started=$(date +%s)
run_count=0

# No flag leaves the world at its fixed 1000x1000.
for world_flag in "" "--constant-density"; do
  for per_process in $PER_PROCESS; do
    for count in $PROCESSES; do
      agents=$((per_process * count))
      for repeat in $(seq "$REPEATS"); do
        echo "  weak  n=$count  agents=$agents  ${world_flag:-fixed world}  ($repeat/$REPEATS)"
        # $world_flag is left unquoted on purpose: when it is empty it must
        # disappear rather than be passed as an empty argument.
        # shellcheck disable=SC2086
        mpirun -n "$count" "$DIST" "$agents" "$STEPS" \
          $world_flag --results "$RESULTS" > /dev/null
        run_count=$((run_count + 1))
      done
    done
  done
done

elapsed=$(( $(date +%s) - started ))
echo
echo "done: $run_count runs in $((elapsed / 60))m $((elapsed % 60))s"
echo "results in $RESULTS ($(( $(wc -l < "$RESULTS") - 1 )) rows)"
