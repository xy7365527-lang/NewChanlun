//! #1491（ADR 0037 §五/§六）：S 发布内容布局组件——由库内 `rule_revision` 唯一解析。
//!
//! 组合规则 `<base>+<教义组件…>+bounded-layout/<n>`：组件正交，布局组件只定快照累计数组次序
//! 与 CC-007 事实承载，不改目录轴、不改教义判定。串内无 `bounded-layout` 组件 ⟹ ADR 0037 之前的
//! 写者，按定义是 layout/1（`Layout::Sorted`），不是兜底；出现未知 `bounded-layout/<n>` 一律拒绝。
//! 读写两侧只经 [`Layout::of_rule`] 取布局，再由 [`Layout::epoch`] 给出公开主序的代际前缀，
//! 两种布局不接在同一 if-else 上作准入兜底（#799/#804）。

/// 布局组件名（含分隔符的前缀）；版本号紧随其后。

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Layout {
    /// layout/1：累计数组按内容键排序（ADR 0037 之前全部 v3 库；只读）。
    Sorted,
    /// bounded-layout/1：累计数组按出生序（代际单调不减，同代内沿 layout/1 原键）。
    Bounded1,
}

impl Layout {
    /// 从完整 `rule_revision` 串取布局组件；同串至多一个布局组件。
    pub(crate) fn of_rule(rule_revision: &str) -> Result<Self, String> {
        let layout = split(rule_revision)?.1;
        #[cfg(test)]
        if let Some(forced) = FORCED_LAYOUT.with(std::cell::Cell::get) {
            return Ok(forced);
        }
        Ok(layout)
    }

    /// 库内 meta `rule_revision` 的布局：写者、历史审计与 owner 恢复同源只取此处或
    /// 由同一 meta 读出的串经 [`Self::of_rule`]，不另设默认布局。
    pub(crate) fn of_library(conn: &rusqlite::Connection) -> Result<Self, String> {
        let rule = crate::meta_get_opt(conn, "rule_revision")?
            .ok_or_else(|| "VersionUnsupported：库缺 meta.rule_revision".to_string())?;
        #[cfg(all(test, feature = "s_capture_native_testonly"))]
        if let Some(forced) = FORCED_LIBRARY_LAYOUT.with(std::cell::Cell::get) {
            split(&rule)?;
            return Ok(forced);
        }
        Self::of_rule(&rule)
    }

    /// 公开主序的代际前缀：layout/1 恒为 0（同一代际桶，退化为原键序）；
    /// bounded-layout/1 取调用方给出的事件代——活动对象与见证/关系/观察取出生代，
    /// 撤回对象取撤回代（ADR 0037 §一）。
    pub(crate) fn epoch(self, generation: i64) -> i64 {
        match self {
            Self::Sorted => 0,
            Self::Bounded1 => generation,
        }
    }

    /// 六族公开主序键（ADR 0037 §一）＝（[`Self::epoch`] 代际前缀，layout/1 原键）。
    /// 活动对象传首获知代，撤回对象传撤回代，见证/关系/观察传出生代（表列
    /// `published_generation`）；raw_history 按 seq，不经本键。快照数组排序、RankSet 键、
    /// 增量维护、恢复与历史审计的次序判据只经本函数取键；layout/1 前缀恒 0，逐位退化为原键序。
    pub(crate) fn rank<K>(self, generation: i64, key: K) -> (i64, K) {
        (self.epoch(generation), key)
    }
}

#[cfg(test)]
thread_local! {
    /// #1491（仅测试，负控故障注入）：本线程强制 [`Layout::of_rule`] 的结果。`of_library`
    /// 内部也经 `of_rule`，故作用域内写者、历史审计与 owner 恢复同被强制；负控有效只靠
    /// 作用域——历史测试只包写者发布调用，审计在作用域外按库布局取序才拒收。包住整个
    /// `owner.publish` 会令写者与恢复两侧同改、不拒收，owner 恢复臂负控改用只作用于
    /// `of_library` 的 `FORCED_LIBRARY_LAYOUT`。不是准入兜底（#799/#804）。
    static FORCED_LAYOUT: std::cell::Cell<Option<Layout>> = const { std::cell::Cell::new(None) };
}

