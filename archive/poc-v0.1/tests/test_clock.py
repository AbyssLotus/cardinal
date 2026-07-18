from engine.core.clock import TickBoundary, WorldClock


def test_advance_within_hour():
    clock = WorldClock(day=0, minute=0)
    assert clock.advance(30) == []
    assert clock.stamp() == (0, 30)


def test_advance_crosses_hours_and_days():
    clock = WorldClock(day=0, minute=23 * 60 + 30)  # 23:30
    boundaries = clock.advance(120)  # -> day 1, 01:30
    assert boundaries == [
        TickBoundary("day", 1, 0),
        TickBoundary("hour", 1, 1),
    ]
    assert clock.stamp() == (1, 90)


def test_label():
    clock = WorldClock(day=3, minute=13 * 60 + 5)
    assert clock.label() == "Day 3, 13:05"
