# #1394 历史索引逐代推进工位

- 状态：在跑，承接 #1394 TB-03-B 与已批准 #1340 C 增补。
- 分支：`codex/1394-history-sweep-20261003`。
- 固定起点：`5d742b11730d130e63d1f0f0b565389e9f13c301`，接续既有隔离实施，不改 main。
- 工作区：`/Users/silencehan/.codex/worktrees/issue1394-history-sweep/NewChanlun`。
- 主控：本会话 Root；实施采用原生 Codex 工蜂，归属 object_diff/object_index/history 及对应测试。
- 范围：当前 Capture 内的独立对象索引逐代推进，与既有 diff 判据共核；来源前缀复用绑定当前完整 raw owner 及实际修订可见区间；同步 Driver 请求内复用纯历史计算，每阶段重新取得并逐字绑定当前完整原件。保持全部根、生命周期、错误优先级及动态读者边界。
- 不纳入：跨请求免读、新公共资源数值、生产放行、前端 #1460、main 合入与推送。
- 验收：有限旧新判据对拍、靶向 Rust 测试、既有冻结实料 owned 副本的完整语义对拍和工作量计数、独立评审。
- 原封存运行：旧工作区 `.scratch/formal-s-streamed-20261003/` 及外盘 `issue1394-streamed-ab-20261003-01` 保持封存。
- 请求内复用验收：51 次靶向检查与 1 次语义补验通过，6 次完整 canonical bytes 对照一致，代码和诊断独评通过；639 项源码及三份冻结 binary 绑定已复核。证据为 `.scratch/s-native-request-audit-20261003/ROOT-ACCEPTANCE.json`，证据包不入仓。
- 诊断边界：72 状态的三次只读 fresh 审计由 18.908 秒降至 7.149 秒，进程 peakRSS 由 503,955,456 增至 631,570,432 bytes；这不是 ingest 吞吐、完整 9011 或资源资格。S7 跨请求保留仍为 0；30 秒网络超时不等于同步 handler 已返回。
- 后续：源码本地提交后，生成新具名非 test binary 与 runner 完整绑定，继续完整 9011 A/B 验收；整图 goal 保持 active。
