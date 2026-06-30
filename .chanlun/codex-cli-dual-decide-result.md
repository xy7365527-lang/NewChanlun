# codex CLI 双裁决结果（2026-06-29）

异质源：OpenAI Codex CLI v0.125.0（model=gpt-5.5, reasoning=xhigh）。
之前 codex-challenger subagent 撞 OpenAI API 429（insufficient_quota）；codex CLI 有独立认证/额度，本轮成功调用。
调用方式：`codex exec --skip-git-repo-check -c 'service_tier="fast"'`（config.toml 的 `service_tier="priority"` 是无效枚举，须覆盖；API 不支持 flex，fast 可用）。

## 裁决1：§9 same-unit axiom vs rust depth_weight（谱系646声称的定义冲突）

**verdict：C（范畴错误 / 消解）**

codex 独立结论（与代理侧倾向一致）：

- `rust leg.units = base_units * depth_weight` 是 **不同的/下游量**——theta_v0 的资本加权目标敞口，**不是** §9 的 `q_v` 手数。
- §9 公理是**条件式** `a_v=1 ⟹ q_v=q_parent`，治理 active voice-state 的手数恒等与 §11 notional 投影下的精确对冲抵消。
- `depth_weight` 是可调资金帽/设计参数，用于 sizing/target 投影。不等权重本身**不违反** §9。
- 唯有把 `leg.units` 重释为 `q_v` 时，rust 才会在跨 depth 对冲对上违反 §9——**那个重释本身就是范畴错误**。

下游推论：
- acceptance ΔSharpe 检验的是 theta_v0 加权帽策略行为，**不是**"忠实 §9 精确抵消 coverage"。
- pivot 到 `π^bsp`（离散择时层）**真正消解**此冲突；§9 仅在 BSP 决策被提升进 voice-state sizing/coverage 时才重现。

## 裁决2：barspec bar_seconds 归属

**verdict：方案1（Dataset）**，真概念边界，非 bikeshedding。

codex 独立结论（与 Lead 初判一致）：

- bar_seconds 属于 Dataset/数据加载元数据，不属于 ThetaConfig。
- ThetaConfig 已被定义为"可扫描的 Θ 外部参数空间"（Phase 6 sweep 目标）；谱系 #232 把 ConfigurationSpace 限定为方向自由度空间。把 bar_seconds 放进去 = 把"输入数据采样率"误建模成"算法可自由组合旋钮"。
- 扫描器尚未实装**不改变**这个语义契约，只是错误暂时未被执行出来。

正确性复核：
- `load_by_symbol(..., 60)` 在当前 SYMBOLS 全 1min 下可接受，但应标注为"当前数据清单不变量"；一旦混入 1s/多粒度文件，应从 manifest/SYMBOLS 粒度列读取。
- 真正的正确性缺口：下游仍硬编码年化分钟数，需改成 `bars_per_year(ds.bar_seconds)`（A 点 wiring 未完成）。

## 认识论等级

两裁决均为概念层判断（L0/定义层），由异质源（codex CLI）独立产出，未伪造同质判断冒充。
