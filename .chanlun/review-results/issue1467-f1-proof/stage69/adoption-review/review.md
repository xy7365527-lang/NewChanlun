# Stage69 冻结 v1 文档采用复审

2026-10-06，归属研究票 #1467 与实验票 #1468。结论为“须修订两处采用文字”。两份作者结果及独评的有限接受结论可以保留；主摘要的实质表述准确。进度补充的 S1 出边表述和交付说明的时钟责任表述必须修订，再对文字差异复核。此结论只管文档采用，不判定研究总目标完成。

## 范围与输入身份

本次实际读回 `adoption-inputs-v1.json` 指定的七项输入，七项 SHA256 全部匹配。输入清单 SHA256 为 `9fb81b5a3e8b6ed6a69eb4dc99f4491fae42e9cb8def2fc3dc2113953ffcbfc1`。审查结束再读七项，字节未变，见 `inputs-before-sha256.json` 和 `inputs-after-sha256.json`。

审查完整阅读两份作者文字与两份独评，并核对所需原始 JSON 结果、运行回执、stdin/stdout 和预测存档。29 项补充原物的指纹、清单匹配及只读计数见 `originals-audit.json`。四份仓内作者/独评文字与外部档案同名原物 SHA 相同。本轮不运行窗口算法、训练、预测、R_W 或已有独评程序，新增这些运行次数均为零。

本次先按会话要求检索过记忆登记，仅用于识别既有研究范围；所有 Stage69 判断来自本次读取的冻结输入与原物。未使用旧记忆判断当前结果，也未把本次文字复审称为第三次独立数学或训练复算。

