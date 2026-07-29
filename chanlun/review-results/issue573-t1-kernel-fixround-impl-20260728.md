# #573 T1 修复轮收口报告（claude sonnet 实施车三）

> 日期：2026-07-28；性质：实施车三 stdout 收口报告原文（影子评审 M1/M2/M3 定点修复）
> 原始日志：/tmp/issue573-t1-fixround-20260728.log

并行线未提交面完全未被吞（原 11 项一个不少，新增的 2 个 issue573-*-impl 报告是并行线自己产生的，非我所动）。commit 干净落地。

## 收口输出

**1. commit SHA + stat**
`b8f5dcf03c2df5c506137412998583da29cd6a97` — `fix(theta): #573 T1 修复轮——内核 migrate/settle 守卫 + 声明措辞收敛（影子评审 M1/M2/M3）`
3 files changed, 45 insertions(+), 4 deletions(-)：`ledger_kernel/mod.rs`（+7/-2）、`ledger_kernel/tests.rs`（+35）、`nest_lifecycle.rs`（+7/-2，全注释行）

**2. 实测基线指纹 vs 收口指纹**
- `--lib`：开工实测基线（HEAD 原始态,非影子报告的 2075）= **2079/1/137**；收口 = **2081/1/137** = 基线+2 ✓（唯一红仍为 #491 `extract_signals_bit_exact_digest_guard`，在案恒红）
- `--no-fail-fast` 全量：收口 = **2145/1/146**（唯一红同上）；内核子集 `ledger_kernel` = 30 passed（28 + 2 新负控），两个新负控均命中预期 panic

**3. 对拍零 diff 证据**
用 HEAD 原始态重编译取新基线 `/tmp/issue573-baseline3/`（原有 `target/release` 二进制无法确认对应哪个 HEAD，弃用），修复版重编译后重跑到 `/tmp/issue573-afterfix/`：8 面 `cmp` 全部 **SAME**（p123 20k/100k 的 stdout/dump/lifecycle + p92 20k/100k stdout）。p123 stderr 唯一差异是 `prefix_s`（墙钟，20k: 0.137→0.138，100k: 3.673→3.642），全部计数字段（triggers/reevals/pan_*/wm_cross_*/term_*/shadow_*）逐字相同。

**4. M1/M2/M3 逐项落位**
- **M1**（`mod.rs:413`）：`migrate` 加 `assert!(!self.entries.contains_key(&to), ...)`，负控测试 `migration_to_occupied_key_fails_loud`（`tests.rs`）
- **M2**（`mod.rs:274`）：`settle` 首行加 `assert!(!self.state().is_terminal(), "终态禁再落账（禁复活）")`，负控测试 `settle_twice_fails_loud`（`tests.rs`）
- **M2 doc 订正**（`nest_lifecycle.rs:235-236`）：`LedgerState` doc 段「承载…终态吸收、禁复活、终态钟只写一次」订正为「内核承载终态吸收（`admit`）与禁复活（`settle` 守卫，本轮新增）；终态钟只写一次由『`settle` 守卫 + 域侧 `write_settlement` 写入点』联合保证」
- **M3**（三处，措辞收敛为「凡经 push_revision/settle 写入路径不会不一致 + assert_invariants 调用点逮住绕过」）：`mod.rs:16`（doc 表格）、`mod.rs:220-221`（`LedgerEntryCore` doc）、`nest_lifecycle.rs:461-462`（`impl LedgerEntryCore<NestPolicy>` doc）
- 无跳过项——四处目标句均在 #602 后仍可定位，未出现"找不到原句"的情形

**5. 偏离/存疑**
- 无代码逻辑偏离；`git diff` 自证 nest_lifecycle.rs 三行改动全为 `///` 注释行，未碰任何代码行
- 唯一流程偏离：为拿到严谨的对拍基线，额外做了一次"回退到 HEAD 原始态重编译→取基线→恢复修复版重编译→对拍"的往返（而非直接用会话内已有的 21:49 release 二进制），因为那份二进制不能确认对应的 HEAD 是否为当前 `fd585859fa`——按规则"nest 恒不触发,任何 diff = 你改错了"的验收严格性,选择了可信但更费时的路径
- M4/M5（影子报告的两条 MEDIUM）不在本轮修复范围内，按票面口径分别交由 #575（T3）票面裁定,未在本轮触碰
