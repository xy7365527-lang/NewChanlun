# Stage57 冻结整合与同一 main8 普通组合独立审查

2026-10-05；研究 #1467，归图 #1465。名分：工作草稿。复核者 `/root/review_stage57_integration` 为新上下文工位，未继承作者会话。读取指定冻结入口、两作者稿、两实际声明读回、两份独立比较及必要控制字段；本次是整合忠实度审查和一个普通组合证明，不是重新签署全部历史数学。

**结论：在明示范围内通过（passed_scoped），没有阻断 finding。** 主汇总、ScopeClarifications、总证据 JSON 与 own progress patch 忠实保留两包的结论和限制。同一 D57-main8 在 τ=8 确实提供 j=3、r=4 的 RawTerminal，并可复用已审 T57 得到固定 K/C/s 的连续 TouchSpan 最大性。原义 P、ER57、B56、固定 F₂ 和市场增量均未获证明。

## 冻结身份与计数

入口 `integration-input-manifest-v1.json` SHA-256：`ab4ea5f4090e22f2b9849f3cc95e27453ddd9cad7cf9e50de41d07931ea87b79`。总证据 `stage57-evidence.json` SHA-256：`6c13dd7148a31c2e466a8b801bf07742bafb33996f0f78f28799300aa21ea27a`。两者均与派发锁一致。

| 清单组 | 记录数 | 大小与 SHA 核验 |
|---|---:|---|
| 整合入口输入 | 9 | 全部吻合 |
| own progress patch | 1 | 吻合 |
| 总证据 files | 39 | 全部吻合 |
| 总证据 external_receipts | 9 | 全部吻合 |

这是 58 条清单记录、49 个独立路径：入口中 8 件已在总证据 files，patch 又出现在 external_receipts，9 个重复出现不另算独立文件。39+9 组内部是 48 个独立路径；入口额外加入总证据 JSON 本身。入口 manifest 是另外核验的锚文件，不混入上述 49。计数表示文件身份，不表示独立实验或独立样本数。逐项结果保存在仓外 `integration-review/input-integrity.json`。

9 个 external_receipts 项实为两份读回回执、两份比较回执、两份作者运行 manifest、Real 超时回执、保留的 Real 尝试源及 own patch；后两种文件不能改称额外成功运行。两份独立运行 manifest 另按对应读回回执中的 SHA 核验，状态抽取见 `status-summary.json`，不并入原 39+9 清单计数。

## 整合忠实度

| 承重事项 | 核对结果 |
|---|---|
| D54 与 RawTerminal | 三 seed 至最终 L 全触 C 的普通证明只支持 D54 成功→RawTerminal。RawTerminal 的 j≥s+2 不替代 D54 的 seed 后逐邻、好角色及发布条件；无逆向等价。whole 的 NoCore 前导与从 s 起的 TouchSpan 分开。 |
| 泛型根与解释桥 | 根接收 Type 0、任意二元关系 le、总 Tape、已有 ht；不构造终点，不认证 seed 来源、一般几何或在线检测。实数闭交、有限前缀补齐和 D54 提取均保留为普通证明。首次阈值仅针对该关系。 |
| 原义运输 | ER57 须涵盖同一身份的全部合法延伸，包括新核、改类型及 R 内终点；当前未证，也无满足全部原义前件的反例。不可延伸与完成、前界、核解释及 F₂ 仍分立。 |
| A57 | 明示原 Step/Through、真实 Reads、长度及所有原状态切点有效双边 QuoteOf；构造器长度检查不保证该域。一般适配是普通证明，范围是实际 L1 spread 支撑，不是深档、成交价或交易所协议。 |
| 新旧贡献 | Stage46/47 的包络作为已审依赖；Stage37 整包仍按原待审状态读取。primitive_strict 是独立条件辅助声明，不在有限 witness_root 闭包内。A57 与完整 D54ᴰ 扫描未被冒称为有限根已形式化。 |
| 控制与来源 | 两控制共 15 事件、17 个含初态前缀、0 市场样本。main8 的 whole 拥有 E1…E6，E8 首发；M=H 只拥有 E1…E3，L 为 E4…E6，R 为 E7，E8 为封 R 支持。M/H 在本例相等，不能因此抹掉它们与 L、R 的角色和来源区别；L 未曾吸入 H。 |
| None 与二次封腿 | 两 None 块拥有 E2…E5，过滤会丢事件且伪造相邻。仅将已确认端点送入普通标量封腿器，E8 只有四条封腿、无输出；含活动端值的代数骨架不能冒充逐事件只追加输入。 |
| own progress patch | 仅新增 Stage57 摘要，准确记 RawTerminal、未证 ER57、A57 与有限根分层、E1…E6/E8、None 禁拼接及两包普通组合；没有缩减两条 F₁、P1–P4 或授予原义资格。审查对象是冻结 patch，未执行该 patch 或改 Progress。 |

