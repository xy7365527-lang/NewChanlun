APPROVE（复审 e701a925f7；首审 97ad97d297 为 REQUEST_CHANGES，见下文）

# #1406 C8 独立评审：`tb03c-book/2`（codex/1406-c8-funding @ 97ad97d297，基线 911d795fde）

评审人：/root/owner_1406/review_c8（非作者，只读）。证据命令：`git diff 911d795fde 97ad97d297`；远端实跑 `NC_TARGET=target-1406-c8review nc-remote codex/1406-c8-funding rust cargo test --features s_session --lib economic_session` ⟹ `163 passed; 0 failed; 8 ignored`，sha=97ad97d297193752…，8 个 `tb03c_funding_tests::*` 与 `tb03c_book_tests::book_revision_dispatch_is_explicit_and_closed` 均出现 `ok`（日志 `/tmp/nc-remote/20261020-233914-97ad97d297.log:886,921-928`）。Oracle 本机 `python3 s_session/tests/fixtures/tb03c_funding/funding_oracle.py --check` ⟹ `ok`。

## 结论

`/1` 与 Legacy 行为保持住了，`/2` 也没有回落到宽松路径，主干实现方向正确。需要改的是开局批的时间门：它只看批次自报的 `known_at` 是否晚于有效期，不看是否早于签署生效日，导致批次可以引用尚未存在的授权。另有四条 MEDIUM：Q 与 manifest 既有 `fixed_q` 两个来源不绑定；⌈Q/L⌉ 的向上取整没有任何测试能区分；C(ℓ)=Unknown 只在 B 拒、E 侧没有 typed 等待，和 OQ-3 不一致；`/2` 域里转移开出的重书账盖的是 `tb03c-book/1`。

## HIGH

**H1. `known_at < authorized_at` 不拒绝（作者未做项 3）——同意主控认定为缺陷。**
`tb03c_funding.rs:308-326` 只做 `known_at > valid_through ⟹ PolicyExpired`，没有下界。依据：ADR 0040:56 写「`authorization_ref` 指该 profile funding-record 的版本」，ADR 0040:59 写新签署版本「只作用于生效后」；ADR 0031:12 写「新增批次只能由显式授权事件 `principal_contribution` 形成」。批次在 `authorized_at` 之前就已知，等于引用一个当时不存在的授权，同样的道理对 policy-record 的 `authorized_at` 也成立（容量签署也是生效后才能拿来做 Q ≤ C）。作者说「ADR 只规定了到期，没有规定生效起点的拒绝口径」——ADR 没给错误名，但「只作用于生效后」本身就是下界，不需要另裁；最多需要裁的是错误类型名（建议沿用 `PolicyExpired` 族或新增 `PolicyNotYetEffective`，由主控定）。
要求：对 funding-record、policy-record 两者加 `known_at < authorized_at ⟹ 拒绝`，并补负控（`known_at = 2026-10-19` 拒、`2026-10-20` 当日可开）。

## MEDIUM

**M1. `funding.Q` 与 manifest 既有 `fixed_q` 是两个互不绑定的 Q。**
`/2` 每重在 `chongs[i].funding.Q`（`tb03c_funding.rs:187`）冻结 Q，但同一重的 `chongs[i].fixed_q`（`store.rs:141`，`service.rs:737` 要求原件 `book_identity.fixed_q` 与之相等）仍独立存在；书账 `fixed_q` 取自原件（`tb03c_book.rs:425`），`book.funding.Q` 取自合同。测试夹具本身就是反例：`fixed_q:"1"` 而 `Q:100000`（`tb03c_funding_tests.rs:64,156`）。ADR 0034 裁定五「Q（固定，动作基）……只能由 `q_rebase` 政策版本改写」与现码 q_rebase 改写的 `fixed_q`（`tb03c_book.rs:449-459`）是同一个量；下游 `finance.rs:857` 用 `fixed_q` 做最坏数量界。一本书里两个 Q 违背「同一判断不得两档」的精神。要么 `/2` 下 `Contract::of` 核 `fixed_q == Q`（若二者量纲不同则须先裁清量纲：ADR 0034:29 有「Q + units_g」与「资金口径 q_ref + cash_g」两口径），要么在报告里显式登记为待裁并开票。

**M2. ⌈Q/L⌉ 的向上取整没有被任何测试区分。**
全部夹具 `margin_leverage.value = "1"`（`tb03c_funding_tests.rs:80`），oracle 三个 profile 也都是 `margin_leverage: 1`（`funding_oracle.py:28-30`）。L=1 时 ceil、floor、round 结果相同，把 `tb03c_funding.rs:339-342` 改成 `c.q / self.leverage` 全部测试仍会通过。ADR 0040:52 明写「P0 保证金向上取整，其余向下取整」，这是本片核心公式。要求：补一个 L>1 且不整除的用例（如 Q=100001、L=2 ⟹ 50001，金额 50000 必须拒），oracle 也加一组非 1 杠杆的推导。

