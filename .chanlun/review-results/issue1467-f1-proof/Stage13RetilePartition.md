# Stage13：固定三段重切的成员守恒与自身核心

日期：2026-10-03，#1467，命题 `DC-RETILE-PARTITION-v1`。承接 [开工卡](Stage13WorkCard.md)与 Stage12 待重切分支。名分：研究关系、局部构造和定义内 L0 证明；完整 F₂ 尚未成立。

## 本轮结果

已把固定三段重切写成独立关系并证明：任意有限输入都有唯一的固定切分，尾部少于三段，全部子窗及尾部能逐项还原原成员序列。子窗成功与否不改变后续窗口位置。

每个通过的子窗复用真实 `Origin.centerFullOfConfirmed` 构造器，核心、外缘和源起止全部来自自身三段；被拒绝子窗仍保存原三成员。重切时刻参与新对象的可用时间，不能仅取成员原先的确认时间。

接入两份已有合法 DC 见证：Stage12 九段路径得到三个局部核心；Stage8 反例仍只得到两个，第三窗拒绝。两者都完整保留九成员。本轮没有通过滑窗补救或继承母核心让反例通过。

## 独立切分关系与守恒

`Cut`只有两种规则：

1. 剩余长度 `<3`，只能整体留作尾部；
2. 剩余长度至少三，前三项组成一个固定块，再对剩余序列应用同一关系。

证明构造存在，并从关系本身推出其输出等于固定切分的规范结果；任何两个满足关系的结果因此相同。另证明：

`flatten(全部块) ++ 尾部 = 原输入`，

`原长度 = 3 × 块数 + 尾长`，且 `尾长 < 3`。

随后对每块分类，无论通过或拒绝，其 `source` 都等于原块。因而分类后仍有 `recover(全部结果,尾部)=原输入`。这是逐项、保序的恢复等式，不只是总数相同。在原来源序列有效的前提下，它不新增重复或丢失成员；共享端点不等于重复消费单元。

“唯一”仅针对这项已指定的固定切分规则，不宣称完整走势划分、生命周期或递归塔唯一。

## 子窗局部资格

每块三个 `ObservedUnit`保留旧 DC 来源与端价。满足实际 `Origin.CenterConfirmedComplete`时产生 `localCore`；其 `CenterFull`由原构造器计算，已一般性证明：

- ZD/ZG等于自身三段的全交集边界；
- DD/GG等于自身三段的外缘；
- 源 start/end来自自身第一/第三成员。

构造器的外缘包含核心证明直接继承。没有可供调用者填写母核心的字段，也没有失败后换宽判据的分支。

否则产生 `rejected`，仍带完整三成员和否定局部谓词的证明。在任意人工载荷上，拒绝可能来自方向或几何错误；本轮另证：**对满足旧 DC Whole 的连续三单元，方向交替已由旧几何定理保证，因此拒绝恰等价于 `ZG≤ZD`，即没有严格核心。**在这个有效域内才称它为 NoCore。

原始裸载荷的切分不自动取得历史真实性；真实输入仍须携带 DC 来源和可知性证明。两份见证都使用已有完整 Whole 证据，没有拿虚构三段表代替它。

## 新对象的可用时间

每块保存：

`availableAt = max(重切触发时刻, 三成员 knownAt 的最大值)`。

一般定理证明它不早于触发参数或任何成员的确认。触发参数的真实性仍由调用者提供证据；max公式不能自行证明触发发生过。

在接续 Stage12 的见证中，待重切状态确实在观察10形成。虽然前两组成员更早就已确认，三个新重切结果的 availableAt均为10。它们没有被回标为观察4或7已产出的重切子对象。这仍不是上级走势的完成证书。

## 两份既有见证的实际接入

四元组按 `(ZD,ZG,DD,GG)`记录。

| 输入 | 固定块0 | 固定块1 | 固定块2 | 原成员恢复 |
|---|---|---|---|---|
| Stage12 C，九段待重切 | (10024,10032,10020,10040) | (10030,10050,10024,10060) | (10028,10060,10026,10070) | 全部九成员、原顺序 |
| Stage8 合法反例 | (10025,10045,10025,10045) | (10040,10045,10030,10050) | NoCore，保留三成员 | 全部九成员、原顺序 |

两者均有三个块、空尾部，所有块（含拒绝块）的 availableAt都是10。Stage8 的第三窗没有核心对象；若只取成功中心，便不足以还原输入，所以完整结果必须同时保留拒绝块。本轮根目标直接检查了完整还原。

不足三段尾部的处理由任意有限列表的一般定理覆盖，不以两份恰好九段、尾部为空的见证代替该证明。

## 证据与复现

根 `Retile.exact_root`，精确类型：

`PARTITION_TARGET ∧ CELL_TARGET ∧ DC_REJECT_TARGET ∧ WITNESS_TARGET`。

run `133de82274314a7da367a6e84ce5e815`，Lean4.31 精确检查 exit0，machine/exact_root均通过。实际假定仅标准 `propext/Classical.choice/Quot.sound`，无 sorryAx、自定假定或不安全依赖。摘要见 [stage13-lean-verification-summary.json](stage13-lean-verification-summary.json)，源绑定见 [source-sha256-stage13.json](source-sha256-stage13.json)。

初次精确检查遇到 `Decidable` 实例展开不足；当前 [v1a 锁](retile-spec-v1a.sha256)只在该实例证明中展开既有 `CenterConfirmedComplete`，没有改变谓词、定义域或目标。原 [v1 锁](retile-spec-v1.sha256)与[初版源](RetileSpec-v1-elaboration-failed.txt)保留为历史，不用于检查当前源。另将含自由变量的长度证明由 `decide` 改为符号化简；没有修改关系规则或见证。

完整证据：`/Users/silencehan/Documents/Codex/research-evidence/issue1467/retile-v1a/`。独立读回/比较包已准备，尚未派发；semantic仍为not_reviewed。没有修改formal/生产、下载新行情或重跑旧市场枚举。

从工作树根目录复现，使用新证据目录：

```sh
LEAN_PATH="$PWD/formal/.lake/build/lib/lean" python3 \
  .agents/research-tools/xsoc1-9e0da0c3/plugins/lean-verify/scripts/verify_lean_project.py \
  --project "$PWD/.chanlun/review-results/issue1467-f1-proof" \
  --target-file RetileProof.lean --declaration Retile.exact_root \
  --expected-type 'Retile.PARTITION_TARGET ∧ Retile.CELL_TARGET ∧ Retile.DC_REJECT_TARGET ∧ Retile.WITNESS_TARGET' \
  --lean /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lean \
  --lake /Users/silencehan/.elan/toolchains/leanprover--lean4---v4.31.0/bin/lake \
  --direct --build-timeout 60 --strict-exit \
  --output /Users/silencehan/Documents/Codex/research-evidence/issue1467/retile-reproduction-2
```

## 仍需解决

输出只有局部核心或拒绝证据，没有 `Move` 或 `completed` 构造子。通过子窗仍不能直接上送为完整高一级走势；NoCore片段的后续组合、成功离开分支、其他生命周期状态以及父帧身份/修订与新对象身份的映射仍未定义。当前函数接收成员列表和触发时间，完整驱动器还需保存父构造帧的上下文。

下一步应从“局部核心已齐”转向“哪些带来源的对象具备完整走势资格”，明确完成条件及允许上送的接口；不能用切分/局部构造通过替代此语义门。两条F₁的全部资格、完整F₂、独立审查和四臂增量价值保持开放。
