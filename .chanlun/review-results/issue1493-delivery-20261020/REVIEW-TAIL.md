**VERDICT: PASS 附必改**

必改只有一项，且是收据文字与自带附件的数字不一致（非代码）；两处代码面均无问题。

## ① 新测试 `m3_synthetic_segment_layer_does_not_stall`（l3_delta_r_alpha.rs:2573）

**能锁住已知停产，且两条断言各自独立判别。** 远端日志 `/tmp/nc-remote/20261020-152304-5814093e66.log` 读数 `segments=32 strokes=293 last_segment_end=2958 last_stroke_end=2997 bars=3000`，与作者所述一致。main 基线的停产读数在探针日志（`20261019-220446-b87c45fea3.log` 等 `[probe1420] n=3000 strokes=293 segments=14 last_end=1708`）中有记录，同一笔流（293 笔）下：14 < 30 触发第一条；2997−1708=1289 > 150 触发第二条。两条断言都能把基线判红，不是恒真占位。

索引口径一致：`Segment.end_index` 取自末笔的 `end_index`（segment.rs:868），与 `Stroke.end_index` 同一 merged-bar 索引空间；段终点 ≤ 末笔终点，`usize` 相减无下溢。测试在 `mod tests` 内 `use super::*`，直接调 `parse_layer`（生产解析路径），非 `#[ignore]`，已随 lib 套件计数：9f3b5281eb 全量 `2866 过/0 败/150 ignored`（afd7eadf37 为 2865，+1 恰为本测试）。

**脆性观察（非阻断）**：段数下限 30 与读数 32 只差 2 段（约 6%），这是停产的代理指标，正常的段规则微调有可能让它单独误红；真正的停产锁是第二条「末段距右缘 ≤150」（读数 39，余量充足）。若日后第一条单独失败而第二条仍绿，应按阈值重定而非当停产处理。建议在注释里点明这一分工，不要求本轮改。

## ② `--relax-query`（tb02x_runtime.py:113-116、:310）

**仅限测试夹具，不触生产、默认行为不变。** 该开关只改变 `prepare` 子命令写入 `config-A.json` 的 `query_resource_config` 路径：`relax = design == "e2_real" or args.relax_query`，默认 False 时 turn 用例仍指向冻结的 `tests/fixtures/tb01c/query-resources.json`，与改动前逐字等价。放宽文件 `query-resources-relaxed.json` 本来就在 prepare 开头无条件生成（:81-83），e2 用例原本就引用它，所以 turn 用例开了开关只是复用同一份预算（`verify_deadline_ms` 120000、捕获/缓存 2/4 GiB），没有新增语义参数。`s_resources` 的 `response_timeout_ms` 放宽仍只对 e2_real 生效，未被开关带入。全仓 grep `relax_query|relax-query|tb02x_runtime` 仅命中此文件本身，无 CI、launcher 或生产代码引用。符合主控裁定「宽松预算仅限测试夹具（与 e2 同口径）」。

## ③ 收据 RECEIPT.md ↔ #1493 最新评论（issuecomment-6287964291）

**主体读数自洽，FAIL 原因照实写了。** 两边一致的项：HEAD 9f3b5281eb、二进制 sha256 `bb0805b1…666d`、私有 target `target-1493` 串行、两例 40 cut 采完、手算神谕 PASS（cut 27 `PENDING_CASE_ONE`→cut 35 `CASE_ONE`，`HAND-ORACLE.json` 两例 `status: PASS` 且 `pending_first_cut=27 / confirmed_cut=35`）、第三视图 10 分钟超时（两例 `browser.stderr.txt` 均为 `page.waitForFunction: Timeout 600000ms`）、e2 known-51 32.1 s / known-42 37.2 s（附件 json `elapsed_ms` 32070 / 37195，`result: not_verified`）、脚本总判 FAIL（`log-tail.txt` 与 `evidence-private/20261020-162946-9f3b5281eb.log` 三例 `rc=1`）。收据 §结论第 12 行明确写「脚本判定为 FAIL」并列四条原因，与主控裁定要求的「照实写明脚本总判 FAIL 的原因」相符；转出票 #1497 目前只在评论中出现，收据未提——这是裁定在收据提交之后才下的，关票声明补写即可。

**必改：收据第 14 行的 readiness 失败次数是首跑旧数。** 收据写「turn_up 失败 74 次、turn_down 66 次」，但本目录自带的复跑 `READINESS-DIAGNOSTICS.json` 的 `counts.failed` 为 turn_up **65**、turn_down **59**；74/66 是首跑版本（`ff2d1bac7b` 中同文件）的数字，复跑收据换附件时正文没跟着改。同理第 16 行「40 cut 实际 106–108 s」与第 21 行「各约 105.5 s」是首跑/复跑两套读数并列。既然收据自称「本目录文件取自私有 target 串行复跑」且「读数与首跑一致」，应把第 14 行改为 65/59（或注明为首跑读数），第 16 行与第 21 行统一到复跑数值。定性结论（第三视图超时、超 30 s 期限）不受影响。

## 其他检查读数

- `git diff afd7eadf37..HEAD -- rust s_session`：仅两文件，+27 行测试、+5/−1 行 Python，无生产逻辑。
- 合并提交 `5e749a4a29`：双亲 `22de52d696`（PR #1582 当前 head）与 `bf871af9ce`（= `origin/main`）；`git diff HEAD^1 HEAD --stat` 无 rust/s_session 条目，main 侧确无该两目录改动。注意该合并提交尚未推送，PR head 仍是 `22de52d696`，其 CI 中 test / fixture-drift / s-observer 已绿，**rust-check 与 DevSkim 在我核对时仍 pending**（DevSkim 失败与 main 同源，作者已说明）。
- 本机未跑 cargo，未改任何文件，未输出任何令牌。
