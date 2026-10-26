# 收据 workers/journal-merge-implementation-0035/REPORT.md

- 原件：`workers/journal-merge-implementation-0035/REPORT.md`（相对 issue1488-ticket-manager-20261018/，仓外）
- sha256：`038dbe82ffeb2ed7dbca79175aace67fb6c4140b1cbbff202f0ab9f0b05bd7e4`
- 字节数：9332；行数：106

## 关键行（行号: 原文）

```
1: # #1488 0035 报告：收据 key 合并 + originals 增量许可（裁定 issuecomment-6278995984 二、三）
5: - 终点：`cc9fe7e1d6`
8: - 结论：normal_append 转为 PASS。**write_budget 仍 FAIL**，这一项未闭合，原因见 §6。其余锁全部 PASS；宽对拍没有新增失败。
10: ## 1. 修法
16: ## 2. 数据结构
27: ## 3. 消费者改读对照
39: ## 4. 读数前后
47: | normal_append 锁 | FAIL | **PASS** |
52: ## 5. 锁与负控
54: 新增或改动的锁都已追加进 `.github/1488-uo-locks.txt`，标题为 “0035，裁定 issuecomment-6278995984 二、三”：
57:   - **断言载体从 keys 改为 events。这是本票第三处经裁定修改的断言，依据 issuecomment-6278995984。**
75: ## 6. 未闭合：write_budget
85: 旧时代对照在 `19cae1dc09`（日志 `/tmp/nc-remote/20261020-082434-19cae1dc09.log`）：该锁 PASS、retained 0，只有一次 `owner_complete_hit`，没有 `owner_effects`，走的是冷路径。也就是说，旧版的“释放”来自写时 effects 放不下而软降级，并不是保留规则主动释放候选。（另外，本次改动前的 journal 溢出路径会硬报“publication暂存资源已失效”，而不是软释放。）
87: 现在 journal 合并后 effects 放得下了，候选满足现有保留规则（audit_owner.rs 约 880 行：`charge.total ≤ complete_retention_cap − raw_retained`），于是被正当保留。该锁的 retained==0 期望与效率提升直接冲突。要让它转 PASS，只能改资源帽或保留规则，或者改原断言，这些都在禁区内。**这一项需要主控另行裁定**，本段没有越界修改。
89: ## 7. 宽对拍与 CI
91: - **全锁**（`control/uo-remote.sh`，HEAD `cc9fe7e1d6`，`/tmp/nc-remote/20261020-083447-cc9fe7e1d6.log`）：73 ok，1 FAILED，失败的就是 write_budget。
94:   - `uo (s_session,s_capture_native_testonly)`：73 PASS，1 FAIL(101) = write_budget。
95:   - `uo (s_session)`：success，1 PASS，其余 73 条在这个 feature 组里本就不编译，显示为 NOT_LISTED。
99: ## 8. 写集
```
