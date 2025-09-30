#!/usr/bin/env bash
# TTS Performance Benchmark Script
# Tests synthesis speed with various text lengths and NFE step values

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
API_URL="${TTS_API_URL:-http://localhost:27121}"

# Test texts of varying lengths
SHORT_TEXT="Hello world, this is a test."
MEDIUM_TEXT="The quick brown fox jumps over the lazy dog. This is a medium length sentence for testing purposes."
LONG_TEXT="In the beginning, the universe was created. This has made a lot of people very angry and has been widely regarded as a bad move. Meanwhile, on Earth, a small blue planet orbiting an insignificant star, humanity continues its quest to understand the mysteries of existence."

# Test configurations
NFE_STEPS=(8 16 24 32)
VOICE_ID="${VOICE_ID:-walter}"

echo "==================================="
echo "iShowTTS Performance Benchmark"
echo "==================================="
echo "API URL: $API_URL"
echo "Voice ID: $VOICE_ID"
echo ""

# Function to test synthesis
benchmark_synthesis() {
    local text="$1"
    local nfe_step="$2"
    local text_label="$3"

    echo -n "Testing $text_label (NFE=$nfe_step)... "

    # Make API request and measure time
    start_time=$(date +%s.%N)

    response=$(curl -s -w "\n%{http_code}" -X POST "$API_URL/api/tts" \
        -H "Content-Type: application/json" \
        -d "{
            \"text\": \"$text\",
            \"voice_id\": \"$VOICE_ID\",
            \"nfe_step\": $nfe_step
        }")

    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)

    end_time=$(date +%s.%N)

    if [ "$http_code" != "200" ]; then
        echo "FAILED (HTTP $http_code)"
        return 1
    fi

    # Calculate metrics
    elapsed=$(echo "$end_time - $start_time" | bc)

    # Extract audio duration from response (assuming base64 audio is returned)
    # This is approximate - would need to decode WAV header for exact duration
    audio_size=$(echo "$body" | jq -r '.audio_base64' | wc -c)
    sample_rate=$(echo "$body" | jq -r '.sample_rate // 24000')
    waveform_len=$(echo "$body" | jq -r '.waveform_len // 0')

    if [ "$waveform_len" -gt 0 ]; then
        audio_duration=$(echo "scale=3; $waveform_len / $sample_rate" | bc)
        rtf=$(echo "scale=3; $elapsed / $audio_duration" | bc)
        echo "OK - Time: ${elapsed}s, Audio: ${audio_duration}s, RTF: $rtf"
    else
        echo "OK - Time: ${elapsed}s"
    fi
}

# Check if backend is running
if ! curl -s -f "$API_URL/api/voices" > /dev/null 2>&1; then
    echo "ERROR: Backend not responding at $API_URL"
    echo "Please start the backend first:"
    echo "  cd $PROJECT_ROOT && ./scripts/start_all.sh"
    exit 1
fi

echo "Backend is running. Starting benchmark..."
echo ""

# Warmup
echo "Warming up..."
curl -s -X POST "$API_URL/api/tts" \
    -H "Content-Type: application/json" \
    -d "{\"text\": \"warmup\", \"voice_id\": \"$VOICE_ID\"}" > /dev/null

sleep 2

# Run benchmarks
echo ""
echo "--- Short Text Benchmark ---"
for nfe in "${NFE_STEPS[@]}"; do
    benchmark_synthesis "$SHORT_TEXT" "$nfe" "SHORT"
    sleep 1
done

echo ""
echo "--- Medium Text Benchmark ---"
for nfe in "${NFE_STEPS[@]}"; do
    benchmark_synthesis "$MEDIUM_TEXT" "$nfe" "MEDIUM"
    sleep 1
done

echo ""
echo "--- Long Text Benchmark ---"
for nfe in "${NFE_STEPS[@]}"; do
    benchmark_synthesis "$LONG_TEXT" "$nfe" "LONG"
    sleep 1
done

echo ""
echo "==================================="
echo "Benchmark Complete"
echo "==================================="
echo ""
echo "Analysis:"
echo "- Lower RTF is better (target: <0.5 for real-time)"
echo "- NFE 16 should be ~2x faster than NFE 32"
echo "- Compare quality by listening to generated samples"
echo ""
echo "To test quality, use the frontend at http://localhost:8080"