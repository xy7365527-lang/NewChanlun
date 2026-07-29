use super::*;

pub(super) struct PanProviderFixture {
    pub(super) projection: ExactThreeProjection,
    pub(super) blocks: Vec<MoveBlock>,
    pub(super) legs: Vec<LowerLeg>,
    pub(super) view: LevelAsOfView,
    pub(super) hist: Vec<f64>,
    pub(super) dif: Vec<f64>,
    pub(super) close_src: Vec<usize>,
}

fn pan_seed(index: usize, center: Center) -> ExactThreeSeed {
    ExactThreeSeed {
        source_id: ElementId {
            level: 1,
            ordinal: index as u64,
        },
        source_sub_count: 3,
        start_index: center.start_index,
        end_index: center.end_index,
        center,
        core_provenance: SeedCoreProvenance::SelfConsistent,
    }
}

pub(super) fn pan_provider_fixture() -> PanProviderFixture {
    let centers = [
        Center {
            zd: 100,
            zg: 200,
            dd: 90,
            gg: 210,
            start_index: 0,
            end_index: 2,
        },
        Center {
            zd: 300,
            zg: 400,
            dd: 290,
            gg: 410,
            start_index: 3,
            end_index: 5,
        },
        Center {
            zd: 350,
            zg: 450,
            dd: 250,
            gg: 460,
            start_index: 4,
            end_index: 8,
        },
        Center {
            zd: 500,
            zg: 550,
            dd: 490,
            gg: 560,
            start_index: 12,
            end_index: 14,
        },
        Center {
            zd: 600,
            zg: 650,
            dd: 590,
            gg: 660,
            start_index: 15,
            end_index: 17,
        },
        Center {
            zd: 580,
            zg: 640,
            dd: 570,
            gg: 670,
            start_index: 18,
            end_index: 20,
        },
        Center {
            zd: 700,
            zg: 750,
            dd: 690,
            gg: 760,
            start_index: 21,
            end_index: 23,
        },
    ];
    let projection = ExactThreeProjection {
        version: ProviderVersion::EXTENDED_TO_EXACT_THREE_V3,
        seeds: centers
            .iter()
            .copied()
            .enumerate()
            .map(|(index, center)| pan_seed(index, center))
            .collect(),
    };
    let blocks = vec![
        MoveBlock {
            start_center: 0,
            end_center: 2,
            kind: MoveKind::Consolidation,
            dir: None,
            status: MoveStatus::Completed,
        },
        MoveBlock {
            start_center: 2,
            end_center: 4,
            kind: MoveKind::Trend,
            dir: Some(Direction::Up),
            status: MoveStatus::Completed,
        },
        MoveBlock {
            start_center: 4,
            end_center: 6,
            kind: MoveKind::Consolidation,
            dir: None,
            status: MoveStatus::Active,
        },
    ];
    let legs = vec![
        LowerLeg {
            id: ElementId {
                level: 0,
                ordinal: 0,
            },
            direction: Direction::Down,
            start_index: 1,
            end_index: 3,
            lo: 360,
            hi: 460,
        },
        LowerLeg {
            id: ElementId {
                level: 0,
                ordinal: 1,
            },
            direction: Direction::Down,
            start_index: 9,
            end_index: 11,
            lo: 300,
            hi: 380,
        },
    ];
    let mut hist = vec![0.0; 32];
    hist[1..=3].fill(-5.0);
    hist[9] = -1.0;
    hist[10] = 20.0;
    hist[11] = -1.0;
    let dif = vec![0.0; 32];
    let close_src: Vec<_> = (0..32).collect();
    let query = LevelViewQuery {
        level: 1,
        coordinate_window: CoordinateWindow { start: 0, end: 23 },
        as_of: 31,
        version: C2VersionTuple::auto_pairing(),
    };
    let view = LevelAsOfView {
        query,
        cache_key: C2CacheKey::from_query(&query).unwrap(),
        moves: Vec::new(),
        pairs: Vec::new(),
        pair_confirmations: Vec::new(),
    };
    PanProviderFixture {
        projection,
        blocks,
        legs,
        view,
        hist,
        dif,
        close_src,
    }
}

/// 票 #497：19 处调用共享 `level=1`；折叠该常量参数，语义与直调
/// `provide_nest_candidate_events_resident(1, ...)` 逐字等价。
#[allow(clippy::too_many_arguments)]
pub(super) fn pan_resident(
    projection: &ExactThreeProjection,
    blocks: &[MoveBlock],
    legs: &[LowerLeg],
    view: &LevelAsOfView,
    hist: &[f64],
    dif: &[f64],
    close_src: &[usize],
    residence: Option<PanResidence<'_>>,
) -> Vec<NestCandidateEvent> {
    provide_nest_candidate_events_resident(
        1, projection, blocks, legs, view, hist, dif, close_src, residence,
    )
}
