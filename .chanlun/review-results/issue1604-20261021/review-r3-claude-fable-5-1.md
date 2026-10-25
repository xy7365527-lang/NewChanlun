# #1604 第三轮非作者独审（claude-fable-5-1，只读）

- 审查对象：PR #1663，范围 `git diff 89b0d7ca89..b07de8c83b`（主控裁定 N-2 升 `s-segment/5` 的 `d91bfdad83` 与文档登记 `b07de8c83b`）。
- 作者：Codex 子代理（gpt 系）；审查者 `claude-fable-5-1` 经 `~/.local/bin/claude-proxy`（Claude Code CLI，本机 CLIProxyLocal 代理），非作者。
- 工具限制：Read/Grep/Glob 与只读 git/sed/grep/rg/wc；未跑 cargo/pytest/node，未调 gh。提示原文：`prompt-r3.md`。日期：2026-10-22。
- 作者侧处置（本记录之后，无代码提交）：MEDIUM-1 的 Python 侧由 S 二进制测试 `tb02w_killed_case_two_scan_budget_resume_and_recover`（`contract_check` 对真实写出的 `/5` 快照跑 `validate_payload` 与 `validate_segment_links`，含 SECOND 扫描预算与续扫游标）覆盖，远端定向与 CI 分片均过；JS 侧同类样本缺口与 LOW-1～4 登记于 `.chanlun/review-results/issue1604/IMPLEMENTATION.md` §7 第 8 条。

---

结论：PASS

## 四条要求逐条核对

| # | 要求 | 判定 | 落点 |
|---|---|---|---|
| 1 | `s-segment/5` 要求 `second-seq-scan-budget/2` | 满足 | Python `s_session/s_tb02_contract.py:574-575`（`_budget_policy(v5)`）、`:764-765`（`v5`/`v4` 分档）、`:680`、`:1110`；JS `s_session/browser/q3-typed.js:246`（`budgetPolicy = v5 => …`）、`:335`、`:360`、`:540` |
| 2 | `/4` 恢复只收 v1，留作历史合同；读路径认 /3、/4、/5 | 满足 | `/4` 只收 v1：`_budget_policy(v5)` 在 `/4` 下为 False → `SECOND_SCAN_BUDGET_POLICY_V3`（`s_tb02_contract.py:574-575`），JS 同（`q3-typed.js:246`）。读路径三版并收：`s_tb02_contract.py:475/:502/:505/:507`、`q3-consumer.js:882/:1421/:1434/:1435/:1437`、`index.html:1372`、`s_query_integrity.py:410`、`q3-typed.js:23`（`RULE_CONTRACTS` 保留 `…+segment-turn/1 → /4` 并新增 `…+second-seq-scan-budget/2 → /5`） |
| 3 | 写路径只写 /5；按 schema 分派的读路径都认 /5 | 满足 | 写：`rust/src/bin/s_session_v2/tb02c.rs:20`（`SCHEMA = "s-segment/5"`），`:95` 所有 7 个 KIND 的 envelope 用同一 `SCHEMA`，K1 游标随 CC-056 走同一 `add`；`:53` `restore_cursors` 也只认 `SCHEMA`；`rust/src/bin/s_structure_session.rs:105-106` `RULE_REVISION` 加 token。Rust 全仓无 `"s-segment/4"` 代码字面量（只剩注释）。读：见第 2 条各落点，bundle `tb01c-client.js:2603/:3307` SHA 注释随之更新 |
| 4 | 测试：/4+v1 过、/5+v1 拒、/5+v2 过、/4+v2 拒（Python 与 JS 各覆盖） | 满足 | `s_session/tests/test_tb02c_turn_contract.py:215-217`（/4+v1 过）、`:219-220`（/4+v2 拒）、`:222-225`（全部 v4 样本经 `as_v5` 改写后 /5+v2 过）、`:227-230`（/5+v1、/5+v3 拒）、`:211-213`（规则绑 /5，历史规则仍绑 /4）。`judge`（`:154-162`）每条同时跑 `c.validate_payload` 与 `node -e … require(tb01c-client.js).validateSegmentPayload`，并要求两边错误文案相等，所以 JS 侧由同一组用例覆盖 |

变异抓取能力：若把 `v5` 退回 `v4` 逻辑（即 `/4` 也要 v2），`test_v4_with_budget_v1_still_accepted` 与 `test_v4_with_budget_v2_rejected` 会双双失败；若 `/4` 误收 v2，`:219` 失败；若 `expected_version` 没放行 `/5`，`:222` 失败。四个方向都能抓到。

## 其他核查项

