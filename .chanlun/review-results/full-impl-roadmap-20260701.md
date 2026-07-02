# 完整缠论策略安装 DAG(impl-architect #124,2026-07-01)

> 编排者裁定:"把整个策略装上再测。全互斥定义策略+完全互斥分类的数学系统,已经实现了很多,只是没有安装。"
> 完整性基准=canonical(strict_hybrid_state_machine_strategy.md S_Θ + FULL_USER_FORMULA_SOURCE.md)→formal/Origin/*.lean→rust/src/theta_v0/。
> 本 DAG = "已 Rust 实装 but 回测未安装" 的接入施工图。L0/L1(代码事实+规划)。

## 审计纠正(trust-but-verify 改 gap 矩阵三处)
1. **P1 塔桥已存在**:assemble_gamma_with_tower(interp.rs:308)已接真嵌套塔,econ_positive.rs:201 已调用。真缺口精确到两点:① nest_confirm(interp.rs:240)只喂单级 chain(base case),未用塔 descend 构造多级 N^δ;② econ 不读 Candidate.nest_confirmed(只读 dir/level/sigma_higher)。工作量 大→中。
2. **P2 缩小**:次级别力度已真算(descend.rs:155 sub_level_type1 消费真 MACD)。P2 简化仅一类背驰门用 MACD 面积代理(divergence.rs:228)。
3. **P1 原料齐备**:LeveledMove(recursive_tower.rs:94)携 start_index/end_index+sub_moves=构造 NestInterval 数据源,全在 tower_i 内。

编排者痛点"都实现了却不实装"精确成立:classifier/nest.rs::NestCertificate::n_delta(方向化完整 N^δ)+strategy/nest.rs::chi_bool(多级完整)都实装,生产链只调单级退化 base case。

## 可并行工位(T0 全部并行,general-purpose)
| 工位 | 补齐 | 工作量 | bit-exact | 依赖 |
|------|------|--------|-----------|------|
| **W1 区间套接入** | P1核心:interp.rs nest_confirm→tower 多级链调 NestCertificate::n_delta;econ 用 nest_confirmed 作准入门 | 中 | 信号集变(预期) | 半阻塞C1(Cand判据) |
| **W2 一类背驰门** | P2:核验 §9.3 四条 vs divergence.rs。知识库对"完整力度背驰"无超越 MACD 面积的可编码定义(§9.3 自认工程简化)→W2=核验/补齐到§9.3四条,忠实则零改动 | 中 | 背驰门变 | 无 |
| **W4 类型透传** | P4最安全:透传已算 bsp_class(interp.rs:79)+买卖侧→signals/SignalDecomp/CSV。纯增字段,信号集不变 | 小 | **零违规** | 无 |
| **W7 正规出场** | P7:econ 出场口径4→closed_loop CloseRoot/ReduceCore(sell.rs:50)。第二类闭环 still-MISSING(sell.rs:35)诚实标注不臆造 | 中 | 出场口径变 | 无 |
| **W8 l_max** | P8极小:确认 BTC 未撞 L6 则 config.rs:74 无害 | 极小 | 无 | 无 |

## 串行尾
**W-VERIFY**(依赖 W1/W2/W4/W7/W8 全完成):对照知识库12概念 checklist 核验无残留 + 引擎识别层对拍(改前后 Classification 识别数值 bit-exact)+ 完整回测 + 有效域声明更新。

## ★矛盾上浮 C1(/escalate,选择类)
`Cand^δ_ℓ(x)` 谓词定义式缺失(spec 疑点2,nest.rs:146 [需人工确认])。W1 完整实装需它但 spec 无权威定义:
- **A**(Cand=破中枢,宽松)vs **B**(Cand=次级别第一类,严格,更合 §11.3)
- 给不同信号集,不可模糊化(no-workaround)。W1 骨架先做,空洞待 C1 结算——非"简化一半",是定义缺失的诚实暴露。
- 结算方式:编排者 /escalate 或 Gemini decide。

## bit-exact 边界(编排者痛点第3条)
- **引擎 bit-exact(不该破)**:笔/段/中枢/塔识别数值。本轮无工位碰识别层(W1只读tower/W4纯增字段/W2只碰背驰门)。触碰须对拍+codex审。
- **回测信号集变化(预期变,非违规)**:W1准入门/W2背驰门/W7出场口径改→台账变=从简化到完整的定义性后果。现有1304测试中断言旧信号集的部分(interp.rs:1032/1053)预期失败,重写到新语义,不是 fix 到旧值。
- 判据:测试失败→"修复需改 parser/classifier 识别输出吗?"需要=引擎违规停下;不需要=预期变。W4 破 bit-exact = W4 有 bug。

## 里程碑切分
- **M-complete-single**(本轮):P1/P2/P4/P7/P8=单标的旧缠论(§1-§11)完整。完成后可声明"单标的旧缠论无简化残留"。
- **M-next**(范围外):P3 §12 跨标的等价关系(比价/等价/不变量/四矩阵/流转)=另一维度,单标的回测定义上不覆盖,需多标的数据管线。基础:reference_lu_qiyuan/k4_fold_channel_model/.chanlun/definitions/{bijia,dengjia,liuzhuan}.md。不阻塞本轮。

## 下游推论
W-VERIFY 后现有 alpha 否证(奇偶交替/σ_higher/夏普)有效域从"识别层子集"扩到"完整单标的旧缠论",但仍单标的 L2,P3 缺,不能外推"完整缠论无 alpha"。
