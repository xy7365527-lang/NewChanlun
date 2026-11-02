# 收据 workers/premerge-0086/REPORT.md

- 原件：`workers/premerge-0086/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`d7a3b0dd3eed56614f04b79d31b0fec34db588b35b667ad63db126b2e306aa4b`
- 字节数：10992；行数：108

## 关键行（行号: 原文）

```
1: # premerge-0086 REPORT：并入 S4 锁、再合 main（#1597）、清零 #1488 自带红
3: 工位 budget-handoff-fix-v2-checkout，分支 codex/1488-budget-full-chain-fix-v2-20261018，起点 026e3e97b0，终点 **f0aeaf2817**。未 push main，未开 PR，未发 GitHub 评论（只推了 uo-ci 用的 CI 分支）。过程记录见 NOTES.md。
5: ## 1. 各步提交
15: ## 2. #1597 SourceCoords 替换清单
28: ## 3. 每条红的根因与修法
34: | content_publication `native_publication_begin_and_consumers_share_parent_scope` | 959d860065 按 P1c 裁定改为 RAII 许可、到真实析构才释放；begin 校验用的临时许可在返回时即还。探针读数：before=128、M=96、begin_held=224=before+M，其余断言全成立 | 只改首条断言 | **改断言（附裁定 P1c，control/1488-budget-full-chain-counterexample-fix-v2.md:9）——第十八处候选** |
39: | lib `s_content::native_owner_codec_bridge_tests` ×5（cross_scope、avl_fixed_delta、partial_promotion、codec_original_moves、codec_original_sparse） | 断言写的是 813e1f92b3 之前 `permit.retain()` 的语义（许可脱手、父账永久留账）。959d860065 按 P1c 改为 RAII 持原父许可，直到真实析构。在 026e3e97b0 上这 5 条同点同红（log 20261021-162101-026e3e97b0），说明合并前
41: ## 4. 验证读数（终点 f0aeaf2817）
63:   - uo (s_session)：**success**，4 PASS + 94 NOT_LISTED
64:   - uo (native)：**failure**，97 PASS，唯一失败是 S4 FAIL(124)，原因见 §6-2
83: ## 5. 生产 release nobid（`--features s_session`，objcopy 去 build-id 后 strip）
85: | 提交 | sha256 | 字节 |
90: | 56ddd8d86f / **f0aeaf2817（终点）** | **6e7294ca5ffa6410444071ae7d4c40da61db3a9f35a94d408a7aa716825a2ddf** | 10593760 |
96: - **步 3**：对生产二进制**零影响**。7fbffdc4fb 与终点逐字节 cmp IDENTICAL。
98: ## 6. 需负责人裁定
100: 1. **改断言追加清单**（接 CLOSE-GATE-DRAFT §4 的十七处），都依据 P1c：
103: 2. **S4 锁在 uo workflow 下必定 FAIL(124)**：issue-1488-uo-iter.yml 每条锁按 debug 跑、超时 600s，而 S4 在 debug 下是 O(N²)（release 下约 1000s）。需要决定改 workflow 让 S4 走 release，还是把 S4 移出 uo 锁表另行跑。在此之前，“终面 CI 锁表全绿含 S4”这一项无法在现 workflow 上达成。
```