- **历史规则 `…+segment-turn/1` 只读登记**：`tb02a_facts.rs:935`（17 轴表）、`tb02b.rs:38`（新笔 /2 合同，供 `source_pending.rs:27` 与 `source_audit.rs:1654/:1680` 读旧库）、`s_structure_session.rs:7801`（tb1405 旧规则清单）、`source_pending.rs:573-576`。会话打开门 `s_structure_session.rs:382-388` 只放行 `RULE_REVISION`/`V3_RULE_REVISION`，旧 token 自动落入只读连接。全仓 grep `"s2-axis-quantifiers` 与 `crate::RULE_REVISION` 的 match 臂，本轮范围内无遗漏。
- **Python / JS 对称性**：`_budget_policy(v5)`↔`budgetPolicy(v5)`、`_scan_checkpoints(…, v4, v5)`↔`scanCheckpoints(…, v4, v5)`（元素/检查仍按 v4，游标按 v5）、`_second_terminal(…, v5)`↔`secondTerminal(…, v5)`、`validate_segment_links` 的三处 schema 元组↔`q3-consumer.js:1421/:1434/:1435/:1437`、段端门版本筛选 `== schema`↔`=== r.payload.schema_revision`（`:1444`）均逐项对应。每个 schema 对预算 token 恰有一个必填值、无回退分支，不构成 #799「宽严两档」。
- **行数**：`q3-typed.js` 702/702、`q3-consumer.js` 2886/2886、`tb01c-client.js` 7668/7668、`index.html` 2234/2234（基线 `89b0d7ca89` ↔ 工作树）；`--numstat` 为 12/12、7/7、21/21、1/1，全部同行替换。DevSkim 基线按 `规则|文件|行号` 键入，受影响行号不漂。
- **bundle 内容**：`tb01c-client.js:2604-3305` 与 `q3-typed.js`、`:3308-6193` 与 `q3-consumer.js` 用 `diff` 逐字节相同。
- **文档**：`xianduan.md:348` 在第 7 条末追加、明标「工程登记、不改本条语义」，原文一字未动；`docs/reference-theta-v0.md:23` 同行追加一句；`IMPLEMENTATION.md:98-106` §7 第 2 条改写为第三轮落点，MEDIUM-1 标已裁已落，列出的文件/测试名与实际一致（`tb1405_old_rule_store_with_hot_wal_is_never_rewritten` 在 `s_structure_session.rs:7789`）。
- **提交说明**：两条提交主题为 `feat(s-session):`、`docs:`，正文均带「编号声明」，无 fix/close/resolve 关键词。

## 新发现

**HIGH**：无。

**MEDIUM**
- MEDIUM-1（测试覆盖，非阻断）：`/5` 的三个预算 token 落点里，只有 CC-056 `scan_budget.policy` 被 v5 用例覆盖；SECOND 序列 `scan.budget.policy`（`s_tb02_contract.py:680` / `q3-typed.js:335`）与 K1 游标 `budget_policy`（`:586` / `:248`）在 `/4`、`/5` 下都没有样本（`test_tb02c_turn_contract.py` 的 `first()` 是 FIRST 序列、`version()` 的 `scan_cursors=[]`；`test_tb02w_contract.py` 只到 `/3`）。把 `_second_terminal(..., v5)` 或 `_scan_checkpoints` 内 `_fold_cursor(..., v5)` 错传成 `v4`，现有测试抓不到。同样，`validate_segment_links` / `q3-consumer bindSegmentSequences` 的 `/5` 引用闭合（含段端门版本筛选改为按段自身 schema）没有 `/5` 用例（`test_tb02x_contract.cjs:40` 仍以 `RULE_V4` 跑）。建议补一组 `as_v5` 的 SECOND/游标样本与一条 `/5` 链接闭合用例。

**LOW**
- LOW-1（范围外、既有）：`rust/src/bin/s_session_v2/tb02b.rs:29-41` `contract_for_rule` 自 #1493 起就缺 `"s2-axis-quantifiers+new-bi/2+segment-second/2"`（#1420 规则）一臂，`tb02a_facts.rs:933` 有而此处无；基线 `89b0d7ca89` 已如此，本轮未引入，但按本轮同样的「历史规则只读登记」口径应一并补上（影响 `source_pending`/`source_audit` 读 #1420 规则旧库时报「规则修订未声明」）。
- LOW-2：`s_query_integrity.py:410` 与 `q3-consumer.js:716` 的 V5 轴表条目没有任何测试断言（`test_tb02w_contract.py:346` 只测 V3），三张规则表（`RULE_CONTRACTS`、`_published_axes` known、JS `RULE_AXES`）之间也无键集一致性锁。
- LOW-3（可读性）：`s_tb02_contract.py:768` v4 分支的期望元组首元素写成 `p["schema_revision"]`（自比恒真），JS 侧 `q3-typed.js:361` 写显式字面量；行为等价（v4 仅在 /4、/5 为真），但两边形式不对称。
- LOW-4（登记措辞）：`xianduan.md:348` 新增句写「CC-013／CC-012／CC-056 payload 升 s-segment/5」，实际 `tb02c.rs:95` 的 `add` 对 CC-011 种子/候选与 CC-055 也写同一 `SCHEMA`；与 #1493 原句口径一致，非错误，仅不完整。

## 看过什么 / 未核实什么

看过：`git log/diff 89b0d7ca89..HEAD` 全部 15 个文件的 diff；`tb02c.rs:20-97`（写出点与 `restore_cursors`）；`s_structure_session.rs:99-112/:344-389/:7786-7830`；全仓 grep `"s2-axis-quantifiers`、`segment-turn/1`、`s-segment/N`、`second-seq-scan-budget/N`、`contract_for_rule`/`for_rule_revision`/`equal_is_evidence` 调用点；`test_tb02c_turn_contract.py` 全文、`test_tb02x_contract.cjs:14-71`、`s_tb02_contract.py:362-383`；`.github/devskim-baseline.json` 中四个浏览器文件的条目形式；bundle 两段与源文件的 `diff`。

未核实：未跑 cargo/pytest/node（按本轮约束），因此 Rust 编译、Python/JS 用例实际通过情况与 `build_tb01c_browser.cjs --check` 结果均以作者登记为准；`tb01c-client.js:2603/:3307` 两条 SHA256 注释的数值未能本地重算（哈希命令被沙箱拦下），只验证了 bundle 段与源文件逐字节相同；未查远端 CI、未调用 gh。
