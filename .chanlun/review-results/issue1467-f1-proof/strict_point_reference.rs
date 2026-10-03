// #1467 R_W-v1研究参考：只用于点K输入，不是生产补丁。
// 成笔必须同时满足合并K不共用、原始极值间隔、顶高于底。
// 极值候选的在线贪心更新为明确的研究选择，不冒称原文唯一规定。
use newchan_rust::theta_v0::types::{Bar, Direction, Stroke};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Group {
    pub first: usize,
    pub last: usize,
    pub price: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    pub top: bool,
    pub raw: usize,
    pub merged: usize,
    pub price: i64,
    pub known_at: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reference {
    pub groups: Vec<Group>,
    pub fractals: Vec<Node>,
    pub anchors: Vec<Node>,
    pub strokes: Vec<Stroke>,
}

fn pair_ok(a: &Node, b: &Node) -> bool {
    a.top != b.top
        && b.merged >= a.merged + 3
        && b.raw > a.raw + 3
        && if a.top {
            a.price > b.price
        } else {
            b.price > a.price
        }
}

pub fn build(bars: &[Bar]) -> Reference {
    let mut groups: Vec<Group> = Vec::new();
    for (i, b) in bars.iter().enumerate() {
        assert_eq!(
            (b.open, b.high, b.low, b.close),
            (b.close, b.close, b.close, b.close)
        );
        assert_eq!(
            b.source_index, i,
            "reference domain uses contiguous observation indices"
        );
        if let Some(g) = groups.last_mut().filter(|g| g.price == b.close) {
            g.last = i;
        } else {
            groups.push(Group {
                first: i,
                last: i,
                price: b.close,
            });
        }
    }
    let mut fractals = Vec::new();
    for i in 1..groups.len().saturating_sub(1) {
        let (l, m, r) = (&groups[i - 1], &groups[i], &groups[i + 1]);
        let top = m.price > l.price && m.price > r.price;
        let bottom = m.price < l.price && m.price < r.price;
        if top || bottom {
            fractals.push(Node {
                top,
                raw: m.first,
                merged: i,
                price: m.price,
                known_at: r.first,
            });
        }
    }
    let mut anchors: Vec<Node> = Vec::new();
    for f in &fractals {
        match anchors.last() {
            None => anchors.push(f.clone()),
            Some(last) if last.top == f.top => {
                let extends = if f.top {
                    f.price > last.price
                } else {
                    f.price < last.price
                };
                if extends {
                    *anchors.last_mut().unwrap() = f.clone();
                }
                // 同价保留更早的原始极值位置，本版本明确选择。
            }
            Some(last) if pair_ok(last, f) => anchors.push(f.clone()),
            Some(_) => {} // 尚不成笔，不丢掉当前锚点。
        }
    }
    let strokes = anchors
        .windows(2)
        .map(|w| Stroke {
            direction: if w[0].top {
                Direction::Down
            } else {
                Direction::Up
            },
            start_index: w[0].raw,
            end_index: w[1].raw,
            start_price: w[0].price,
            end_price: w[1].price,
        })
        .collect();
    Reference {
        groups,
        fractals,
        anchors,
        strokes,
    }
}
