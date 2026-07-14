# 角度 3 深度 Bug 审计：证书链、严格区间套与 goal 脚本

## 审计范围

- 当前分支：`gap3-rework-codex9-fix`
- 当前 HEAD：`bf1854d71960`
- Rust：`strict_nest_check.rs`、`theta_v0` 中证书、漏斗、`CandDeltaEvent`、确认路径
- Python：HEAD 中的 `scripts/goal_events.py`、`scripts/ceremony_scan.py`
- 裁决：`chanlun/escalate/strict-nesting-rulings-20260708.md` 裁决 1/2
- 注意：两个 Python 脚本在当前工作树已删除，但 HEAD 内容与 `.chanlun/archive/goals-ceremony-20260707/scripts/` 归档副本 SHA-256 完全相同，以下仍按原路径和 HEAD 行号报告。

## Findings

### 1. `NestCertificate::n_delta()` 无法验证“完成时递降”和逐级方向来源

- **严重级：高**
- **位置：**
  - `rust/src/theta_v0/classifier/nest.rs:150-173`
  - `rust/src/theta_v0/classifier/nest.rs:201-217`
  - `rust/src/theta_v0/classifier/nest.rs:308-340`
  - 实际旁路构造者：`rust/src/theta_v0/backtest/econ_positive.rs:969-974`
- **问题：**
  - `NestRung` 只保存 `interval` 和调用方传入的 `cand: bool`，没有 `level`、`side`、`confirm_src` 或候选事件身份。
  - `n_delta()` 只验证 `cand ∧ Sub ∧ terminal.confirm_side`，无法重新验证装配器声称的：
    - 全链同方向；
    - `parent.confirm_src <= child.confirm_src`；
    - 级别严格逐级；
    - rung 确实来自对应的 `CandDeltaEvent`。
  - 所有字段均为 `pub`，其他模块可以直接构造一个 `n_delta()==true`、但从未满足完成时递降门的“证书”。`econ_positive.rs` 已经直接构造该类型，并非纯理论风险。
- **修复建议：**
  - 让 `NestCertificate`/`NestRung` 字段私有，仅允许经校验构造器创建；或在 rung 中保存 `{level, side, confirm_src, event_key}`。
  - 新增 `validate()`，逐级验证方向、级别连续性、完成时递降、Sub、Cand 和 terminal。
  - `n_delta()` 不应把无法验证的装配前置条件宣传成证书自身已证明的性质。

### 2. 裁决 2 的 `pan_div_diag` 在正常盘整路径上不可达

- **严重级：高**
- **位置：**
  - `rust/src/theta_v0/classifier/recursive_tower.rs:817-853`
  - `rust/src/theta_v0/classifier/decompose.rs:159-168`
  - `rust/src/theta_v0/classifier/decompose.rs:213-224`
  - 裁决：`chanlun/escalate/strict-nesting-rulings-20260708.md:10-13`
- **问题：**
  - `level_cand_delta()` 在 `gate_dir` 不是趋势门时于 `recursive_tower.rs:831-833` 直接 `continue`。
  - `center_trend_gate()` 只对 `MoveKind::Trend` 返回 `Some`。
  - 但稍后的 `pan_div_diag` 又要求同一个中枢属于 `MoveKind::Consolidation`。
  - 对正常、唯一中枢定位路径，这两个条件互斥，因此 `judge_pan_div()` 根本不会在真正的盘整块上执行。现有测试只断言趋势事件的诊断位为 false，没有盘整正例。
  - 这解释了现存全量报告中所有级别 `pan_div_diag=0`；该数字不能证明没有盘整背驰。
- **修复建议：**
  - 把事件提取拆成两个独立分支：
    - 趋势分支产生 `CandDeltaEvent.cand_delta`；
    - 盘整分支独立调用 `judge_pan_div`，只产生诊断事件，绝不进入证书链。
  - 更稳妥的是使用独立 `PanDivDiagEvent`，避免用一个趋势候选事件承载互斥范畴。
  - 增加“盘整背驰诊断为 true、但 `cand_delta=false` 且装配产量不增加”的裁决 2 正例测试。

### 3. 截断重放只打印警告，没有从代码上强制 verdict 无效

- **严重级：中**
- **位置：**
  - `rust/src/bin/strict_nest_check.rs:1051-1057`
  - `rust/src/bin/strict_nest_check.rs:1148-1158`
  - `rust/src/bin/strict_nest_check.rs:1427-1435`
