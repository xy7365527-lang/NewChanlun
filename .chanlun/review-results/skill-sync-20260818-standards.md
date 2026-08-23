# Standards 轴评审：skill 同步（ticket-919-final 工作树，.agents/skills/）

> 说明：本评审的对象（13 文件内容同步 diff）在评审过程中被本体全量回滚（git checkout），
> 当前 `git diff HEAD -- .agents/skills/` 为空。原 Standards 子代理深挖锁生成机制时卡死（ipython 长任务
> 挂起 ~1 小时），已由本体删除；以下收尾三项由本体直接执行并如实记录。

## (a) 回滚验证

`git diff HEAD -- .agents/skills/` 输出为空；`git status --short .agents/skills/` 为空。
仓库内 13 个文件已恢复 HEAD（本机定制形态），无残留。

## (b) 台账核验（skills-lock.json computedHash vs 仓库当前文件 sha256）

| skill | 仓库文件 sha256 | skills-lock.json computedHash | 结果 |
|---|---|---|---|
| grilling | 44331dda… | 368d3dd7… | 失配 |
| implement | dc27831f… | 2139cfed… | 失配 |

注：两份哈希算法均为对 SKILL.md 原始字节的 sha256；抽查 2 个即见失配。
按 roster-20260814 记录，失配成因是**用户钦定的 12 处本机定制**（一次一问族、PRD 族、
implement 去 disable 旗等）——仓库文件是「上游 + 本机定制」的合成形态，而 skills-lock.json
记的是同步时上游原件的哈希。该失配是既定状态，非本次变更引入。

## (c) 结论（只陈述，不裁决）

- 本次变更已回滚为 0：Standards 轴没有可评审的 diff，无违规、无气味。
- HEAD 本机定制内容与 skills-lock.json 台账**系统性失配**（成因见上）；是否需要把
  skills-lock.json 的语义从「上游原件哈希」改为「仓库现行内容哈希」，或补一条本机定制清单，
  属台账口径问题，需编排者拍板（不在本次 skill conflict 收敛范围内）。
