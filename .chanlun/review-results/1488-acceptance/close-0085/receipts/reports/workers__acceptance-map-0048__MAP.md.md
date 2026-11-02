# 收据 workers/acceptance-map-0048/MAP.md

- 原件：`workers/acceptance-map-0048/MAP.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`681173cd03c241d9d7093198ba8fe31d4495f490989553f54624c54f439117be`
- 字节数：27175；行数：296

## 关键行（行号: 原文）

```
1: # #1488 整票验收缺口图（acceptance-map-0048）
10: ## 0. 总判断（先读这段）
12: 1. **整票五个验收框目前一个都不能勾。** 已有证据全部是锁级或切片级的，CI 39364508567 是最近一次全绿，基于 fff166。没有任何 S86、MIN12、Δ_R 曲线、正式入口或浏览器运行是在 #1488 代码上做的。
13: 2. **新发现的阻断：main 已经把规则修订从 `new-bi/1` 升到 `new-bi/2`（#1405 同价规则）。**
16:    - 段 A 写的是「layout/1，`rule_revision` 不变，与 r6/r7 逐位相同」。r6/r7 是 #1394 线在 new-bi/1 下的产物（`.chanlun/review-results/issue1323-grilling-20261013/1489-AFFECTED-CODE.md:46`）。合 main 后的新二进制写 new-bi/2，CC-004/005/006 的 object_id 因 rule_revision 进配方必然换新（ADR 0037:29 同理）。所以**按字面，段 A 在合 main 后不可达**，必须裁定换基线（见裁定点 D1）
23: 4. **范围 (1) 主体已做，范围 (2) 只做了局部，范围 (3) 完全没人查过。**
27: ## 1. 范围（实现层）三条
29: ### R1 `SequenceMemo::sync` 整段写后缀
33:   - 序列切片 `3483942f`，CI 38690171268，23/23、33/33 锁通过。
52: - **资金/账户/spec 取舍**：不涉及资金和账户。「剩余来源逐一说明」是否允许把 fractal append 留到段 B（CC-007 单元批记录会整体替换 fractal 序列）需要裁定，见 D5。
54: ### R2 六阶段增量维护
66: | object_plan | A2 有效原账借用、真实 winner edits（db9887d6，CI 38722158322）；object_index `wire_at` 增量 | 锁级 | 部分。评论 :457 写「完整旧状态扫描」仍是独立问题，B histories 全扫描未清 |
67: | generate | A2 prepared 后缀与完整 generate 同核（35db1eb7，CI 38731231166） | 锁级 | 部分。delta=None 仍走全量 `sync`（generation.rs:1123） |
68: | publish_borrowed | 统一所有权段 0017→0045，Ledger/父 Arc/WorkScope 等 | CI 39364508567 | 做的是预算/所有权，**不是读量增量**，Δ_R 关系未知 |
69: | prepare_source_audit | B2 native source（46066625，CI 38762285862，234/234）；`source_call_stats` 计数 | 锁级 + warm/cold 7843 节点 | 最接近完成，有计数器，但未在 S86 上拟合 |
83: - **依赖**：「系数与 g 无关」在 layout/1 下受中段插入影响。票面已经写了「以 Δ_R 归一后的系数为准」，但 aux 快照的读量 ∝ 后缀长度 ∝ g，需要裁定这算不算「Δ_R 内」（D6）。
86: ### R3 第 46 条跳变
110: ## 2. 验收五框
112: ### A① 段 A：S86 全 86 条逐位对拍
114: - **原文**：「段 A（layout/1，`rule_revision` 不变）：同输入下所有公开根（basis root、逐代 snapshot_root）、回执、对象与 r6/r7 逐位相同：S86 全 86 条对拍，以 `compare_r7_r6.py` 的 INVARIANT/CONDITIONAL 口径。」
124: - **补齐动作**（取决于 D1 的裁定）：
126:     - 需要重建对拍脚本：按 ADR 0037/#1489 口径恢复 INVARIANT/CONDITIONAL/ALWAYS_DIFFERENT 三类；ALWAYS_DIFFERENT 类似乎是时间戳、pid 一类，需要重新列举，并独审。
134: - **资金/账户/spec 取舍**：不涉及资金和账户。**换基线属于 spec 级取舍，必须裁定（D1）。**
136: ### A② 每阶段对 Δ_R 线性，S86 g=10..86 曲线
151: - **依赖**：R2 改造完成；D6 裁定。
152: - **资金/账户/spec 取舍**：不涉及资金和账户；D4/D6 需裁定。
154: ### A③ MIN12 冻结流 52 根，10 s 帽内，第 51 条起出现 FORMED/EXTENDING
160:   - #1419 S9 正式运行只到 43 根，结论 PASS_EXCEPT_BROWSER_DEFERRED。
175: - **资金/账户/spec 取舍**：不涉及资金和账户；oracle 换规则版本需裁定（D2）。
177: ### A④ 增量≡全量对拍锁 + 变异体，旧路径只作离线对照臂
204: ### A⑤ 正式会话入口 + 公共查询 + 浏览器核对
217:   - #1419 主控裁定用 `s_v2_seed_testonly` 造库，但冻结源中未找到 v2_seed。
228: - **依赖**：#1497、#1460（若裁定归它）、#1419。
229: - **资金/账户/spec 取舍**：不涉及资金和账户；D3 需裁定。
233: ## 3. 关票门（`docs/agents/delivery-discipline.md`）
242: | G4 | 多 commit 交付逐 commit 可编译 | 冻结源侧 261 提交，含 45 对 negctrl 与 revert 提交（合计 90 条；单个 negctrl 提交可能编译但锁红，个别可能不编译）；评论里有 39091004977/39093582170 编译失败的历史 | **需裁定合入形态**：squash，还是保留历史并逐 commit `cargo check`（261 次，WSL 约数小时）（D9） |
246: | G+ | 动 CI 把关面须附 origin/main SHA 与 CI run，红灯照实写 | 0046 改 runner CI 文件 | 关票附合并后 main SHA 和 CI run 链接 |
250: ## 4. 特别核实结论
252: 1. **SequenceMemo::sync 整段后缀**：已做（generation.rs:1446-1620，切片锁 23/23、33/33 在 fff166 祖先上通过）。残留逐元素 append 位于 `content_producer.rs:1890/2508` 和 `content_bi.rs:396/400`（非 delta 路径）。S86 不可达比例**未测**。
262: 5. **正式入口、查询、浏览器**：工具齐全（fff166 与 main 一致），前置是 MIN12 oracle、#1497、D3 裁定。
266: ## 5. 建议执行顺序与并行切片
268: **前置（串行）**：P0 0047 合并转绿 → 合并面独审 → 裁定 D1/D2/D3/D9。之后所有运行都只在合并面上进行。
272: | S1 基线与工具重建 | 找回或定义 S86 输入（含哈希）；重写对拍脚本（INVARIANT/CONDITIONAL/ALWAYS_DIFFERENT）；用 main 6badaa6049 生成 S86 基线 r8（双跑） | `scripts/` 或 `.chanlun/review-results/1488-acceptance/`（新） | 独审脚本；r8 双跑逐位一致 | 代码约 300 行；WSL 约 1 h，约 10 GiB | 与 S2、S3、S4 并行 |
284: ## 6. 需主控或用户裁定的点
286: - **D1（spec 级，阻断）段 A 基线**：main 已升到 new-bi/2，合 main 后与 r6/r7（new-bi/1）逐位相同不可达，原件和 `compare_r7_r6.py` 也已丢失。建议改为「合并面 #1488 二进制 vs main 6badaa6049 完整路径二进制」在 S86 上逐位对拍（基线 r8，重写对拍脚本并独审）。需要批准改写票面验收①的字面。
293: - **D8 合 main 人工闸**：0047 合并含隐性语义冲突（group_reasons 去 equal_extreme）。合并面独审范围和合入时点需要拍板。
```
