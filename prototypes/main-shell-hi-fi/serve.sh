#!/bin/sh
# 一次性原型：开 Run 配置与游离 Run 入口。浏览器打开后用底栏或 ← → 切换。
#   ?variant=A            终端位配置台
#   ?variant=B            票内配置抽屉
#   ?variant=C            先选 Agent 再填表
#   ?viewport=desktop|phone
cd "$(dirname "$0")"
PORT="${PORT:-8767}"
echo "http://127.0.0.1:${PORT}/?direction=codex-map&variant=A"
exec python3 -m http.server "$PORT"