/// #1491（仅测试）：在 `f` 内令 [`Layout::of_rule`] 恒返回 `layout`（规则串仍先经 [`split`]
/// 校验；`None` 即不强制；经 `of_library` 取库布局亦然）；返回或展开时恢复原值。
#[cfg(test)]
pub(crate) fn with_forced_layout_for_test<T>(layout: Option<Layout>, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<Layout>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0;
            FORCED_LAYOUT.with(|forced| forced.set(previous));
        }
    }
    let _restore = Restore(FORCED_LAYOUT.with(|forced| forced.replace(layout)));
    f()
}

#[cfg(all(test, feature = "s_capture_native_testonly"))]
thread_local! {
    /// #1491（仅测试，owner 恢复臂负控）：本线程只强制 [`Layout::of_library`]，即历史审计
    /// 与 owner 恢复（`append_owner_cut` → `restore_for_owner`）取库布局处；写者经
    /// [`Layout::of_rule`] 取 `metadata.rule_revision` 的布局，不受影响。与 `FORCED_LAYOUT`
    /// 的区别：后者作用于 `of_rule`，写者与恢复两侧同改；本注入只改恢复/审计侧，使同一
    /// `owner.publish` 内写者按库布局发布、恢复按另一布局重建。不是准入兜底（#799/#804）。
    static FORCED_LIBRARY_LAYOUT: std::cell::Cell<Option<Layout>> =
        const { std::cell::Cell::new(None) };
}

/// #1491（仅测试）：在 `f` 内令 [`Layout::of_library`] 恒返回 `layout`（库规则串仍先经
/// [`split`] 校验；`None` 即不强制）；返回或展开时恢复原值。写者侧 [`Layout::of_rule`] 不受影响。
#[cfg(all(test, feature = "s_capture_native_testonly"))]
pub(crate) fn with_forced_library_layout_for_test<T>(
    layout: Option<Layout>,
    f: impl FnOnce() -> T,
) -> T {
    struct Restore(Option<Layout>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0;
            FORCED_LIBRARY_LAYOUT.with(|forced| forced.set(previous));
        }
    }
    let _restore = Restore(FORCED_LIBRARY_LAYOUT.with(|forced| forced.replace(layout)));
    f()
}

