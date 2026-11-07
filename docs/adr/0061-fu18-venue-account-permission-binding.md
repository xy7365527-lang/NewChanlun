# ADR 0061：受控场所、账户、权限——P1 凭据作用域版本、profile 绑定、发送门与拒发终态

> 正本：本文件是 `P1-hl-testnet` 上凭据作用域版本、场所 profile 绑定、X 发送门与拒发终态的正本。
> 最后裁定：#1540（2026-11-07，主控裁定，[裁定评论 6702983888](https://github.com/Newchanlun/NewChanlun/issues/1540#issuecomment-6702983888)）
> 受影响代码清单：见「受影响代码清单」节（实施归 #1365 系列，等 #1942 合入后承接）

**日期**：2026-11-07
**裁定人**：#1323 主控（Opus）代理裁定。用户 2026-11-07 明确取消授权门（「我现在也不需要什么授权了」）。
**状态**：已采纳（T 类）。代理起草 → 非作者独审（Fable，VERDICT: PASS，四条必改）→ 主控裁定。起草稿、独审与场所文档原件：`/Volumes/AgentStorage/issue1540-fu18-draft/{draft.md,review.md,venue-docs/}`。本 ADR 只定形状、顺序、词汇与证据载体；代码实施与运行证据归下游票，不声称实现完成。
**行号基线**：仓内文件引文按 main `0b3e7f3cf5`（2026-11-07）。SPEC 行号按 `gh issue view 1340 --json body --jq .body` 输出；INTERFACE-CONTRACTS 指 `.chanlun/review-results/issue1339-r2-spec-20260909/view/spec/INTERFACE-CONTRACTS.md`。

## 问题

#1540（FU-18）票面三条验收：能力有 success / Unsupported / fault 证据；凭据作用域固定；未授权不发送。范围只到 [ADR 0028](0028-first-capability-verification-profile.md) 的 `P1-hl-testnet` v1，P2 与真钱首发归 #1561。

要挡的风险：今天 X 只看凭据环境变量是否非空（`rust/src/bin/x_execution_session.rs:180-188`）。用户如果把主钱包私钥放进凭据文件，X 照样认为有凭据。testnet 与 mainnet 共用地址和私钥（ADR 0028:36），这等于事实上授予了主网划转权限。本 ADR 让这种情形在任何写请求之前被拦下，并把「谁授权、授权了哪个版本、凭什么相信这把钥匙不能划转」分别落成可核记录。

起草时发现的两处张力，由本 ADR 裁掉：

1. **同名异物。** ADR 0028:22 的 `credential_scope` 指作用域值加来源类别；代码里的 `credential_scope` 是 `{env, file}` 管道配置（`x_execution_session.rs:58`、`production_session.rs:185-187`）。不分开的话，挪一次凭据文件路径也会算成作用域变更。
2. **P1 是否跑机器层。** ADR 0028:46 字面只说 `production`，ADR 0039:287 与代码 `x_execution_session.rs:109` 照此只在 production 拒发，ADR 0045:24 也据此读 SR-3 在 P1 恒不触发。但 ADR 0028:71 定 TB-09 以 P1 验收，AT-09（#1340:421）要求「被真实部署阻止」。P1 上不核，TB-09 在 P1 上只能证明「production 分支有这段代码」。

## 裁定

### 一、凭据作用域版本（Q1 = A，吸收必改 1、3）

- 场所 profile 记录的 `credential_scope` 对象写三项：`class`（`trade_no_transfer`）、`source`（来源类别，不是路径）、`public_id`（该 API wallet 的公开地址）。换钥匙必改 `public_id`，按 ADR 0028:28 即新 profile 版本。
- **凭据作用域版本 = `(profile_id@version, venue_profile_sha256)`**，`venue_profile_sha256` 是整份场所 profile 记录的 sha256。不另算子对象哈希：ADR 0028:46 对「profile 变更」与「作用域变更」的处置相同（都重核），子哈希不带来新判定。
- 凭据值不记录、不落库、不派生任何持久标识。X 为第 4 步核验在内存中由私钥推出公开地址，推导结果是公开信息；X 本就持有凭据（ADR 0039:285），这一步不扩大凭据的可见面。
- 排除：人工递增 `scope_revision`（靠人记得递增，且与「任一字段变更即新版本」形成两个版本号）；凭据值哈希作身份（由凭据值派生持久标识）。

