"""M4: markets, trading, quest completion, rumor propagation."""

from engine.core.loop import SimulationLoop
from engine.narrator.plain_narrator import PlainNarrator
from engine.persistence.saves import create_save
from engine.systems.combat import get_encounter


def _loop(save):
    return SimulationLoop(save.registry, save.store, save.rng, PlainNarrator())


def _hunt_until_goo(loop, save, tries=12):
    for _ in range(tries):
        loop.submit("hunt slime")
        for _ in range(200):
            loop.submit("attack")
            if get_encounter(loop.ctx) is None:
                break
        if any(i["def_id"] == "item.tw_goo"
               for i in save.store.get_inventory("player")):
            return True
        loop.submit("wait 600")
    return False


def test_markets_price_goods_daily(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=3)
    loop = _loop(save)
    loop.advance_days(1)
    rows = save.store.get_market("market.tw_hub")
    assert rows                                        # market seeded on first tick
    ration = save.store.get_market_row("market.tw_hub", "item.tw_ration")
    assert ration is not None and abs(ration["price"] - 8) < 1  # near base at equilibrium
    # vehicles are stocked goods too
    assert save.store.get_market_row("market.tw_hub", "vehicle.tw_mule") is not None
    save.store.close()


def test_buy_and_sell_move_col_and_market(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=3)
    loop = _loop(save)
    loop.advance_days(1)
    result = loop.submit("shop")
    assert "Ration" in result.text

    before = save.store.get_player()["col"]
    result = loop.submit("buy ration 2")
    assert "Bought 2x Ration" in result.text
    player = save.store.get_player()
    assert player["col"] < before
    rations = sum(i["qty"] for i in save.store.get_inventory("player")
                  if i["def_id"] == "item.tw_ration")
    assert rations == 5                                # 3 starting + 2 bought

    # dumping stock floods supply: price falls after the next day tick
    price_before = save.store.get_market_row("market.tw_hub", "item.tw_ration")["price"]
    loop.submit("sell ration 5")
    loop.advance_days(1)
    price_after = save.store.get_market_row("market.tw_hub", "item.tw_ration")["price"]
    assert price_after < price_before
    save.store.close()


def test_buy_a_vehicle_for_real(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=3)
    loop = _loop(save)
    loop.advance_days(1)
    result = loop.submit("buy mule")
    assert "Bought 1x Pack Mule" in result.text
    assert save.store.get_player()["col"] <= 200       # 500 minus ~300
    owned = [dict(r) for r in save.store.conn.execute(
        "SELECT * FROM entities WHERE kind='vehicle'")]
    assert len(owned) == 2                             # starting autocart + the mule
    result = loop.submit("mount mule")
    assert "You mount the Pack Mule" in result.text
    save.store.close()


def test_insufficient_funds(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=3)
    loop = _loop(save)
    loop.advance_days(1)
    result = loop.submit("buy autocart")               # 2500 vs 500 col
    assert "you carry" in result.text
    save.store.close()


def test_crashed_market_cannot_be_bought_for_free(tmp_path, testworld_path):
    """Regression test for the Test 3 playtest finding: dumping stock to
    crash a market's price below 1 col used to let `int(price * qty)`
    truncate every subsequent buy to 0 col, making goods free and — via
    an untouched second market — a source of infinite profit.
    """
    save = create_save(tmp_path / "s1", testworld_path, seed=3)
    loop = _loop(save)
    loop.advance_days(1)

    # Simulate a crashed market directly (mirrors the report's live method
    # of dumping ~20,000 units in one sale to force the index down).
    save.store.upsert_market_row("market.tw_hub", "item.tw_ration",
                                 supply_idx=50.0, demand_idx=1.0, price=0.4)

    before = save.store.get_player()["col"]
    result = loop.submit("buy ration 1")
    assert "Bought 1x Ration for 0 " not in result.text   # never free
    after = save.store.get_player()["col"]
    assert after < before                                 # col actually spent
    assert before - after >= 1                             # floored at 1 col/unit

    # Buying 60 units one at a time (as the live exploit did) must cost at
    # least 1 col each — no infinite free stock to launder elsewhere.
    before = save.store.get_player()["col"]
    for _ in range(60):
        loop.submit("buy ration 1")
    spent = before - save.store.get_player()["col"]
    assert spent >= 60
    save.store.close()