下文“主摘要”指 `Stage69LongerHistoryAndActualLearner.md`，“交付说明”指 `stage69/DeliveryStatus-v1.md`，“进度补充”指外部证据目录的 `progress-addition-v1.md`。行号均对应此次冻结 v1。研究档案根为 `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/`；完整外部档案根为 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/`。

## 必须修订

### R1：把五条边与固定 S1 的关系写成无出边

定位：[进度补充第 1 行](/Users/silencehan/Documents/Codex/research-evidence/issue1467/stage69/progress-addition-v1.md:1) 的“5边均离开固定S1，根非空0”。在路径叙述里，“边离开 S1”会被读成边由 S1 发出，与后半句和实际边表矛盾。

依据：[数学独评第 47–57 行](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/stage69/joint-review/review.md:47) 及原物 `joint-review/run/result.json` 的 `edges`，五条边是 4→9、9→18、13→18、18→27、22→27，没有 S1 出边；原物 `rooted_nonempty=0`。主摘要第 11 行已准确表达这一关系。

最小修订为“5条边的起点均非S1，固定S1的非空根路径为0”。若同步提高进度句的可读性，可以紧接“内部/固定根/原义获证三元组分别为1/0/0”，但这不是改变数学结果或新增证明。

### R2：明确 label_known 筛选由 host 单独承担

定位：[交付说明第 11 行](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/stage69/DeliveryStatus-v1.md:11) 的“全部时钟筛选由可信host与具名学习器共同承担”。这句话没有保留两种时界的责任区别，容易把标签可知性误读成学习器共同验证。

依据：[学习器独评第 17–23 行](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.chanlun/review-results/issue1467-f1-proof/stage69/learning-review/REVIEW.md:17) 与第 62 行。当前子进程只复核 decision 时界，label_known 由可信 host 筛选；成功 fit 不接收 label_known。实际原物 A/B stdin 与 stdout 配对后，每臂唯一成功 fit 的行字段均只有 `product_id`、`decision_ns`、`features`、`label`。带 `label_known_ns` 的负例被拒，不能算学习器取得该字段后验证。主摘要第 39 行已经准确保留这一区别。

最小修订为“decision时界由host筛选并由学习器复核；label_known资格仅由可信host筛选，学习器不接收该字段，也不独立证明上游标签时钟。没有OS权限沙盒。”

## 可以接受的采用内容

| 核查项 | 此次采用位置与判断 | 作者、独评及原物依据 |
|---|---|---|
| 内部/固定根/原义三元组为 1/0/0 | 主摘要 25–29 行、交付说明 7 行准确。内部数值链未升格为完整初始分解；原义的零保持“未获证”，未变成不可能性定理。 | 数学独评 86–90 行；`joint-review/FINAL.json` 与 `run/result.json` 分列三类计数；`joint-author/internal-chain.json` 保留 E22…81 缺口、原义完成 unknown。 |
| 固定根和局部证齐 | 主摘要 19–27 行保留 S1、state21、链首 state81、Own 区间及 198/378/558 局部证齐。P 的技术 Up 没有改为概念 U；没有移根追认。 | 数学独评 49–59、86–90 行；作者内部链各因子的 `candidate_known`、`conceptual_direction`、`original_completed`。 |
| H2 不是 Stage68 追加后缀 | 主摘要 15 行明确保留 443 个重叠观察中 442 个价格不同，拒绝长度单变量归因。较短交付/进度句没有另称“追加后缀”。 | 数学独评 33 行；`joint-review/run/history-comparison.json` 的 `prefix_extension=false`；作者 `stage68-comparison.json` 同口径。 |
| label_known 与权限边界 | 主摘要 37–39 行、进度补充“可信host标签时钟”可接受。交付说明按 R2 修订。无 OS 沙盒、学习器仍有宿主权限的限定保留。 | 学习器独评 17–23、62 行；两臂成功 fit 载荷逐项读取结果。 |
| 浮点复算与模型 SHA | 主摘要 43 行、交付说明 9 行明确容差内复现不等于位级相同；进度补充“浮点容差”不声称 SHA 一致。A/B 概率最大差 2.60e−12/2.63e−11，低于 1e−9，类别相同。 | 学习器独评 33–42 行；`learning-review/independent-results.json` 的概率差及两臂 `byte_identical=false`。 |
| 172/400/800/1002 的计数 | 主摘要 37、45 行与交付说明 9 行合读，保持每臂 172 个未拟合决策、每臂 400 个主预测、400 个共同决策上的 800 次主推理，以及 200 次重问和 2 次全空控制，共 1002 次。未把它们称为 1002 个独立市场样本。 | 学习器独评 55–57、80 行；原始 A/B 预测各 572 行，172 `not-yet-fitted`、400 `predicted`；原始配对日志各 501 次成功 predict。 |
| 有限 B 与已探索后段 | 主摘要 33、35、54、60 行，交付说明 11 行和进度补充均保留 B13 非强 B、后段非未见确认。相同步数不等于相同容量；没有把 B 较差诊断解释成盘口信息无效。 | 学习器独评 3、29、80 行；作者 `run-v1/receipt.json` 参数数为 A33/B81，scope 明写同一已看分钟。 |
| RSS 与耗时 | 主摘要 56 行、交付说明 11 行只声称采样证据，未宣称全运行硬上限。fit 日志包含解析/输出，未当成生产延迟。 | 学习器独评 70–72 行；作者 receipt 采样之和为 128172032 字节，`not_os_enforced=true`。该数也不是同时驻留实测峰值。 |
| 控制与独立性的范围 | 主摘要 45–47 行没有把保存后段变更、同模型重复问答或进程负例改写成任意合法后缀非干涉。54 行保留 null 不计 witness 组及组数非独立样本量；作者自检没有冒称独评。 | 学习器独评 59–80 行；独评结果与 supplementary 的非空组交集均空；原始作者检查标 `independent_review=false`。 |
| 未证义务与后续研究 | 主摘要 29、60、62 行及交付说明 11 行保留 RootArm、初始外缘、一般完成、Owner/Next、NE、完整 F₁/F₂、强 B、B_seed/C、公平四臂及未见确认的缺口。局部接受没有被写成 P1–P4 或价值结论。 | 数学独评 5、94–114 行与学习器独评 3、80–82 行。 |

## 封存前置与本次检查的限度

已实际读回仓内 `stage69/copy-manifest-v1.json`。它存在，正确路径不是外部档案根。交付说明第 5 行对该清单的引用没有缺失问题。本次首次按外部路径查找未命中，随后按仓内相对路径核实并更正；不把这次路径误读列为作者缺陷。

主摘要第 64 行所链 `stage69-evidence.json` 在本次审查时尚不存在。采用复审的仓内副本也须在本报告封存后复制。主控已说明证据总表要在取得本报告 SHA 后生成；这是最终封存前置，不是两份独评结果的缺陷。最终采用前须补齐并验证该总表、复审副本与链接，绑定 H2 的三个源文件 SHA、四份作者/独评原物及本次修订后的文档身份。此处不把尚未形成的最终总表声明为已审。

R1、R2 修订应产生新的输入清单和差异回执；本 v1 报告及其七项输入身份继续保留。复核只需确认两处文字和封存引用，没有理由重跑已审数学窗口、训练或推理。对主摘要其余实质内容没有必须修订项。全程只在指定 `adoption-review/` 目录产出，没有修改被审文件、仓内文件、tracker、提交或推送。
