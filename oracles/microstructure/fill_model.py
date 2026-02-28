"""
Tier 6: Fill Probability Model
Monte Carlo estimate of fill probability given latency and queue position.
Binary option markets are highly queue-sensitive.
"""

import random
from dataclasses import dataclass


@dataclass
class FillParams:
    queue_depth_ahead: float  # Total size ahead of us in queue (USD)
    order_size: float  # Our order size (USD)
    latency_ms: float  # Our estimated execution latency (ms)
    arrival_rate: float  # Market arrival rate: orders per ms
    avg_trade_size: float  # Average trade size consuming queue (USD)
    time_to_expiry_s: float  # Binary option time to expiry (seconds)


def simulate_fill_probability(params: FillParams, n_simulations: int = 10_000) -> dict:
    """
    Monte Carlo simulation of fill probability.

    Models queue as a Poisson process: orders arrive and consume depth ahead.
    We fill if queue ahead of us is consumed before order expires or we cancel.

    Returns:
        Dictionary with fill probability statistics.
    """
    filled = 0

    for _ in range(n_simulations):
        remaining_queue = params.queue_depth_ahead
        time_elapsed_ms = params.latency_ms  # Start after our latency

        while time_elapsed_ms < params.time_to_expiry_s * 1000:
            # Time until next market order (Exponential distribution)
            inter_arrival = random.expovariate(params.arrival_rate)
            time_elapsed_ms += inter_arrival

            if time_elapsed_ms >= params.time_to_expiry_s * 1000:
                break  # Expired

            # Trade size (lognormal around avg)
            trade_size = random.lognormvariate(mu=0, sigma=0.5) * params.avg_trade_size
            remaining_queue -= trade_size

            if remaining_queue <= 0:
                filled += 1
                break

    fill_prob = filled / n_simulations
    return {
        "fill_probability": fill_prob,
        "expected_fill_ratio": min(
            1.0, params.order_size / max(params.queue_depth_ahead, 1)
        ),
        "n_simulations": n_simulations,
    }


def net_ev(
    gross_ev: float,
    fill_probability: float,
    slippage_fraction: float,
    taker_fee_fraction: float,
    order_size: float,
) -> float:
    """
    Compute net EV after all friction.

    net_ev = fill_prob * (gross_ev - slippage - fees) - (1 - fill_prob) * opportunity_cost
    """
    friction = slippage_fraction + taker_fee_fraction
    filled_ev = fill_probability * (gross_ev - friction) * order_size
    # Opportunity cost when we don't fill: assume 0 (no capital at risk)
    return filled_ev


if __name__ == "__main__":
    params = FillParams(
        queue_depth_ahead=25_000,  # $25k ahead of us
        order_size=5_000,  # $5k order
        latency_ms=12.0,  # 12ms total latency
        arrival_rate=0.1,  # 1 trade per 10ms
        avg_trade_size=1_000,  # Average $1k trade
        time_to_expiry_s=300,  # 5 minutes
    )

    result = simulate_fill_probability(params)
    print(f"Fill probability: {result['fill_probability']*100:.1f}%")

    net = net_ev(
        gross_ev=0.03,  # 3% theoretical edge
        fill_probability=result["fill_probability"],
        slippage_fraction=0.0015,  # 15bps
        taker_fee_fraction=0.002,  # 20bps taker fee
        order_size=5_000,
    )
    print(f"Net EV on $5,000 order: ${net:.2f}")
