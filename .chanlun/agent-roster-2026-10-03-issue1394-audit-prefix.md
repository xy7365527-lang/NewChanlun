# #1394 跨请求审计复用隔离工位

- 状态：在跑，承接 #1394 TB-03-B 与已批准 #1340 C 5.1–5.9。
- 工作区：`/Users/silencehan/.codex/worktrees/issue1394-audit-prefix/NewChanlun`。
- 分支：`codex/1394-audit-prefix-20261003`；起点：`713e680d33c38de3d8f1f5272982c40d7f1bd4c5`。
- 当前职责：已完成成本判向、全持久根可达性统计和 Native 测试专用运行态根观测；下一步由 Root 核定新节点暂存/保留的完整边界。不把请求局部上下文直接长期持有，不凭闭包外计数删除节点。
- 硬约束：每次写前仍取得当前完整原件、独立索引与负依赖，实际读取失败立即拒绝。跨请求保留必须按 S7 原源成本计量和驱逐，缓存不携带提交许可；普通/Owned 路径与公开资源定义保持。
- 隔离：正在运行的完整 9011 尝试仍使用 history-sweep 工作区冻结的 713e 版本及私有 S7=1；本工位不改其任何源、输入、参数或运行域。
- 验收方向：冷/暖完整结果对拍、追加与修订失效、原件损坏拒绝、资源驱逐和实际成本；不预先声称 9011、容量资格或生产放行。
- 已验观测：现成真实 S3 四代夹具（3 次 append、1 次 revision2）及 after_prepare 真实错误回滚/正常 discard；Ordinary/Native 全持久 typed 行和声明根闭包实际字节对拍通过，revision 产生 10 次撤回。单次 Native release 构建、1 项靶向通过、独评 PASS。
- 观测结果：四个提交阶段的持久根与存活缓存根联合闭包外分别为 108/268/570/889 节点；第三代检索过上阶段 runtime-only 闭包内 11 节点。检索日志不单独证明缓存复用因果，也不给删除资格。
- 观测证据：`.scratch/issue1394-runtime-roots-20261003/ROOT-ACCEPTANCE.json`；测试专用代码入仓，scratch 证据不入仓。105 完整运行态、inflight 局部和其他进程缓存未覆盖。
- Native 测试专用暂存原型：prepare/resolve/publish 显式共享本事务 owner；原审计和原 SQL 错误拒绝保留，最终仅 flush 持久根与存活缓存根闭包所需的新节点，旧节点不删。默认关闭，生产/Ordinary/Owned 路径不启用。
- 暂存原型验收：2 项靶向通过、独评 PASS；四代完整非节点 typed 行、根及其原 bytes 一致，最终节点 7847→6960。after_prepare 及已 INSERT 两条后第三次失败均完整回滚、正常 discard 并可重试。物理 INSERT 错误时点不声明与 eager 等价；heap/Job/max_store 资格未成立。证据为 `.scratch/issue1394-staged-new-nodes-20261004/ROOT-ACCEPTANCE.json`。
- 下一验证：可用两个新自有105副本，经原 Driver/profile/clock 输入冻结第106条作冷重启对照；不从零重建105，不据直接 handle 诊断宣称30秒网络或完整9011通过。
- 禁止：旧未授权原件、main 合入/推送、GOAL 事件修改、新公共资源数值、前端 #1460、K4。
