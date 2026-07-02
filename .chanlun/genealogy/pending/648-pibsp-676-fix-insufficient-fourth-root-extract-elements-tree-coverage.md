---
id: "648"
number: 648
status: 已结算   # 【编排者裁决 D 视图分离，chat「子声部.pdf」27页严格推导，2026-06-29】超越上呈 A/B/C 三选一。根因精确化=host^op=host^struct 混用(非 P1∧P2 不可避免定理)；解法 D=分离结构视图树 T_i(↓r_i,保留P1) 与操作 carrier forest K_i(endpoint-complete,host^op 定义域)，host^op 仍严格右端点命中(保留P2a,不违638)，只改 host 宇宙 T_i→K_i(P2b)。非 workaround(PDF§11:不伪父链/不级别差/不区间包含)=真扬弃。§10定理1:endpoint-complete K_i + 解释器生成父voice ⟹ 子声部可激活。settled 落盘走 /ritual(019c)。task#14 实装 K_i。
date: "2026-06-29"
type: domain   # 改判：编排者裁决=视图分离（结构视图 ⊥ 操作视图）。原 type=contradiction（A/B/C 待裁），裁决 D 消解为双视图分离（host^op≠host^struct）。
depends_on: ["676", "644", "231", "090"]
related: ["638", "645", "646", "640", "547"]
negation_form: "aufhebung"   # 改判：原 expansion(三→四根因)/三方向互斥待裁 → 第四方案 D 扬弃。否定(host混用)+保留(P1+P2a+π^cov bit-exact)+提升(host^op≠host^struct 双视图)。
title: "π^bsp 子声部=0【编排者裁决 D 视图分离消解】根因精确化=host^op=host^struct 混用(非 P1∧P2 不可避免)。解 D：分离 T_i(结构视图树=↓r_i,保留P1) 与 K_i(操作 carrier forest=endpoint-complete,host^op 定义域)；host^op 仍严格右端点命中(保留 P2a,不违638)，只改 host 宇宙 T_i→K_i(P2b)。非 workaround(PDF§11:不伪父链/不级别差/不区间包含)=真扬弃。§10定理1:endpoint-complete K_i + 解释器生成父voice(非子反推父) ⟹ 子声部可激活。推翻『π^bsp 子声部层=wrong object』倾向——子声部=0 是 host 宇宙不足(可修)非 goal#5 本质错误；命题A(645)π^cov≠π^bsp 仍成立"

negation_source: "编排者价值判断（chat「子声部.pdf」27页严格推导，权威）+ 本号 L2 源码事实（676 修复后子声部仍=0，host-hit 2-8%/parent_active=0）。裁决 D 超越本号上呈的 A/B/C 三选一。"

# 裁决 D（编排者，chat 权威）改判：
#   原本号上浮三方向互斥待裁（A 改 extract_elements 多级森林 / B 重定义 hostOf / C 接受退化定理）。
#   编排者经「子声部.pdf」27页严格推导给出第四方案 D（视图分离），超越三选一：
#   - 根因精确化：子声部=0 是 host^op=host^struct（操作 host 与结构 host 混用）导致，
#     **非** 单纯 P1(单最高级降序树)∧P2(严格右端点命中) 的不可避免定理（C 被否）。
#   - 解 D：分离两个视图——
#       T_i = 结构视图树（↓r_i 单最高级降序，保留 P1，保留 π^cov bit-exact）；
#       K_i = 操作 carrier forest（endpoint-complete，host^op 的定义域）。
#     host^op 仍严格右端点命中（保留 P2a，**不违 638**），只改 host 宇宙 T_i→K_i（P2b）。
#   - A 被超越：A 要改 T_i（冲击 π^cov bit-exact）；D 不改 T_i，另建 K_i ⟹ π^cov bit-exact 保留。
#   - B 被否：B 放宽 hostOf 到区间包含（违 638）；D 的 host^op 仍严格右端点命中（不违 638）。
#   - 非 workaround（PDF §11 明示）：不伪造父链、不用级别差、不区间包含 ⟹ 真扬弃。
#   - §10 定理1：endpoint-complete K_i + 解释器生成父 voice（非子反推父）⟹ 子声部可激活。

