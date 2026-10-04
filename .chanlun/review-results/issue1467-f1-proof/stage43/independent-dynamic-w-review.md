# #1467/#1468 动态 W 首段独立数据与代码评审

日期：2026-10-04。范围：只读研究树中的 `DynamicWDomain-v1.md`、`dynamic_w_domain_probe.mjs` 及其冻结依赖；正式证据为 `dynamic-w-domain-v1-verified/`。本评审只确认有限样本的数据、算法与报告是否相符，不裁定缠论原义或通用定理。

**结论：该有限首段检验通过，未发现阻断缺陷。** 独立复跑和另一套原始订单重建均得到 e55 / capture line422 首败、e54 冻结、6 个成熟核心和 2 个完成对象。确认恰在 e20/e45，未产生第三对象。此结论不授予生产 F2 消费资格。

## 1. 独立证据与实际运行

研究树：`/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun`。

证据根：`/Users/silencehan/Documents/Codex/research-evidence/issue1467`。

实际运行：

```sh
cd /Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun
node --check .chanlun/review-results/issue1467-f1-proof/dynamic_w_domain_probe.mjs
node .chanlun/review-results/issue1467-f1-proof/dynamic_w_domain_probe.mjs --out /Users/silencehan/Documents/Codex/research-evidence/issue1467/dynamic-w-domain-v1-independent
cd /Users/silencehan/Documents/Codex/research-evidence/issue1467/dynamic-w-domain-v1-independent
shasum -a 256 -c output-hashes.sha256
python3 /Users/silencehan/Documents/Codex/research-evidence/issue1467/completion-interface-v1/independent_dynamic_w_check.py
```

Node 语法检查、复跑、四项输出哈希校验和独立 Python 验算均 exit=0。Python 脚本不导入作者实现；直接读取原始捕获首段，每一步由订单集合重新聚合所有价档，用 `fractions.Fraction` 解析原始十进制价量、计算 W 和比较区间。平台用 `itertools.groupby` 分组，再以成熟边变号触发前块完成；不同于作者两个实现的批量边块映射。

核验量：原始 capture 前 422 行；line340 快照中 49,166 笔订单；独立从快照序号选出 line297–339 的 43 条补应用；后续 line341–422 的全部 82 条应用；55 个模型观测；82 对 mutation 前后报价（初始化27对、后续55对）；n=0…55 共56个前缀。源序号连续核至 `19247051671`。

计数由原始订单是否实际改变独立判定，未直接采信 Stage26 的 disposition：

| 阶段 | open | canceled done | match | received | nonresting done |
|---|---:|---:|---:|---:|---:|
| 初始化补应用 | 15 | 12 | 0 | 16 | 0 |
| B0 后 | 26 | 28 | 1 | 26 | 1 |

因此后续正好是55条簿变和27条无簿效应消息。源码与本文不包含订单身份；独立脚本只在内存内处理身份。

## 2. 逐项核验结果

**动态 W 与精度。** 探针第35–41行使用当前真实双边最优价及当前聚合量，分母含价格定点 scale，单位换算正确。冻结适配器将原始十进制直接转为 BigInt，Rat 用约分及交叉乘法；价格比较未走浮点。独立 Fraction 重建逐项相等，所有核验状态满足 `b<W<a`。e12 是真实 match 扣 maker 残量后的新 W，仍在新 profile 域内，原 e1–e11 的11观测平台合法封口；没有据固定 W 的退出结果续接对象。

**初始化和事件过滤。** line340 首次 ready，快照与43条补应用只形成 B0；种子从下一条真实簿变起编 e1。独立读取 raw 自行以 snapshot sequence 选出的补应用恰与 Stage26 一致。初始化27次簿变未算平台种子，received 与 nonresting done 未算种子，簿外价位的真实增撤即使不改 W 仍保留为观测。`x0=x1` 实测成立。

**连续性和来源。** 全部125条应用逐条核原始 sequence、capture line、native type、native time、capture time、available line/time、规范化结果和 disposition。capture 行341–422连续；源序号从快照开始经补应用到停止点连续；每条 post-B0 capture 恰有一条应用。连续性账与55个观测的来源引用均与原始记录对应。Stage16 仅作为 B0 报价交叉核验，没有加入平台种子。

**因果时钟。** 每个模型前缀只由当时已追加的 W 构造；源码第118–127行在整批报价与 frontier 对账后标记 publication。独立检查全部非空前缀 publication 与其最后观测可知时刻相等。初始化补应用的 available capture 均为 line340，并不冒充其较早的 native 时刻；完成种子以及完成 native/capture 时间均回查原始消息。Node 会整文件读入和验 SHA，但关系求值只推进到首败，没有用后续值修改前缀。独立 Python 的订单重放仅至line422。

**首败与冻结。** 前六成熟平台区间为 e1–11、e12–17、e18–22、e23–42、e43–46、e47–52；e53/e54 仍是两观测待定平台，未提前生成成熟反向边。e55 在 line422 将其封口，首败原因精确为 `closed_short_platform`，源序号 `19247051671`。native=`2021-01-01T00:00:02.072849Z`，available capture=`2021-01-01T00:00:03.0699408Z`。状态冻结到e54，R保留e43–e55且e55单独标未分析，没有删除短平台或另选窗口。

