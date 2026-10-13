//! #1419 TB-02-E：共用中枢扫描的事实侧车（PLAN S4，#804 L2）。
//!
//! 生产塔与 S 事实层取中枢数据只走一份扫描：[`super::recursive_tower::scan_center_facts`]。塔配
//! [`NoopSink`]（不算事实、不分配），S 写者配 [`FactSink`] 收集每次 seed 评估与每个成立窗口，再由
//! 本模块派生发布所需的事实：中枢六变量与贡献成员（[`CenterWindowFacts`]）、每单元归属
//! （[`UnitFacts`]）、扫描游标（[`ScanState`]）。派生只读扫描报出的窗口与 [`super::center`] 的同一组
//! 判据函数，不另写 seed、延伸或 Zn 判据。
//!
//! 归属规则来自根裁定（`ROOT-RULINGS-1419.md` D9/D-G/D-I 与 r2 第 6 条 R2/R3/R6），原文未讲到这一层：
//!
//! - 成员 ⟹ `CENTER_MEMBER`；≥9 成员窗口内的单元 ⟹ `NOT_YET_ASSIGNED/UPGRADE_PENDING_TB02H`（R3，D-I）。
//! - 结算中枢 A 之后、下一中枢 B 之前：B 已成立 ⟹ 全部 `CONNECTOR(A→B)`；B 未成立 ⟹ 突破段与「以自身为
//!   起点的 seed 已评估且失败」的单元 `CONNECTOR(A→pending)`，其余 `NOT_YET_ASSIGNED/SEED_NOT_YET_EVALUABLE`。
//! - 首个中枢之前，以及尚无任何中枢时 ⟹ `NOT_YET_ASSIGNED/NO_LEFT_CENTER_IN_WINDOW`（R2），区分信息放
//!   `seed_from_self`；突破段的 `seed_from_self` 记 `NOT_EVALUATED`（R6）。
//! - 进行中单元（不在扫描输入里）由调用方记 `NOT_YET_ASSIGNED/UNIT_NOT_COMPLETED`。
//!
//! 等待原因 `UNIT_NOT_COMPLETED`/`SEED_NOT_YET_EVALUABLE` 是 S0 oracle 提议名，按 r2 R1 并入 D-J schema 独审定名。

use super::super::types::{Center, Segment, Tick};
use super::center::{breaks_upward, is_zn, CenterRoles, RoleVar, SeedFacts, UnitRange};
use super::recursive_tower::{scan_center_facts, WinMeta, WindowScanCursor};

/// 生产塔 L0 的单元列表：同一层已确认线段逐个经塔同一投影（`pipeline::segment_to_unit`）。S 事实层用它与
/// 已发布的 `CC-013.segment` 逐位对账（schema v1 §1 输入合同），保证与塔扫描同一输入（#804 L3）。
pub fn l0_units(segments: &[Segment]) -> Vec<UnitRange> {
    segments
        .iter()
        .map(super::pipeline::segment_to_unit)
        .collect()
}

/// 扫描事件接收方。`WANTS_FACTS == false` 时扫描不计算 [`SeedFacts`]。
pub trait ScanSink {
    /// 是否需要 seed 事实。
    const WANTS_FACTS: bool;

    /// 以 `at` 为起点评估了一次 seed；`formed` 为扫描所用 build 的判定。
    #[inline]
    fn seed(&mut self, _at: usize, _facts: &SeedFacts, _formed: bool) {}

    /// 一个成立窗口扫完（结算或延伸到输入末尾）。
    #[inline]
    fn window(&mut self, _window: ScanWindow) {}
}

/// 塔路径：什么都不收。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoopSink;

impl ScanSink for NoopSink {
    const WANTS_FACTS: bool = false;
}

/// 一个成立窗口：成员 = `units[seed_at..exit]`；`exit < len` ⟹ `units[exit]` 是突破段、中枢已结算。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScanWindow {
    /// seed 第一单元下标（首成员）。
    pub seed_at: usize,
    /// 首个不延伸的单元下标；等于输入长度时窗口开放。
    pub exit: usize,
    /// 成员数达到 `UPGRADE_TOTAL_SEGMENTS`（≥9），塔按第33课重切子中枢。
    pub upgraded: bool,
    /// 本窗口塔侧产出的中枢数（<9 为 1）。
    pub emitted: usize,
}

/// 一次 seed 评估。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SeedAttempt {
    /// seed 第一单元下标。
    pub at: usize,
    /// 三单元几何事实。
    pub facts: SeedFacts,
    /// 是否成立（扫描所用 build 的判定）。
    pub formed: bool,
}

/// 收集全部事件。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FactSink {
    pub attempts: Vec<SeedAttempt>,
    pub windows: Vec<ScanWindow>,
}

impl ScanSink for FactSink {
    const WANTS_FACTS: bool = true;

