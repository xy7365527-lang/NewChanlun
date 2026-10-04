# #1467 Stage44：F1 完成约定的独立源义与数学审查

审查日期：2026-10-04。审查人不是原稿作者。绑定已有 #1467，只审本轮完成约定及固定 F2 交接范围，不裁定全套理论，不修改候选。

只读来源树 R = `/Users/silencehan/.codex/worktrees/order-event-base-research/NewChanlun`，实际 HEAD 为 `1c080e31e91840333f8a1bbaf0a4573276b8e9c1`。原稿 A = [f1-completion-scope.md](/Users/silencehan/Documents/Codex/research-evidence/issue1467/base-completion-v1/f1-completion-scope.md)，实际 SHA-256 为 `cdf58e8f2f4cded394a973b8b4de9bce6ed2af5e740f0da502027cb70246e0f2`，与委派一致。

本报告中的 `084:54` 等均指 R 下 `docs/chanlun/text/blog/084-第84课.md:54`；定义、ADR 与研究协议的路径也均相对于 R。来源哈希在文末逐份列出。审查独立回查原课、定义和协议，并重跑原稿全部 JavaScript 断言；未把历史审查摘要当作原文证据。

## 1. 审查结论与准入边界

可采纳原稿的有限结论：084/037 允许研究具名 F1 的初始完成约定；这不等于初始完成已经取得原义 a1 资格。RT 四核例确实不能原样继承 035 在左对象末自有核处的停止证书。规则内唯一不推出不同规则间同切点；017 类型接续也不单独推出 Z-3 的二元方向交替。

存在一项阻断精确判题的问题：A:117–127 把全称的 `MAX-035` 与研究许可性的 `BASE-CUT` 写成由四核例区分的两条，二者实际不处于同一逻辑层，也不互为否定。该段应先拆成研究许可与待证存在命题，才可作为下一轮的数学验收题。此问题不推翻上述局部结论，也不构成 RT 的否证。

原稿没有证明 RT 的 a1 身份、B5 的初始延伸语义、一般方向交替或完整 F2 闭合。其 A:115、125、142 已主动保留这些未决。B0–B7 可以作为待实例化的责任表，不能被接续任务称为已满足的充分判据或已冻结的完整 Ω。

## 2. 阻断问题：MAX 与 BASE-CUT 尚不是同一个判题

### B-1：研究许可不能与对象级全称约束作排他比较

定位：[A:117](/Users/silencehan/Documents/Codex/research-evidence/issue1467/base-completion-v1/f1-completion-scope.md:117)、[A:121](/Users/silencehan/Documents/Codex/research-evidence/issue1467/base-completion-v1/f1-completion-scope.md:121)、[A:125](/Users/silencehan/Documents/Codex/research-evidence/issue1467/base-completion-v1/f1-completion-scope.md:125)。

`MAX-035` 断言任一新 F1 的最低 U 都必须吸收下一枚更高核心。`BASE-CUT` 当前只说可以提出跨界后继见证的完成定义，再证明其合法性。允许提出定义，即使最后被 MAX 拒绝，也不与 MAX 矛盾。更直接地说，035 自己就可读取下一反向核心作为前一 U 的结束证据；“证据跨界”与“同向核先被原 U 吸收”完全可以同时成立。

因此四核例实际证明的是：在固定的 RT 归属中，左 U 末自有核是 20，而 `30 > 20` 且 `25 > 20`，所以用于 30 的 `25 < 30` 不能移作在 20 处的 035 停止证书。它没有证明 `MAX-035` 假，也没有证明存在满足 Ω 的 RT 构造。A:125 的“精确区分两条”必须收窄到“区分 RT 所声明的切点与 035 旧证书适用的切点”。

最小修复是保留三项，分别标名分与量词，不需要找一句原文替具体 RT 切点背书：

1. **BASE-CUT，研究许可**：可以把具名初始完成、跨界证据读取和归属关系作为候选定义研究。这不是候选存在定理，更不是语义通过证书。
2. **MAX-035，待审全称命题**：先明确候选类 C、已冻结的 Ω、核心身份与时序，以及“高于”的关系。若范围是点平台，只用数值 `c_(j+1) > c_j`；若推广到有外缘的中心，就必须使用对应的具名关系，不能把数值排序默认为 M-2 的外缘分离。还须明确其主语是已合格构造，还是尚未通过的所有候选。
3. **BASE-CUT+，待证存在命题**：存在一个具名 B 及历史 h₀，B 在它声明的证明域上满足固定 Ω；h₀ 的核心列为 `10,20,30,25`，B 确认一份自有核仅为 `10,20` 的左 U，`end=6`、`confirm=12`，30/25 归右。当前没有这一证明。