def test_large_dump_has_intra_trade_slippage(tmp_path, testworld_path):
    """A single massive sell shouldn't price its entire quantity at one
    flat pre-trade quote — later units in the same trade should earn
    less than earlier ones as the price drifts (Test 3 compounding
    factor: 'zero within-trade slippage').
    """
    save = create_save(tmp_path / "s1", testworld_path, seed=3)
    loop = _loop(save)
    loop.advance_days(1)
    save.store.add_item_instance("iteminst.dumptest", "item.tw_ration", "player", qty=2000)

    before = save.store.get_player()["col"]
    loop.submit("sell ration 2000")
    earned = save.store.get_player()["col"] - before

    row = save.store.get_market_row("market.tw_hub", "item.tw_ration")
    flat_quote_earnings = row["price"] * 0.6 * 2000
    assert earned < flat_quote_earnings                    # drifted down, not flat
    save.store.close()


def test_talk_lists_quest_needs(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=5)
    loop = _loop(save)
    loop.advance_days(1)                               # quest becomes available
    loop.submit("go far village")
    result = loop.submit("talk dan")
    assert "parcel must reach Hub Town" in result.text
    assert "needs 1x Slime Goo" in result.text
    save.store.close()


def test_quest_completion_end_to_end(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=5)
    loop = _loop(save)
    loop.advance_days(1)                               # quest goes available
    assert _hunt_until_goo(loop, save)                 # earn the goo in the field
    loop.submit("go far village")
    result = loop.submit("give goo")
    assert "Dan gives you 2x Ration" in result.text
    assert "presses 25" in result.text                 # col reward paid out
    assert "won't forget it" in result.text            # authored success outcome
    assert save.store.get_player()["col"] == 525       # 500 start + 25 reward

    quest = next(q for q in save.store.get_quests()
                 if q["def_id"] == "quest.tw_errand")
    assert quest["state"] == "completed"
    assert save.store.get_reputation("npc.tw_dan") > 0
    memories = save.store.conn.execute(
        "SELECT * FROM npc_memory WHERE npc_id='npc.tw_dan' AND kind='assistance'"
    ).fetchall()
    assert memories and memories[0]["decays"] == 0     # he'll never forget
    headlines = [e["headline"] for e in save.store.get_chronicle(50)]
    assert any("won't forget it" in h for h in headlines)

    # talking now shows warmth, and expiry never fires
    result = loop.submit("talk dan")
    assert "friendly" in result.text or "warm" in result.text
    loop.advance_days(5)
    quest = next(q for q in save.store.get_quests()
                 if q["def_id"] == "quest.tw_errand")
    assert quest["state"] == "completed"               # terminal states stay terminal
    assert save.store.get_entity("npc.tw_dan")["state"].get("mood") != "glum"
    save.store.close()


def test_rumors_spread_the_players_name(tmp_path, testworld_path):
    save = create_save(tmp_path / "s1", testworld_path, seed=5)
    loop = _loop(save)
    loop.advance_days(1)
    assert _hunt_until_goo(loop, save)
    loop.submit("go far village")
    loop.submit("give goo")
    loop.advance_days(6)                               # evenings together; word travels
    rumors = save.store.conn.execute(
        "SELECT * FROM npc_memory WHERE npc_id='npc.tw_eve' AND kind='rumor'"
    ).fetchall()
    assert rumors                                      # Eve heard about the deed
    assert rumors[0]["subject_id"] == "player"
    assert rumors[0]["rumor_certainty"] is not None
    assert rumors[0]["rumor_certainty"] < 1.0          # secondhand, less certain
    save.store.close()
