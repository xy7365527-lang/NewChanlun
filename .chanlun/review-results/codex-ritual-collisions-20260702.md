# codex 裁决③：三组谱系编号碰撞（576×2 / 644×2 / 647×2）统一编号空间方案

- 日期：2026-07-02
- 模式：codex decide（codex-cli，ChatGPT 订阅通道）
- 完整交互记录（prompt+response 原文）：`.chanlun/review-results/codex-decide-20260702-160223-3b46.md`
- 委托来源：task #26，编排者 2026-07-02 明令「要裁决的全问codex」
- 任务边界：**仅裁决方案，不执行改号**（改号是下游行动，待 Lead 派发）

## 一、结论（三组判决）

裁决原则：**dag.yaml 官方注册优先，孤儿单调续号**。当前最大已用编号 = 673
（dag.yaml + pending/ + settled/ 三路交叉核实），674/675/676 均空闲（已复核）。

| 冲突号 | 保留原号者（dag.yaml 官方注册） | 孤儿 → 新号 |
|---|---|---|
| 576 | `turning-node-topology-strength-orthogonal-product-20260624.md`（E⊗F 转折节点正交双源，重大概念分离） | `576-ledger-r-vs-tw-three-stage-semantic-alignment.md` → **674** |
| 644 | `644-acceptance2-four-layer-penetration-...-parent-carrier-injection-gap.md`（14166 坐标系伪证一阶诊断） | `644-meta-rule-probe-must-traverse-production-path-no-coordinate-fork.md` → **675**，独立保留 |
| 647 | `647-meta-observation-object-identity-must-precede-falsification-design.md`（否证设计前置同一性，二阶元规则） | `647-pibsp-subvoice-zero-structural-not-fixture-multivoice-hedge-claim-inflation.md` → **676** |

### 644 组核心争点判决：675 与 647 是父子/泛化关系，两者都保留

644-meta-rule-probe（→675）**不是** 647-object-identity 的重复早期草稿：
- 675 = 具体工程元规则实例（"探针必须走生产路径，禁止坐标系分叉"）
- 647 = 更高阶抽象（"否证设计前置同一性"，统一 644 坐标系亚型 + 645 投影亚型）
- 合并会丢失 pattern-buffer frequency 追踪（diagnostic-coordinate-phantom.yaml 2/3）
  和具体工程诊断约束。675 保留时应标注"已被 647-object-identity 泛化引用"。

### 576 组与 task #24 的协调

task #24（已完成，`codex-ritual-choices-20260702.md`）对 576-ledger 的**内容**
裁决为立场 C（LedgerState+TWState 双层并置）。本裁决只处理编号唯一性，与内容
裁决正交。协调方式：task #24 产出及下游实装任务（task #31 "576=C 实装"）落库时
引用应统一改用 **674-ledger-r-vs-tw**。

### 647 组标准引用后缀

正式采用 `647-object-identity` 作为 647 号的 canonical 引用后缀（prose/related/
歧义环境用），数值 id 仍是 647。649 号文件已自发使用此形式，无需改。

## 二、改号需同步更新的全部引用点

### dag.yaml（`.chanlun/genealogy/dag.yaml`）
1. 保留 576/644/647 三个官方条目不动
2. 新增注册 674、675、676 三个条目（file 字段指向改名后的文件）
3. 修复数据质量缺口：id '576'（如缺）、'647'、'649' 的空 title 同轮补全
4. 647 条目的 depends_on 中指向 meta-rule-probe 的 "644" → "675"
   （已坐实：647 文件第 20 行 related 写明"644=姊妹元结构的上位抽象"，
   该 644 指元规则即 meta-rule-probe，非一阶事件文件——一阶事件引用保留 644）
5. 648 条目 depends_on："647" → "676"；如改文件名则同步 file 字段
6. `647-WITHDRAWN` 死条目保持原样，不参与本轮