### 二、profile 绑定的载体与冲突（Q2 = A，吸收必改 3）

- ADR 0028 十个字段落成一份具名版本化**场所 profile 记录**（`venue-profile/1`，形状同 ADR 0057:43-51 的版本化记录）。它是政策记录，不含任何凭据值。
- X 执行 profile 升到 `x-execution-profile/2`：按 `{path, profile_id, version, sha256}` 引用场所 profile 记录，按 `{path, version, sha256}` 引用 #1770 能力记录；原 `credential_scope.{env,file}` 改名 **`credential_source.{env,file}`**，只管部署管道。挪凭据文件路径不改场所 profile 记录，因而不改作用域版本。
- X 库**绑定行按 `(profile_id, version)` 追加**，不做单键覆盖（取代 `x_execution_session.rs:127-130` 的 upsert 读法）。同 `profile_id@version` 而哈希不同 → `IdentityConflict`，不覆盖，进程非 0 退出（INTERFACE-CONTRACTS:11）。合法升版本时旧行保留，库里能看出曾经绑过哪些账户与钥匙。
- 排除：十个字段直接塞进 X profile（部署路径与政策同一哈希，失效触发被噪声触发）；只按 `profile_id@version` 绑定不校验哈希（改值不升版本发现不了，违反 ADR 0028:52）。

### 三、权限证据三层（Q3 = A，吸收必改 2、3）

三层分开记，任何一层不替代另一层：

| 层 | 是什么 | 载体 | 不证明什么 |
|---|---|---|---|
| 授权声明 | 用户签的 `credential_scope.class` 与 `authorization` 行 | 场所 profile 记录 | 钥匙实际有什么权限 |
| 场所权限事实 | 场所协议对这一类钥匙允许签什么动作 | #1770 能力记录 `session_permission` 项的来源栏：URL、取回日期、sha256 | 手里这把钥匙属于哪一类 |
| 运行时核验 | X 启动后、任何写请求前，核对手里这把钥匙属于声明的那一类、绑在声明的账户上 | X 库绑定行与 X 观测记录 | 场所规则本身 |

**Hyperliquid 场所权限事实**（不开 research 票；2026-11-07T05:49Z 重新抓取公开文档，原件存 `/Volumes/AgentStorage/issue1540-fu18-draft/venue-docs/`，抓取日志 `FETCH-LOG.tsv`）：

| 文件 | sha256 | 本 ADR 用到的内容 |
|---|---|---|
| `info-endpoint.md` | `f4228fc7734af5dbf420e5bdf2b25c248a5b5287035ab2d8a645dc14ce64f0bd` | info 端点 `{"type":"userRole","user":<地址>}`，对 API wallet 返回 `{"role":"agent","data":{"user":<主账户>}}`，其余为 `missing / user / vault / subAccount`。info 查询不签名、不带凭据 |
| `exchange-endpoint.md` | `87cca079cdf4bd68848995198cb1ffee57d55f576b08c341ebed27e23e7cfcd0` | `withdraw3`、`usdSend`、`spotSend`、`sendAsset`、`usdClassTransfer`、`approveAgent` 的 action 带 `signatureChainId`，属用户签名方案；`agentSendAsset`（「can be signed by an agent. Destination must match the source address」）与 `vaultTransfer` 不带，属 L1 方案，agent 可签。另有「Deposit into staking (agent)」与 `agentSetAbstraction` 等标 agent 的动作 |
| `nonces-and-api-wallets.md` | `e63b1672b6d05dddc9b61e327c16e769024140e32482d5627f037c1eafe9b6c2` | API wallet 即 agent wallet，由主账户批准代其签名；查询账户数据要传主账户地址 |
| `signing.md` | `c81b9086c339402ca552ce2ecfba62eeaccf432e58cd110947bcf8d7c4ddcbec` | 两套签名方案：`sign_l1_action` 与 `sign_user_signed_action` |