- **问题：**
  - 设置 `STRICT_NEST_MAX_BARS < total_bars` 时只打印“判定不作数”，后面仍照常计算 `overall` 并据此返回成功/失败。
  - 当前冻结常数通常会让短前缀失败，但这是数据偶然性，不是代码保证；一旦基线、fixture 或常数变化，诊断前缀可能被错误报告为正式 PASS。
- **修复建议：**
  - 引入 `full_replay = replay_bars == total_bars`，硬纳入 `overall`。
  - 截断模式输出独立 `DIAGNOSTIC/INCONCLUSIVE`，退出码不得为成功。
  - 报告标题和硬门字段明确区分正式全量与前缀诊断。

### 4. trade 方向在校验前窄化，`255` 会被接受成合法 `-1`

- **严重级：中**
- **位置：** `rust/src/bin/strict_nest_check.rs:338-352`
- **问题：**
  - `dir` 先执行 `i64 as i8`，再检查是否为 `±1`。
  - 因 Rust 截断语义，`255 as i8 == -1`，`257 as i8 == 1`；非法输入可静默变成反方向合法交易，污染 E1 三锚统计。
  - 其他负索引同样先转 `usize`，只靠后续越界偶然拦截。
- **修复建议：**
  - 保留原始 `i64`，先验证精确等于 `1` 或 `-1`，再 `i8::try_from`。
  - 索引用 `usize::try_from`，负数立即报输入错误，并带 JSONL 行号。

### 5. 手写 JSON 解析器既拒绝合法 JSON，也可能接受错误字段

- **严重级：中**
- **位置：**
  - `rust/src/bin/strict_nest_check.rs:310-327`
  - `rust/src/bin/strict_nest_check.rs:1711-1725`
- **问题：**
  - `json_raw` 只匹配精确字符串 `"key":`，合法的 `"key" : 1` 会被拒绝。
  - 它不理解字符串转义、嵌套对象或重复 key，可能在字符串内容中误命中字段。
  - `array_body` 找到 key 后直接寻找下一个 `[`，不验证冒号和字段值边界；损坏 JSON 也可能被当作输入。
- **修复建议：**
  - 使用 `serde_json` 和带 `deny_unknown_fields` 的类型化结构。
  - 明确拒绝重复字段、非有限浮点数、无效日期和错误数组类型。
  - JSONL 错误必须包含行号与原始字段名。

### 6. `terminal_missing` 按 top-level 重复计数，不是唯一缺失 terminal 数

- **严重级：中**
- **位置：**
  - `rust/src/bin/strict_nest_check.rs:1238-1247`
  - `rust/src/theta_v0/backtest/runner.rs:199-205`
- **问题：**
  - 每个 `top` 都重新遍历相同 L0 bases，并在 callback 内累加缺失 terminal。
  - 同一个缺失 base 在 `L1…L5` 会被计算多次。报告字段却写成“terminal 查无次数”，容易被理解为唯一基例数。
  - 当前值为 0，因此尚未显现；首次出现缺失时数字会按可用 top 数膨胀。
- **修复建议：**
  - 在 top 循环前一次性计算 `missing_base_keys: HashSet<BaseIdentity>`。
  - 如果需要 per-top 诊断，单独输出 `missing_terminal_per_top`，不要与唯一缺失数混用。
  - `BaseIdentity` 至少包含 `level/side/confirm_src/interval`，不要只用可能碰撞的二元键。

### 7. “首个归零段”只识别配对归零，漏掉候选空集和 terminal 归零

- **严重级：中**
- **位置：** `rust/src/bin/strict_nest_check.rs:1443-1450,1500-1517`
- **问题：**
  - 检测条件强制要求当前级 `cand_candidates > 0`。
  - 如果首个损失来自：
    - L0 terminal 全部查无；
    - 某高级别 `cand_candidates == 0`；
    - 事件级数与分类级数不一致；
    则不会被识别为首个归零，只会输出模糊的“需检查证书终门或更高层候选空集”。
- **修复建议：**
  - 把漏斗明确拆成 `structural → Cand → terminal/base reachable → pair edges → reachable candidate → certificate`。
  - 首个归零应查找“上一阶段非零、当前阶段为零”，并输出确定原因枚举，而不是仅搜索 pair-success 为零。

### 8. 环境变量布尔/数值解析是 fail-open

