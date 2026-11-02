#!/usr/bin/env bash
set -euo pipefail
repo="${1:-/Users/silencehan/Projects/NewChanlun-sandcastle-host}"
venv="${2:-/Volumes/AgentStorage/issue1742-deepagents-pilot-20261027/venv}"
key="${3:-/ABSOLUTE/PATH/TO/APP-PRIVATE-KEY.pem}"
boat_config="${4:-/ABSOLUTE/PATH/TO/BOAT-CONFIG.json}"
boat_key="${5:-/ABSOLUTE/PATH/TO/BOAT-CONTROLLER.key}"
print_command() { printf '%q ' "$@"; printf '\n'; }
executor=(env "PYTHONPATH=$repo/.sandcastle/deepagents-pilot"
  "PILOT_BOAT_CONFIG=$boat_config" "PILOT_BOAT_API_KEY_FILE=$boat_key"
  "$venv/bin/python" -m pilot1742.executor)
printf '%s\n' '# #1742 D6 切流命令预览。本脚本只打印，不执行。' \
  '# 先由部署 owner 在安全轮次边界部署已合入且通过验收的 main。' \
  '# 确认日志路径、现役版本、进程父子关系、旧票去向和原聊天 owner 的交接记录。' \
  '# 控制端 shell 先离开旧宿主目录；宿主内其他 shell 和其后代仍算旧执行进程。'
print_command cd /
print_command "$venv/bin/python" "$repo/.sandcastle/drain-control.py" freeze
printf '%s\n' '# 确认 deepagents:queue 标签已存在；缺失时由部署 owner 创建后再启动 engine。'
print_command gh label list --search deepagents:queue --repo xy7365527-lang/NewChanlun
print_command gh variable set SANDCASTLE_LEGACY_FREEZE --body 1 --repo xy7365527-lang/NewChanlun
printf '%s\n' '# 先停健康监控的周期自愈；已在飞的 monitor 由后续进程快照跟踪到退出。'
print_command launchctl bootout "gui/$UID/com.newchanlun.health-monitor"
print_command "${executor[@]}" cutover snapshot --workers-log "$repo/.sandcastle/logs/workers.jsonl"
printf '%s\n' '# 等所有旧工蜂、远端 agent-* job 及 harvest 单轮自然收尾。' \
  '# 核 snapshot 内进程均退出；engine 等常驻父进程只能在无在飞子进程的轮次边界卸载。' \
  '# 由 owner 逐个确认后才执行下面的 bootout，不使用 kill。'
for label in sandcastle-claimer sandcastle-harvest wayfinder-engine decision-claimer arch-hotzone; do
  print_command launchctl bootout "gui/$UID/com.newchanlun.$label"
done
printf '%s\n' '# 旧宿主内业务 shell 由其 owner 离开该目录或结束，确认其后代也已退出。' \
  '# 若旧日志缺关闭事件，先用 cutover pending 导出精确生命周期。' \
  '# 生产审计释放另批：按 migration-drain-1742.md 准备请求，再运行 audit-release；不要伪造 sandbox/closed。'
print_command "${executor[@]}" cutover pending --workers-log "$repo/.sandcastle/logs/workers.jsonl"
print_command "${executor[@]}" cutover verify --workers-log "$repo/.sandcastle/logs/workers.jsonl" --private-key "$key"
print_command "${executor[@]}" cutover record --workers-log "$repo/.sandcastle/logs/workers.jsonl" --private-key "$key"
printf '%s\n' '# 必须先完成 verify/record，再 bootstrap 新服务；arch-hotzone 是否重启由部署 owner 在 record 后决定。' \
  '# 把 plist 模板的 __REPO__、__VENV__、__PRIVATE_KEY_PATH__、__BOAT_CONFIG__、__BOAT_API_KEY_FILE__ 渲染成已核对的绝对路径。' \
  '# 确认 plist 的 PATH 能找到 Boat CLI；密钥只填仓外文件路径，不填内容。' \
  '# 确认新执行器的 main 基线对象已在源检出中；先只读核对队列及逐 run 待批请求。'
print_command launchctl bootstrap "gui/$UID" "$HOME/Library/LaunchAgents/com.newchanlun.wayfinder-engine.plist"
print_command env "SANDCASTLE_HOST_REPO=$repo" bash "$repo/.sandcastle/run-wayfinder-engine.sh" --once --dry-run
print_command "${executor[@]}" serve --once --workers-log "$repo/.sandcastle/logs/workers.jsonl" --private-key "$key"
print_command launchctl bootstrap "gui/$UID" "$HOME/Library/LaunchAgents/com.newchanlun.deepagents-executor.plist"
printf '%s\n' '# serve --once 会执行已获签批的票，不是 dry-run。没有签批时只生成待批请求。' \
  '# 回退先让新执行器到安全边界，处理全部 active 持有，再以冻结记录的 generation resume。' \
  '# resume 立即使 cutover.json 失效；新队列停，engine 恢复 sandcastle。' \
  '# 回退恢复清单：sandcastle-claimer、sandcastle-harvest、wayfinder-engine、decision-claimer、arch-hotzone；health-monitor 最后。' \
  '# 已持有票不得直接改回旧标签，未持有票由 owner 逐票处置。先恢复旧服务，最后恢复 health-monitor 自愈。'
