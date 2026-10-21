# #1406 C8 实施报告：`tb03c-book/2`（ADR 0040 资金签署接入开局批）

分支 `codex/1406-c8-funding`，起点 `codex/1406-owner-20261020 @ 911d795fde`。作者自报，未自评通过；评审另由非作者做。

## 结论

新增书账子合同 `tb03c-book/2`。它沿用 `/1` 的全部原件形状与金额主干，并在三个位置收紧：开局批按 ADR 0040 裁定四第 4/5 步核对；非开局追加批输出 AwaitingPolicy{funding_revision}；q_rebase 输出 AwaitingPolicy{q_rebase, (S,ℓ)}。`/1` 与缺省域的分派、文案和书账形状不变。远端 `cargo check --features s_session --lib --bins --tests` 通过，`cargo test --features s_session --lib economic_session` 结果为 163 passed / 0 failed / 8 ignored；8 个 ignored 都是基线既有的 `#[ignore]`（HFS/固定载体类）。

## 提交

| 提交 | 内容 | 可编译证据 |
|---|---|---|
| `4817b0aef8` | 实现、新测试、oracle、对既有测试 1 处必要修改 | 远端 check EXIT=0 `/tmp/nc-remote/20261020-232347-4817b0aef8.log`；test 163/0/8 `/tmp/nc-remote/20261020-232517-4817b0aef8.log` |
| `0095ad3bba` | 测试收紧：负控逐条核错误文案；删除未用常量 | check EXIT=0 `/tmp/nc-remote/20261020-232834-0095ad3bba.log`；test 163/0/8 `/tmp/nc-remote/20261020-232954-0095ad3bba.log` |
| 报告提交 | 只加本报告 | 不改代码 |

注：本分支创建后，基线分支 `codex/1406-owner-20261020` 又前进了 `5e7b7d9916`（删除 `finance_tests.rs` 中的 `Fixture.applicants`）。本分支没有包含它；两者文件不重叠，合并时无冲突面。

## 改动清单（行号以 HEAD 为准）

- `rust/src/economic_session/book.rs:7-44`：新增 `TB03C_BOOK_V2` 与 `Rules::Tb03cV2`；`Rules::is_tb03c()` 表示 `/1`、`/2` 同族，`Rules::revision()` 给出 meta 绑定文本；`rules()` 显式只认缺省、`/1`、`/2` 三值，其余拒绝。
- `book.rs:113`、`:198`：decode 的 tb03c 判定改为 `is_tb03c()`（`/1` 路径不变）。
- `book.rs:386-404`：新增 `apply_in(manifest, …)`，这是 B 应用的唯一域内入口；`/2` 读 manifest 冻结的资金合同。`book.rs:412-420`：`book::apply(…, Rules::Tb03cV2)` 返回 MissingDependency，不能绕过合同。
- `rust/src/economic_session/tb03c_funding.rs`（新文件）：
  - `Contract::of` 位于 `:128-255`，负责解析和核对资金三件。缺件返回 MissingDependency；字段多余或缺失返回 SchemaUnsupported；政策族、profile、(S,ℓ) 或 q_ref 不符返回 IdentityConflict；ΣQ 超过 cap 返回 InvalidDomain。`/1` 与缺省域夹带 `funding` 时返回 SchemaUnsupported（`:130`）。
  - `check_opening` 位于 `:273-353`，负责开局批核验：(S,ℓ) 必须与冻结的重一致；恰一笔批；authorization_ref、authorization_version、currency 必须与签署一致；超过有效期返回 PolicyExpired；C(ℓ) 为 Unknown 时返回 AwaitingPolicy capacity level；Q ≤ C(ℓ)；amount = ⌈Q/L⌉。核验通过后返回写入书账的资金出处。
  - `pending_reason` 位于 `:360-388`，`is_appended_contribution` 位于 `:391`，供 E 侧等待原因使用。
