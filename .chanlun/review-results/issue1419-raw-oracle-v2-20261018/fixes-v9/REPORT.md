# #1419 raw oracle v9：同代冲突首报键按排序遍历（R8-1）与 R7-2 评估

工作草稿，活期绑定 #1419。基线 `00913c2081`（v8，独审 PASS：#1419 issuecomment-6263317155）。作者 /root/owner_1419c（接续负责人），不自审。

## R8-1（LOW）已修

`driver/fixed_cut_driver.py` `_check_cut` 的同代完整切面比较原先遍历 `FIXED - {...}` 这个 set，首个报出的冲突键随 PYTHONHASHSEED 变化，observations 不能逐字节复现。现改为 `sorted(...)` 遍历。判据集合与拒绝条件不变，只固定报错顺序；两条同代冲突夹具现在稳定报 `同代完整切面冲突:input_frontier`（排序后 input_frontier 先于 seq_range），与 v8 作者记录一致。

验证：`fixes-v6/run_checks.py` 在 PYTHONHASHSEED=0/1/2/3 下各跑一次，均 24 tests / 0 failures / 0 errors / 178 observations；四次的 observations.json 与 raw-exchange.jsonl 逐字节相同（`fixes-v9/seed-determinism.sha256` 记 seed1–3，seed0 即入库的 `fixes-v6/v9-*`）。tests stdout 只差耗时行。

## R7-2（LOW）评估：不属本票验收必需，保持登记

q3-consumer.js（2803-2822）对空批会按当前代重取完整快照，要求 projection 与客户端已应用状态相同。driver 不应用 Delta，watch 输出固定 `closure_verified: False`，因此它没有“已应用 projection”可比，任何代际的验收主张都必须来自 `closed_snapshot`。`closed_snapshot` 经 `call` → `_check_projection` 把 (cut_generation, history_mode) 绑定到唯一 projection 摘要与引用，同代同模式摘要冲突即拒。所以“同一切面下内容被换”在验收需要的那次闭包快照处就会被拒，空批不重取不会让错误结果进入证据。完整的 q3 等价需要 driver 实现 Delta 应用与六族对拍，这本就是已声明的 Delta/Snapshot UNVERIFIED 范围，归后续，不阻塞本票。若正式运行改为以 watch 结果直接作为某代证据，本评估失效，须先实现此项。

## 哈希

| 文件 | SHA256 |
|---|---|
| driver/fixed_cut_driver.py | 47ddf3d4ebb32ac2fb7b0b953ccec089da801938bf42d303f2cf761d19fbde5e |
| fixes-v6/v9-observations.json | b5ab9785402c9cf007064e250bcd30e682c40ceabe96c92a579bf7a6f389aeee |
| fixes-v6/v9-raw-exchange.jsonl | 81b3be7f15ac49ea908f4b01f288b8aa0c9de489085259178fdfbc1aa482a8c9 |
| fixes-v6/tests.v9.stdout | 2ec3ba0bbfe4ff804bce3adc311bdc60e3d72eb13857b1b1e4c605790ef462ff |

产品、Rust、正式 S3/Q、浏览器全部 NOT_RUN。包级 MANIFEST.json / SHA256SUMS 未重封。
