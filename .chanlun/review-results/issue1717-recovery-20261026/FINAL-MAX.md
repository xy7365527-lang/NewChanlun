# #1717 I2 SR扩量新版本返修核查报告（Opus MAX级）

**审查时间**：2026-10-26 11:35  
**审查者**：主控（Opus 5 MAX模式，独立异家族审查）  
**审查分支**：`sandcastle/issue-1717`  
**审查head**：`61a7c3acfd8a5da6d8bb770165599ceacd6ce70d`（已修复日期验证）  
**前置head**：`501e505d38`（首轮"同核消费"实装）、`98be9899a1`（原工蜂schema实装）

---

## 一、裁定结论

**BLOCKED**（不具备生产接线资格）

---

## 二、BLOCK原因（四项致命缺口）

### 1. **无业务事实输入，不比较已签阈值**

#### 实查路径
- **评估入口**：`rust/src/economic_session/tb03c_sr_expansion.rs:325` `assess_sr_expansion(record)`
- **当前行为**：
  ```rust
  // line 330-340：Operational直接取conditions[0]发冻结/退出
  if state == "Disabled" || state == "AwaitingPolicy" {
      return Ok(SrExpansionAssessment { action: "none", .. });
  }
  // line 342-360：Operational时取第一个条件的action/scope，直接返回
  let cond = &conditions[0];
  let action = required(cond, "action")?;
  Ok(SrExpansionAssessment { action: action.to_string(), .. })
  ```
- **签名**：`fn assess_sr_expansion(record: &Value) -> Result<SrExpansionAssessment>`  
  **实际输入**：只有政策记录（`record`），无事实输入（无当前资金数、持仓量、账户净值等）

#### 合同要求（issue #1717原文）
> 输入：具名新SR版本、supersedes:v1、扩量条目启用状态、**已签阈值/单位/作用域/动作**、**该版本要求的事实**。  
> 新版本只能在**显式适用时点**被消费；必须**实际比较事实和签署判据才出动作**。

#### 缺失
1. **无事实参数**：`assess_sr_expansion()` 签名不含业务事实（当前账户数值、持仓规模、fill数量等）
2. **无阈值比较**：全文 `rg -n "threshold.*value|fact.*threshold|compare.*threshold"` 仅命中2处字段引用（`:47` 文档注释、`:698` 测试修改阈值值），**零比较逻辑**
3. **Operational直接发动作**：`:330-360` 只检查 `signed==true` 即输出 `action="freeze_new"`，根本不看阈值与事实

#### 结论
**签署只授予判据，绝不意味着条件已经发生**。当前实装把"政策已签"当成"条件已触发"，这是原prompt第1项BLOCK的精确描述。

---

### 2. **create_sr_expansion_event只构造Value，没写既有账本**

#### 实查路径
- **事件构造函数**：`rust/src/economic_session/tb03c_sr_expansion.rs:385` `create_sr_expansion_event()`
- **当前行为**：
  ```rust
  pub(crate) fn create_sr_expansion_event(
      assessment: &SrExpansionAssessment,
      fact_key: &str,
      recorded_seq: i64,
  ) -> Value {
      json!({ "event_kind": SR_EXPANSION_EVENT_KIND, .. }) // 返回JSON Value
  }
  ```
- **签名**：`-> Value`（返回serde_json::Value），**无账本写入**

#### 搜索实际持久化路径
```bash
rg -n "create_sr_expansion_event\(" rust/src --type rust | grep -v "^rust/src/economic_session/tb03c_sr_expansion.rs"
# 结果：(Bash completed with no output)
```
**无外部调用者**，该函数只在自身测试模块内被调用（`:654`、`:673`、`:724`），未接入任何账本写入路径。

#### 搜索账本写入关键词
```bash
rg -n "ledger.*write|book.*append|persist.*event" rust/src/economic_session/tb03c_book.rs --type rust
# 未命中与SR扩量事件相关的写入
```

#### 合同要求
> 真实事件带SR版本、事实引用、作用域，**写既有账本**；恢复原事件版本而非最新配置。

#### 缺失
1. **函数只返回Value**，不执行写操作
2. **无外部调用者**接它的返回值写账本
3. **restore只反序列化**（`:413` `restore_sr_expansion_assessment(event)`），不涉及真实持久存储的读取

#### 结论
第2项BLOCK：无账本写入实装，事件无持久身份。

---

### 3. **五个actual_consumer测试只自调三函数，无实际消费者**

