# #1419 独审六项修复候选 v3

工作草稿，绑定 #1419。**五项已修；K1 第四项部分修复，恢复 owner 接入为 PENDING_BUDGET_API。** 未编译、未运行 Rust/Lean/产品，尚待本轮差异独审；不是 APPROVE，不宣称整票完成。

修复起点为干净 `2694afcf147f3ba4114fde386470886c2428efc0`，产品对照仍为 #1488 的 `5cb7253d46d973669e7d10cd3331e12c9ef313b1`。donor 固定 `634f62cddfaddf656a35ca9b9db20cb716b62bff`。本轮修改 14 个 Rust 文件（13 个已跟踪、1 个新增），未提交；相对 5cb 的完整产品差异为 **47 个 Rust 文件、6 份原有夹具**。来源身份逐项见 `SOURCE-MANIFEST.json`。

## 六项结果

| 独审项 | 定位与处理 | 剩余界限 |
|---|---|---|
| 1 TextValue 类型 | `source_contributions.rs:1539` 将 touched 改为 `Tree<TextValue>`；`:1670`/`:1675` 按 `id.as_str()` 查询。插入继续调用原 `owned/native_text`，未转回普通 String。 | 源可证的泛型参数/借用错误已修；Native feature 实际类型检查 NOT_RUN。 |
| 2 必填缓存字段 | 五文件只补 33 行 `interval_cache: None`，逐文件字节与 donor 一致，原测试语句不改。 | 全仓 593 个已跟踪 Rust 文件枚举直接名、LM/LM2 别名及 Self 字面量，找到 35 个相关构造，缺项 0；静态枚举不等于编译。 |
| 3 标签投影缓冲 | `content_producer.rs:2600` 使用借用 `reasons.iter().copied().chain(Option)`；删除额外 to_vec/push，顺序及重复项语义不变。`:4089` 既有完整载荷对拍增加“方向未定 + 同价多根”场景。 | 只消除了本轮新增的标签投影缓冲；既有 CValue 业务包装未整体重写，不能宣称全部 B 包装已持费。 |
| 4 非空 K1 | `second_fold.rs:616` 用新增 `second_fold/wire_match.rs` 借用核验，消除核验时 probe.to_wire 与 wire.clone 两棵临时树；原 16 字段权威 `wire_view` 直接驱动比对。 | **PENDING_BUDGET_API**：`:669` 的恢复登记 clone 仍在，原 PlainJson 不覆盖该副本。待 #1488 非复制许可链最终接口；详见 `K1-OWNER-HANDOFF.md` 的精确调用图、补丁锚点、owner/失败释放及 Native 测试缺项。本项未闭环。 |
| 5 Lean 绑定 | `recursive_tower.rs:145`、`:268`、`center.rs:386`、同类消费者 `descend.rs:89` 明写 donor 完整 SHA 与有效域、当前 Lean 仍旧定义；`tests/center_pair_1424.rs:366` 8 例定位到 donor 导出模块。 | `formal/` 未改、Lean 未执行；最终统一树绑定待根核验。工作例原独立 oracle_pairs.py 在当前及 donor 均缺失，已撤去测试注释中的无条件独立生成断言。 |
| 6 C 续扫引用 | `recursive_tower.rs:343`、`:641` 改引 #812 Z-5、`.chanlun/definitions/zhongshu.md:382` §6.1（裁定首句在 :384）。028:770 原句主语确为“跳空”，未扩成一般突破段依据。 | `last_member+2`、开放尾部 `j`、实际 `i = if j < units.len() { j + 1 } else { j }` 均不变；不重裁。 |

构造器补齐数量：`backtest/econ_positive/tests.rs` 22（含 LM2 两处）、`backtest/runner_tests.rs` 4、`classifier/cand_predicate.rs` 3、`classifier/signal/cert.rs` 2、`classifier/voice_eat.rs` 2。其余两处为 recursive_tower 的生产 from_unit/compose，原已有 Some 缓存。完整定位和枚举方法在 `CONSTRUCTOR-CENSUS.json`；LM 别名的消费者经 from_unit/compose 构造，未发现遗漏字面量。没有整文件搬入无关 donor 增量。

## 本轮实际验证

