# ADR 0056：三模式同核与 adapter 边界（历史回放 / paper / live）

> 正本：本文件是 #1538（FU-16）三模式共享核心、feed/clock/matcher-venue/传输/受控持久化 adapter 的 owner 落点、E 接纳原件单一 schema 与跨模式同输入等价三层测试锁的正本。
> 最后裁定：#1538（2026-10-28，用户 P 类批准，[批复评论](https://github.com/xy7365527-lang/NewChanlun/issues/1538#issuecomment-6472079617)）
> 受影响代码清单：见本文「受影响票与 blocking 边变化」节「受影响代码清单」段（`store.rs` / `book.rs` / `production_session` / `x_execution_session`，实施归 #1769、#1381、#1770、#1425）

**状态**：已采纳（P 类，用户批准），文书待 main 合入。本 ADR 只定边界、schema 与判据；实现与运行证据归消费票，不以本 ADR 声称实现完成（同 ADR 0052:6）。
**裁定依据**：#1538 批复评论。原件：草稿、独审、考古、分级与汇总存档于 `.chanlun/review-results/issue1538-decision-20261028/`（源 `/Volumes/AgentStorage/decisions/1538/`）。两轮独审均 BLOCK（R1/R3/R6），按 graph-engineering.md:73 转 P 类，由用户批复生效。
**行号基线**：草稿行号以起草时 main 为准；本文照搬草稿行号，不重定位。落地基线 main `3087582e45`。

## 裁定