**M3. C(ℓ)=Unknown（以及 PolicyExpired）只在 B 拒，E 侧记成 `Unresolved`，与根裁定 OQ-3 不一致（作者未做项 2）。**
#1406 根裁定 OQ-3（issue 评论第 3 条）：「E 侧 AwaitingDependency／AwaitingPolicy 挡 B 应用（typed 等待，不建收据）；普通风险 Pending 不挡 B」。按此口径 AwaitingPolicy 是 E 侧的 typed 状态，B 侧只是兑现。现码 `tb03c_e.rs:93-109` 的 `pending_reason` 只覆盖 q_rebase 与 funding_revision，开局 C(ℓ)=Unknown 落到 `Unresolved`；`resolve_image`（`tb03c_e.rs:199-210`）不挡 `Unresolved`，Resolve 证据可以把它清掉，之后 B 仍每次以 `AwaitingPolicy：capacity level=…` 拒（`tb03c_funding.rs:327-332`）。资金不会被放行，但 E 与 B 对同一原件给出不同的等待类型，E 侧还允许以证据「解决」一个只有新签署才能解决的缺口——这正是 `tb03c_e.rs:199` 对 `capacity` 槽已经禁止的事。E 侧拿得到 manifest，`pending_reason` 可以对开局原件同样算出 `{kind:AwaitingPolicy, slot:capacity, symbol, level}`。作者的「满足任务要求」只是 B 不建收据这一半。

**M4. `/2` 域内转移开出的重，书账盖 `tb03c-book/1`。**
`open_book` 以 `funding.is_some()` 选版本（`tb03c_book.rs:417-421`），而转移目标走 `funded = None`（`tb03c_book.rs:733-746` 的 `_ => None`），于是 meta 为 `tb03c-book/2` 的域里出现 `book.book_revision = "tb03c-book/1"` 的书账。Python 查询侧 `economic_query.py:67-72` 认 `/1` 而拒 `/2`，结果同一 `/2` 域里转移重可读、开局重不可读。书账版本应取自域规则（`apply_with` 已知道是 V2），与是否带资金出处无关。附带：(a) 转移目标不核开局批的依据成立——ADR 0034:30 裁定六明写目标 P0 由 `principal_contribution{source: transfer}` 形成、Q_new 激活时按目标级量档冻结，ADR 0040「受影响票」把换档激活复核归 #1445/#1446；但这意味着 ADR 0034:23「Q ≤ C(ℓ) 在开重与换档激活时校验」的激活半边在 `/2` 尚无门，报告应明写为待 #1445/#1446 接，而非仅引裁定六。

## LOW

- **L1 负控文案未逐条钉住。** `missing_funding_pieces_are_typed_rejections` 11 例只核前缀（`tb03c_funding_tests.rs:459-462`，同前缀 `IdentityConflict` 有三例、`InvalidDomain` 两例）；`capacity_gate_q_over_c_and_unknown` 的 Q>C 例只核 `InvalidDomain`（`:388-390`）。另外 `assert_eq!(image, book::empty_image())`（`:366`）比较的是本地新建空图像，恒真，不证明「失败不改状态」（服务链测试 `:521-524` 倒是真核了 B 状态不变）。
- **L2 未覆盖的拒绝分支**：开局零批（`tb03c_funding.rs:288-291`）、多批（`:292`）、开局 (S,ℓ) 与合同不符（`:276-278`）、policy-record / capacity 单独过期（三者 valid_through 夹具相同，只有 funding-record 那条被命中）。
- **L3 `/1` 与 Legacy 夹带 `funding` 即拒（作者边界 d）。** `Config::load` 以前不做顶层键闭集，`/1` manifest 带 `funding` 原本被静默忽略，现在 `SchemaUnsupported`（`tb03c_funding.rs:129-136`）。按 #799/OQ-7 我赞成拒绝而非忽略；这是对 `/1` 配置面的收紧，现有夹具无此形状，记录在案即可。
- **L4 陈旧文案/文档**：`tb03c_allocation.rs:50` 仍报「净额分配须 tb03c-book/1」但现已接受 `/2`；`economic_query.py:68` docstring「与 Rust book::rules 同义」已不成立（Rust 认 `/2`，Python 拒）。后者是作者已列的未做项 5，建议开后续票而非只留在报告里。
- **L5 `authorization_ref` 形状与 ADR 字面差一层。** ADR 0040:56 与 derivation-result `authorization_ref: v1` 把它定为「版本」；本片按任务说明取 `<policy_id>@<profile_id>` 并另加 `authorization_version`。两字段合起来等价且都被核（`tb03c_funding.rs:294-304`），无功能缺陷，但宜在定义正本或票里登记这一形状选择。
- **L6 `margin_leverage.source` 未与 derivation-input 记录绑定。** ADR 0040 把 `margin_leverage` 放在 `#1558/chong-derivation-input/v1` 记录里；manifest 只冻结 `{value, source}` 自由文本（`tb03c_funding.rs:168-170`）。本片可接受，推导记录接入属 #1444。
- **L7 `known_at` 为原件自报。** 过期门完全取决于原件里写的日期，与 E 观察时间（`first_known_ns`）无关；TestOnly 范围内可接受，但真实来源接入时须改为绑定可审计时间，建议在 H1 修复时一并在注释中带票号说明。

