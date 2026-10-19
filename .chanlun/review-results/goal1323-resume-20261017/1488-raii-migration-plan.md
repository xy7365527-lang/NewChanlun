# #1488 Plain 载荷 RAII 最小迁移方案

日期：2026-10-17。状态：**供主控裁定的可实施方案，非已实现、非验收通过**。恢复树 `resume-1488`，原基线 `c2c9c5640a`。源码只读，没有 cargo/编译探针/服务/外盘操作。禁止修改 `construction_state.rs`、`audit_owner/native.rs`、`s_content/capture_allocation/scratch.rs`。

## 结论

在三个禁区不动的前提下，可以让 Native/Plain 载荷真实持许可直至销毁；现有 `NativeReservation` 已有正确 Drop，缺的是**上层值的拥有者**。不能只修改 `audit_store.rs` 的 retain，必须同时改少数通用持有类型以及实际移出这些类型的调用点。

最重要的无禁区接法已经找到：`construction_state.rs` 按值持 `sc::CodecVec<RawWinnerEdit>` 和 `business_json::RawPrefixFacts`；前者是 `capture_allocation/work.rs:167` 的私有 storage 结构，后者的定义在非禁区 business_json.rs。给这两类实际 owner 加许可，其既有 move 路径即自动把许可带入 PendingRawState/BoundRawAdvance，不需要改禁区字段或接口。

“最小”指不改领域判据、不重写禁区、复用现有容器外壳；**不代表三个文件就能闭合整个 production 调用图**。冷审计、生成、来源、对象/辅助索引和最终审计都能返回 Plain 值，必须逐条迁移真实 owning 路径。可以并行分工，但不可把一个垂直切片当成六成功或全图完成。

## 最新 shared_parent 峰值变更复核

R 直接确认其三文件已 rustfmt/diff 检查，测试仅新增未运行；本审查重新读取实际源码。

最初新分支用 `resources.peak() > limit`，仍有域错：此处 limit 是 `effect_limit - set_bytes`，effect_limit 已扣 raw/source。全父峰值包含它们，不能与减过它们的余帽比较。此缺陷已通知 R/主控。

主控现已把 `audit_owner.rs:2035` 附近共享分支改成 `resources.peak() > self.limit`；本次复读确认。self.limit 是 CompleteCandidate 的完整父帽，issue_publication_resources 同时核它与当前冻结 context 一致。`>` 而非 `>=` 有直接依据：native.rs 的 Ledger::reserve_total 接受 `new_live <= self.limit`，Recording 为剩余额度作整额预留，合法峰值可恰好等于全帽。

非共享旧 `candidate + effects.peak >= limit` 原门保留。后半 `candidate.charge.total >= limit` 也应保留：该字段重算只含 candidate 的 capture/historical/currentSQL/q/inline，不含外层 raw/set，与缩小的 validation_limit 对应。`handoff_peak = entry_reserved + budget.peak()` 是外层报告字段，未看到再加 candidate 的准入门。该修复消除确定的二次扣费；没有实际运行结果，不能保证原六成功已经通过。

R 新正控为旧保留 5 MiB、全帽 8 MiB、journal 子帽 64 KiB，真实 Recording.finish 后核局部峰小于 1 MiB、父峰不越 8 MiB；**NOT_RUN**。这只覆盖峰值域，不替代跨代临时载荷释放锁。

## 先冻结的持有契约

### 1. 不改变 Plain/Owned 的名分

禁区 native.rs:106/114/122 已有 `RefValue::Plain`、`RefFields::Plain`、`JsonOwner::Plain` 成功锁。因此保留这些 variant，改变其内部拥有者，不新增一个冒充 Owned 的 Native 路径，也不改禁区测试。

建议在非禁区库文件新增一个小的 Native 持有模块（例如 `s_content/capture_allocation/native_owned.rs`，由既有 mod 导出）：

```rust
pub struct NativeValue<T> {
    value: T,                              // 必须先析构
    permit: Option<NativeReservation>,      // 使用原 WorkScope.reserve_native 签发
}
// 只提供 &T/借用视图、受控 move/transform；不提供任意 DerefMut。
// 不提供丢许可的 into_inner，不提供 infallible 深 Clone。
```

一个独立 String/ContentRef/JSON clone 可在已知结构上界下先 reserve，再构造并移动进 NativeValue。两个 String 的 ContentRef 可共享一次覆盖两块的许可；许可是在构造之前取得，不能事后按“看起来差不多”的 bytes 补签。真实 JSON 的子项可被移出时，需将子项许可一并移动，不能单给父 JSON 一张许可证然后任意 `take()` 子项。

深复制使用 `try_clone_admitted(&WorkScope)`：新旧载荷同时持费，原对象不退账。值相等/哈希/序列化只读 payload，与原 Plain 字节一致。已离域对象可只读和销毁；扩容或深复制需要本次活动 scope，不能保存旧 WorkScope 引用跨请求。

可先冻结的方法名/形状如下（类型草案，不宣称已编译）：