**完成、范围和成员归属。** M1拥有e1–e17及K0/K1，种子e18/e19/e20；M2拥有e18–e42及K2/K3，种子e43/e44/e45。e18/e19、e43/e44尚未确认，确认恰在20/45。所有56个前缀独立检查 C+R 原始观测恰覆盖一次、成熟核心自有列表不重复、活动尾部锚引用不算成员。每个对象的 start/end 读真实观察状态；range 由含起终状态的实际 W 序列求 min/max，独立精确相等，没有以方向或插值代替真实范围。后继种子作为证明引用不被前对象吞作成员，旧完成对象在所有后续前缀逐字段不变。

M1确认对应line370，native=`2021-01-01T00:00:01.680719Z`，available=`2021-01-01T00:00:03.0696994Z`。M2对应line408，native=`2021-01-01T00:00:02.024855Z`，available=`2021-01-01T00:00:03.0699058Z`。

## 3. 独立性与未检验边界

作者的 `naive` 没有调用原 `platforms/validRelation`，但仍共用 `rel.R` 及输入 W，因此不能单靠作者两路结果一致排除有理运算、观察过滤或适配器的共同缺陷。本评审以 Python Fraction、从原始记录独立选择43条初始化、全订单重新聚合、原始消息独立分类补核这些层；该样本没有发现共同缺陷。不存在一份有限测试能够证明“一切共同缺陷均不存在”，本结论只覆盖以上实际核验对象。

非阻断局限：

- 此样本只有一个后续单应用批序列，没有实际检验多应用恢复批或乱序恢复的通用语义；文档明确保留该边界。
- size-change 分支未被本首段触及；独立脚本也刻意只接受本段实际出现的消息种类，不能外推交易所全协议支持。
- 作者复用适配器的初始化循环没有像post-B0循环那样逐项断言全部时钟元数据；本评审实际回查43条均正确。这是可加强断言的地方，不构成当前数据结果错误。
- first-ready 来自已冻结 Stage26 的在线状态账；本次重建验证该B0及其来源，没有重做 Stage26 的全部恢复状态机资格评审。
- 这是历史探索分钟的首段，不是留出确认集。未重放余下分钟、换窗、下载、训练、调参、比较市场收益，未证明信息增益。
- 文档将两对象、候选方向和 R_W 观察轴限制在有限诊断，未声称原文走势、第三对象、严格父核心或 F2 原义消费资格。该表达未越过本次实测证据；本评审未承担缠论定理审查。

## 4. 指纹与复跑一致性

源文件（均位于研究树 `.chanlun/review-results/issue1467-f1-proof/`）：

| 文件 | SHA-256 |
|---|---|
| DynamicWDomain-v1.md | `1fa56bf6efe886a3d1f1dfaa624fca50aa5910cd0657402aa379f8e793970701` |
| dynamic_w_domain_probe.mjs | `0030296057afb4b051d53aef4996e26720329cad1d7f4efab442dc6d25472afc` |
| PlateauBaseCandidate-v2.md | `ff4e1d062e78891ff2c563ae06ba3ac842ac10885a5d275795fcd852624daa43` |
| plateau_base_probe.mjs | `0a630a6bbc418a0fa57be621ea2089123005a5b3dcd712622c81738f2a3515ac` |
| stage42/plateau_data_domain_probe.mjs | `8deeb0bbfb6f4110c610542fdf4e9c4bcb632168de937cb8187186f76d6a0a95` |

输入指纹经作者脚本本次实际验证：原始 full.ndjson `8fa0b1a21a02140aafcc679fb656f39499f8e24c4b0928a436852b4f9d7407fd`；Stage16 `a52374d3d1fc922eca44a692cc4243db2a083ee93346bc5b2181292ae66a47a5`；Stage26 gzip `5c233367178b64ddbcd992bf869793f4bf12a415ed90d50d1ee1d481d04763cf`，解压字节 `6a3c7eb8a47c5c2e9cdbbee77200286be6133349f2a5d328d3cec300c0496b30`；旧固定W摘要 `32e59e88edcbe18c8b90b5dcccf6de50a86177474f111cb1f8cadad61169ddc9`。

`dynamic-w-domain-v1-independent/` 对比正式 `dynamic-w-domain-v1-verified/` 的四个JSON及manifest，**逐字节相同**。这是实际读取字节比较的结果。本组摘要并无运行时间或输出目录字段，故排除字段集合为 `[]`；stdout中的`out`不同，不属于输出JSON比较。原生/capture时间是业务证据，不排除。

| 输出 | 两目录共同 SHA-256 |
|---|---|
| summary.json | `f3e01dbd71ed8d944fc111bd1a83eca7253e752be373417fa29b34d001b3b242` |
| observations.json | `98065709bba09fcbca4445951af1158f39cdc9ecddbb8c5de8fc2f62ec8d4f32` |
| continuity.json | `a7694f04a62a1353062d4e6102abe6253f7d7536c68926ff89efbf17aa9c3a6e` |
| prefixes.json | `c66c87bd77fa529c367ac1ffcacca7c679649ac0476acb0c5bf48e28ed45a20d` |
| output-hashes.sha256 | `cebeb19838e010040ffdcad2daf4ff87fde5d262f237e75cd25a0a36bde8cde3` |

独立附证：同目录 `independent_dynamic_w_check.py` 与 `independent-dynamic-w-check.json`。作者结果未覆盖，仓内文件未修改，无提交、推送或外部消息。
