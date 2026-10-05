# Stage69 实际 A/B 学习器独立审查

审查日期：2026-10-06；归属：#1467。结论：**接受本次有限特征、固定模型的离线工程接入和复现诊断；未发现阻塞这一限定结论的实质缺陷。** 未取得强 B、全历史合法后缀非干涉、未见确认、统计显著性、因果信息增益、交易收益或生产时延结论。

本审查为新上下文独评。仅读取指定冻结作者材料及其五项当前输入，不读 MEMORY、ResearchProgress、作者会话或其他勘察结论。使用研究工作树的 [rigorous-open-math-research 技能](/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun/.agents/skills/rigorous-open-math-research/SKILL.md)。所有新增文件在本目录，没有改作者材料、仓库、正本、生产、formal 或 tracker。

## 冻结与身份

作者根 manifest SHA 为 `903a7e367e4e55c16ecb6709b17e41316b5edf237308eb2eb2ea3fc237b4ed7c`；FINAL SHA 为 `2b304dcf42f295507b5c26918a520af592d522341ce23b01146ef3172fefe15a`，均与审查输入指定值逐字相符。验证了作者根 manifest、run-v1 manifest、source-freeze、WorkCard pin；全部作者文件和五项源输入在审查前后保持相同大小和 SHA，详见 `input-hashes-before.json` / `input-hashes-after.json`。

A/B/label/evaluation 各572行，逐行共同身份一致，BTC-USD，网格间隔100ms，范围为逻辑样本2.8s–59.9s。A profile 为 `trade-history-grid-a/1`，B 为 `book-history-grid-b/1`，标签为 `published-mid-first-change/2`。B 的前五字段与 A 每行完全一致。Stage56 receipt 指定的 evaluation SHA 与当前文件一致；partition 边界20s、40s一致。

独立检查**把实际成功 fit 载荷逐字段与当前 A/B 源文件和 evaluation 重新投影对照**，没有以载荷自洽或 host 资格文件自洽代替源身份。400个主推理输入也逐条对照当前源行；800个评价 join 对照当前 evaluation 和主预测。

## 资格与因果边界

`run-author.mjs:29–39` 用 BigInt 检查共同身份、100ms网格、`known > resolved >= decision`，并选择 `decision < 1609459220000000000`、已观测有效类别、`known < cutoff` 的训练行。独立实现使用 Python 任意精度整数复核，恰为源索引3–171，共169行：127 up、42 down、0 none。前三行 `ineligible_at_decision` 的 label/known 保持 null，未冒充 none。训练索引不含任何后段行。

纳秒 `cutoff−1 / cutoff / cutoff+1` 独立资格控制分别为真/假/假；`decision==cutoff` 与不可用标签拒绝。作者四项资格控制只是宿主纯函数控制，不是子进程对标签可知性的验证。当前子进程只复核 decision 时界，不收到 label_known；**标签可知性由具名可信 host 承担**。继承 Stage56 已审封桶时钟语义，本次不重新恢复原始20,868条消息，也不声称重新认证那些上游导出证明。

成功 fit 的顶层仅为 op/version/arm/cutoff/rows，行仅有身份、该臂 features、历史 label。成功 infer 仅含 op/version/身份/features。A 恰五字段，B 恰十三字段；成功载荷没有 B 越入 A、订单序号、历史包、路径、文件 hash、评价 mask、partition、known、future watermark 或后段 label。模型存有 training-request SHA 作为审计元数据，但它不是全文件摘要，不参与特征或 logits。

读取、计算和权限须区分：host 已读完整离线文件及标签，用于资格控制和最终评价；已运行学习器代码只消费限定 stdin，core 仅导入 crypto，stdin 仅导入 readline/core，没有数据文件读取或 evaluator 依赖。子进程仍拥有宿主文件权限；清理环境变量、白名单和不同 PID 都不是 OS 沙盒，不能防恶意替换代码越界读取。

## 独立训练与数值结果

`independent_audit.py` 使用 Python 标准库重新构造单位转换、train-only 预处理、三类 softmax 和200步全批梯度。没有导入或调用作者 fit/predict/gradient 当 expected；作者 saved-evidence 检查也没有被当成 oracle。只各复算 A/B 一次，共2次独立拟合，没有新候选、搜索、调参或新增市场行。

