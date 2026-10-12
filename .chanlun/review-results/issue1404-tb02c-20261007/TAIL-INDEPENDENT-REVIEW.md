# #1404 TB02C 尾部独立代码审查报告

结论：**PASS**。在指定尾部差异中未发现置信度超过 80% 的新增概念矛盾或工程缺陷。此结论支持尾部改动的审阅；同价身份的教义裁定仍属于 #1343/#1405，本报告没有替其定案。

## 固定对象与方法

- 工位：`/Users/silencehan/.codex/worktrees/goal1323-structure-delivery/NewChanlun`。
- 基线：`95165b79d7dea25c2f3b45e666596c03edde4c13`。
- 评审 HEAD：`230f7cdb06feb5710d97a50cffe7cf01b6063041`，已用 `git rev-parse HEAD` 实核。
- 范围：两端 SHA 直接比较，仅 `rust/src/theta_v0/parser/stroke.rs`，184 行新增、10 行删除。下文该文件行号全部属于评审 HEAD。
- 文件 SHA256：`057c1fedc52587c9488bdb27dc6f3e1ddf9da0591a3d7ce265ca36bff3e3c429`，与本轮尾部测试源指纹相符。
- 先尝试 codebase-memory 图发现；当前项目名未索引，Rust 图中又无 `endpoint_at` / `detect_fractal_at`，因此定向读取固定 SHA 的实际 diff、函数及必要的 inclusion/fractal 上下文。未读取旧评审结论或复制旧报告。
- 已读本工位 `AGENTS.md`、`.chanlun/definitions/bi.md`；工位清洁，指定差异的 `git diff --check` 通过。本评审未编译、未重跑测试、R3、服务、166 或 9011，未改仓库、合入、推送或关票。唯一写入为本报告。

## 概念层质询结果

**通过。** 正本 `bi.md:106-117` / `:298-309` 要求在 merged 域不共用 K、在 raw 域按实际顶底根计数、方向有效。实现 `stroke.rs:254-267` 保留三条件：merged gap 至少 3、raw 序位之间至少配置的根数、顶价严格高于底价；`Conditions::formed`（`:39-42`）仍要求三位全真且 Waiting 为空。`bi.md:313-319` 的严格更极端端点取舍及末笔可延伸约束，未在此尾部中变更为宽松成笔判据。

| 核查项 | 实际证据与反例核对 | 判定 |
|---|---|---|
| endpoint Waiting 只归当前判断的依赖 | `endpoint_at:160-166` 在 merged 三组上识别分型后，仅取中心组的 high 根（顶）或 low 根（底）；`:180-187` 仅给这组根的空集/多根加身份 Waiting。分型几何在 `fractal.rs:42-54` 只读取三组标准化高低值，不消费左右组或中心另一侧极值的 raw 身份。因此侧组同价、顶组 low 同价、底组 high 同价不影响当前 raw 间隔的输入。新测试 `stroke.rs:762-791` 明确证明顶 high 根唯一、low 有两根仍合法成笔；`:996-1031` 检查活动右组 high tie 出现及消失时顶实际根仍唯一。 | PASS |
| 原始同价根完整保留 | `stroke.rs:200-204` 仍完整克隆 `extreme_roots`，多根时 `raw_position=None`。`:293-294` 在 PairFact 中保留两个完整端点。`endpoint_at:169-178` 的成员及方向/封口证据展开没有删减。包含层未在此 diff 中修改，`inclusion.rs:563-584` 分别保存组和步骤的 high/low 全部根；`:690-701` / `:887-898` 对同价来源追加而非挑选。移除的是与活动判据无关的阻断条件，不是原始来源事实。 | PASS |
| 同型严格比较不替同价身份裁定 | `stroke.rs:222-246` 依赖整数价格严格比较，非 EQ 时 Waiting 只来自保留端点；被舍弃端点仍在 PairFact 的 old/new 里。`:410-424` 仅在尚无笔且 Waiting 全为 `raw_extreme_identity_tie` 时允许替换活动端点，替换的是完整端点对象。新夹具 `:866-908` 含上下镜像，检查新活动端点两根 `[56,70]` 和 `raw_position=None` 都保留，之后继续遇到真正唯一端点才成笔。没有为新根选择早根/晚根。 | PASS |
| Unknown 不得择根放行 | raw 间隔只在两端 raw 序位都存在时计算（`:255-258`）；多根仍产生 None。三位 `[true,None,true]` 不满足 `formed`，且 `:452-459` 阻断。跨门槛反例 `:838-862` 的两根分别给出 raw 间隔 3 和 2，明确要求无笔、保留 `[7,14]` 和 None。即使多根的各种间隔都够长，现实现也仍保持身份 Unknown，未自行增加任选根或存在量词准入。 | PASS |
| 移除 blanket poison 不漏等价竞争 | 同型 EQ 的 `UNDETERMINED`（`:232-233`）、保留双锚（`:301-302`）以及阻断/撤尾（`:393-405`）均未改变。异型 pair 有 false 时，`:452-459` 仅取消后续扫描的冻结：当前 pair 仍留在 `out.pairs`（`:461`），且仍不成笔。反例 `:796-833` 保留 `[false,None,true]`，随后价格严格更极端且根唯一的端点才替换并成笔。false 来自 merged 间距或顶底价时，与未知根选择无关，所有根选择下此 pair 都失败；不存在用宽松条件重判该失败 pair 的分支。若合取没有 false，身份 Waiting 仍阻断后续；若同型价格 EQ，仍不选择任何分支。 | PASS |
| 延伸全扫与增量 checkpoint 一致 | 两路径都调用 `endpoint_at` 与 `ScanState::apply`（`:359-361`、`:588-597`）；`:417-420` 的延伸仍重新调用同一个 `pair`，必须经同一个 `Conditions::formed`。checkpoint 冻结范围为右组已稳定的 mid（`:581-605`），先恢复尾笔、current、blocked 及各事实长度（`:496-508`）。新增转移至多改旧尾一笔，没有破坏该备份边界。`check_incremental_prefixes:704-741` 每个 raw 前缀比较公开 Stroke、去掉纯来源展开后的完整 StrokeFacts、稳定前缀及每 bar 至多两次扫描；新夹具均调用它。`:1036-1068` 原有撤尾/恢复/延伸反例保护前笔确认字段。 | PASS |
| #799 / #804 一致性 | `AGENTS.md:48-58` 禁同一判断的宽松兜底；`:66-80` 要求同输入同输出的实际对拍。此 diff 未改变三个准入条件或 `formed`，未新增替代成笔入口。same-kind 严格价格取舍与异型成笔是不同判断；false pair 后继续处理新端点也没有重判同一个失败 pair。全扫/增量仍共用判定核，已有实际逐前缀测试日志支持一致性。 | PASS |

