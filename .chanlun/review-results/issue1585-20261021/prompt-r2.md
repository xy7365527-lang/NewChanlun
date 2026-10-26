你是非作者独立审查者，只读。仓库当前目录为 PR #1664（#1585）的工作树，分支 harvest/issue-1585。用简体中文。输出第一行必须是「结论：PASS」或「结论：BLOCK」。

范围：新提交 `git diff 94c0b35bc5..a952c39e16`（三个提交：7e00b153cc、3d873f0a5e、a952c39e16），并核对上一轮审查 `.chanlun/review-results/issue1585-20261021/review-claude-fable-5-1.md` 中 M1、M2 的处置是否到位。

核对要点：
1. M1：`rust/src/theta_v0/backtest/capture_oos.rs` 的逐 bar「闸 + 段确认开平仓」是否抽成 `Replay::seg_step` 且主回测循环 `s3_capture_oos_real_btc` 真的调用它；终局 λ 样本与理论分母是否经 `train_gaps` / `final_seg` 取段；`segment_gn.rs` 主统计是否经 `s2_samples` 取样。负控 `tb1585_s3_refuses_segment_confirmation_under_marker` 与 `tb1585_s2_reads_only_valid_prefix_under_marker` 是否直接喂带标记的层调这些函数，以致把接线还原（`match readable_confirmed_len(layer)` 改回 `layer.segments_confirmed_len`；`segments_valid_prefix()` 改回 `segments`）时必红。红绿证据见 `.chanlun/review-results/issue1585-20261021/red-green.md`（可用 git show 看取证分支提交 7ef31cc11a 的 diff，如本地存在）。
2. 功能语义是否不变：对比 94c0b35bc5 版本的主回测循环，逐行确认重构前后的控制流、计数、开平仓、last_tradable_close 更新、止损/窗界/顺延出场顺序等价；λ gap 计算与 F-08 截断是否等价。
3. M2：注释里的 X-38 是否全改成 X-39；`.chanlun/definitions/xianduan.md` §15 新增 X-39 行是否符合该节体例、行号是否准确（逐个 sed 核对）、是否误动 X-38（#1600）那行。
4. L2：两把 #327 见证锁是否先断言拒读再断言数据身份。L4：新提交是否带编号声明行、首行不含 fix/close/resolve #号。
5. 负控是否存在奖励作弊（断言空转、正控不成立、条件恒真等）。

工具只读，不跑 cargo。给出 HIGH/MEDIUM/LOW 分级发现，每条带文件:行号。末尾写「看过什么 / 未核实什么」。
