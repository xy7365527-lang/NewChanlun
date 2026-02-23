# Session: 2026-02-18-bspdiff

**时间**: 2026-02-18
**基底 commit**: 9ab58ca + (pending)
**分支**: claude/cc-session-briefing-setup-TXHg8

## 定义基底

| 名称 | 版本 | 状态 |
|------|------|------|
| baohan | v1.3 | 已结算 |
| beichi | v1.1 | 已结算 |
| bi | v1.4 | 已结算 |
| fenxing | v1.0 | 已结算 |
| level_recursion | v1.0 | 已结算 |
| maimai | v1.0 | 已结算 |
| xianduan | v1.3 | 已结算 |
| zhongshu | v1.3 | 已结算 |
| zoushi | v1.6 | 已结算 |

## 谱系状态

- 生成态: 0 个
- 已结算: 19 个（005 从 pending 移入 settled）

## 中断点

- ✅ maimai v1.0 发布仪式
- ✅ 四分法谱系 018
- ✅ BspDiff 身份键映射 diff 修复（位置对位→身份键映射，修复 I27 违反）
- ✅ 005 pending 指针清理（moved to settled/）
- ✅ 否定传播测试（9 个场景：Move/Zhongshu/部分否定/否定+新生）

## 恢复指引

1. 读取此文件获取状态指针
2. BspDiff 已完成：diff_buysellpoints() 改为身份键映射 diff
3. 新增 tests/test_bsp_diff_identity.py（10 个测试）
4. 新增 tests/test_bsp_negation_propagation.py（9 个测试）
5. 全套件 1140 passed, 10 skipped
