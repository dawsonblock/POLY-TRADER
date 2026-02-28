import numpy as np

def simulate_day(p_edge_mean=0.01,
                 fee=0.002,
                 slippage=0.003,
                 latency_penalty=0.001,
                 trades=100):

    pnl = 0.0

    for _ in range(trades):
        edge = np.random.normal(p_edge_mean, 0.01)
        ev = edge - (fee + slippage + latency_penalty)

        if ev > 0:
            pnl += ev

    return pnl

def run_mc(days=10000):
    print(f"Running Monte Carlo simulation for {days} days...")
    print(f"Assumptions: Fee: 0.2%, Slippage: 0.3%, Latency Penalty: 0.1%")
    results = [simulate_day() for _ in range(days)]
    mean_pnl = np.mean(results)
    std_pnl = np.std(results)
    print(f"--- Results ---")
    print(f"Mean Daily Return: {mean_pnl:.6f}")
    print(f"Daily Std Dev: {std_pnl:.6f}")
    
    # Calculate Sharpe ratio (assuming 0 risk-free rate for daily)
    sharpe = (mean_pnl / std_pnl) * np.sqrt(365) if std_pnl > 0 else 0
    print(f"Estimated Annualized Sharpe Ratio: {sharpe:.2f}")
    return mean_pnl, std_pnl

if __name__ == "__main__":
    run_mc()
