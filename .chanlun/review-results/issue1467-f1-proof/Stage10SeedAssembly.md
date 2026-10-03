# Stage10：四个已确认 DC 单元内必有局部中枢起点

日期：2026-10-03。研究票 #1467，命题 `DC-LOCAL-SEED-BOUND-v1`。承接 [v4 协议](GoalReframe-v4.md)与 [开工卡](Stage10WorkCard.md)。数学结果为定义内 L0 推论；订单基底和完整 F₂ 的资格继续待证。

## 新结果及其作用

对任意正阈值 δ、任意满足旧 `DCSpec.Whole` 的有限历史，以及其任意四个连续已确认单元 `a,b,c,d`：

**`(a,b,c)` 与 `(b,c,d)` 至少一个满足实际 `Origin.CenterConfirmedComplete`。在这两个窗口中，满足“自身成立且更早窗口不成立”的起点存在且唯一。**

这消去一个具体义务：在上述几何域里，局部中枢的初始查找不必无界等待；有四条已确认单元时，至多跳过一个三段起点。定理没有预先假设已有中枢，因而不是从非空前提重述非空。它接到已证明的 DC 全域关系和真实 Origin 三段判据，未借用 Rust 解析函数的输出唯一性。

**“四”是寻找三段核心所需单元数的上界，中枢判据仍然吃三段。**本结果不改九段升级的固定三段重切，不允许失败重切子窗转用滑窗放行。Stage8 的反例继续有效。

最早起点唯一也不表示两个窗口不能同时成立，更不表示整个走势分解唯一。成员覆盖、延伸与重切、连接的同级构成和上层完成条件仍须给出独立关系；这是组装的局部起点，尚不是完整组装算法。

## 论证

四条首尾相连、严格交替的上/下/上/下单元，对应五个端点：

`x₀ < x₁ > x₂ < x₃ > x₄`。

前三条的核心边界为 `[max(x₀,x₂), min(x₁,x₃)]`，成立要求下沿严格小于上沿。若它为空或退化为单点，由 `x₂<x₁`、`x₂<x₃` 和 `x₀<x₁` 可推出 `x₃≤x₀<x₁`。于是后三条核心化为：

`[max(x₂,x₄), x₃]`。

其两个候选下界 `x₂,x₄` 都严格小于 `x₃`，故严格非空。下/上/下/上的情形对偶成立，包括前一个核心恰为单点的边界。

`DCSpec.dc_geometry`提供正 δ 下的严格幅度、相邻共享端点与方向翻转。`dc_four_core`读取这些已证性质，取得上述几何析取，再调用旧 `DCOriginBridge.dc_three_center`接入 Origin 的方向交替与严格三段交集谓词。没有对四条单元增加“共同覆盖一个母核心”的假设。

`FirstCore`独立定义为当前窗口成立、所有更早窗口不成立。存在性来自上述析取；若两个不同起点都最早，较后的最早性与较早者成立矛盾。该证明只覆盖两个局部候选窗口的规范选取，不借此宣称全塔唯一。

## 上界不能降到三：正数量订单观察见证

固定 bid=10000、ask=10200、卖量=10000。正买量依次为：

`1299,1429,1111,1236,1050,1173`。

按既有精确数量加权及半值向上量化，得到：

`10023,10025,10020,10022,10019,10021`。

δ=1。Lean在原 `DCSpec.Whole` 关系下证明整个六观察分解：四条已确认单元 `0→1,1→2,2→3,3→4`，确认序号分别为 `2,3,4,5`，以及活动尾部 `4→5`。同一个根还验证全部六个数量为正、投影值精确相等。

| 窗口 | 三个区间 | 核心与结果 |
|---|---|---|
| a,b,c | [10023,10025]、[10020,10025]、[10020,10022] | 下沿10023 > 上沿10022，不成立 |
| b,c,d | [10020,10025]、[10020,10022]、[10019,10022] | [10020,10022]，严格成立 |

