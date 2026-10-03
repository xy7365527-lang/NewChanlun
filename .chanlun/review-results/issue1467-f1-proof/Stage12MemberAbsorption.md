# Stage12：回入配对的成员吸收与九段分流

日期：2026-10-03，#1467，命题 `DC-CENTER-ABSORB-v1`。接续 [开工卡](Stage12WorkCard.md)及 Stage11。名分：候选构造状态的一个转换分支；数学证据为定义内 L0，不是完整生命周期或生产准入。

## 本轮推进

已定义一个可独立检查的吸收关系：一个普通构造帧在明确离开后回入核心时，将两条已确认单元纳入成员，保持原三段 seed 和核心，推进修订号，并按实际确认时间更新 knownAt。总成员不足九时仍为活动构造；达到九时只进入待重切。

该关系有存在唯一输出、成员序列守恒、核心不变和确认时间不提前的 Lean 证明。另有两份合法 DC 见证：一份区分旧冻结框与增长后的构造帧；另一份实际走完成员数 `3→5→7→9`，并证明九段后普通活动输出不存在。

这里的“唯一”只指**已冻结字段更新和分流关系的唯一输出**，不宣称 F₂ 语义唯一分解已证明。

## 旧冻结框与构造帧不能混用

本轮回查 [#905](https://github.com/xy7365527-lang/NewChanlun/issues/905) 与 [#487 的关票记录](https://github.com/xy7365527-lang/NewChanlun/issues/487#issuecomment-5106033064)：旧框的 leave/retest 必须紧邻其右边界，价格失败后不能向后搜索替代配对；同价位不代表同一时间框，也不允许远期配对认领旧 Owner。

本候选遵守这个边界。旧帧及其失败事实保留；追加真实成员后产生的是另一个构造版本，输入右端和下一配对都改变。**没有把旧框改名后重新放行同一失败配对，也没有定义交易 Owner 的迁移。**新构造版本是否最终对应一个可发布中枢身份，仍需完整生命周期及归属证明。

吸收的依据为中枢正本 S-3/§5.4：冲出又回入时，相关单元进入延伸与外缘，初始核心不变。这里把“怎样更新构造载荷”落实成具名候选；它不自动裁定所有边界更新、所有初态或所有配对失效原因。

## 关系的输入与输出

`Frame`包含三个原始 seed 单元、追加成员 `extra`、`revision` 和 `knownAt`。全部成员为 `seed ++ extra`；当前右端取真实最后成员。ZD/ZG始终从原三段 seed 计算，后续吸收不重算母核心。

`Returned(side, frame, leave, ret)`要求 seed 局部判据成立，源端点接续，离开/回抽方向配套，确实离开对应核心边界，而回抽区间重新接触核心。它不是“任何失败都允许吸收”：方向、连接或 seed 不合法的失败不能凭本关系进入分支。两侧用同一结构，side 作为输入参数；空扩展帧的上行判据已证明与 Stage11 的 `UpPair` 等价。

`Recorded`逐字段规定：

- 三个 seed 对象保持；追加成员为旧 extra 加上 `[leave,ret]`；
- 修订号加一；knownAt取旧状态和两输入确认时间的最大值。

`Absorb`还要求输入帧少于九成员，并独立规定输出类别：新成员数 `<9` 为 `active`，`≥9` 为 `retilePending`。两支互斥，不能在普通延伸失败后换一个较宽准入分支。

证明直接从字段契约和数量条件推出存在唯一性，没有将“等于某程序返回值”作为关系的定义。已证的守恒式为：

`前导 ++ 新成员 ++ 后继 = 前导 ++ 旧成员 ++ [leave,ret] ++ 后继`。

所以在输入来源序列本来无重复的前提下，此更新不会新增重复或丢失成员。关系本身不凭空证明任意人工载荷来自市场；下面两份见证另用旧 `DCSpec.Whole` 及 `visible` 校验来源和时点。后续完整驱动器仍须携带这一来源资格。

## 见证B：增长不会复活旧框

复用 Stage11 已证明的路径 B，不重新构造或改写它的完整 DC 证明。seed为 `[10024,10032]`，初始构造版本有三成员、右端源索引3、knownAt=4。

第一配对上行离开到10050，回抽到10030，回入核心。回抽源段 `4→6` 在观察7确认，吸收后：

| 项目 | 旧构造帧 | 吸收后构造帧 |
|---|---|---|
| 成员数 | 3 | 5 |
| 修订号 | 0 | 1 |
| 当前右端 | 3 | 6 |
| knownAt | 4 | 7 |
| seed核心 | [10024,10032] | [10024,10032] |

Lean证明旧/新成员列表分别等于观察4/7的真实 `visible` 列表。

旧帧对原失败配对仍判失败。后续 `6→8→9` 的配对，也不符合旧帧的右端接续；它的价位虽合格，不能认领旧帧。对已吸收成员的新构造帧，它恰是立即后继，几何配对可成立。这是对**不同已知对象和不同配对**计算同一判据，不是将旧失败改判为成功。

该新配对结果仍不授权清算旧挂起、不证明新中心身份，也不宣布完整走势完成。

## 见证C：九成员时必须进入重切

固定 bid=10000、ask=10200、卖量10000，通过旧精确投影从正买量生成：

`10040,10020,10032,10024,10050,10030,10060,10028,10070,10026,10030`，δ=2。

同一根证明完整旧 DC 分解和全部十一点正量/精确投影。三个离开/回入配对依次把成员数从3推到5、7、9；确认时刻分别为6、8、10。三个更新后的成员列表都与相应截止时刻的真实 `visible` 列表逐项相等。

到九成员时，`retilePending` 是该关系唯一输出；Lean另外证明**不存在**满足该吸收关系的普通 `active` 输出。不是只检查某程序恰好选择了待重切。

待重切对象保留全部九个低级成员和原 seed 来源。它不产生上级 completed 对象，不把原核心冒充重切子核心。下一步仍须按 Z-4 对固定三段子窗重算，保留 NoCore 片段；Stage8 的非法子窗反例没有被绕过。

## 机器证据与复现

根 `CenterFrame.exact_root`，精确类型：

`UPDATE_TARGET ∧ B_TARGET ∧ NINE_TARGET`。

run `19b93c36720243229dc7fa9f9f7fa787`，Lean4.31 精确检查 exit0，machine/exact_root 均通过；依赖仅标准 `propext/Classical.choice/Quot.sound`，没有 sorryAx、自定假定或不安全依赖。声明锁在证明前建立且未改变，首轮精确检查通过。

摘要见 [stage12-lean-verification-summary.json](stage12-lean-verification-summary.json)，源绑定见 [source-sha256-stage12.json](source-sha256-stage12.json)。完整日志和导入清单位于 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/center-frame-v1/`。独立审查包已准备但未派发，`semantic.status=not_reviewed`。

工作树根目录复现，使用新输出目录：

```sh
LEAN_PATH="$PWD/formal/.lake/build/lib/lean" python3 \
  .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project "$PWD/.chanlun/review-results/issue1467-f1-proof" \
  --target-file CenterFrameProof.lean --declaration CenterFrame.exact_root \
  --expected-type 'CenterFrame.UPDATE_TARGET ∧ CenterFrame.B_TARGET ∧ CenterFrame.NINE_TARGET' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/center-frame-reproduction-2
```

本轮只写研究包，没有修改 formal/生产、下载新行情或运行确认实验。必要研究依赖随新根编译，不重复旧市场回放或枚举。

## 仍未完成

这个分支没有覆盖未离开、未确认配对、成功离开、初态边界选择等其他输入；也没有实现完整状态机、事件存储或崩溃恢复。吸收后来源相等不证明新构造版本获得教义或交易归属资格。

下一项可直接接续的是 `retilePending` 的固定三段重算与所有成员的去向：有核心与 NoCore 结果均须保存原级别来源，不能只输出成功子窗后丢掉其余成员，更不能直接把局部核心包装成已完成上级走势。完整 F₂、两条 F₁ 的全部资格、独立语义审查及增量价值保持开放。