```rust
NativeValue::<String>::try_from_str(text: &str, work: Option<&WorkScope>)
    -> Result<NativeValue<String>, CodecError>;
NativeValue::<ContentRef>::try_from_view(view: RefView<'_>, work: Option<&WorkScope>)
    -> Result<NativeValue<ContentRef>, CodecError>; // 只复制，不提前digest验证
CodecVec::<T>::try_reserve_native(&mut self, work: &WorkScope, additional: usize)
    -> Result<(), CodecError>;
BusinessMap::<K,V>::try_reserve_native(&mut self, work: &WorkScope, additional: usize)
    -> Result<(), CodecError>;
```

Native 构造器应是具体类型的受控构造，不暴露“任意 T + 任意声称字节数”作为计费已正确的凭证。通用内部聚合只负责所有权，不授予材料/预算资格。容器已有 Native 许可后，旧无 admitted 参数的 reserve/insert 仅可在已有容量内成功，需要增长时要求活动 scope；所有实际增长点由调用方显式走 admitted 方法，不能自动重用过期作用域。

### 2. 原外壳携 Native 容器许可

`CodecVec<T>` 保持原类型与 API 可见性，在私有 storage 或末字段持 Native 容器许可；元素的内部费用由元素 owner 持有。`native_codec_room` 改成调用容器的 admitted reserve，不能再 retain。扩容流程：保留旧许可 → 申请覆盖完整新块的新许可 → fallible reallocate → 成功后替换容器许可 → 旧块已真实释放才 drop 旧许可。失败时旧容器/旧许可不变。

必须一起改 `CodecVecIntoIter`：它拥有 Vec 原分配，必须连同容器许可一起接走；未消费元素先析构，再释放底层 buffer 及许可。next() 移出的元素本身仍持自身许可，不能只在迭代器上挂全部元素费用。clear/pop 不释放仍存在的 backing capacity。`try_into_plain` 对带许可的值不能丢许可证：新增显式带所有权转换，迁移真实调用者；无 scope 的旧 Plain 和 Owned 原语义保持。

`BusinessMap<K,V>` 同理增加 Native 桶容量许可，`native_map_room(&mut BusinessMap)` 的外部签名不必变；这保留 native.rs 内既有 map 测试。`insert` 在容量已预留时不得暗扩桶，remove 的 K/V 仍各自持内部分配许可；只清项不假定桶已释放。

`native_std_map_room(&mut std::HashMap)` 和 `native_tree_room(len)` 无法把许可绑定真实 owner，不能保留这两个签名又声称 RAII 完成。迁移到封装 NativeMap/NativeTree，或改实际 owning struct 的私有字段为持费 map/tree；禁止基于地址的全局注册表。BTree 上界可高水位保守持有到树 drop，不按 len 下降猜测物理节点释放。

### 3. 返回值不能在库层已 retain 后再包壳

库层还存在三个直接 retain 面：`path.rs:64` 的 native_reference、`:620` 的字段结果、`closed.rs:690` 的动态结果扩容。现 `job.rs::ScopedFrame::into_frame` / ScopedBody::into_plain 也擦掉 RAII。

因此应添加/改造**带许可的 Native 输出入口**，在 reserve 发生的那层把原 permit 移入输出 owner，再由 bin 接收。不能先调用原 ContentRef 返回接口（已经 retain），再在 bin 新 reserve 同一内容；这会变成双收费且旧费永不退。

保持公开 ContentRef exact 四字段，不在它上面硬塞预算字段。`path.rs` 已有共享 RefOutput/FieldOutput 核，可为 NativeValue<ContentRef> 和带许可字段容器增加输出适配，复用同一遍历/错误优先级。旧无 scope ContentRef 接口继续工作；Native 主链改走新 typed 返回。不要把可含坏 format/root 的传输声明强行换成已核 RefOwner：这会提前验证并改变原错误时机。对原核已确认的 fixed ref 可以另行无堆优化，但不是本次 RAII 的必需前置。

`RefFields::get` 目前会无条件 clone ContentRef；迁移后 Native 调用用借用 view，或 `get_admitted` 深复制并取得独立许可。JSON 的 `take`/移动子项同理：不能让父聚合的唯一许可先释放、子 Value 却裸露在外；优先借用，确需拥有结果则做同 scope 的受控独立复制，或将明确的子项 owner 一起移动。不能在通用 `DerefMut<Value>` 上放任任意拆树。

`content_store` 主链读用 ScopedFrame；确需独立旧 CheckedFrame 的调用点必须改成携原 permit 的 typed frame，不能在旧 into_frame 上加“退出时释放累计数”。job.rs 本身不是禁区，可加不会丢许可的消费转换，保留旧无 scope API。

## 最小可并行文件分工

以下为分工建议，实际派发由主控决定；各组不得覆盖对方文件。先用上述类型/保留 Plain variant/禁止裸 into_inner 的契约冻结一次，再实施。