- **严重级：中**
- **位置：** `rust/src/bin/strict_nest_check.rs:1051-1067,1079`
- **问题：**
  - `STRICT_NEST_P1_PERBAR=0` 仍会启用逐 bar 模式，因为代码只检查变量是否存在。
  - `STRICT_NEST_PROFILE=0` 同样启用。
  - `STRICT_NEST_MAX_BARS=abc` 和非法 checkpoint 被静默忽略并回退默认值，CLI 自动化无法发现拼写错误。
- **修复建议：**
  - 复用 runner 中已有的显式布尔解析方式，接受固定的 `0/1/true/false`。
  - 数值存在但非法时直接返回错误，不要静默采用默认值。
  - 长期建议改用 `clap` 正式参数，并把环境变量只作为可选默认值来源。

### 9. goal event 的幂等、唯一性和引用完整性存在 TOCTOU 竞态

- **严重级：高**
- **位置：** `scripts/goal_events.py:404-434,577-613`
- **问题：**
  - 执行顺序是“读取历史→检查幂等/唯一性→再次读取做引用检查→append”，整个过程没有文件锁。
  - 两个并发 writer 可以同时观察到 key/goal/evidence 不存在，然后都 append：
    - 同一 `idempotency_key` 写两次；
    - 同一 `goal_id` 重复声明；
    - 同一 `evidence_id` 重复；
    - 两个并发 `GOAL_AMEND` 同时通过旧 vector hash。
  - 这直接破坏脚本声称的重试安全和乐观锁语义。
- **修复建议：**
  - 对同一个 events 文件使用跨进程排他锁，锁内完成一次读取、全部校验、单行 append、flush 和 `fsync`。
  - `_check_reference_integrity` 接收同一个锁内 history，禁止二次读取。
  - 增加 multiprocessing 并发测试，而不只是顺序重试测试。

### 10. writer/scanner 对损坏 JSONL fail-open，可在残缺历史上继续写或宣告 clean terminate

- **严重级：高**
- **位置：**
  - `scripts/goal_events.py:438-451`
  - `scripts/ceremony_scan.py:57-87`
  - `scripts/ceremony_scan.py:1744-1754`
- **问题：**
  - writer 遇到坏行直接跳过，然后仍允许 append。坏行里可能正是已有 idempotency key、goal 或 evidence，导致重复和引用歧义。
  - scanner 虽暴露 `skipped_event_lines`，但仍继续 reducer、ready 和 `clean_terminate` 判定。
  - 并发读取正在写入的末行时，活跃 GOAL_SET/CHECK_FAIL 可能被当成坏行跳过，从而产生错误 reachable/terminal 状态。
- **修复建议：**
  - writer 对任何坏行必须 fail closed，禁止继续写。
  - scanner 发现坏行时强制 `clean_terminate=false`，输出 P0 数据修复工位，禁止 materialize 或自动 spawn。
  - 校验每行顶层必须是 JSON object，而不是仅“能被 `json.loads` 解析”。

### 11. schema 只检查字段“存在”，多个关键标量可写成数组/对象并令 reducer 崩溃

- **严重级：高**
- **位置：**
  - `scripts/goal_events.py:348-366`
  - `scripts/goal_events.py:160-176`
  - 下游触发点：`scripts/goal_reducer.py:146-149`
- **问题：**
  - `goal_id`、`description`、`base_head`、`sub_goal_id`、`artifact` 等没有统一的非空字符串类型校验。
  - 例如 `goal_id=[]` 不等于 `None` 或 `""`，会通过 writer 并进入 JSONL；reducer 随后把 list 当字典 key，触发 `TypeError: unhashable type: list`。
  - `sub_goals[].id/desc` 也只检查 truthiness，非字符串对象可以通过。
- **修复建议：**
  - 为每种事件定义完整的字段类型 schema。
  - 所有 ID、描述、hash、引用字段使用 `_nonempty_str`；`base_head` 校验 SHA 格式或显式允许的 sentinel。
  - 在 append 前对规范化后的完整事件再次执行 schema 验证。

### 12. CLI 自动生成的 goal ID 在同一秒、同一描述下确定性碰撞

- **严重级：中**
- **位置：** `scripts/goal_events.py:649-656`
- **问题：**
  - ID 只有“秒级时间戳 + 描述 hash”。
  - 已用纯函数复现：同一秒连续两次调用得到完全相同的 `g-20260710T190113Z-72fe98dd`。
  - 与 finding 9 的无锁 append 叠加时，两个 CLI 进程可把同一 ID 写入两次。
- **修复建议：**
  - 使用 UUIDv7、纳秒时间加随机熵，或在锁内检查并递增冲突后缀。
  - CLI 支持显式 `--idempotency-key`，重试应复用事件而不是生成新身份。

