# #1419 raw oracle v7：watch seq_range 连续性与追平空页

工作草稿，活期绑定 #1419。基线 `ff0524b0`（v6）。作者 /root/owner_1419，修复 raw-v6 重审（`raw-v6-review-owner/RAW-V6-INDEPENDENT-REVIEW.json`）的 R6-1、R6-2。

- R6-1：`driver/fixed_cut_driver.py` watch 增加 `delta.seq_range.from == last_seq+1`，与 `s_session/browser/q3-consumer.js` 的 Delta 断代判据一致。`fixes-v6/test_v6.py` 的 `watch_material` 原用 seq_range={7,7}，按 q3 改为连续（from=上一代 last_seq+1，to=本代 input_frontier）。
- R6-2：空批仅在 head==after（已追平）时放行，`has_more` 必为 false，观测头经既有 `_check_cut` 须与当前同代完整切面一致；放行后本次调用止于当前代，游标不动。未追平的空页仍拒。
- 新测试 `test_v7_seq_range_continuity_and_caught_up_empty_page`：断链拒且不推进、修正后同游标续接、追平空页正控、未追平空页与同代观测头变脸负控。

验证：`fixes-v6/run_checks.py v7` 纯 Python 离线 23 tests / 0 failures / 174 observations（`fixes-v6/tests.v7.stdout`）。产品、Rust、正式 S3/Q、浏览器全部 NOT_RUN。Delta 应用与 Delta/Snapshot 六族对拍仍 UNVERIFIED。包级 MANIFEST.json / SHA256SUMS 未重封，以本节哈希为准：

| 文件 | SHA256 |
|---|---|
| driver/fixed_cut_driver.py | 8c5b03edcdaf86ed2335b75d44a92d1a820ef32469db551ef0572fb089bf68c0 |
| fixes-v6/test_v6.py | a4d4ab392001975a590d455d96cc0b1135100552b63cabd8420e55cad41eb8e3 |
| fixes-v6/tests.v7.stdout | d042559ec32dfe2dd9f56cd49ee4fcf060290e523fbc5e0b2a44233901852c42 |
| fixes-v6/v7-observations.json | ad99645978eb1ea63162a3701e6b963f2fe23eb676a9a3b4987cc048f7361b90 |

需要独立复审后才可接纳。
