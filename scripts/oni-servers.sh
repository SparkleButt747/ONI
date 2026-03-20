#!/bin/bash
# ONI Model Servers — start/stop/status for llama-server instances
# Each model tier runs on its own port.

set -euo pipefail

MODELS_DIR="$HOME/.cache/llama.cpp/models"
LLAMA_SERVER="$(which llama-server 2>/dev/null || echo /opt/homebrew/bin/llama-server)"
LOG_DIR="/tmp"

case "${1:-help}" in
  start)
    echo "Starting ONI model servers..."

    # MIMIR (Heavy) — Qwen3.5-27B Dense (Planner)
    if [ -f "$MODELS_DIR/Qwen3.5-27B-UD-Q8_K_XL.gguf" ]; then
      $LLAMA_SERVER \
        --model "$MODELS_DIR/Qwen3.5-27B-UD-Q8_K_XL.gguf" \
        --port 8081 --flash-attn on --ctx-size 32768 \
        --cache-type-k q8_0 --cache-type-v q8_0 \
        --n-gpu-layers 99 --threads 8 --threads-batch 16 \
        --batch-size 512 --ubatch-size 512 \
        --parallel 2 --jinja \
        --reasoning-format deepseek \
        --temp 0.6 --top-k 20 --top-p 0.95 --min-p 0.0 \
        --repeat-penalty 1.0 \
        > "$LOG_DIR/oni-heavy.log" 2>&1 &
      echo "  Heavy  (MIMIR)  → :8081 [PID $!]"
    else
      echo "  Heavy  (MIMIR)  — SKIPPED (model not found)"
    fi

    # FENRIR (Medium) — Qwen3-Coder-Next 80B MoE (Executor)
    if [ -f "$MODELS_DIR/UD-Q6_K_XL/Qwen3-Coder-Next-UD-Q6_K_XL-00001-of-00003.gguf" ]; then
      $LLAMA_SERVER \
        --model "$MODELS_DIR/UD-Q6_K_XL/Qwen3-Coder-Next-UD-Q6_K_XL-00001-of-00003.gguf" \
        --port 8082 --flash-attn on --ctx-size 65536 \
        --cache-type-k q4_0 --cache-type-v q4_0 \
        --n-gpu-layers 99 --threads 8 --threads-batch 16 \
        --batch-size 512 --ubatch-size 256 \
        --parallel 1 --jinja \
        --temp 1.0 --top-k 40 --top-p 0.95 --min-p 0.01 \
        --repeat-penalty 1.0 \
        > "$LOG_DIR/oni-medium.log" 2>&1 &
      echo "  Medium (FENRIR) → :8082 [PID $!]"
    else
      echo "  Medium (FENRIR) — SKIPPED (model not found)"
    fi

    # SKULD (General) — GLM-4.7-Flash 30B MoE (Critic)
    if [ -f "$MODELS_DIR/GLM-4.7-Flash-UD-Q8_K_XL.gguf" ]; then
      $LLAMA_SERVER \
        --model "$MODELS_DIR/GLM-4.7-Flash-UD-Q8_K_XL.gguf" \
        --port 8083 --flash-attn on --ctx-size 32768 \
        --cache-type-k q4_0 --cache-type-v q4_0 \
        --n-gpu-layers 99 --threads 8 --threads-batch 16 \
        --batch-size 512 --ubatch-size 256 \
        --parallel 2 --jinja \
        --temp 0.7 --top-p 1.0 --min-p 0.01 \
        --repeat-penalty 1.0 \
        > "$LOG_DIR/oni-general.log" 2>&1 &
      echo "  General(SKULD)  → :8083 [PID $!]"
    else
      echo "  General(SKULD)  — SKIPPED (model not found)"
    fi

    echo ""
    echo "Logs: $LOG_DIR/oni-{heavy,medium,general}.log"
    echo "Check: $0 status"
    ;;

  stop)
    echo "Stopping ONI model servers..."
    pkill -f "llama-server.*--port 808[1-5]" 2>/dev/null && echo "Done." || echo "No servers running."
    ;;

  status)
    echo "ONI Model Servers:"
    for port in 8081 8082 8083; do
      if [ "$port" = "8081" ]; then label="Heavy  (MIMIR) "; elif [ "$port" = "8082" ]; then label="Medium (FENRIR)"; else label="General(SKULD) "; fi
      if curl -s --max-time 2 "http://localhost:$port/health" >/dev/null 2>&1; then
        model=$(curl -s --max-time 2 "http://localhost:$port/v1/models" 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['data'][0]['id'])" 2>/dev/null || echo "unknown")
        echo "  :$port $label — UP ($model)"
      else
        echo "  :$port $label — DOWN"
      fi
    done
    ;;

  logs)
    tier="${2:-heavy}"
    tail -f "$LOG_DIR/oni-${tier}.log"
    ;;

  *)
    echo "Usage: $0 {start|stop|status|logs [heavy|medium|general]}"
    exit 1
    ;;
esac
