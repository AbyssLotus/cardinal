"""Anthropic-backed prose renderer (§15). M5 implementation.

Prime directive holds: the narrator receives committed, perception-filtered
deltas and returns prose. It has no tool access and no ability to mutate
state. If the `anthropic` package is missing, no credentials resolve, or the
API errors, rendering silently degrades to the deterministic plain narrator —
LLM output is never a dependency of state computation.
"""

from __future__ import annotations

import json

from engine.narrator.base import Narrator, PerceptionContext
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.store import Delta

GM_SYSTEM_PROMPT = """\
You are the narrator of a persistent simulated world. You render committed
simulation results into second-person present-tense prose (2-6 sentences).
Inviolable rules:
- Describe ONLY the state changes and context you are given. Never invent
  events, items, NPCs, dialogue facts, or outcomes.
- Confirmed-canon facts in context are inviolable. If content is missing,
  say less - never improvise to fill the gap.
- Never reveal hidden information (NPC goals, unobserved stats, other
  regions' events).
- Never narrate the player's choices - only the results of their stated
  action. Always end awaiting input.
- No meta-talk, no mechanics-speak unless it is diegetic UI (HP, cursors,
  status readouts), no headers or bullet lists - flowing prose only.
"""


class LlmNarrator(Narrator):
    """Renders via the Anthropic API; falls back to PlainNarrator whenever
    the LLM path is unavailable, so the engine runs headless regardless."""

    def __init__(self, model: str = "claude-opus-4-8", max_tokens: int = 400):
        self.model = model
        self.max_tokens = max_tokens
        self._fallback = PlainNarrator()
        self._client = None
        self._disabled = False
        try:
            import anthropic  # noqa: F401 — optional dependency
            self._client = anthropic.Anthropic()
        except Exception:
            self._disabled = True

    def render(self, deltas: list[Delta], perception: PerceptionContext) -> str:
        if self._disabled or self._client is None:
            return self._fallback.render(deltas, perception)
        try:
            context = self._build_context(deltas, perception)
            response = self._client.messages.create(
                model=self.model,
                max_tokens=self.max_tokens,
                system=[{"type": "text", "text": GM_SYSTEM_PROMPT,
                         "cache_control": {"type": "ephemeral"}}],
                messages=[{"role": "user", "content": context}],
            )
            if response.stop_reason == "refusal":
                return self._fallback.render(deltas, perception)
            text = "".join(block.text for block in response.content
                           if block.type == "text").strip()
            return text or self._fallback.render(deltas, perception)
        except Exception:
            # one failure disables the LLM path for the session — a flaky
            # network must not add latency to every subsequent turn
            self._disabled = True
            return self._fallback.render(deltas, perception)

    def _build_context(self, deltas: list[Delta],
                       perception: PerceptionContext) -> str:
        """Everything the player can legitimately perceive, and nothing else."""
        payload = {
            "time": perception.clock_label,
            "location": perception.location_snapshot,
            "present_npcs": perception.present_npcs,
            "events_this_turn": [
                {"kind": d.kind, **d.payload} for d in deltas
            ],
        }
        if perception.player_status:
            payload["player_status"] = perception.player_status
        return ("Narrate this turn to the player.\n\n"
                + json.dumps(payload, indent=2, default=str))