topo_effect: "dissolve:648-pibsp-subvoice-zero:view-separation[T_i=structural-view-tree(↓r_i,P1,π^cov-bit-exact) ⊥ K_i=operational-carrier-forest(endpoint-complete,host^op-domain)] | preserve:P1+P2a+638-strict-endpoint-hit+π^cov-bit-exact | negate:host^op=host^struct-conflation | lift:host^op≠host^struct-dual-view | record:§10-theorem1-endpoint-complete-K_i+interpreter-generates-parent-voice⟹subvoice-activatable"

contradiction:
  description: |
    【原矛盾 + 编排者裁决 D 消解】676 诊断 π^bsp 子声部=0 三根因（pi_bsp_timing.rs）：
    (a) §11 close 优先（P3 出场）开仓前平掉反向 active 根声部；
    (b) open_new active_root_by_carrier.find 只比方向不比 carrier；
    (c) buy/sell 分两循环但 active 快照只取一次。
    本号实装 676 修复路径后子声部仍恒=0（OKLO/CL/BTC），坐实**第四根因**：
    extract_elements 单最高级降序树仅覆盖 ~36-42% L0 走势，bsp 点 92-98% 落 orphan
    frontier 段（树外）⟹ host-miss ⟹ ∂ 根；少数 host-hit 父容器又无 active 声部
    （parent_active=0）⟹ 联合条件（host-hit ∧ 父容器有 active）真实数据结构性为空。

    **编排者裁决 D（视图分离，消解矛盾）**：第四根因的真因不是「P1∧P2 的不可避免定理」
    （本号原倾向 C），而是 **host^op=host^struct 混用**——把结构视图树 T_i（↓r_i 单最高级
    降序，π^cov 用）直接当作 host^op（操作 carrier 附着）的 host 宇宙。两个视图的定义域不同：
    - T_i（结构视图）：单最高级降序树，coverage.rs:218 不重复作根铁律，π^cov bit-exact 依赖它。
    - K_i（操作视图）：endpoint-complete carrier forest——所有走势端点都是合法 carrier，
      是 host^op 的正确定义域。

    解 D 分离二者：host^op 仍严格右端点命中（保留 P2a，不违 638），只把 host 宇宙从 T_i
    换成 K_i（P2b）。endpoint-complete K_i ⟹ host-hit 率不再受 T_i 的 36-42% 覆盖局限；
    解释器生成父 voice（§10 定理1，非子反推父）⟹ 子声部可激活。

    **为何非 workaround**（PDF §11）：不伪造父链、不用级别差当父子（547 否定）、不放宽
    hostOf 到区间包含（638 否定）。D 是真扬弃——否定（host^op=host^struct 混用）+ 保留
    （P1 结构树 + P2a 严格右端点命中 + π^cov bit-exact）+ 提升（host^op≠host^struct 双视图）。

  layer: 实装   # host^op（操作 carrier forest K_i）与 host^struct（结构视图树 T_i）分离——实装双视图。非定义冲突（638/§3 定义保留），是 host 宇宙的视图分离。
  trigger: "task#2 实装 676 修复后子声部仍=0（本号坐实第四根因）→ 上浮 A/B/C → 编排者经「子声部.pdf」27页裁决 D（视图分离），超越三选一。"

