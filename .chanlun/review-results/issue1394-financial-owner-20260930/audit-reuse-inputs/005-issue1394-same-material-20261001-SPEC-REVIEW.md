# #1394 同字节回读证明复用：独立 Spec 复核

结论：PASS，限固定候选对已采纳设计和 C 合同的遵守。未发现需退回的合同反例。收益、完整源规模 runtime 资源峰值、ScopeRoot 可用性及 C/full9011 均未据此验收。

候选 commit `c6255d308fc69d94178916a1dc5534546fb77591`，完整 tree `eaa252e03f0e292cc594178fcc1fc24f27f437d0`，Rust tree `7196a77fef56cf87cbe91dc4b135326a142e2395`。基线 `818e024fdcfacb94564bd97d0be92890f7ab649b`。五份 tracked 文件干净；本次只写 SPEC-REVIEW.md/json，没有构建、运行全套、修改产品或 tracker。图未索引的既有事实已由 Lead 指定，本次直接读固定路径和差异。

批准来源实际路径为 `/Users/silencehan/Projects/NewChanlun-1394-content-v3/.chanlun/review-results/issue1394-content-v3-20260916/APPROVED-SOURCE.md`，当前 SHA256 `fab912c6ebe3613969d65fc924368a1d5c9b343f0c3f7a123241645413771d30`。采用 Lead 已明确的批准状态；历史文件中的“待裁”文字不被本审查重裁。采纳设计见同工作树 `.scratch/issue1394-same-frame-reuse-20261001/DESIGN.md`。增加 history/state 两文件是落实 audit-only 范围，符合已批准五文件边界。

| 合同 | 复核结果与证据 |
|---|---|
| 每次真实回读 | PASS。`content_store.rs:91` 每次执行 SELECT，当前 SQL error、Missing 和 u8 转换先于 `:272` 的材料检查；命中不能代替查询。Missing/error 后保留旧证明本身不导致成功，后续必须再次读到逐字相同当前素材才可用。 |
| 精确 pure F | PASS。`job.rs:412` 同时比较 requested digest、tag 和完整 body。相等仅复制私有 Copy usage 并移入当前 Vec；不同调用原 `checked`。无 Clone trait、unsafe、Deserialize、unchecked 构造。usage 的 digest/长度均由同一素材决定。 |
| 变化不兜底 | PASS。`content_store.rs:163` 变化先移除旧 proof 并回减 capacity，再按当前字节核 F；坏原件不入表。F 不成立就拒绝，不另换宽松判据。 |
| 默认及预算分支 | PASS。ReadStore new 为 None。history 旧 read_history_metered/read_indexed_history 经 false；私有 audit 入口仅 budget None 启用。state 的第二个 reader 同样仅 None 启用。Some 始终 source.read，保留 SQL 内 F、prepare、第二次 F、accept、precheck 和原计费门。 |
| 当前上下文 | PASS。NodeSource 文档只放行逐字相同的 pure F；s_content 验证逻辑未改。HashReader 仍核请求摘要、deadline 前后门和 overlay 分派。Template/Public、JsonInner、节点角色、header、闭包及独立索引/业务检查不由命中代验。 |
| 完整审计 | PASS。state 控制/raw/history、逐代 source/catalog 和 current catalog 仍逐项执行；history 的控制头、manifest/公开根、重建、对象/辅助索引检查未删。调用完整 audit 的外部文件不在差异中，三次审计没有合并或成功记忆。 |
| 局部资源资格 | PASS。私有 BTreeMap 最多 65,536 条，保存 body 的实际 capacity 总和最多 4 MiB。复制前筛长度/条数，复制后筛实际 capacity。资格满仍执行相同严格 F，不改变请求 cap 或资源错误。变化移除准确回减。 |
| 所有权与生存期 | PASS。当前 SQL Vec 与保存的复制 Vec 各自拥有；返回 Vec 可变不会改保存素材。history 与 source/catalog reader 各自拥有局部 map，随函数返回销毁，不进入 Driver/Runtime/keeper/AuditedState。 |

独立重跑现成 release testbin 两项，均实际运行 1 test、exit 0、0 failed，合计 2 passed。

- `content_store::tests::same_material_actual_transaction_changes_recheck_and_remove_stale_proof`：同一事务真实 UPDATE body 后当前读取拒绝、F 加 1、移除旧 proof、capacity 回到 0；恢复后改 tag 仍拒。
- `v3::history::tests::same_read_frame_keeps_template_public_and_json_inner_rules`：Template 合法的占位符在 Public 模式拒绝，且 F 计数未增加，证明命中没有替代模式门；先合法读取 JSON_TEXT 后将其嵌入 JsonInner 仍拒绝，错误 header 仍拒。

独立 testbin SHA256 `fc5b2630922d8e368baaf752d51e2e243b67760db034ee1465894b9956ab35a6`。精确命令、stdout、退出码见 JSON。两次运行后自由空间分别为 329,216,000 与 328,855,552 bytes，高于 250 MiB；未触发构建或服务。

另复核作者原始日志，第二轮共 24 个不同靶向测试通过，覆盖外部 WAL 连接短读间 mutation、SQL 错误/Missing/tag 越界、返回 Vec 隔离、不同 reader、容量资格、budget 双 F/门序、deadline 以及 state 冷重开损坏等。此处是复用日志证据，不声称本工位重新跑了 24 项。首轮 bin-store/bin-history 的 0 tests 不计通过，即使 BUILD-RESULTS 中记录了命令 exit0/PASS。本次为 Rust 固定候选复核，按 Lead 的 no-fullsuite 要求没有跑 pytest。

当前五源码逐字等于候选 HEAD blob，并逐项匹配 SOURCE-BINARY-BINDING。现存 EVIDENCE-MANIFEST 全部条目 SHA 重核一致。release 非 test rustc 的 opt-level=3、无 --test 及 exit0 已从原执行证据核对，本次没有重新构建。release SHA256 `c098bf76bb7870443b7564714ba2f9110f2c5593a3180ca13f9888c01ee16d17`；candidate.patch SHA256 `677164b5a0ca4e43ab325502853f3ea72d4ad01313fb43a5fe8152124f19068e`，等于当前 git diff --binary。`git diff --check` exit0。原 APFS 575 文件与原源完整性沿用 Lead 指定已核证据，不在本工位重复扫描。

4 MiB 只限制保留 payload capacity。map/allocator 元数据、当前 SQL Vec、首次复制瞬时峰值和完整规模 heap/RSS 保持 UNMEASURED；原跨请求 audit_source_bytes=0 不能说明局部内存为零。此次两个反例及现有 root/header 测试不冒充完整 manifest/control/root 上下文对拍。下一 pilot 仍须同源 S16/完整 DB 副本、Rust 所有返回字段、schema/48,403 rows/全部 BLOB 不变，并实测 SQL 保持、F 变化、素材命中与资格峰值、RSS/可测 heap 和 walltime。原 stage probe 未测纯 hash 核耗时，不能先承诺收益。Spec PASS 不关闭 #1394/#1323、不宣布 C/full9011 或 main 完成。