    fn seed(&mut self, at: usize, facts: &SeedFacts, formed: bool) {
        self.attempts.push(SeedAttempt {
            at,
            facts: *facts,
            formed,
        });
    }

    fn window(&mut self, window: ScanWindow) {
        self.windows.push(window);
    }
}

/// 一次全量扫描（自下标 0 起）的输出与事实。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CenterScanFacts {
    /// 输入单元数（全部已完成）。
    pub len: usize,
    /// 塔侧输出（与 `detect_centers_windowed_resume(units, build, 0)` 逐位相同）。
    pub centers: Vec<(Center, (usize, usize))>,
    pub metas: Vec<WinMeta>,
    pub cursor: WindowScanCursor,
    /// 每次 seed 评估，按扫描顺序。
    pub attempts: Vec<SeedAttempt>,
    /// 每个成立窗口，按扫描顺序。
    pub windows: Vec<ScanWindow>,
}

/// 对已完成单元做一次全量扫描并收集事实。`build` 取塔同级所用的判据（L0 = `center_from_segments`）。
pub fn scan_center_facts_full(
    units: &[UnitRange],
    build: fn(&UnitRange, &UnitRange, &UnitRange) -> Option<Center>,
) -> CenterScanFacts {
    let mut sink = FactSink::default();
    let (centers, metas, cursor) = scan_center_facts(units, build, 0, &mut sink);
    CenterScanFacts {
        len: units.len(),
        centers,
        metas,
        cursor,
        attempts: sink.attempts,
        windows: sink.windows,
    }
}

/// 一个角色变量的发布事实（下标为单元在输入中的绝对下标）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleFact {
    pub var: RoleVar,
    pub value: Tick,
    /// 取到该值的成员，升序。
    pub contributors: Vec<usize>,
}

impl RoleFact {
    /// 同价见证：贡献成员 ≥2 时为其全体，否则空。
    pub fn same_price_witness(&self) -> &[usize] {
        if self.contributors.len() >= 2 {
            &self.contributors
        } else {
            &[]
        }
    }
}

/// 一个成立窗口的发布事实（CC-016/017/022 的数据源）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CenterWindowFacts {
    /// 首成员 = seed 第一单元。
    pub first: usize,
    /// 末成员。
    pub last: usize,
    /// 是否已结算（有突破段）。
    pub settled: bool,
    /// 突破段（结算时）。
    pub break_unit: Option<usize>,
    /// 离开方向（结算时；未结算无定义，根裁定 r2 R4）。
    pub break_up: Option<bool>,
    /// 续扫锚 = last+2（结算时，R8）。
    pub resume_anchor: Option<usize>,
    /// 下一个待测延伸的单元（开放时 = 输入长度）。
    pub next_tested_unit: Option<usize>,
    /// ≥9 成员：已发布对象标 `UPGRADE_PENDING_TB02H`，DD/GG/D/G 不发布（R3）。
    pub upgrade_pending: bool,
    /// 与 seed 第一单元同向的成员，升序。
    pub zn_members: Vec<usize>,
    /// 六变量，顺序同 [`RoleVar::ALL`]。
    pub roles: Vec<RoleFact>,
}

impl CenterWindowFacts {
    /// 成员数。
    pub fn count(&self) -> usize {
        self.last - self.first + 1
    }

    /// 取某一角色变量。
    pub fn role(&self, var: RoleVar) -> &RoleFact {
        self.roles
            .iter()
            .find(|r| r.var == var)
            .expect("六变量齐全")
    }
}

/// 某单元以自身为起点的 seed 评估结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeedFromSelf {
    Formed,
    Failed,
    NotEvaluated,
}

impl SeedFromSelf {
    pub fn as_str(self) -> &'static str {
        match self {
            SeedFromSelf::Formed => "FORMED",
            SeedFromSelf::Failed => "FAILED",
            SeedFromSelf::NotEvaluated => "NOT_EVALUATED",
        }
    }
}

/// `NOT_YET_ASSIGNED` 的等待原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WaitReason {
    NoLeftCenterInWindow,
    SeedNotYetEvaluable,
    UpgradePendingTb02h,
    UnitNotCompleted,
}

impl WaitReason {
    pub fn as_str(self) -> &'static str {
        match self {
            WaitReason::NoLeftCenterInWindow => "NO_LEFT_CENTER_IN_WINDOW",
            WaitReason::SeedNotYetEvaluable => "SEED_NOT_YET_EVALUABLE",
            WaitReason::UpgradePendingTb02h => "UPGRADE_PENDING_TB02H",
            WaitReason::UnitNotCompleted => "UNIT_NOT_COMPLETED",
        }
    }
}

