#!/bin/sh
# 一次性原型：Host 用量页三种信息层级。浏览器打开后用底栏或 ← → 切换。
#   ?variant=A            独立页 · 仪表盘
#   ?variant=B            浮层 · 先筛后看
#   ?variant=C            总览里的账本
#   ?usage=mixed|sparse|empty|unreachable
cd "$(dirname "$0")"
PORT="${PORT:-8767}"
echo "http://127.0.0.1:${PORT}/?direction=codex-map"
exec python3 -m http.server "$PORT"