### 谱系文件（`.chanlun/genealogy/pending/`）
7. `576-ledger-r-vs-tw-three-stage-semantic-alignment.md`
   → 改名 `674-ledger-r-vs-tw-three-stage-semantic-alignment.md`，frontmatter id → "674"
8. `644-meta-rule-probe-must-traverse-production-path-no-coordinate-fork.md`
   → 改名 `675-meta-rule-probe-...md`，frontmatter id → "675"，
   增加与 647-object-identity 的泛化关系标注
9. `647-meta-observation-object-identity-...md`：保留 id "647"；
   文内指向 meta-rule-probe 的 "644" 引用改写为 "675"（一阶事件引用保留 644）
10. `647-pibsp-subvoice-zero-...md`
    → 改名 `676-pibsp-subvoice-zero-...md`，frontmatter id → "676"
11. `648-pibsp-647-fix-insufficient-fourth-root-extract-elements-tree-coverage.md`
    → depends_on `["647",...]` → `["676",...]`；正文所有指向 pibsp-subvoice 的
    裸 "647" → "676" 或 "676-pibsp-subvoice-zero"；建议文件名同步改为
    `648-pibsp-676-fix-...md` 并更新 dag file 字段
    （648 标注已结算但物理仍在 pending/——独立数据质量缺口，可同轮修，非前置）
12. `649-meta-observation-...md`：已用 "647-object-identity" 限定形式，无需改

### 活引用 vs 历史快照
13. `.chanlun/review-results/ritual-queue-20260702.md`：活跃队列，更新为本裁决映射
    （647 拆为 647-object-identity 与 676-pibsp；补 674/675 映射）
14. `.chanlun/sessions/*.md`、历史 review-results：历史快照，**不回溯修改**，
    映射关系以本文件为准
15. `pattern-buffer/diagnostic-coordinate-phantom.yaml` 的 source_genealogy ["644"]：
    经核实指向 644-acceptance2（官方持有者，一手诊断记录），**无歧义，不需要改**
16. `BACKLOG-CLASSIFICATION-20260629.md` 中 "647-pibsp-subvoice" 条目：
    活跃分类报告，如仍被消费则同步为 676-pibsp-subvoice

## 三、代理质询判定（简化质询：裁决成立）

1. 问题是否真实存在？——是。648 号 depends_on 裸 "647" 已产生实际歧义
   （指向孤儿而非官方持有者）；649 号被迫自发使用限定后缀规避。
2. 是否已被其他机制覆盖？——否。ritual-queue 只有问题清单无方案；
   task #24 只裁内容不裁编号。
3. 严重性判定是否合理？——合理。Codex 指出的最大风险（648 裸 647 漏改导致
   parent 指向错误）是实际可发生的引用断裂。
4. 增量坐实：Codex 对 647→675 引用改写给的是条件式建议（"若指向 meta-rule-probe
   则改"），本代理已核实该条件为真（647 文件 related 第 20 行"姊妹元结构的
   上位抽象"表述只能指元规则），条件式升级为确定性同步点（上方第 4/9 条）。

## 四、边界条件

- 执行前若编号空间已被并发 teammate 消费超过 673，674/675/676 整体顺延到
  新的连续空号段，保持三孤儿相对顺序（ledger→probe→pibsp）
- 若 dag schema 不允许字符串后缀引用，`647-object-identity` 只作 canonical
  alias（title/slug/prose），不替代数值 id
- 若编排者对 644 组另裁"合并"（推翻父子关系判定），则 675 号回收、
  pattern-buffer 追踪需迁移到 647 条目下——此为唯一可翻转点

## 五、影响声明

- 本文件本身不改任何谱系文件/dag.yaml——纯裁决方案
- 下游执行改号将触及：dag.yaml（6 处）、pending/ 5 个文件（3 改名 + 2 文内引用）、
  ritual-queue-20260702.md、（可选）BACKLOG-CLASSIFICATION-20260629.md
- 涉及任务协调：task #31（576=C 实装）落库引用应用 674
