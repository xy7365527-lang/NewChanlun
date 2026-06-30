---
id: "648"
number: 648
status: 生成态   # 矛盾上浮，待编排者裁决修复方向（扩展 forest vs 重定义 hostOf 对应 vs 接受退化）。
date: "2026-06-29"
type: contradiction   # 647 提的修复路径已实装但子声部仍=0——647 根因清单不完备，缺第四根因。
depends_on: ["647", "644", "231", "090"]
related: ["638", "645", "646", "640"]
title: "π^bsp 647 修复（按 carrier 匹配 + host-miss 不丢弃）已实装但子声部仍恒=0——647 三根因清单不完备：第四根因 = extract_elements 单最高级降序树仅覆盖 ~36-42% L0 走势，bsp 点 92-98% 落树外（orphan frontier 段）⟹ host-miss ⟹ ∂ 根；少数 host-hit 的父容器又无 active 声部（parent_active=0）⟹ 联合条件（host-hit ∧ parent-container-voice-active）真实数据上结构性为空"
negation_source: "源码事实 L2（pi_bsp_timing.rs 实测 OKLO/CL/BTC）：647 提的修复路径（open_new 按 carrier 匹配 + 子声部反向证书 host-miss 不丢弃）已落实装，子声部仍=0。诊断计数器坐实：host-hit 率 OKLO 2.4%（23/966）/ CL 6.1%（24/395）/ BTC 7.7%（28/363）；三标的 parent_active_found 恒=0。tower 各级 vs extract_elements 树覆盖（OKLO@mid）：tower L0=1360 走势 → tree 仅 486（36%），L1=399→162（41%）。"
negation_form: "expansion"   # 647 根因域扩张——三根因 → 四根因（新增 extract_elements 树覆盖局限）。

contradiction:
  description: |
    647 诊断 π^bsp 子声部=0 根因为三项（均在 pi_bsp_timing.rs bin 内）：
    (a) §11 close 优先（P3 出场）在开仓前平掉反向 active 根声部；
    (b) open_new 内 active_root_by_carrier.find 只比方向不比 carrier；
    (c) buy/sell 分两循环但 active 快照只取一次。
    647 给出修复路径：「open_new find 按 carrier 匹配 + 子声部反向证书在 P3 出场前评估
    （或区分『出场反向证书』与『开子声部反向证书』语义）」。

    本号实装了 647 的修复路径，并按生产路径（interp.rs:498 coverage_elements_with_tower）
    对齐——用 attach_bsp_carrier_indexed + build_tree_endpoint_index 建结构父子（carrier =
    hostOf 容器 ElementId，parent_id = 真 Compose 父容器，host-miss 退化为 ∂ 根而非丢弃；
    出场证书改为严格按 carrier 配对，反向证书在子 carrier 开子声部而非平父声部）。

    **修复后子声部仍恒=0**（OKLO/CL/BTC 三标的）。诊断计数器坐实第四根因：
    - host-hit 率 2.4%-7.7%——92-98% 的 bsp 点在 extract_elements 树里找不到 hostOf。
    - 即使 host-hit（24/CL），其中 19 有真 Compose 父容器，但**父容器上有 active 声部 = 0**。
    - 联合条件（host-hit ∧ parent-container-voice-active）在真实数据上结构性为空。

    第四根因的结构来源：extract_elements（coverage.rs:209-223）只展开 tower **最高非空级别**
    作为根（line 218-219 `break`），更低级别仅由 Compose.subs 真嵌套带出。故 tree 只覆盖
    「能从最高级 compose 链降序到达的走势」——OKLO@mid 仅覆盖 L0 走势的 36%（486/1360）。
    其余 64% 的 L0 走势是「orphan frontier 段」（尚未被组合进最高级的近期未确认前沿），
    不在 tree 中。bsp 点跨**全格**（classification.levels[*].bsp 含所有级别端点），
    大多落在 orphan 段上 ⟹ tree_idx.get((level, source_index)) 未命中 ⟹ ∂ 根声部。

    **矛盾形式**：两个均被声明正确的对象不可同时满足子声部激活——
    - extract_elements 约定：tree = 单最高级降序（siblings/orphan frontier 排除，coverage.rs:218 铁律）
    - π^bsp / 信号层约定：bsp 点跨全 segment 格（所有级别 + orphan frontier）
    carrier 父子桥只能附着落在降序树内的 bsp（~2-8%）；其余结构性孤立为 ∂ 根。
    子声部要求 host-hit ∧ 父容器有 active 声部——后者要求父走势容器自身携 bsp 开的声部，
    而高级走势容器同 bar 罕有 bsp ⟹ 联合条件空。

  layer: 实装   # 非定义冲突（§3 声部树定义 + 638 hostOf 定义本身无矛盾）；矛盾在两个实装约定（extract_elements 树构造 vs bsp 全格）的有效域不交叠。
  trigger: "task#2 subvoice-tower-bridge 实装 647 修复路径后实测子声部仍=0；诊断计数器坐实 host-hit 率 + parent_active=0。"

