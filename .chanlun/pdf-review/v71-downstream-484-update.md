# v71-swarm/downstream-484-multi-tf-status-update（v2：blocked 降级版）

**时间**: 2026-04-24
**topo_address**: v71-swarm/downstream-484-status-update
**parent_callback**: team-lead
**认识论等级**: L2（真实代码审查 + 测试执行对齐）
**版本历史**:
- v1（撤回）: 5 条推论标 resolved——声明膨胀，未校验代码严格性
- v2（当前）: 5 条推论降级为 blocked——基于 codex-challenge-484-impl 确认的代码违规

## 结论

484 号谱系的 5 条 downstream_inferences 从 `resolved` 降级为 `blocked`——不是因为实装未完成，而是因为**代码违反 090 号 no-patch-mentality**（`extract_c_segment_timestamps` 中 4 处 clamp 掩盖越界不变量），**测试未覆盖违规路径**（10/10 测试通过但不测 clamp 分支），以及 **run_cross_scale_nested_search 软失败级别过轻**（`logger.debug + continue` 静默遮蔽跨TF背驰跳过事件）。

测试 10/10 通过是事实，但"测试通过 ≠ 实装严格正确"——这是本次升级的教训（090 号声明膨胀的典型案例）。

## 质询三步分析（回应 team-lead 工位间矛盾）

### 定义回溯："resolved" 的严格定义

按 484 号 downstream_inferences 字段语义，status=resolved 应表达"该推论的实装忠实、严格、无补丁思维"。v1 版本错误地将"测试 10/10 通过"等价于"实装正确"——这忽略了测试只覆盖已有代码路径，不覆盖"代码实现是否违反不变量"这一正交维度。按 090 号严格性："代码能做什么就声明什么"——clamp 路径让代码"大致能工作"而不报错，声明 resolved 就是声明膨胀。

### 反例构造：test_seg_index_out_of_range_raises 测了什么？

该测试只覆盖 `move.seg_end >= len(segments)` 这一条**显式 raise 分支**（multi_tf_adapter.py:368-371）。**不覆盖** line 396-403 的 `max(0, min(seg_first.i0, n_merged-1))` 等 4 处 clamp 路径——那是 Codex 标记"致命"的位置。测试通过证明的是"显式 raise 分支工作"，不是"clamp 路径不存在"。

### 推论检验：status=resolved 的下游影响

P6 YAML 路径让 ceremony_scan 忽略 484 工位——但代码违反 no-patch-mentality 时，resolved 就是**声明膨胀**（090 号禁令第 5 条）。后果：
1. 基因组层认为 484 已闭合，不再重访——但代码层缺陷持续潜伏
2. 任何基于 484 推论的上游谱系（如未来的 T6 实装）会建立在错误基底上
3. 蜂群视野收敛到虚假的"完成"状态——这是蜂群治理失败的根因形态

## 降级决策理由

**选 blocked 不选 /escalate**：

- Codex 2 条"致命"判定中，1 条（clamp）**确认成立**，1 条（low_snap 未定义）**经 agent 查证为误判**（实际有 `low_snap = None` 初始化 + `if low_snap is None: continue` 保护）。
- Codex 3 条"重要"判定：merged_to_raw 一致性、ts 反向未验证、软失败可观测性——agent 审查代码后**全部确认成立**。
- 测试覆盖不足——agent 读取 `tests/test_multi_tf_cross_scale.py`，确认 clamp 路径无测试，ts 反向无测试，merged_to_raw 不一致无测试。
- Codex 的"致命"判定有 1 条误判是 context 精简导致，不是系统性过严——剩余 2 致命 + 3 重要均是**真实代码问题**。
- 不存在定义矛盾或需要编排者价值判断的内容——因此**不 /escalate**。

按四分法分类：这是"行动"——四分法允许自动执行，无需上浮。

## 五条 blocked 的具体阻塞项

