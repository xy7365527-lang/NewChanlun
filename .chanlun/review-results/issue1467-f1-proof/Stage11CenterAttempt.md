# Stage11：固定 seed 首对证据的因果性、迟确认与不回填

日期：2026-10-03，#1467，命题 `DC-CENTER-FIRST-PAIR-v1`。承接 [开工卡](Stage11WorkCard.md)。名分：研究中的固定边界配对关系与定义内 L0 证明，不是完整中枢生命周期、完整 F₂ 或可交易信号。

## 结果

本轮为上层组装补出一个可复核环节：**先从 DC 完整关系取截至 t 已确认的单元及其端价，再对固定三段 seed 后的首对离开/回抽取证。**

一般定理证明，在正δ、旧 `DCSpec.Whole` 合法域内，两条历史若截至 t 的观察相同，所产生的可见对象列表完全相同；任意固定起点的首对判定因而一致。两个合法正量见证进一步区分：回抽端点发生、回抽被确认、以及后来出现的另一个合格配对。这三者没有被混为一件事。

本轮仍没有完整组装关系。延伸后 seed 右端怎样改变、九段重切怎样影响成员、完整走势何时完成，都是开放义务；不会把配对通过直接变成上层 completed。

## 关系与来源

`visible(p,r,t)`保留所有 `knownAt≤t` 的旧 DC 单元，并将每个单元与其真实端价组成列表。载荷包括源索引、方向、端点发生索引和确认索引；没有只留下价格包络。

对固定起点 start，`firstPairUp`只检查 `visible.drop start` 的前五个成员：三个 seed 成员 `a,b,c`，以及紧随的 `leave,ret`。关系要求：

- a/b/c 满足实际 Origin 局部中枢谓词；
- 五个对象的源端点连续接续；
- leave 向上、ret 向下；leave 端价高于 seed 的 ZG，ret 的整个区间低点也严格高于 ZG；
- 首对由列表中的立即后继位置决定，没有从后面搜索一个成功配对来替换它。

这些检查没有赋予 DC 单元教义上的完整次级走势身份。本关系也只对**由 visible 生成的对象**取得旧 DC 来源资格；人工拼出的裸载荷列表不能自动取得该资格。

规则依据已回查：第18课64行讲离开后回抽不回的中枢破坏；第20课60行讲三类配对、62行明写“必须是第一次”；第43课18行讲背驰导致原走势类型终止。三处均位于正文界内，所援引部分没有注者标记；第20课62行末另有娇注，本轮不引用该注。对应正本是中枢 S-3 与买卖点 #816 B-3。**中枢破坏和完整走势终止仍属不同对象层次。**

严格端点沿用 `formal/Origin/BspClassification.lean` 的 `IsType3Buy`，没有另设宽判据。`type3_bridge`把实际源连接、方向、离开及回抽事实转成该现有谓词的前件；`leftCenter/firstRetrace`由配对计算，不是调用者任填 true。桥中的第一/二类辅助字段只用于构造既有载体，不作为背驰、趋势或其他买卖点证据。

这里的“首对”限定于**冻结三段 seed 的右端**。若后续扫描改变该对象的成员和右边界，必须定义新的边界版本及证据关系；本结果不能当作某个动态中枢永久终结的证书。完整扫描中，失败配对是否吸收、何时重定边界，尚未在本轮裁定。

## 一般前缀因果证明

旧 `DCSpec.dc_causal`已经保证相同前缀下，已确认单元列表相同。新增证明补上价格载荷的读入边界：

`start < finish < knownAt ≤ t`。

该不等式来自 `dc_geometry`。所以新视图读取的每个端价都位于共享前缀内，不能把相同的来源索引悄悄配上未来价格。映射后的完整列表相等，进而任何固定 start 的 `firstPairUp` 命题等价。

这是定义内的后缀无关性，尚未验证 Rust 在线实现、事件到观察时钟映射或发布延迟。knownAt 的单位仍是输入观察序号；接到 Stage9 行情后，必须继续传播实际采集可见时间，不能将此序号直接解释成交易所或本机时钟。

## 合法分叉见证

固定 bid=10000、ask=10200、卖量10000，以旧精确投影公式生成正买量。δ=2，观察路径：

