# Distributed Swarm

A distributed implementation of Reynolds' boids model, built to measure what
spatial decomposition actually costs and when it stops paying off.

<img src="images/flocking-animated.svg" width="440" alt="250 boids scattering, then forming flocks">

250 agents over 600 steps, looping. Colour is direction, so a flock shows up as
a patch of one colour: they start scattered and heading everywhere, then clump
into groups that move together. Agents crossing an edge reappear on the other
side — the world wraps around.

Boids is decentralized by construction: every agent steers using only the
neighbours inside its perception radius `r`. That makes it look trivially
parallel, but in practice it isn't. Moving the simulation from one machine to a
cluster introduces communication, synchronization, and load-imbalance costs that
don't exist sequentially, and flocking makes the last one worse over time
because agents cluster, so a uniform spatial partition drifts steadily out of
balance.

This repository contains the implementation and the measurement harness for a
bachelor thesis on that question.

**[Measurement report →](https://claude.ai/code/artifact/63c88f90-a005-4247-acf5-329335607866)**
— the findings, with charts.

## Research questions

1. **Fidelity** — does the distributed simulation reproduce the same emergent
   behaviour as the sequential one?
2. **Scaling** — how do runtime and throughput respond to added nodes, both at
   fixed total swarm size (strong scaling) and at fixed per-node load (weak
   scaling)?
3. **Crossover** — at what point do communication and synchronization dominate
   the useful work?
4. **Balance** — how much does flock-induced clustering degrade load balance
   across nodes, and how much barrier wait time does that cost?

## Approach

| Concern | Choice |
| --- | --- |
| Neighbour search | Uniform grid / cell lists, cell edge ≥ `r` (O(n) average, not naive O(n²)) |
| Partitioning | Vertical strips over the same uniform grid, one per process |
| Boundary handling | Ghost-cell exchange, ghost width ≥ `r` |
| Ownership transfer | Agent migration on region crossing |
| Coordination | Bulk Synchronous Parallel — one superstep per simulation step |

The sequential baseline uses the *same* uniform grid as the distributed
version. Comparing against a naive O(n²) implementation would report
algorithmic speedup as if it were distribution speedup — so the naive search
stays in the codebase only as the checker the grid is verified against, and is
never what speed is measured against.

Model parameters (`r`, steering weights `w_s`/`w_a`/`w_c`, `v_max`, `Δt`) are
fixed rather than tuned, so that only the execution architecture varies between
runs.

Correctness is established two ways. A test runs the whole distributed scheme —
split, swap borders, step, hand over, repeat — inside a single process and
compares against the sequential result, so algorithm errors surface under plain
`cargo test`. Both runners then reduce their final state to a fingerprint, which
catches errors in the message passing that the first check cannot see.

## Repository layout

A Cargo workspace of three crates. `core` holds everything both runners share
and knows nothing about MPI — that is what lets the two executions run identical
model code, so any difference in their results comes from the distribution
itself.

```
Cargo.toml       workspace
crates/core/     the model — one file per idea:
                   vector2d, agent, params, constants   the pieces
                   geometry                             wrap-around distances
                   neighbours/brute_force               the slow, obvious search
                   neighbours/grid                      the fast search
                   steering, simulation                 the three rules, one step
                   partition, borders, migration        who owns which strip,
                                                        what is copied across it,
                                                        and what moves between
                   metrics, report, recording, timing   measuring and reporting
crates/seq/      sequential baseline
crates/dist/     distributed runner (MPI, via rsmpi)
bench/           sweep.sh, sweep-sizes.sh
data/            recorded runs and results (gitignored; real results added
                 deliberately with `git add -f`)
analysis/        results.py — the tables; report.py — the charts
```

## Building and running

Requires Rust and an MPI implementation. On macOS:

```bash
brew install open-mpi
```

Then:

```bash
cargo build --release
cargo test --workspace

cargo run --release -p swarm-seq                  # default: 1000 agents, 600 steps
cargo run --release -p swarm-seq -- 500 300       # 500 agents, 300 steps

mpirun -n 4 ./target/release/swarm-dist           # 4 processes, defaults
mpirun -n 8 ./target/release/swarm-dist 2000 400  # 8 processes, 2000 agents
```

`--release` matters for anything you intend to measure. The distributed binary
must be launched through `mpirun` rather than `cargo run`, since `mpirun` starts
one copy of the executable per rank.

`cargo test` does not cover `swarm-dist` — its checks only run under `mpirun`,
so run that separately.

## Watching a run

Both runners take `--dump` and `--every`. In the distributed one, the root
process gathers everyone's agents, so the file holds one whole swarm per step —
the same shape the sequential runner produces, plus a column saying which
process owned each agent.

```bash
# record the same run both ways
cargo run --release -p swarm-seq -- 800 400 --dump data/sequential.csv --every 5
mpirun -n 4 ./target/release/swarm-dist 800 400 --dump data/distributed.csv --every 5

# side by side: identical positions, coloured by direction and by owner
python3 analysis/render.py --compare data/sequential.csv data/distributed.csv \
    data/comparison.html

# one run on its own
python3 analysis/render.py data/sequential.csv data/single-run.html

# or still frames, for a document
python3 analysis/render.py data/sequential.csv --svg images/run.svg
```

On the comparison page the dots sit in the same places on both sides at every
step, and on the right you can watch a dot change colour as it crosses a strip
boundary — one process handing it to another.

## Timing a run

Both runners report how long they took. The distributed one can also break a
step into its parts:

```bash
mpirun -n 4 ./target/release/swarm-dist 4000 400 --measure-phases
```

Separating waiting from communicating takes care. If you simply time the
communication, a process that finishes early sits inside its receive call
waiting for a slower neighbour, and that waiting gets counted as communication —
so communication looks expensive and uneven load looks free, and the conclusion
about what limits scaling comes out backwards. `--measure-phases` adds a barrier
after computing and before communicating, which soaks up the waiting. That
barrier costs a little speed, so runs measuring overall speed leave it out and
runs measuring the breakdown put it in.

Each part is the slowest process's figure, so they add to more than the wall
clock: different processes are slowest at different parts.

Both runners report two numbers. **`simulating`** covers the simulation and
nothing else — progress lines and recording sit outside it — and it is the one
to compare between them. **`wall clock`** covers everything the program did.

Timing on a laptop varies by around 3% between runs of the same configuration,
so a single run is not a measurement. Every configuration needs several runs and
a median.

## Reproducing the measurements

```bash
bench/sweep.sh                              # strong and weak scaling, one swarm size
bench/sweep-sizes.sh                        # strong scaling across swarm sizes
REPEATS=3 STEPS=200 bench/sweep.sh          # a quicker version

python3 analysis/results.py                 # the tables
python3 analysis/report.py data/analysis.html
python3 analysis/report.py --serbian data/analysis-sr.html
```

Every run appends one row to `data/results.csv`. Rows accumulate and are never
overwritten, so an interrupted sweep loses nothing. Every row carries the
fingerprint of the swarm the run ended with: a fast result and a slow one can
only be compared if they computed the same thing.

Raw measurement output is committed rather than summarised, so every number and
figure in the thesis can be traced back to the run that produced it.

### Where the measurements run, and what that costs

All of it runs on one machine with 10 cores, and the process count never goes
above 10. Past that, processes share cores and the timings measure the operating
system moving processes around rather than the simulation. The sweeps refuse to
run oversubscribed rather than quietly producing numbers that mean nothing.

This is a limitation worth stating plainly: these are processes on one machine,
sharing a memory bus, not separate machines with a network between them.
Communication is therefore cheaper here than it would be on a cluster, so the
point at which communication starts to dominate arrives later in these
measurements than it would on real distributed hardware.

The world stays the same size as the swarm grows, so a larger swarm is also a
denser one: each agent has more neighbours, and the work grows roughly with the
square of the swarm size rather than linearly.

## Thesis

Bachelor thesis (in Serbian): *Distribuirani Swarm: Implementacija i analiza
skalabilnosti Swarm algoritama u distribuiranom okruženju*
— Računarski fakultet Univerziteta Union, mentor Dr Jelena Vasiljević.

## License

MIT — see [LICENSE](LICENSE).
