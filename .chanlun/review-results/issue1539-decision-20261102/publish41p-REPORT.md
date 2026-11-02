# #1539 FU-17 送批包（fu17-publish41p）

## 结论

本目录是 #1539 最终送批的同一版本：41F 返修稿经 41K 非作者独审 PASS，再按 41K 的三项非阻断订正（N1–N3）做同义修改。推荐值、分类与合同与 41F 逐字相同。状态仍是 P / `needs-approval`，**用户尚未批准**；本包只供用户 `/graph-approve`，不落正本、不接边、不改标签。

## 给用户批什么

一次 `/graph-approve`，对象是本目录的 `shadow-comment.md` 问表、`draft.md`（含附件 A、B）与 `archaeology.md`/`archaeology.json`：

- Q4、Q12 两条 C 的推荐答案；
- Q9、Q10、Q11、Q13、Q15 五条 B 的默认值；
- 落地时在 ADR 0038、ADR 0039 文末追加 append-only「修订（#1539）」节的授权（ADR 0030、ADR 0056 不改）。

批准后才落正本、取新 ADR 号、接 #1539 blocks #1425 边。六 Destination、#1279 完整验收、资金隔离与恢复门照旧。

## 审查依据

- 独审：`/Volumes/AgentStorage/fu17-review41k/REPORT.md`、`review.json`。GPT 家族非作者工位 `/root/fu17_review41k`，未参与起草；verdict PASS，blockingFindings 为空。
- 被审稿：`/Volumes/AgentStorage/fu17-repair41f/`（原件保留未动，含 live 快照 `gh/` 与生成脚本 `work/`）。
- 证据基线：冻结 main `9de18e606b2c0330162a83e2a030898b46c3690f`，`/Volumes/AgentStorage/fu17-repair40r/frozen/`；SPEC 原文 `/Volumes/AgentStorage/fu17-repair40r/gh/body1340.md`。
- 前序：`/Volumes/AgentStorage/decisions/1539/`（两轮 BLOCK 转 P）、`fu17-repair40r/`、`fu17-review40y/`（BLOCK B1–B4）。

## 41P 改了什么

| 项 | 41K 发现 | 订正 | 位置 |
|---|---|---|---|
| N1 | `unit_price` 在冻结 `tb03c_per_leg.rs:31`，旧区间 :28-30 少一行 | 区间改 :28-31 | draft 5 处；archaeology.md/json 各 2 处；shadow-comment、review-response、repair41f-REPORT 各 1 处 |
| N2 | 1425 边被写成「边已有」，live 无此边 | 改为「沿用 40R 拟新增边，批准落地时接 #1539 blocks #1425」 | review-response B2；repair41f-REPORT B2 行 |
| N3 | command 费用可有命令因果锚，不能一概称无 manifest | 改为「不都以单笔成交 manifest 为前件，不能一律套成交量/单价投影；command 费用按 `command_id` 归属，period 费用按既裁坐标（ADR 0024:43 裁定六）」 | draft :108；archaeology Q13 推理链第 5 步（md/json） |

三项都不改变论据或推荐：N1 是标签少一行（核实命令本就读 28–31）；N2 只是状态措辞，草稿 :217/:222 已写待新增；N3 收窄一句推理，推荐值与问表 Q13 行原本即按 `command_id`/既裁坐标归属。

同步的包内指针：问表标题填入「41K 非作者独审 PASS」并指向本目录与 41K；草稿附件 A 原件链补 41F、41K，获批版本指本目录；草稿「考古对照」指本目录考古；archaeology.json `edition` 改为 `fu17-publish41p` 并加 `correction41p` 说明；问表末尾补「尚未获用户批准」状态行；review-response 末尾追加 41P 节。追加文本也由 `work/apply_41p.py` 生成。

## 文件

- `draft.md`、`archaeology.md`、`archaeology.json`、`shadow-comment.md`、`review-response.md`：送批版本。
- `repair41f-REPORT.md`：41F 作者报告的同步副本（只施 N1、N2）。
- `work/apply_41p.py`：从 41F 原件生成本包，每条替换断言命中次数；可复跑。

## 验证

纯文书，未跑 Rust、CI 或实验。`work/apply_41p.py` 全部 20 条替换按预期次数命中，`archaeology.json` 解析通过，包内已无 `per_leg.rs:28-30`。与 41F 的 diff 只含上表各项与指针。考古查询、命中、14/6/4 等计数与 A8/B5/C2 分类未变，沿用 41F/41K 证据。

## 发布

发布前读 #1539：3 条评论，无 41F/41P 影子问表，最近问表为 6544082577。已追加一条新评论 [6551181286](https://github.com/xy7365527-lang/NewChanlun/issues/1539#issuecomment-6551181286)，正文即本目录 `shadow-comment.md`；旧评论未改。读回（`gh-readback.json`、`gh-readback-body.md`）：OPEN，标签 `wayfinder:grilling`、`ready-for-agent`、`needs-approval` 不变，评论数 4，正文与 `shadow-comment.md` 仅差 gh 输出的末尾换行。全部 `gh` 调用经 `env -u GH_TOKEN -u GITHUB_TOKEN -u GH_DEBUG gh`；读回时间 2026-11-01T00:29:26Z（`date -u`）。边与标签未动。