- `rust/src/economic_session/tb03c_book.rs`：`open_book` 在 `/2` 下盖 `tb03c-book/2` 版本并写 `book.funding`（`:417-441`）；`/1` 书账形状不变。`apply` 保留 4 参形状，委托 `apply_with`；新增 `apply_v2`（`:665`）。`/2` 的 q_rebase 在 `:702`，追加批在 `:739`，开局核验在 `:744`。
- `rust/src/economic_session/tb03c_e.rs:93-98`：`/2` 的 Pending 原因先取 `pending_reason`；`/1` 返回 None，因此原 `capacity` 文案不变。`:202-210`：Resolve 证据不能解除 q_rebase / funding_revision 等待。`:287`：Resolve 域判定改为 `is_tb03c()`。
- `rust/src/economic_session/store.rs:121`：Config::load 接入资金合同校验。`:797`：init 绑定 `revision()`。`:994`：recover/audit 核对 `revision()`。`:1131`：审计重放走 `apply_in`。`:1495`：改为 `is_tb03c()`。
- `rust/src/economic_session/service.rs:719/728/788/844/922/1056`：`== Tb03cV1` 改为 `is_tb03c()`；`:1110` 的 ApplyAllocation 改走 `apply_in`。
- `tb03c_allocation.rs:49`、`tb03c_transfer.rs:504/520`：同族判定改为 `is_tb03c()`；源扣账证明改用源域自己的版本与 `apply_in` 重放。
- `mod.rs`：注册 `tb03c_funding` 与 `tb03c_funding_tests`。
- 新测试 `rust/src/economic_session/tb03c_funding_tests.rs`。新 oracle `s_session/tests/fixtures/tb03c_funding/funding_oracle.py` 与 `FUNDING-ORACLE.json`，与既有 `tb03c/` 夹具目录并列，未改动 `tb03c/` 下任何文件（WE5/WE7 冻结夹具未触碰）。

写集说明：除指定文件外，还改了 `service.rs`、`tb03c_allocation.rs`、`tb03c_transfer.rs`、`mod.rs`。原因是这些位置原本用 `== Rules::Tb03cV1` 判断 tb03c 族；新增第三个枚举值后，如果不改，`/2` 域会被这些判断当成旧域。这些改动都是机械替换，`/1` 的判定结果不变。

## 既有测试的唯一改动（需主控知悉）

`tb03c_book_tests.rs:399-401` 的 `book_revision_dispatch_is_explicit_and_closed` 原本把 `"tb03c-book/2"` 列为必须拒绝的闭集外样本。按任务要求定义 `/2` 后，这条断言与需求直接冲突。我把该样本改为 `"tb03c-book/3"` 并加了注释，其余断言逐字未动。这与「不得修改任何既有测试断言」的字面要求冲突，原因是需求本身要求接受 `/2`，无法同时满足。

## manifest 形状（`/2`）

```
book_revision: "tb03c-book/2"
funding:
  funding_record: {profile_id, policy_id:"#1557/funding-authorization/v1", version, supersedes, cap(=Q_total), currency,
                   authorizer, authorized_at, valid_through[, scope]}          # 字段恰为 ADR 0040 funding-record
  margin_leverage: {value, source}
chongs[i].funding:
  Q: 本重总投入额
  policy_record: {profile_id, policy_id:"#1355/quantity-tier-policy/v1", version, supersedes[, scope], symbol,
                  operation_level:"L<n>", q_ref, tiers, capacities[{level, C|"Unknown", source, valid_through}],
                  frequency_evidence_ref, authorizer, authorized_at, valid_through}
```

开局批分量在 `/1` 字段之外还要带 `authorization_version` 与 `known_at`（`YYYY-MM-DD`）。代码里只有政策族标识与字段名，没有签署数值常量；签署值只作为测试夹具中的黄金值出现。

## ADR 条款与代码的对应

| 条款 | 代码 |
|---|---|
| ADR 0040 裁定四第 4 步：开局批 = ⌈Q/L⌉，authorization_ref 指 funding-record 版本 | `tb03c_funding.rs:273-353` |
| ADR 0040 裁定四第 5 步：q_ref 指 funding-record；开重复核 Q ≤ C(ℓ) | `:216-222`（q_ref）、`:327-338`（Q ≤ C） |
| ADR 0034 裁定四：C(ℓ) Unknown ⟹ AwaitingPolicy{capacity}，未知不是无限 | `:327-332` |
| ADR 0034 裁定一：量档记录字段 | `POLICY_FIELDS`、`CAPACITY_FIELDS` |
| ADR 0040 签署记录：funding-record 八字段 + scope | `FUNDING_FIELDS` |
| ADR 0040 通则 Q5：只从当前生效版本读，不写常数 | 数值全部来自 manifest |
| ADR 0040 裁定四末段：Q 变大不自动改现役 Q ⟹ AwaitingPolicy{q_rebase,(S,ℓ)} | `tb03c_book.rs:702`、`tb03c_funding.rs:360-388`、`tb03c_e.rs:93-98` |
| ADR 0040 裁定四末段：改总投入须签新版本 ⟹ 追加批等待 | `tb03c_book.rs:739`、`pending_reason` 的 funding_revision |
| ADR 0031 裁定一：本金批字段（含 known_at、authorization_ref） | 开局批分量的 `known_at` / `authorization_ref` |
| 根裁定 OQ-7：新版本只对新域生效、无回落 | `book.rs:35-44`；`Contract::of` 拒绝 `/1` 与缺省域夹带资金字段 |

