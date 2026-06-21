---
name: Mac Migration Status
description: Windows→Mac迁移部署完成状态
type: project
---

Mac Studio（silencehan），从 Windows PC（hanju）迁移。

部署完成（2026-04-05）：
- ~/.claude/ — 25 agents, 13 commands, 5 skills (含 philosophy-essay), settings.json, hooks, rules
- ~/rtas/ — .claude(23 agents, 15 commands, 12 skills) + .rtas(definitions/genealogy/sessions) + scripts
- ~/trading/ — 83个文件（70个迁移脚本 + 13个 tws_*.py）
- ~/academic/ — 3.7GB，ANTH0098, HPSC0041_Dissertation, HPSC0042, HPSC0061, HPSC0111, HPSC0162, literature
- ~/Projects/NewChanlun/ — 缠论量化系统
- ~/Projects/trading/ — 也有一份交易脚本（Claude Code 那边写的新脚本在这里）

/Users/hanju/ 仍保留原始文件。academic 有重复目录（空格 vs 下划线命名）待清理。

**Why:** 用户从 Windows 迁移到 Mac Studio
**How to apply:** 如果用户问文件在哪，优先查 silencehan 目录
