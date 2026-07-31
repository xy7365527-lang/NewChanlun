# issue #822 研究报告：仓外缠论实现如何处理「≥9 段升级重切」的子中枢区间

- 权威性声明：本报告所有内容**只作参考旁证**，不作裁定依据。本仓认的权威分层是缠师原文→Lean 形式化→生产读数三层，社区实现哪一层都不在。
- 结论先行（详见末尾总判）：**查到 0 个开源实现严格照搬第 33 课「9 段→按 3 段分组→重算各子中枢 [ZD,ZG]」的完整算法**；查到 1 个真实项目**明确讨论并有意识地放弃了这条约定**（有文档记录取舍理由）；查到 2 个项目在"9 段"这个数字上有代码触达，但触达的是另一个功能（三类买卖点判定），不是子中枢区间重算。

---

## 1. 有没有实现「≥9 段重切」

### 1.1 Vespa314/chan.py（最主流的开源缠论 Python 框架，⭐️数最高）

- 可信度：**源码（可验证）**
- 文件：[`ZS/ZS.py`](https://github.com/Vespa314/chan.py/blob/main/ZS/ZS.py)，`combine()` / `do_combine()` 方法，第 115-143 行；[`ZS/ZSList.py`](https://github.com/Vespa314/chan.py/blob/main/ZS/ZSList.py) `try_combine()`，第 157-160 行。
- 实测抓取的源码（2026-07-30 从 `main` 分支拉取）：

```python
# ZS/ZS.py:115-143
def combine(self, zs2: 'CZS', combine_mode) -> bool:
    if zs2.is_one_bi_zs():
        return False
    if self.begin_bi.seg_idx != zs2.begin_bi.seg_idx:
        return False
    if combine_mode == 'zs':
        if not has_overlap(self.low, self.high, zs2.low, zs2.high, equal=True):
            return False
        self.do_combine(zs2)
        return True
    elif combine_mode == 'peak':
        if has_overlap(self.peak_low, self.peak_high, zs2.peak_low, zs2.peak_high):
            self.do_combine(zs2)
            return True
        else:
            return False
    ...

def do_combine(self, zs2: 'CZS'):
    if len(self.sub_zs_lst) == 0:
        self.__sub_zs_lst.append(self.make_copy())
    self.__sub_zs_lst.append(zs2)
    self.__low = min([self.low, zs2.low])
    self.__high = max([self.high, zs2.high])
    self.__peak_low = min([self.peak_low, zs2.peak_low])
    self.__peak_high = max([self.peak_high, zs2.peak_high])
    self.__end = zs2.end
    self.__bi_out = zs2.bi_out
    self.__end_bi = zs2.end_bi
```

- **判定：这不是第 33 课「9 段升级」**。这是「中枢**扩展**」（两个相邻同向中枢区间有重叠 → 合并成一个更大中枢，并把原中枢存进 `sub_zs_lst` 留档），触发条件是 `zs_algo`/`zs_combine_mode` 配置（`normal`/`over_seg`/`auto`），与「单个中枢内部笔数达到 9」完全无关。
- 全仓 `quick_guide.md`（[原文](https://github.com/Vespa314/chan.py/blob/main/quick_guide.md)）里 `## 中枢` 一节（第 443 行）**是空的**——只有标题，紧接着就是下一节「策略实现」，没有任何正文。全文档搜索「9段」「延伸」「扩展」「升级」，除了买卖点相关的 `divergence_rate` 外**无一处出现**。也就是说这个最主流项目的官方文档**完全没有提及**第 33 课这条 9 段升级规则。
- 结论：**确认没有实现**「9 段→重切」这条具体算法（不是"没找到"，是拉了源码、读了 combine 逻辑、读了全部文档目录，确认它做的是另一件事）。

### 1.2 yijixiuxin/chanlun-pro（另一主流开源项目）

- 可信度：源码（部分可验证；核心算法文件被混淆）
- 文件：[`src/chanlun/cl_utils.py`](https://github.com/yijixiuxin/chanlun-pro/blob/master/src/chanlun/cl_utils.py) 第 353-356 行：

```python
# 回调不进入中枢的，产生三类买卖点
"cl_mmd_cal_not_in_zs_3mmd": "1",
# 回调不进入中枢的(中枢大于等于9段)，产生三类买卖点
"cl_mmd_cal_not_in_zs_gt_9_3mmd": "1",
```

- **判定：这个「9」不是子中枢重切，是三类买卖点(3B/3S)判定规则的一个开关**——当中枢内部笔数 ≥9 时，是否允许「回调不进入中枢」触发 3 类买卖点。跟 [ZD,ZG] 怎么算完全无关。
- 该仓库真正的中枢构造核心算法在 `src/chanlun/cl.py`，经检查该文件是 **pyarmor 加密混淆的字节码**（`__pyarmor__(...)` 打头，无法读出源码逻辑）。**这意味着该项目"是否/如何实现 9 段重切"这一问题在源码层面查不到**——这是"查不到"，不是"确认没有"。
- 该仓库的一个第三方 fork [wangwangdog/a-stock-analyst](https://github.com/wangwangdog/a-stock-analyst)（`chanlun-pro/src/chanlun/cl.py`, `cl1.py`, `cl2.py`, `cl3.py`, `cl5-buy.py`）里同样出现多处 `f"离开中枢后回调不进入（中枢≥9段）"` 字符串——同一个 3 类买卖点判定逻辑的多版本迭代/改写，不是子中枢区间计算。可信度：源码可读，但仍是围绕买卖点判定，不涉及区间重算。

### 1.3 waditu/czsc（另一主流缠论工具库）

- 可信度：**查不到**（不是确认没有）
- 该项目自 1.0.x 起核心算法（分型/笔/中枢/信号）已经**迁移到 Rust**，通过 PyO3 以 `czsc._native` 扩展形式暴露给 Python（[Releases 页面](https://github.com/waditu/czsc/releases)）。Rust 源码未在本轮检索中定位到公开仓库路径，纯 Python 的旧实现在 0.9.x 之前的历史版本里，本轮未深入拉取。
- **明确标注：这条是"没找到"，不是"确认没有实现"。** 需要进一步拉取 czsc 0.9.x 分支或 Rust `_native` 源码仓库才能给出确定判断。

### 1.4 moremeds/argon（个人/小型交易看板项目，非"主流"缠论库，但直接相关且文档完整）

- 可信度：**源码 + 项目设计文档（均可验证）**
- 文件：[`web/lib/chanlun.ts`](https://github.com/moremeds/argon)（仓库为私有/未公开确认，本轮通过 `gh search code` 命中并用 `raw.githubusercontent.com` 直接拉取成功，说明至少历史上曾公开索引）第 275-294 行、`src/uw_scan/chanlun/core.py` 第 219 行、设计文档 `docs/research/2026-07-14-chanlun-tv-view-research.md` 第 169-173 行。
- 该项目**明确讨论过第 33 课这条规则、并且明确选择不实现**，原话（逐字引用，来自 `docs/research/2026-07-14-chanlun-tv-view-research.md`）：

  > "**Pragmatic envelope 中枢升级, not textbook 九段升级.** Consecutive same-level zhongshus whose `[zd, zg]` ranges overlap merge into one level-2 zone spanning both in time, with price envelope `[min(zd), max(zg)]`. The textbook 九段升级 recursion (nine-segment pivot-of-pivots construction) is explicitly out of scope; merging is transitive by construction (3+ consecutive overlapping zones collapse to one level-2 zone)."

  以及 v1 设计正文（同文档第 46 行）：

  > "v1: standard extension, no pivot merging (zs_combine off), no 9-leg 中枢升级."

  代码对应（`web/lib/chanlun.ts:275-294`，注释原文）：

  > "中枢升级 (pragmatic): consecutive same-level zones whose [zd, zg] ranges ... ENVELOPE [min(zd), max(zg)]. Documented deviation — textbook 九段升级 ..."

- **判定：这是四条来源里唯一一个"点名承认没做 9 段重切、并解释了替代方案与理由"的项目**。它采用的是「相邻同级别中枢区间重叠即合并，取并集包络 `[min(zd), max(zg)]`」的**扩展式**做法（跟 Vespa314/chan.py 的 `combine_mode='zs'` 本质相同——按重叠判合并、按 min/max 取包络，都是「扩展」不是「9 段延伸重切」）。

---

## 2. 实现了的，子中枢区间怎么算

**本轮没有查到任何项目真正实现「9 段→按 3 段分组→各子中枢独立算 [ZD,ZG]」这套算法**，所以这一问在源码层面**无正例可答**。

唯一能回答的是"扩展"（不是"9 段延伸"）这条相邻路径下的合并算法，两个独立项目（Vespa314/chan.py、moremeds/argon）**做法一致**：取参与合并的各中枢区间的**并集包络**（`low=min(...)`, `high=max(...)`），而不是"各自独立保留区间"或"继承某一个母中枢的核心"。Vespa314/chan.py 额外把被合并前的原始中枢存进 `sub_zs_lst` 留档（可回溯查看合并前的子结构），但**用于交易判定的区间已经是合并后的包络**，不是子中枢各自的区间。

社区文章层面：WebSearch 工具对多篇文章（知乎问题 429200506、chanluns.com「中枢扩展升级」页、新浪博客「关于中枢九段升级的再分辨」）做了聚合式摘要，声称"9 段按 3 段一组各自算子中枢区间，再把三个子中枢的[ZD,ZG]当'三根K线'求重叠区间"是"网友总结"的一种算法。**这条不可信，标记为噪音**：
- 逐一用 WebFetch 直接抓取原文验证后，**chanluns.com 那篇文章明确不包含这个算法**（WebFetch 原话：「文章所述的『更高一级别中枢区间』仅指前后两个原始中枢的重叠区域（2-11或2-7），并未涉及进一步的子分组计算方法」）。
- 新浪博客「关于中枢九段升级的再分辨」同样经 WebFetch 直接验证，**不包含这个算法**（WebFetch 原话：「作者并未提供关于子中枢价格区间计算的具体论述或公式」）。
- 知乎问题 429200506、452352663 两条链接均返回 403，未能核实。
- 因此，WebSearch 工具给出的"三根K线求重叠"算法**无法追溯到任何一篇可核实的原始来源**，很可能是搜索工具自身在多篇零散资料上做的拼接式幻觉总结，**不作为证据使用**。

---

## 3. 没实现的，怎么处理无限延伸

- **只有 moremeds/argon 这一个项目明确谈到了取舍**（见第 1.4 节引用）：它承认"标准延伸不设 9 段上限"（`standard extension, ... no 9-leg 中枢升级`），代价是可能出现理论上应该升级为更大级别但代码里仍按同级别中枢处理的情形；它选择用"扩展式包络合并"作为折中，而不是抛弃升级概念。文档没有进一步论证"不设上限会不会导致走势类型判定出问题"——这个问题在该项目文档里**没有被展开讨论**，只是承认了 deviation 并注明范围（out of scope）。
- Vespa314/chan.py 的 `quick_guide.md` 和源码注释中**没有找到任何一处讨论"中枢无限延伸对走势类型判定的影响"**。翻查其 GitHub issues 搜索（`gh search code`/`WebSearch`）也没有命中相关 issue 讨论串；本轮未逐条翻查该仓库 issue 列表，此处**明确标注为"没找到"，非"确认没有讨论"**。
- 结论：四条来源中，**只有 1 条（argon）**做了"取舍说明"，其余在文档/注释层面都是沉默——既不实现，也不解释为什么不实现。

---

## 4. 社区共识文本

- 知乎「如何理解缠论规定中枢延伸九段就要级别扩展？」（429200506）、「中枢延伸过九段升级?」（452352663）——两条链接均 403，本轮**未能核实内容**，只能确认标题命中问题、无法逐字引用。
- chanluns.com「中枢扩展升级」页（`chanlun-zhongshu4`）——经 WebFetch 核实：**只讲"形成中枢扩展一定会有 9 段"这一结论性论述和中枢扩展的 7 个特点，不讲子中枢区间怎么算**。可信度：社区教程站，非源码。
- 新浪博客「关于中枢九段升级的再分辨」——经 WebFetch 核实：**讨论了升级的三种方式和 9 段概念定义，不讲子中枢区间计算公式**。可信度：论坛/博客发言。
- **没有找到任何一篇经过核实的社区文章，明确讲清楚"9 段重切后子中枢区间怎么算"**。检索到的疑似"网友总结算法"（三根 K 线求重叠）经逐篇核实后**查无实据**，不采信。

---

## 090 照实小结

| 项目 | 是否实现 9 段重切 | 判定依据 |
|---|---|---|
| Vespa314/chan.py | **确认没有实现**（区分于"扩展"合并） | 源码读完 `ZS.py`/`ZSList.py`，文档"中枢"章节为空 |
| yijixiuxin/chanlun-pro | **查不到**（核心算法文件被 pyarmor 混淆） | `cl_utils.py` 里的"9"是买卖点判定开关，非区间重算；`cl.py` 无法读源码 |
| waditu/czsc | **查不到**（核心已迁移 Rust，未定位到源码） | 未深入检索 |
| moremeds/argon | **确认没有实现，且有文档明确记录取舍** | 设计文档逐字承认"no 9-leg 中枢升级"，用扩展式包络合并替代 |

---

## 总判

这批外部证据对本仓 #812 Z-4 的裁定**有一点参考价值，但价值很薄，且是反向价值**：它没有提供任何"别人怎么算子中枢区间"的正面算法可以参照抄用（因为**没有一个项目真正做了这件事**），它提供的价值是负面确认——**第 33 课「9 段重切」这条约定在主流开源缠论实现和可核实的社区文章里都是冷门到几乎不存在的**：两个最主流的开源项目（chan.py、chanlun-pro）要么明确没做要么核心代码不可读；唯一一个明确讨论过这个问题的项目（moremeds/argon，非主流、小众）主动放弃了它，改用更简单的"相邻中枢重叠即合并、取区间并集包络"的扩展逻辑；社区文章层面能核实到的两篇都不讲这个问题，一篇声称讲了的"三根K线求重叠"算法经核实是查无实据的检索工具幻觉。

**本仓面对的"原文空白"（#819 已定案）在外部世界同样是空白——不是本仓没查到答案，是这题在整个中文缠论开源/社区生态里基本没人正面解过。** 这个结论本身可以作为 #812 Z-4 的一条弱旁证：本仓要给出这条区间公式的话，找不到"抄作业"的对象，只能自己定义并在 AGENTS.md 里落一条明确取舍（类似 argon 那种"承认 deviation、写清楚代价"的做法，是四条来源里唯一值得参考的工程姿态，但也仅限于"怎么写清楚一个取舍"这个方法论层面，不涉及具体数值公式）。