/// 单元归属三值（恰一）。中枢以窗口序号（[`CenterScanFacts::windows`] 下标）指称。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnitAssignment {
    Member {
        center: usize,
    },
    /// `right == None` ⟹ 右端中枢尚未成立（pending）。
    Connector {
        left: usize,
        right: Option<usize>,
    },
    NotYetAssigned(WaitReason),
}

/// 一个单元的归属与 seed 记录。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitFacts {
    pub assignment: UnitAssignment,
    pub seed_from_self: SeedFromSelf,
}

/// 扫描停在哪里。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScanState {
    /// 自 `seed_start` 起的 seed 还缺单元，下一个需要完成的是 `awaiting_unit`。
    SeekingSeed {
        seed_start: usize,
        awaiting_unit: usize,
    },
    /// 末个中枢（窗口序号 `center`）开放，下一个待测延伸的是 `next_unit`。
    Extending { center: usize, next_unit: usize },
}

impl CenterScanFacts {
    /// 每个成立窗口的发布事实。`units` 必须是本次扫描的输入。
    pub fn window_facts(&self, units: &[UnitRange]) -> Vec<CenterWindowFacts> {
        assert_eq!(units.len(), self.len, "units 必须是本次扫描的输入");
        self.windows
            .iter()
            .map(|w| {
                let members = &units[w.seed_at..w.exit];
                let roles = CenterRoles::of_members(members);
                let settled = w.exit < self.len;
                CenterWindowFacts {
                    first: w.seed_at,
                    last: w.exit - 1,
                    settled,
                    break_unit: settled.then_some(w.exit),
                    break_up: settled.then(|| breaks_upward(&units[w.exit], roles.zd, roles.zg)),
                    resume_anchor: settled.then_some(w.exit + 1),
                    next_tested_unit: (!settled).then_some(w.exit),
                    upgrade_pending: w.upgraded,
                    zn_members: members
                        .iter()
                        .enumerate()
                        .filter(|(_, u)| is_zn(u, &members[0]))
                        .map(|(k, _)| w.seed_at + k)
                        .collect(),
                    roles: RoleVar::ALL
                        .iter()
                        .map(|&var| RoleFact {
                            var,
                            value: roles.value(var),
                            contributors: roles
                                .contributors(members, var)
                                .into_iter()
                                .map(|k| w.seed_at + k)
                                .collect(),
                        })
                        .collect(),
                }
            })
            .collect()
    }

    /// 单元 `k` 以自身为起点的 seed 记录。
    pub fn seed_from_self(&self, k: usize) -> SeedFromSelf {
        match self.attempts.iter().find(|a| a.at == k) {
            Some(a) if a.formed => SeedFromSelf::Formed,
            Some(_) => SeedFromSelf::Failed,
            None => SeedFromSelf::NotEvaluated,
        }
    }

    /// 每个已完成单元的归属（下标 0..len）。进行中单元不在其内，由调用方记 `UNIT_NOT_COMPLETED`。
    pub fn unit_facts(&self) -> Vec<UnitFacts> {
        (0..self.len)
            .map(|k| UnitFacts {
                assignment: self.assignment_of(k),
                seed_from_self: self.seed_from_self(k),
            })
            .collect()
    }

    fn assignment_of(&self, k: usize) -> UnitAssignment {
        if let Some((idx, w)) = self
            .windows
            .iter()
            .enumerate()
            .find(|(_, w)| w.seed_at <= k && k < w.exit)
        {
            return if w.upgraded {
                UnitAssignment::NotYetAssigned(WaitReason::UpgradePendingTb02h)
            } else {
                UnitAssignment::Member { center: idx }
            };
        }
        // 左邻中枢 = 末成员在 k 之前的最后一个窗口；没有 ⟹ 首中枢之前或尚无中枢（R2）。
        let Some(left) = self.windows.iter().rposition(|w| w.exit <= k) else {
            return UnitAssignment::NotYetAssigned(WaitReason::NoLeftCenterInWindow);
        };
        if let Some(right) = (left + 1 < self.windows.len()).then_some(left + 1) {
            return UnitAssignment::Connector {
                left,
                right: Some(right),
            };
        }
        let is_breakout = self.windows[left].exit == k;
        if is_breakout || self.seed_from_self(k) == SeedFromSelf::Failed {
            UnitAssignment::Connector { left, right: None }
        } else {
            UnitAssignment::NotYetAssigned(WaitReason::SeedNotYetEvaluable)
        }
    }

    /// 扫描停止处。
    pub fn scan_state(&self) -> ScanState {
        match self.windows.last() {
            Some(w) if w.exit == self.len => ScanState::Extending {
                center: self.windows.len() - 1,
                next_unit: self.len,
            },
            _ => ScanState::SeekingSeed {
                seed_start: self.cursor.consumed,
                awaiting_unit: self.cursor.consumed + 2,
            },
        }
    }
}