复算目标为平均三类交叉熵加 `0.001/2 × 非截距权重平方和`；每步先求完梯度再用0.05统一更新，零初始化，截距不受惩罚。单位为价/量/spread 除1e8、since_trade 除1e9、mid2_delta 除2e8、计数不缩放、不平衡为分子/正分母。训练可见非空值决定均值，均值插补后以全部169行作总体方差分母；零标准差改1，每字段再附一个未经标准化的缺失指示。由此 A/B 参数数为33/81。

训练缺失数与作者一致：A 两个1s滚动字段各7缺失；B 同样保留这7行，并且两个发布L1滚动字段各10缺失。全空/常量预处理的均值、scale 与维数规则独立复核。none 在训练中缺类，但始终保留第三输出头；后段 none 为合法 observed_no_change（middle 3行，late 24行），与前三行初始化不可用不同。

| 比较 | A | B |
|---|---:|---:|
| 权重最大绝对差 | 1.97296e−12 | 1.23000e−11 |
| 复算模型对1002次推理中本臂501次概率的最大绝对差 | 2.59603e−12 | 2.62569e−11 |
| 从保存权重独立重算概率的最大绝对差 | 2.22045e−16 | 2.22045e−16 |
| 预测类别不一致数 | 0 | 0 |

预先写入脚本的权重/概率/指标绝对容差为1e−9，预处理采用1e−11乘 `max(1, |a|, |b|)` 的相对尺度。Python 采用 `math.fsum`、梯度先按列求和再除 n；作者 JS 左到右累计，并逐行除 n。均值差约1e−11美元，在约2.9万美元的价格上属浮点求和差异，并会经标准化和梯度传播。因此这里接受数值复现，**不声明位级相等**；最大指标差约5.8e−11，远小于1e−9，所有类别稳定。

同用 JS JSON.stringify 作为纯序列化 codec 后，原模型内容 SHA 能准确复现，但独立重算模型内容 SHA 不同。A 独立 SHA=`8367af16e701822db9cddb34c7c20100a3550b7cc7c104db689019a3ba00b565`；B=`abcc4499b820a1731e12a7fa5a8a1e55ebb110cde91da6f7d1d35da7b50e24e0`。内容哈希与 pretty-printed 模型文件哈希也分开。模型、400个主预测/臂和逐项差值保存在本目录。

| 臂 | 已探索区间 | accuracy | 独立 mean logloss | 独立 multiclass Brier |
|---|---|---:|---:|---:|
| A | 20–40s | 0.670 | 1.116496663627 | 0.584533470639 |
| A | 40–60s | 0.675 | 1.727737235770 | 0.627623913477 |
| B | 20–40s | 0.470 | 1.396322165750 | 0.771154528704 |
| B | 40–60s | 0.205 | 3.262462917445 | 1.302707495675 |

Brier 定义为每行三个类别平方误差之和再求均值，未除以类别数；logloss 的1e−15下限仅是诊断数值保护。每臂后两段各200行，全部产生预测并进入评价；没有按缺失或事后 loss 删除行。

## 进程、时钟与控制究竟证明什么

存档 A/B 分别521/520个请求，PID91746/91747，exit0；每臂恰1次成功fit、400次主预测、100次前缀重问、1次全空推理、1次inspect。A/B 有18/17个拒绝回执，合计35，其中33个在具名拒绝例文件、另2个是未拟合预测。逐行检查 stdin/stdout 原字节SHA、输入与输出配对SHA、PID、stderr operation、事件交替及wall时间。host `ask` 的 pending约束和逐次 await 源码，与“先收到响应，再发下一行”的全部日志一致。

训练后模型只读；400次主预测后仍保持各172个 `not-yet-fitted` 记录，没有回填早段。这172条是逻辑回放状态，不能解释成2021年真实在线进程当时的观测。最终inspect模型、预处理及内容hash与fit回执相同。A完成退出后才启动B，最终评价join发生在两臂全部主预测和控制结束之后。

