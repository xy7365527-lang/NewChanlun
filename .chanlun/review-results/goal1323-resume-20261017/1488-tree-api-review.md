# 代码审查报告

日期：2026-10-17。绑定 #1488。仅审 `.sandcastle/worktrees/resume-1488/rust/src/bin/s_session_v3/audit_store.rs` 新增 `NativeTree::get_mut`、`remove_entry` 与 `native_tree_removed_entry_keeps_payload_charge_after_tree_and_scope_drop`；其他已有改动不重复审查。参照 `1488-tree-consumer-api.md`。

## 概念层质询结果

通过：本增量不改变领域判据。get_mut 借既有值、remove_entry 移出 K/V 及其内部原许可；树自身高水位费仍由 permits 保留，符合此前批准的资源所有权合同。下述测试问题是生命周期观测时点错误，不是另设业务定义。

## 工程层审查

| 严重级别 | 数量 | 状态 |
|---------|------|------|
| CRITICAL | 0 | pass |
| HIGH | 1 | 待修正必要测试锁 |
| MEDIUM | 0 | info |
| LOW | 0 | note |

结论：**WARNING，当前增量尚不能给静态 PASS。**

### HIGH-1：闭包内忽略仍活着的 CodecWork metadata，新增测试会确定失败

位置：`audit_store.rs:1111`。

`baseline` 在 `budget.codec_work()` 之前取得（1091），而 1111 在 `.run` 闭包内部。`Budget::codec_work` 调用 `CodecWork::new`；后者先向同一父账签发非零 `metadata_bytes()`。`Budget::live()` 读取汇总账，包含该 Metadata 分项。`CodecWork::run` 只有等闭包返回后才 drop(scope)/drop(self) 释放这块 metadata。因此 drop(tree) 后应为 **外层 baseline + 当前工作域 metadata + 5 + 7**，不能为 baseline + 12。正常无泄漏实现也会触发这一断言。

证据链：`audit_owner/native.rs:393–407`（汇总 live 与 codec_work）→ `scratch.rs:266–291`（先准入 metadata）→ `scratch.rs:293–306`（闭包返回后才销毁 scope/self）；无需运行即可判断当前断言缺项。所有 Rust 测试依旧 NOT_RUN，没有冒称已观察到失败。

建议：在 run 闭包入口、任何树/文本分配之前记录 `scope_baseline = budget.live()`，1111 改比 `scope_baseline + 12`；1117/1119 位于 run 返回后，保留外层 baseline。不要放宽生产账目或提前释放 WorkScope 来迎合测试。

### 通过的窄 API 核查

- get_mut 直接调用 BTreeMap::get_mut，只给 `&mut V`，不提供键或结构变更，也无 DerefMut。V 的动态字段仍必须自己走准入接口；任意泛型可变引用不替消费者证明深层持费。
- remove_entry 直接按值移动 `(K,V)`，没有 clone、解包 token 或额外 release；树 high_water 和 permits 均不动。缺键返回 None，不改账。后续插入对既有高水位的使用逻辑未改。
- 测试使用的 NativeValue<String> 已实现 Borrow<str>/Ord，`"entry"` 查询类型和 tuple 标量修改在静态上可行；返回 entry 不借用 scope，许可持原 parent，可以跨 scope 保留。除了 HIGH-1，未发现高置信新增类型错误。

## 验证边界和指纹

已读取 staged/unstaged 上下文与精确增量，图工具未索引 NativeTree 后精读文件。`git diff --check -- .../audit_store.rs` exit 0。未运行 cargo/rustc、服务、HTTP、外盘或任何 Rust 测试；未修改源码。不是整树编译或原六成功验收。

```text
bbf779d712870390c38b9ce40dcae2f466a87fef4f96eda99b51ba3057eca1cb  rust/src/bin/s_session_v3/audit_store.rs
```

## 返修窄复审（2026-10-17）

**当前结论：限定静态 PASS，HIGH-1 已消除，未关闭缺陷 0。** 上述 WARNING 与旧哈希保留为首审历史。

已直接读取新测试字节：run 闭包入口在树/文本分配之前记录 `scope_baseline = budget.live()`；drop(tree) 后的内部断言使用 `scope_baseline + 5 + 7`；run 返回后的两个断言仍使用外层 baseline。这恰好区分存活工作域 metadata 与跨域保留文本，没有改生产释放时机或放宽费用断言。

本次只核该修正，没有扩测、没有运行 Rust，也不重复审其他消费者。主控报告单文件 rustfmt 通过；本审未另跑该检查。新增测试及整树编译仍 **NOT_RUN**，静态 PASS 不替代原帽、六成功或整图验收。

新审核指纹：

```text
003bd49836a27816ed4aa456a6394f896ef2c3a8f02bcaea851ec8dcb1a85486  rust/src/bin/s_session_v3/audit_store.rs
```
