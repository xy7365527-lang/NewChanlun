# #1393 Sandcastle run-02 静态实施交付

本轮完成了授权范围内的 **W 初始化产品源码、观察/历史适配、独立 oracle 与定向测试源码**。结论限定为静态实施；**Rust 编译与测试、正式 S 会话/API/浏览器、新进程产品恢复、Lean 检验均 NOT_RUN，不能据此关 #1393、父 TB、SPEC 或整图**。管理者须对实际差异另起独立会话评审，再安排获准的执行验证。

名分：绑定 open 票 #1393 的工作草稿。未暂存、未提交；报告及全部新增文件由管理者收包入 Git。本工蜂没有认领/评论/更改依赖、push、merge、切分支、修改配置、启动其他工蜂或读取凭据。没有把自己声称为父任务的原生子代理。

## 实际源与权威

- checkout：`/Volumes/AgentStorage/issue1393-sandcastle-20261018-worktree`；固定源 HEAD `3315ed1d59ba31ffbad3f0b959702eaa92f4ddbd`。
- HEAD tree：`8e854b779f62e035a0e00caed9ee060257ff15c5`。这是基线树；未创建交付 commit/tree，不能把它当成包含本轮差异的树。
- 改动前工作区干净；结束时暂存区仍为空。完整文件 SHA256 与已跟踪差异 SHA256 见同目录 `MANIFEST.json`；它同时枚举新增文件，弥补普通 `git diff` 不包含未跟踪文件的问题。
- 已读 `CLAUDE.md`、`AGENTS.md`、`.agents/skills/ask-matt/SKILL.md`、`docs/agents/{wayfinder-workflow,delivery-discipline,generation-constitution}.md`、`.sandcastle/CODING_STANDARDS.md`。直接接续已签实施链，没有重开规划或新设批准门。
- 语义权威为本树 `.chanlun/definitions/baohan.md` v1.6 与管理者保存的 #1342 resolution `comment6149622771`；已读 #1393 票面与签署 SPEC 的精确接口/持久合同。未使用暂缺的 `ROOT-RULINGS-1393` 原件的转述创设定义。
- 管理目录 `/Volumes/AgentStorage/issue1393-sandcastle-20261018-management` 中 `issue1393.json`、`comments1342.json`、`dependencies1393.json` 为本轮票据来源。前置 #1373/#1359/#1342 均 closed；未把前置关闭当成本票验证证据。
- 复用 `run-01-static-report.md` 的源码对账及 U01–U13 手算起点；run-01 的网络阻塞发生史、结果及管理目录原件没有被改写。旧 b46 工位未读取或修改。
- 当前工具列表未提供知识图谱接口，代码定位按允许回退使用定向 `rg` 与文件读取。

## 编码前 reference-coding 门

管理者完成检索，本工蜂在写产品之前读取 `REFERENCE-RESULT.md` 和其引用的 JSON 原件；没有在内层重试 HTTP、重装或重索引。准确请求与借鉴范围如下，原件 SHA256 收入 `MANIFEST.json`：

| 原件/索引 | 查询及实际结果 | 本轮用途 |
|---|---|---|
| `xerj-nc1320src-inclusion.json` / `nc1320src-docs` | multi_match `inclusion`，字段 `title^2/body/defs/code/impl`，size=5；total=8，返回5 | 既有增量/历史代码定位；`incremental.rs` 的 classify_at 等。不是 W 参考算法 |
| `xerj-nc1419gate-inclusion.json` / `nc1419gate-docs` | multi_match `IncrFactsState inclusion`，字段 `title/body/defs/code/impl`，size=5；total=1 | `content_structure.rs`，ID `088d49d1a65a785e77b1c0a4c09ec24e`；读取 `restore_second_cursors` 周边接口。仅确认邻线恢复接口存在，未移植为本树源码 |
| `xerj-nautilus-reference-all.json`、`xerj-nautilus-state-reference.json` / `nc1320ref-docs` | match_all size=500，返回409；在 body/code/title/impl 限定筛选 restore/replay/warmup/warm_up/cache/historical | 阅读 `node.py` ID `7a03a018a7fa12f974ba888efb5db3e7` 中完整 `_run_streaming`；对应符号 ID `69a01484a7462917295deb7747d55d5c` |

Nautilus 的实际代码在整个流建立一个 DataBackendSession，将所有 data_configs 登记后逐 chunk 喂给保留的 engine。借鉴仅为“原始流与引擎态分开、状态跨切片保持”的工程原则；没有复制其代码，没有把它说成缠论初始化实现或 oracle。W 判据仍完全来自 #1342/baohan。