用户 2026-10-28 批复：「1538 四条全批」（[#1538 评论 6472079617](https://github.com/xy7365527-lang/NewChanlun/issues/1538#issuecomment-6472079617)，`/graph-approve`）。四条按草稿推荐原样生效；B 类默认值全部采纳，A 类按出处沿用，分九条落为本 ADR，并在 CONTEXT.md 补一条词条。

正本落点：docs/adr/0056-three-mode-shared-core-adapter-boundary.md

### 一、共享核心（沿用 A）

三模式共用 #1340 C10.2（:345）五项：核心 reducer、身份（ADR 0051 `domain-event/1`）、责任、执行生命周期、观察协议。两个核心 owner 的实现只有一个（ADR 0027:46）。可比较的全状态就是 ADR 0038:49-54 六面。任何模式特有的结构判定、重状态推进或交易决策程序都违反 ADR 0027:46。

### 二、模式与装配

| 模式 | `evidence_class` | 依据 |
|---|---|---|
| 历史回放 | `simulation`（P0） | ADR 0052:250；ADR 0028:28 |
| paper | `experiment` | ADR 0028:25 的 `P1-hl-testnet` 取 `experiment`；ADR 0039:284 正式 paper 是 X 内 HL adapter，而 :272 只在 experiment/production 装场所 adapter `[新缠论:推论]` |
| live | `production` | CONTEXT.md:374；ADR 0039:272 |

装配只由 `production_session` 按 profile 的 `evidence_class` 和 manifest 的「adapter 选择」段决定（ADR 0039:270/:272），版本内不切换（ADR 0028:28）。各域进程不自选 adapter。

### 三、adapter 替换表（owner 落点）

| adapter | owner 进程 | simulation | experiment / production | 三模式不变的部分 |
|---|---|---|---|---|
| feed·结构源 | S（经 S.AcceptInput） | `history-input-profile/1` 的 `structure.source_ref`（ADR 0052:264；`input_source` 按 ADR 0052:306-308） | 个股线实时流（ADR 0023:11/:30-34） | C02.1 原件形状（:259）；源声明在 init 持久化，运行中不切换（#1769:13） |
| feed·执行行情 | X（adapter 选择段） | 受控撮合消费的行情 | 场所行情（#1524 交付） | 永远不进 S；按来源身份判别（#1769:12；28G:170） |
| clock | manifest `clock` 段，各域按其读 | `logical-bar/1`（ADR 0052:296） | `System`（`store.rs:317-318`） | 身份不含 wall-clock（ADR 0051:172）；`TestOnly` 不进 `production_session` |
| matcher/venue | X 进程内（ADR 0039:284-285；C09.7） | 受控撮合，规则 `cross`（#1718:17-18），附 ADR 0052:279 不变式 | X 内场所 adapter；能力证据按 C10.5（:348） | X 唯一调用边；同一 ExactCommand 投影 |
| 传输 | X 对外一段 | 进程内调用，零网络 | 场所 HTTP/WS，按一次调用合同（C09.7） | 域间 `session_transport` 同一实现（C09.1；`session_transport.rs:1-3`） |
| 受控持久化 | 各域单写者 | 同一 SQLite/WAL 实现 | 同一实现 | 只换 DB 路径；启动前件不满足即拒写（C09.2 :331）；锁与 epoch 见 ADR 0039:292、`store.rs:653` |

场所账户、权限与具体端点归 #1540（FU-18）。成交/持仓唯一真账与 crosswalk 归 #1539（FU-17）。本表不定这两项。

### 四、E 接纳原件只有一份 schema

E.CaptureExternal（INTERFACE-CONTRACTS:35）对三模式只认一份原件 schema：ADR 0051:244 的 `source`，加 `book.rs:117-124` 的必填项（`fact_key` 在 `:124`，定义在 `:93`）。`format` 是来源标签，取三值之一：受控撮合原件、场所 adapter 原件、TestOnly 夹具原件。E 校验 `format` 与会话 `evidence_class` 一致（TestOnly 夹具原件只配 `TestOnly` 时钟的组件会话），不一致返回 `SchemaUnsupported`，不落行，不改走其他校验。受控撮合和场所 adapter 产出同一形状。本条对所有具名场所 adapter 一视同仁（HL、#1382 的 IBKR paper、#1400 的 Binance 只读查询都产「场所 adapter 原件」），场所、账户与权限的绑定仍归 #1540/#1351。

回报录放会话（裁定七 (c)）是 `simulation` 会话，时钟为 `logical-bar/1`，E 只接受「受控撮合原件」：录放 matcher 把录制件重发为受控撮合原件，`format` 改标为「受控撮合原件」，`source` 四元（`source_namespace`、`source_epoch`、`external_fact_id`、`source_cursor`）原样保留，`received_time` 由逻辑时钟重写。「TestOnly 夹具原件只配 `TestOnly` 时钟」与录放会话无交集：录放会话由 `production_session` 拉起，按裁定五第 2 条不可能配 `TestOnly` 时钟，出现 TestOnly 夹具原件即 `SchemaUnsupported`、不落行，并因行数不等使裁定七 (c) 判 FAIL。

### 五、时钟与期限

1. 时钟 kind 是封闭枚举，由 `evidence_class` 一一推出：`simulation→logical-bar/1`，`experiment/production→System`。不设可选值。
2. `production_session` 遇到 `TestOnly` 时钟，或时钟 kind 与 `evidence_class` 不符，在拉起任何域之前以 `InvalidDomain` 退出。
3. `fault_schedule`（ADR 0052:300）只在 `simulation` 会话合法。`store.rs:263` 现在的 `TestOnly` 绑定只保留给直调域 bin 的组件夹具。
4. `job.rs` 的 `Clock` 和 `TransportBounds` 是资源闸，三模式实现不换，数值走 manifest `resources`，进 `content_hash`。期限触发即失败，不重试换判据（ADR 0027:32）。

### 六、权威排他（沿用 A）

一个会话只有一个 `evidence_class`，也只有一个外部事实源（C10.2 :345；ADR 0052:250）。模拟 fill 与真实 fill 不得同会话并存。混源只能出现在另一个会话里（见裁定七 (c)）。simulation 会话的产出不进 TStage 触闸计数（ADR 0038:74），分证不交叉引用（ADR 0038:75）。

### 七、同输入等价的三层 oracle（测试锁）

- **(a) 同模式**：沿用 ADR 0038 H1/H2（:80/:81），本 ADR 不重裁。
- **(b) 跨模式前段**：
  - 同一份 S 结构原件集，同一组 policy/model/seed/scheduler，simulation 会话与 experiment 会话的第 1 面（S 结构快照）逐字段相等。
  - 同一 ExactCommand 夹具经 X 唯一调用边，分别走 simulation adapter 与 HL adapter（本地夹具端点）。X 侧调用记录除 adapter 身份与场所回执外逐字段相等，ADR 0039 A4 写权闭包 diff 为空（28G:220）。
- **(c) 跨模式后段，回报录放**：
  - 输入：取一次 experiment/production 原会话里 S 已持久的结构输入记录，以及 X 外效 journal 里的场所回报、更正、bust、持仓回报。
  - 重放：在新的 `simulation` 会话中以 `model_ref.model_id = recorded-venue-replay/1` 运行。录放 matcher 收到 X 调用边上的 ExactCommand 投影后与录制命令比对，相等则按录制件重发回报（`format` 改标、`source` 四元原样、`received_time` 由逻辑时钟重写，见裁定四末段）；不等即硬错误停进程，不回落到 `cross` 撮合（同 ADR 0052:279 不变式）。
  - 比较面与时点：ADR 0038:49-54 六面全部字段；时点为每个 bar 收盘后 checkpoint（ADR 0038:56）。各面导出按（域, `event_id`）排序，因此同一 bar 内的行序不参与比较（ADR 0038:56 批内顺序 `Unknown`）；提交序号的值另列为下表第 8 项。时间戳与提交序号分属第 3、8 项，各自独立配对，不互相替代（INTERFACE-CONTRACTS:9）。
  - 判定程序：对两次运行在每个 checkpoint 导出六面，把下表「重命名闭集」作为一一映射施加到原会话导出上，再与重放会话导出逐字节比较。
  - **重命名闭集**（共八项；每项都是两种会话装配不同而必然产生的差异，原会话一个值只对应重放会话一个值，反之亦然）：

    | # | 字段 | 出现面 | 原会话取值 | 重放会话取值 | 必然不同的成因 |
    |---|---|---|---|---|---|
    | 1 | 会话 ID、运行 ID | 六面记录头 | 原会话 ID | 重放会话 ID | 两个会话；C10.4（:347）点名的非语义字段 |
    | 2 | `make_token` 的 PID:ns | Begin token | 原进程 PID 与纳秒 | 重放进程 PID 与纳秒 | `s_structure_session.rs:126-133` 取进程号与系统时间 |
    | 3 | 时钟写出的 `received_ns`、`first_known_ns`、`commit_ns`（`service.rs:686-688`）及 E 原件 `source.received_time` | 第 1–6 面中带这些字段的行 | `System` 读数 | `logical-bar/1` 读数 | 裁定五第 1 条按证据类绑定时钟；映射按（域, `event_id`, 字段名）逐条配对 |
    | 4 | 证据类与时钟、模型回显：`evidence_class`、`clock.kind`、`model_ref` | profile/manifest 回显 | `experiment` 或 `production`；`System`；原会话撮合模型 | `simulation`；`logical-bar/1`；`recorded-venue-replay/1` | 裁定二、五：录放只在 `simulation` 合法 |
    | 5 | X adapter 身份（adapter 名称与端点标识） | 第 6 面 journal 条目 | 场所 adapter（如 HL adapter） | 受控撮合（录放 matcher） | 裁定三 matcher/venue 行 |
    | 6 | E 原件 `format` 标签 | 第 4 面原件 | 场所 adapter 原件 | 受控撮合原件 | 裁定四：`simulation` 会话 E 只收受控撮合原件 |
    | 7 | adapter 线格式请求体（实现若在 journal 里记录） | 第 6 面 journal 条目 | 场所 HTTP/WS 请求体 | 无（录放 matcher 不出网） | 线格式在 X 调用边之后、属 adapter 自身输出；它另由 (b) 的本地夹具金样锁（28G:273）逐字节判定 |
    | 8 | 持久提交序号 | 各域写入行 | 原会话序号 | 重放会话序号 | 同一 bar 内的到达与提交次序在 experiment/production 由场所与网络决定，ADR 0038:56 已裁批内顺序为 `Unknown`、不作比较维度；映射按（域, `event_id`）配对，且须保持 bar 归属与跨 bar 次序：原会话第 k 个 bar 内的行只能映到重放会话第 k 个 bar 内的行 |

  - 闭集之外的字段一律原值逐字节相等，包括：`source` 四元、`fact_key`、`allocation_id`、`family_id`、`target`、cause 链、订单/成交 epoch 与前后状态、数量/价格/费用、场所回执内容（如 `exec_id`）、物理持仓、各重 B 账与 E 责任、S 结构对象与修订链。
  - **PASS** 当且仅当以下五条同时成立：施加映射后六面逐字节无差异；每项映射都是一一的（无两个原值映到同一重放值，也无反向）；被改名的值只出现在表中所列字段位置；每个 checkpoint 六面行数相等；重放全程未触发命令不符硬错误。任一不成立即 **FAIL**。
  - 这是 #1382:12「各核心语义输出逐字段相等」的二值落法。禁止只比净仓（C10.4 :347）。
- `recorded-venue-replay/1` 只在 simulation 对拍会话中合法，产出是 simulation 证据（ADR 0038:74），不作正式成交或持仓来源。

### 八、旧外壳与旧设计票（沿用 A）

- 现有 Python/NT 外壳只做模式输入、协议与展示适配（C09.1 :330），不得成为第二个 launcher（chong-traceability.md:52；ADR 0039:285）。逐条去向归 #1550 与 TB-10 #1368。
- #1329 只登记本 ADR 的指针，不解除暂停，不关票（评论 5596449009；C12.3 :382）。

### 九、词汇

CONTEXT.md「运行边界」节（:364）补一条「同核 adapter（Shared-Core Adapter）」：只列六类名称（结构 feed、执行行情 feed、clock、matcher/venue、传输、受控持久化）及各自 owner 进程，不写接口。

## 验收清单

（`<ADR>` 代表 docs/adr/0056-three-mode-shared-core-adapter-boundary.md。本票只验文书、票状态与边；运行类测试是消费票的验收，见下一段，不作本票关票条件。）

- [ ] 文件存在：`test -f <ADR>` 退出码 0。
- [ ] `rg -c '^正本落点：docs/adr/' <ADR>` 输出 = 1。
- [ ] `for t in logical-bar/1 System TestOnly evidence_class recorded-venue-replay/1 重命名闭集 simulation_and_hl_adapters_share_the_x_call_edge E.CaptureExternal session_transport SchemaUnsupported InvalidDomain; do rg -F -q "$t" <ADR> || echo "MISSING $t"; done` 输出为空。
- [ ] `rg -F -c '同核 adapter（Shared-Core Adapter）' CONTEXT.md` 输出 ≥ 1（当前为 0）。
- [ ] 票状态：`gh issue view 1538 -R xy7365527-lang/NewChanlun --json state --jq .state` 输出 `CLOSED`。
- [ ] 票状态：`gh api repos/xy7365527-lang/NewChanlun/issues/1538/comments --paginate --jq '[.[]|select(.body|test("^用户 [0-9]{4}-[0-9]{2}-[0-9]{2} 批复"))]|length'` 输出 ≥ 1（P 类 resolution 首行格式，见 `.agents/skills/graph-approve/SKILL.md:16`）。
- [ ] 承接登记：`for n in 1329 1381 1382 1400 1523 1524 1525 1769 1770; do c=$(gh issue view $n -R xy7365527-lang/NewChanlun --comments | grep -c 'FU-16 承接登记'); [ "$c" -ge 1 ] || echo "MISSING #$n"; done` 输出为空（当前九票都是 0）。
- [ ] #1329 不被关：`gh issue view 1329 -R xy7365527-lang/NewChanlun --json state --jq .state` 输出 `OPEN`。
- [ ] 边不删：`gh api repos/xy7365527-lang/NewChanlun/issues/1538/dependencies/blocking --jq 'map(.number)|sort|map(tostring)|join(",")'` 输出 `1362,1366,1367,1525,1539,1540,1550`。
- [ ] blocker 已解除：`gh api repos/xy7365527-lang/NewChanlun/issues/1538/dependencies/blocked_by --jq '[.[]|select(.state=="open")]|length'` 输出 `0`。

## 受影响票与 blocking 边变化

**受影响票**

下列测试名是建议名，由各票 owner 实施时固定。判据统一为 `cargo test <完整路径> -- --exact` 的输出含 `1 passed`。

- **#1769**（OPEN，`production_session` 与 `x_execution_session` 骨架）：消费裁定二、三、五。时钟枚举补 `logical-bar/1`；时钟 kind 与 `evidence_class` 绑定，不符时在拉起域之前 `InvalidDomain`；`production_session` 拒收 `TestOnly` 时钟。建议测试：`clock_kind_bound_to_evidence_class`、`production_session_rejects_testonly_clock`、`simulation_store_keeps_wal_full_sync`。
- **#1381**（OPEN，TB-07-A）：消费裁定五第 3 条。`fault_schedule` 在 simulation 会话下触发，不依赖 `TestOnly` 时钟。建议测试：`simulation_fault_schedule_without_testonly_clock`。
- **#1770**（OPEN，X 内 HL adapter 离线合同）与 **#1425**（OPEN，TB-04-F 回报应用）：消费裁定四。场所 adapter 和受控撮合产出同一 E 原件 schema；E 校验 `format` 与 `evidence_class` 一致。建议测试：`matcher_and_hl_adapter_emit_same_e_raw_schema`、`e_rejects_raw_format_mismatching_session_evidence_class`。
- **#1525**（OPEN，TB-04 纵片 C）：:26「paper/live 共用同核证明」按裁定七 (b) 落成 `simulation_and_hl_adapters_share_the_x_call_edge`（28G:220）。
- **#1382**（OPEN，TB-08-A）：AC :11/:12 按裁定三、七 (b)(c) 落值。其 IBKR paper 准备登记（[评论 6405903436](https://github.com/xy7365527-lang/NewChanlun/issues/1382#issuecomment-6405903436)「新adapter边界仍受#1538约束」）按裁定二（paper 装配为 `experiment`）、三、四消费，不另立场所专属规则。建议测试：`cross_mode_structure_face_equal`、`recorded_venue_replay_six_faces_equal`、`recorded_venue_replay_command_mismatch_is_hard_error`、`recorded_venue_replay_unlisted_field_diff_fails`（在闭集外字段注入一处差异，须判 FAIL）。
- **#1400**（OPEN，TB-08-B 具名场所只读能力与事实完整性接入会话）：其准备登记（[评论 6405905587](https://github.com/xy7365527-lang/NewChanlun/issues/1400#issuecomment-6405905587)）写明「完整adapter边界受#1538约束」。交付（正文 :7 身份、订单、持仓与回报前沿查询；:12 逐项能力证据与「事实唯一来源」）消费裁定三 matcher/venue 行（查询走 X 内场所 adapter，X 是唯一执行网络请求的进程，C09.7 :336）、裁定四（查询结果以「场所 adapter 原件」进 E）、裁定六（单一外部事实源）；能力证据形状沿用 C10.5（:348）。本票不改其 AC；它已经 #1382、#1366 被本票传递阻塞，不加直接边。建议测试：`venue_readonly_query_enters_e_as_venue_adapter_raw`。
- **#1523**（OPEN，纵片 A：HL 适配器与正式 paper launcher）：批复评论 6452353777 保留的「paper/live 同核」义务，按本 ADR 裁定一至七消费：paper 装配为 `experiment`，X 内 HL adapter 遵裁定三表，同核证明按裁定七 (b)(c)。本票不改其 AC，也不改其边。
- **#1524**（OPEN，纵片 B：HL trade tick/1m feed 与断线恢复）：交付受裁定三表「feed·执行行情」一行约束。HL feed 只进 X 的 adapter 选择段，永远不进 S；按来源身份判别（#1769:12）。:7「与 #1304/#1305 回放 driver 共核」按裁定一、八读作：共用同一核心，不另立 driver 主干。
- **#1411**（OPEN，TB-07-C）：裁定七 (a) 沿用 ADR 0038 H1/H2，不加新义务；裁定五第 4 条的期限数值进 `content_hash`。
- **#1539**（FU-17）、**#1540**（FU-18）、**#1550**（FU-19）：被本票阻塞的裁决票，分别消费裁定六（单一事实源）、裁定三（场所 adapter 位置，账户/权限仍归 #1540）、裁定八（外壳只做 adapter）。
- **#1362**（TB-04）、**#1366**（TB-07）、**#1367**（TB-08）：被本票阻塞的汇总票，关票后边自然满足。
- **#1368**（TB-10）：Python 外壳终态实施沿用 chong-traceability.md:52，不变。
- **#1329**（OPEN，暂停）：登记本 ADR 指针，不解除暂停，不关。
- **#1340**（SPEC）：C10.2/C10.4 由本 ADR 落成接缝合同，SPEC 本身不改。

**受影响代码清单**（下游实施，不在本票改）：

- `rust/src/economic_session/store.rs:253-258`、`:263-264`（时钟枚举与故障绑定，归 #1769/#1381）；
- `rust/src/economic_session/book.rs:106-107`（format 闸改为来源标签校验，归 #1770/#1425）；
- `rust/src/bin/production_session.rs`、`rust/src/bin/x_execution_session.rs`（main 上尚不存在，不编行号，归 #1769，同 chong-traceability.md:50）。

**blocking 边变化**

- 不新增，不删除。依据 chong-traceability.md:58、ADR 0052:364 同款先例。
- 保留 7 条：#1538 blocks #1362、#1366、#1367、#1525、#1539、#1540、#1550。
- 已解除，不改：#1533、#1536 blocks #1538（两票都已 CLOSED）。
- 不加 #1538 → #1382、#1400、#1523、#1524 的直接边。四票都已经被 #1366 阻塞（实测 #1382 blocked_by 含 1366，#1400 为 1382、1362、1363、1366、1365、1351，#1523 含 1366，#1524 为 1366、1769），#1366 又被本票阻塞，传递约束已经成立，加直接边是冗余。
- 无 research/probe 缺口，不开新票。
