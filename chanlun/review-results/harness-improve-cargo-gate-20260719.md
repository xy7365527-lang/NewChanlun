# improve-harness 单环记录：cargo 禁令授权门（2026-07-19）

方法来源：lopopolo/harness-engineering `playbooks/improve-harness.md`（读后应用，未拷贝其布局/策略）。

## 作业契约
- 目标与版本：/tmp/kimi-nest-mainline 镜像工作树（主仓禁写期）；p124 S=8 重放收口在途。
- 代表作业：任一 worker 在禁令窗口内于本工作区尝试 cargo 编译/测试。
- 验收结果：侵入路径被拒 + 诊断可行动 + 在跑重放零扰动。
- 证据口径：cargo 自身报错行、二进制 mtime、重放 pid 存活、零上下文 worker 复述。
- 授权包络：仅镜像工作树可写；主仓只读。
- 预算/停止：单环一次干预；失败即回滚（删 rust/.cargo/config.toml 即可）。
- 疑似缺口：Authority——禁令只以散文携带（goal/wire/派发词），靠主控人肉接力。

## 基线证据（本 session 实证轨迹）
禁令不变量出现在 ≥3 处散文（goal、台账、C-3 派发词）且每次派发都需重申；
无任何执行边界阻止 `cargo build` 重建并替换 target/release 下在用的重放二进制。

## 最早失败交接点与所有者
交接点：worker 获得 Bash 能力即获得 cargo 能力，许可与能力混淆。
所有者：工作区构建配置（rust/.cargo/config.toml）——唯一能塑造所有未来编译调用的权威点。

## 干预假设
在构建配置加 RUSTC_WRAPPER 哨兵门（.chanlun/locks/CARGO_BAN + cargo_gate.sh），
则任何触发 rustc 的调用在禁令窗口内快速失败并给出指向哨兵的诊断，
机制：危险路径（重建二进制）必经 rustc，门恰好落在危险面上；只读 cargo 操作不受影响。

## 实装与验证（claim 边界双层）
1. 原生检查：`cargo check --offline` 被拒，cargo 报 `exit status: 87`，CARGO_GATE 三行诊断输出。
   （测量勘误：验证脚本回显的「cargo 退出码: 0」是 grep 管道的退出码，作废；拒绝证据以 cargo 自身 error 行为准。）
2. 无扰动：p124_shard_replay mtime 1784419004 前后不变；pid 7130 存活（R 态 98% CPU）。

## fresh 重跑证据
零上下文 haiku 子代理独立执行 cargo check：正确判定「环境约束而非代码错误」，
跟随诊断指针读 CARGO_BAN，明确表态门退休前不绕过、不删门、改用只读命令或等待。
（干预确实被检索且被正确解读——非“成功但从未用到”。）

## 决定：retain（暂定）
- 携带成本：两文件 + 一配置；哨兵删除即透传，无需二次改动。
- 退休条件：C-3 RECON_PASS 后主控 `rm .chanlun/locks/CARGO_BAN`；若后续窗口复用，仅重建哨兵。
- 已知边界（诚实登记）：a) 透传路径（无哨兵）未实测，解禁后首次编译即验证；
  b) 全缓存的 cargo test 理论上可不触 rustc 而绕门——但危险面（重建二进制）必触，在保护范围内；
  c) 单环单 worker，一次前后对照，不构成一般化处置效应主张。