追加批的依据：ADR 0040 裁定四末段写明「改总投入、`l_start`、容量、频率门或杠杆都签新版本……只作用于生效后」；ADR 全文没有授权追加投入自动生效。因此在 `/2` 下，非开局的 principal_contribution 返回 AwaitingPolicy{funding_revision}。E Pending 与 B 拒绝采用同一口径，Resolve 证据也不能解除。

## 测试（远端实际执行，日志 `/tmp/nc-remote/20261020-232954-0095ad3bba.log`）

`economic_session::tb03c_funding_tests::` 下的 8 个新测试全部 ok：

- `rules_resolve_exactly_three_values`：只认三值；`/3`、`/0` 和带空格的值都拒绝；`book::apply(…Tb03cV2)` 不能绕过合同。
- `oracle_matches_signed_golden_values`：FUNDING-ORACLE.json 与签署黄金值逐项相等，包括 L1 档量 1,110,400。
- `signed_profiles_open_every_derived_chong`：按 oracle 遍历三个 profile 的每个重并开局，即 P0 ⟨BTC,L0,100000⟩ 批 100000、P1-hl-testnet ⟨BTC,L0,500⟩ 批 500、P0-acceptance-5m 的 L0 1,110,000、L1 3,200,000、L2 690,000。同输入调用两次输出相同，并逐字核对 `book.funding` 出处。
- `opening_batch_negative_controls_are_typed_and_stateless`：金额少 1、金额多 1、错 authorization_ref、错 version、错 currency、过期（2027-10-21）、known_at 形状错误，均核对错误类型与文案；同输入重复调用得到同一错误；有效期当日可以开局。
- `capacity_gate_q_over_c_and_unknown`：Q = C+1 返回 InvalidDomain；Q = C 可以开局；L3 Unknown 返回 `AwaitingPolicy：capacity level=L3`。
- `missing_funding_pieces_are_typed_rejections`：缺 funding-record、缺 margin_leverage、缺整个 funding、缺 ⟨Q, policy-record⟩、q_ref 错误、policy 级别错误、funding 政策族错误、funding-record 缺字段、杠杆为 0、ΣQ 超过 cap、`/1` 或缺省域夹带资金合同。每项都同时经 `Contract::of` 和 `store::Config::load` 验证，两者错误一致。
- `appended_batch_and_q_rebase_await_new_signature_in_b`：B 侧追加批和 q_rebase 的新文案，以及 `/1` 原文案 `AwaitingPolicy：capacity；C(ℓ) 未签署，fixed_q 不变` 不变。
- `v2_domain_binds_revision_waits_in_e_and_replays`：完整经过 `service::handle` 的 E Capture 与 B Apply。开局 Applied；追加批与 q_rebase 在 E Pending 的原因为 `{kind:AwaitingPolicy, slot:funding_revision|q_rebase, symbol:BTC, level:L0}`；B 拒绝时图像不变；meta 为 `tb03c-book/2`；E/B audit 通过；recover 后图像逐字一致。

`/1` 与 Legacy 回归：除上文那一处必要改动外，既有 economic_session 测试全部未改并通过（163 passed，包括 `tb03c_book_tests`、`tb03c_e_tests`、`tb03c_c5_*`、`tb03c_allocation_tests` 和 legacy 重放锁）。全部测试通过，因此不需要对照 `origin/main` 判断失败来源。

独立 oracle 在本机执行 `python3 s_session/tests/fixtures/tb03c_funding/funding_oracle.py --check`，输出 ok。它只用标准库与整数，按裁定四第 1–4 步推导，并与 ADR 0040 的 derivation-result 一致。

## 未做项与依据

1. 转移开出的目标重（`transfer_opening`）不受开局批核验。依据是 ADR 0034 裁定六：目标新重的 P0 由 `principal_contribution{source: transfer}` 形成，不来自本 profile 的 funding-record。换档激活时的 Q ≤ C 复核属于 #1445/#1446（ADR 0040「受影响票」）。
2. E 侧不为「开局时 C(ℓ) = Unknown」另设 Pending 等待，只由 B 拒绝并且不建收据，这满足任务要求。若希望 E 也显式挂等待，需要另行裁定。
3. 没有校验 `known_at < authorized_at`（签署生效前的原件），因为 ADR 只规定了到期，没有规定生效起点的拒绝口径。
4. 频率门与 `l_start` 推导（裁定四第 1–3 步）不在书账内执行，只在 oracle 中推导并与 manifest 冻结的重清单对拍；推导实现属于 #1444。
5. `book.funding` 是新增的书账字段，只出现在 `/2` 书账中。读视图或下游 reader 是否需要它，本片没有处理。
6. 空头重的开局批同样按 ⌈Q/L⌉ 核对。依据是 ADR 0040 裁定四第 4 步「声部为空时就是投入保证金 M」；三阶段在换向后是延续还是重置，属于 ADR 0040「未裁项」，本片不涉及。

