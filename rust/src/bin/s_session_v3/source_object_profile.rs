use super::sc;
use std::{cell::RefCell, time::Instant};
const KINDS: &[&str] = &["CC-004.inclusion_step", "CC-005.inclusion_group", "CC-006.local_shape", "CC-007.fractal_description", "CC-007.development_batch", "CC-054.knowledge_state", "CC-004.initialization", "CC-004.undetermined_raw", "CC-008.endpoint", "CC-008.new_bi_pair", "CC-009.same_kind", "CC-010.bi", "CC-055.relation", "CC-055.change_event", "CC-056.version", "CC-011.segment_seed", "CC-012.feature_sequence", "CC-013.segment", "CC-055.segment_relation", "CC-055.segment_change", "CC-056.segment_version", "CC-011.seed_candidate", "CC-016.center_seed_attempt", "CC-016.center", "CC-017.center_roles", "CC-022.unit_assignment", "CC-055.center_relation", "CC-055.center_change", "CC-056.center_version", "CC-018.unit_core_position", "CC-020.center_pair", "CC-055.pair_relation", "CC-055.pair_change", "CC-056.pair_version", "other"];
const PARTS: [&str; 7] = ["fields_identity_slot", "json_coords", "payload_open", "input_references", "fact_key", "typed_identity", "payload_business"];
#[derive(Clone, Copy, Default)]
struct Count { calls: u64, nanos: u64, frames: u64 }
struct Profile { generation: i64, counts: [[Count; 7]; KINDS.len()] }
thread_local! { static PROFILE: RefCell<Option<Profile>> = const { RefCell::new(None) }; }
pub(super) fn begin(generation: i64) {
    let active = (60..=76).contains(&generation) && std::env::var("W2_OBJECT_PROFILE").is_ok_and(|v| v == "1");
    PROFILE.with(|p| *p.borrow_mut() = active.then(|| Profile {generation, counts: [[Count::default(); 7]; KINDS.len()]}));
}
pub(super) struct ObjectProbe { kind: usize, mark: Option<(Instant, u64)> }
impl ObjectProbe {
    pub(super) fn new() -> Self {
        Self {kind: KINDS.len()-1, mark: PROFILE.with(|p| p.borrow().is_some()).then(|| (Instant::now(), sc::frame_calls_on_thread()))}
    }
    pub(super) fn kind(&mut self, kind: &str) { self.kind = KINDS.iter().position(|v| *v == kind).unwrap_or(KINDS.len()-1); }
    pub(super) fn part(&mut self, part: usize) {
        if let Some((time, frames)) = self.mark {
            let now = Instant::now();
            let current = sc::frame_calls_on_thread();
            PROFILE.with(|p| {
                if let Some(p) = p.borrow_mut().as_mut() {
                    let count = &mut p.counts[self.kind][part];
                    count.calls += 1;
                    count.nanos += now.duration_since(time).as_nanos() as u64;
                    count.frames += current - frames;
                }
            });
            self.mark = Some((Instant::now(), sc::frame_calls_on_thread()));
        }
    }
}
pub(super) fn end() {
    PROFILE.with(|p| {
        if let Some(p) = p.borrow_mut().take() {
            for (kind, counts) in KINDS.iter().zip(p.counts) {
                for (part, count) in PARTS.iter().zip(counts) {
                    if count.calls > 0 {
                        eprintln!("W2-OBJECT-PROFILE {}", serde_json::json!({"generation":p.generation,"kind":kind,"part":part,"calls":count.calls,"nanos":count.nanos,"frames":count.frames}));
                    }
                }
            }
        }
    });
}
