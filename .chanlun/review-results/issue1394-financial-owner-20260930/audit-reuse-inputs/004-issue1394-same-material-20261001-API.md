# #1394 同字节回读证明复用：候选 API 边界

唯一新增库能力是 `CheckedFrame::checked_same_material(&self, &NodeDigest, u8, Vec<u8>) -> Result<Self, CodecError>`。摘要、tag 和完整 body 全等时，仅复制库内私有 Copy usage，并移入调用方当前 Vec；任一不同重新 `CheckedFrame::checked` 当前素材，失败原样拒绝。没有 Clone trait、unchecked constructor、unsafe 或反序列化构造。

方法自身不证明 SQL 来源，也不证明 Mode/Flavor/shape/闭包/业务准入。NodeSource 文档只允许先实际回读全原件，再复用纯 F 证明。请求 digest 与所有业务 verifier 继续逐次核验。

ReadStore 默认关闭。仅 state 的完整 audit 通过 `read_indexed_history_for_audit` 及 sources/catalog 的 reader，在 BudgetNone 下私有 opt-in；独立 read_history/read_indexed_history 默认原路径。BudgetSome 仍调用 source.read、原双 F、prepare/accept/precheck，所有公开 budget/wire/config 不变。

每个 reader 拥有局部 BTreeMap，随 reader 销毁，彼此不共享。保留 Vec capacity 总和最多 4 MiB、条数最多 65,536，只决定优化资格；满或超大仍原 checked，不新增请求拒绝。复制前按 len/条数筛，复制后按 actual capacity 筛；变化素材先删除旧帧并回减，坏新素材不入表。Missing、SQL error 和 tag 转换错误先处理，旧素材不能兜底。

4 MiB 仅是保留 payload capacity 上限。Map 元数据、allocator、当前 SQL Vec 和首次复制瞬时峰值不属于此上限。完整规模的 runtime RSS/heap/收益均 UNMEASURED，旧 audit_source 跨请求量纲仍为结构性 0。