resolution:
  type: 已结算（视图分离 D）   # 编排者裁决 D：分离 T_i(结构视图,P1) 与 K_i(操作 carrier forest,host^op 定义域)。非 A/B/C 任一。chat 权威价值判断。
  description: |
    【编排者裁决 D，chat「子声部.pdf」27页严格推导】子声部=0 根因精确化为 host^op=host^struct
    混用（非 P1∧P2 不可避免定理），解法 D=视图分离：
    - T_i = 结构视图树（↓r_i 单最高级降序，保留 P1，π^cov bit-exact 不动）。
    - K_i = 操作 carrier forest（endpoint-complete，host^op 的定义域）。
    - host^op 仍严格右端点命中（保留 P2a，不违 638），只改 host 宇宙 T_i→K_i（P2b）。
    - §10 定理1：endpoint-complete K_i + 解释器生成父 voice（非子反推父）⟹ 子声部可激活。

    超越上浮的 A/B/C：A 改 T_i（冲击 π^cov bit-exact）被规避（D 不改 T_i 另建 K_i）；
    B 放宽 hostOf 到区间包含（违 638）被否（D host^op 仍严格右端点命中）；
    C 接受退化定理被否（子声部=0 是 host 宇宙不足=可修，非结构必然）。

    非 workaround（PDF §11：不伪父链/不级别差/不区间包含）= 真扬弃。
    实装 = task#14（K_i 操作 carrier forest）。结算分类=视图分离（已定，编排者价值判断）；
    settled 落盘携带概念广播=019c 编排者权，走 /ritual。genealogist 记录裁决，不自落盘。
  decided_by: 编排者   # chat「子声部.pdf」27页严格推导（价值判断/方案选择=选择类，编排者权）

negated:
  description: "(1) 676 三根因清单完备。(2)【原本号倾向 C】子声部=0 是 P1(单最高级降序树)∧P2(严格右端点命中) 的不可避免定理（结构必然，不可修）。(3)【645 倾向】π^bsp 子声部层是 wrong object（goal#5 多声部对冲是本质错误问题）。(4) 修复必在 A/B/C 三方向中三选一。"
  why_negated: "(1) 本号实装 676 全部修复路径子声部仍=0（第四根因 extract_elements 树覆盖局限）。(2) 编排者裁决 D：第四根因真因是 host^op=host^struct 混用（可修），非 P1∧P2 不可避免——分离 K_i(endpoint-complete) 即可修，C 被否。(3) 裁决 D 推翻 wrong object 倾向：子声部=0 是 host 宇宙不足（K_i 未建），非 goal#5 本质错误；命题A(645)π^cov≠π^bsp 仍成立，但 π^bsp 子声部层是可激活的真对象（待 K_i 实装）。(4) 编排者经「子声部.pdf」27页给出第四方案 D（视图分离），超越三选一——A/B/C 都在『单一 host 宇宙』前提下，D 否定该前提（双视图）。"

new_output:
  definitions:
    - "【裁决 D】host^op ≠ host^struct（双视图分离）：结构视图树 T_i（↓r_i 单最高级降序，P1，π^cov bit-exact）⊥ 操作 carrier forest K_i（endpoint-complete，host^op 定义域）。"
    - "【裁决 D】子声部=0 根因精确化 = host^op=host^struct 混用（把 T_i 当 host^op 宇宙），非 P1∧P2 不可避免定理。可修（建 K_i），非结构必然。"
    - "【裁决 D】host^op 保留严格右端点命中（P2a，不违 638），只改 host 宇宙 T_i→K_i（P2b）。"
    - "【裁决 D §10 定理1】endpoint-complete K_i + 解释器生成父 voice（非子反推父）⟹ 子声部可激活。"
    - "676 修复路径（按 carrier 匹配 + host-miss 不丢弃）是必要非充分（缺 K_i 视图）；第四根因（extract_elements 树覆盖 36-42%）经裁决 D 归因为 host 宇宙错配（用 T_i 而非 K_i），可修。"
  code_changes: |
    rust/src/bin/pi_bsp_timing.rs（已落 676 修复，128+ / 52−）：carrier 改真嵌套塔，host-miss 退化 ∂ 根，出场证书按 carrier 配对。
    待实装（task#14，裁决 D）：K_i 操作 carrier forest（endpoint-complete）作 host^op 定义域；
    host^op 严格右端点命中映射到 K_i（不改 T_i / coverage.rs:218 铁律 / π^cov bit-exact）；
    解释器生成父 voice（§10 定理1）。
  orchestration_changes: |
    方法论：①上浮的 A/B/C 三选一可能共享一个未察觉的前提（单一 host 宇宙）——编排者裁决 D
    否定该前提（双视图）超越三选一。蜂群上浮时应标注『三方向是否共享隐藏前提』。
    ②『接受退化为定理』（C）前须核根因是否真不可避免——本号原倾向 C（P1∧P2 不可避免），
    编排者证伪（host 混用可修）。把可修的工程错配误判为结构定理 = 过早接受退化（161 务实否定的对偶）。

