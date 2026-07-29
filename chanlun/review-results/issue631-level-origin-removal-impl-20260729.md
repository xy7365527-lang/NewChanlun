# #631 level_origin 处置实施报告（claude sonnet）

> 日期：2026-07-29；性质：实施车 stdout 收口报告原文
> 原始日志：/tmp/issue631-dispatch-20260729.log

干净——只剩其他并行工位的未提交改动，与本票无关，全部保留原样。

## 收口输出

**1. commit SHA**：`d9f1860124b660007d10bb249713f74b3e1e9c20`（`kimi-nest-mainline-20260717`，仅含本票 14 个文件）

**2. 意图核查结论：删**
- `git show bbbd8f89fa` 确认该字段是 #218 证书索引三部曲"顺带入库、同文件不可分"的副产物，非独立设计意图；`classify_impl`/`classify_with_tower_incremental` 终装点从未写入非零值
- 关键发现：main 分支已有**独立先例**——issue #434（同一空转字段缺陷，来自 #115 影子评审）经 grilling 裁定后以 SPEC #455（commit `2d1abf9786`）**照删**，理由与本票归因完全同构：级别身份正主是 `LevelProjectionLayer.identity.level`，`level_origin` 是恒零冗余拷贝，全仓无消费方依赖它做跨级比较/去重
- 本分支（`kimi-nest-mainline-20260717`）在 main 那次修复之前就已分叉，从未拿到该提交，故此处需要独立复现同一处置

**3. GOLDEN 二次重锚证据**：`signal.rs` GOLDEN 从 `0xe6a2_63e3_43e4_3845` → `0xe371_3897_d9bf_978c`，`extract_signals_bit_exact_digest_guard` 实跑转绿。该新值与 main 分支 #455 提交记录的历史值**逐位相同**——两个独立分支对同一改动的实跑结果一致，构成交叉印证。

**4. 指纹/对拍**：
- `cargo test --release --lib`：删除前后均 **2204 passed / 0 failed / 138 ignored**
- `p123_fast_replay` 20k bars stdout 对拍：**零 diff**（用临时 `git worktree add` + patch 隔离手法，把本票 diff 单独应用/反应用到干净 HEAD 上前后跑测，避免被同一共享工作区里其他并行票据——#635 的 `retrace_ledger/*`——的未提交改动污染对照）

**5. 偏离**：无实质偏离。唯一操作性调整：未直接复用 main #455 的历史 GOLDEN 常量，而是坚持实跑重锚（按票面要求"新实跑值"），事后确认两者一致。commit message 中额外记录了与 main 独立先例的交叉印证关系，供影子评审核对。
