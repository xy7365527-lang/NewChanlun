现在我已收集所有关键证据，准备撰写独审报告。

---

# PR1681 Native R14修复独立审查报告

**审查结果：PASS**

## 一、审查依据

- **base commit:** `7dc86ef26e5db906325834bd376d720a6036b2e8` (merge(#1602): consume approved main CI groups from #1723)
- **final head:** `047676ae2504f45ad7ea701c17f41622409488a7` (style(#1602): apply remote rustfmt to R14 schema fixture)
- **审查日期：** 2026-10-26
- **授权：** ticket #1709 只读独审，无修改权限
- **范围：** 冻结git对象读取，不访问工作树、不执行cargo/rustc

## 二、提交链与差异结构

从base到final head共2次提交：

```
ac7540436a test(#1602): account for retained policy provenance DDL in R14 baseline
047676ae25 style(#1602): apply remote rustfmt to R14 schema fixture
```

**diffstat：**
- `ac7540436a`: `rust/src/bin/s_session_v3/audit_owner.rs` +28/-2行
- `047676ae25`: 同文件格式化差异（缩进与换行调整）

**核验：** ac754→047676 diff仅为rustfmt格式patch，无逻辑变化。具体为4227-4237行的query_row调用与assert_eq表达式的缩进与换行调整。

## 三、根因分析

### 原失败证据

**远端日志：** `/tmp/nc-remote/20261026-103738-7dc86ef26e.log`

**冻结head：** `7dc86ef26e5db906325834bd376d720a6036b2e8`

**测试名：** `v3::audit_owner::r14_source_keeper_seal_and_next_request_share_explicit_parent`

**原样输出：**
```
R14_SEAL_CENSUS {"node_source_claim_bytes":111361,"node_source_claims":803,"node_source_physical_permits":0,"permits":1147,"sealed_bytes":399158}
R14_P2B_DROP {"claims_bytes":111361,"contract_len":113,"index_bytes":234000,"index_slots":1040,"layout_drop":16640,"sealed":399158,"sealed_before_p2a":527102,"slot_before_p2a":241,"slot_now":225}

thread 'v3::audit_owner::r14_source_keeper_seal_and_next_request_share_explicit_parent' (2672) panicked at src/bin/s_session_v3/audit_owner.rs:4413:17:
assertion `left == right` failed: 封存降幅恰为①+②
  left: 127944
 right: 128001
```

**关键等式失败：**
```
527102 - 399158 = 127944 (实际封存降幅)
111361 + 16640  = 128001 (来源合同额 + 索引槽宽降幅)
差值 = 57B
```

### 根因定位

1. **生产schema已增列：** commit `f8dec1a54f`（fix: #1602 外部回执持久化记录溯源）在`rust/src/bin/s_structure_session.rs`的`structure_deltas`表增加`policy_records TEXT NOT NULL DEFAULT 'legacy/unknown'`列，共57字节（逗号、换行、缩进及列声明）。

2. **测试基准未更新：** base版本`7dc86ef26e`的`audit_owner.rs:4401`行使用历史函数`sealed_before_p2a_for(current_len) = 526876 + 2*current_len`，未计入新增DDL的schema增量。

3. **封存物理账目正确：** 节点803、claims 111361B、layout_drop 16640B、封存399158B均符合生产实测，无资源帽放松或计费错误。

## 四、修复内容核验

### ac7540436a 修复提交

**文件：** `rust/src/bin/s_session_v3/audit_owner.rs`

**行号4213-4242（DDL fixture与查询）：**

```rust
const LEGACY_DELTA_SCHEMA: &str = r#"CREATE TABLE structure_deltas (
  generation INTEGER PRIMARY KEY,
  session_id TEXT NOT NULL,
  catalog_revision TEXT NOT NULL,
  base_cut TEXT NOT NULL,
  next_cut TEXT NOT NULL,
  seq_range_json TEXT NOT NULL,
  index_frontier TEXT NOT NULL,
  input_frontier INTEGER NOT NULL,
  catalog_run_status TEXT NOT NULL,
  catalog_evidence_json TEXT NOT NULL,
  delta_json TEXT NOT NULL
)"#;
const POLICY_RECORDS_COLUMN: &str =
    ",\n  policy_records TEXT NOT NULL DEFAULT 'legacy/unknown'";
let current_schema: String = f.conn.query_row(
    "SELECT sql FROM sqlite_schema WHERE type='table' AND name='structure_deltas'",
    [], |row| row.get(0),
).unwrap();
let expected_schema = LEGACY_DELTA_SCHEMA.replace(
    "\n)", &format!("{POLICY_RECORDS_COLUMN}\n)"),
);
assert_eq!(current_schema, expected_schema, "R14仅增加已批准的记录溯源列");
let schema_growth = POLICY_RECORDS_COLUMN.len();
```

**验证精度：**
- `POLICY_RECORDS_COLUMN`字面量实测57字节（含`,\n  policy_records TEXT NOT NULL DEFAULT 'legacy/unknown'`）
- `schema_growth = POLICY_RECORDS_COLUMN.len()` 从具名常量直接取字节长度，未从封存结果或许可总和倒算
- 新增schema精确匹配断言，运行时查询SQLite实际DDL与历史DDL+新列对比

**行号4426（基准修正）：**

```rust
let sealed_before_p2a = sealed_before_p2a_for(current_len) + schema_growth;
```

**等式恢复：**
- 原基准：`526876 + 2*113 = 527102`
- 新基准：`526876 + 2*113 + 57 = 527159`
- 新降幅：`527159 - 399158 = 128001`
- 右侧：`111361 + 16640 = 128001`
- **等式通过**

**行号4432-4448（断言保留）：**

```rust
assert_eq!(claims.1, expected_claims(current_len), "①来源合同额之和");
assert_eq!(layout_drop, 16_640, "②索引槽宽连带降幅");
assert_eq!(
    sealed_before_p2a - sealed,
    claims.1 + layout_drop,
    "封存降幅恰为①+②"
);
```

**未放松断言：** 节点数803、layout_drop精确16640、来源合同额推导公式、封存降幅等式均原样保留。

### 未修改文件

**`rust/src/bin/s_session_v3/cross_request_source_cache.rs`行53-63（Binding代码）：**

```rust
super::request_audit::push_cell(
    &mut rows,
    ExactRows::read_admitted(
        tx,
        c"SELECT rowid,* FROM sqlite_schema ORDER BY rowid",
        &[],
        &[],
        work,
    )?,
    work,
)?;
```

**确认：** 该binding原样保留`sqlite_schema`行读取，DDL文本为池的真实持有物，未修改。

**`rust/src/bin/s_structure_session.rs`：**

生产DDL在`f8dec1a54f`已增加`policy_records`列，本次修复未再改动生产代码，仅更新测试基准。

## 五、红绿证据

### 原候选失败（红）

**日志：** `/tmp/nc-remote/20261026-103738-7dc86ef26e.log`

**head绑定：** `ref=codex/1681-native-repair-20261026 sha=7dc86ef26e5db906325834bd376d720a6036b2e8`

**命令：** `cargo test --locked --bin s_structure_session --features s_session,s_capture_native_testonly v3::audit_owner::r14_source_keeper_seal_and_next_request_share_explicit_parent -- --exact --nocapture --test-threads=1`

**结果：** `test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1032 filtered out; finished in 2.06s`

**退出码：** `EXIT=101 SECONDS=57`

### 修复后通过（绿）

**日志：** `/tmp/nc-remote/20261026-104015-ac7540436a.log`

**head绑定：** `ref=codex/1681-native-repair-20261026 sha=ac7540436a0d951db74aeb45a8d0a98d4afd923c`

**命令：** 同上（先rustfmt再测试）

**原样输出：**
```
R14_SEAL_CENSUS {"node_source_claim_bytes":111361,"node_source_claims":803,"node_source_physical_permits":0,"permits":1147,"sealed_bytes":399158}
R14_P2B_DROP {"claims_bytes":111361,"contract_len":113,"index_bytes":234000,"index_slots":1040,"layout_drop":16640,"schema_growth":57,"sealed":399158,"sealed_before_p2a":527159,"slot_before_p2a":241,"slot_now":225}
ok
```

**结果：** `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1032 filtered out; finished in 1.91s`

**退出码：** `EXIT=0 SECONDS=75`

**关键字段对比：**
| 字段 | 红日志 | 绿日志 | 差异 |
|------|--------|--------|------|
| sealed_before_p2a | 527102 | 527159 | +57 (schema_growth) |
| sealed | 399158 | 399158 | 一致 |
| claims_bytes | 111361 | 111361 | 一致 |
| layout_drop | 16640 | 16640 | 一致 |
| node_source_claims | 803 | 803 | 一致 |

## 六、最终格式差异

**047676ae25提交：** 远端rustfmt patch回收

**diff：** `audit_owner.rs`行4227-4237缩进与换行调整，具体为：
- `f.conn.query_row(...)`拆分多行
- `LEGACY_DELTA_SCHEMA.replace(...)`换行
- `assert_eq!(current_schema, expected_schema, ...)`拆分

**逻辑验证：** ac754→047676无测试重跑，格式差异不影响有效性。

## 七、未覆盖CI

本独审核验：
1. 精确测试名的远端红绿证据
2. 冻结源码diff与行号对应关系
3. 关键等式的输入来源与推导路径
4. 格式提交与逻辑提交的隔离

**未覆盖：**
- 最终head `047676ae25`的完整CI矩阵（由主控安排）
- 其他测试套（1032 filtered out）
- macOS本地cargo验证（遵守preferences.md第6条）

## 八、审查结论

**PASS理由：**

1. **根因正确：** 修复针对生产schema增列后的过时基准，非计费逻辑错误或资源帽问题。
2. **方法符合原则：** Fix Root Causes——更新历史baseline与schema增量，不削弱断言；Model the Domain——具名历史DDL常量与新增列表示基准变化。
3. **精度保持：** 节点803、claims 111361B、layout 16640B与既有合同保持，schema增量57B从字面量取长度，不倒算。
4. **红绿充分：** 独立远端同head/同测试名的红→绿证据完整，关键字段与等式精确匹配。
5. **格式隔离：** ac754逻辑修复与047676格式patch分离，diff仅为rustfmt缩进，无新增逻辑。
6. **未放松计费：** 物理账目断言、重复计费负控、事后追认、跨父账等测试代码原样保留。

**未授权事项：**
- 本独审不构成main合入批准
- 不授权关闭#1602或其他issue
- 最终CI门与隔离策略批准门由主控执行

**建议后续：**
- 安排异家族模型复核最终head
- 运行完整CI矩阵（含1032个过滤测试）
- 确认其他R14相关测试未受影响

---

**审查人：** Claude (Anthropic) 独立非作者审查者  
**审查时间：** 2026-10-26  
**工位：** `/Volumes/AgentStorage/issue1681-native-repair-20261026`