#### 测试清单
- `:627` `actual_consumer_sr_v1_preserved_disabled`  
- `:644` `actual_consumer_awaiting_policy_no_action`  
- `:661` `actual_consumer_operational_produces_action`  
- `:687` `actual_consumer_version_replay_preserves_original`  
- `:714` `actual_consumer_full_pipeline`

#### 实查每个测试的覆盖面
```rust
// :633 调用 assess_sr_expansion(&sr_v2_disabled)，检查返回值字段
// :649 调用 assess_sr_expansion(&sr_v2_unsigned)
// :654 调用 create_sr_expansion_event(&assessment, ..)，检查返回的JSON字段
// :666 调用 assess_sr_expansion(&sr_v2_signed)
// :673 调用 create_sr_expansion_event(..)
// :692 调用 assess_sr_expansion(&sr_v2_signed)
// :693 调用 create_sr_expansion_event(..)
// :701 调用 restore_sr_expansion_assessment(&event_v2)
// :720 调用 assess_sr_expansion(&sr_v2)
// :724 调用 create_sr_expansion_event(..)
// :729 调用 restore_sr_expansion_assessment(&event)
```

**全部测试只调用本模块三个函数**（`assess_sr_expansion` / `create_sr_expansion_event` / `restore_sr_expansion_assessment`），**无外部生产者调用**。

#### 搜索现役SR入口
```bash
gh issue view 1380 --json title,state --jq '.title,.state'
# [impl] TB-06-G 结构撤回、业务退出与在途保留（SPEC #1340）
# OPEN
```
**#1380仍OPEN**，无完成的冻结/退出执行者。

#### 搜索模块外引用
```bash
rg -n "tb03c_sr_expansion|SrExpansionAssessment" rust/src --type rust | grep -v "^rust/src/economic_session/tb03c_sr_expansion.rs"
# rust/src/economic_session/mod.rs:19:mod tb03c_sr_expansion;
```
**只有mod声明，无pub use、无外部import**。

#### 合同要求
> 输入具名SR版本/.../启用状态/已签阈值单位范围动作**及该版本要求的事实**。只在**显式适用时点消费**；动作仅声明范围全部重冻结新承担或同一退出。真实事件带SR版本/事实引用/作用域，**写既有账本**。

#### 缺失
1. **无现役SR生产者**：#1380未完成，无实际冻结/退出执行器调用 `assess_sr_expansion()`
2. **无实际业务事实输入**：测试全部用手造JSON（`:437` `minimal_disabled_record()`、`:455` `enabled_unsigned_record()`），无真实账户数据
3. **无账本消费路径**：`create_sr_expansion_event()` 返回的JSON未被任何账本写入函数消费
4. **无状态迁移**：测试不验证冻结状态、不验证退出状态，只检查JSON字段相等

#### 结论
第3项BLOCK：五个测试是**自调测试**（unit test of local functions），不是**消费测试**（integration test of production path）。原prompt明确要求"不能用函数测试名当证明"。

---

### 4. **把真正执行推给#1380，缩掉本票已批准要求**

#### 文档声明
`rust/src/economic_session/tb03c_sr_expansion.rs:1-32`（模块顶层注释）：
```rust
//! 本模块定义 SR 扩量记录的数据结构与验证逻辑，**不实现实际的风险评估或退出执行**
//! （评估→冻结/退出由 #1380 TB-06-G 承接）。
//! ...
//! #1380 TB-06-G 承接：
//! - 读取 `SrExpansionAssessment.action` 和 `scope` 字段
//! - 执行实际的 `freeze_new`（冻结新承担）或 `exit_all_in_scope`（退出）
```