## 现有实现与剩余义务对账

| 基线实际情况 | 本轮源码交付 | 尚未获得的证据 |
|---|---|---|
| batch/旧增量已有 raw 相邻首对扫描，但未定 raw 放进 merged | `InclusionResult` 增加 `pending_raw/final_direction`，正式 `merged` 在未定时为空；稠密标注 group/high/low 改为 Option | Rust 执行对拍 |
| facts 首根建 seed，首次包含后永久 unsettled | 完整 facts 与紧凑生产状态共用 `IncrFactsState`：缓存 pending、只检测新增末对、激活时从 raw0 重折；无严格对则永远未定 | Rust 全字段、生产入口执行 |
| 单组合并快照恢复默认 UP | 快照携真实最终方向，`from_result` 不再从末两组猜测或默认 UP | DOWN 单组断点测试执行 |
| 初始化激活改变此前整个前缀 | 激活当次 `merged_confirmed_len=0`；新笔扫描 checkpoint/等待态清空，下一 append 才发稳定前缀证书 | 下游真实缓存路径运行 |
| 观察只有 steps/groups，没有逐 raw 未定身份 | 新增 `CC-004.initialization` 与 `CC-004.undetermined_raw`，首对原件、政策、raw refs、first_known/cut 可查 | 正式 Snapshot/Watch/浏览器 |
| diff 只认识同 slot 替代 | 按 parser 的组成员将每条旧未定记录映射到新组，复用撤回表 `superseded_by`、replaces 及关系边；可多对一 | 正式持久/审计恢复执行 |
| 原内容若重新出现可能碰到旧 withdrawn 对象 | 未定记录把 `pending_since_cut` 写进规范 payload；连续未定沿用出生 cut，重新出现新建结构身份；raw 身份不变 | 正式修订序列执行 |
| 公共 Python/JS 校验只接受旧 types/字段/版本 | 增加必要 W schema，仍核精确字段、政策版本、内容 SHA、来源与生命周期；旧 cut 按旧规则读取 | Python/纯 JS toy 已测；真实链未测 |
| 无 raw 初始化具名 Lean 定义 | 新增 `RawInclusionInitialization.lean`，空/单/首对/全相邻包含/从 raw0 重折证明及 U04/U10 例 | Lean 编译与证明核验；默认 build root 尚未接入 |

### 行为约束

1. 空/单根/全包含/非链相邻包含都没有正式组。未定观察逐 raw 保留，无 envelope、无方向占位对外发布。
2. 首个严格 raw 相邻对的原始左右条目保留在 `initial_direction_evidence`；其前缀坐标也作为初始化依赖，修订此前 raw 能使结构身份更新。建立后每步仍用实际 acc/incoming 更新方向。
3. `contains`、`merge`、`strict_dir`、`fold_step`、`fold_all` 函数体与固定基线逐字一致，静态 SHA 校验已通过。组锚仍为首 raw，OHLC/volume/untradable 沿原核；RD-02 极值根集合仍按到达序保留。
4. 没有更改新笔准入、SECOND、center 判据或 Python 冻结对照 `src/newchan/a_inclusion.py`。新旧正式组的几何对拍源码已提供；**本轮没有运行 Rust，不能宣称已定输入域执行逐位通过**。新政策下初始化元数据与内容对象 ID 本来就会变化，不称公共 JSON 与旧规则全字节相同。
5. 初始化政策是 `raw-initialization-w/1`；raw revision 仍在 input_refs；结构版本仍由 generation/structure_cut 承担；object_revision 沿既有不可变内容对象合同为1，变化产生新对象及替代边，四者不混用。
6. 旧 cut 依然按既有 first_known/withdrawn_generation 投影，不回填。新的多对一边由同一 R2 owner 写入，audit/read_facts 直接复用，未建立第二本历史账。

## 实际写集与共享接口

完整枚举以 `MANIFEST.json` 为准。

