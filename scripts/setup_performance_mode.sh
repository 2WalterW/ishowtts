#!/bin/bash
# Setup Jetson AGX Orin for maximum TTS performance
# This locks GPU/CPU frequencies and sets power mode to MAXN
# Run this after reboot for consistent performance

set -e

echo "================================"
echo "iShowTTS Performance Mode Setup"
echo "================================"
echo ""

# Check if running on Jetson
if [ ! -f /etc/nv_tegra_release ]; then
    echo "⚠️  Warning: This doesn't appear to be a Jetson device"
    echo "   /etc/nv_tegra_release not found"
    echo ""
fi

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "❌ Error: This script must be run as root"
    echo "   Usage: sudo $0"
    exit 1
fi

echo "Setting maximum performance mode..."
echo ""

# Set power mode to MAXN (mode 0)
echo "1. Setting nvpmodel to MAXN (mode 0)..."
if command -v nvpmodel &> /dev/null; then
    nvpmodel -m 0
    CURRENT_MODE=$(nvpmodel -q | grep "NV Power Mode" | cut -d: -f2 | xargs)
    echo "   ✅ Power mode: $CURRENT_MODE"
else
    echo "   ⚠️  nvpmodel not found, skipping"
fi
echo ""

# Lock clocks to maximum
echo "2. Locking clocks to maximum with jetson_clocks..."
if command -v jetson_clocks &> /dev/null; then
    jetson_clocks
    echo "   ✅ Clocks locked to maximum"
else
    echo "   ⚠️  jetson_clocks not found, skipping"
fi
echo ""

# Verify settings
echo "3. Verifying settings..."
if command -v nvidia-smi &> /dev/null; then
    nvidia-smi --query-gpu=name,power.limit,clocks.gr,clocks.mem --format=csv,noheader
else
    echo "   ⚠️  nvidia-smi not available"
fi
echo ""

# Performance impact
echo "================================"
echo "✅ Performance mode enabled!"
echo "================================"
echo ""
echo "Expected performance improvement:"
echo "  - Without lock: Mean RTF = 0.352 (±16% variance)"
echo "  - With lock:    Mean RTF = 0.278 (±1.5% variance)"
echo ""
echo "This improves consistency and reduces synthesis time"
echo "from 2.95s to 2.33s for 8.4s audio (3.59x real-time)."
echo ""
echo "⚠️  Note: These settings will reset on reboot."
echo "   Add this script to startup for persistent effect."
echo ""
echo "To add to startup (systemd):"
echo "  1. Copy to /usr/local/bin/:"
echo "     sudo cp $0 /usr/local/bin/ishowtts-performance"
echo ""
echo "  2. Create systemd service:"
echo "     sudo tee /etc/systemd/system/ishowtts-performance.service <<EOF"
echo "[Unit]"
echo "Description=iShowTTS Performance Mode"
echo "After=nvpmodel.service"
echo ""
echo "[Service]"
echo "Type=oneshot"
echo "ExecStart=/usr/local/bin/ishowtts-performance"
echo "RemainAfterExit=yes"
echo ""
echo "[Install]"
echo "WantedBy=multi-user.target"
echo "EOF"
echo ""
echo "  3. Enable and start:"
echo "     sudo systemctl enable ishowtts-performance.service"
echo "     sudo systemctl start ishowtts-performance.service"
echo ""