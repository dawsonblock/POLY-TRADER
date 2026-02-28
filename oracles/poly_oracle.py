"""
POLYMARKET ORACLE (The Brain)
Listens to Binance. Prices the Polymarket contract. Signals Rust.
"""

import asyncio
import json
import websockets
import zmq
import os
from dotenv import load_dotenv
import math
import time
from scipy.stats import norm

load_dotenv()

# --- CONFIGURATION ---
BINANCE_WS = os.getenv("BINANCE_WS", "wss://fstream.binance.com/ws/btcusdt@bookTicker")
ZMQ_ADDR = os.getenv("ZMQ_ADDR", "tcp://127.0.0.1:5555")

# Target Polymarket Contract: "Bitcoin > $110k by March 15"
STRIKE_PRICE = 110000.00
TIME_TO_EXPIRY_DAYS = 15.0
IMPLIED_VOLATILITY = 0.55  # 55% annualized vol


class PolyBrain:
    def __init__(self):
        self.context = zmq.Context()
        self.socket = self.context.socket(zmq.PUSH)
        self.socket.bind(ZMQ_ADDR)
        print("[ORACLE] ZMQ IPC Bridge Bound.")

    def calculate_fair_value(self, current_price):
        """
        Prices a Binary Call Option using standard quantitative finance math.
        Returns the theoretical probability (0.00 to 1.00).
        """
        time_in_years = TIME_TO_EXPIRY_DAYS / 365.0

        # Prevent division by zero if expiring today
        if time_in_years <= 0.0001:
            return 1.0 if current_price > STRIKE_PRICE else 0.0

        # Standard d2 calculation for binary options
        d2 = (
            math.log(current_price / STRIKE_PRICE)
            - (0.5 * IMPLIED_VOLATILITY**2) * time_in_years
        ) / (IMPLIED_VOLATILITY * math.sqrt(time_in_years))

        fair_value = norm.cdf(d2)
        return fair_value

    async def run(self):
        print(f"[ORACLE] Listening to Binance. Target Strike: ${STRIKE_PRICE}")

        async with websockets.connect(BINANCE_WS) as ws:
            while True:
                msg = await ws.recv()
                data = json.loads(msg)

                # Best Bid/Ask midpoint on Binance Futures
                bid = float(data["b"])
                ask = float(data["a"])
                mid_price = (bid + ask) / 2.0

                # Calculate True Probability
                fair_value = self.calculate_fair_value(mid_price)

                # Create Signal Payload
                # In production, you also track Polymarket's current order book here.
                # If fair_value is 0.65, and Poly is at 0.55, we fire.

                signal = {
                    "timestamp": time.time(),
                    "asset": "BTC",
                    "binance_price": mid_price,
                    "poly_fair_value": round(fair_value, 4),
                    "action": "EVALUATE",
                }

                # Fire and forget to Rust over ZeroMQ (sub-50us latency)
                self.socket.send_json(signal)


if __name__ == "__main__":
    brain = PolyBrain()
    asyncio.get_event_loop().run_until_complete(brain.run())