/// 去掉布局组件后的规则串（教义组件部分）与布局；目录轴表等教义侧判据只看前者。
fn registered(rule_revision: &str) -> Result<(&'static str, Layout), String> {
    let (doctrine, layout) = match rule_revision {
        crate::W_STRUCTURE_RULE_REVISION => (crate::W_STRUCTURE_RULE_REVISION, Layout::Bounded1),
        "s2-axis-quantifiers+new-bi/1" => ("s2-axis-quantifiers+new-bi/1", Layout::Sorted),
        "s2-axis-quantifiers+new-bi/1+bounded-layout/1" => ("s2-axis-quantifiers+new-bi/1", Layout::Bounded1),
        "s2-axis-quantifiers+new-bi/2" => ("s2-axis-quantifiers+new-bi/2", Layout::Sorted),
        "s2-axis-quantifiers+new-bi/2+bounded-layout/1" => ("s2-axis-quantifiers+new-bi/2", Layout::Bounded1),
        "s2-axis-quantifiers+new-bi/1+segment-first/1" => ("s2-axis-quantifiers+new-bi/1+segment-first/1", Layout::Sorted),
        "s2-axis-quantifiers+new-bi/1+segment-first/1+bounded-layout/1" => ("s2-axis-quantifiers+new-bi/1+segment-first/1", Layout::Bounded1),
        "s2-axis-quantifiers+new-bi/1+segment-second/1" => ("s2-axis-quantifiers+new-bi/1+segment-second/1", Layout::Sorted),
        "s2-axis-quantifiers+new-bi/1+segment-second/1+bounded-layout/1" => ("s2-axis-quantifiers+new-bi/1+segment-second/1", Layout::Bounded1),
        "s2-axis-quantifiers+new-bi/2+segment-second/1" => ("s2-axis-quantifiers+new-bi/2+segment-second/1", Layout::Sorted),
        "s2-axis-quantifiers+new-bi/2+segment-second/1+bounded-layout/1" => ("s2-axis-quantifiers+new-bi/2+segment-second/1", Layout::Bounded1),
        "s2-axis-quantifiers+new-bi/2+segment-second/2" => ("s2-axis-quantifiers+new-bi/2+segment-second/2", Layout::Sorted),
        "s2-axis-quantifiers+new-bi/2+segment-second/2+bounded-layout/1" => ("s2-axis-quantifiers+new-bi/2+segment-second/2", Layout::Bounded1),
        "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1" => ("s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1", Layout::Sorted),
        "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+bounded-layout/1" => ("s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1", Layout::Bounded1),
        "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2" => ("s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2", Layout::Sorted),
        "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+bounded-layout/1" => ("s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2", Layout::Bounded1),
        "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1" => ("s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1", Layout::Sorted),
        "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+bounded-layout/1" => ("s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1", Layout::Bounded1),
        "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+raw-initialization-w/1" => ("s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+raw-initialization-w/1", Layout::Sorted),
        "s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+raw-initialization-w/1+bounded-layout/1" => ("s2-axis-quantifiers+new-bi/2+segment-second/2+segment-turn/1+second-seq-scan-budget/2+center-seed/1+center-pair/1+raw-initialization-w/1", Layout::Bounded1),
        _ => return Err(format!("VersionUnsupported：未登记完整规则 {rule_revision}")),
    };
    Ok((doctrine, layout))
}

pub(crate) fn split(rule_revision: &str) -> Result<(String, Layout), String> {
    registered(rule_revision).map(|(doctrine, layout)| (doctrine.to_string(), layout))
}

