两处 `#[cfg(test)] let mut x = x;` 重绑定核对完毕，语义一致。以下是评审结论。

# 评审结论：**PASS-WITH-NITS**

范围 B（`s_session_v2/**`、`s_session_v3/**`、`s_structure_session.rs`、`s_content/**`）在 HEAD 64b8bc5e35 对 `origin/main` 的差异中，**没有发现生产路径行为改变、没有误删有调用方的项、没有违反票面规矩（无新增文件级 `#![allow]`、无 `let _ =` 丢 Result）**。五个关注点全部核过，详述如下；之后是建议处理的小问题（按严重度降序）。

## 五个关注点的核对结果

**(1) w3 删除的 27 项与疑点 1** —— 全部确证无调用方。`cache.rs` 6 个薄包装（make_room/put_root_cert/derived_get/derived_put/mark_reachable/certify_retained_originals）全仓 grep 只剩 `evidence_store.rs:1400/1416` 的 `self.derived_get/derived_put`，那是 `Reader` 自己的方法，同名不同物。`path.rs` 7 个 `_ctx`/`navigate` 包装、`fragment.rs` 的 `public_header`/`hex`（只剩 `FragmentText::hex` 方法）、`tb02a.rs` 的 validate_prices/stored_prices、`s_structure_session.rs` 的 `parse_batch_value`，均无引用。疑点 1：`mod.rs:1635–1683` 的 `validate_audit_phase` 已经完整覆盖被删解构的三项（`key != expected_key` → "phase消息归属错误"；`index >= count` → "attempt phase域错误"；`begin`/`commit` 对 `published` 的两条判定），而且 `message.ok_or_else(|| err("phase没有原消息"))?` 先于后面保留的 `messages.get(id).ok_or_else(..)?`（约 1983 行）触发，所以保留那一行只是冗余、不改行为。

**(2) cfg 门控是否丢生产代码** —— 抽查的门控项与使用面一致：`tb02a::enrich_raw_row` 门在 native，唯一调用方 `s_structure_session.rs:2367` 在 `#[cfg(feature = "s_capture_native_testonly")] fn read_raw_event_at`（2363 行）里；`ControlFacts::len` 门 `all(test, native)`，调用方 `state.rs:2042/2067` 是 native 测试；`read_audit_message`/`validate_audit_message` 门 native，调用方 `audit_owner/q.rs:244/750`；`history.rs` 的 `objects.into_entries()` 只在 `not(native)` 分支用。0af10387de 改 native→json 的约 20 处抽查了 verify_fresh（history.rs:3984 测试）、bytes_extend（business_json.rs:1920/1943 测试）、三处 parse_owned、audit_profile_id、cause_code，调用方均在 json 测试面。

**(3) 无条件 allow** —— 范围 B 约 48 处全部是 item 级、带 `reason = "#1608：…"`、理由具体可查。但「旧包装入口」21 处存在政策不一致，见下面 N1。

**(4) 测试故障钩子** —— `s_allocation.rs` 的 SqlStep 臂改成 `tx.progress_handler(1, Some(|| true)).map_err(|e| e.to_string())?; Ok(None)`，与 SqlPrepare 臂的 `?` 处理一致，安装失败会以 Err 冒上来。测试侧 driver.rs（~2421/2753）、prepared.rs（~1727）、request_audit.rs（~1239）、cross_request_source_cache.rs（~1668/1676）的 `.expect(...)` 安装失败即 panic 测试红，符合「不能静默丢 Result」。

**(5) 语法改写** —— 三处 let-else（q.rs `RawEvents::Plain`、object_index.rs `push_change` 与 q_content.rs `Request::anchor` 的 `JsonOwner`）在 `not(json)` 下不可反驳，因为 `Owned` 变体本身在 `business_json.rs:17–21 / 1418–1422` 被 json 门住，两种构建各取一条分支，语义等价。两处 `let x = …; #[cfg(test)] let mut x = x;`（history.rs:2442–2456 的 `owned` 只在 cfg(test) 下调 set_trace_callbacks/set_overlay_peak_callback；state.rs:897–904 的 `value` 只在 cfg(test) 下写 `capture_ns`）非测试构建拿到的值与改前一致。`HashReader::with_audit_gate`/`for_audited` 由 `pub(crate)` 收窄到 `pub(super)`，调用方都在 `s_session_v3` 内，不影响。

## 发现（按严重度）

