# Hyperliquid margin table 改动对已开仓位的影响 + disableDex 语义

- 票：[#942](https://github.com/xy7365527-lang/NewChanlun/issues/942) 第 6、7 问（本票在凭证约束下仅剩可做的两问，2026-08-08 编排者「走乙」裁定后的范围表）；图 [#931](https://github.com/xy7365527-lang/NewChanlun/issues/931)
- 性质：**纯文档查证**。全程无签名、无下单、无凭证。
- 文档语料：`https://hyperliquid.gitbook.io/hyperliquid-docs/llms-full.txt`（2026-08-08 重新抓取，495,870 字节，与 [#939](https://github.com/xy7365527-lang/NewChanlun/issues/939) 报告使用的语料字节数相同，视为同一版本）
- 第三方 SDK 交叉核验：`nktkas/hyperliquid`（TypeScript，社区维护但对 HIP-3 deployer actions 有逐字段注释）`src/api/exchange/_methods/perpDeploy.ts` @ commit `db80d598f6e4672edc090fe69994ecec97ebc980`
- GitHub 代码检索：`gh api search/code`（需 gh 认证，已用本机 `gh` 凭证；注意 GitHub code search 不支持 `org:` 精确限定 HIP-3 相关仓库全集，检索范围见下方各条附带的检索式）

---

## 结论先行

**第 6 问（margin table 下调是否改已开仓位的强平价）**：**会改**。官方强平价公式 `liq_price = price - side * margin_available / position_size / (1 - l * side)` 中 `l = 1 / MAINTENANCE_LEVERAGE`，且原文明写「For assets with margin tiers, **maintenance leverage depends on the unique margin tier corresponding to the position value at the liquidation price**」——这是一个实时求值的量，不是开仓时刻的快照。维护保证金公式 `maintenance_margin = notional_position_value * maintenance_margin_rate - maintenance_deduction` 同样只由**当前**保证金表（margin tiers）决定。文档里「Leverage is only checked upon opening a position」这句**只覆盖初始杠杆/初始保证金的选择**，不覆盖维护保证金——两者是文档里明确分开的两个小节（Initial Margin and Leverage / Maintenance Margin and Liquidations），后者从未说过"锁定在开仓时刻"。

**第 7 问（disableDex 语义）**：**全站与已知第三方 SDK 加起来仍然零说明文字**，只多确认了两件事：① `disableDex` 是**整个 DEX 级**动作（参数是 dex 名字符串，不含 coin），与**逐资产**的 `haltTrading`（参数是 `{coin, isHalted}`）在作用域上不是一回事；② 谁能调与 `haltTrading` 走同一套权限模型——deployer 本人，或被 `setSubDeployers` 显式按 `variant: "disableDex"` 授权的地址。**对已开仓位的后果查不到**，两个已知动作里只有 `haltTrading` 有官方文字描述"取消挂单+按当前标记价结算"；`disableDex` 完全没有对应描述,是否触发结算、是否需要先对全部资产 `haltTrading` 才能调用、是否可逆，均未查到。

---

## 第 6 问详答：margin table 改动 vs 已开仓位强平价

### 6.1 官方原文——维护保证金公式（与开仓时刻无关）

【官方原文】Margin tiers 页：

> `maintenance_margin = notional_position_value * maintenance_margin_rate - maintenance_deduction`
>
> On Hyperliquid, `maintenance_margin_rate` and `maintenance_deduction` depend only on the margin tiers, not the asset.
>
> `maintenance_margin_rate(tier = n) = (Initial Margin Rate at Maximum leverage at tier n) / 2`. For example, at 20x max leverage, `maintenance_margin_rate = 2.5%`.

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/trading/margin-tiers>

`maintenance_margin_rate` 由 `maxLeverage`（保证金表里的字段）直接决定。部署者用 `setMarginTableIds` 把某资产指到另一张表、或用 `InsertMarginTable` 改表本身，都是在改这个输入——公式本身不含任何"仅对新开仓生效"的开关。

### 6.2 官方原文——强平价公式（逐笔实时求值）

【官方原文】Computing Liquidation Price 节：

> The precise formula for the liquidation price of a position is
>
> `liq_price = price - side * margin_available / position_size / (1 - l * side)`
>
> where
>
> `l = 1 / MAINTENANCE_LEVERAGE`. **For assets with margin tiers, maintenance leverage depends on the unique margin tier corresponding to the position value at the liquidation price.**

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/trading/liquidations>

`MAINTENANCE_LEVERAGE` 直接来自保证金表；公式没有"入场时刻固定值"这个概念——它写的是"取决于……的那一档"，用现在时态描述一个随时可重新求值的量。同一页往前几句还写：

> Once a position is opened, a liquidation price is shown. This price has the certainty of the entry price, but **still may not be the actual liquidation price** due to funding payments or changes in unrealized pnl in other positions (for cross margin positions).

——即官方自己就说"开仓时显示的强平价不是最终值"，理由列了资金费率、其他仓位盈亏两项，**没有列举"保证金表变了"这一项，但也没有把保证金表排除在"会变"的范围之外**——这句话本身不构成对第 6 问的直接肯定或否定，只作为佐证：官方从未承诺开仓时的强平价是固定值。

### 6.3 「Leverage is only checked upon opening a position」到底覆盖什么

【官方原文】Initial Margin and Leverage 节（与 6.2 引用的 Maintenance Margin and Liquidations 节是文档里紧邻但明确分开的两个小节）：

> Leverage can be set by a user to any integer between 1 and the max leverage. Max leverage depends on the asset.
>
> The margin required to open a position is `position_size * mark_price / leverage`. …
>
> **The leverage of an existing position can be increased without closing the position. Leverage is only checked upon opening a position. Afterwards, the user is responsible for monitoring the leverage usage to avoid liquidation.**

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/trading/margining>

这句话的主语是"用户选择的杠杆值（initial margin 的输入）"——它说的是：用户开仓时选了 5x，之后即使账户状态变化导致有效杠杆超过 5x，系统不会因此强制平仓或拒绝，用户自己担监控责任。**这条与"维护保证金率随保证金表实时变化"不矛盾、也不覆盖它**：一个管的是"用户输入的杠杆倍数是否被二次校验"，一个管的是"强平判定公式里的参数是否随时间变化"。二者在文档结构上分属不同小节，字面上没有交集。

### 6.4 【推断】综合结论与推断链

**推断链**：
1. 强平价公式的 `MAINTENANCE_LEVERAGE` 依赖"当前保证金表" → 前提① 已由 6.2 官方原文实锤。
2. 维护保证金公式 `maintenance_margin_rate` 同样只依赖保证金表（tiers），不含任何"按开仓时刻锁定"的项 → 前提② 已由 6.1 官方原文实锤。
3. 「Leverage is only checked upon opening」只覆盖初始杠杆选择，不覆盖维护保证金/强平价计算 → 前提③ 已由 6.3 的小节边界实锤。
4. 三个前提叠加 ⟹ **部署者下调保证金表（提高维护保证金率）会立刻改变所有已开仓位的强平价计算结果**，不需要用户做任何操作、不需要等到下次开仓。

**未查到、需要标注为缺口的部分**：
- 官方文档**没有一句话直接说"改保证金表会影响已开仓位"**——上面是从两处独立公式反推出的必然结果，不是直接引文。若要把"会改"升级为"官方明确承认会改"，仍然查不到那句直接陈述。
- **改动生效的时间粒度未知**：是链上下一个区块立即生效，还是有任何延迟/排队机制，文档未提及；`setMarginTableIds`／`InsertMarginTable` 的类型定义（`hyperliquid-docs/for-developers/api/hip-3-deployer-actions`）里也没有速率限制或预告期的字段，与 `SetDeployerFees` 明确写"rate limited to one change per 30 days"形成对比——**保证金表改动没有类似的限速条款，查不到任何限制**。
- 是否存在"部分仓位因保证金表下调而立即处于强平线以下（需要立即强平）"的过渡期特殊处理（例如宽限期），**查不到**。

---

## 第 7 问详答：disableDex 语义

### 7.1 全站检索——确认仍是零说明文字

检索式：对 495,870 字节的 `llms-full.txt` 做 `grep -n -i disableDex`，命中 2 处，均为类型定义本身（TS 风格与其重复出现的 JSON 风格 schema 段落），**无任何自然语言说明**：

```
type PerpDeployAction =
  ...
  | {
      type: "perpDeploy";
      disableDex: string;
    };
```

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/hip-3-deployer-actions>

与 [#939](https://github.com/xy7365527-lang/NewChanlun/issues/939) 报告记录一致（该报告也是「全站检索仅 1 处命中类型定义本身」，本次重新抓取命中 2 处是因为该 gitbook 页面在文档 llms-full.txt 里被完整重复渲染了一次表格+代码块两种格式，不是新增说明）。

### 7.2 第三方 SDK 交叉核验——多了字段级注释，仍无行为说明

【第三方 SDK 原文】`nktkas/hyperliquid`（社区维护、非官方，但对协议做了细粒度类型注释）:

```typescript
v.object({
  /** Type of action. */
  type: v.literal("perpDeploy"),
  /** Name of the perp dex to disable. */
  disableDex: v.string(),
}),
```

来源：<https://github.com/nktkas/hyperliquid/blob/db80d598f6e4672edc090fe69994ecec97ebc980/src/api/exchange/_methods/perpDeploy.ts#L237-L242>

对应测试文件只验证"未授权地址调用会被 API 拒绝"，没有验证行为：

```typescript
{ disableDex: "test" },
```

来源：<https://github.com/nktkas/hyperliquid/blob/db80d598f6e4672edc090fe69994ecec97ebc980/tests/api/exchange/perpDeploy.test.ts#L195>

检索式：`gh api "search/code?q=disableDex+repo:nktkas/hyperliquid"`（2 处命中，均在上）；同一检索式对 `hyperliquid-dex/hyperliquid-python-sdk` 命中 0 处（该官方 Python SDK 未收录 HIP-3 deployer actions）；未检索到官方 `hyperliquid-dex/hyperliquid-rust-sdk` 中的对应实现（本次未逐一验证该仓库，只覆盖了 nktkas 与官方 python-sdk 两个仓库，**非全体 SDK 穷举**）。

**新增的唯一实质信息**："Name of the perp dex to disable"——即参数是 **dex 名字符串**，作用域是**整个 DEX**（对比 `haltTrading` 的参数是 `{coin: string, isHalted: boolean}`，作用域是**单个资产**）。这条不是行为说明，只是确认了作用域粒度，来自第三方注释而非官方，标注为参考而非依据。

### 7.3 disableDex 与 haltTrading 的关系——【官方原文】能确认的部分

**类型层面（官方 schema）**：两者是 `PerpDeployAction` 联合类型里的两个**平行、互斥**的变体，不是同一动作的两个名字：

```
haltTrading: { coin: string; isHalted: boolean };
...
disableDex: string;
```

来源同 7.1。**haltTrading 逐资产、可逆（isHalted 是布尔开关，`false` 即恢复交易）；disableDex 整个 dex、参数里没有布尔开关**，字面上看不出是否可逆。

**权限层面（官方原文）**：两者受同一套委托机制管辖——`SubDeployerInput.variant` 字段的官方注释：

> `variant: string; // corresponds to a variant of PerpDeployAction. For example, "haltTrading" or "setOracle"`

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/for-developers/api/hip-3-deployer-actions>

**⟹「谁能调」的答案**：dex 的 deployer 本人天然拥有全部 `PerpDeployAction` 变体的权限；此外 deployer 可用 `setSubDeployers` 把某个具体变体（含 `disableDex`）单独授权给某个地址。因此"能调 disableDex 的人"= deployer + 被显式加进 `disableDex` 变体授权名单的地址，**这条与 haltTrading 走的是完全同一套权限模型**（这一点官方原文直接支持，是本报告里为数不多能对第 7 问给出确定性回答的子问题）。

**行为语义层面（含 disableDex 对已开仓位的后果）**：**查不到**。官方文档、社区 SDK 注释、SDK 测试用例三处检索均无一句描述。以下均为【推断】，非官方陈述：

- 【推断】HIP-3 spec「Settlement」节紧邻写「Once all assets are settled, a deployer's required stake is free to be unstaked」——"settled" 指的是用 `haltTrading` 把 dex 内**每个资产逐一**halt。这句话在文档结构上出现在 `haltTrading` 段落之后、且该节标题就叫「Settlement」，`disableDex` 从未在这一节或附近出现。**推断链**：若 `disableDex` 是"整体退出 DEX"的收尾动作，逻辑上应当依赖"全部资产已 settled"这个前提；但文档从未把 `disableDex` 和这个前提连起来，**这条推断没有官方原文支撑，只是结构位置上的联想**，不能当结论用。
- 【推断】另一种可能：`disableDex` 是比 `haltTrading` 更严厉的"熔断"动作——不逐资产结算，而是直接冻结整个 DEX（例如拒绝新的挂单/成交，但**不主动**把已开仓位按标记价结算，只是让它们"晾在那"，直到强平引擎照常按市场价格把它们打掉，或者部署者恢复 dex）。这个假设同样**没有官方原文支撑**，只是基于"参数里没有 isHalted 布尔开关、也没有恢复语义"这一结构性观察的猜测。
- 两种假设**互相冲突**（一个假设 disableDex ⊇ haltTrading 的效果，另一个假设 disableDex 与 haltTrading 是独立的两套动作、后果不同），**本报告不裁定哪个对**，因为没有能裁定的官方证据。

### 7.4 关于「11. 美股停牌」补问里的引用——一并核对

issue 里 2026-08-07 补的第 11 问引用了「该动作【官方原文】"cancels all orders and settles positions to the current mark price"，无预告无赔付」，这句原文本次重新核验，出自 `haltTrading` 而非 `disableDex`：

> The deployer may settle an asset using the `haltTrading` action. **This cancels all orders and settles positions to the current mark price.** The same action can be used to resume trading, effectively recycling the asset.

来源：<https://hyperliquid.gitbook.io/hyperliquid-docs/hyperliquid-improvement-proposals-hips/hip-3-builder-deployed-perpetuals>（HIP-3 spec「Settlement」节，与 #939 报告一致，本次原文核对无出入）

⟹ 第 11 问里"该动作"应理解为 **`haltTrading`**（逐资产），不是 `disableDex`（整个 dex）——**这两个动作在讨论"部署者能拿已开仓位怎么办"时不能混为一谈**，`haltTrading` 有明确的强制结算文字，`disableDex` 完全没有。

---

## 部署者权力清单（汇总判定，含 #939 已查内容）

| 动作 | 作用域 | 【官方原文】对已开仓位的记载 | 预告/时间锁 | 赔付 | 可逆性 |
|---|---|---|---|---|---|
| `haltTrading`（`isHalted: true`） | 单个资产（coin） | **有**：「cancels all orders and settles positions to the **current mark price**」 | **未查到任何预告期或时间锁** | 无——这是协议层强制结算，不是罚没，本身不涉及赔付概念；若结算时价格不利，用户自担 | **可逆**：同一动作 `isHalted: false` 可恢复交易（原文「The same action can be used to resume trading」） |
| `disableDex` | 整个 DEX | **查不到**——全站+第三方 SDK 检索均无说明文字 | 查不到 | 查不到 | 查不到（类型里没有布尔开关，无法从 schema 反推） |
| `setMarginTableIds` / `InsertMarginTable`（改保证金表） | 单个资产或整表 | **本报告 6.4 推断**：会改变已开仓位的强平价（强平价公式实时读当前保证金表），但官方从未直接写"会影响已开仓位"这句话 | **未查到任何速率限制**（对比 `setDeployerFees` 有「rate limited to one change per 30 days」的明文限速，保证金表改动没有对应条款） | 不涉及——这不是罚没机制，是参数调整 | 可逆（部署者可以再次调整回去，但对已经因此被强平的仓位无法挽回） |
| 质押罚没（slashing，验证人投票触发，非部署者单方面动作，列入对照） | 部署者的 500k HYPE 质押 | 不直接结算用户仓位，但「the slashed stake by the deployer is **burned instead of being distributed to affected users**」（#939 已查） | 由验证人投票决定，无用户可预知的时间表 | **明确无赔付**——罚没的 HYPE 销毁，不进受害用户口袋 | 不适用 |

**三行结论**：
1. **`haltTrading` 是目前唯一一个有官方文字描述"部署者可强制结算已开仓位"的动作**——无预告、无赔付，但至少行为本身（按当前标记价结算）是写在文档里的。
2. **`disableDex` 是文档里唯一一个"部署者拥有但完全没写会做什么"的动作**——这本身就是风险：不知道后果比知道后果更难防御。
3. **保证金表下调不需要专门的"结算"动作就能杀伤已开仓位**——它不取消订单也不主动结算，而是通过改变强平价公式的输入，让已开仓位在**下一次价格波动**时更容易撞上强平线。三者叠加：部署者手上至少有一个"直接结算"工具（haltTrading）、一个"未知严重程度"的工具（disableDex）、一个"间接但同样有效"的工具（改保证金表）。

---

## 未查到的部分与原因（照 090 逐条）

1. **disableDex 对已开仓位的具体后果** —— 官方文档、第三方 SDK 注释、SDK 测试用例三处检索均为零说明，Hyperliquid L1 共识层不开源，无法从实现代码反查。
2. **disableDex 是否需要先对 dex 内全部资产 haltTrading 才能调用** —— 无官方原文，7.3 节两种假设互相冲突，无法裁定。
3. **disableDex 是否可逆** —— 类型签名里没有布尔开关（不像 haltTrading 的 isHalted），无法从 schema 反推，官方文字缺失。
4. **保证金表改动生效的时间粒度（是否有延迟/排队/宽限期）** —— `setMarginTableIds` / `InsertMarginTable` 类型定义无相关字段，文档正文无相关说明；与 `setDeployerFees` 有明文限速形成对比但不能反推保证金表也有同等限速。
5. **官方对"改保证金表会影响已开仓位"这句话的直接陈述** —— 查不到直接陈述，6.4 节的结论是从两处独立公式反推出的必然结果，非直接引文，已在报告中明确标注为【推断】。
6. **官方 Rust SDK（`hyperliquid-dex/hyperliquid-rust-sdk`）里 HIP-3 deployer actions 的实现细节** —— 本次检索只覆盖了 `nktkas/hyperliquid`（TS，社区）与 `hyperliquid-dex/hyperliquid-python-sdk`（官方 Python，命中 0，未收录该功能），未逐一核对 Rust SDK，非穷举全部官方/社区 SDK。

**凭证纪律遵守情况**：全程零签名、零下单、零凭证使用；`gh api search/code` 仅用于公开代码检索，未涉及任何私钥或账户操作。