## 逐项核对结果

1. **`/1`、Legacy 不变**：已改既有测试只有 `tb03c_book_tests.rs:399-401` 一处（`/2`→`/3` 加注释），其余断言逐字未动（diffstat 只有该文件 3 行变动）。`rg 'Tb03cV1|book_rules\(\)'` 全仓核过：`book.rs:113/198`、`service.rs:719/728/788/844/922/1056`、`store.rs:797/994/1495`、`tb03c_e.rs:287`、`tb03c_allocation.rs:49` 的 `is_tb03c()` 替换对 `/1` 判定等价；`tb03c_e.rs:80` 的 `== Legacy`、`voice.rs:160`、`service.rs:711/927`、`store.rs:1109` 直接传规则值，`/2` 走 tb03c 分支，无漏改。B 应用的三个调用点（`service.rs:1110`、`store.rs:1131`、`tb03c_transfer.rs:520`）全部改走 `apply_in`；`book::apply(…, Tb03cV2)` 返回 MissingDependency（`book.rs:414-418`）。转移源证明改用源域自己的规则（`tb03c_transfer.rs:504/520`），比原先写死 V1 更正确。
2. **#799**：`/2` 严格校验失败（`check_opening`、`Contract::of`、追加批、q_rebase）一律返回 Err，没有任何分支退到 `/1` 的 `apply` 或宽松处理；`apply_in` 对 V2 只走 `apply_v2`。未发现违规。
3. **金额/引用对 ADR 0040**：公式 `(Q+L-1)/L`（`tb03c_funding.rs:339-342`）在 Q、L 均为正整数下即 ⌈Q/L⌉，L 来自 manifest `margin_leverage.value`，无代码常量（只有政策族标识与字段名，符合 Q5）。三 profile 黄金值与 ADR 0040 derivation-result 一致：P0 100000；P1 500；P0-acceptance-5m L0 1,110,000 / L1 3,200,000 / L2 690,000，L1 档量 ⌊3200000·347/1000⌋ = 1,110,400。Oracle 只用标准库，从 Q_total、C(ℓ)、F_min、L 逐步推导，不 import 产品代码、不抄 Rust 黄金值。缺口见 M2。
4. **边界选择**：(a) 依据成立，但激活半边的 Q ≤ C 门须明写待接（M4 附带）；(b) 与 OQ-3 不一致（M3）；(c) 缺陷（H1）；(d) 赞成（L3）。
5. **负控与持久层**：开局批 7 条负控逐条核了类型与文案片段，且同输入两次同输出；服务链测试经 `service::handle` 真实写 E/B SQLite，失败时核 B `latest` 不变，`store::audit` 两域重放、`store::recover` 后图像逐字相等，meta 为 `tb03c-book/2`——这部分是真走持久层的。不足见 L1/L2。
6. **实跑**：163/0/8，与作者报告一致，新测试名全部执行。

---

# 复审（codex/1406-c8-funding @ e701a925f7，对比 97ad97d297）

**结论：APPROVE。** H1、M2、M3、M4 已修，并有测试锁住；M1 经独立核对，量纲证据成立，按主控裁定不绑定、另开票。没有残余 HIGH/MEDIUM。剩下的是需要主控开票或登记的跟进项。

**证据**：
- 远端测试：`NC_TARGET=target-1406-c8review nc-remote codex/1406-c8-funding rust cargo test --features s_session --lib economic_session`，sha=e701a925f7dc…，结果 `167 passed; 0 failed; 8 ignored`。12 个 `tb03c_funding_tests::*` 全部 ok。日志：`/tmp/nc-remote/20261021-000157-e701a925f7.log`。
- oracle：`funding_oracle.py --check` 输出 ok。
- 并入基线：`git merge-tree --write-tree 97ad97d297 6c8477ff6f` 得到 `54a0eb051c…`，与 `f4961d8ae7^{tree}` 相同，说明 merge 是自动合并结果，没有手工改动。

## 逐条

