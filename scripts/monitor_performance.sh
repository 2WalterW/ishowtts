#!/bin/bash
# Performance monitoring script for iShowTTS
# Runs continuously to track GPU, memory, and performance metrics

set -e

# Configuration
LOG_DIR="logs/monitoring"
INTERVAL=60  # Seconds between measurements
PYTHON="/opt/miniforge3/envs/ishowtts/bin/python"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Create log directory
mkdir -p "$LOG_DIR"

# Get timestamp
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
GPU_LOG="$LOG_DIR/gpu_${TIMESTAMP}.log"
PERF_LOG="$LOG_DIR/performance_${TIMESTAMP}.log"
TEMP_LOG="$LOG_DIR/temperature_${TIMESTAMP}.log"

echo "iShowTTS Performance Monitor"
echo "=============================="
echo ""
echo "Logs:"
echo "  GPU:         $GPU_LOG"
echo "  Performance: $PERF_LOG"
echo "  Temperature: $TEMP_LOG"
echo ""
echo "Interval: ${INTERVAL}s"
echo "Press Ctrl+C to stop"
echo ""

# Cleanup on exit
cleanup() {
    echo ""
    echo "Monitoring stopped. Logs saved to:"
    echo "  $LOG_DIR/"
    exit 0
}
trap cleanup INT TERM

# Check if GPU is locked
check_gpu_lock() {
    CUR_FREQ=$(cat /sys/devices/gpu.0/devfreq/17000000.ga10b/cur_freq 2>/dev/null || echo "0")
    MAX_FREQ=1300500000

    if [ "$CUR_FREQ" -lt "$MAX_FREQ" ]; then
        echo -e "${YELLOW}WARNING: GPU not locked to max frequency${NC}"
        echo "  Current: $CUR_FREQ"
        echo "  Maximum: $MAX_FREQ"
        echo "  Run: sudo jetson_clocks"
        return 1
    else
        echo -e "${GREEN}GPU locked to max frequency${NC}"
        return 0
    fi
}

# Initial check
echo "Initial system check:"
check_gpu_lock
echo ""

# Write headers
echo "timestamp,gpu_util%,gpu_mem_used_mb,gpu_mem_free_mb,gpu_temp_c" > "$GPU_LOG"
echo "timestamp,rtf,synthesis_time_ms,audio_duration_s" > "$PERF_LOG"
echo "timestamp,gpu_temp,cpu_temp,thermal_zone0,thermal_zone1" > "$TEMP_LOG"

# Monitoring loop
iteration=0
while true; do
    iteration=$((iteration + 1))
    timestamp=$(date '+%Y-%m-%d %H:%M:%S')

    # GPU metrics
    if command -v nvidia-smi &> /dev/null; then
        gpu_util=$(nvidia-smi --query-gpu=utilization.gpu --format=csv,noheader,nounits)
        gpu_mem_used=$(nvidia-smi --query-gpu=memory.used --format=csv,noheader,nounits)
        gpu_mem_free=$(nvidia-smi --query-gpu=memory.free --format=csv,noheader,nounits)
        gpu_temp=$(nvidia-smi --query-gpu=temperature.gpu --format=csv,noheader,nounits)

        echo "$timestamp,$gpu_util,$gpu_mem_used,$gpu_mem_free,$gpu_temp" >> "$GPU_LOG"
    fi

    # Temperature metrics
    thermal0=$(cat /sys/devices/virtual/thermal/thermal_zone0/temp 2>/dev/null || echo "0")
    thermal1=$(cat /sys/devices/virtual/thermal/thermal_zone1/temp 2>/dev/null || echo "0")
    thermal0_c=$((thermal0 / 1000))
    thermal1_c=$((thermal1 / 1000))

    echo "$timestamp,$gpu_temp,$thermal0_c,$thermal0_c,$thermal1_c" >> "$TEMP_LOG"

    # Display current status
    echo -e "[${GREEN}$timestamp${NC}] Iteration $iteration"
    echo "  GPU: ${gpu_util}% | Mem: ${gpu_mem_used}MB / $((gpu_mem_used + gpu_mem_free))MB | Temp: ${gpu_temp}°C"

    # Parse recent backend logs for performance metrics
    if [ -f "logs/backend.log" ]; then
        # Extract recent RTF values (last 10 seconds worth)
        recent_rtf=$(grep "RTF:" logs/backend.log | tail -5 | awk '{print $(NF-1)}' | sed 's/RTF://' || echo "")
        if [ -n "$recent_rtf" ]; then
            # Calculate mean
            mean_rtf=$($PYTHON -c "import sys; vals=[float(x) for x in '$recent_rtf'.split() if x]; print(f'{sum(vals)/len(vals):.3f}' if vals else 'N/A')" 2>/dev/null || echo "N/A")
            echo "  Recent mean RTF: $mean_rtf"
        fi
    fi

    # Warnings
    if [ "$gpu_temp" -gt 80 ]; then
        echo -e "  ${RED}WARNING: GPU temperature high (${gpu_temp}°C)${NC}"
    fi

    if [ "$gpu_util" -lt 50 ]; then
        echo -e "  ${YELLOW}INFO: GPU utilization low (${gpu_util}%)${NC}"
    fi

    echo ""

    # Sleep
    sleep "$INTERVAL"
done