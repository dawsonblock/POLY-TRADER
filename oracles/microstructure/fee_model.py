"""
Tier 6: Fee & Net EV Model
Computes all-in cost of a round trip trade, including fees, funding, and spreads.
"""

from dataclasses import dataclass


@dataclass
class FeeSchedule:
    # Binance futures (price-taking side)
    binance_taker_bps: float = 4.0  # 4bps taker
    binance_maker_bps: float = 2.0  # 2bps maker rebate (negative cost)

    # Polymarket CTF exchange
    poly_taker_bps: float = 20.0  # 20bps taker (typical binary market)
    poly_maker_bps: float = 5.0  # 5bps maker

    # Gas / settlement (Polygon L2)
    settlement_usdc: float = 0.10  # ~$0.10 per order on Polygon


def round_trip_cost(
    order_size: float,
    fee_schedule: FeeSchedule,
    binance_side: str = "taker",  # "taker" or "maker"
    poly_side: str = "taker",
) -> dict:
    """
    All-in cost for one round-trip arbitrage: Binance entry + Polymarket exit.

    Returns:
        cost_bps: Total cost in basis points
        cost_dollars: Total dollar cost
        break_even_edge_bps: Minimum edge required to be profitable
    """
    binance_bps = (
        fee_schedule.binance_taker_bps
        if binance_side == "taker"
        else -fee_schedule.binance_maker_bps
    )
    poly_bps = (
        fee_schedule.poly_taker_bps
        if poly_side == "taker"
        else fee_schedule.poly_maker_bps
    )

    total_bps = binance_bps + poly_bps
    cost_dollars = (total_bps / 10_000) * order_size + fee_schedule.settlement_usdc

    break_even = total_bps  # Must have > total_bps edge to profit

    return {
        "binance_bps": binance_bps,
        "poly_bps": poly_bps,
        "total_cost_bps": total_bps,
        "cost_dollars": cost_dollars,
        "break_even_edge_bps": break_even,
    }


if __name__ == "__main__":
    schedule = FeeSchedule()
    result = round_trip_cost(5_000, schedule, binance_side="taker", poly_side="taker")
    print("All-in round trip cost for $5,000:")
    print(f"  Binance:  {result['binance_bps']:.1f} bps")
    print(f"  Poly:     {result['poly_bps']:.1f} bps")
    print(
        f"  Total:    {result['total_cost_bps']:.1f} bps  (${result['cost_dollars']:.2f})"
    )
    print(f"  Break-even edge required: {result['break_even_edge_bps']:.1f} bps")