## 需要主控裁定的点

- 既有测试 `book_revision_dispatch_is_explicit_and_closed` 的样本从 `/2` 改为 `/3`，与「不改既有断言」的字面要求冲突，需要确认接受。
- 写集超出列名文件：`service.rs`、`tb03c_allocation.rs`、`tb03c_transfer.rs`、`mod.rs` 有机械替换。
- `/1` 与缺省域的 manifest 如果夹带 `funding` 字段，现在会被拒绝。这是对 `/1` 行为的收紧，现有夹具中没有这种 manifest；也可以改为静默忽略，但那等于回落。
- 开局批新增两个分量字段：`authorization_version` 与 `known_at`（日期文本）。其中 version 单独成字段，是因为任务把 authorization_ref 定成 `<policy_id>@<profile_id>`，引用本身不含版本。
- 上面「未做项」第 1、2 条的边界。

---

# 返修（C8 独审 REQUEST_CHANGES，`/Volumes/AgentStorage/issue1406-owner-20261020-review/c8-review.md`）

返修前先并入基线新提交：`f4961d8ae7` 是对 `origin/codex/1406-owner-20261020` 的 merge，带入 `5e7b7d9916` 和 `6c8477ff6f`。返修提交为 `52bfb7a343`，提交时按要求加了 `-c core.hooksPath=/dev/null`。

## 各提交的远端证据

| 提交 | 内容 | 远端证据 |
|---|---|---|
| `f4961d8ae7` | 并入基线 | check EXIT=0，`/tmp/nc-remote/20261020-235239-f4961d8ae7.log`（用临时分支名送 nc-remote，跑完已删分支） |
| `52bfb7a343` | 返修 | check EXIT=0，`/tmp/nc-remote/20261020-234751-52bfb7a343.log`；`cargo test --features s_session --lib economic_session` 结果 **167 passed / 0 failed / 8 ignored**，`/tmp/nc-remote/20261020-234913-52bfb7a343.log`。8 个 ignored 与基线相同 |
| 报告提交 | 只改报告 | 不改代码 |

12 个 `tb03c_funding_tests::*` 测试全部 ok。

## 逐条回应

**H1（已修）**：在 `tb03c_funding.rs` 的 `check_opening` 中，funding-record 与 policy-record 都加了下界。`known_at < authorized_at` 时返回 `PolicyNotYetEffective：原件 known_at … 早于 <record> 签署生效日`，依据是 ADR 0040:56 与 :59 的「只作用于生效后」。capacity 条目本身没有 `authorized_at`，它归 policy-record 的签署管辖。该检查放在过期检查之前。新增负控：`2026-10-19` 被拒，`2026-10-20` 当日可以开局；policy-record 单独晚签（`2026-11-01`）时，`2026-10-21` 被拒、`2026-11-01` 可以开局。

**M1（不绑定，两个量量纲不同，请另开票）**：`fixed_q` 是**数量单位**，`funding.Q` 是**金额（USDC 投入额）**。证据如下：

- `rust/src/economic_session/finance.rs:850-858`：`worst = actual_quantity + plan.quantity`，再与 `fixed_q` 比较（「最坏量超过固定 Q」）。这是数量和数量的比较；金额另有 `locked_amount` 与 `r.amount` 的比较（`:847`）。
- `finance.rs:859-863`：数量单位（`units.quantity.unit`）与货币（`units.money.currency`）分别核对，说明账本把两者分成两个维度。
- `tb03c_book.rs`（`split`，约 :449-459）：拆股时 `fixed_q` 与 `actual_quantity` 用同一个整数比 num/den 重基。只有股数才会这样重基，金额不会。
- `book.rs:230` 与 `store.rs:141` 都把它作为 `book_identity` 和路由上的正整数，`service.rs:737` 要求原件与路由相等，口径是持量上界。
- 与此相对，`funding.Q` 按 ADR 0040 签署记录是「本重分得的总投入额」，开局批 `amount = ⌈Q/L⌉` 记入金额 `principal.batches`（ADR 0031:12，币种 USDC）。