#### 原票批准范围（issue #1717）
> ## 本票范围  
> 仅 SR 新版本扩量域；不改变 #1380 原 SR1/2，不以未签新增阈值阻断旧动作。
>
> ## 已批准行为合同  
> I2 SR 扩量新版本  
> 上游：#1599、ADR0013/0045。**消费者为 SR 评估→既有冻结/退出事件入口→账本**；#1380原SR1/2不受新增阻塞。  
> 输入：具名新SR版本/.../启用状态/**已签阈值/单位/作用域/动作**、**该版本要求的事实**。  
> 输出：带SR版本、**事实引用**与作用域的动作或不动作结果。动作只允许非选择性冻结新承担，或对声明范围内全部重同一退出。  
> 拟新增：...；**消费者须展示原SR原件、旧行为对照、新版本事件以及同核回放输出**；**不是只证明解析字段成功**。

#### 当前交付
1. **只有schema**（数据结构 `SrExpansionAssessment`、验证函数 `validate_sr_expansion_record()`）
2. **无实际消费者**（无事实输入、无阈值比较、无账本写入、无冻结/退出状态迁移）
3. **无同核回放证明**（测试不跑现役会话、不读既有账本、不验证恢复后状态）

#### 原prompt诊断
> 实现把真正执行再次推给#1380，缩掉#1717已批准明确要求。**先决能力若不存在，应交真实缺口和上游，而不是伪造完成**。

#### 结论
第4项BLOCK：**本票已批准"消费者为SR评估→既有冻结/退出事件入口→账本"**，当前交付只有"SR评估"（且无事实输入），"既有冻结/退出事件入口"和"账本"两段均缺失。文档注释声明"不实现实际的风险评估"与ticket批准的"消费者为SR评估"直接矛盾。

---

## 三、实查证据汇总

### 当前head文件清单
- **主实现**：`rust/src/economic_session/tb03c_sr_expansion.rs`（744行）
- **mod声明**：`rust/src/economic_session/mod.rs:19`
- **无外部引用**：`rg -n "tb03c_sr_expansion" rust/src --type rust | grep -v tb03c_sr_expansion.rs` 只有一处mod声明

### 测试运行记录
- **原工蜂日志**：`/tmp/nc-remote/20261026-111939-98be9899a1.log`（head `98be9899a1`）
- **测试结果**：9项，8通过，1失败（`rejects_invalid_date`）
- **61a7c3acfd修复**：补充日期有效性检查（月份1-12，日期1-31，2月≤29，4/6/9/11≤30）
- **当前测试覆盖**：全部9个测试均在 `tb03c_sr_expansion.rs` 模块内自调，无跨模块测试

### 缺失路径精确定位
1. **事实输入路径**：无。`assess_sr_expansion(record: &Value)` 签名不含事实参数。
2. **阈值比较路径**：无。`rg -n "threshold.*value|compare|>" rust/src/economic_session/tb03c_sr_expansion.rs` 无比较逻辑。
3. **账本写入路径**：无。`create_sr_expansion_event()` 只返回JSON，无外部调用者写账本。
4. **冻结/退出执行器**：无。`rg -n "freeze_new|exit_all_in_scope" rust/src/economic_session/*.rs | grep -v tb03c_sr_expansion.rs` 无外部实现。
5. **#1380消费者**：`gh issue view 1380 --json state` 返回 `OPEN`，未完成。

---

## 四、必要上游与最小下一行动

### 上游真实缺口
1. **#1380 TB-06-G**：[impl] TB-06-G 结构撤回、业务退出与在途保留（状态：OPEN）  
   **缺失能力**：
   - 实际冻结新承担（`freeze_new`）的执行器
   - 实际退出声明范围全部重（`exit_all_in_scope`）的执行器
   - 读取 `SrExpansionAssessment` 并真实改变持久状态的调用路径

2. **事实生产者**（待确定ticket）：  
   **缺失能力**：
   - 当前账户净值、持仓数量、未实现盈亏等业务事实的持续生产
   - 与SR阈值对比所需的同量纲数值（USDC金额、份数、百分比等）
   - 适用时点判断（何时检查SR条件：每次fill后？每个bar结束？手动触发？）

### 最小下一行动（按依赖顺序）

#### A. 补全本票范围内必须交付的路径
1. **修改 `assess_sr_expansion()` 签名**，增加事实输入参数：
   ```rust
   pub(crate) fn assess_sr_expansion(
       record: &Value,
       current_facts: &BusinessFacts  // 新增：包含账户净值、持仓等
   ) -> Result<SrExpansionAssessment, String>
   ```
2. **实装阈值比较逻辑**：
   ```rust
   // 在 assess_sr_expansion() 内
   let threshold_value = cond["threshold"]["value"].as_str().unwrap();
   let current_value = current_facts.get_comparable_value(cond["threshold"]["unit"]);
   if current_value < parse(threshold_value) {
       return Ok(SrExpansionAssessment { action: "none", .. }); // 未触发
   }
   // 触发后才返回 action
   ```
3. **接入账本写入**：
   - 确定现役账本结构（`rust/src/economic_session/tb03c_book.rs`）
   - 在 `assess_sr_expansion()` 后添加 `ledger.append_event(create_sr_expansion_event(..))` 调用
   - 或将 `create_sr_expansion_event()` 改为 `fn(..) -> Result<(), String>` 内部写账本

#### B. 补全前置能力（需上游票）
4. **开票#1380子任务**："SR扩量实际冻结/退出执行器"  
   **交付**：实现 `freeze_new_chong()` 和 `exit_all_in_scope_chong()`，读取 `SrExpansionAssessment` 并改变持久状态
5. **开票新票**："业务事实持续生产与SR适用时点判断"  
   **交付**：
   - 定义 `BusinessFacts` 结构体（账户净值、持仓明细、可用余额等）
   - 实装生产者（每次fill后更新、或每个bar结束更新）
   - 定义SR检查时机（policy记录中声明？硬编码？）

#### C. 补全测试证据
6. **非零消费测试**（本票范围，依赖A.1-A.3）：
   - 手造含阈值的policy记录 + 超阈值的 `BusinessFacts`
   - 调用 `assess_sr_expansion(record, facts)` 验证返回 `action="freeze_new"`
   - 调用账本写入，读账本验证事件已持久化
   - 恢复该事件，验证 `restored.version` 保持原版本
7. **真实回放测试**（本票范围，依赖B.4-B.5）：
   - 跑真实会话产生一笔fill，账户净值超SR阈值
   - 触发 `assess_sr_expansion()`，验证账本写入SR扩量事件
   - 调用冻结执行器（#1380产出），验证新承担被冻结
   - 中断+恢复会话，验证冻结状态保持

### 安全checkpoint条件
**当前不满足任何已批准交付门，建议安全回交**：

1. **BLOCKED交付**：
   - 本报告 `FINAL-MAX.md` 写入 `.chanlun/review-results/issue1717-recovery-20261026/`
   - 标注"BLOCKED（不具备生产接线资格）"
   - 附本节"必要上游与最小下一行动"
   - 不删历史原件（`61a7c3acfd` 保留）
   - 不把五个 `actual_consumer_*` 测试改名或声称为真正消费证明

2. **不合并main**：
   - 当前分支 `sandcastle/issue-1717` 保持独立
   - 等A.1-A.3（本票范围）+ B.4-B.5（上游）全部完成后再开合并审查

3. **不关票**：
   - #1717保持OPEN
   - 引用本报告说明BLOCKED原因
   - 待上游完成后重开实施

---

## 五、原工蜂实装复盘（三次提交）

### 98be9899a1（原工蜂首提交，2026-10-26T03:14前）
- **交付**：schema定义 + 9个测试
- **测试结果**：9项，8过，1败（`rejects_invalid_date`）
- **问题**：`date()` 函数只检查格式（10字符、`-` 位置、全数字），不验证日期有效性

### 501e505d38（首轮"同核消费"，2026-10-26某时刻）
- **声称交付**："补全SR扩量同核消费路径"
- **实际新增**：
  - `assess_sr_expansion()` 函数（无事实输入，Operational直接发动作）
  - `create_sr_expansion_event()` 函数（只返回JSON，无账本写入）
  - `restore_sr_expansion_assessment()` 函数（只反序列化，无持久读取）
- **测试新增**：5个 `actual_consumer_*` 测试（全部自调本模块三函数，无外部调用）
- **问题**：文档声明"不实现实际的风险评估或退出执行"，与ticket批准的"消费者为SR评估→既有冻结/退出事件入口→账本"矛盾

### 61a7c3acfd（修复日期验证，2026-10-26 11:28）
- **交付**：补充 `date()` 函数的日期有效性检查
- **修复**：月份1-12，日期1-31，2月≤29，4/6/9/11≤30
- **测试结果**：预期9项全过（待nc-remote验证）
- **问题**：修复测试失败，但四项BLOCK仍存在

---

## 六、与原prompt要求的逐项对照

| 原prompt要求 | 当前实装 | 符合性 |
|---|---|---|
| 1. assess_sr_expansion()没有任何业务事实输入，根本不比较已签threshold | ✅ 确认：签名无事实参数，无阈值比较 | **不符合** |
| 2. create_sr_expansion_event只构造Value，没写既有账本 | ✅ 确认：返回JSON，无外部调用者写账本 | **不符合** |
| 3. restore只反序列化，没有真实持久身份 | ✅ 确认：`:413` 只从JSON恢复结构体 | **不符合** |
| 4. 五个actual_consumer_*测试只是自调这三函数，没有消费现役SR入口/正式会话/账本 | ✅ 确认：全部调用本模块函数，无外部入口 | **不符合** |
| 5. 实现把真正执行再次推给#1380，缩掉#1717已批准明确要求 | ✅ 确认：文档声明"不实现风险评估"，#1380仍OPEN | **不符合** |
| GOAL：先追出现役事实生产者、SR v1评估及冻结/退出/账本消费者，核其真实调用、输入与状态迁移 | ❌ 未执行：无事实生产者、无冻结/退出执行器、无账本写入 | **未完成** |
| 若真实缺少消费者/事实合同，不新造第二风险引擎或假事实源；直接交精确路径/搜索和真实上游票作为BLOCK | ✅ 本报告交付：已列明缺失路径、已定位#1380上游、已建议新开事实生产者票 | **符合** |

---

## 七、不得标注为完成的理由

### 原prompt明确禁止
> REPORT：本分支 .chanlun/review-results/issue1717-recovery-20261026/FINAL-MAX.md，明确IMPLEMENTED或BLOCKED、实查head/路径/函数/当前调用者、证据、必要上游和最小下一行动。**将先前无事实直接动作的实现明确标为不具备生产接线资格**；不删历史原件、不把旧自调测试改名当真正消费。**输出`<promise>COMPLETE</promise>`仅表示本轮结束，结论必须据证据**。

### 当前状态
- ✅ 已实查head：`61a7c3acfd`
- ✅ 已列明路径：三个函数签名、调用点、文件行号
- ✅ 已定位调用者：无外部调用者，只有测试自调
- ✅ 已列证据：搜索命令、日志文件、文件统计
- ✅ 已明确BLOCKED：四项致命缺口
- ✅ 已列必要上游：#1380、事实生产者
- ✅ 已建议最小下一行动：7条（A.1-A.3本票、B.4-B.5上游、C.6-C.7测试）
- ✅ 已标注不具备生产接线资格：见"一、裁定结论"
- ✅ 不删历史原件：`61a7c3acfd` 保留
- ✅ 不改名旧测试：5个 `actual_consumer_*` 保持原名，明确标注为"自调测试，非消费证明"

### 不能标注完成的原因
1. **输出不是"IMPLEMENTED"**：四项BLOCK均未修复
2. **无新代码符合合同**：当前head无事实输入、无阈值比较、无账本写入、无状态迁移
3. **测试不证明生产路径**：9个测试全部自调，无跨模块、无真实会话、无持久账本
4. **上游能力缺失**：#1380未完成，无冻结/退出执行器

---

## 八、报告签署

**审查者**：主控（Opus 5 MAX模式）  
**日期**：2026-10-26  
**审查模式**：独立异家族审查（非作者）  
**结论依据**：
- issue #1717原文（批准SPEC）
- 实际代码路径（`rust/src/economic_session/tb03c_sr_expansion.rs:325-429`）
- 测试覆盖（9项全部自调）
- 外部引用搜索（零调用者）
- 上游状态（#1380 OPEN）

**不标注完成**，不输出 `<promise>COMPLETE</promise>`。  
**本轮结束**：已交付精确缺口诊断、必要上游、最小下一行动。

---

## 附录：关键搜索命令复现

```bash
# 1. 搜索三函数外部调用
rg -n "assess_sr_expansion\(|create_sr_expansion_event\(|restore_sr_expansion" rust/src --type rust | grep -v tb03c_sr_expansion.rs
# 结果：无外部调用

# 2. 搜索模块外引用
rg -n "tb03c_sr_expansion|SrExpansionAssessment" rust/src --type rust | grep -v tb03c_sr_expansion.rs
# 结果：rust/src/economic_session/mod.rs:19:mod tb03c_sr_expansion;

# 3. 搜索阈值比较逻辑
rg -n "threshold.*value|fact.*threshold|compare.*threshold" rust/src/economic_session/tb03c_sr_expansion.rs
# 结果：:47（文档注释）、:698（测试修改阈值值），无比较逻辑

# 4. 搜索账本写入
rg -n "ledger.*write|book.*append|persist.*event" rust/src/economic_session/tb03c_book.rs --type rust
# 结果：无SR扩量相关写入

# 5. 确认#1380状态
gh issue view 1380 --json state --jq '.state'
# 结果：OPEN

# 6. 统计测试文件
ls -la rust/src/economic_session/*.rs | wc -l
# 结果：40（含测试文件，但无跨模块SR扩量测试）

# 7. 读取原测试日志
cat /tmp/nc-remote/20261026-111939-98be9899a1.log | grep -A 20 "test result:"
# 结果：FAILED. 8 passed; 1 failed（rejects_invalid_date）
```

---

**END OF REPORT**