impact:
  affected_modules:
    - "rust/src/bin/pi_bsp_timing.rs → 676 修复已落；待 task#14 接 K_i 操作 carrier forest（host^op 定义域）。"
    - "rust/src/theta_v0/strategy/coverage.rs:209-223 extract_elements（T_i 结构视图）→ 裁决 D **不改**（保留 P1 + π^cov bit-exact）；另建 K_i（endpoint-complete）。"
    - "K_i 操作 carrier forest（新建，task#14）→ host^op 定义域，endpoint-complete；host^op 严格右端点命中映射到 K_i。"
    - "rust/src/theta_v0/strategy/coverage.rs:405 attach_bsp_carrier_indexed → host^op 改附着到 K_i（不改 638 严格右端点命中语义，只改 host 宇宙）。"
  affected_definitions:
    - "676（生成态）：本号坐实其根因清单不完备（必要非充分）；裁决 D 给出充分解（K_i 视图）。维持生成态待 /ritual（可随 648 一并辨认）。"
    - "645（命题A）：π^cov≠π^bsp **仍成立**；但裁决 D 推翻『π^bsp 子声部层=wrong object』倾向——子声部层是可激活真对象（待 K_i），子声部=0 是 host 宇宙不足非本质错误。645 维持生成态。"
    - "638（settle）：裁决 D host^op 保留严格右端点命中（P2a），**不违 638**（B 被否）。维持 settle。"
    - "547（settle）：裁决 D 非 workaround（不用级别差伪造父子）。维持 settle。"
    - "231（settle）：本号是有效域≠定义域活实例（676 修复 L1 fixture 有效，L2 真实数据子声部=0）；裁决 D 修正有效域（K_i endpoint-complete 扩 host-hit 域）。维持 settle。"
  downstream_implications:
    - "【裁决 D】π^bsp 子声部层是可激活真对象（待 K_i 实装），非 wrong object——goal#5『多声部对冲 alpha』是可测真问题（K_i 建后产生 L2 可证伪实测），非错误问题。推翻 645 的 wrong object 倾向（命题A π^cov≠π^bsp 本身仍成立）。"
    - "task#14（K_i 操作 carrier forest 实装）= 行动类，已 spawn（in_progress）。task#3 l2l3-fullwindow-rejudge 依赖子声部激活——K_i 建后前置条件满足。"
    - "π^cov bit-exact 不受裁决 D 影响（T_i 不动）——D 在操作视图侧（K_i）新建，结构视图侧（T_i/π^cov）保留。"

related_records:
  parent: "676（π^bsp 子声部=0 实装根因；本号坐实不完备 + 编排者裁决 D 给充分解）"
  children: []
  related:
    - "644（parent carrier 注入 gap——本号坐实注入已落但用错 host 宇宙 T_i；裁决 D 修为 K_i）"
    - "645（命题A π^cov≠π^bsp 仍成立；裁决 D 推翻其 wrong object 倾向——子声部层可激活）"
    - "638（settle，host^op 保留严格右端点命中，不违）"
    - "547（settle，非 workaround 不用级别差）"
    - "231（有效域≠定义域——676 修复 L1 有效 L2 失效；裁决 D K_i 修正有效域）"

