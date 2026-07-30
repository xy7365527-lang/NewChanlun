# #641 实装收尾双轴评审 · Standards 轴终态复审

日期：2026-07-29　评审面：`git diff 039cf86974...0de84f4b09`（3 commit，+2978/-4）
标准源：`AGENTS.md` + `docs/agents/{generation-constitution,delivery-discipline,stat-provenance}.md` + Fowler 12 条基线
（`.claude/rules/` 已归档下线，不作依据）　只读评审，未改代码

**结论：FAIL（1 HIGH / 2 MED / 3 LOW）**

## (a) 违反文档标准处

### S-1 【HIGH】★主缝「双路径逐字节一致」测试是恒真式 — `rust/src/theta_v0/classifier/mod.rs:4444-4452`

`replay` 与 `prefix_incremental` 是**同一函数、同一实参、同为 fresh `TowerCache`** 的两次调用，
`assert_eq!` 恒成立；真正走跨步增量宿主的 `incremental`（4442 行，共享 cache）只被 4453 行的非空断言用到，
**从未与任何全历史重建对比**。该测试的 doc 自称「若增量宿主与全历史重建有任何分叉，逐 revision 的
`PartialEq` 当场变红」——分叉不会变红。

违 AGENTS.md「非真空锁」实践与 `delivery-discipline.md`「说豁免时同时给出能验真的证据……豁免不是理由句，
而是一个可以被检验的命题」；commit message 的「主缝双路径」验收句由此悬空。

### S-2 【MED】读数「同源」声明与实现不符 — `rust/src/theta_v0/classifier/chain_cert/mod.rs:736-738`

`ChainBookSummary` doc：「读数逻辑此前散在两个诊断 bin 里各写一遍（`issue550_event_battery` 的
`print_chain_summary` 与 p123 的 `chain_dump_line`）……放进库内 ⟹ 两个 bin 同源」。实际 p123 的
`chain_dump_line`（`p123_fast_replay.rs:626-687`）仍逐 certificate 手写 `count()`/`adjacent`/`crossed`/
`skip = edges.len() - adjacent`，一行未迁。注释描述的收口只发生在 battery 一侧。

### S-3 【MED】digest 与 summary 取自**不同簿状态** — `rust/src/bin/issue550_event_battery.rs:160 / 168 / 215`

`summarize()`（160）在幂等重放 `book.advance(...)`（168，**会写簿**）之**前**，`book.digest()`（215）在其之后。
`idempotent_replay_delta != 0` 时（恰是要排障的那次），同一行报告里的分桶数与 digest 分属两个簿状态，
而 digest 正是 §八/§九 引用的锚值。违 `stat-provenance.md`「引用的那一刻能不能被人一眼判定」的可复核要求。

### S-4 【LOW】报告缺统计口径行 — `chanlun/review-results/issue641-n3-chain-impl-20260729.md`

`stat-provenance.md` 轻档要求归档报告标一行 `**统计口径**: …`；本报告 §五 的 BTC 三窗读数（skip_ratio、
digest、耗时）已被 §九与 commit message 引用，全文零命中「统计口径」。（口径边界：该规范字面锚
`.chanlun/review-results/`，本报告落在 `chanlun/review-results/`，故降为 LOW。）

### S-5 【LOW】bin 用法行未随新参数更新 — `rust/src/bin/issue550_event_battery.rs:4`

新增第三位定位参数 `chain_every`（`:120-126`，默认 5_000），头部 `//! 用法：` 仍写
`-- <btc_1m_full.json> [max_bars]`。魔数 5_000 亦无出处注记。

### S-6 【LOW】pub 导出零非测试调用者 — `chain_cert/mod.rs:498`（`chain_paths`）、`:184`（`CHAIN_SEGMENT_PREDICATE`）、`:793`（`latest_of`）

全仓 grep 仅本模块与其 `tests.rs` 命中。按 `generation-constitution.md` §1 判据表，「生产零可达但有测试
调用者」= **deprecated 待退役**——三个 API 落地即入该格。模块整体「纯产出零消费」有 #636 裁定豁免，
但豁免的是 `ChainCertificateBook`（bin 真调），不覆盖这三个从未被任何调用点触达的导出。

## (b) 基线臭味（Fowler 12 条）

- **Comments（注释代替修正）**：S-2 的 doc 描述了一次未发生的重构；`chain_cert/mod.rs:1032-1035`
  的「★恒 0」注释与 `mod.rs:4463` 的运行期断言重复陈述同一事实（后者是真载体，前者可删）。
- **Duplicated Code**：p123 `chain_dump_line` 与 `ChainBookSummary::summarize` 的边/节点分桶逻辑同形两写（S-2）。
- **Long Function（增量方向）**：`evaluate`（`chain_cert/mod.rs:552-629`，78 行）把节点留痕、边构造、
  可扩展性、三态判定、探针五件揉在一处；`print_chain_summary`（battery `:135-217`）83 行。均属本 diff 新增，
  方向偏长，但各段线性无嵌套分支，仅记录不判 FAIL。
- **未见**：Feature Envy / Data Clumps / Shotgun Surgery / Primitive Obsession 无新增证据；
  `ChainBookSummary` 25 字段是 DTO，不判 Large Class。

## 判定依据外的照实登记

`unreachable!` 两处（`:698` `breach_reason`、`:1162` `on_append`）均附完整推导链，符合「机器载体优于注释
声明」的既有纪律，**不计为臭味**；`ChainKey::new` 的 `assert!` 同理。三 commit 的 message 均引 GitHub
真实 issue `#641`，符合 `delivery-discipline.md` 编号防复用条款。