可把两项真正可比较的数学命题写成下列骨架。`GoodΩ(B)` 必须由独立列出的 Ω 定义，不得写成“B 的程序正常运行”，也不能把争议 MAX 偷放进去后再声称证明了 MAX：

```text
MAXΩ：
  对每个 B∈C，若 GoodΩ(B)，则其每份已完成最低 U，
  不能在下一枚同级核心仍满足 AboveB 的位置之前封定。

BASE-CUT+：
  存在 B∈C 与 h₀∈H_B，GoodΩ(B)，且 B 在 h₀ 上形成上述 RT 左 U，
  其下一枚同级核心为 30，而末自有核心为 20，AboveB(30,20)。
```

在相同 C、Ω、核心身份和 AboveB 下，BASE-CUT+ 才会给出 MAXΩ 的反例。四核算术没有建立 `GoodΩ(B)`。以上只是精确化建议，不是本审查宣布任一侧成立。

018:42–44 还提供了“趋势延伸体现为同级同向中枢继续产生”的独立语义责任。不能只说 MAX 的名字含 035，就忽略 018；但 018 这几句也没有单独规定所有新 F1 中的核心归属算法。具体分岔仍须由独立 Ω 与 RT 的 `Extends_RT`/`Closed_RT` 关系处理。

此项阻断的是“已经得到可直接二选一的精确验收命题”，不阻断继续研究 RT，也不要求修复全仓引擎。

## 3. 非阻断修订

### N-1：结束位置与确认时点在一处记号中混用

定位：[A:29](/Users/silencehan/Documents/Codex/research-evidence/issue1467/base-completion-v1/f1-completion-scope.md:29) 定义 `Closed_B(M,e,w)` 中 e 为结束边界；[A:85](/Users/silencehan/Documents/Codex/research-evidence/issue1467/base-completion-v1/f1-completion-scope.md:85) 却写 `Closed_RT(left,e12)`。本例结束位置是 e6，e12 是确认时点，且该写法省掉了见证参数。

最小修复可统一成 `Closed_RT(left,end=6,w)`，另记 `knownAt(w)=confirm=12`；或明确改用四参数 `Closed_RT(left,end=6,confirm=12,w)`。A:79–85 的文字与表格已经区分两时点，故这是形式记号不一致，不是算例本身错误。修订稿必须改正后再引用精确证书。

### N-2：B2 的“同向”与 B7 的“非空”应在实例化时展开

A:48 已引用 M-1/M-2，所以没有另立只有标量排序的趋势判据。但后续验收表应写出：恰一枚相应级别核心为 P，趋势的相邻核心按 M-2 使用外缘分离。点平台外缘退化为同一点时，才可约化为核值严格同向；这一约化的适用域见 `stage42/classification-bridge.md:52–54`。

A:53 的非空升级和终止应沿 `GoalReframe-v4.md:31–36` 保留量词：给出非空完成/升级见证，有限输入递归有终止依据；并非每个有限前缀、每个历史都必须升级。A:97、102 已避免了后一误读。A:38 的含前提式也是保持性责任，不是已经证明 F2 适用前提。递归实例化时需要逐层证明输入资格、输出资格和适用域，不得仅因条件蕴涵成立就宣布闭合。

### N-3：029 答疑的作者归属链应向前多引几行

A:179 用 `029:358–364` 描述归属核对。实际作者块开始于 `029:340` 的“缠中说禅：2007-02-12 17:04:25”，`:342` 是读者提问，`:360` 的等号引出 `:362` 作者回答；`:364` 已是下一答疑块的署名时间。结论归属正确，建议把核对范围改为 `029:340–362`，避免把下一条署名当作本条证据。

## 4. 原文授权与 B0–B7 的逐项核验

084:52 明许替换启动程序，并要求分解唯一；084:54 明说 F1 之前不必再有递归，F1/F2 可以不同，第二条中枢过程规则不变；084:56 把 F1 产物明确称为最低中枢、走势类型。037:170 作者回答反驳的是“三笔同价就翻转”，并说明 a0 的定义不改变上递归函数的定义。它没有指定任何新完成算法正确。