**`trade_no_transfer` 的定义**：场所**不保证** API wallet 不能划转。`trade_no_transfer` 在 HL 上成立，靠三条叠加：

1. 钥匙是 agent，不是主钱包（第 4、7 步可核）；
2. 对外提现与转给他人需要主钱包签名（上表场所事实）；
3. X 的场所写代码没有任何 transfer 变体（ADR 0039:495 A4 写权闭包；#1770 `hl_adapter_has_no_standalone_send_entry`）。

正本与下游不得把 ADR 0028:22 的字段值读成「场所保证 API wallet 不能划转」。实施时重新抓取上述文档，把日期与 sha256 写进 #1770 能力记录 `session_permission` 的来源栏；内容变了就按新内容重判这三条。

排除：用户权限声明或只读元数据即算证明（声明与事实都「通过」时钥匙仍可能是主钱包）；划转探针（不在 allowed_actions，属 #1340:446 范围外，钥匙若是主钱包探针本身就会划走资金）；只靠场所文档不做运行时核验（证明不了手里这把钥匙是哪一类）。

### 四、P1 授权记录（Q4 = A）

授权写在场所 profile 记录的 `authorization` 行（ADR 0028:26 原位）：`{authorization_id, authorizer, authorized_at}`，随 profile 版本签，计入 `venue_profile_sha256`。任何字段变更出新版本后须重签。X 核的是声明存在且 `authorizer` 等于约定的用户标识，不是签名验证。P1 授权先于机器层签署；ADR 0028:48 只约束 P2 的顺序，P1 不搬用 ADR 0028:72 的 P2 元组。

### 五、P1 同强度机器层（Q5 = A，C 类选择）

- `experiment` 与 `production` 走同一发送门：四项组成（旧 writer 禁入证据、X 排他锁、profile 身份绑定、凭据作用域版本）同样核验，同样在 ADR 0028:46 五类失效触发后首发前重核，同样失败即拒发。
- P1 产出的指纹、绑定行、核验记录都标 `experiment`，永不作为生产证据（ADR 0028:48、:71；CONTEXT.md「证据类」）。P2 首发前仍须在目标部署重核。
- ADR 0045 签署的值不变：SR-3 在 P1 上仍不触发，发送门拒发是发送前件，不生成退出授权记录。改的是 ADR 0045:24 自标 `[新缠论:推论]` 那句的前提读法：ADR 0028:46 的机器层不再只限 production。修订指针落在 ADR 0045 文末。
- 回退：用户如果要维持「机器层只限 production」的读法，说一句即可回退成 B（只在 production 核验）。代价是 TB-09 无法在 P1 上满足 AT-09「被真实部署阻止」。
- 排期：P1 发送要等 #1383 交付指纹算法；#1340:380 的切片顺序 TB-04 → TB-09 → TB-08 本就如此，不新增排程边。

### 六、拒发词汇与持久化（Q6 = A）

- 拒发原因 = INTERFACE-CONTRACTS:15 共同错误集八类之一，加一个封闭细分码。与绑定行同一事务写入（沿 `x_execution_session.rs:98-135`）。
- 启动前就无法装配的错误（引用记录缺失、版本不认、哈希不符、同版本异内容、缺凭据）进程非 0 退出、不写库。装配成功但不满足发送前件的，持久化 `send_state=refused` 后继续只读服务。
- 现有值 `MissingTb09Fingerprint` 读作类别 `MissingDependency`、细分 `Tb09Fingerprint`。可保留 `send_refusal_reason` 原字段原值，另加 `send_refusal_class` 列；选哪种由实施票定，对外可读的类别必须来自共同错误集。

