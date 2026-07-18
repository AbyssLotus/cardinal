# services/observe/

Vol. V Ch. 8 — the instruments. Meters (derived, read-only, stream-free — never
persisted as truth), telemetry series (one instrumentation feeding dashboard, soak
report, and CI signature alike, §8.1), the causal debugger (why fact(X)? what-did
event(E)? trace(A ⇒ B)? — §8.2), liveness envelopes and silence alarms, and the
inspector.

**Law:** observe everything, perturb nothing (§8.6) — instrumentation consumes no RNG
streams, forces no actualization, and if attaching the debugger changes the world, the
debugger is a participant. The income-bleed catch (§8, Designer Note) is this service's
founding story: build the meter that would have seen it on day one.