epistemological_levels:
  - proposition: "676 修复路径（按 carrier 匹配 + host-miss 不丢弃）实装后子声部仍=0（OKLO/CL/BTC）"
    level: "L2（pi_bsp_timing.rs 实测三标的：host-hit 2.4%-7.7%，parent_active 恒 0）"
    increment: "高：676 根因清单不完备的 L2 坐实（修复后行为未翻转）"
  - proposition: "子声部=0 根因 = host^op=host^struct 混用（非 P1∧P2 不可避免定理），可修"
    level: "L0（编排者「子声部.pdf」27页严格推导：双视图分离 + §10 定理1）"
    increment: "高：根因从『结构必然(C)』修正为『host 宇宙错配(可修)』——证伪过早退化"
  - proposition: "解 D：分离 T_i(结构视图,P1,π^cov bit-exact) ⊥ K_i(操作 carrier forest,endpoint-complete,host^op 定义域)，host^op 保留严格右端点命中(P2a,不违638)"
    level: "L0（「子声部.pdf」§10/§11 严格推导：非 workaround=真扬弃）"
    increment: "高：第四方案 D 超越 A/B/C 三选一（否定单一 host 宇宙隐藏前提）"
  - proposition: "endpoint-complete K_i + 解释器生成父 voice(非子反推父) ⟹ 子声部可激活"
    level: "L0（「子声部.pdf」§10 定理1）+ 待 L2（task#14 K_i 实装后实测，不预设结果）"
    increment: "高：子声部可激活性的 L0 判定；L2 待 K_i 实装坐实"
---

# 648 π^bsp 子声部=0【编排者裁决 D 视图分离消解】

## 一句话结论（裁决后）

编排者经 chat「子声部.pdf」27页严格推导裁决 **方案 D = 视图分离**（超越上浮的 A/B/C 三选一）：
子声部=0 根因精确化 = **host^op=host^struct 混用**（非 P1∧P2 不可避免定理）。解 D 分离结构视图树
**T_i**（↓r_i 单最高级降序，保留 P1，π^cov bit-exact 不动）与操作 carrier forest **K_i**
（endpoint-complete，host^op 定义域）；host^op 仍严格右端点命中（保留 P2a，**不违 638**），
只改 host 宇宙 T_i→K_i（P2b）。**非 workaround**（PDF §11：不伪父链/不级别差/不区间包含）= 真扬弃。
§10 定理1：endpoint-complete K_i + 解释器生成父 voice（非子反推父）⟹ 子声部可激活。

## 裁决 D vs 上浮 A/B/C

| 方案 | 内容 | 裁决 |
|------|------|------|
| A | 改 extract_elements 多级森林 | 超越——A 改 T_i 冲击 π^cov bit-exact；D 不改 T_i，另建 K_i |
| B | 放宽 hostOf 到区间包含 | 否——违 638；D host^op 仍严格右端点命中 |
| C | 接受退化为定理（子声部结构性不可达） | 否——根因是 host 宇宙不足（可修）非 P1∧P2 不可避免 |
| **D** | **视图分离 T_i ⊥ K_i** | **裁决**——否定单一 host 宇宙隐藏前提（A/B/C 共享） |

三方案 A/B/C 共享隐藏前提『单一 host 宇宙』；D 否定该前提（host^op≠host^struct 双视图）。

## 扬弃结构（aufhebung）

- **否定**：host^op=host^struct 混用（把结构视图树 T_i 当操作 host 宇宙）。
- **保留**：P1（T_i 单最高级降序）+ P2a（host^op 严格右端点命中，不违 638）+ π^cov bit-exact。
- **提升**：host^op≠host^struct 双视图——T_i（结构/π^cov）⊥ K_i（操作 carrier forest，endpoint-complete）。