**登记的张力**：ADR 0034:23 写 `tier_quantity = ⌊Q·num/den⌋（quantum 单位）`，ADR 0034:29 有「Q + units_g」与「q_ref + cash_g」两种口径；ADR 0040 又从**金额** Q=3,200,000 推出档量 1,110,400。所以「Q（固定，动作基）」到底是数量还是金额，在正本层两处读法不一致。现码 q_rebase 改写的是数量 `fixed_q`。本片不绑定两者，夹具保持 `fixed_q:"1"` 与 `Q:100000`，等主控另开票裁定。

**M2（已修）**：oracle 新增 `leverage_cases`，两组都是不整除样本：`{Q:100001, L:2 ⟹ 50001}` 和 `{Q:100000, L:3 ⟹ 33334}`。Rust 测试 `opening_batch_rounds_up_when_leverage_does_not_divide` 从 oracle 读这两组：向上取整的值通过，向下取整的值和 +1 的值都按完整文案被拒。把公式改成 floor 后，这条测试必然失败。

**M3（已修）**：`tb03c_funding::pending_reason` 在 `/2` 下对开局原件判断：如果重的 C(ℓ) 缺条目或为 Unknown，E Pending 记为 `{kind:AwaitingPolicy, slot:capacity, symbol, level}`。capacity 槽原本就被 Resolve 守卫挡住（`tb03c_e.rs` 的 `capacity` 分支），B 侧的 `check_wait` 也按 capacity 拒绝。测试 `unknown_capacity_opening_waits_in_e_and_resolve_cannot_clear_it` 走真实 `service::handle`，覆盖以下几点：E 的 Pending 原因；Resolve 返回 `AwaitingPolicy：capacity；Resolve 证据不能替代用户签署的容量`，且 E 图像不变；B 返回 `AwaitingPolicy：capacity；B 不应用、不建收据，fixed_q 不变`，B 图像不变、没有收据；已签容量的开局不挂这个等待。`/1` 的 Pending 文案不变（`pending_reason` 对 `/1` 返回 None）。

**M4（已修；换档门不在本片）**：`open_book` 的版本改为取自域规则，不再看是否带资金出处。测试 `v2_transfer_opened_chong_carries_v2_revision` 用 c5 的真实转移目标原件，在 `/2` 合同下开出的书账为 `tb03c-book/2`，且除版本字段外与 `/1` 结果逐字相同（也不带 `funding`）。**ADR 0034:23 要求的「换档激活时 Q ≤ C(ℓ)」在 `/2` 目前没有门，归 #1445/#1446 接**（ADR 0040「受影响票」）。转移目标不核开局批，依据是 ADR 0034 裁定六。

**LOW**

- L1（已修）：去掉了恒真的 `assert_eq!(image, empty_image())`。纯函数负控改为「同输入同输出（含错误）」；「失败不改持久状态」由两条服务链测试读 B/E 实际图像核对。所有负控（开局 12 条、缺件 12 条、Q>C、Unknown）都改为 `assert_eq!` 核对**完整**错误文本。
- L2（已修）：新增开局零批、两批、S 不符、ℓ 不符的用例；新增 `each_signed_record_window_gates_alone`，分别让 policy-record 单独过期、capacity 单独过期、policy-record 单独未生效，并覆盖边界当日。
- L3（记录在案）：`/1` 与缺省域夹带 `funding` 时拒绝。评审赞成，不改。
- L4（已改文案）：`tb03c_allocation.rs:50` 改为「净额分配须 tb03c-book/1 或 /2」，只在非 tb03c 域出现，没有测试断言这条文案。`s_session/economic_query.py` 的 docstring 改为说明 Python 读者尚未接 `/2`、遇到会拒绝。Python 读者接 `/2` 建议另开后续票，我没有开票（按指令不在 GitHub 动作）。
- L5（登记）：`authorization_ref = <policy_id>@<profile_id>` 加上 `authorization_version`，合起来等价于 ADR 0040:56 的「指 funding-record 的版本」。这个形状选择需要在定义正本或票里登记，由主控决定落在哪里。
- L6（不改）：`margin_leverage` 只冻结 `{value, source}`，接入 derivation-input 记录属于 #1444。
- L7（已加注释）：`check_opening` 中注明 `known_at` 是 TestOnly 原件自报的日期，接入真实来源时要改为绑定可审计的观察时间。注释引的是本票评审项，没有新票号；需要主控开票后补票号。

## 返修后仍未做

- 换档激活时的 Q ≤ C 复核（#1445/#1446）。
- `fixed_q` 与 `funding.Q` 的口径裁定（M1，等主控开票）。
- Python 读者接 `/2`（L4）。
