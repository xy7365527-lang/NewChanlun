# /escalate: Stop-Guard 对 Lead 在"全 orchestrator-gated pending + 责任方存活"时死锁

> 2026-06-30 自主推进中反复触发。编排者睡眠期间无解,正式上浮待裁。

## 矛盾(真矛盾,非可自决)

Stop-Guard check (谱系 pending 非空 → 阻断 Lead 停止)与"pending 结算属 genealogist+编排者,非 Lead 自结算"**互锁**:
- 26 pending 经 genealogist 完整 018 分类:25 个 orchestrator-gated(选择类/语法记录待编排者裁 + 646/648 待 /ritual + 645 可 /ritual + 642/644 待 codex 裁 I-1/I-2 + 622/626 LFS 无权限),1 个(641)行动类已解决待结算。
- **Lead 无任何合法的 pending-reducing 动作**:自结算选择类=越权(019c/编排者权);642/644 明写"待 codex 裁 I-1/I-2"=选择类;622/626 蜂群无 git-lfs 权限。
- 571 责任方过滤说"蜂群级持续性由责任方存活保证,非下降为个体命题"——genealogist-3 责任方**已常设存活**。但 Stop-Guard 仍对 **Lead** "身份不可确定保守阻断"。

## 死锁本质

编排者睡眠期间,Lead 把所有行动类 pending 推进/结算后,剩余全 orchestrator-gated。此时:
- Lead 不能自结算(越权)
- Lead spawn 更多工位无法减少 orchestrator-gated pending(它们待编排者价值判断,非工位可产出)
- Stop-Guard 因 pending 非空持续阻断 Lead → **Lead 被困在无限"spawn-等待-再阻断"循环,每轮消耗 context 但不减 gated pending**