**N1｜中｜政策不一致：w2 保留 21 处「旧包装入口」与 w3 的删除标准相悖**
文件：`control_audit.rs`（verify_s3_markers/schema_ddls/verify_s3_schema/verify_s3_publications）、`q_wire.rs`（parse/try_reply/encode）、`history.rs`（revalidate_current_for_reuse/hash_json）、`request_audit.rs`（HistoricalDependencyBinding::read）、`reader.rs`（delta_anchor）、`q_snapshot.rs`（mode/exact/token）、`q_watch.rs`（cursor_at/exact）、`q_content.rs`（exact_view）、`source_audit.rs`（hash/audit_generated_until）、`object_index.rs`（bind_shape_refs）、`audit_store.rs`（StoreView::verify_many）。
理由：这些与 w3 删掉的 27 项形状相同（`fn x(..) { x_admitted(.., None) }` 一类纯转发、全仓无调用方），w3 判「确证孤儿才删」并删了，w2 却用无条件 allow 留下。两套标准并存，下次清告警的人无法判断哪一边是正本。
建议：二选一——按 w3 标准在本票删掉，或在票面/后续票写明「v3 旧入口保留等 X 接线」的具体理由并列清单；现在的 reason 文本只说「旧包装入口」，没有说明为何不删。

**N2｜低｜`commit.rs:118–123` `publish_borrowed_with_workspace` 的 cfg_attr 条件与外层 cfg 同一条件，等于无条件 allow**
理由：`#[cfg(feature = native)]` 之下再写 `#[cfg_attr(feature = native, allow(dead_code))]`，条件恒真，实际是一个被归在「条件 allow」里的无条件 allow；该函数全仓无调用方（prepared.rs:320 直接调 `_admitted`）。
建议：改成 `#[allow(dead_code, reason = …)]` 如实归类，或与 N1 一并处理（删除）。

**N3｜低｜`cross_request_source_cache.rs:19–23` `Binding::read` 同 N2**
理由：整个模块在 `v3/mod.rs:52–53` 已被 `#[cfg(feature = native)]` 门住，再在 item 上写 `cfg_attr(feature = native, allow)` 恒真；无调用方。
建议：同 N2。

**N4｜低｜`state.rs:2356–2363` `audit_owned_capture_for_testonly` 是 `cfg(all(test, json))` + 无条件 allow 的孤儿测试入口**
理由：名字写明 testonly，却没有任何测试调用；门控加 allow 两层叠加说明作者也不确定它该不该在。
建议：确认是否为 #913 意义上的探针；是则在 reason 里写明探针用途，不是则删。`evidence_store.rs` 的 `describe` 与 `s_fact_cache.rs` 的 `last_outcome` 属同类（cfg(test)+allow 的孤儿诊断），处理方式一致即可。

**N5｜低｜验证矩阵缺口：macOS-only 路径与 json+sqlite 组合未编译**
文件：`financial_session.rs:489–490`（`#[cfg(target_os = "macos")] fn sqlite_memory_used`，调用方 `heap_probe` 同门，门控本身正确）；`q_retained.rs:26–30`、`q_runtime.rs:2130–2134`（测试模块门 `all(test, json, sqlite)`）。
理由：作者 7 组远端验证都在 Linux，且没有 sqlite+json、sqlite+native、三者全开。这些组合下若有新告警，本票「清零」的断言就不成立，而 CI 防回潮门（另一位评审负责）如果只跑那 7 组同样测不到。
建议：远端补跑一次 `--features s_session,s_allocation_json_testonly,s_allocation_sqlite_testonly`（低成本，一条命令）；macOS 路径至少在本地非沙盒环境跑一次 `cargo check`。我本机禁跑 cargo，未能代做。

**N6｜信息｜`catalog_evidence.rs:21` 存在既有的文件级 `#![allow(dead_code)]`**
理由：不是本分支引入（diff 中是上下文行），但票面明文禁止文件级 allow，这一处在票后仍会存在。
建议：不阻塞本票；另开票或在 #1608 收尾说明中点名，避免被当成本票合规样板。

**N7｜信息｜`mod.rs` ~1983 保留的 `messages.get(id).ok_or_else(..)?` 冗余**
理由：如上，`validate_audit_phase` 已先报同一错误；留着无害，只是读者会以为它承担校验。
建议：可顺手删，或不动。

## 没有发现的问题（供交叉评审对账）
- 没有新增 crate/文件级 `#![allow]`；没有 `let _ =` 丢弃 Result。
- `rusqlite` `fetch_update` → `try_update` 的替换（audit_owner.rs）语义相同。
- `audit_store.rs` 去掉 `#[allow(unexpected_cfgs)]`、改由 Cargo.toml `[lints.rust] unexpected_cfgs check-cfg` 声明两个 issue1394 cfg，符合票面「未声明 cfg 在 Cargo.toml 声明」。
- `rustc` 的 dead_code 分析会把只从 allowed-dead 项可达的项也判死，所以 `parse_with_allocation`、`hash_json_until`、`verify_mode` 用 `cfg_attr(not(test), allow)` 是对的，不是过度 allow。
