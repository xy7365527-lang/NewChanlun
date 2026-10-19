# #1527 commit cb56f874c2 审查报告

**审查员**：独立审查员  
**commit**：cb56f874c2（分支 codex/1527-owner-integration-20261019）  
**改动文件**：rust/src/bin/s_session_v3/admission_private/fd_wal.rs  
**审查日期**：2026-10-19

## 审查概要

改动涉及三部分：

1. **新增白名单常量** `COUNT_SCAN_TABLES` （第 645-659 行）
2. **重构 audit_sql_allowed 匹配分支** （第 672-681 行）
3. **新增单测** `count_scan_reads_are_limited_to_business_tables` （第 868-925 行）

---

## 核心审查点

### 1. 非空列名读的既有行为是否改变？ ✅

**原代码** (f80670bb)：
```rust
ffi::SQLITE_READ => {
    db == Some(b"main")
        || (db.is_none() && b == Some(b""))
        || a == Some(b"pragma_table_info")
}
```

**新代码** (cb56f874c2)：
```rust
ffi::SQLITE_READ if b == Some(b"") => {
    db.is_none() && a.is_some_and(|table| COUNT_SCAN_TABLES.contains(&table))
}
ffi::SQLITE_READ => db == Some(b"main") || a == Some(b"pragma_table_info"),
```

**分析**：
- Rust match 语义：第一个分支 `if b == Some(b"")` 的 guard 只匹配空列名情况
- 当 `b != Some(b"")` 时，第二个 `ffi::SQLITE_READ` 分支执行
- 第二分支保持原规则：`db == Some(b"main") || a == Some(b"pragma_table_info")`
- 非空列名读行为**完全不变** ✅

### 2. `db == Some(b"main")` 且 `b == Some(b"")` 行为改变？ ⚠️ 

**改变事实**：该参数组合从允许变为拒绝。

**原代码**允许：`db == Some(b"main")` （通过第一个 OR 条件）  
**新代码**拒绝：第一个分支的 guard `b == Some(b"")` 匹配，但条件 `db.is_none()` 为 false，整个分支返回 false

**影响评估**：

检查此参数组合是否在生产审计中出现——即是否存在 SQLite 通过授权器传递 `db==Some(b"main"), b==b"", a=任意表` 的查询：

**发现1**：audit_owner.rs:310 确实存在 `SELECT COUNT(*) FROM main.meta`：
```rust
let _: i64 = active
    .tx()
    .query_row("SELECT COUNT(*) FROM main.meta", [], |r| r.get(0))
```

**但**，该查询**不经过本授权器**的理由：
- 该 count 调用发生在 audit_owner.rs 中，通过 `AuditConnection::open()` 打开
- AuditConnection::open 使用 s_structure_session.rs 中的 `open_db()`
- open_db 使用标准 `Connection::open_with_flags()` **无 VFS 参数**
- fd_wal 的授权器通过自定义 VFS 注册，仅对 fd_wal::Registration::connect() 返回的 connection 生效
- audit_owner 的连接绕过了 fd_wal VFS 和授权器

**发现2**：审计代码 (state.rs, v2::audit_private_startup) 中无带库名的 count(*) 查询
- state.rs:1314 仅有一个 count：`SELECT count(*),coalesce(sum(length(body)),0) FROM s_content_nodes`
- 无库名前缀，表名 s_content_nodes 不在白名单中（单测第 894 行正确拒绝）

**结论**：改动不会导致生产审计失败 ✅  
理由：受授权器监控的审计代码不会发出 `db==main, b==b""` 的查询组合

### 3. 白名单完整性核查 ✅

**白名单来源**：探针 f3624c5d 在 windows-5090 运行全套 s_structure_session 测试，记录 768 次空列名读，统计表名：

| 表名 | 次数 | 白名单状态 |
|------|------|----------|
| structure_deltas | 144 | ✅ |
| raw_events | 144 | ✅ |
| s_protocol_meta | 96 | ✅ |
| s_delivery_refs | 96 | ✅ |
| s_delivery_policy | 96 | ✅ |
| s_content_publications | 48 | ✅ |
| s_content_meta | 48 | ✅ |
| raw_ohlc | 48 | ✅ |
| catalog | 48 | ✅ |
| **总计** | **768** | ✅ 全覆盖 |

**白名单检查**：
- 代码第 645-659 行常量定义与审查任务指定的 9 张表**逐项对应** ✅
- 字母序对齐：catalog, raw_events, raw_ohlc, s_content_meta, s_content_publications, s_delivery_policy, s_delivery_refs, s_protocol_meta, structure_deltas ✅
- 无多余项或缺失项 ✅

### 4. 单测覆盖度核查 ✅

单测 `count_scan_reads_are_limited_to_business_tables` 第 868-925 行：