本轮实证:已多轮 spawn(行动类链 #53-#58 + settle + 641),每轮真实推进一点,但 gated pending 从 28→26 后无法再降(剩余全待编排者)。

## 选项(待编排者裁)

1. **改 571 责任方过滤**:当"全部剩余 pending 经 genealogist 分类为 orchestrator-gated + genealogist 责任方存活"时,Stop-Guard 应放行 Lead(责任方存活已保证蜂群级持续性,Lead 无合法动作时困住 Lead 是空转)。
2. **保持现状**:Lead 必须保持活跃直到编排者醒来裁决 pending(但这空耗 context,且 Lead 无实质动作)。
3. **Stop-Guard 增加"orchestrator-gated 豁免"**:pending 标记 orchestrator-gated 后不计入 Lead 阻断(只计入编排者/genealogist 责任)。

## 关联谱系
- 649(编排者裁决超越选项集)——本死锁可能需编排者否定共享隐藏前提(Stop-Guard"pending非空=Lead不可停"的前提在全gated时失效)
- 658(reducer 验收语义未完备)——同源:结算/停止语义在边界 case 未定义
- 571(责任方过滤)——本 escalate 直接质询其对 Lead 的适用边界

## Lead 已尽动作(本轮~33 commit)
alpha goal L0-L3真封 + 行动类链闭环(643/566a结算) + 五次假阳性纠正 + 选择类durable escalate + 责任方genealogist-3常设守。641待结算(下方同步spawn)。

---

## hook 根因定位(2026-06-30 补充,ceremony-completion-guard.sh 行号证据)

死锁是**设计层确定行为**,非我漏条件:
- **行 574-576**:`# 责任方判定：Lead 无条件责任方（全 swarm 判据）；teammate 按 responsible_agents 过滤` → Lead(main session 无 agentType)被无条件 `IS_RESPONSIBLE=1`。
- **行 559**:`swarm 判据（负责全局调度/spawn 结算工位）→ 无条件阻断；责任方过滤仅作用于 teammate`。
- **行 606**:`身份不可确定 → fail-safe 阻断`(main session 无 agent-setting)。
- **行 560-561**:`responsible_agents 字段缺失的 pending → 视为责任方；agentType 不可确定 → 同样保守阻断`。

⟹ **Lead 在任何 pending 非空时被无条件阻断**,唯一逃生阀是 check0 的 context≥85%(行 80 `RELEASE_PCT=85`)。

## 死锁的确定形式(编排者裁决所需的精确命题)
当满足:(1)所有剩余 pending 经 genealogist 018 分类为 orchestrator-gated(选择类/语法记录/待 /ritual/待 codex 裁/基础设施无权限) ∧ (2)Lead 已穷尽行动类结算 ∧ (3)genealogist 责任方常设存活 ∧ (4)context<85%
则:Lead 无任何合法动作(自结算=越权 019c),但 Stop-Guard 无条件阻断 ⟹ **Lead 被迫空转 spawn 假工作或轮询直到 context 自然涨到 85%**。

本轮实证:从此状态起 Lead 已多轮被阻断,每轮要么 spawn 工位(行动类已尽后只能制造假工作=违 no-patch)要么轮询(消耗 context)。escalate 后我选择**停止喂循环**——制造假 pending-reducing 工作是非严格产出,蜂群语法不合法(090号)。

## 建议裁决(三选一,编排者)
1. **hook 改**:check3 增 orchestrator-gated 豁免——当所有 pending 的 responsible_agents 均不含 Lead/swarm(即全是 genealogist/编排者责任)且 genealogist 责任方存活时,放行 Lead(蜂群级持续性已由存活责任方保证,无条件阻断 Lead 是空转)。
2. **保持现状**:接受 Lead 空转至 85% context 逃生(浪费 ~5-9% context 但不改 hook)。
3. **责任标记**:给 25 个 orchestrator-gated pending 的 frontmatter 加 responsible_agents 显式排除 Lead(但 638/645 等已有该字段?需核)——使 check3 的 teammate 过滤路径覆盖 Lead。

我(Lead)倾向 1——这是 571 责任方过滤逻辑的自然延伸(责任方存活=持续性保证,不应无条件阻断无合法动作的 Lead)。但这是 hook/CLAUDE.md 层改动,触及基因组→020 阻断等待,**必须编排者裁,Lead 不自改**。

---

## 最终确认(穷尽所有Lead脱困路径,2026-06-30)

逐一证伪所有"看似合法"的Lead脱困动作:
1. **行动类pending结算**:已全做(643/566a/641→settled)。无残留。
2. **646/648落盘**:status已裁但落盘锁`/ritual`(019c memory广播=编排者权,文件头明写"settled落盘走/ritual")。genealogist不能自落盘。
3. **补responsible_agents字段排除Lead**:**对Lead无效**。hook行606+行23:main session无agent-setting→agentType不可确定→`print('1');sys.exit(0)` fail-safe阻断,**先于**parse_responsible(责任方字段检查是teammate路径)。Lead走不到字段检查就被无条件阻断。
4. **decide()代理裁改hook**:hook/CLAUDE.md=基因组,改触发020阻断等待(需人类编排者)。

⟹ **Lead(main session)在"全orchestrator-gated pending + context<85%"时,无任何数据层/动作层脱困路径。** 唯二出路:
- (a) context≥85%触发check0逃生阀(行80 RELEASE_PCT=85)放行+autocompact
- (b) 人类编排者裁决(改hook方案1 / 或/ritual结算pending清空 / 或手动覆盖)

这是Stop-Guard"不允许停止"与no-patch"不允许非严格产出"的**真矛盾纯粹形式**:Lead无合法动作时,满足前者(产出动作)必违后者(动作=假工作)。本session已实证~12轮:每轮Stop-Guard block,Lead无真动作,拒绝假工作。

## Lead最终姿态(no-workaround严格执行)
停止产出任何动作。识别真矛盾、完整escalate、拒绝绕过(即使绕过=满足Stop-Guard让turn结束)。等编排者裁决或context自然逃生。继续动作=制造假推进=违090号声明膨胀/no-patch语法规则。