### 七、拒发覆盖范围（Q7 = A，C 类选择，只对 P1 签署）

拒发在当前 `writer_epoch` 内为终态，拦截一切写请求：下单、撤单、reduce-only 平仓，以及急停展开出的命令。只读查询与恢复对账（ADR 0030:48「先只读对账在途订单」）照常。

这是 P1 上的安全语义，不是风险偏好：发送门核的是「发送者是谁、是不是唯一」，撤单与平仓同样是用这把未核身份的钥匙对场所写。放行撤单就等于新立一条「身份未核也免核」的政策。代价是拒发期间系统内急停不可用，用户只能在场所网页端撤单（ADR 0028:38：场外动作是外部事实，系统按回报记账）；P1 是测试币。**本条只对 P1 签署，P2 激活时重裁**：届时「拒发期间真实挂单撤不了」是真金损失，作为 P 类由用户选。

离开拒发只有一条路：修正输入后以新 `writer_epoch` 重启，重走全部步骤。不自动重试，不回退旧 writer（ADR 0028:46），不因失败换值放行（ADR 0041:15）。修正若改了 profile 字段，即新版本，属失效触发。

### 八、公开地址不入仓（C 类选择）

仓库里只放 `profile_id@version` 加 sha256。场所 profile 记录本体（含 `account` 与 `public_id`）放部署状态目录。ADR 0028:20 说地址不是凭据，入仓不违反凭据纪律；但 testnet 与 mainnet 共用地址，`account` 就是用户主网钱包的公开地址，写进仓库会把用户的主网身份与代码库关联。仓内测试只用现场生成的非敏感测试地址。

### 九、测试钥匙

测试私钥一律一次性现场生成，不进仓库，不复用（含任何真实 testnet 钥匙）。

## 数据形状

场所 profile 记录（存部署状态目录，不入仓；尖括号为占位）：

```json
{"record": "venue-profile/1", "profile_id": "P1-hl-testnet", "version": 1,
 "environment": "testnet", "venue": "hyperliquid", "dex": "main",
 "account": "<主钱包公开地址>", "endpoint": "api.hyperliquid-testnet.xyz",
 "credential_scope": {"class": "trade_no_transfer",
                      "source": "user_api_wallet_separate_from_master",
                      "public_id": "<API wallet 公开地址>"},
 "execution_form": "Netting",
 "allowed_actions": ["query", "place", "cancel", "reduce_only_close"],
 "evidence_class": "experiment",
 "authorization": {"authorization_id": "<id>", "authorizer": "<用户>", "authorized_at": "<时间>"}}
```

X 执行 profile `x-execution-profile/2`：`mode`；`venue_profile: {path, profile_id, version, sha256}`；`capability_record: {path, version, sha256}`；`credential_source: {env, file}`；`tb09_fingerprint`（TB-09 算法产出，#1383）。

X 库绑定行（按 `(profile_id, version)` 追加，与 `send_state` 同一事务）：`profile_id、profile_version、venue_profile_sha256、capability_record_version、authorization_id、evidence_class、credential_public_id_verified、session_permission_verified、writer_epoch、send_state、send_refusal_class、send_refusal_detail`。凭据作用域版本由 `(profile_id@profile_version, venue_profile_sha256)` 读出，不另存一列。

## 发送门（8 步）

全部在构造任何写传输之前。`experiment` 与 `production` 同一顺序。

