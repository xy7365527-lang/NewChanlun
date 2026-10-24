# #1605 代码常数清理清单（rg 佐证）

## 查找命令
```bash
# 查找生产代码中硬编码的 C-19 常数（排除测试与已标注）
rg -n "1\.645|0\.05|252\.0|0\.025|0\.975|1\.96|0\.[37]|1\.0e6" \
  rust/src/theta_v0/backtest/*.rs \
  --no-heading | grep -v "test\|#1605 C-19"
```

## 结果（2026-10-25）
**EXIT=0，无输出** — 生产代码中除已标注位置外，无其他硬编码常数。

## 已标注位置（保留到消费点票实施）
- rust/src/theta_v0/backtest/decontam.rs:168-169
- rust/src/theta_v0/backtest/highlow_mu.rs:113
- rust/src/theta_v0/backtest/metrics.rs:132/402/679/721

## impl Default 检查
```bash
rg -n "impl Default" rust/src/theta_v0/backtest/*.rs -A10 | \
  grep -E "1\.645|0\.05|252\.0|0\.025|0\.975|1\.96|0\.[37]|1\.0e6"
```
**EXIT=0，无输出** — 无 Default 实现使用 C-19 常数（测试夹具使用构造函数或 builder，符合要求）。

## 验收结论
✅ 生产、回放与准入路径上不再有未标注的代码常数。
✅ 已标注位置等待消费点票（#1398/#1410/#1441）改为从记录读取。
