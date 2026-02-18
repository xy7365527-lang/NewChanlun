# 022 — 比价关系与等价关系的形式化

**状态**: 已结算
**类型**: domain
**日期**: 2026-02-18
**结算日期**: 2026-02-18
**结算方式**: /ritual（编排者决断 — 选择类）
**溯源**: [旧缠论] 第9课（比价变动构成独立买卖系统）+ [旧缠论:隐含] + [新缠论]
**域**: 比价关系（第二阶段入口）

---

## 矛盾描述

ontology-v1 命题5 声明"区间套从时间级别扩展到空间级别"，并给出形式化路径：A/B 比价直接除 → K线 → 走势 → 缠论结构。但这条路径缺少严格定义：

1. **比价K线的构造规则**未形式化 → ✅ equivalence.py + ratio_relation_v1.md §1
2. **等价关系未定义** → ✅ equivalence.py (validate_pair) + ratio_relation_v1.md §2
3. **比价走势语义未规范化** → ✅ capital_flow.py + ratio_relation_v1.md §1.2
4. **四矩阵拓扑未落地** → ✅ matrix_topology.py + ratio_relation_v1.md §3

## 结算理由

四个矛盾点全部有：规范（ratio_relation_v1.md）+ 实现（4个模块）+ 测试（82个）+ E2E 验证（IR-3 完备性）。

编排者决断：
1. 比价和等价是跨标的维度的基础概念，需要独立定义文件（与笔、线段同级）
2. 规范升级需走 /ritual（结晶必须经过谱系记录已结算）

## 发现过程

第一阶段完成度评估触发。1140 测试全过，9 个定义全结算，区间套（时间维度）已实现。自然进入第二阶段入口：命题5 的空间维度扩展。

## 产出

### 定义层（/ritual 结算）
- `.chanlun/definitions/bijia.md` v1.0 — 比价关系定义（跨标的维度基础概念）
- `.chanlun/definitions/dengjia.md` v1.0 — 等价关系定义（跨标的维度基础概念）

### 规范层
- `docs/spec/ratio_relation_v1.md` — 正式规范 v1.0（从设计稿升级）

### 实现层
| 模块 | 功能 | 测试 |
|------|------|------|
| `equivalence.py` | 等价对验证 + 比价K线构造 | 17 |
| `ratio_engine.py` | 多对比价并行分析调度 | 15 |
| `capital_flow.py` | 比价走势→资本流转语义映射 | 18 |
| `matrix_topology.py` | 四矩阵拓扑管理 | 28 |
| `test_ratio_pipeline.py` | 比价K线E2E管线验证 | 4 |

总计：82 测试，全量套件 1218 passed

## 推导链

1. ontology-v1 命题1：缠论 = 资本运动的形式语法
2. ontology-v1 命题5：同一语法适用于比价K线
3. 第9课：比价变动构成独立买卖系统，与资金流向相关
4. 第一阶段管线已完备 → 比价K线可直接输入已有管线
5. 但管线只处理数据结构，缺概念层的语义定义
6. ∴ 需要形式化比价关系和等价关系，才能使管线输出携带资本流转含义
7. 四个缺口（构造规则+等价关系+语义映射+四矩阵）均已有规范+实现+测试
8. 编排者决断：加定义文件 + 走 /ritual → 结算

## 谱系链接

- **前置**: 020-constitutive-contradiction（ontology-v1 命题5 是本条的起点）
- **前置**: 已有管线的所有定义（fenxing, bi, xianduan, zhongshu, zoushi, beichi, maimai, level_recursion, baohan）
- **关联**: 已有实现 src/newchan/synthetic.py::make_ratio()（已整合为委托）

## 影响

- `.chanlun/definitions/bijia.md` — 新建
- `.chanlun/definitions/dengjia.md` — 新建
- `docs/spec/ratio_relation_v1.md` — 从设计稿升级为正式规范
- `src/newchan/equivalence.py` — 新建
- `src/newchan/ratio_engine.py` — 新建
- `src/newchan/capital_flow.py` — 新建
- `src/newchan/matrix_topology.py` — 新建
- `src/newchan/synthetic.py` — make_ratio 重构为委托

## 来源

- [旧缠论] 第9课："比价关系的变动，也可以构成一个买卖系统，这个买卖系统是和市场资金的流向相关的"
- [旧缠论] 第72/73课：比价关系为三个独立系统之一
- [旧缠论:隐含] 比价K线构造（买卖系统需要K线，比价变动K线化是逻辑必然）
- [新缠论] ontology-v1 命题5：区间套空间扩展
- [新缠论] 等价关系严格定义（本规范首次形式化）
- [新缠论] 卢麒元四矩阵拓扑
