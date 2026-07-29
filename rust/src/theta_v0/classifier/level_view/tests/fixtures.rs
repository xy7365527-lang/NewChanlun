use super::*;

pub(super) fn unit(start: usize, dir: Direction, lo: Tick, hi: Tick, ordinal: u64) -> LeveledMove {
    LeveledMove::from_unit(
        &UnitRange {
            start_index: start,
            end_index: start + 9,
            direction: dir,
            lo,
            hi,
        },
        ElementId { level: 0, ordinal },
    )
}

pub(super) fn extended_windows() -> (Vec<LeveledMove>, Vec<LeveledMove>) {
    use Direction::{Down, Up};
    let lower = vec![
        unit(0, Up, 90, 110, 0),
        unit(10, Down, 95, 115, 1),
        unit(20, Up, 98, 112, 2),
        unit(30, Down, 96, 116, 3),
        unit(40, Up, 130, 145, 4),
        unit(50, Down, 132, 148, 5),
        unit(60, Up, 135, 150, 6),
        unit(70, Down, 125, 140, 7),
        unit(80, Up, 155, 170, 8),
        unit(90, Down, 150, 165, 9),
        unit(100, Up, 160, 175, 10),
        unit(110, Down, 140, 155, 11),
        unit(120, Up, 180, 190, 12),
        // ★R1 夹具补腿：回试段（Down，低点 170 > w2.zg=165 不重回核心）——与腿12（Up，
        // 端点 190 > 165 破核心）构成对 w2 的第三类买点（037:18），供全合取确认测试；
        // 旧 as_of=129 的既有测试按 end ≤ as_of 过滤本腿，行为不变。
        unit(130, Down, 170, 195, 13),
    ];
    let w0 = LeveledMove::compose(
        &lower[0..4],
        Center {
            zd: 98,
            zg: 110,
            dd: 90,
            gg: 116,
            start_index: 0,
            end_index: 39,
        },
        1,
        ElementId {
            level: 1,
            ordinal: 0,
        },
    );
    let w1 = LeveledMove::compose(
        &lower[4..8],
        Center {
            zd: 135,
            zg: 140,
            dd: 125,
            gg: 150,
            start_index: 40,
            end_index: 79,
        },
        1,
        ElementId {
            level: 1,
            ordinal: 1,
        },
    );
    let w2 = LeveledMove::compose(
        &lower[8..12],
        Center {
            zd: 160,
            zg: 165,
            dd: 140,
            gg: 175,
            start_index: 80,
            end_index: 119,
        },
        1,
        ElementId {
            level: 1,
            ordinal: 2,
        },
    );
    (vec![w0, w1, w2], lower)
}

pub(super) fn mbar(source_index: usize) -> Bar {
    Bar {
        source_index,
        timestamp: source_index as i64,
        open: 0,
        high: 0,
        low: 0,
        close: 0,
        volume: 0,
        untradable: false,
    }
}

pub(super) fn trend_block(dir: Option<Direction>) -> MoveBlock {
    MoveBlock {
        start_center: 0,
        end_center: 2,
        kind: if dir.is_some() {
            MoveKind::Trend
        } else {
            MoveKind::Consolidation
        },
        dir,
        status: MoveStatus::Completed,
    }
}

pub(super) fn query(version: C2VersionTuple) -> LevelViewQuery {
    LevelViewQuery {
        level: 1,
        coordinate_window: CoordinateWindow { start: 0, end: 129 },
        as_of: 129,
        version,
    }
}