pub(crate) fn doctrine_is(rule_revision: &str, doctrine: &str) -> bool {
    registered(rule_revision).is_ok_and(|(registered, _)| registered == doctrine)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_component_parses_orthogonally() {
        assert_eq!(
            split("s2-axis-quantifiers+new-bi/2").unwrap(),
            ("s2-axis-quantifiers+new-bi/2".to_string(), Layout::Sorted)
        );
        assert_eq!(
            split("s2-axis-quantifiers+new-bi/2+bounded-layout/1").unwrap(),
            ("s2-axis-quantifiers+new-bi/2".to_string(), Layout::Bounded1)
        );
        assert!(split("s2-axis-quantifiers+bounded-layout/1+new-bi/1").is_err());
        assert!(Layout::of_rule("s2-axis-quantifiers+new-bi/2+bounded-layout/2").is_err());
        assert!(
            Layout::of_rule("s2-axis-quantifiers+bounded-layout/1+new-bi/2+bounded-layout/1")
                .is_err()
        );
    }

    #[test]
    fn doctrine_is_matches_split_comparison() {
        let doctrines = ["s2-axis-quantifiers+new-bi/2", "a", "a+b+c"];
        let rules = [
            "s2-axis-quantifiers+new-bi/2",
            "s2-axis-quantifiers+new-bi/2+bounded-layout/1",
            "s2-axis-quantifiers+bounded-layout/1+new-bi/2",
            "bounded-layout/1+s2-axis-quantifiers+new-bi/2",
            "s2-axis-quantifiers+new-bi/2+bounded-layout/2",
            "s2-axis-quantifiers+new-bi/2+bounded-layout/1+bounded-layout/1",
            "s2-axis-quantifiers+new-bi/2+new-bi/2",
            "s2-axis-quantifiers",
            "s2-axis-quantifiers+new-bi/2+",
            "s2-axis-quantifiers++new-bi/2",
            "a",
            "a+bounded-layout/1",
            "a+b",
            "a+b+c",
            "a+b+c+bounded-layout/1",
            "a+b+bounded-layout/1+c",
            "a+b+bounded-layout/",
            "bounded-layout/1",
            "",
        ];
        for doctrine in doctrines {
            for rule in rules {
                assert_eq!(
                    doctrine_is(rule, doctrine),
                    split(rule).is_ok_and(|(d, _)| d == doctrine),
                    "{rule} vs {doctrine}"
                );
            }
        }
    }

    #[test]
    fn epoch_is_birth_order_only_under_bounded_layout() {
        assert_eq!(Layout::Sorted.epoch(7), 0);
        assert_eq!(Layout::Bounded1.epoch(7), 7);
    }

    /// 五族代表键（活动/撤回对象的 OrderKey 形状、见证、关系、观察）：(出生代, 原键)。
    /// 本模块两个 rank 测试只验 [`Layout::rank`] 元组字典序本身，不覆盖六族接线；接线由
    /// `object_index`、`aux_index` 与 `history` 的 `layout` 测试锁。
    fn sorted_by<K: Ord + Clone>(layout: Layout, rows: &[(i64, K)]) -> Vec<usize> {
        let mut order: Vec<usize> = (0..rows.len()).collect();
        order.sort_by(|a, b| {
            let (ga, ka) = &rows[*a];
            let (gb, kb) = &rows[*b];
            layout
                .rank(*ga, ka.clone())
                .cmp(&layout.rank(*gb, kb.clone()))
                .then(a.cmp(b))
        });
        order
    }

    #[test]
    fn layout_rank_sorted_equals_original_key_order_for_all_families() {
        let objects = [
            (3, (0, "", 7)),
            (1, (1, "z", 1)),
            (2, (1, "a", 2)),
            (1, (0, "", 9)),
        ];
        let withdrawn = [(4, (1, "b", 1)), (2, (0, "", 1)), (3, (1, "a", 1))];
        let witnesses = [(2, ("o-b", 1)), (1, ("o-a", 2)), (3, ("o-a", 0))];
        let relations = [
            (2, ("s", "r", "b")),
            (1, ("s", "r", "c")),
            (3, ("a", "r", "x")),
        ];
        let observations = [(5, ("k", Some(9))), (1, ("k", None)), (2, ("j", Some(1)))];
        fn original<K: Ord>(rows: &[(i64, K)]) -> Vec<usize> {
            let mut order: Vec<usize> = (0..rows.len()).collect();
            order.sort_by(|a, b| rows[*a].1.cmp(&rows[*b].1).then(a.cmp(b)));
            order
        }
        assert_eq!(sorted_by(Layout::Sorted, &objects), original(&objects));
        assert_eq!(sorted_by(Layout::Sorted, &withdrawn), original(&withdrawn));
        assert_eq!(sorted_by(Layout::Sorted, &witnesses), original(&witnesses));
        assert_eq!(sorted_by(Layout::Sorted, &relations), original(&relations));
        assert_eq!(
            sorted_by(Layout::Sorted, &observations),
            original(&observations)
        );
    }

    #[test]
    fn layout_rank_bounded_is_generation_monotone_then_original_key() {
        // 修订对象（新出生代 3）落尾；撤回代较晚者移入撤回尾；同代内沿原键。
        let objects = [
            (3, (0, "", 7)),
            (1, (1, "z", 1)),
            (1, (0, "", 9)),
            (2, (1, "a", 2)),
        ];
        assert_eq!(sorted_by(Layout::Bounded1, &objects), vec![2, 1, 3, 0]);
        let withdrawn = [(4, (0, "", 1)), (2, (1, "b", 1)), (2, (1, "a", 1))];
        assert_eq!(sorted_by(Layout::Bounded1, &withdrawn), vec![2, 1, 0]);
        for rows in [&objects[..], &withdrawn[..]] {
            let order = sorted_by(Layout::Bounded1, rows);
            assert!(order.windows(2).all(|w| {
                let (a, b) = (&rows[w[0]], &rows[w[1]]);
                a.0 < b.0 || (a.0 == b.0 && a.1 <= b.1)
            }));
        }
    }
}
