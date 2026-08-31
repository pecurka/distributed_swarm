# distributed_swarm

A distributed implementation of Reynolds' boids model, built to measure what
spatial decomposition actually costs and when it stops paying off.

Boids is decentralized by construction: every agent steers using only the
neighbours inside its perception radius `r`. That makes it look trivially
parallel, but in practice it isn't. Moving the simulation from one machine to a cluster introduces communication, synchronization, and load-imbalance costs that don't exist sequentially, and flocking makes the last one worse over time because agents cluster, so a uniform spatial partition drifts steadily out of balance.

This repository contains the implementation and the measurement harness for a
bachelor thesis on that question.

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

Early. Toolchain and skeleton in place; no simulation logic yet.

- [x] Toolchain verified — ranks start, exchange with neighbours, and synchronise
- [x] Core model types — vectors, agents, parameters, toroidal geometry
- [ ] Sequential baseline (uniform grid + steering rules)
- [ ] Single-node multi-process version
- [ ] Ghost-cell exchange + agent migration
- [ ] Fidelity comparison against the baseline
- [ ] Benchmark harness
- [ ] Scaling measurements

## Repository layout

A Cargo workspace of three crates. `core` holds everything both runners share
and knows nothing about MPI — that is what lets the two executions run identical
model code, so any difference in their results comes from the distribution
itself.

```
Cargo.toml       workspace
crates/core/     the model: vectors, agents, parameters, toroidal geometry
crates/seq/      sequential baseline
crates/dist/     distributed runner (MPI, via rsmpi)
bench/           benchmark configurations and run scripts
data/            raw measurement output (committed — results are reproducible)
analysis/        plot and table generation
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

cargo run --release -p swarm-seq          # sequential, default swarm size
cargo run --release -p swarm-seq -- 500   # sequential, 500 agents

mpirun -n 4 ./target/release/swarm-dist   # distributed, 4 ranks
```

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