由此可采纳“初始完成约定可以写进候选 F1”，名分仍是从来源推得的研究许可。不能把这几句变成“唯一分块就充分合法”，也不能把一个旧 F1 的特定结束事件未经桥接推广为所有 F1 的算法。017:40 的不可再分层为非递归基提供了直接实例。083:14–22 承认笔基底可造、但论述其稳定性差；本仓 S-5 的不采用决定仍约束生产。085 的本篇标题及做顶叙述没有提供初始构造定义，原稿来源订正成立。

| 合同 | 核验结果 | 不能由此省掉的工作 |
|---|---|---|
| B0 | 来源、有效域、尾部和缺口由 v4:23、31–36 支持，是研究协议要求，非假冒原文 | 写出 H_B，保存域外及尾部；不能选择性删除不利事件后续接 |
| B1 | 084:52、038:18 支持规则内唯一；037:172–174 支持同一式中不重复归属 | 关系独立给出、证明存在及唯一；确定性实现不能代替它 |
| B2 | 084:56、017:44–52、M-1/M-2 支持本级核心、分类及同向条件 | 核心身份与外缘必须具名；最低零件与原义 a1 分开；计数不推出已完成 |
| B3 | 018:40–44 区分可完成与实际结束；v4 P2 规定时点与前缀忠实 | 发生、结束、可知、确认、发布不能混写；确认见证不自动改成对象成员 |
| B4 | 017:30、36、60 与 018:38 支持完成下跌之后接上涨或盘整，镜像适用 | 约束的是实际声明的原义完成对象及其连续覆盖，不能删掉中间活动片段制造假相邻 |
| B5 | S-6 与 ADR0011 对消费角色及延伸有直接的本地责任 | DEP 无回流与 EXT 延伸相容分开证明；EXT 的 RT 实例仍空缺 |
| B6 | 018:24 明定连续、次级、已完成三组件；正本 Z-1/Z-2/Z-3/Z-8 规定范围、严格交集、方向及连接成员 | 完整路径范围与核心计数分开；P 的技术方向不能冒充概念方向；连接成员不能漏 |
| B7 | 084:54–56 固定上递归，v4 P1 要求分类、闭合、有限递归终止及非空见证 | 下层接续或父核存在不能替代父走势完成，更不能替代一般闭合 |

没有发现这张表明确豁免真正原义义务，或额外要求 F1 等于旧线段/成交笔算法。其缺口是部分谓词尚未实例化，尤其 B5、固定 Ω 与全层输入资格。这是原稿声明的未决，不应升级为本轮已经完成的证明。

同样，原稿把第 37 课 a+A+b+B+c 的 c 内两枢、三类点、新高低限制在相应趋势背驰对象上，符合 037:16–22；未将这些条件删出真正使用它们的动态消费方。对于仅报告有界初始结构结果，不需要先证明全部背驰与交易能力；一旦声称完整动态 F2，实际消费的前提仍须补齐。

## 5. S-6、DEP 与 EXT

`level_recursion.md:283–295`、`zhongshu.md:24–37`、`ADR0011:15–21` 支持依赖单向：横向读法消费纵向产物，结果不得回灌纵向。这里检查的是证据与构造依赖，不能只查某个模块有没有被调用。一个从原始历史直接定义的 F1 可以实证没有横向回流，无须先构造旧塔。

`ADR0011:55–70` 又明确把“继续往上造的中间产物保留延伸”归于消费角色。它自己标注为本地推断，推自 038:26 与 038:30，原稿 A:113 对名分的说明正确。无回流不蕴含延伸相容，两项必须分开。另一方面，这个角色规则没有指定每个新 F1 必须调用旧线段函数，也没有单独证明 MAX 对任何基底成立。

RT 仍要提供一个独立可验的条件，说明在其原义主张下 30 为何可归右，且这样做没有把同一未结束对象换 ID 拆开。“我规定它归右”可以定义程序，却不单独完成 B5。这一缺口与原稿 A:115 一致；当前不能以“原生 F1”解除它。

## 6. 类型逻辑、P 与 Z-3

A:61 的量词反例成立。两个规则分别唯一选择二项分块与三项分块，并不推出对两规则取并后仍唯一。它只否定从各规则内唯一到跨规则同切点的逻辑推导，不认证计数分块为合法缠论构造。

