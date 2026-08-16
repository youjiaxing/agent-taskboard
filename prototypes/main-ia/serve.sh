#!/bin/sh
# 一次性原型：主界面信息架构。浏览器打开后用底栏或 ← → 切换 A/B/C。
cd "$(dirname "$0")"
PORT="${PORT:-8765}"
echo "http://127.0.0.1:${PORT}/?variant=A"
exec python3 -m http.server "$PORT"
