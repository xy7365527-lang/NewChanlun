# T(S) 一致性检验报告

谱系引用：366号下游推论3、350号收敛紧度定义。
认识论等级：L0（结构差异推导） + L1（管线正确性） + L2 方案（blocked by real data）。

## 1. 被验证的两个度量

### 严格 T(S)（convergence.py — 350号定义）

```
T(S) = D(S) * L(S) * C(S)
```

- D(S)：NestedDivergence chain 中有效层数（Divergence 非 None）
- L(S)：链中最高 level_id
- C(S)：方向一致率（dominant direction count / depth）
- **输入**：NestedDivergence — 需要 nested_divergence_search() 在 RecursiveOrchestratorSnapshot 上实际检测到跨级别背驰嵌套链
- **性质**：乘法；C=0 时整体归零

### 扩展 tightness proxy（scanner_l2_validation.py::_compute_structural_tightness）

```
proxy = BSP(+1.0) + moves(+0.5) + zhongshus(+0.3) + sum(recursive_layers)
```

每递归层：moves → +1.0, 仅 zhongshus → +0.5。

- **输入**：RecursiveOrchestratorSnapshot — 只检查结构存在性，不需要背驰检测
- **性质**：加法；任何结构存在即贡献正分

### 原始 compute_nesting_tightness（stock_scanner.py）

```
tightness = count(levels with buy points)
```

仅统计有买卖点的级别数。更严格但更稀疏（5年日线仅 2/24 标的触发）。

## 2. L0 结论：排序不一致的充分条件

从定义推导，两个度量的结构性差异构成排序不一致的充分条件：

### 差异1：乘法 vs 加法

T(S) = D * L * C 是乘法结构。proxy 是加法结构。

反例：
- 标的 A: T(S) = 5*5*1 = 25（5层全一致）, proxy = 1.8（仅 L1 有结构）
- 标的 B: T(S) = 2*5*1 = 10（2层全一致）, proxy = 4.8（L1 + 3层递归有 moves）
- T(S) 排序: A > B，proxy 排序: B > A

### 差异2：一致性因子

T(S) 包含 C(S)（方向一致率），proxy 不区分方向。

反例：
- 标的 A: 4层全一致 → T = 16, proxy = X
- 标的 B: 4层半一致 → T = 8, proxy = X（同结构 = 同 proxy 分）
- T(S) 排序: A >> B (2x)，proxy 排序: A = B

### 差异3：零 T(S) + 非零 proxy（最频繁场景）

标的有中枢、走势甚至 BSP，但没有跨级别嵌套背驰链 → T(S) = 0, proxy > 0。

这是最常见的情况：nested_divergence_search() 返回空列表时 T(S) = 0，但 _compute_structural_tightness 仍然可以给出正分。

## 3. L1 结论：管线各自正确

12 个测试验证了：
- convergence_tightness() 正确实现 D*L*C 乘法
- None 条目正确排除
- 一致性计算正确
- 单调性：同 level_max 同一致性下 depth 增大 → score 增大
- 空链/全 None 链 → 全零

测试文件：`tests/test_tightness_proxy_consistency.py`

## 4. L2 验证方案（blocked_by: real_data_needed）

### 前置条件

需要 baostock API 获取真实日线数据。scanner_l2_validation.py 已有数据获取管线。

### 实验设计

对 scanner_l2_validation.py 的 24 个标的，同时计算：
1. 扩展 proxy（已有：_compute_structural_tightness）
2. 严格 T(S)（需新增：对每根 bar 的 snap 调用 nested_divergence_search → convergence_tightness）

对比指标：
- Kendall tau rank correlation
- 排序位移量（已有：_compute_rank_divergence）
- 零 T(S) 但非零 proxy 的标的占比

### 预期结果

基于 L0 分析，预期：
1. **大部分标的 T(S) = 0**：跨级别嵌套背驰链是稀疏事件（5年日线24标的中可能仅 2-5 个有非零 T(S)）
2. **proxy 覆盖率远高于 T(S)**：proxy 在结构存在即给分，大部分标的有正 proxy
3. **排序相关性弱**：在非零 T(S) 标的子集中可能有正相关，但全集上相关性被大量 (T=0, proxy>0) 对拉低

### 实现路径

在 scanner_l2_validation.py 的 run_engine_on_symbol 函数中，增加对每根 bar 的 snap 调用 nested_divergence_search，取 max T(S)，与 max proxy 对比。

代码骨架：

```python
from newchan.a_nested_divergence import nested_divergence_search
from newchan.convergence import convergence_tightness

# 在 run_engine_on_symbol 的 for bar in bars 循环中：
nds = nested_divergence_search(snap)
if nds:
    best_nd = max(nds, key=lambda nd: convergence_tightness(nd).score)
    ts_score = convergence_tightness(best_nd).score
    if ts_score > max_ts_score:
        max_ts_score = ts_score
```

### 为什么不能用合成数据（L0 推导）

合成数据构造 NestedDivergence 和结构深度时，两者的相关性完全由构造者控制。
"合成数据的独立性验证是同义反复"（formalization-validity-domain.md）。
只有真实市场数据的结构分布才能回答"proxy 排序近似 T(S) 排序"这个经验命题。

## 5. 影响声明

- **改动**：新增 `tests/test_tightness_proxy_consistency.py`（12 个测试）
- **影响范围**：验证性测试，不修改任何产品代码
- **边界条件**：如果 T(S) 定义变更（350号谱系修改），测试中的期望值需同步更新
- **下游推论**：如果 L2 验证确认排序不一致，scanner_l2_validation.py 中的 proxy 需要重新评估其选股有效性——proxy 可能仍然有用（覆盖率高），但不能声称它 "近似 T(S)"
