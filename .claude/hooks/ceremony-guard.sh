#!/bin/bash
# PreToolUse Hook — ceremony 守卫
# [已废弃] 075号更新后此 hook 不再执行实际逻辑。
# 结构能力由 skill 提供（事件驱动），不再需要检查 structural teammates。
# 保留此文件避免 settings.json 引用断裂。
# 如需恢复前置检查逻辑，在此处添加即可。

set -euo pipefail

# 废弃占位：直接放行
exit 0
