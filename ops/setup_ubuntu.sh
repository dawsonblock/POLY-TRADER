#!/bin/bash
set -e

echo "[SETUP] Booting Polymarket Sniper Environment (AWS Ubuntu / Debian)"

# 1. Update and basic utilities
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential curl pkg-config libssl-dev cmake python3-pip python3-venv tmux htop screen git libzmq3-dev

# 2. Network Optimization (TCP Kernel Tuning for Low Latency)
echo "[SETUP] Tuning Kernel for TCP Low Latency..."
sudo bash -c 'cat << EOF > /etc/sysctl.d/99-poly.conf
net.core.rmem_max = 16777216
net.core.wmem_max = 16777216
net.ipv4.tcp_rmem = 4096 87380 16777216
net.ipv4.tcp_wmem = 4096 65536 16777216
net.core.netdev_max_backlog = 5000
net.ipv4.tcp_window_scaling = 1
net.ipv4.tcp_timestamps = 0
net.ipv4.tcp_sack = 1
EOF'
sudo sysctl -p /etc/sysctl.d/99-poly.conf

# 3. Python Environment (Optional for Microstructure Models)
echo "[SETUP] Installing Python Dependencies..."
cd /opt/poly-trader
python3 -m venv venv
source venv/bin/activate
pip install websockets scipy python-dotenv

# 4. Rust Toolchain
echo "[SETUP] Installing Rustup..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"

# 5. Build DTK v2
echo "[SETUP] Compiling DTK v2 for Release..."
cd /opt/poly-trader/execution/dtk
cargo build --release

# 6. Build Replay Verifier
echo "[SETUP] Compiling Replay Verifier..."
cd /opt/poly-trader/execution/replay
cargo build --release

# 7. Install Systemd Services
echo "[SETUP] Installing Systemd Services..."
sudo cp /opt/poly-trader/ops/poly-sniper.service /etc/systemd/system/dtk.service

sudo systemctl daemon-reload
sudo systemctl enable dtk

echo "[SETUP] COMPLETE. Start the kernel with: 'sudo systemctl start dtk'"
