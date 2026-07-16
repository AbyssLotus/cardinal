"""Supply/demand pricing per settlement market (§11). M4 implementation.

Day-tick, per (market, stocked good):

    price = base_value × (demand_idx / supply_idx) ^ price_elasticity

Player trades push the indices (buying raises demand and eats supply; dumping
loot floods supply); every index mean-reverts toward 1.0 at the merchant
restock rate — the abstract form of merchants hauling goods back in by wagon.
Prices visibly move the day after you move the market.

A market stocks a good when the good's category starts with any of the
market's `stock_categories` (vehicles included — a market listing `ground`
or `mount` sells vehicles).
"""

from __future__ import annotations

from engine.persistence.store import Delta
from engine.systems import SystemContext

MIN_INDEX = 0.05


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    if granularity != "day":
        return []
    rule = ctx.registry.rule
    elasticity = rule("economy.price_elasticity", 0.35)
    restock_days = max(1, rule("economy.merchant_restock_days", 3))
    deltas: list[Delta] = []

    for market in sorted(ctx.registry.by_kind("market"), key=lambda m: m.id):
        for good in stocked_goods(ctx, market):
            row = ctx.store.get_market_row(market.id, good.id)
            supply = row["supply_idx"] if row else 1.0
            demand = row["demand_idx"] if row else 1.0
            # mean reversion: merchants restock shortages and clear gluts
            supply = max(MIN_INDEX, supply + (1.0 - supply) / restock_days)
            demand = max(MIN_INDEX, demand + (1.0 - demand) / restock_days)
            price = good.market.base_value_col * (demand / supply) ** elasticity
            deltas.append(Delta(kind="market_update", payload={
                "market_id": market.id, "item_def": good.id,
                "supply_idx": round(supply, 4), "demand_idx": round(demand, 4),
                "price": round(price, 2)}))
    return deltas


def stocked_goods(ctx: SystemContext, market) -> list:
    goods = []
    for kind in ("item", "vehicle"):
        for good in sorted(ctx.registry.by_kind(kind), key=lambda g: g.id):
            if good.market.base_value_col <= 0:
                continue
            if any(good.category.startswith(c) for c in market.stock_categories):
                goods.append(good)
    return goods


def current_price(ctx: SystemContext, market_id: str, good) -> float:
    row = ctx.store.get_market_row(market_id, good.id)
    return row["price"] if row else float(good.market.base_value_col)


def trade_cost(ctx: SystemContext, market_id: str, good, qty: int,
               player_buys: bool) -> tuple[int, int]:
    """Quote the total col for a trade of `qty` units.

    Closes two issues from the Test 3 playtest finding:

    - **Truncation-to-free.** A flat ``int(price * qty)`` lets any
      per-unit price under 1.0 col round the whole purchase down to 0.
      Every non-zero-priced unit is floored at 1 col here, so a crashed
      market can never hand out goods for free.
    - **Atomic market-crash / no intra-trade slippage.** Previously the
      entire quantity of a single buy or sell command was priced at one
      flat pre-trade price. This walks a *local* (unpersisted) copy of
      the market's supply/demand indices through the trade in chunks,
      so a single massive dump or buy drifts the price it pays instead
      of clearing at one stale quote. The real persisted indices are
      still updated exactly once by `record_trade()`, same as before —
      this function only affects what the trade costs, not the
      day-tick pricing model.

    Returns (filled_qty, total_col). Pure function: does not mutate
    market state.
    """
    if qty <= 0:
        return 0, 0
    rule = ctx.registry.rule
    elasticity = rule("economy.price_elasticity", 0.35)
    ratio = 1.0 if player_buys else rule("economy.merchant_buy_ratio", 0.6)
    row = ctx.store.get_market_row(market_id, good.id)
    supply = row["supply_idx"] if row else 1.0
    demand = row["demand_idx"] if row else 1.0
    base_value = good.market.base_value_col
    volatility = good.market.volatility

    n_chunks = min(qty, 20)
    chunk_base = qty // n_chunks
    chunk_extra = qty % n_chunks
    total = 0
    for i in range(n_chunks):
        chunk_qty = chunk_base + (1 if i < chunk_extra else 0)
        if chunk_qty <= 0:
            continue
        price = base_value * (demand / max(MIN_INDEX, supply)) ** elasticity
        unit_price = price * ratio
        chunk_cost = round(unit_price * chunk_qty)
        if unit_price > 0:
            chunk_cost = max(chunk_cost, chunk_qty)  # floor: 1 col per unit
        total += chunk_cost
        swing = 0.05 * chunk_qty * (1.0 + volatility)
        if player_buys:
            demand += swing
            supply = max(MIN_INDEX, supply - swing * 0.5)
        else:
            supply += swing
            demand = max(MIN_INDEX, demand - swing * 0.25)
    return qty, total


def record_trade(ctx: SystemContext, market_id: str, good, qty: int, player_buys: bool) -> None:
    """A trade is just a supply/demand input; the day tick reprices."""
    row = ctx.store.get_market_row(market_id, good.id)
    supply = row["supply_idx"] if row else 1.0
    demand = row["demand_idx"] if row else 1.0
    price = row["price"] if row else float(good.market.base_value_col)
    swing = 0.05 * qty * (1.0 + good.market.volatility)
    if player_buys:
        demand += swing
        supply = max(MIN_INDEX, supply - swing * 0.5)
    else:
        supply += swing
        demand = max(MIN_INDEX, demand - swing * 0.25)
    ctx.store.upsert_market_row(market_id, good.id, round(supply, 4),
                                round(demand, 4), price)
