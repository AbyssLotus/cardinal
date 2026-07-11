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
