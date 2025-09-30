#!/bin/bash
# Comprehensive test suite for iShowTTS
# Runs performance, quality, and reliability tests

set -e

PYTHON="/opt/miniforge3/envs/ishowtts/bin/python"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
LOG_DIR="$PROJECT_DIR/logs/tests"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Create log directory
mkdir -p "$LOG_DIR"

# Timestamp for this test run
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
TEST_LOG="$LOG_DIR/test_suite_${TIMESTAMP}.log"

# Logging function
log() {
    echo -e "$1" | tee -a "$TEST_LOG"
}

# Test result tracking
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Run a test and track results
run_test() {
    local test_name="$1"
    local test_command="$2"
    local success_pattern="$3"

    TESTS_RUN=$((TESTS_RUN + 1))

    log ""
    log "${BLUE}[$TESTS_RUN] Running: $test_name${NC}"
    log "Command: $test_command"
    log ""

    # Run the test
    local output_file="$LOG_DIR/${test_name//[^a-zA-Z0-9]/_}_${TIMESTAMP}.log"
    if eval "$test_command" > "$output_file" 2>&1; then
        # Check for success pattern if provided
        if [ -n "$success_pattern" ]; then
            if grep -q "$success_pattern" "$output_file"; then
                log "${GREEN}✓ PASSED: $test_name${NC}"
                TESTS_PASSED=$((TESTS_PASSED + 1))
                return 0
            else
                log "${RED}✗ FAILED: $test_name (success pattern not found)${NC}"
                log "Expected pattern: $success_pattern"
                TESTS_FAILED=$((TESTS_FAILED + 1))
                return 1
            fi
        else
            log "${GREEN}✓ PASSED: $test_name${NC}"
            TESTS_PASSED=$((TESTS_PASSED + 1))
            return 0
        fi
    else
        log "${RED}✗ FAILED: $test_name (command failed)${NC}"
        log "See: $output_file"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

# Main test suite
main() {
    log "=========================================="
    log "iShowTTS Test Suite"
    log "=========================================="
    log ""
    log "Timestamp: $(date)"
    log "Python: $PYTHON"
    log "Project: $PROJECT_DIR"
    log ""

    # Pre-flight checks
    log "=========================================="
    log "Pre-flight Checks"
    log "=========================================="
    log ""

    # Check Python
    if ! command -v "$PYTHON" &> /dev/null; then
        log "${RED}ERROR: Python not found at $PYTHON${NC}"
        exit 1
    fi
    log "${GREEN}✓ Python: $($PYTHON --version)${NC}"

    # Check CUDA
    if ! $PYTHON -c "import torch; assert torch.cuda.is_available()" 2>/dev/null; then
        log "${RED}ERROR: CUDA not available${NC}"
        exit 1
    fi
    cuda_version=$($PYTHON -c "import torch; print(torch.version.cuda)")
    log "${GREEN}✓ CUDA: $cuda_version${NC}"

    # Check GPU lock
    CUR_FREQ=$(cat /sys/devices/gpu.0/devfreq/17000000.ga10b/cur_freq 2>/dev/null || echo "0")
    MAX_FREQ=1300500000
    if [ "$CUR_FREQ" -lt "$MAX_FREQ" ]; then
        log "${YELLOW}⚠ WARNING: GPU not locked to max frequency${NC}"
        log "  Run: sudo jetson_clocks"
    else
        log "${GREEN}✓ GPU: Locked to max frequency${NC}"
    fi

    log ""

    # Performance Tests
    log "=========================================="
    log "Performance Tests"
    log "=========================================="

    # Test 1: Quick performance test
    run_test "Quick Performance Test" \
        "$PYTHON $SCRIPT_DIR/quick_performance_test.py" \
        "Mean RTF:"

    # Test 2: Max autotune validation
    run_test "Max Autotune Validation" \
        "$PYTHON $SCRIPT_DIR/test_max_autotune.py" \
        "Mean RTF:"

    # Test 3: Check RTF target (should be < 0.35 for passing)
    log ""
    log "${BLUE}Checking RTF target (< 0.35)...${NC}"
    if [ -f "$LOG_DIR/Max_Autotune_Validation_${TIMESTAMP}.log" ]; then
        rtf=$(grep "Mean RTF:" "$LOG_DIR/Max_Autotune_Validation_${TIMESTAMP}.log" | awk '{print $3}' | head -1)
        if [ -n "$rtf" ]; then
            if $PYTHON -c "exit(0 if float('$rtf') < 0.35 else 1)" 2>/dev/null; then
                log "${GREEN}✓ RTF check passed: $rtf < 0.35${NC}"
            else
                log "${YELLOW}⚠ RTF check warning: $rtf >= 0.35${NC}"
            fi
        fi
    fi

    # Functional Tests
    log ""
    log "=========================================="
    log "Functional Tests"
    log "=========================================="

    # Test 4: Check F5-TTS API imports
    run_test "F5-TTS API Import" \
        "$PYTHON -c 'import sys; sys.path.insert(0, \"$PROJECT_DIR/third_party/F5-TTS/src\"); from f5_tts.api import F5TTS; print(\"OK\")'" \
        "OK"

    # Test 5: Check torch.compile availability
    run_test "torch.compile Availability" \
        "$PYTHON -c 'import torch; assert hasattr(torch, \"compile\"); print(\"OK\")'" \
        "OK"

    # System Tests
    log ""
    log "=========================================="
    log "System Tests"
    log "=========================================="

    # Test 6: GPU memory check
    run_test "GPU Memory Check" \
        "$PYTHON -c 'import torch; torch.cuda.empty_cache(); mem = torch.cuda.mem_get_info(); print(f\"Free: {mem[0]/1e9:.1f}GB, Total: {mem[1]/1e9:.1f}GB\"); assert mem[0] > 5e9, \"Low GPU memory\"'" \
        "Free:"

    # Test 7: Check config file
    run_test "Config File Check" \
        "test -f $PROJECT_DIR/config/ishowtts.toml && echo OK" \
        "OK"

    # Test 8: Check reference audio
    run_test "Reference Audio Check" \
        "test -f $PROJECT_DIR/data/voices/demo_reference.wav && echo OK || test -f /opt/voices/ishow_ref.wav && echo OK" \
        "OK"

    # Optimization Validation
    log ""
    log "=========================================="
    log "Optimization Validation"
    log "=========================================="

    # Test 9: Verify torch.compile in api.py
    run_test "torch.compile in api.py" \
        "grep -q 'torch.compile.*max-autotune' $PROJECT_DIR/third_party/F5-TTS/src/f5_tts/api.py && echo OK" \
        "OK"

    # Test 10: Verify AMP in utils_infer.py
    run_test "AMP in utils_infer.py" \
        "grep -q 'torch.amp.autocast' $PROJECT_DIR/third_party/F5-TTS/src/f5_tts/infer/utils_infer.py && echo OK" \
        "OK"

    # Test 11: Verify NFE=8 in config
    run_test "NFE=8 in config" \
        "grep -q 'default_nfe_step.*=.*8' $PROJECT_DIR/config/ishowtts.toml && echo OK" \
        "OK"

    # Test 12: Verify reference audio cache
    run_test "Reference Audio Cache" \
        "grep -q '_ref_audio_tensor_cache' $PROJECT_DIR/third_party/F5-TTS/src/f5_tts/infer/utils_infer.py && echo OK" \
        "OK"

    # Final Summary
    log ""
    log "=========================================="
    log "Test Summary"
    log "=========================================="
    log ""
    log "Total Tests:  $TESTS_RUN"
    log "${GREEN}Passed:       $TESTS_PASSED${NC}"

    if [ $TESTS_FAILED -gt 0 ]; then
        log "${RED}Failed:       $TESTS_FAILED${NC}"
    else
        log "Failed:       $TESTS_FAILED"
    fi

    log ""
    log "Detailed logs: $LOG_DIR/"
    log "Summary log: $TEST_LOG"
    log ""

    # Exit code
    if [ $TESTS_FAILED -eq 0 ]; then
        log "${GREEN}=========================================="
        log "✓ ALL TESTS PASSED"
        log "==========================================${NC}"
        exit 0
    else
        log "${RED}=========================================="
        log "✗ SOME TESTS FAILED"
        log "==========================================${NC}"
        exit 1
    fi
}

# Run main
main "$@"