以上逐项对照两作者稿及其实际读回/比较正文，而非只读取 passed 标签。正文对原文对象与量词的摘要与已审源义比较相符；本工位未新增原文解释或重新认证历史作者身份。

## 同一 D57-main8 的普通组合证明

只读已冻结 `direct-evidence/controls-v1.json`、`results-v1.json` 中 `results[0].prefixes[n=8]` 的实际字段，并对照直接读回的 Main 表。固定身份 `D57-main8:K0`、其 seed F0/F1/F2、C=[100,102]、s=0；不以别处同值核心代换身份。

| i | 自有事件 | Iᵢ | endAt i | sealAt i |
|---:|---|---|---:|---:|
| 0 | E1 | [100,102] | 1 | 2 |
| 1 | E2 | [100,102] | 2 | 3 |
| 2 | E3 | [100,102] | 3 | 4 |
| 3=L | E4…E6 | [1,102] | 6 | 7 |
| 4=R | E7 | [1,99] | 7 | 8 |

F0…F2 的完整范围等于 C，F3 的完整范围包含 C，因此 i=0…3 每条都触 C；j=3≥s+2=2。r=4=j+1，sealAt 4=8≤τ=8，而 I₄.high=99<100=C.low，故 I₄∩C=∅。这四项按独立关系定义组装 RawTerminal，不使用 published、retired 或 Completed 来证明接触。

κ 满足 2<3<4<7<8，且逐项有 1<2、2<3、3<4、6<7、7<8。已审的有限已封前缀→总 Tape 普通解释因而可用于这五行；补齐仅是数学载体，不把活动 F5 改成已封，也不制造未来订单。原身份及来源由已审 A57 适配保留，泛型根本身不生成这些绑定。

设任意自然数 k>3。则 0≤4≤k。若 TouchSpan(C,0,k)，其索引 4 分量必须断言 I₄ 触 C，与 99<100 矛盾。因此对所有 k>3 均无同一 K/C/s 的连续 TouchSpan。此反证就是已审 T57 在同一 main8 上的应用，对保留到 R 的所有真实追加及其保守解释仍有效；未来再次触 C 不能使跨过 F4 的整段恢复逐腿全触。

这个普通组合给固定核心/起点连续片的最大性；不是第二份市场控制、不是新机器根，也不是全体订单历史必有终点。关系阈值为 8，L.end=6<8；实际 E8 发布只是相容的数值读数，没有被当作原义完成的前提。取得新核、换起点、换分解或合法终点落在 R 内的原义延伸仍须 ER57，不授 P/B56/F₂。

## 机器状态与失败记录

| 路线 | 作者 exact 运行 | 独立读回运行 | 后续普通比较 |
|---|---|---|---|
| 最大性 | `9a4cb8befb1d4700bc4444f50374681e`，exact_root_passed=true | `ead2b4bb330b49638ca610f113927c1d`，declaration_closed_uncompared，exact_root_passed=false | passed_scoped |
| 直接路线 | `517df05d332443609db0e65e8bb8cb5c`，exact_root_passed=true | `04e249c751b34b8fb5d35f02b72f49eb`，declaration_closed_uncompared，exact_root_passed=false | passed_scoped |

四份原 manifest 的机器语义状态均为 not_reviewed；后两份比较另文完成，不回写旧 manifest。泛型读回 wrapper 与五项 Lean 子命令均 0；直接读回 wrapper=1 是 strict-exit 缺少预期合同比较，16 项本地模块编译与独立打印成功，不是 Lean 失败。直接路线仍有 7 个 Origin olean 复用，未建立全量源到产物重新构建对应。两根的公理为 propext/Quot.sound；primitive_strict 的 Classical.choice 不能并入有限根。

Real 回执只有 exit=null、SIGTERM、ETIMEDOUT；240 秒为作者记录的上限，无实际起止时刻，不认证精确持续时间。Real 原源及旧锁保留、泛型合同另名；没有具体 Real 实例编译通过结论。作者稿的“240秒超时”由 Scope 第5项明确收窄到这项证据强度，主汇总和 JSON 没有扩张。

总证据 `integration_review.status_at_freeze=PENDING` 是冻结时事实，保持不改。本次通过由本报告与新增仓外回执表达，不把它伪装成冻结之前已完成。

本次只执行字节/SHA 核验、JSON 只读抽取和上述手算；未运行 Lean、JS 数学检查器、旧测试、枚举或市场任务。只写本报告及 `stage57/integration-review/` 仓外证据，未改冻结包、Progress、formal、教义、生产或 main，未提交外发。读取并遵循 math-research-workflow、rigorous-open-math-research 及 v2-verification-loop；本报告与回执是可复核文件绑定，不是平台签名或整套研究目标完成认证。