谱系比对：定向检索 `.chanlun/genealogy/` 的同价、#1343、#1405及宽严兜底相关内容，没有发现能将本次尾部识别为既知矛盾的同价裁定记录；不据此宣称教义已结算。当前正本与任务明确保留的同价身份未裁状态没有被本差异覆盖。

## 无同价可达域与旧 R3 复用

本项做了独立的静态域检查，没有重跑 R3。固定 HEAD 的 `s_session/tests/fixtures/tb02c/raw-ledger.json` 与基线字节完全相同，SHA256 为 `32d00df0b3e349411b79e34e284242bce5b63dce97a03b6cb2eb17a8dec27aa4`。本评审直接读取两端 Git blob，并逐对读取 first_up / first_down 的高低值：每方向 28 根 raw、27 个相邻对，27 对全部高低严格同向，raw_index 均唯一。

因此每个前缀都无包含，每组只有一根 raw：所有 high/low 根唯一，raw 序位映射完整。逐 hunk 检查后，旧版与新版 endpoint 的身份 Waiting 都为空；非 EQ 同型 pair 的 Waiting 都为空；新增的尚无笔替换条件仅重复原本可走的空 Waiting 分支；异型失败 pair 在两版中也都没有 Waiting。因此这些冻结输入上的端点、pair、笔、确认及下游输入逐字段不因本尾部差异改变。若出现同型 EQ，两版仍走相同阻断分支；根唯一并不需要假定同型价格绝不相等。

已直接读取 `/Volumes/AgentStorage/issue1404-formal-r3-20261007/` 下 first_up-a / first_up-b / first_down-a / first_down-b 四轮的小型原始收据：各 `EXIT.json` 为 exit 0，公共查询 oracle 为 28 cuts passed，浏览器为 cut 16/28 passed 且无 page_errors，normal-recovery 的 cut 16/28 equal=true。另读本轮 `structure/r3-small-receipts-check.json` 的 20 份小收据摘要核对记录，均 match=true。捕获阶段 `evidence/RESULT.json` 自身仍写 oracle/browser not_evaluated，实际通过依据来自后续独立 oracle/browser 收据，未把 CAPTURED 当作验收。

**复用边界**：这支持上述冻结 R3 输入对本尾部不受影响，以及旧 R3 证据继续适用；不是本 HEAD 新启动服务/浏览器的运行证明，也不把有限的无包含夹具提升为全市场或所有同价输入的逐位等价结论。未读取、复制、重哈希持久数据库或大 core 原件。

## 工程层审查

概念层通过后，核对生产 hunk 的转移、副作用、来源保留、raw 稀疏坐标、旧尾备份、准入空值和测试覆盖。未发现新增错误处理、安全、性能或测试缺口达到可报告阈值。文件/函数既有长度不是本差异的新缺陷，也不单凭长度制造 HIGH。

| 严重级别 | 数量 | 状态 |
|---|---:|---|
| CRITICAL | 0 | pass |
| HIGH | 0 | pass |
| MEDIUM | 0 | info |
| LOW | 0 | note |

## 实际验证证据及限度

- 复用 `/tmp/goal1323-tb02c-tail-tests.log:549-565`：13 个 TB02B 测试 passed。更强的本轮记录为 `/tmp/goal1323-native-20261012/structure/tail-test-execution.json` 与 `tail-tests-verified.log:514-530`：实际进程 exit 0、HEAD 与本报告相同、源文件前后未变且工位 clean；13 passed。独立复核源指纹及日志 SHA256（`24875bef608498017f84fa94e49b6b205bf102ed039d5191c1e29edfba1479ca`）均一致。
- 本轮尾部测试包括四个新增身份依赖夹具、既有账簿稀疏坐标/每前缀对拍、撤尾恢复延伸及确认冻结。长波记录 `tail-tests-verified.log:527`：1200 bars、2395 scanned_mids、478 endpoint_transitions、238 strokes，具备实际非空结果及线性扫描锁。
- 复用 `/tmp/goal1323-tb02c-fixture-tests.log:514-524`：8 个 TB02C 夹具 passed，0 failed。未将该旧日志包装退出码推断成新进程退出码。
- 此次没有新运行构建或端到端服务；测试与 R3 的动态事实来自列明的原始运行材料，代码推论来自固定 SHA 的实际实现。

最终裁决：**PASS**，无阻断发现。