resolution:
  type: 待结算   # 矛盾上浮，修复方向需编排者裁决。
  description: |
    本号坐实 647 根因清单不完备（缺第四根因）。647 的三根因修复**必要但不充分**——
    本号已实装该修复（按 carrier 匹配 + host-miss 不丢弃 + 出场证书按 carrier 配对），
    子声部仍=0，因为联合条件（host-hit ∧ 父容器有 active）真实数据结构性为空。

    三个互斥的修复方向（编排者裁决，禁 workaround 强造子声部）：
    (A) extract_elements 改为**多级森林**：所有 tower 级别的走势都作根（不只最高级），
        orphan frontier 段也进树 ⟹ host-hit 率升。代价：tree 规模膨胀（L0 全量 1360 而非 486），
        且改 extract_elements 会冲击生产 π^cov 路径 bit-exact（coverage.rs 核心，须重测全 lib）。
        风险：可能违反 coverage.rs:218「不重复作根」铁律（547 级别差伪造的否定来源）。
    (B) 重定义 hostOf 对应：bsp 点不要求 source_index == tree 元素 rho，改用区间归属或
        最近祖先映射。代价：违反 638「严格右端点命中非区间包含」settle（638 漏洞②的精确否定）。
    (C) 接受退化为定理：π^bsp 在「单最高级降序树 + bsp 全格」约定下，子声部结构性不可达
        是定义的必然推论，非缺陷。则命题A（645）的「可分离」永久标 L0 代数可分（公式不同），
        多声部对冲层在此约定下不存在 L2 实测——goal#5 的「多声部对冲 alpha」是错误问题
        （645 已部分指向「wrong object」）。

    禁止的 workaround（no-workaround）：伪造父链、用级别差当父子（547 否定）、
    或放宽 hostOf 到区间包含（638 否定）来强行造非零子声部。

  decided_by: 待裁决

negated:
  description: "(1) 647 三根因清单完备（按 carrier 匹配 + P3 前评估即可激活子声部）。(2) 『嵌套塔喂子声部』能在真实数据上激活 §16 多空双开/短差对冲层。"
  why_negated: "(1) 本号实装 647 全部修复路径，子声部仍=0；诊断坐实第四根因（extract_elements 树覆盖局限）。(2) host-hit 率 2-8% + parent_active=0 ⟹ 联合条件结构性空，子声部不可达。"

