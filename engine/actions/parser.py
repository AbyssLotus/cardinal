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

    if verb in ("hunt", "fight"):
        if not args:
            raise ParseError("Hunt what?")
        species = _resolve_monster(" ".join(args), registry)
        if species is None:
            raise ParseError(f"You know of no creature called {' '.join(args)!r}.")
        return [Action("hunt", target=species, raw_input=text)]

    if verb == "attack":
        # in combat: strike (optionally with a named technique);
        # out of combat: 'attack boar' means go hunt one — the loop decides.
        argument = " ".join(args) if args else None
        return [Action("attack", parameters={"argument": argument}, raw_input=text)]

    if verb == "guard":
        stance = args[0] if args else "dodge"
        if stance not in ("parry", "dodge", "block"):
            raise ParseError("Guard how? parry, dodge, or block.")
        return [Action("guard", parameters={"stance": stance}, raw_input=text)]

    if verb in ("flee", "run"):
        return [Action("flee", raw_input=text)]

    if verb == "equip":
        if not args:
            raise ParseError("Equip what?")
        return [Action("equip", parameters={"name": " ".join(args)}, raw_input=text)]

    if verb == "skills":
        return [Action("skills", raw_input=text)]

    if verb in ("use", "hack", "pick", "open", "press", "pull"):
        if not args:
            raise ParseError(f"{verb.capitalize()} what?")
        return [Action("interact",
                       parameters={"verb": verb, "name": " ".join(args)},
                       raw_input=text)]

    if verb in ("mount", "ride", "board", "drive"):
        if not args:
            raise ParseError("Mount what?")
        return [Action("mount", parameters={"name": " ".join(args)}, raw_input=text)]

    if verb in ("dismount", "unmount", "park"):
        return [Action("dismount", raw_input=text)]

    raise ParseError(
        f"You don't know how to {verb!r}. Try: look, status, skills, wait <min>, "
        "go <place>, hunt <creature>, attack [technique], guard <stance>, flee, "
        "equip <item>, use/hack/open <device>, mount <vehicle>, dismount.")


def _resolve_monster(name: str, registry: Registry) -> str | None:
    if registry.find(name) is not None:
        return name
    needle = name.lower().rstrip("s")
    for monster in registry.by_kind("mon"):
        if needle in getattr(monster, "name", "").lower():
            return monster.id
    return None


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