对相邻异类三值串，独立枚举得到 12 个串。其中 `U,P,D` 与 `D,P,U` 不能在固定 U=+、D=−、P 可临时取 ± 的设定下实现相邻二元方向交替。第一种串要求 P 同时为 − 和 +。031:386 的作者答复与 `qushi.md:44–47` 也确认盘整无概念方向；`break_direction` 是技术标记。

这准确证明了“类型接续条件单独不蕴含二元方向接口”。它没有提供合法全局 U/P/D 三元组，因而不能被写成 Z-3 与原义走势必有矛盾的反例。A:71、146 已限定到这一强度，限定应保留。P/P 也只能按 038 特定操作读法或 029 特定三买前提分析，不能从存在局部许可推到无条件纵向许可。

## 7. RT 四核独立核算与最小性范围

本审查另造 12 个合法新增/撤单源事件，逐事件更新订单残量，得到 `Q=10³,20³,30³,25³`。这是合成的协议内算例，不是实际市场采集。每次同值观察都来自不同事件，没有复制同一快照。固定买价 100、卖价 102、卖量 10；全部买量为正，故也处于 v2 的 D_W 数值条件内。

只条件性采用原稿声明的 RT 归属，不假装已经定义出 RT 的全域关系。独立核算得到：

| 项目 | RT 左对象 | RT 右活动尾部 | 沿用 035 局部同向核延伸的左对象 |
|---|---|---|---|
| 自有核 | 10、20 | 30、25 | 10、20、30 |
| 独占事件 | e1…e6 | e7…e12 | e1…e9 |
| 用于几何范围的状态 | B0…B6 | B6…B12 | B0…B9 |
| 实际范围 | [10,20] | [20,30] | [10,30] |
| 结束位置 | 6 | 尚无结束证明 | 9 |
| 此轮确认 | 12 | 不声明已完成 | 12，仅局部 035 证书 |

右尾部自有核的包络是 [25,30]，但完整路径先读到边界状态 20，故真实范围是 [20,30]。边界状态共读不等于重复拥有 e6 或核心 20。这个区别必须留到 B6，不能只保存自有核范围或首末净方向。

RT 的反转证据是 `30→25`，但其左末自有核是 20。`30≤20` 与 `25≤20` 都为假，`25≤30` 为真。四核例因此可以排除旧证书的直接迁移；它不证明 RT 违反或满足最终 Ω。此时只有一份 RT 声明的完成对象，不能核三个完成组件的父中枢，更不能核父走势完成。

最小性只能在所声明形态内给出：左 U 至少两枚自有核心，整枚归右的极值核一枚，其后的反向确认核心一枚，核心不重用，故至少 4 枚；每枚三次不同源事件才能成熟，故至少 12 事件。本例达到该下界。这不是所有 F1、所有完成证书或全部 RT 失败见证的全局最小性。

## 8. 实际运行记录

运行环境：Node `v22.23.1`，Python `3.14.6`。记录时刻 `2026-10-04T10:50:27Z`。全部运算只使用本地文件及内存，没有访问生产引擎。

第一次核算用 Python 从 A 的 `javascript` 代码围栏逐字提取代码，调用 `node --input-type=module`，未改代码。提取内容 SHA-256 为 `c929dceff849991623ba0d2bc6be0fff7041eaabe3090730410734a5703b9fab`。exit=0，stderr 为空，stdout 为：

```json
{"typeDistinctTriples":12,"notAlternatable":[["U","P","D"],["D","P","U"]],"ocLocal035Stops":true,"ocTypes":["D","D"],"rtNo035StopAtLastOwnedCore":true,"allAssertionsPassed":true}
```

第二次运行是本审查自写的以下独立事件与范围核算。使用 `node --input-type=module` 标准输入，exit=0。其打印的三范围为 `[10,20]`、`[20,30]`、`[10,30]`，结束/确认分别为 6/12 与 9/12，类型枚举结果同上。它检验有限算术、残量合法性和题设归属，不实现或验证 RT 的一般完成关系。