| id | 阻塞项 | 修复要求 | 谱系引用 |
|----|--------|---------|---------|
| 484-1 | 软失败 `logger.debug + continue` 过轻 | 升级为 `logger.warning`，加 skip_reason 字段 | codex 报告问题6 |
| 484-2 | clamp 掩盖不变量（multi_tf_adapter.py:396-403, 4 处 max/min）| 去掉 clamp，改严格断言 raise IndexError | 090 号 no-patch-mentality，codex 报告问题1 |
| 484-2 | merged_to_raw 本地重建未与上游一致性校验 | 从 RecursiveOrchestratorSnapshot 暴露 merged_to_raw 或增加同源校验 | 209 号商空间映射，codex 报告问题2 |
| 484-2 | ts_start > ts_end 未验证，返回反向时间窗静默掩盖 | 返回前断言 `raw_start <= raw_end`，否则 raise | codex 报告问题3 |
| 484-3 | 依赖 484-2，上游 clamp 给错窗口则本步骤静默返回空 | 待 484-2 修复后间接解除 | codex 报告问题3 下游 |
| 484-4 | 低级别快照缺失时软失败过轻 | 同 484-1 修复策略 | codex 报告问题6 |
| 484-5 | 测试覆盖 clamp、ts 反向、merged 不一致均缺失 | 补 3 条新测试，覆盖 clamp 越界 raise + ts 反向 raise + 排序异常 bars | codex 报告问题7 |

## 边界条件

| 条件 | 当前 | 翻转阈值 |
|------|------|----------|
| 5 条 action blocked | Codex 2 致命 + 3 重要确认 | 若修复工位完成 clamp→raise、ts 反向校验、warning 升级、测试补充——status 可重新评估为 resolved |
| 谱系 YAML 字段生效 | blocked 在 OVERRIDE_STATUSES 中（downstream_audit.py line 18）| 若 downstream_audit 优先级链被重构需重新验证 |
| 谱系主体未改动 | 仅 frontmatter status 字段从 resolved 变 blocked | 144 号结构性不变量保护持续生效 |
| Codex 误判 2 条已过滤 | low_snap 和 max_levels 经 agent 复查确为误判 | 若发现 Codex 其他误判点，需进一步质询 |

## 影响声明

- 修改文件：`.chanlun/genealogy/settled/484-multi-tf-architecture-audit.md`——仅 frontmatter `downstream_inferences[*].status` 字段从 `resolved` 改为 `blocked`，附 codex 报告引用
- 未修改：谱系主体内容、depends_on、negates、核心结论、epistemological_level 等
- 未修改：`src/newchan/topology/multi_tf_adapter.py`、`src/newchan/topology/multi_tf_pipeline.py`（本工位职责是 status 更新，不是代码修复——需另一工位处理）
- ceremony_scan 输出：5 条 blocked 被 downstream_audit 单独计数，不进 unresolved 列表——工位数 0，但这是"已识别阻塞"不是"已解决"
- 产生下游工位：修复工位 `fix-484-clamp-and-observability`（待 lead 决定是否立即 spawn）

## 测试执行证据（测试通过但不证明严格性）

```
============================= test session starts ==============================
collected 10 items
...
======================== 10 passed, 2 warnings in 1.78s ========================
```

**注意**：10/10 测试通过是事实，但不证明代码严格——这是本次教训。测试只测已有代码分支的行为，不测"代码是否违反不变量"。

## ceremony_scan 最终状态

```
$ python3 scripts/ceremony_scan.py | jq '.workstations[] | select(.name | contains("484"))'
(无输出——blocked 状态被 downstream_audit 单独归类，不在 unresolved 工位列表中)
```

## 引用

- `.chanlun/review-results/v71-codex-484-review.md`——Codex 异质审查报告（v71 工位产出）
- `src/newchan/topology/multi_tf_adapter.py:396-403`——clamp 违规位置
- `tests/test_multi_tf_cross_scale.py:223-232`——`test_seg_index_out_of_range_raises`（不覆盖 clamp 路径）

## 谱系引用

- **484 号**：本次更新对象
- **090 号 no-patch-mentality**：clamp 违反的规则
- **144 号结构性不变量保护**：约束只改 frontmatter status，不改主体
- **209 号商空间映射**：merged_to_raw 一致性问题的谱系基底
- **238/237 号**：484 号直接 depends_on，维持不动

## 教训（为未来工位记录）

1. **"测试通过"不等价于"实装严格"**——测试只覆盖已有代码路径，不覆盖"代码实现是否违反不变量"
2. **resolved 的证据要求**：代码层无 no-patch-mentality 违反 + 测试覆盖关键边界（包含 raise 路径断言） + 异质审查通过
3. **本地重建的幂等性需要校验**：如 merged_to_raw 从上游 snapshot 重建时，必须与原始来源一致性校验，否则时区/排序偏差会导致漂移
4. **软失败可观测性**：debug 级别对于"跨级别搜索跳过"这类非平凡事件过轻，应升级到 warning + 原因字段
