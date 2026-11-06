源坐标 P2 分诊与最小复现（PR #1449 / #1371）
===============================================

**P2 成立。** 固定 head `750f933bddffc8251a880555b9beae2cdce75690`。在 13 个新建隔离库执行 89 个真实 CLI 命令，确认非法坐标先 accepted；部分输入随后成功发布缺失或错挂见证的结构。负数例则在持久 Begin 后失败，留下恢复闭门。没有修改修复前产品、历史原件或 GitHub，也没有构建、HTTP、GUI 或全套测试。

| 输入或场景 | 实际结果 |
|---|---|
| 健康 0/1/2 | generation 1 TOP，三槽分别引用 e0/e1/e2 |
| 01、+1 | accepted；对象坐标变成1，raw引用仍保留非规范原串 |
| bad、9223372036854775808 | accepted；generation 1 TOP，但槽0空引用，e0错误挂入槽2 |
| -1 | accepted；advance exit1（持久整数必须是i64）；generation0，advance_state仍begun |
| 同批 e0/e1 都为0 | accepted；TOP坐标[0,0,1]，e0见证丢失 |
| 已发布后另一event_id占1 | accepted；generation2发布TOP/FALLING，出现重复坐标与空见证 |
| 更换namespace/source_epoch/instrument后占1 | 各自都accepted，混序发布，空见证；表中raw身份并未合并 |
| 同身份同坐标rev2、重复及旧版回放 | 正常TOP→RISING；重复和旧版均replay，不复活 |
| 同身份rev3移到另一坐标 | 现有IdentityConflict拒绝，generation2不变 |
| >2^53规范坐标 | 正常精确TOP与完整引用 |

证据可从 [机器分诊](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-SOURCE-COORD-TRIAGE.json) 的每case/step链接进入原始stdout/stderr、输入、回执与只读数据库状态。完整 [89次命令原件结果](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-SOURCE-COORD-PROBE/run/RESULT.json)、[来源绑定](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-SOURCE-COORD-PROBE/run/SOURCE.json) 与 [探针哈希清单](/Users/silencehan/Documents/Codex/2026-09-12/newchanlun-1371-recovery-20260912T0920Z/work/PR1449-SOURCE-COORD-PROBE/MANIFEST.json) 均保留。原二进制为 6,203,448 字节、SHA `e6fdf9deb087561ad574748ad991cf8f71f1b2a9bf9027f6678e8a8d358b9a59`；原run2来源manifest绑定的全部Rust/profile/catalog文件与750f逐项同SHA，执行的是新目录副本。

代码与合同

`cmd_accept` 的数值校验在 [1380行](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/rust/src/bin/s_structure_session.rs:1380) 漏掉seq；同身份新revision不得换坐标已有 [1457行](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/rust/src/bin/s_structure_session.rs:1457) 保证。业务身份是namespace、source_epoch、instrument、event_id，revision另列。有效事件保留每身份最新revision后，坐标排序失败回退0；advance的映射失败回退-1，Bar又回退0再转usize。两个Map按裸坐标查见证，造成实测的丢失/错挂。

已发布 [接口合同](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/INTERFACE-CONTRACTS.md:9) 要求身份/内容绑定、精确整数、来源顺序和完整见证；[SPEC](/Users/silencehan/Projects/NewChanlun-1371-codex-r12/.chanlun/review-results/issue1339-r2-spec-20260909/payload/spec/SPEC.md:254) 要求投影验证。SPEC未规定独立来源的坐标全局唯一。当前单一S计算序没有按来源分开计算，不能以换namespace等方式让冲突坐标混入。跨会话的源坐标不受此次唯一归属限制。

最小修复与回归

1. accept事务内要求seq为规范、非负、i64和usize可无损表示；不要求从0起或连续，保留>2^53。
2. 同一计算序中坐标只能归一个业务identity；该identity的多个revision合法共享坐标。不能加简单UNIQUE(source_coord)，也不能把真正另一事件当成修订。
3. 严格读取持久坐标，去掉0/-1/as usize回退；存量损坏在新增Begin之前报StorageUnavailable，不做隐式修复。
4. 入口回归应验证批内先合法后非法的事务回滚、同批及后批跨身份碰撞、合法修订/replay/旧版不复活、大整数，以及存量损坏时Begin与代际零新增。既有4320行仅测整数helper；4362/4462/5390行覆盖修订及大整数，不能代替seq入口反例。

本报告是修复前分诊。负数没有发布新结构；非规范串、不可解析串与重复坐标的后果分别列明。它不代表整个PR或B验收结论。