| 文件 | 改动职责 |
|---|---|
| `rust/src/theta_v0/parser/inclusion.rs` | W 状态、pending 与正式组分离、快照方向、稠密标注、事实同源、激活缓存证书 |
| `rust/src/theta_v0/parser/mod.rs`、`stroke.rs` | 解析合同说明/独占测试挂载；激活时撤销扫描缓存，不改笔判据 |
| `rust/src/bin/s_session_v2/tb02a_facts.rs` | 初始化/未定对象、policy/首对/发生身份、跨 slot 成员对应、目录观察 |
| `rust/src/bin/s_session_v2/tb02a.rs` | 公共事实投影接受两个新 kind |
| `rust/src/bin/s_structure_session.rs` | 仅追加 W rule token、把 previous 活动集传给 builder、明确 replaces、挂独占测试 |
| `s_session/s_tb02_contract.py`、`s_query_integrity.py` | 新对象与政策绑定校验；在已知旧结构规则后剥离 W token 选择原轴/段 schema |
| `s_session/browser/tb01c-client.js`、`index.html` | 同合同字段/版本校验、两个对象中文名称；未做视觉改版 |
| `rust/src/theta_v0/parser/{inclusion_tb02u_tests,tb02u_tests}.rs` | 手算全字段、batch/incr、每边界恢复、修订、稀疏坐标、1555 小输入域测试源码 |
| `rust/src/bin/s_session_v2/tb02u_tests.rs` | 同 S owner 的 cut/replaces/revision/recover/first_known/rule 门测试源码 |
| `rust/tests/fixtures/tb02u/oracle.json` | U01–U11 静态例及 U12/U13 四切面独立手算账 |
| `s_session/tests/test_tb02u_contract.py` | Python/纯 JS schema、内容身份、政策与原件绑定 toy 测试 |
| `formal/Origin/RawInclusionInitialization.lean` | raw 初始化具名定义、6 条定理和2条具体工作例 |
| 本报告目录 | 报告、纯模型、静态验证脚本、验证结果与交付清单 |

### #1419 组合必须独审的实际重叠

- `tb02a_facts.rs`：`build` 新增 `previous: &[Value]` 参数；新增初始化字段/kinds；目录 CC-004 扩为同前缀对象；新增 `replacing_group`。#1419 的 producer/内容接口需在组合树逐段接线，不能直接覆盖整文件。
- `s_structure_session.rs`：本基线 rule 是 `s2-axis-quantifiers+new-bi/1+segment-second/1`，本轮仅在后面追加 `+raw-initialization-w/1`。**组合时保留 #1419 的 `new-bi/2+segment-second/2+center-seed/1+center-pair/1`，只追加 W token**；不能用本树旧串替换其 C…F 新规则。
- `inclusion.rs`：`InclusionResult`/Ref 新字段；`DenseBarAnnotation.group/group_high/group_low` 变为 Option；`InclusionFacts` 新增 pending/raw首对证据；`IncrFactsState` 未定时 merged/groups 空，初始 unsettled=true，激活当次稳定前缀证书=0。
- `s_tb02_contract.py`/`s_query_integrity.py`/`tb01c-client.js`：本树校验器只认识本基线的 C/D 版本，W token 适配已做。#1419 组合必须在其新版 schema/axis 选择上叠加 W，不能照搬本基线 `/1` 映射覆盖 `/2`。
- 未修改 `content_bi/content_producer/tb02b/tb02e`、v3 `content_structure/source_structure/generation` 或配置。未读取当前1419/1488工位源码，不把本树缺文件说成它们已废。主仓四 WIP、resume-1419、1488/1394 冻结包未触碰。

## AC 与运行状态

| #1393 AC | 本轮状态 |
|---|---|
| 每个未建/建立/修订叶真实运行，保原件/方向依据 | **正式执行 NOT_RUN**；产品源码及 U01–U13 oracle/测试已写，纯模型已验证这些工作例 |
| 已定域逐位保持、几何不变、一般初始化成功 | **静态实施完成，Rust 验证 NOT_RUN**；五个几何/折叠函数体未改，新增全字段与穷举对拍源码 |
| 正式会话非真空输入、公共查询/变化和浏览器核对 | **NOT_RUN**；R2 发布/替代/查询/浏览器契约已适配。toy 校验不替代本条 |
| 独立 oracle、新进程重放、持久边界恢复 | **部分证据**：手算 oracle、纯模型新进程重放/SQLite重开通过；实际产品重放、恢复及故障测试 **NOT_RUN** |

共享 family/A/故事清单的其他量词与跨 TB 义务照原 SPEC 保留；本报告不将它们整项销账。没有为了获得绿灯改 ignore、缩 SPEC 或替换已裁判据。

### 已实际执行