| 步 | 检查 | 不满足时（类别 / 细分） | 处置 |
|---|---|---|---|
| 1 | 两份引用记录存在、版本可认、哈希与声明一致；`public_id ≠ account`；`evidence_class` 与 `mode` 一致 | 缺记录 `MissingDependency`；版本不认 `SchemaUnsupported`；哈希不符 `IdentityConflict`；`public_id == account` 或证据类不一致 `InvalidDomain` | 进程非 0 退出，不写库 |
| 2 | 与已持久绑定行比对 | `IdentityConflict`（同版本异内容） | 进程非 0 退出，不覆盖 |
| 3 | 凭据存在 | `MissingDependency`（既有，`x_execution_session.rs:180-188`） | 进程非 0 退出 |
| 4 | X 在内存中由凭据推出的公开地址 = `credential_scope.public_id` | `IdentityConflict` / `CredentialPublicId` | 持久化 refused，只读服务 |
| 5 | `authorization` 行完整，`authorizer` 为用户 | `AwaitingPolicy` / `Authorization` | 同上 |
| 6 | TB-09 指纹存在且与本次计算一致（含本次绑定与作用域版本） | 缺 `MissingDependency` / `Tb09Fingerprint`；不符 `IdentityConflict` / `Tb09Fingerprint` | 同上 |
| 7 | 会话权限运行时核验：无签名、不携带凭据的 info 查询（HL 为 `userRole(public_id)`），结果须为 `role=agent` 且 `data.user == account` | `UnsupportedCapability` / `SessionPermission`（查询不可用、来源记录缺或结果不符） | 同上；不换口径重试放行（ADR 0041:15） |
| 8 | 全部通过 | 无 | `send_state` 进入可发送状态（具体值由写代码落地的叶定） |

第 1 步原稿写 `VersionUnsupported`，不在共同错误集内；按本 ADR 裁定六改为 `SchemaUnsupported`。

## 能力矩阵（C10.5 × success / Unsupported / fault）

读法：**success** 是一条证明该能力在受测范围内成立的可核记录，P1 上的真实 success 由下游票授权后跑出，带 `experiment`。**Unsupported** 是能力记录对该项声明 `Unsupported{reason}`，adapter 在构造任何传输之前返回 `UnsupportedCapability`，依赖该项的动作不放行（AT-15，#1340:427），且只验收失败处理，不替代成功路径（C12.4，#1340:383）。**fault** 只列类别与出处，不重裁；场所明确拒单是回报不是 fault。现有合同没写的场所事实，一律写「实施现场核实，未核前声明 Unsupported」。