| 工作包 | 独占文件/职责 | 首个可审结果 |
|---|---|---|
| A：库容器与 typed codec 输出 | 新 native_owned.rs；capture_allocation/mod.rs、work.rs、audit_json.rs；s_content/path.rs、ref_owner.rs、closed.rs、job.rs 及必要导出 | CodecVec/IntoIter、BusinessMap 和 Native ref/fields 能从原 reserve 点带许可返回；不改 scratch.rs |
| B：bin 适配与 raw 垂直切片 | audit_store.rs、business_json.rs、v2/content_store.rs、audit_retention.rs 的新类型支持 | Plain variant payload 持费；RawPrefixFacts/RawPrefixChanges 与原 CodecVec winner_edits 跨禁区 move 后仍持原许可 |
| C：长期业务与生成消费面 | source_audit.rs、source_prefix.rs、source_contributions.rs、source_pending.rs、source_delta.rs、source_order.rs、cross_request_source_cache.rs、history.rs | borrowed view 不复制；移动/替换时费用随新旧载荷；对应 oracle 代码只做相同类型接入，不改判据 |
| D：索引/控制消费面与 owner 对账 | object_index.rs、aux_index.rs、catalog_evidence.rs、native_record_fields.rs、control_audit.rs、request_audit.rs；audit_owner.rs/effects.rs/construction_set.rs | 所有临时 plan/row/refs 真 drop 退账，candidate/set 持久保留不双计；仍同一 publication scope |

A/B 是最先可启动的两个独立包。C/D 可同时枚举调用点、确定变量 owner 与迁移差异，等类型接口落地后修改；不得各自实现第二套 NativeValue。B 中 business_json.rs 同时管文本/JSON/raw，只归一个作者，避免切成两个写入者。

受影响调用点不是只有表中词条：本次精确搜索已发现约 20 个 bin 模块调用 native_*；其中 source_audit_oracle/source_pending_oracle 需配合类型调整。这不授权改 oracle 判据或减少校验。纯测试参数/生成值构造器跟随同文件修改，不动禁区 native.rs 内测试。

## 建议先完成的单条垂直闭环

优先 `raw winner_edits + RawPrefixFacts`：A 给 CodecVec 真实许可；B 改 raw 三张索引表和临时 changed/removed/added 所有者，移除对应 retain。`construction_state.rs:267–317` 的 restore → advance_admitted → changes.winner_edits move → into_facts_retained 原调用形状保持不动。由 business_json 的 owning type 负责最终 drop，不从 RawPrefixFacts 的统计 retained 推断许可金额。

这个闭环小于整个对象/辅助索引迁移，而且直接验证“禁区不动仍可带许可”。但即使它成功，也不能清除其他 Plain 返回值的阻塞。其后按 typed refs/text/JSON → source/history → object/aux 的数据流推进。

## 新旧费用交接必须明示

- 对新 RAII 分配，原 reserve 对象直接 move，不调用 retain；无需第二次预算。
- 对当前 `native::Budget::reserve(...).commit(0, actual)` 已建立的长期费用，只能在可指出原收费点和实际 owning 容器的地方建立精确交接记录；不能用全局 live 差、charge.total 差或重新遍历数据猜 release。
- 不允许同时由 RAII token 和旧 `budget.release(payload_capacity)` 释放同一费。迁移各容器时必须删除对应旧释放，并把统计 RetainedDelta 与资源许可退还区分。
- 如果旧 API 只返回 String/Vec 且费用已 retain，没有可转交 token，就回到该构造点改成 typed 返回；不能在远端消费者创建一个“抵消许可”。
- 不将 Plain 改为 Owned/scratch Layout 域。禁区 native.rs 的 `scope.snapshot().live == 0` 与 `native_observation > 0` 要保持；结构上界仍诚实标注为 Native 上界，不声称严格 heap/RSS。

## 有限验证条件

新增源码锁应覆盖实际所有者，而非重复实现公式：

1. 同一持久父账重复创建/销毁 String/ref/fields，销毁后 live 回到原基线，父 peak 可增长但不当 live；两个同时存活值都收费。
2. Vec/map 扩容失败保旧内容与旧许可；成功峰值覆盖新旧共存；IntoIter 提前 drop 与移出元素仍持费。
3. 原 scope 返回后 typed 值可读/销毁；无新活动 scope 不可扩容；新请求同父 scope 才能继续成长。
4. RawPrefixFacts/winner_edits 经原 PendingRawState/BoundRawAdvance move 后许可存活，真实旧 facts/临时 edit 销毁后才归还。
5. Native ref/path 的成功字节、坏 format、缺字段和 source 错误顺序与旧核同输入对拍，不能靠新拒绝掩盖类型迁移。
6. 费用层闭合后才运行原六 exact 成功锁，16/8 MiB 不改；保留 C>L/2、合法 peak==完整帽、raw/set 非零正控及尾检失败禁止 COMMIT。

本机仍不运行 cargo；源码/合同独审完成后，由主控安排已有许可范围内的具名远端验证。不得用单个 raw 垂直切片、类型检查或新 RAII 单测代替 #1488 与全图交付。
