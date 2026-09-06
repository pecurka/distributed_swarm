# distributed_swarm

A distributed implementation of Reynolds' boids model, built to measure what
spatial decomposition actually costs and when it stops paying off.

Boids is decentralized by construction: every agent steers using only the
neighbours inside its perception radius `r`. That makes it look trivially
parallel, but in practice it isn't. Moving the simulation from one machine to a cluster introduces communication, synchronization, and load-imbalance costs that don't exist sequentially, and flocking makes the last one worse over time because agents cluster, so a uniform spatial partition drifts steadily out of balance.

This repository contains the implementation and the measurement harness for a
bachelor thesis on that question.

<img src="images/flocking-animated.svg" width="440" alt="250 boids scattering, then forming flocks">

250 agents over 600 steps, looping. Colour is direction, so a flock shows up as
a patch of one colour: they start scattered and heading everywhere, then clump
into groups that move together. Agents crossing an edge reappear on the other
side — the world wraps around.

![A longer run as still frames](images/flocking.svg)

The same thing as still frames, with 600 agents.

## Research questions

1. **Fidelity** - does the distributed simulation reproduce the same emergent
   behaviour as the sequential one?
2. **Scaling** - how do runtime and throughput respond to added nodes, both at
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
| Partitioning | Data decomposition over the same uniform grid |
| Boundary handling | Ghost-cell exchange, ghost width ≥ `r` |
| Ownership transfer | Agent migration on region crossing |
| Coordination | Bulk Synchronous Parallel — one superstep per simulation step |

The sequential baseline uses the *same* uniform grid as the distributed
version. Comparing against a naive O(n²) implementation would report
algorithmic speedup as if it were distribution speedup.

Model parameters (`r`, steering weights `w_s`/`w_a`/`w_c`, `v_max`, `Δt`) are
fixed rather than tuned, so that only the execution architecture varies between
runs.

## Status

The sequential baseline works. `swarm-dist` splits the world across processes,
swaps border agents between them, and gets a single step exactly right — but
does not yet hand agents over when they cross a boundary, so it cannot yet be
trusted over many steps.

- [x] Toolchain verified — processes start, exchange with neighbours, synchronise
- [x] Core model types — vectors, agents, parameters, toroidal geometry
- [x] Sequential baseline — steering rules, uniform grid, deterministic setup
- [x] Visualisation — record a run to CSV, draw it as a page or an SVG
- [x] Splitting the world — vertical strips, one per process
- [x] Border sharing — copies of edge agents swapped between neighbours
- [ ] Handing agents over when they cross a boundary
- [ ] Fidelity comparison against the baseline
- [ ] Benchmark harness
- [ ] Scaling measurements

### One step is already provably correct

Before each step, every process sends its two neighbours a copy of the agents
standing in a band along its edges, as wide as an agent can see. Those copies
are read-only: a process uses them to work out how its own agents should steer,
and never moves them.

That makes a single step exactly right. A test splits a 1000-agent swarm across
1, 2, 3, 4, 8 and 20 processes, runs one step of the whole scheme, puts the
results back together, and checks them against the sequential run — they must
match bit for bit, not merely closely. A second test runs the same thing with no
border copies and checks the answer *differs*, so the first test cannot pass by
the copies doing nothing.

Both run under plain `cargo test`. The border logic lives in `swarm-core` with
no MPI in it precisely so the whole scheme can be checked inside one process.

### What the distributed runner still does not do

**Nobody hands agents over.** Ownership is decided once at the start and never
revisited, so an agent that walks into the next strip keeps being simulated by
the process it started in. The runner reports this directly, as a count of
agents standing outside the strip that still owns them:

```
  step   agents per process (min / average / max)   strayed
     0     239 /   250.0 /   264       0
   600     239 /   250.0 /   264     561
```

Over half the swarm, by step 600. That column is there on purpose: without it
the steady 1.06x imbalance reads like healthy load balance, when in fact the
counts describe where agents *started*, not where they are.

Until that is fixed, only a single step can be checked against the sequential
run. Many steps in a row need agents to be handed over as they cross.

The uniform grid is checked against the every-agent-against-every-agent search:
both must produce bit-identical results, step after step. The slow version stays
in the codebase as that checker and is never what speed is measured against —
comparing to it would report replacing a bad algorithm as if it were a gain from
spreading the work out.

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
                   neighbours                           the slow, obvious search
                   grid                                 the fast search
                   steering, simulation                 the three rules, one step
                   partition, borders                   who owns which strip,
                                                        and what crosses between
                   metrics, report, recording           measuring and reporting
crates/seq/      sequential baseline
crates/dist/     distributed runner (MPI, via rsmpi)
bench/           benchmark configurations and run scripts
data/            recorded runs. Demo output is ignored by git; real measurement
                 results are added deliberately with `git add -f`, so every
                 figure can be traced to the run that produced it
analysis/        render.py — turns a recorded run into a page or an SVG
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

cargo run --release -p swarm-seq                # default: 1000 agents, 600 steps
cargo run --release -p swarm-seq -- 500 300     # 500 agents, 300 steps

mpirun -n 4 ./target/release/swarm-dist         # 4 processes, defaults
mpirun -n 8 ./target/release/swarm-dist 2000 400  # 8 processes, 2000 agents
```

### Watching a run

The simulation can save agent positions to a file, and a separate script turns
that into something you can look at. Recording is off unless you ask for it, so
runs whose speed matters write nothing.

```bash
# run and record every 5th step
cargo run --release -p swarm-seq -- 800 600 --dump data/run.csv --every 5

# an interactive page: play, pause, scrub through the run
python3 analysis/render.py data/run.csv data/run.html
open data/run.html

# or still frames, for putting in a document
python3 analysis/render.py data/run.csv --svg images/run.svg
```

The renderer uses nothing outside Python's standard library, so there is
nothing to install. The interactive page carries all its data inside it — one
file, works offline, but it grows with agents times frames, so use a larger
`--every` for long runs.

`--release` matters for anything you intend to measure. The distributed binary
must be launched through `mpirun` rather than `cargo run`, since `mpirun` starts
one copy of the executable per rank.

Note that `cargo test` does not cover `swarm-dist` — its checks only run under
`mpirun`, so run that separately.

## Reproducing the measurements

<!-- TODO: exact commands, hardware/cluster description, and how to regenerate
     every figure  -->

Raw measurement output is committed rather than summarized, so every number and
figure in the thesis can be traced back to the run that produced it.

## Thesis

Bachelor thesis (in Serbian): *Distribuirani Swarm: Implementacija i analiza
skalabilnosti Swarm algoritama u distribuiranom okruženju*
— Računarski fakultet Univerziteta Union, mentor Dr Jelena Vasiljević.

<!-- TODO: link doc -->

## License

MIT — see [LICENSE](LICENSE).
