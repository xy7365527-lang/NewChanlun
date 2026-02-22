#!/bin/bash
# PreToolUse Hook — ceremony 守卫
# [已废弃] 075号更新后此 hook 不再执行实际逻辑。
# 结构能力由 skill 提供（事件驱动），不再需要检查 structural teammates。
# v31 审计：已从 settings.json 注册中移除。此文件可安全删除。
# 保留仅作为历史参考。

set -euo pipefail

# 废弃占位：直接放行
exit 0
