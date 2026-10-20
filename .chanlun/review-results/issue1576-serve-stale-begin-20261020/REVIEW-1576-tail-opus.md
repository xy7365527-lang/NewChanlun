## #1576 尾部补评（d3e7fd9e0e..c6bac5ca6f，rust/）

### 范围
`git diff d3e7fd9e0e..c6bac5ca6f -- rust/` 只改了 `rust/src/bin/s_session_v2/mod.rs`（+10/−3）：
- `stale_begin_on_historical_rule` 的拒绝文案（第 1267 行）里，路径②补写了 v2 init 的全部必需参数，并说明了“不带后三者会建成 v1 库”。
- 测试里 `for path in [...]` 的断言改为匹配新文案，路径②的测试注释也一并订正。

### 核查
1. **必需参数与 `dispatch`/`init_v2` 一致**
   - `dispatch` 第 1280–1297 行：命令是 `init`，且带了 `--session-generation`、`--clock-plan`、`--delivery-retain-generations` 中**任意一个**，就进入 `init_v2`。之后 `--db`、`--session`、`--catalog`、`--profile`、`--session-generation`、`--clock-plan`、`--delivery-retain-generations` 都要经过 `arg()` 或 `positive_arg()` 取值。缺任何一个都会报 `缺 {key}`，所以文案说“缺一不可”是准确的。
   - `--session` 在 v1 的 `cmd_init` 里有默认值 `s-session-default`，在 v2 里却是必填。文案把它列为必填，是对的。
   - `init_v2` 第 951–953 行还要求 `session-generation` 和 retain 必须是正数。文案里写的 `<新代>` 没有与此冲突。
2. **“不带后三者会建成 v1 库，serve 不收”是否准确**
   - 三个参数都不带时，`dispatch` 返回 `Ok(false)`，命令落到 `s_structure_session.rs:4163` 的 `cmd_init`，建出来的是 v1 库，这点准确。
   - 只带其中一两个时不会建成 v1 库，而是在 v2 分支里报缺参数。这和“缺一不可”的说法一致，不构成误导。
   - 对 v1 库，`serve` 在第 1216–1217 行 `!active` 处拒绝（`SchemaUnsupported：serve需要显式v2初始化`），所以“serve 不收”成立。
3. **测试断言与文案是否匹配**
   - 新断言 `"v2 \`init\`"` 和 `"--clock-plan --delivery-retain-generations"` 都能在新文案里原样找到。
   - `"v2 \`init\`"` 包含了旧的 `` "`init`" ``，比旧断言更严。其余断言（`catalog`/`snapshot`/`meta --db`/`#1368`、`starts_with("VersionMismatch：")`、不含“请监督器先正式recover”、拒绝前未绑定 socket、库字节不变）都没动。
4. **有无回归或放松拒绝**
   - `serve` 第 1224–1230 行的判定逻辑没有改：`advance_state != idle` 时，历史规则库返回 `VersionMismatch`，当前规则库返回 `StaleWriter`，两种情况都仍是 `Err`。
   - 这次只改了文案字符串和测试注释，拒绝条件没有放松。

### 发现
- **BLOCK**：无。
- **WARN**：无。
- **INFO-1**：测试没有断言 `--session-generation`，也没有断言“v1 库”这句提示。它们都是文案的一部分，以后有人删掉，测试也不会报错。属于可选加固，不影响本次结论。

### 结论
新文案描述的 v2 init 必需参数与 `dispatch`/`init_v2` 的实际行为一致，“不带后三者会建成 v1 库，serve 不收”也准确。测试断言与文案匹配，没有引入回归，也没有放松拒绝。上一轮的 WARN-1/INFO-1 已解决。

VERDICT: PASS
