"""INTERPRET step: input text -> Action objects.

M1 scope: a structured command grammar. Natural-language interpretation
(LLM-assisted parsing to the same Action objects) arrives in M3 — the
Action contract stays identical either way.
"""

from __future__ import annotations

from engine.actions.actions import Action
from engine.core.registry import Registry


class ParseError(Exception):
    pass


def parse(text: str, registry: Registry) -> list[Action]:
    words = text.strip().lower().split()
    if not words:
        raise ParseError("Say what you do.")
    verb, args = words[0], words[1:]

    if verb in ("wait", "rest"):
        minutes = 60
        if args:
            try:
                minutes = int(args[0])
            except ValueError:
                raise ParseError(f"wait takes a number of minutes, not {args[0]!r}")
            if minutes <= 0:
                raise ParseError("time only moves forward")
        return [Action("wait", parameters={"minutes": minutes}, raw_input=text)]

    if verb in ("look", "l"):
        return [Action("look", raw_input=text)]

    if verb in ("status", "stats", "st"):
        return [Action("status", raw_input=text)]

    if verb in ("go", "travel", "goto"):
        if not args:
            raise ParseError("Go where?")
        target = _resolve_location(" ".join(args), registry)
        if target is None:
            raise ParseError(f"No place called {' '.join(args)!r} is known to you.")
        return [Action("travel", target=target, raw_input=text)]

    raise ParseError(f"You don't know how to {verb!r}. Try: look, status, wait <min>, go <place>.")


def _resolve_location(name: str, registry: Registry) -> str | None:
    """Match a location by id or (partial) name."""
    if registry.find(name) is not None:
        return name
    if registry.find(f"loc.{name.replace(' ', '_')}") is not None:
        return f"loc.{name.replace(' ', '_')}"
    needle = name.lower()
    for loc in registry.by_kind("loc"):
        if needle in getattr(loc, "name", "").lower():
            return loc.id
    return None