这里确实存在三个连续已确认单元却没有严格核心，故上述普遍保证不能缩成三条。见证最早起点为1；未入选的首单元 `a` 保留为原级别前导，根目标显式保留 `[a] ++ [b,c,d]` 与原输出的对应，不能丢掉 a，也不能提升它的级别来补连接。

该见证仍是合法正量 L1 的合成构造，不是 Coinbase L3 事件或撮合行为证明。Stage9 的真实消息样本未用于本次定理或参数选优。

## 时序和语义边界

见证中的末成员 d 到观察5才确认，局部种子不得在其端点发生的观察4回填为已经可知。一般定理以“已确认四单元”为输入条件，不保证这些单元何时出现，也不承诺有限墙钟时间内一定有四条。

`CenterConfirmedComplete`是现有局部三段谓词的名字；它未替新候选证明上层走势完成、九段升级、同级连接或完整分类。它的“Complete”不能被解释为本研究全部 F₂ 义务已完成。

更高层对象的包络未必来自首尾接续且严格交替的端点，不能直接沿用四单元界。R_D 直接订单路线若将来独立满足相同几何前提，可使用这个条件引理；本结果没有证明供需符号或其他多变量构造满足前提。

本轮没有新增上层完成充分判据、把 active 改成 completed、改变有效域、修生产核或改教义。下一承重问题是：这个局部 seed 在延伸、破坏或重切后，怎样与前导/连接/尾部共同形成不丢成员、级别正确且具有独立完成证书的对象。单有本定理仍不能启动主 C 增量确认实验。

## 机器证据与复现

根声明：`SeedAssembly.exact_root`。

精确类型：`SeedAssembly.SEED_BOUND_TARGET ∧ SeedAssembly.SHARP_WITNESS_TARGET`。一般目标量化任意 p、正δ、合法完整分解及任意连续四单元；见证目标包含完整 DC 关系、成员保留、首窗失败、次窗成立、最早起点、核心数值和正量投影。

[精确检查摘要](stage10-lean-verification-summary.json)：run `440c8a99f5034651a29bec765a1ab9ae`，`machine_verification_passed=true`，`exact_root_passed=true`；传递假定仅 `propext/Classical.choice/Quot.sound`，无 `sorryAx`、自定假定或不安全依赖。`semantic.status=not_reviewed`。原始日志与完整导入清单位于 `/Users/silencehan/Documents/Codex/research-evidence/issue1467/seed-assembly-v1a/`。独立读回与比较小包已冻结为 pending-review，未派发、不算独评通过。

从研究工作树根目录复现（新的证据输出目录）：

```sh
LEAN_PATH="$PWD/formal/.lake/build/lib/lean" python3 \
  .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project "$PWD/.chanlun/review-results/issue1467-f1-proof" \
  --target-file SeedAssemblyProof.lean --declaration SeedAssembly.exact_root \
  --expected-type 'SeedAssembly.SEED_BOUND_TARGET ∧ SeedAssembly.SHARP_WITNESS_TARGET' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/seed-assembly-reproduction-2
```

使用本工作树已安装的验证工具和已构建的 Origin 模块。研究本地导入由工具在证据目录重新编译，正式源文件不被改写。输入/源文件绑定见 [哈希清单](source-sha256-stage10.json)；不是生产或全仓形式化验收，不触发正式源改动的 fixture gate。

冻结记录：初版锁 [v1](seed-assembly-spec-v1.sha256)在语义验证前遇到本环境未提供 `∃!` 记法的解析失败。原源保留为 [未解析 v1](SeedAssemblySpec-v1-unparsed.txt)；当前 [v1a 锁](seed-assembly-spec-v1a.sha256)将同一命题逐字展开为“存在 i，且任何满足关系的 j 等于 i”，未改前提或结论。旧 v1 锁是历史记录，不用于检查当前源。其后只修证明体的 Tick 展开、标准 Lean 语法与 Fin 值归约，最终精确检查没有遗留未证目标。

本轮交付的是一个一般局部存在性/选取引理与其尖锐性见证；#1467、原 P1–P4 和活动 goal 均保持开放。