- **H1 已关闭。** `tb03c_funding.rs` 的 `check_opening` 先查下界：funding-record 和 policy-record 的 `authorized_at` 都参与判断，`known_at < authorized_at` 返回 `PolicyNotYetEffective：原件 known_at … 早于 <record> 签署生效日`；之后再查过期。错误名采用主控裁定的名称。负控按完整文案断言了以下情形：`2026-10-19` 被拒，`2026-10-20` 当日可以开局；policy-record 单独晚签时，`2026-10-21` 被拒、`2026-11-01` 可以开局（`each_signed_record_window_gates_alone`）。capacity 条目没有 `authorized_at`，由 policy-record 的签署管辖，这样处理合理。
- **M1：不绑定，证据成立。** 我独立核了以下几点：
  - `finance.rs:850-858` 用 `actual_quantity + plan.quantity (+ others)` 与 `fixed_q` 比较，报错文案是「最坏量超过固定 Q」。这是数量对数量的比较；金额由 `:847` 的 `locked + bound ≤ r.amount` 另行管。
  - `:859-863` 分别核 `units.quantity.unit` 和 `units.money.*`，说明账本里数量和金额是两个维度。
  - `tb03c_book.rs` 的 `split`（约 :449-459）拆股时，按同一整数比重基 `actual_quantity` 和 `fixed_q`；只有股数会这样重基。
  - `funding.Q` 按 ADR 0040:34/:52-56 是 USDC 投入额，开局批 ⌈Q/L⌉ 记入金额本金批。
  因此两者量纲确实不同，首审 M1 关于「同一个量的两个来源」的判断不成立，我撤回这一定性。作者登记的正本张力也属实：ADR 0034:23/:29 的「Q（固定，动作基）」可以读成数量或金额，ADR 0040 从金额 Q 推出档量 1,110,400，这正是另开票要裁的内容。
- **M2 已关闭。** oracle 新增 `leverage_cases`：`{100001, L2 ⟹ 50001}` 和 `{100000, L3 ⟹ 33334}`，由 `ceil_div` 独立推导，并同时给出 `floor_batch`。`opening_batch_rounds_up_when_leverage_does_not_divide` 断言 ceil 被接受、floor 和 ceil+1 按完整文案被拒；如果实现改成向下取整，这条测试会失败。
- **M3 已关闭。** `pending_reason` 对 `/2` 开局原件处理如下：C(ℓ) 缺条目或为 Unknown 时，E 记 `{AwaitingPolicy, capacity, symbol, level}`。它与 B 的判断共用 `Chong::signed_capacity`，两侧口径一致。`unknown_capacity_opening_waits_in_e_and_resolve_cannot_clear_it` 通过真实 `service::handle` 核了：E Pending 原因；Resolve 被既有 capacity 守卫拒绝，E 图像不变；B 返回 `AwaitingPolicy：capacity；B 不应用、不建收据…`，B 图像不变，`receipts` 为空；已签容量的开局返回 None。`/1` 的 `pending_reason` 仍返回 None。PolicyExpired 和 PolicyNotYetEffective 仍只在 B 拒绝，这两者是签署窗口外的终态拒绝，不是可等待的缺口，我接受这样处理。
- **M4 已关闭。** `open_book` 的版本改为取自域规则（`v2: bool`）。`v2_transfer_opened_chong_carries_v2_revision` 用 C5 真实转移目标原件验证：`/2` 结果盖 `tb03c-book/2`、不带 `funding`，除版本外与 `/1` 结果逐字相同。报告已写明：换档激活时的 Q ≤ C 在 `/2` 下还没有门，由 #1445/#1446 接手。
- **L1 已关闭。** 恒真的断言已删除。开局 12 条、缺件 12 条、Q>C、Unknown 全部改用 `assert_eq!` 比较完整错误文本；「失败不改持久状态」交给两条服务链测试核实际图像。
- **L2 已关闭。** 补齐了零批、两批、S 不符、ℓ 不符，以及三种签署窗口各自单独过期或未生效的用例，并覆盖了边界当日。
- **L3、L5、L6 维持现状。** 均为登记或不改项。
- **L4 已改文案。** Python 读者接 `/2` 待另开票。
- **L7 已加注释，但注释缺票号。** 按 #793 的纪律（注释必须带票号），主控开票后需要补上。

## 待主控处理（不阻塞本片）

1. 另开票裁定 `fixed_q`（数量）与 `funding.Q`（金额）在 ADR 0034「Q」上的口径（M1）。
2. #1445/#1446 接手 `/2` 下换档激活时的 Q ≤ C 门。
3. 另开票让 Python 读者接 `/2`（L4）。
4. 在定义正本或票里登记 `authorization_ref`+`authorization_version` 的形状（L5）。
5. 开票后在 L7 注释中补上票号。