| # | C10.5 能力项 | success 证据与载体 | Unsupported | fault（不重裁） | 下游 |
|---|---|---|---|---|---|
| 1 | 身份与代际 | X 库绑定行（profile_id@version、`venue_profile_sha256`、能力记录版本、`writer_epoch`）；X 观测记录带 `source_namespace`、`source_epoch`（#1770 第 5 项） | 不适用：身份是发送前件，缺失或冲突走拒发 | `StaleWriter`（`x_execution_session.rs:101-107`）；`IdentityConflict`（INTERFACE-CONTRACTS:11） | #1769 后续叶 / #1383；#1401 |
| 2a | 取消 | 撤单金样与观测记录（#1770 `hl_adapter_maps_order_cancel_query_to_golden_requests`）；P1 真实撤单回执（#1412） | 能力记录 `order_target_form` 未知值 → X 启动 `InvalidDomain` | 失响应 `DeliveryUnknown`，按目标查询，不重发（#1340:310） | #1770、#1412 |
| 2b | 改单 | 无 | `Unsupported{原子改单合同未证（C07.8）}`；改价走撤单后再下单 | 不发请求 | #1770；#1412:11 |
| 3 | 累计完整性 | E 已 Recorded 的场所 adapter 原件，种类由能力记录声明（ADR 0060:44） | 场所给不出某类所需证据 → 该子项 Unsupported，减少责任保原潜量 | 源游标缺口进 Pending 或封闸（ADR 0060:42；#1340:297） | #1425、#1400、#1412 |
| 4 | 幂等 / 查询范围 | `cloid` 投影单射且跨重启稳定（#1770）；单订单查询（#1770）；账户级查询（#1400） | 同 ID 重投默认 Unsupported（C07.6 要求另有获准且已实证的强幂等合同）；账户级查询在 #1400 落地前 Unsupported | `DeliveryUnknown`；查询 `Unavailable`（INTERFACE-CONTRACTS:41）；无隐式写重试（#1770） | #1770、#1400、#1422 |
| 5 | 费用 / 更正 | fill 类费用随原 fill 进 E（ADR 0060:52-53） | HL 更正/bust 回报形状实施现场核实，未核前 Unsupported（#1412:13） | 按 ADR 0032 裁定五/六与 ADR 0060:46 | #1425、#1412 |
| 6 | 会话权限 | 三层记录（裁定三）：`authorization` 行；能力记录 `session_permission` 来源栏（URL、日期、sha256）；绑定行 `session_permission_verified` 与 `userRole` 观测记录 | 第 7 步做不成 → `UnsupportedCapability` / `SessionPermission`，拒发；profile 未声明 reduce-only 时 `reduce_only=true` → `UnsupportedCapability`（#1770） | 核验查询失败 → 本次启动拒发，不换口径重试 | #1769 后续叶 / #1383（门）、#1770（变体）、#1400（真实只读核验） |
| 7 | 挂单限制 | 能力记录声明 tick、lot、最小名义值、在挂单上限，附来源；场所拒单作为观测记录（ADR 0028:32） | 未声明项 Unsupported，依赖该项的事前校验不放行 | 场所拒单是回报不是 fault | #1770、#1412 |
| 8 | 时间语义 | 原件带 `received_time`（ADR 0056:52）与场所时间戳；能力记录声明其含义 | 场所无时间戳字段 → Unsupported | 按 ADR 0051 外壳与 C08.6 | #1400、#1425 |
| 9 | 唯一外部事实来源 | 会话一个 `evidence_class`、一个外部事实源（ADR 0056:59） | 不适用（必备项） | `format` 与 `evidence_class` 不一致 → `SchemaUnsupported`，不落行（ADR 0056:46） | #1425 |

ADR 0028:24 的动作维度：

| 动作 | success | Unsupported | fault | 下游 |
|---|---|---|---|---|
| 下单 | 金样（#1770）；P1 真实接受/拒绝/部分成交回报（#1412） | 订单类型表达不了 → `UnsupportedCapability`，不改型 | `DeliveryUnknown`，不重发 | #1770、#1412、#1422 |
| reduce-only 平仓 | `r=true` 金样（#1770 `hl_adapter_reduce_only_follows_frozen_command`） | profile 未声明 → `UnsupportedCapability` | 同上 | #1770、#1412 |
| 单订单查询 | 只读端点金样，写端点 0 请求（#1770） | 无 | `Unavailable`；`unknownOid` 只成观测记录 | #1770 |
| 急停序列 | 撤全部挂单加 reduce-only 平仓的展开与受控测试（#1525）；P1 可执行性（ADR 0028:32） | 拒发期间不可用（裁定七） | 按 ADR 0030:48 续做 | #1525、#1412 |
| 划转、提现、子账户创建 | 不适用 | adapter 无此变体，不可表达；A4 写权闭包锁住场所写代码（ADR 0039:495） | 不适用 | #1770 `hl_adapter_has_no_standalone_send_entry` |

本矩阵不新增能力项。

## 受影响正本

随本 ADR 同一 PR 落地，均为 append-only，不改原文：

1. [ADR 0028](0028-first-capability-verification-profile.md) 文末「修订（#1540）」：`credential_scope` 三项与作用域版本构成；`trade_no_transfer` 按三条叠加读；机器层（:46）对 `experiment` 同样生效；`authorization` 行（:26）为 P1 授权记录；`account` 与 `public_id` 不入仓。
2. [ADR 0039](0039-production-entry-inventory-and-unique-launcher.md) 文末「修订（#1540）」：:285 的凭据来源读作 `credential_source`；:287-288 启动门对 `experiment` 同样生效，并按本 ADR 发送门 8 步与拒发终态执行。
3. [ADR 0045](0045-sr-v1-policy-signed.md) 文末「修订（#1540）」：SR-3 签署值不变；:24 前提读法改为机器层不限 production；回退方式。
4. `CONTEXT.md`「运行边界」节新增两条词条：「凭据作用域版本」「权限证据三层」。