```text
A: 10040,10020,10032,10024,10050,10042,10046
B: 10040,10020,10032,10024,10050,10042,10030,10034,10060,10045,10055
```

同一冻结根证明两条路径满足完整旧 DC 关系，并逐点验证正数量及精确投影。不是直接手写一组假定有效的段。两者都先产生 seed `[10024,10032]`，前四个已确认单元相同。

| 截止观察 | A 的首对证据 | B 的首对证据 |
|---|---|---|
| 5 | 仅四个已确认单元；回抽仍活动，无首对证据 | 与 A 的完整可见列表相同 |
| 6 | 回抽 `4→5` 才确认；低点10042高于ZG10032，首对通过 | 回抽继续到10030，仍未确认 |
| 7 | 本例输入已结束，不外推 | 回抽 `4→6` 确认，低点10030进入核心，首对失败 |
| 10 | 本例输入已结束，不外推 | 后来离开到10060、回抽到10045，虽均在原ZG之上，固定 seed 的首对仍失败 |

观察5的10042，在 A 中成为回抽终点，在 B 中只是回抽途中的价格。两历史此时完全相同，不能把 A 的后来确认回标为观察5已经确认。这里不否定统计预测，只限制结构证书何时成立。

B 的后续配对是另一个对象，不能替首对改写结果。反过来，这也没有证明该配对不可能归属于未来另一个中枢或另一个边界版本；其归属须由完整生命周期处理。

## 证据与复现

根：`CenterAttempt.exact_root`。精确类型：

`VIEW_CAUSAL_TARGET ∧ TYPE3_BRIDGE_TARGET ∧ WITNESS_TARGET`。

三个部分分别覆盖：一般合法前缀的完整可见视图与首对命题一致性；固定首对向实际 Origin 三类买点谓词的条件桥；两份完整 DC/正量见证、迟确认和不回填。

Lean4.31 精确检查 exit0，run `ed3a72fe701c42088f6cecce4966106f`；machine/exact_root 均通过，传递假定仅 `propext/Classical.choice/Quot.sound`，没有 sorryAx、自定假定或不安全依赖。声明文件在证明前冻结，哈希检查一致，未改变验收。

摘要：[stage11-lean-verification-summary.json](stage11-lean-verification-summary.json)。原始命令、日志与完整导入清单：`/Users/silencehan/Documents/Codex/research-evidence/issue1467/center-attempt-v1/`。独立读回/比较包为 pending-review，未派发；`semantic.status=not_reviewed`，作者核对不记作独评通过。

复现需要已有研究工具与 Origin 编译模块；本轮仅补构建 `Origin.BspClassification` 及其必要依赖，未改 formal 源。工作树根目录执行：

```sh
LEAN_PATH="$PWD/formal/.lake/build/lib/lean" python3 \
  .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project "$PWD/.chanlun/review-results/issue1467-f1-proof" \
  --target-file CenterAttemptProof.lean --declaration CenterAttempt.exact_root \
  --expected-type 'CenterAttempt.VIEW_CAUSAL_TARGET ∧ CenterAttempt.TYPE3_BRIDGE_TARGET ∧ CenterAttempt.WITNESS_TARGET' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/center-attempt-reproduction-2
```

使用新的输出目录。源/原文绑定见 [source-sha256-stage11.json](source-sha256-stage11.json)，声明锁见 [center-attempt-spec-v1.sha256](center-attempt-spec-v1.sha256)。没有重跑旧枚举、下载市场数据、启动确认实验或改生产；formal 源无修改，不需重生 fixture。

## 下一项

本轮把“成员已经可知”和“固定首对证据成立”分开并建立桥，但完成条件的缺口还在。下一步应明确一个携带边界版本的状态转换：失败配对怎样吸收为延伸、成功配对怎样划分原成员/连接/后继对象、九段重切如何保留 NoCore 片段。若这些分支改变 seed 右端，需在同一关系里重算 first-pair 归属，不能沿用旧布尔结果。

上行首对是本轮具名实例；一般可见视图定理没有方向限制，下行实例尚未单独桥接。完整 F₂、两条 F₁ 路线的全部资格、独立语义审查和四臂增量实验均保持开放。
