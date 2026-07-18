"""Seeded RNG substreams (§2).

Each named subsystem draws from its own `random.Random` stream derived from
the save seed, so adding draws to one system never perturbs another —
a requirement for golden-test reproducibility. Stream states are persisted
in the `rng_streams` table and restored on load.
"""

from __future__ import annotations

import hashlib
import json
import random


def _derive_seed(seed: int, name: str) -> int:
    digest = hashlib.sha256(f"{seed}:{name}".encode()).digest()
    return int.from_bytes(digest[:8], "big")


class RngManager:
    def __init__(self, seed: int):
        self.seed = seed
        self._streams: dict[str, random.Random] = {}

    def stream(self, name: str) -> random.Random:
        if name not in self._streams:
            self._streams[name] = random.Random(_derive_seed(self.seed, name))
        return self._streams[name]

    # --- persistence -----------------------------------------------------

    def dump_states(self) -> dict[str, str]:
        return {
            name: json.dumps(rng.getstate(), default=list)
            for name, rng in self._streams.items()
        }

    def load_states(self, states: dict[str, str]) -> None:
        for name, raw in states.items():
            version, internal, gauss = json.loads(raw)
            rng = random.Random()
            rng.setstate((version, tuple(internal), gauss))
            self._streams[name] = rng
