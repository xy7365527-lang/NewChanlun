//! #1491（ADR 0037 §五/§六）：S 发布内容布局组件——由库内 `rule_revision` 唯一解析。
//!
//! 组合规则 `<base>+<教义组件…>+bounded-layout/<n>`：组件正交，布局组件只定快照累计数组次序
//! 与 CC-007 事实承载，不改目录轴、不改教义判定。串内无 `bounded-layout` 组件 ⟹ ADR 0037 之前的
//! 写者，按定义是 layout/1（`Layout::Sorted`），不是兜底；出现未知 `bounded-layout/<n>` 一律拒绝。
//! 读写两侧只经 [`Layout::of_rule`] 取布局，再由 [`Layout::epoch`] 给出公开主序的代际前缀，
//! 两种布局不接在同一 if-else 上作准入兜底（#799/#804）。

/// 布局组件名（含分隔符的前缀）；版本号紧随其后。
const COMPONENT: &str = "bounded-layout/";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Layout {
    /// layout/1：累计数组按内容键排序（ADR 0037 之前全部 v3 库；只读）。
    Sorted,
    /// bounded-layout/1：累计数组按出生序（代际单调不减，同代内沿 layout/1 原键）。
    Bounded1,
}

// #1491：发布比较器（object_index/aux_index 公开主序）接线前仅测试消费；接线后删除本豁免。
#[cfg_attr(not(test), allow(dead_code))]
impl Layout {
    /// 从完整 `rule_revision` 串取布局组件；同串至多一个布局组件。
    pub(crate) fn of_rule(rule_revision: &str) -> Result<Self, String> {
        Ok(split(rule_revision)?.1)
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
}

/// 去掉布局组件后的规则串（教义组件部分）与布局；目录轴表等教义侧判据只看前者。
pub(crate) fn split(rule_revision: &str) -> Result<(String, Layout), String> {
    let mut layout = None;
    let mut rest: Vec<&str> = Vec::new();
    for component in rule_revision.split('+') {
        match component.strip_prefix(COMPONENT) {
            Some(version) => {
                let parsed = match version {
                    "1" => Layout::Bounded1,
                    _ => {
                        return Err(format!(
                            "VersionUnsupported：未知布局组件 {component}（rule_revision {rule_revision}）"
                        ))
                    }
                };
                if layout.replace(parsed).is_some() {
                    return Err(format!(
                        "VersionUnsupported：rule_revision {rule_revision} 含多个布局组件"
                    ));
                }
            }
            None => rest.push(component),
        }
    }
    Ok((rest.join("+"), layout.unwrap_or(Layout::Sorted)))
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
        // 组件正交：位置不改语义。
        assert_eq!(
            split("s2-axis-quantifiers+bounded-layout/1+new-bi/1").unwrap(),
            ("s2-axis-quantifiers+new-bi/1".to_string(), Layout::Bounded1)
        );
        assert!(Layout::of_rule("s2-axis-quantifiers+new-bi/2+bounded-layout/2").is_err());
        assert!(
            Layout::of_rule("s2-axis-quantifiers+bounded-layout/1+new-bi/2+bounded-layout/1")
                .is_err()
        );
    }

    #[test]
    fn epoch_is_birth_order_only_under_bounded_layout() {
        assert_eq!(Layout::Sorted.epoch(7), 0);
        assert_eq!(Layout::Bounded1.epoch(7), 7);
    }
}