- `python3 .chanlun/review-results/issue1393-sandcastle-20261018/verify_model.py`：4 项通过；11 静态例 + 4 历史切面，1555 个长度0–4的小区间序列（含单点）、逐前缀/序列化恢复/方向对偶；toy SQLite 历史/replaces/重开及独立 Python 子进程重放。不是 Rust，也不是正式 S 的持久测试。
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s s_session/tests -p test_tb02u_contract.py -v`：5 项通过。其 JS 子项仅 Node 加载纯校验模块，3 个合法/4 个非法 toy 对象，无浏览器/HTTP/真实会话。
- `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s s_session/tests -p test_tb02d_validate_once.py -v`：9 项通过；仅对本轮触及的共享校验器做原有 schema/生命周期/hash 上下文回归，未运行其他片全套。
- `python3 .chanlun/review-results/issue1393-sandcastle-20261018/verify_static.py`：PASS；五个既有函数体逐字相同、政策 token 仅追加、Python AST 成功、Lean 无证明占位。
- 9 个本轮 Rust 文件的 `rustfmt --check --edition 2021 --config skip_children=true`：退出0。只证明格式/语法可解析，不是 Rust 类型检查、编译或测试通过。
- `git diff --check` 退出0；HEAD/tree/工作区与暂存区只读核对。

### 明确 NOT_RUN

cargo、rustc、全部 Rust 测试执行、产品二进制、真实会话/API/浏览器、Long/全量历史、真实资金或订单、lake/Lean 编译、fixture drift、CI、独立会话评审。fixture drift 会启动构建，按本轮禁令未执行。Lean 新文件未加入 `formal/lakefile.toml` 默认 roots（本轮禁止动配置）；其具名证明仍须定向编译，并由管理者安排默认验证接线，不能按静态无占位宣称证明已核。

## 后续精确验证合同（建议，不构成本轮执行授权）

1. **先固定源**：管理者收齐 `MANIFEST.json` 文件，检查实际差异后提交。本轮只交未暂存写集；取得交付 commit/tree 并独立评审。与 #1419 合成后重新记录组合 commit/tree/rule、复核接口及消费者，评审不能从当前树外推。
2. **独占 Rust 定向入口**：在获准且已装工具链环境进入 `rust/`，先用相同 target/features 加 `-- --list` 核实际名称，再运行：
   - `cargo test --locked --lib --features backtest_bin tb02u_ --jobs 1`
   - `cargo test --locked --features s_session --bin s_structure_session tb02u_ --jobs 1`
   - 已定域追加两个精确回归过滤：`tb02a_trace_matches_existing_kernel_and_preserves_actual_roots`、`tb02a_direction_is_from_actual_accumulator_not_previous_raw`，同 lib/features，不加 `--ignored`、不扩全量。
3. **真实持久合同**：用本票 toy OHLC 单独 S 存储，依次接受 U12-A/U12-B/U13-C/U13-D；在每个 Advance 前后、after_batch/Commit 边界安排中断与新进程恢复。固定输入/receipt/raw revision、rule、policy、source/features、schema、目录与构建 hash；验证 Snapshot/Watch/AsKnown 的 raw永久身份、first_known、pending_since_cut、组成员/首对、每条多对一 replaces 和旧 cut 不变。另以同事件序列全新进程重放核对象身份及规范批次。测试源码中的同进程 reopen 不能替代这些故障/新进程证据。
4. **公共消费者**：在获准真实会话下读同源 API，浏览器显示未定 raw→组替代，再查旧 cut。新增 Python/JS toy 测试只证明字段/身份校验，不能替代真实浏览器。核未定期无正式组/分型/笔/段，方向长期不出现的非链输入不自行解锁。
5. **Lean**：已预装工具链时，定向 `cd formal && lake env lean Origin/RawInclusionInitialization.lean`，核6定理与2实例；其职责仅为初始化几何分支。未证明 Rust 全字段等价、RD-02或 S 持久协议。默认 roots 接线与 fixture drift 仍须由管理者在获准后处理；不自行重型构建。
6. **资源/监督**：建议独占小输入，单构建任务 `--jobs 1`，记录 CPU/RSS/磁盘、超时、退出码与 stderr；预留6 GiB内存及独立临时目录，编译超过20分钟或超预算即保现场交管理者，不换判据、不加 ignore、不自动跑 Long。正式外效仍禁止；所有失败分清语义、类型/构建、环境及资源原因。

不存在需新增缠论裁定才能继续的语义缺口。剩余为受本轮禁令限制的真实执行证据、独立评审、组合接口及正式封包；管理者负责后续，不以本报告替代批准或验收。