`CHECK-RUN-1.json` 保存静查驱动原 stdout/stderr/exit，`COMMANDS.json` 保存子命令。14 文件 rustfmt 解析/格式通过；缓存字段、TextValue 借用、投影缓冲、K1 核验去树、保护文件未改等静查通过。最后仅修改数字缓冲的解释注释，`PACKAGE-COMMANDS.json` 再记录该文件 rustfmt 与封包核验，不重跑旧 420 toy 或无关全量项。

相对 5cb 的**产品** `git diff --check -- rust s_session` 退出 0；相对修复 HEAD 的已跟踪增量检查退出 0；**包含旧报告的全树**检查退出 2，命中旧 v2 `SOURCE.patch` 中空上下文行的空格。本轮没有修改旧证据来把该结果变绿。新 `SOURCE.patch` / `DELTA.patch` 使用零上下文，避免同类 artifact 误导；二者只读反向 `git apply --reverse --check --unidiff-zero` 均退出 0。该结果仅证明补丁能反向应用，不证明编译或行为。

新增未跟踪 matcher 另用 `git diff --no-index --check /dev/null <文件>` 检查：真实 exit=1、无空白诊断，直接扫描尾空白为 0；不能将该命令记成 exit=0。首次封包断言误要求 0 的失败记录保留于 `FINAL-STATIC-ATTEMPT1.json`，随后仅更正检查解释，未改产品。最终 53 项产品 hash、复用原件 hash、保护写集核对见 `FINAL-STATIC.json`。

新借用 matcher 只使用当前 K1 的序列化类型；对象/数组长度、字段存在性、整数规范文本、bool/null 都逐项核对，不把缺字段视同 null。已读取本机缓存 serde_core 1.0.228 的 Serializer trait 必需方法，名称无遗漏；这只是源码对照，仍不能替代 Rust 类型/借用检查。生产态没有 JSON clone/to_owned/collect/format! 构造；数学重折叠工作区保持原实现。恢复登记副本未持费的问题未隐藏。

新增/变更的定向锁源码均为 **NOT_RUN**：

- `second_fold.rs:777`：`borrowed_k1_match_agrees_with_legacy_normalization`，用旧 normalized 造树核作测试对照，递归删除/增添/篡改字段与数组，覆盖四种终点、非空成员和整数极值。
- `tb02w_tests.rs:853`：`tb02w_k1_borrowed_restore_keeps_nonempty_contract`，实际非空游标双方向、数字/文本 wire、篡改后重折叠与预算保持。
- `content_producer.rs:4089`：扩展完整组载荷/请求 oracle 对拍场景，同时消费原因与同价标签。
- 五文件 33 处原测试构造补齐；没有新增 `ignore` 或降低原断言。

## 收包边界

`SOURCE.patch` 是相对 5cb 的完整产品候选，`DELTA.patch` 仅本轮相对 2694 的修复。`WRITESET.json`、`SHARED-CALLS.md` 点名真实写集与共享调用；`INPUTS.json` 记录独审、既有报告、参考响应及 donor Lean 的来源 hash；旧 v2 报告保留原样，不描述为已验本轮树。`REPORT-MANIFEST.json`/`SHA256SUMS` 只做本报告封包校验。

未纳 #1394 C1、#1393 W；本轮未动 inclusion、tb02a build(previous)、规则/catalog，也未动 #1488 scratch/work/audit_owner/q/effects。完整 SOURCE 包保留 2694 已继承的 audit_owner 规则门差异，但那不是本轮抢改预算接口。根组合时须使用 DELTA 边界辨认本轮写集，并保 C…F+W 在同一 exact rule 下逐项映射，不能用当前旧 W 口径代替 #1393 增量。

实际 Rust 编译、非空 Native 恢复持费、独审回审、正式 raw→S3/Q 至少两中心一中心对、修订、独立双跑、持久恢复、浏览器、市场域、完整历史及最终资源资格仍欠。默认 ROUND_BUDGET=0 保留；新生成 checkpoint 恒空不抵扣具名非空 K1 合同。≥9/上级构造/趋势/破坏/R5、terminal BSP/γ/K/G 仍按原 deferred 边界留根。

未认领/发评论/改 tracker，未 commit/push/merge/切分支/改 Git 配置，未启动其他工蜂，未读凭据，未运行 cargo/rustc/产品/服务/Long/真实库/历史跑批/Lean。当前身份为外部 Sandcastle Codex 工蜂，不自称父任务原生子代理；真实 session 登记交运行时。