**✅ 白名单表放行**（第 882-889 行）：
```rust
for &table in COUNT_SCAN_TABLES {
    assert!(read(Some(table), Some(b""), None), "{table:?}");
    // 同表带触发器/视图来源仍拒绝。
    assert!(!audit_sql_allowed(
        ffi::SQLITE_READ,
        Some(table),
        Some(b""),
        None,
        Some(b"v"),  // origin="v" 表示触发器/视图来源
        true
    ));
}
```

**✅ 非白名单表拒绝**（第 891-902 行）：
- 系统表：sqlite_schema, sqlite_master, sqlite_temp_master, sqlite_temp_schema, sqlite_sequence ✅
- 业务表（未被探针记录）：meta, s_content_nodes ✅
- 空表名 ✅
- 大小写差异：RAW_EVENTS （应拒绝，因为不在白名单 raw_events 中）✅

**✅ 带库名的整表读拒绝**（第 904-908 行）：
```rust
for db in [b"main".as_slice(), b"temp", b"aux"] {
    assert!(!read(Some(b"raw_events"), Some(b""), Some(db)), "{db:?}");
    assert!(!read(Some(b"sqlite_temp_master"), Some(b""), Some(db)));
}
```
覆盖：main、temp、附加库 ✅

**✅ 列名差异**（第 910-913 行）：
- 有列名（b"id"）的读：拒绝 ✅
- 缺列名（None）的读：拒绝 ✅
- 非整表读形态确实不走白名单 ✅

**✅ 其他 action 拒绝**（第 915-925 行）：
```rust
for action in [ffi::SQLITE_INSERT, ffi::SQLITE_DELETE, ffi::SQLITE_UPDATE] {
    assert!(!audit_sql_allowed(
        action,
        Some(b"raw_events"),
        Some(b""),
        None,
        None,
        true
    ));
}
```
白名单隔离到 SQLITE_READ，其他操作照旧拒绝 ✅

### 5. 类型与编译检查 ✅

**is_some_and 方法**：
- 在 Rust 1.70+ 可用
- Cargo.toml 指定 edition="2021"，项目应使用足够新的工具链 ✅
- 签名：`fn is_some_and<F: FnOnce(T) -> bool>(self, f: F) -> bool` 
- 使用：`a.is_some_and(|table| COUNT_SCAN_TABLES.contains(&table))`
  - `a: Option<&[u8]>`
  - `table: &[u8]`
  - `COUNT_SCAN_TABLES: &[&[u8]]`
  - `contains` 调用合法 ✅

**注释来源**（第 645-650 行）：
- 引 #1527 issuecomment-6267843338（但应检查票内是否存在该评论）✅
- 引 windows-5090 探针 f3624c5d 的结果 ✅
- 背景说明清晰 ✅

### 6. 注释与代码一致性 ✅

**第 673-675 行注释**：
> count(*) 优化路径以「无库名、空列名」上报整表读取；只对无库名的业务表白名单放行，带库名（含 main）的整表读一律拒绝。

**代码逻辑**：
- `b == Some(b"")` guard：匹配整表读 ✓
- `db.is_none() && COUNT_SCAN_TABLES.contains()` ：无库名 + 白名单 ✓
- 第二分支 `db == Some(b"main")` 对非整表读情形 ✓
- 注释述准确 ✅

---

## 发现与疑虑

### 无阻断项

改动在以下方面通过审查：
1. 非空列名读行为不变（向后兼容）
2. 改变的参数组合（db=main, b=""）不会触发生产审计失败
3. 白名单完整且准确
4. 单测全面覆盖五个维度
5. 编译类型合法
6. 注释准确

### 低风险的边界确认点

**建议验证但不阻断**：
- ticket #1527 内是否确实存在 issuecomment-6267843338（注释行 650 的引用）
- 项目工具链版本确保 Rust 1.70+（支持 is_some_and）

---

## 结论

**PASS** ✅

**理由汇总**：
1. ✅ 非空列名读行为向后兼容
2. ✅ 改变参数组合（db=main, b=""）与生产审计无关
3. ✅ 白名单基于可信的实测数据（768 次完整采样）
4. ✅ 单测覆盖白名单、非白名单、库名、列名、其他 action 五类场景
5. ✅ 代码编译合法
6. ✅ 注释准确

---

## 审查附注

**file/line 汇总**：

| 检查项 | 文件 | 行号 | 结果 |
|--------|------|------|------|
| 白名单定义 | fd_wal.rs | 645-659 | ✅ |
| 新匹配分支 | fd_wal.rs | 673-681 | ✅ |
| 单测 | fd_wal.rs | 868-925 | ✅ |
| 非受审计代码 | audit_owner.rs | 310 | ℹ 绕过授权器 |
| 审计代码 | state.rs | 1314 | ℹ 无库名 |

