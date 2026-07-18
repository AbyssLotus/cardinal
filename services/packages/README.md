# services/packages/

Vol. IV made executable: the world-package loader and validator.

Planned: manifest + engine-range enforcement (Vol. IV Ch. 1), the content pipeline
(parse → resolve → compose → validate → seal, deterministic with provenance,
Vol. IV Ch. 3 §3.3), the five-layer validation stack (schema, reference, coherence,
world, scenario — Vol. IV Ch. 7 §7.1), rules and domain-selection loading
(Vol. IV Ch. 2), generation orchestration (the layer model, Vol. IV Ch. 4), and
scenario binding (Vol. IV Ch. 5).

**Law:** Layer 3 coherence mechanically enforces the ownership matrix (Appendix A) and
composition coverage; missing rules are failures, never defaults (Vol. IV Ch. 7 §7.5.4–5);
the sealed runtime form is what saves pin (Vol. IV Ch. 3 §3.3.3). The POC's registry
(archive/poc-v0.1/engine/core/registry.py) is this service's validated ancestor —
evidence, not precedent.
