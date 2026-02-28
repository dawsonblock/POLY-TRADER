#!/bin/bash
set -e

echo "[SETUP] Booting APEX AI Oracle Environment (AWS H100 / Ubuntu)"

# 1. Update and basic utilities
sudo apt update && sudo apt upgrade -y
sudo apt install -y build-essential python3-pip python3-venv pciutils linux-headers-$(uname -r)

# 2. Python Environment & CUDA Dependencies
echo "[SETUP] Installing PyTorch & Mamba-2..."
cd /opt/poly-trader
python3 -m venv venv_ai
source venv_ai/bin/activate
pip install python-dotenv

# Install PyTorch with CUDA 12.1 support
pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu121

# Install State Space Model dependencies (Requires NVCC and CUDA toolkit)
pip install causal-conv1d>=1.2.0
pip install mamba-ssm

# 3. Install Systemd Service
echo "[SETUP] Installing APEX Oracle Systemd Service..."
sudo cp /opt/poly-trader/ops/apex-oracle.service /etc/systemd/system/

sudo systemctl daemon-reload
sudo systemctl enable apex-oracle

echo "[SETUP] COMPLETE. Ensure XDMA drivers are loaded for PCIe '/dev/xdma0_h2c_0' before starting."
echo "Start service with: 'sudo systemctl start apex-oracle'"