```javascript
import assert from 'node:assert/strict';
const orders = new Map([['S0',{p:100,q:10}],['A0',{p:102,q:10}]]);
const events = [
 ['add','U1',99,1],['cancel','U1',1],['add','U2',99,1],
 ['add','S1',100,10],['cancel','U2',1],['add','U3',99,1],
 ['add','S2',100,10],['cancel','U3',1],['add','U4',99,1],
 ['cancel','S2',5],['cancel','U4',1],['add','U5',99,1],
];
const Q=[10];
for(const e of events){
 if(e[0]==='add'){ assert.ok(!orders.has(e[1])); orders.set(e[1],{p:e[2],q:e[3]}); }
 else {const o=orders.get(e[1]);assert.ok(o && o.q>=e[2]);o.q-=e[2];if(o.q===0)orders.delete(e[1]);}
 Q.push([...orders.values()].filter(o=>o.p===100).reduce((n,o)=>n+o.q,0));
}
assert.deepEqual(Q.slice(1),[10,10,10,20,20,20,30,30,30,25,25,25]);
const kernels=Q.slice(1).filter((_,i)=>i%3===2);
const span=(a,b)=>[Math.min(...Q.slice(a,b+1)),Math.max(...Q.slice(a,b+1))];
const rtLeft={ownedCores:kernels.slice(0,2),ownedEvents:[1,6],pathStates:[0,6],range:span(0,6),end:6,confirmation:12};
const rtTail={ownedCores:kernels.slice(2),ownedEvents:[7,12],pathStates:[6,12],range:span(6,12),coreRange:[25,30],completed:false};
const oldLeft={ownedCores:kernels.slice(0,3),ownedEvents:[1,9],pathStates:[0,9],range:span(0,9),end:9,confirmation:12};
assert.deepEqual(rtLeft.range,[10,20]);assert.deepEqual(rtTail.range,[20,30]);assert.deepEqual(oldLeft.range,[10,30]);
assert.equal(kernels[2]<=kernels[1],false);assert.equal(kernels[3]<=kernels[1],false);assert.equal(kernels[3]<=kernels[2],true);
const types=['U','D','P'];const ts=types.flatMap(a=>types.flatMap(b=>types.map(c=>[a,b,c]))).filter(t=>t[0]!==t[1]&&t[1]!==t[2]);
const directions=t=>t==='U'?[1]:t==='D'?[-1]:[-1,1];
const impossible=ts.filter(t=>!directions(t[0]).some(a=>directions(t[1]).some(b=>directions(t[2]).some(c=>a===-b&&b===-c))));
assert.deepEqual(impossible,[['U','P','D'],['D','P','U']]);
console.log(JSON.stringify({sourceEvents:events.length,kernels,rtLeft,rtTail,oldLeft,minimumCoresConditional:2+1+1,minimumEventsConditional:3*(2+1+1),typeTriples:ts.length,notAlternatable:impossible,assertions:'PASS',qualification:'finite arithmetic and declared ownership only'},null,2));
```

这些通过结果没有检验源义，源义审查依赖前述原课和正本推理；也没有用程序确定性背书理论。

## 9. 来源归属、读取范围与工作树边界

承重正文的原稿行号均复核未漂。017 正文界为 :98，018 为 :68，035 为 :38，037 为 :40，038 为 :44，084 为 :68。037:170、174 是作者答疑，署名与时间在 :164–166；031:386 是 :384 所署作者的回复，:390 是加粗重复。未把读者提问或重复加粗当作新增证据。

018:24 的行内“娇”注、018:46–48 整段编注未进入论证；018:14、50、60 的注也未用作作者结论。037:26、30、038:14、42 的注未使用。083:14–22 未见所引编注；:24 的行内注不用于论证。029:356 是编注，未用于证明 P/P 的条件。017:28、32 的解释性注不用于完成原理。085 按本篇 :1–50 及其后解盘的章节变化核对，没有把个股做顶叙述当作 F1 定义。未依赖图片。

主要逐段复核范围：原课 017:24–100、018:12–68、035:12–38、037:14–22及164–176、038:14–44、029:330–364、031:384–393、083:12–24、084:12–68、085:1–50；定义 `zhongshu.md:16–158`、`qushi.md:40–70,74–131`、`level_recursion.md:43–110,142–148,283–297,329–340`；ADR0011 全文。研究协议 v4:15–36、Stage43 进展及其具名证据段用于核对合同、域、状态和原稿上下文，不作为原课的替代证据。其余来源有哈希核对或定点阅读，哈希通过不表示全文语义已审完。

未改作者原稿、研究树、教义、formal 或生产代码，未提交、推送或外发。执行 `git status --short --untracked-files=no` 时，来源树已有 `ResearchProgress.md` 与 `Stage40CompletionGate.md` 两项修改；本审查未触碰它们，不能把该树称为干净快照。HEAD 与逐份 SHA 共同限定本次读取事实。

