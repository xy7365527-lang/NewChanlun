# Stage69 固定学习器工程工作卡

2026-10-06，#1467（备料关联 #1468）；作者稿，独评尚未发生。所有输出仅在本目录；不改变旧冻结、仓库、生产、formal、tracker、原 goal、P1–P4、R_W 或 R_D。已读取研究工作树的 rigorous-open-math-research 技能。先冻结本卡，后运行。

## 对象与预算

复用固定 BTC-USD 一分钟、A/B 各572行、label profile `published-mid-first-change/2` 和 Stage56 run-v4 普通三切分。三个原始产品 SHA 与 Stage56 evaluation SHA 已按当前字节核对，见 input-hashes-before.json。完整分钟此前已看过；middle/late 都只是复现诊断区间，不是未见确认集。

唯一模型族是带截距的三类 softmax 线性分类器，类别顺序 `[up,down,none]`。零初始化，恰200次确定性全批梯度更新；学习率0.05，L2=0.001，目标为平均交叉熵加 L2/2 乘非截距权重平方和。梯度在一次迭代中累积完再统一更新，无随机数、早停、搜索、类权重或调参。A/B 算法、行资格、迭代数及学习率相同，参数维数不同。

A只接五字段：last_trade_price、seen_trade_count、since_last_trade_ns、observed_trade_count_1s、observed_trade_quantity_1s。B只在此基础上加 bid_price、ask_price、bid_quantity、ask_quantity、spread、queue_imbalance、published_l1_changes_1s、published_mid2_delta_1s。B是13字段有限压缩，并非强B或完整订单历史模型。不接递归 C、B_seed、订单序号或整包history。

最多2个实证拟合模型，每臂一份；每模型169个历史有标签训练行、400个后段决策。控制仅复用同一模型或比较其训练载荷/纯函数投影，不另拟合市场模型。最多572个共同决策，输出总额≤128MiB；进程各使用 --max-old-space-size=64，实际 RSS另记（此选项是V8堆限制，不冒称OS内存隔离）。无新增数据、依赖下载、费用、执行、交易或收益建模。

## 转换与预处理

原精确价量字符串不修改。时间资格/身份/截止均用BigInt比较。数值转换仅在学习器内执行：价、量、spread除1e8；since_last_trade_ns除1e9为秒；mid2_delta除2e8为美元中间价变化；计数除1；不平衡为numerator/denominator（正分母）。整数必须十进制字符串；先解析BigInt，再转Number，拒绝非有限数；浮点近似仅用于模型，不反写精确数据。

每字段只从符合训练资格的169行的非空值计算均值；全空时均值0。缺失填该训练均值，再在全部训练行上算总体方差（分母n）；标准差0时取1。每字段总是另留一个原始缺失指示（0/1，不标准化），因此 A 输入维数10、权重33，B输入维数26、权重81（含3截距）；全空或常量字段不删。拒绝转换、变换、logit、梯度和概率的非有限结果。稳定softmax减最大logit；预测类别平局按固定类别顺序首项。缺类保持三输出头，零初始化同样训练，不补假行或假none。

## 训练与逐行协议

唯一逻辑拟合截止为1609459220000000000（20s）。可信宿主从 Stage56 evaluation-only 选 decision<cutoff、observed label、label_known_ns<cutoff 的行；明确验证known严格晚于resolved，known==cutoff拒绝。既有3不可用行保留身份但不得作为none训练。

学习器独立持久stdin进程。拟合载荷只含版本、臂、fit_cutoff_ns和有资格行（product_id、decision_ns、该臂features、历史label）；不送全标签文件、路径、评价mask/分区、label_known、全文件长度/摘要或未来水位。宿主资格证明单独保存。模型侧再次检查decision<cutoff；可信宿主承担标签可知性。这个协议是具名可信代码边界，不是OS沙盒或敌意程序防护。

模型先实际fit并回执，之后每次仅送一个共同决策的身份及当时有限特征，等待对应回应后才送下一行。20s之前172行记not-yet-fitted，不回填已训练模型。20–40s和40–60s各200行都要有A/B预测或明确失败；不按缺失或事后loss删行。模型状态和预处理拟合后冻结，无在线更新/重置。历史缺口由既有特征的null与缺失指示表达；本轮没有额外warm-up删除或标签驱动重置。逻辑回放时点与现实计算wall time分别记录，不能声称20s真实部署已完成或零生产延迟。

## 验证与交付

严校白名单：A拒B字段和任何seq/history；推理拒label/partition/mask等额外字段；训练拒未来decision、路径及资格字段。控制包括拒绝输入例、缺失行与概率合法性、已拟合模型全部缺失推理、纯预处理全空/常量和缺类梯度控制、known==cutoff拒绝、未来特征/标签扰动不改训练载荷及模型、持续前缀延长不改已发预测、模型/输入/输出字节hash、真实子进程PID/命令/时间/回执。控制变换是同一保存样本上的非干涉检查，不称新历史或独立样本。

评价标签只在全部对应预测完成后由host接回，列 accuracy、mean logloss、multiclass Brier（各类别平方误差之和再按行平均），概率logloss取max(p,1e-15)仅为诊断数值保护。不算或解释B−A信息收益、显著性、泛化、净收益或C−B_seed。重叠label witness不是独立样本。

保存模型、每行预测、训练资格与预处理证据、实际允许载荷与拒绝例、进程日志、输入hash前后、manifest、FINAL。运行前source hash封存，若修复实现保留失败收据并说明；不能看诊断改模型设置。下一阶段才独评。
