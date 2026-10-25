# #1488 干净 linked 历史工位释放

21/21 个限定工位已通过非 force `git worktree remove` 移除，命令退出码均为0。后验确认21个目录均不存在、Git注册均已移除、独立归档引用均仍可解析到原HEAD。

## 实际检查与范围

主控已决定最终 main `8bbe4499d983e674d435261b207a6556715600b4` 替代这些历史候选。执行清单来自归档 `lfs-supplement/clean-linked-removal-commands.txt`，逐条校验其Git目录和目标与inventory一致；没有执行force、rm或prune。

21个HEAD均与归档一致，tracked/untracked为空；ignored清单未增加。每次移除前重新核HEAD、Git状态和全机lsof路径，未见目标cwd或文件句柄。两处.sandcastle控制目录及用户LaunchAgents的精确路径查询无命中；排除logs/worktrees/node_modules。六个已知common dir的alternates无一指向目标linked路径。

Sidebar核到唯一置顶NewChanlun聊天位于主仓且更新早于这些工位；当前活跃主控也在主仓，目标不与其cwd重合。主控明确限定的在飞owner与排除工位未动。

ID25的24份ignored验收原件在删除前与独立归档逐字cmp一致。其他ignored内容为已登记pycache或rust/target，可再生；未删独有验收材料。原位之外的workers、父目录、branch、common dir、restore.git、heads.bundle和LFS补充均保留。

## 逐项结果

|ID|原工位|结果 / 后验注册|
|---|---|---|
|05|`/Volumes/AgentStorage/issue1488-fixed-radix-borrowfix-20261018`|已移除 / 无注册|
|06|`/Volumes/AgentStorage/issue1488-r5-driver-six-20261018/worktree`|已移除 / 无注册|
|08|`/Volumes/AgentStorage/issue1488-stageowner-check-20261018/ci-worktree`|已移除 / 无注册|
|15|`/Volumes/AgentStorage/issue1488-stageowner-check-20261018/source`|已移除 / 无注册|
|19|`/Volumes/AgentStorage/issue1488-stageowner-check-r3-20261018/worktree`|已移除 / 无注册|
|20|`/Volumes/AgentStorage/issue1488-stageowner-check-r4-20261018/worktree`|已移除 / 无注册|
|21|`/Volumes/AgentStorage/issue1488-stageowner-check-r5-20261018/worktree`|已移除 / 无注册|
|23|`/Volumes/AgentStorage/issue1488-successor-testfix-20261018`|已移除 / 无注册|
|24|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/accept-s1-checkout`|已移除 / 无注册|
|25|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/accept-s2-checkout`|已移除 / 无注册|
|26|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/accept-s4-checkout`|已移除 / 无注册|
|27|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/baseline-0080-checkout`|已移除 / 无注册|
|33|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/five-0067-checkout`|已移除 / 无注册|
|34|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/mut-0057-checkout`|已移除 / 无注册|
|35|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/receipt-assess-checkout`|已移除 / 无注册|
|36|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/rerun-0071-checkout`|已移除 / 无注册|
|37|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/review-p2-0093-20261023`|已移除 / 无注册|
|38|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/review2-checkout`|已移除 / 无注册|
|39|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/runner-checkout`|已移除 / 无注册|
|40|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/runner-review-checkout`|已移除 / 无注册|
|42|`/Volumes/AgentStorage/issue1488-ticket-manager-20261018/uo-review-checkout`|已移除 / 无注册|

## 仍保留与后续

ID43、15个历史WIP工位、5个独立clone以及所有reference-coding工位均未处理。本次后验检查inventory中未授权的21个路径仍全部存在。ID43的活消费者与common-dir依赖仍由主控处理，不能据本报告宣称#1488全部工位已清。

外盘df读数由583Gi已用/348Gi可用变为553Gi已用/379Gi可用，期间还有并行工作，因此约30Gi变化不当作精确净回收量。

完整逐项前后态及命令结果见同目录 `1488-CLEAN-LINKED-RELEASE26.json`。未修改GitHub、总账、实现、服务或进程。
