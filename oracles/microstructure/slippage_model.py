"""
Tier 6: Microstructure Slippage Model
Estimates market impact (slippage) from real L2 order book depth.
"""

from dataclasses import dataclass
from typing import List, Tuple


@dataclass
class L2Level:
    price: float
    size: float


def parse_depth_snapshot(raw: dict) -> Tuple[List[L2Level], List[L2Level]]:
    """Parse Binance depth snapshot into bid/ask lists."""
    bids = [L2Level(float(p), float(s)) for p, s in raw.get("bids", [])]
    asks = [L2Level(float(p), float(s)) for p, s in raw.get("asks", [])]
    return bids, asks


def compute_slippage(side: str, order_size: float, levels: List[L2Level]) -> float:
    """
    Estimate average fill price vs best price for a market order of `order_size`.

    Args:
        side:       'buy' or 'sell'
        order_size: Notional size of order in USD
        levels:     Sorted L2 levels (asks for buy, bids for sell)

    Returns:
        Slippage as fraction of best price (e.g. 0.001 = 10bps)
    """
    if not levels:
        return float("inf")

    best_price = levels[0].price
    remaining = order_size
    weighted_sum = 0.0
    total_filled = 0.0

    for level in levels:
        available = level.price * level.size
        fill_here = min(remaining, available)
        weighted_sum += fill_here * level.price
        total_filled += fill_here
        remaining -= fill_here
        if remaining <= 0:
            break

    if total_filled == 0:
        return float("inf")

    avg_fill_price = weighted_sum / order_size
    slippage = abs(avg_fill_price - best_price) / best_price
    return slippage


def compute_volatility_adjusted_slippage(
    side: str,
    order_size: float,
    levels: List[L2Level],
    realized_vol: float,  # Annualized, e.g. 0.80 = 80%
    tau_minutes: float,  # Horizon in minutes
) -> float:
    """
    Apply volatility-adjusted spread widening to base slippage.
    In high-vol regimes, market makers widen spreads by ~sqrt(vol * tau).
    """
    base_slippage = compute_slippage(side, order_size, levels)
    # Spread widening factor increases with vol * sqrt(tau)
    vol_per_minute = realized_vol / (365 * 24 * 60) ** 0.5
    spread_widening = vol_per_minute * (tau_minutes**0.5)
    return base_slippage + spread_widening


if __name__ == "__main__":
    # Example with synthetic L2 book
    example_asks = [
        L2Level(110_000, 0.5),
        L2Level(110_050, 1.0),
        L2Level(110_200, 2.0),
    ]
    size = 50_000  # $50k order
    slip = compute_slippage("buy", size, example_asks)
    print(f"Base slippage for ${size:,} buy: {slip*100:.4f}%")
    adj_slip = compute_volatility_adjusted_slippage(
        "buy", size, example_asks, 0.80, 5.0
    )
    print(f"Vol-adjusted slippage (80% ann vol, 5min horizon): {adj_slip*100:.4f}%")