## 受影响代码清单

基线 main `0b3e7f3cf5`。代码实施写入 `rust/src/bin/x_execution_session.rs`，该区现由 #1381/#1942 持有，等 #1942 合入后由 #1365 系列实施票承接，实施者重核行号。

| 文件:行 | 现状 | 改动 |
|---|---|---|
| `rust/src/bin/x_execution_session.rs:21` | `x-execution-profile/1` | 升 `/2` |
| 同 `:28-34`、`:36-79` | 五项 profile | 引用场所 profile 记录与能力记录；`credential_source` |
| 同 `:98-135` | upsert，无冲突检查 | 按 `(profile_id, version)` 追加绑定行；同版本异内容 `IdentityConflict`；拒发类别与细分 |
| 同 `:109-114` | 只 production 缺指纹拒发 | 发送门第 4-7 步，`experiment` 同强度 |
| 同 `:180-188` | 缺凭据 `MissingDependency` | 不变，第 4 步紧随其后 |
| `rust/src/bin/production_session.rs:177-192` | 读 `credential_scope.env/file` | 读 `credential_source.env/file`；校验两份引用记录哈希 |
| `rust/tests/production_session_entry.rs:233-249`、`:580-607` | `/1` 夹具与既有拒发断言 | 夹具升 `/2`；断言按裁定六保留原值或改类别加细分 |
| `docs/runbook_production_session.md:67-81` | `/1` 示例 | 更新示例与拒发说明 |
| #1770 能力记录 | 只定 `order_target_form` 等 | 补 C10.5 九项与 `session_permission` 来源栏 |

下游建议测试（各票 owner 实施时固定；只在 nc-remote/CI 运行；测试私钥现场一次性生成）。不是本 ADR 的关票条件：

| 票 | 测试 | 断言 |
|---|---|---|
| #1769 后续叶 / #1383 | `x_binds_profile_version_and_rejects_same_version_other_content` | 同 `profile_id@version` 改一个字段重启 → `IdentityConflict` 非 0 退出，既有绑定行不变 |
| 同上 | `binding_rows_append_on_profile_version_bump` | 升版本重启后新旧两行都在 |
| 同上 | `credential_source_path_change_is_not_scope_change` | 只改 `credential_source.file` → 作用域版本不变 |
| 同上 | `x_refuses_send_when_credential_public_id_mismatches` | 现场生成的测试钥匙推出的地址 ≠ `public_id` → refused、`IdentityConflict`，本地监听器 0 连接 |
| 同上 | `profile_with_public_id_equal_to_account_is_invalid` | `public_id == account` → `InvalidDomain`，不建库 |
| 同上 | `x_refuses_send_without_authorization` | 缺 `authorization` → refused、`AwaitingPolicy`，0 连接 |
| 同上 | `experiment_profile_without_tb09_fingerprint_refuses_send` | experiment 缺指纹 → refused、`MissingDependency` |
| 同上 | `refused_send_blocks_place_cancel_and_reduce_only_but_serves_query` | refused 下三种写变体 0 写请求；`query` 返回绑定行与拒发原因 |
| 同上 | `send_refusal_class_is_from_common_error_set` | 所有 refused 行的类别 ∈ INTERFACE-CONTRACTS:15 八类 |
| #1770 | `hl_capability_record_lists_all_c10_5_items` | 九项齐全，每项 `Supported`+来源或 `Unsupported{reason}` |
| #1400 | `session_permission_verified_by_user_role_or_unsupported` | 授权后真实 `userRole` 核验写入绑定行；不可用时 `UnsupportedCapability` 且拒发 |

## 对下游的接口

