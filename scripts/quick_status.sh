#!/bin/bash
# Quick status check for iShowTTS optimization
# Shows current configuration and performance state

set -e

echo "============================================================"
echo "iShowTTS Quick Status Check"
echo "============================================================"
echo ""

# Check GPU lock status
echo "📊 GPU Performance Lock:"
if command -v jetson_clocks &> /dev/null; then
    echo "  Status: $(sudo jetson_clocks --show 2>/dev/null | head -5 || echo 'Unable to check')"
    echo "  Power Mode: $(sudo nvpmodel -q 2>/dev/null | grep "NV Power Mode" || echo 'Unable to check')"
else
    echo "  Not running on Jetson (jetson_clocks not found)"
fi
echo ""

# Check NFE configuration
echo "⚙️  Current Configuration:"
if [ -f config/ishowtts.toml ]; then
    NFE=$(grep "default_nfe_step" config/ishowtts.toml | grep -v "#" | awk '{print $3}')
    echo "  NFE Steps: ${NFE:-'Not found'}"
else
    echo "  Config file not found!"
fi
echo ""

# Check Python environment
echo "🐍 Python Environment:"
if [ -d /opt/miniforge3/envs/ishowtts ]; then
    echo "  Environment: /opt/miniforge3/envs/ishowtts (✅ Found)"
    /opt/miniforge3/envs/ishowtts/bin/python --version 2>&1 | sed 's/^/  /'
    /opt/miniforge3/envs/ishowtts/bin/python -c "import torch; print(f'  PyTorch: {torch.__version__}'); print(f'  CUDA Available: {torch.cuda.is_available()}')" 2>&1
else
    echo "  Environment not found at expected location"
fi
echo ""

# Check for quality samples
echo "🎵 Quality Samples:"
if [ -d .agent/quality_samples ]; then
    SAMPLE_COUNT=$(find .agent/quality_samples -name "*.wav" 2>/dev/null | wc -l)
    echo "  Total samples: ${SAMPLE_COUNT}"
    echo "  Latest: $(ls -t .agent/quality_samples/nfe6_vs_nfe7_* 2>/dev/null | head -1 | xargs basename || echo 'None')"
else
    echo "  No quality samples directory found"
fi
echo ""

# Check recent commits
echo "📝 Recent Commits:"
git log --oneline -3 2>/dev/null | sed 's/^/  /' || echo "  Unable to check git history"
echo ""

# Performance summary
echo "🚀 Performance Status:"
if [ -f .agent/STATUS.md ]; then
    grep "Mean RTF" .agent/STATUS.md | head -3 | sed 's/^/  /'
    grep "Best RTF" .agent/STATUS.md | head -3 | sed 's/^/  /'
else
    echo "  Status file not found"
fi
echo ""

# Check if backend is running
echo "🔧 Service Status:"
if pgrep -f "ishowtts-backend" > /dev/null; then
    echo "  Backend: ✅ Running (PID: $(pgrep -f ishowtts-backend))"
else
    echo "  Backend: ❌ Not running"
fi

if pgrep -f "trunk serve" > /dev/null; then
    echo "  Frontend: ✅ Running (PID: $(pgrep -f trunk))"
else
    echo "  Frontend: ❌ Not running"
fi
echo ""

echo "============================================================"
echo "Quick Commands:"
echo "  Lock GPU:    sudo jetson_clocks && sudo nvpmodel -m 0"
echo "  Test Perf:   /opt/miniforge3/envs/ishowtts/bin/python scripts/validate_nfe7.py"
echo "  Monitor:     /opt/miniforge3/envs/ishowtts/bin/python scripts/monitor_performance.py"
echo "  Start All:   ./scripts/start_all.sh --wait 900 --no-tail"
echo "============================================================"