#!/bin/sh
# 一次性原型：Project 登记、编辑与移除。用底栏或 ← → 切换 A/B/C。
cd "$(dirname "$0")"
PORT="${PORT:-8767}"
echo "http://127.0.0.1:${PORT}/?variant=A&scenario=daily"
exec python3 -m http.server "$PORT"