| 控制 | 本次支持 | 不支持 |
|---|---|---|
| 33+2进程拒绝 | 当前列举字段/时界/未fit/重复fit输入被拒；成功载荷保持白名单 | 任意恶意输入的全覆盖或权限隔离 |
| 4项known资格控制 | host函数在cutoff的严格比较与不可用处理 | learner自行证明label_known或重新证明上游封桶钟 |
| 改400条保存后段特征/标签 | 独立确认变换确实没有改169条train字段，重新投影fit载荷逐字相同 | 重建任意合法原消息后缀、完整历史未来独立性、新市场样本 |
| 100→400后再问最初100条 | 同一已fit模型、相同输入的输出字节相同；冻结代码路径当前不更新状态 | 重启训练、任意历史扩展、另一个未来上的普遍实验结论 |
| 每臂1次全空infer | 当前已fit状态的全空接口输出合法概率，模型未改 | 该模型曾用全空训练数据拟合 |
| synthetic缺类单步梯度 | 缺类头仍参与梯度，没有伪造none训练行 | 三类都有训练覆盖或none的可靠泛化 |

审查另用真实 `learner-stdin.mjs` 启动两进程，在无fit状态下完成16个负例：未拟合预测、decision等于cutoff、晚1ns、禁止known/path/feature、浮点形式纳秒字符串、未fit inspect。**0次成功fit、0条市场预测**，不占用额外模型。该控制没有声称重新执行已fit拒绝矩阵；后者依赖已核冻结日志和源码。

作者实际wall时间为2026-10-05 22:10:41 UTC，逻辑样本时间为2021-01-01。fit日志compute_ns分别27,869,750与45,571,667；计时区间还包括请求解析和输出操作，不是纯优化内核benchmark。这是本次离线代码路径耗时，不能当成生产计算延迟、数据到达延迟或在逻辑20s已就绪的证据。

作者child逐请求RSS采样最大值A59,588,608、B59,293,696字节；host采样最大值68,583,424字节只有汇总值及其采样源码，没有独立逐点host轨迹。两种采样最大值相加128,172,032字节（约122.2MiB）既不是同时驻留量的实测峰值，也不是绝对峰值上界。`--max-old-space-size=64` 只限V8 old space，没有OS RSS硬限。本审查只确认交付输出远小于128MiB，不据采样声称全运行内存硬上限已证。

## 实质缺陷、审查自纠与接受边界

**作者范围内无已确认的阻塞缺陷。** 作者已把全部检查正确标为开发证据，Stage69 本轮由此独评补足；saved-evidence 脚本自身没有独立重算200步模型，也没有逐字段对照当前源训练数据，这两个缺口在本审查中补上，不能倒称作者原脚本就已覆盖。

审查期间一项辅助计数被纠正：初始集合统计把 event_group=null 当成一个组，导致报告 middle/late“交集1”。检查具体值后确认交集只是null；现结果排除null，非空 witness 组数early89、middle108、late112，三段非空组交集均为0。初始结果和更正收据分别保留为 `independent-results-before-null-group-correction.json` / `null-group-count-correction.json`；没有重新fit，模型、概率和指标字节未改。组数也不是有效独立样本量：同段仍有多个决策共享witness，且不同witness还可能受连续市场状态相关。

因此接受的是：同一已探索分钟中、可信host资格筛选下，A五字段/B十三字段接入真实固定学习器的离线工程证据，以及其数值复现。1002次成功推理是800次配对主推理加200次重复和2次接口控制；400个主决策在两臂上配对，不能当800个独立样本。全分钟此前已探索，20–40/40–60均不是未见确认区间。B有更多参数，固定相同步数不等于公平强B或统一容量预算。没有B_seed/C，没有费用或收益模型；未计算p值、显著性或B−A因果信息增益，也不因B诊断较差更换设置。

下一步若要求更强结论，须另提供相应设计与授权；本次无需生产风格改造，未扩展模型族或数据集。

## 可复核交付

主要结果：`independent-results.json`、`supplementary-results.json`、`boundary-controls.json`；独立源码：`independent_audit.py`、`process_boundary_controls.py`、`supplementary_checks.py`；每臂独立模型与400条预测；实际命令见 `COMMANDS.md`。训练逻辑只执行一次/臂；后续修改仅为辅助null组计数和禁止pycache写出，不曾据指标调参。最终输入校验仍保持全部冻结来源不变。
