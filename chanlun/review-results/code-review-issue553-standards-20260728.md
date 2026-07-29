# #553 实装收尾 Standards 轴评审（2026-07-28）

- 工作面：`/tmp/wt-553`（`ticket-553`）；fixed point `e8496fcaf0`；commits `dc0d61052e`、`2a75254b06`
- diff：`rust/src/bin/p123_fast_replay.rs`（+196/−1）+ 验收报告 1 份
- 标准源：`.claude/rules/common/*` 全部 + Fowler 12 条基线
- **结论：PASS**（无 HIGH；2 MED 均为体量增量方向，5 LOW）

## (a) 文档标准违反

**S1 [MED] 文件体量增量方向错误** — `rust/src/bin/p123_fast_replay.rs:1`（现 2646 行，本 diff +196）
`coding-style.md` §File Organization：「200-400 lines typical, **800 max**」「Extract utilities from large modules」。文件已超上限 3.3×，本 diff 继续加压。`EventDump`（371–474）是自足单元，可迁 `rust/src/bin/` 同级模块或 `mod` 文件。

**S2 [MED] 函数体量增量方向错误** — `p123_fast_replay.rs:1104`（`run_targeted_prefix_pass` ≈480 行）
`coding-style.md` §Checklist：「Functions are small (<50 lines)」。本 diff 在其中再插 3 处（1133–1134、1411–1415、1582）。增量小但方向仍向上。

**S3 [LOW] 硬编码 env 名重复 3 处** — `:415`、`:418`、`:468`
`coding-style.md` §Checklist：「No hardcoded values (use constants or config)」。`"P123_EVENT_DUMP"` 字面量应提为 `const`。

**S4 [LOW] 早退路径静默吞写错误** — `:1582`
`event_dump.flush()?` 只在成功尾部；函数中途任一 `?` 早退时 `BufWriter` 靠 `Drop` 冲刷并**丢弃 io error**。违反 §Error Handling「Never silently swallow errors」。与既有 `P421_LIFECYCLE_DUMP:1577` 同形（既存模式，非本 diff 新引入）。

**S5 [LOW] 新增生产码测试覆盖不足** — `:413` / `:465`
`testing.md` 80% + TDD。新增 ~110 行生产码只有 2 个单测，覆盖 `observe` 双态；`from_env()`（`File::create` 失败分支）与 `flush()` 错误分支零覆盖。端到端「既有封印零扰动」已由验收报告 §5.2/§5.3（两窗 SHA-256 双侧相等 + 改动前 binary 三态对照）实测承担，此项不重复计。

## (b) Fowler 基线臭味

**F1 [LOW] Duplicated Code** — `:194–205`（P116）/ `:1125–1132`（P421）/ `:413–423`（P553）
「env→`File::create`→`BufWriter`→`map_err` 中文消息」+ 对称 flush 的三份同形代码。`EventDump` 是正确形状，但 P421 未随之收编，净结果是从 2 份变 3 份。

**F2 [LOW] 无界累加器** — `:398` `revisions: BTreeMap<EventKey, u64>`
开灯时按 distinct `EventKey` 单调增长、全程不回收（全量 4.6M bar 面）。只写不判语境下可接受，登记为已知代价。

**已核查、判非违反**：
- `side={:?}` Debug 序列化（`:437`）—— 与既有 DIV/TERM/FALLBACK/CERT 行（`:1425`、`:1448`、`:2143`、`:2284`）逐字同约定，属协议一致而非新引入脆弱面；且被 `:2585` 起逐字行断言锁定。
- Mutable Data（`seq`/`revisions` 就地自增）—— IO sink 无不可变替代；`coding-style.md` 2026-07-27 注记对本文件性能累加器的豁免虽不直接覆盖 sink，但本条 rationale（并发/隐藏副作用）在单线程只写路径不成立。
- 声明 vs 实际：docstring 三条保证（既有封印零扰动 / 零生产读取 / env 未设零行为差异）与代码一致 —— `observe` 首行 `let Some(writer) = … else { return }` 在计数之前，`revisions`/`seq` 生产侧无第二读点，`grep` 确认无回写事件。无声明膨胀。