本轮接续宜采用修正记录或新版本：保留原稿冻结，拆开 BASE-CUT 与 BASE-CUT+，统一 `end=6`、`confirm=12`，再对独立 Ω 和 RT 的初始延伸条款作新一轮审查。当前仍未取得 RT 合格构造、F2 完整资格或市场价值结论。

## 10. 实际来源 SHA-256

对 A 清单中的 22 个来源逐文件重新计算 SHA-256，22/22 与原稿一致。下列为本审查实际计算值，不是未经核对的原稿转录。

```text
bce847aaacac3b3a79e56b3c49aca6f0c454a0917977bdbb1e0f1f553ddff026  docs/chanlun/text/blog/017-第17课.md
4a661fc843d0d9fcfdf32861b7dc903ee09d824d0075dd0ea7b2c19542bb2f92  docs/chanlun/text/blog/018-第18课.md
5716dbd674811392ac563cde255edec00c2902e3c3561159faa5ce2118fa369f  docs/chanlun/text/blog/029-第29课.md
c784ee15c21cef965b6e073a89224e56b29bad03a9ecd1dc9e130c54083d47d5  docs/chanlun/text/blog/031-第31课.md
0bb81a62b58ecf8900f6c40227194c1fbb139975bae5aa34d7a881363067b023  docs/chanlun/text/blog/035-第35课.md
f360d985b3f3b1fa567ca13e15f2ffaf9eae39dbb418519f75d1f6175ac70753  docs/chanlun/text/blog/037-第37课.md
983ce94575d61ed645fcc753eb59f9a28e8e749826a30d7bf87f19721baedc7b  docs/chanlun/text/blog/038-第38课.md
52652ad410d1c393b46359e406df2133e6630ba5ae940b328c0c7845018bf634  docs/chanlun/text/blog/083-第83课.md
36978e2c0ee0d49fc316611af94a228f7e5d6bfa0e0a05bcd3ad66ad41879c26  docs/chanlun/text/blog/084-第84课.md
3e40b15cd24f384d6da3cb78c75cf3a68bd5d4da6d009225dce8c8e095834cb3  docs/chanlun/text/blog/085-第85课.md
069fc240c8476cc3df62c01d1b658c42f00ec4428777d479163beaadc48ec882  .chanlun/definitions/zhongshu.md
06ac7aee96b089536766ac172b52ff0f8d702f2e4ec01ca2b47b1f3a590e1637  .chanlun/definitions/level_recursion.md
ce48f8cda6a3f02afd9fddbc0f93cb43431524b66472e579a927af5ce4bc3c46  .chanlun/definitions/qushi.md
30098873c67878af850e8ff9502fdf69e332343e3576e551bab6768d6eb59860  docs/adr/0011-operation-decomposition-layer.md
c75db87ef0888cec00dd93edc233b72fa7106fed77e563a1e9c26d25c4f3df37  .chanlun/review-results/issue1467-f1-proof/Stage43CompletionAndDynamicW.md
36159c5f6bd4062a6654bd3f22b3eaf00e678e5d5dacda844cbf659e230ce39d  .chanlun/review-results/issue1467-f1-proof/GoalReframe-v4.md
8d5c9ce3b20959704d3c5d8b6c5a12ab98e2f68a0786afa4c4b103e275ca222e  .chanlun/review-results/issue1467-f1-proof/independent-review-v1/source-scope.md
24922064741d453e13b516c427940ebf611166220d46e8218bafce9ad8931e3c  .chanlun/review-results/issue1467-f1-proof/stage43/w-axis-obligations.md
ca91a02139000990692a25705b447e85bf0b9a31f50904c1739abef05e5a8b16  .chanlun/review-results/issue1467-f1-proof/stage43/panzheng-completion.md
5b0d92d2a32d885ff89647be31c089226ea228fa1cad1385db1825cf20270990  .chanlun/review-results/issue1467-f1-proof/stage43/owned-core-candidate.md
a021c88c8b60b788ae1d87097109d6658db679b7f2fa1b962861d6bc567dc6d0  .chanlun/review-results/issue1467-f1-proof/stage42/classification-bridge.md
ff4e1d062e78891ff2c563ae06ba3ac842ac10885a5d275795fcd852624daa43  .chanlun/review-results/issue1467-f1-proof/PlateauBaseCandidate-v2.md
```