## 对 645 命题A 的影响

命题A（π^cov≠π^bsp）**仍成立**。但裁决 D **推翻**『π^bsp 子声部层=wrong object』倾向：
子声部=0 是 host 宇宙不足（K_i 未建，可修），**非** goal#5 本质错误。π^bsp 子声部层是可激活真对象
（待 task#14 K_i 实装），goal#5 多声部对冲 alpha 是可测真问题（K_i 建后产生 L2 可证伪实测）。

## genealogist 边界声明

genealogist 记录编排者裁决 D（contradiction→视图分离，已结算），不自落盘 settled（019c）。
本号是 chat「子声部.pdf」27页裁决的结构记录。实装 = task#14（K_i），已 spawn。

## 张力检查（019d/020）

### 检查范围
同轮（642-648）∪ 1-hop（676/644/645/638/547/231）∪ Hub（645 覆盖/择时对象、231 有效域、638 hostOf）。

### 张力1：vs 676（parent）——裁决 D 给充分解
676 三根因必要非充分；本号坐实第四根因；裁决 D（K_i 视图）给充分解。本号是 676 的充分化。无矛盾。

### 张力2：vs 645——推翻 wrong object 倾向，命题A 仍成立
裁决 D 推翻 645『π^bsp 子声部=wrong object』倾向（子声部层可激活，待 K_i），但命题A（π^cov≠π^bsp）本身仍成立。645 维持生成态。无矛盾（D 否定的是 645 的一个倾向性下游推论，非命题A 主体）。

### 张力3：vs 638/547（settle）——保留，不违
host^op 保留严格右端点命中（P2a，不违 638）；非 workaround（不用级别差，不违 547）。裁决 D 明确保留两 settle。无矛盾。

### 张力4：vs 231（settle）——活实例 + 修正
676 修复 L1 fixture 有效、L2 真实失效=231 活实例；裁决 D（K_i endpoint-complete）修正有效域。印证 231。无矛盾。

### 概念分离信号检测（中断 #1）
裁决 D 是视图分离（host^op≠host^struct），各视图定义域清晰可分层（T_i 结构 ⊥ K_i 操作）。无不可分层定义矛盾 ⟹ **不触发中断 #1**。genealogist 记录裁决，不发 SendMessage（已由 Lead 消息触发本次更新）。

### 递归运动结构完成检测（020）
- 第0层：本号改判写入（裁决 D 视图分离 + 扬弃结构 + 对 645 影响）。
- 第1层：本号 × 676 → 充分解（净新发现高：第四根因从结构必然修正为 host 宇宙错配可修）。
- 第2层：本号 × 645 → 推翻 wrong object 倾向（净新发现中：子声部层可激活）。
- 第3层：本号 × 638/231 → 保留 settle + 有效域修正（净新发现降：已知约束保留）。
- scope₁(裁决D充分解) > scope₂(推翻wrong object) > scope₃(settle保留)=顶分型。
- **背驰 ∧ 分型 ⟹ 递归运动结构性完成。** 已结算（裁决 D），settled 落盘待编排者 /ritual；K_i 实装=task#14（行动类，已 spawn）。

## 回溯扫描（职责3）

- **676（生成态，parent）**：裁决 D 给充分解；维持生成态待 /ritual（可随 648 一并辨认）。
- **645（生成态）**：命题A 仍成立，wrong object 倾向被推翻；维持生成态。
- **644（生成态）**：注入已落但用错 host 宇宙（T_i），裁决 D 修为 K_i；维持生成态。
- **638/547/231（settle）**：裁决 D 保留（P2a 不违 638 / 非 workaround 不违 547 / 231 活实例+修正），不否定。维持 settle。
- **无 settled 被本号回溯破坏。** 本号是编排者裁决 D（视图分离）的结构记录，已结算，settled 落盘待 /ritual。
