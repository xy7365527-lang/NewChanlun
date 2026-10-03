// #1467 R_W-DC研究候选：固定绝对阈值的方向变换基本单元。
// 单元不是既有缠论笔或线段；没有生产/交易消费者。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub index: usize,
    pub value: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unit {
    pub direction: Direction,
    pub start: Point,
    pub end: Point,
    pub known_at: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Active {
    Undecided { low: Point, high: Point },
    Up { start: Point, peak: Point },
    Down { start: Point, trough: Point },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Constructor {
    pub delta: i64,
    pub history: Vec<i64>,
    pub initialized_at: Option<usize>,
    pub active: Option<Active>,
    pub units: Vec<Unit>,
}

fn rise(a: i64, b: i64, delta: i64) -> bool {
    (a as i128) - (b as i128) >= delta as i128
}

impl Constructor {
    pub fn new(delta: i64) -> Self {
        assert!(delta > 0);
        Self {
            delta,
            history: Vec::new(),
            initialized_at: None,
            active: None,
            units: Vec::new(),
        }
    }
    pub fn push(&mut self, value: i64) {
        let p = Point {
            index: self.history.len(),
            value,
        };
        self.history.push(value);
        self.active = Some(match self.active {
            None => Active::Undecided { low: p, high: p },
            Some(Active::Undecided { mut low, mut high }) => {
                if value < low.value {
                    low = p;
                }
                if value > high.value {
                    high = p;
                }
                let up = rise(value, low.value, self.delta);
                let down = rise(high.value, value, self.delta);
                assert!(!(up && down), "undecided-range invariant violated");
                if up {
                    self.initialized_at = Some(p.index);
                    Active::Up {
                        start: low,
                        peak: p,
                    }
                } else if down {
                    self.initialized_at = Some(p.index);
                    Active::Down {
                        start: high,
                        trough: p,
                    }
                } else {
                    Active::Undecided { low, high }
                }
            }
            Some(Active::Up { start, peak }) => {
                if value > peak.value {
                    Active::Up { start, peak: p }
                } else if rise(peak.value, value, self.delta) {
                    self.units.push(Unit {
                        direction: Direction::Up,
                        start,
                        end: peak,
                        known_at: p.index,
                    });
                    Active::Down {
                        start: peak,
                        trough: p,
                    }
                } else {
                    Active::Up { start, peak }
                }
            }
            Some(Active::Down { start, trough }) => {
                if value < trough.value {
                    Active::Down { start, trough: p }
                } else if rise(value, trough.value, self.delta) {
                    self.units.push(Unit {
                        direction: Direction::Down,
                        start,
                        end: trough,
                        known_at: p.index,
                    });
                    Active::Up {
                        start: trough,
                        peak: p,
                    }
                } else {
                    Active::Down { start, trough }
                }
            }
        });
    }
}

// 以下参考是按首次命中与区间极值的定义穷举，不运行上面的增量状态机。
fn extreme(xs: &[i64], start: usize, end: usize, high: bool) -> Point {
    let mut out = Point {
        index: start,
        value: xs[start],
    };
    for i in start + 1..=end {
        if (high && xs[i] > out.value) || (!high && xs[i] < out.value) {
            out = Point {
                index: i,
                value: xs[i],
            };
        }
    }
    out
}

pub fn reference(xs: &[i64], delta: i64) -> Vec<Unit> {
    assert!(delta > 0);
    let mut initial = None;
    for i in 1..xs.len() {
        let lo = extreme(xs, 0, i, false);
        let hi = extreme(xs, 0, i, true);
        if rise(hi.value, lo.value, delta) {
            initial = Some(if hi.index == i {
                (Direction::Up, lo)
            } else {
                assert_eq!(lo.index, i);
                (Direction::Down, hi)
            });
            break;
        }
    }
    let Some((mut direction, mut start)) = initial else {
        return Vec::new();
    };
    let mut units = Vec::new();
    let mut previous_confirmation = None;
    loop {
        let mut found = None;
        for t in start.index + 1..xs.len() {
            let e = extreme(xs, start.index, t - 1, direction == Direction::Up);
            let hit = match direction {
                Direction::Up => rise(e.value, xs[t], delta),
                Direction::Down => rise(xs[t], e.value, delta),
            };
            if hit {
                found = Some((e, t));
                break;
            }
        }
        let Some((end, t)) = found else { break };
        assert!(end.index > start.index);
        if let Some(old) = previous_confirmation {
            assert!(t > old);
        }
        units.push(Unit {
            direction,
            start,
            end,
            known_at: t,
        });
        previous_confirmation = Some(t);
        start = end;
        direction = if direction == Direction::Up {
            Direction::Down
        } else {
            Direction::Up
        };
    }
    units
}

pub fn reference_active(xs: &[i64], delta: i64) -> (Option<usize>, Option<Active>) {
    assert!(delta > 0);
    if xs.is_empty() {
        return (None, None);
    }
    let mut initial = None;
    for i in 1..xs.len() {
        let lo = extreme(xs, 0, i, false);
        let hi = extreme(xs, 0, i, true);
        if rise(hi.value, lo.value, delta) {
            initial = Some((
                i,
                if hi.index == i {
                    Direction::Up
                } else {
                    Direction::Down
                },
                if hi.index == i { lo } else { hi },
            ));
            break;
        }
    }
    let Some((t, initial_direction, initial_start)) = initial else {
        return (
            None,
            Some(Active::Undecided {
                low: extreme(xs, 0, xs.len() - 1, false),
                high: extreme(xs, 0, xs.len() - 1, true),
            }),
        );
    };
    let units = reference(xs, delta);
    let (direction, start) = if let Some(last) = units.last() {
        (
            if last.direction == Direction::Up {
                Direction::Down
            } else {
                Direction::Up
            },
            last.end,
        )
    } else {
        (initial_direction, initial_start)
    };
    let e = extreme(xs, start.index, xs.len() - 1, direction == Direction::Up);
    (
        Some(t),
        Some(if direction == Direction::Up {
            Active::Up { start, peak: e }
        } else {
            Active::Down { start, trough: e }
        }),
    )
}

pub fn audit(xs: &[i64], delta: i64, units: &[Unit]) {
    for u in units {
        assert!(u.start.index < u.end.index && u.end.index < u.known_at && u.known_at < xs.len());
        assert_eq!(xs[u.start.index], u.start.value);
        assert_eq!(xs[u.end.index], u.end.value);
        let (lo, hi) = if u.direction == Direction::Up {
            (u.start.value, u.end.value)
        } else {
            (u.end.value, u.start.value)
        };
        assert!(rise(hi, lo, delta));
        assert!(xs[u.start.index..=u.end.index]
            .iter()
            .all(|&p| lo <= p && p <= hi));
    }
    for pair in units.windows(2) {
        assert_eq!(pair[0].end, pair[1].start);
        assert_ne!(pair[0].direction, pair[1].direction);
        assert!(pair[0].known_at < pair[1].known_at);
    }
}
