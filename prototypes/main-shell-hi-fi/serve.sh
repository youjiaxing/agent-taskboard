#!/bin/sh
# 一次性原型：三方向对照。浏览器打开后用底栏或 ← → 切换。
#   ?direction=codex-map  Codex 原貌映射（动了结构）
#   ?direction=codex      Codex 气质（结构未改）
#   ?direction=paper      纸面精修（结构未改）
cd "$(dirname "$0")"
PORT="${PORT:-8767}"
echo "http://127.0.0.1:${PORT}/?direction=codex"
exec python3 -m http.server "$PORT"
