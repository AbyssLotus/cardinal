"""Deterministic text renderer — the test/headless narrator.

Same deltas + same perception ⇒ byte-identical output. No randomness,
no LLM, no state access beyond what it is handed.
"""

from __future__ import annotations

from engine.narrator.base import Narrator, PerceptionContext
from engine.persistence.store import Delta


class PlainNarrator(Narrator):
    def render(self, deltas: list[Delta], perception: PerceptionContext) -> str:
        lines: list[str] = [f"[{perception.clock_label}]"]

        for delta in deltas:
            line = self._describe(delta)
            if line:
                lines.append(line)

        snapshot = perception.location_snapshot
        if snapshot:
            lines.append(f"Location: {snapshot.get('name', snapshot.get('id', '?'))}")
            if snapshot.get("safe_zone"):
                lines.append("  (safe zone)")
            services = snapshot.get("services") or []
            if services:
                lines.append(f"  Services: {', '.join(services)}")

        if perception.present_npcs:
            names = ", ".join(npc["name"] for npc in perception.present_npcs)
            lines.append(f"Present: {names}")

        status = perception.player_status
        if status:
            lines.append(
                f"HP {status.get('hp')}/{status.get('hp_max')} | "
                f"Lv {status.get('level')} | {status.get('col')} col"
            )

        return "\n".join(lines)

    def _describe(self, delta: Delta) -> str | None:
        if delta.kind == "chronicle":
            return f"* {delta.payload.get('headline', '')}"
        if delta.kind == "player_history":
            return delta.payload.get("summary", "")
        return None