new_output:
  definitions:
    - "647 修复路径（按 carrier 匹配 + host-miss 不丢弃）是子声部激活的**必要非充分**条件。"
    - "第四根因：extract_elements 单最高级降序树仅覆盖 ~36-42% L0 走势；bsp 点 92-98% 落 orphan frontier 段（树外）⟹ host-miss ⟹ ∂ 根。子声部联合条件（host-hit ∧ 父容器有 active）真实数据结构性为空。"
    - "host-hit 率（L2 实测）：OKLO 2.4% / CL 6.1% / BTC 7.7%；parent_active 恒 0（三标的）。"
  code_changes: |
    rust/src/bin/pi_bsp_timing.rs（128+ / 52−）：
    - carrier 从伪 (level, source_index) → hostOf 容器 ElementId（644 坐标纪律，生产 interp.rs:498 同口径）。
    - 用 extract_elements + build_tree_endpoint_index + attach_bsp_carrier_indexed 建真结构父子。
    - host-miss 退化为 ∂ 根声部（不丢弃，修旧版 continue 丢 92-98% 证书的二级 bug）。
    - 出场证书按 carrier 严格配对（647 修复：反向证书在子 carrier 开子声部，不平父声部）。
    - 子声部可达性诊断计数器 + L2 否定性结果报告（host-hit 率 / parent_active）。
    - 诚实标注：n_children==0 时输出明确声明「Sharpe 仅反映纯根声部退化，不得用作对冲无 alpha 的 L2 结论」。
  orchestration_changes: |
    方法论：①根因诊断须验「修复后行为是否翻转」——647 三根因诊断未实装验证即提修复路径，
    本号实装坐实其不完备（修复后子声部仍=0）。②形式化桥接（carrier 父子）的有效域须 L2 实测
    （host-hit 率），不能从 L0/L1 fixture（生产测试用人造 source_index==rho）推断真实数据有效域
    （231 有效域≠定义域的活实例）。

impact:
  affected_modules:
    - "rust/src/bin/pi_bsp_timing.rs → carrier 父子改真嵌套塔（647 修复已落）；子声部仍=0（第四根因）。"
    - "rust/src/theta_v0/strategy/coverage.rs:209-223 extract_elements → 第四根因结构来源（单最高级降序树）；修复方向 A 须改此处（冲击 π^cov bit-exact）。"
    - "rust/src/theta_v0/strategy/coverage.rs:341 attach_bsp_to_tree / :405 attach_bsp_carrier_indexed → hostOf 严格右端点命中（638）；修复方向 B 须改此处（违 638 settle）。"
  affected_definitions:
    - "647（生成态）：本号坐实其根因清单不完备（三 → 四根因），其修复路径必要非充分。"
    - "645（命题A）：本号坐实『可分离』在此约定下永久 L0 代数可分（子声部结构性不可达 ⟹ L2 实测可分不可达），除非裁方向 A/B。"
    - "638（settle）：修复方向 B 会违反其 hostOf 严格右端点命中——禁。"
    - "547（settle）：修复方向若用级别差伪造父子会违反其铁律——禁。"
    - "231（settle）：本号是有效域≠定义域活实例（carrier 父子桥定义域=全 bsp，有效域=树内 2-8%）。"
  downstream_implications:
    - "π^bsp 在裁决前，CL/BTC/OKLO Sharpe（均≤0）只反映纯根声部退化，**不得**用作『多声部对冲无 alpha』的 L2 结论（与 647 同结论，本号补结构根因）。"
    - "goal#5『多声部对冲 alpha』在当前约定下是错误问题（645 wrong object 的延伸）——须先裁 extract_elements 树构造（方向 A/B/C）才能产生 L2 可证伪的多声部对冲实测。"
    - "task#3 l2l3-fullwindow-rejudge 依赖子声部激活——本号坐实子声部结构性不可达 ⟹ #3 在裁决前无法产生『子声部激活后』的重判（前置条件未满足）。"

related_records:
  parent: "647（π^bsp 子声部=0 实装根因，本号坐实其不完备）"
  children: []
  related:
    - "644（parent carrier 注入 gap——本号坐实注入已落但联合条件空）"
    - "645（命题A wrong object——本号补结构根因）"
    - "231（有效域≠定义域——carrier 父子桥活实例）"
