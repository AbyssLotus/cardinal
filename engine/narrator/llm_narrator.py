"""Anthropic-backed prose renderer (§15). M5 milestone.

The interface is fixed now so the engine never grows a dependency on it:
render() receives committed, perception-filtered deltas and returns prose.
No tool access, no state mutation, no gap-filling — missing content is the
generator's job, not the narrator's.
"""

from __future__ import annotations

from engine.narrator.base import Narrator, PerceptionContext
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.store import Delta

GM_SYSTEM_PROMPT = """\
You are the narrator of a persistent simulated world. You render committed
simulation results into prose. Inviolable rules:
- Describe ONLY the state changes and context you are given. Never invent
  events, items, NPCs, dialogue facts, or outcomes.
- Confirmed-canon facts in context are inviolable. If content is missing,
  say less — never improvise to fill the gap.
- Never reveal hidden information (NPC goals, unobserved stats, other
  regions' events).
- Never narrate the player's choices — only the results of their stated
  action. Always end awaiting input.
- No meta-talk. No mechanics-speak unless it is diegetic UI (HP, cursors).
"""


class LlmNarrator(Narrator):
    """Falls back to the plain narrator until M5 wires the Anthropic client."""

    def __init__(self, model: str = "claude-sonnet-5"):
        self.model = model
        self._fallback = PlainNarrator()

    def render(self, deltas: list[Delta], perception: PerceptionContext) -> str:
        # M5: build a messages request from (deltas, perception) with
        # GM_SYSTEM_PROMPT and return the completion. Headless-safe until then.
        return self._fallback.render(deltas, perception)