### 13. `DECOMPOSE` 不拒绝重复 sub-goal ID，reducer 会静默最后写者覆盖

- **严重级：中**
- **位置：**
  - `scripts/goal_events.py:160-176`
  - `scripts/goal_reducer.py:211-216`
- **问题：**
  - `_validate_sub_goals` 不检查 ID 唯一性、依赖项类型、自依赖或依赖引用存在性。
  - reducer 用 `subs[sg["id"]] = ...`，重复 ID 的前一个描述和 `blocked_by` 会被静默覆盖，直接改变 reachable workstations。
- **修复建议：**
  - writer 强制 sub-goal ID 唯一、依赖 ID 为非空字符串且必须引用同一分解中的节点。
  - 拒绝自环并做 DAG 环检测；不要把循环或拼错依赖退化成永久不可达。

### 14. 审查结果的 consumed 写入非原子，并在扫描成功交付前提前确认消费

- **严重级：高**
- **位置：** `scripts/ceremony_scan.py:800-830`
- **问题：**
  - 读取 YAML 后直接以 `"w"` 截断原文件，再写 `consumed`，没有锁、临时文件或 `fsync`。
  - 两个 scanner 可同时消费；进程在截断后崩溃会留下空文件。
  - 状态在整个 scan 输出成功前就变成 consumed；后续异常或输出丢失会导致 finding 永久消失。
  - `except Exception: continue` 又会吞掉损坏证据。
- **修复建议：**
  - 先完成 workstation 推导和输出持久化，再进行消费确认。
  - 在锁内写同目录临时文件、flush、`fsync`、`os.replace`。
  - 异常必须进入结果中的 P0 error，不能静默继续。

### 15. ceremony 的 terminal 状态枚举漏掉仓库中常见的 completed/closed

- **严重级：中**
- **位置：** `scripts/ceremony_scan.py:22-23,547-575`
- **问题：**
  - `TERMINAL_STATUSES` 只有 `已修复/resolved/background_noise`。
  - 仓库 session 中大量使用 `completed`、`closed`、`已完成`；表格路径又采用精确相等判断，因此已完成项会重新生成 workstation。
  - 这会让 `clean_terminate` 假阴性并重复调度历史任务。
- **修复建议：**
  - 使用统一状态枚举和规范化函数，至少覆盖 `completed/complete/done/closed/已完成/已结算/已修复/resolved`。
  - 不要用任意字符串和 emoji 推断终态；session writer 与 scanner 共用同一 schema。

### 16. `--workstations` 不带值时会意外执行全量扫描

- **严重级：低**
- **位置：** `scripts/ceremony_scan.py:1470,1491-1494`
- **问题：**
  - 参数使用 `nargs="*"`；`--workstations` 单独出现会得到空列表。
  - 后续用 truthiness 判断，空列表走入根 ceremony 全量扫描，而不是“指定了空工位”或参数错误。
- **修复建议：**
  - 改为 `nargs="+"`，或判断 `args.workstations is not None`。
  - `--skills`、`--workstations`、`--summary`、`--materialize-interrupt` 应放入 mutually-exclusive group。

## 裁决 1/2 一致性结论

- **裁决 1 的实际装配路径基本一致：**
  - `Cand^δ` 来源于 `CandDeltaEvent::cand_delta`；
  - `nest.rs:277,328` 在基例和每级父事件上强制 `cand_delta=true`；
  - 盘整诊断位没有被装配器消费。
- **裁决 2 只完成了“不入链”，没有可靠完成“保留诊断”：**
  - `pan_div_diag` 确实不参与证书；
  - 但 finding 2 表明真正的盘整块在进入诊断计算前已被趋势门过滤。
- **`nest.rs:146-155` 文档已过期：**
  - 仍写着 Cand 定义“需人工确认/不臆造”，与 2026-07-08 已采纳裁决及 `nest.rs:224-232` 的正式定义冲突。
  - 建议删除旧疑点标记，避免后续审查误以为裁决 1 尚未落地。

## 验证说明

- Python 两个归档脚本均通过 AST 解析。
- 已直接复现同秒、同描述 goal ID 碰撞。
- 未执行 `strict_nest_check` 全量重放：当前 sandbox 只读，而该二进制会重写 `STRICT-NEST-CHECK.md` 和漏斗报告；本报告不把现存 4,613,599-bar 报告冒充本轮独立复现。
- 本轮未修改任何文件。


