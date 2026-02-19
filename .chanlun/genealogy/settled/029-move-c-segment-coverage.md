# 029 — Move C段覆盖修复

**状态**: 已结算
**类型**: Bug Fix（行动）
**日期**: 2026-02-18
**提交**: 1de48c5

## 矛盾

`moves_from_zhongshus()` 设置 `Move.seg_end = last_zs.seg_end`，不包含最后中枢之后的段（C段）。
背驰检测中 `c_start = zs_last.seg_end + 1`，当 `move.seg_end == zs_last.seg_end` 时，
`c_start > c_end` → **C段永远不形成 → 趋势背驰在自动管线中永远检测不到**。

单元测试未暴露此问题，因为测试手工构造 Move 对象（seg_end 含 C段），绕过了 `moves_from_zhongshus()`。

## 修复

1. `moves_from_zhongshus()` 添加 `num_segments: int | None = None` 参数
2. 非末组 Move：`seg_end` 扩展到下一组 `first_zs.seg_start - 1`（覆盖组间连接段）
3. 末组 Move：`seg_end` 扩展到 `num_segments - 1`（覆盖最后中枢后的所有段）
4. `MoveEngine.process_zhongshu_snapshot()` 新增 `num_segments` 参数
5. `RecursiveOrchestrator.process_bar()` 传递 `len(seg_snap.segments)`

不提供 `num_segments` 时保持旧行为（向后兼容）。

## 影响

- 趋势背驰检测现在能在自动管线（RecursiveOrchestrator）中正常工作
- 进而 Type 1 买卖点（BSP）在集成管线中可被正确触发
- 区间套 level=1 的趋势背驰检测同样受益

## 验证

- 4 个新增测试全通过（含 divergence 集成测试）
- 1308 passed / 1 failed (pre-existing flaky timer) / 45 skipped (data dep)
- 无回归

## 推导链

- **前置**: zoushi.md v1.6（Move 定义），beichi.md v1.1（趋势背驰定义）
- **关联**: 028-ceremony-default-action（发现过程：ceremony 扫描 → YAML 同步 → 实际审计）

## 概念溯源

[旧缠论] 走势类型包含中枢后的离开段（C段），这是趋势背驰检测的前提。
修复只是对齐代码与定义，不涉及新概念。