- **#1365 TB-09**：本 ADR 交出指纹两项组成的定义（profile 身份绑定 = 绑定行；凭据作用域版本 = 裁定一）与拒发词汇。指纹算法、旧 writer 禁入与 X 排他锁的取证归 #1383（`/Volumes/AgentStorage/decisions/1523/draft-28G.md:101`）。#1383 在 P1 上的负对照对 `experiment` 门成立，记录带 `experiment`。#1401 消费「拒发在 epoch 内终态、新 epoch 重走全部步骤」。
- **#1368 TB-10**（经 #1365、#1367 传递）：迁移或回退后 X 新 epoch 必须重走发送门，绑定行不回退。P2 授权记录格式不变（ADR 0028:72）。
- **#1369 TB-11**：整体验收的「能力 hash」取能力记录版本与 `venue_profile_sha256`；P1 证据与 production 分列。
- **#1551 FU-20（AT-18）**：Destination ⑤「目标部署的隔离和权限证据」按裁定三的三层核，缺一不得记权限证据成立。
- **#1367 / #1382 / #1400 / #1412（TB-08）**：#1400 落地第 7 步的真实只读核验与账户级查询；#1412「逐动作核其明确授权」读作 profile `authorization` 行；能力矩阵是 TB-08 的逐项对账表。
- **#1770 / #1769**：#1770 能力记录补九项与 `session_permission` 来源栏；#1769 后续叶（经 #1365 系列）改 X profile 到 `/2`、加绑定行与发送门第 1-7 步。

## 不在范围

- P2 场所、账户、真钱首发、主网存款与数额（#1561；ADR 0028「P2 处置」）。
- TB-09 指纹算法本身、旧 writer 禁入与 X 排他锁的取证（#1383、#1401）。
- `OwnAttempt / MayHaveCalled` 实体规则与交付未知后的责任处置（#1330；ADR 0051:320）。
- 成交/持仓唯一真账与 crosswalk（ADR 0060）、迁移 crosswalk（#1535）。
- 用户 testnet API wallet 是否已备、testnet 余额现读数：实施现场事实，由用户与 #1400 在授权后核实。
- 撤销或更换真实凭据、改账户权限（#1383:14 要求先取得具体授权）。
- 任何真实场所写调用、划转探针、凭据值的记录。
- builder DEX、子账户、强平、abstraction 默认值（ADR 0028:32）。
- SR 清单阈值与退出动作表（ADR 0045、ADR 0030），急停展开（#1525）。
- D/IBKR paper、E/Binance 只读候选的 profile 实例化（ADR 0028「合法候选」节）。本 ADR 的规则与场所无关，只实例化 P1。
- 修改 blocking 边。#1540 阻塞 #1365、#1367、#1369、#1382、#1551，不变。

## 验收

1. 本文件存在于 main，且下列四条各自 `grep -c` ≥ 1：`credential_scope_version` 或「凭据作用域版本」、`send_refusal_class`、`SessionPermission`、`AwaitingPolicy`。
2. `grep -c '修订（#1540' docs/adr/0028-*.md docs/adr/0039-*.md docs/adr/0045-*.md` 每个文件 ≥ 1。
3. `grep -c '凭据作用域版本' CONTEXT.md` ≥ 1。
4. `gh api repos/Newchanlun/NewChanlun/issues/1540/dependencies/blocking --jq 'map(.number)|sort|map(tostring)|join(",")'` 输出 `1365,1367,1369,1382,1551`。
5. #1540 在本 ADR 的 PR 合入后关闭，resolution 评论含本文件路径、独审原件路径与裁定评论链接。

## 溯源标签

全部 `[新缠论:选择]` 或 `[新缠论:推论]`。缠师原文对程序化执行场所无论述（同 ADR 0028）。Q5、Q7、公开地址存放为 `[新缠论:选择]`（C 类，主控裁定，用户可一句话回退 Q5）；`trade_no_transfer` 三条叠加读法、拒发类别映射为 `[新缠论:推论]`（出处见各条）。
