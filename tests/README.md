# tests/

The six-layer pyramid (Vol. V Ch. 8 §8.3), with a home per layer:

```text
1. Unit          → tests/kernel/   (store contract conformance, tick stages, hashing)
2. Contract      → tests/domains/  (schemas, composition rules, declared reads honored)
3. Determinism   → tests/kernel/   (twin runs, save-splice, golden transcripts, chaos)
4. Conservation  → tests/reference/ soaks (every conserved quantity reconciles)
5. Liveness      → tests/reference/ (headless activity envelopes per world)
6. Phenomenology → tests/reference/ (emergence signatures over reference centuries)
       — with tests/probes/ between 2 and 4: minutes-scale exotica isolation
```

**Budgets are law, not aspiration (§8.3):** layers 1–3 per commit; 4–5 nightly;
6 weekly and per release, seed-pinned plus a rotating fuzzed lane. Slow tests that
never run are decorations — schedule them or delete them.

**Failures file their own evidence:** seed, manifest, first anomalous meter tick, and
a causal-debugger bookmark, automatically (§8.5.10).
