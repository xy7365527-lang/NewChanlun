//! #1419/#1393 W 移植非作者独审：Lean `RawInclusionInitialization` 逐字转写的独立 oracle。
//! 只读 IncrFactsState 公开/父模块状态，不复用 first_raw_direction / fold_step。
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum D { Up, Down }

fn lcontains(a: (i64, i64), b: (i64, i64)) -> bool {
    (a.0 <= b.0 && b.1 <= a.1) || (b.0 <= a.0 && a.1 <= b.1)
}
fn lstrict(a: (i64, i64), b: (i64, i64)) -> Option<D> {
    if a.1 < b.1 && a.0 < b.0 { Some(D::Up) } else if b.1 < a.1 && b.0 < a.0 { Some(D::Down) } else { None }
}
fn lmerge(d: D, a: (i64, i64), b: (i64, i64)) -> (i64, i64) {
    match d { D::Up => (a.0.max(b.0), a.1.max(b.1)), D::Down => (a.0.min(b.0), a.1.min(b.1)) }
}
/// Lean firstPair：返回 (offset, d)。
fn lfirst(raw: &[(i64, i64)]) -> Option<(usize, D)> {
    for i in 0..raw.len().saturating_sub(1) {
        if lcontains(raw[i], raw[i + 1]) { continue; }
        if let Some(d) = lstrict(raw[i], raw[i + 1]) { return Some((i, d)); }
    }
    None
}
/// Lean foldFrom：组几何 + 成员下标。
fn lfold(d0: D, raw: &[(i64, i64)]) -> Vec<((i64, i64), Vec<usize>)> {
    let mut out = vec![];
    let mut acc = raw[0];
    let mut mem = vec![0usize];
    let mut d = d0;
    for (k, &b) in raw.iter().enumerate().skip(1) {
        if lcontains(acc, b) { acc = lmerge(d, acc, b); mem.push(k); }
        else { d = lstrict(acc, b).unwrap_or(d); out.push((acc, std::mem::take(&mut mem))); acc = b; mem = vec![k]; }
    }
    out.push((acc, mem));
    out
}

fn mk(coord: usize, low: i64, high: i64) -> Bar {
    Bar { source_index: coord, timestamp: coord as i64, open: low, high, low, close: high, volume: 1.0, untradable: false }
}

/// W 闭式：建立在 raw 下标 e（证据对右端）；步 k 的来源 = raw[..=max(k,e)] 坐标。
fn check_against_oracle(bars: &[Bar], facts: &InclusionFacts, label: &str) {
    let raw: Vec<(i64, i64)> = bars.iter().map(|b| (b.low, b.high)).collect();
    let coords: Vec<usize> = bars.iter().map(|b| b.source_index).collect();
    match lfirst(&raw) {
        None => {
            assert!(facts.initial_direction_unsettled, "{label}: 应未定");
            assert!(facts.groups.is_empty() && facts.merged.is_empty(), "{label}: 未定不得有正式组");
            assert_eq!(facts.pending_raw, bars, "{label}: 未定原件");
            assert_eq!(facts.unmapped_sources, coords, "{label}");
            assert!(facts.initial_direction_evidence.is_none(), "{label}");
            assert!(facts.steps.iter().all(|s| s.action == "waiting" && s.source_coords == vec![s.incoming.source_index]), "{label}");
            assert_eq!(facts.steps.len(), bars.len(), "{label}");
            let p = process_inclusion(bars);
            assert!(p.only_open_tail && p.merged.is_empty() && p.pending_raw == bars && p.final_direction.is_none(), "{label}");
        }
        Some((off, d)) => {
            let e = off + 1;
            assert!(!facts.initial_direction_unsettled, "{label}");
            assert!(facts.pending_raw.is_empty() && facts.unmapped_sources.is_empty(), "{label}");
            let ev = facts.initial_direction_evidence.as_ref().expect("初始化证据");
            assert_eq!((ev.previous_acc.source_index, ev.incoming.source_index), (coords[off], coords[e]), "{label}: 首对");
            assert_eq!(ev.direction == Direction::Up, d == D::Up, "{label}: 首对方向");
            assert_eq!(ev.source_coords, coords[..=e].to_vec(), "{label}: 初始化来源=建立前缀");
            let g = lfold(d, &raw);
            assert_eq!(facts.groups.len(), g.len(), "{label}: 组数");
            assert_eq!(facts.merged.len(), g.len(), "{label}");
            for (i, (geo, mem)) in g.iter().enumerate() {
                let fg = &facts.groups[i];
                assert_eq!((fg.bar.low, fg.bar.high), *geo, "{label}: 组{i}几何");
                assert_eq!((facts.merged[i].low, facts.merged[i].high), *geo, "{label}");
                assert_eq!(fg.members, mem.iter().map(|k| coords[*k]).collect::<Vec<_>>(), "{label}: 组{i}成员");
                assert_eq!(fg.bar.source_index, coords[mem[0]], "{label}: 组锚=首根");
                assert_eq!(fg.confirmed, i + 1 < g.len(), "{label}: 封口");
                let dir_ev = fg.direction_evidence.as_ref().expect("组方向证据");
                let created = mem[0];
                let want: Vec<usize> = coords[..=created.max(e)].to_vec();
                if i == 0 { assert_eq!(dir_ev, ev, "{label}: seed 组证据=初始化证据"); }
                else { assert_eq!(dir_ev.source_coords, want, "{label}: 组{i}方向证据来源"); }
            }
            assert_eq!(facts.steps.len(), bars.len(), "{label}: 步数=raw数");
            assert_eq!(facts.steps[0].action, "seed", "{label}");
            for (k, s) in facts.steps.iter().enumerate() {
                assert_eq!(s.incoming.source_index, coords[k], "{label}: 步序");
                assert_ne!(s.action, "waiting", "{label}: 建立后无 waiting");
                assert_eq!(s.source_coords, coords[..=k.max(e)].to_vec(), "{label}: 步{k}来源");
            }
            let p = process_inclusion(bars);
            assert_eq!(p.merged, facts.merged, "{label}: 两入口 merged");
            assert!(!p.only_open_tail && p.pending_raw.is_empty(), "{label}");
        }
    }
    // IncrInclusion 逐根与全量一致
    let mut inc = IncrInclusion::empty();
    for b in bars { inc = inc.append(*b); }
    assert_eq!(inc.to_result(), process_inclusion(bars), "{label}: IncrInclusion");
}

struct Lcg(u64);
impl Lcg { fn next(&mut self, m: u64) -> u64 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); (self.0 >> 33) % m } }

fn random_bars(rng: &mut Lcg, start: usize, n: usize, span: u64) -> Vec<Bar> {
    let mut c = start;
    (0..n).map(|_| { c += 1 + rng.next(3) as usize; let a = rng.next(span) as i64; let b = rng.next(span) as i64; mk(c, a.min(b), a.max(b)) }).collect()
}

fn fresh(bars: &[Bar]) -> IncrFactsState {
    let mut s = IncrFactsState::empty();
    for b in bars { s.append(*b); }
    s
}

#[test]
fn wreview_hand_counterexamples() {
    // CE1 U04：相邻包含非链式，首尾不得偷方向；再加严格对后从 raw0 UP 重折。
    let ce1 = [mk(0, 0, 10), mk(1, 2, 8), mk(2, 1, 11), mk(3, 3, 12)];
    // CE2 U10 变体：建立在中段 DOWN，单组且来源=整前缀；之后再追加同向/反向。
    let ce2 = [mk(0, 0, 10), mk(1, 2, 8), mk(2, 1, 7), mk(3, 2, 6), mk(4, 3, 9), mk(5, 0, 2)];
    // CE3 建立对恰在首两根，其后全包含，再反向。
    let ce3 = [mk(0, 0, 5), mk(1, 1, 6), mk(2, 2, 5), mk(3, 1, 6), mk(4, -1, 3)];
    // CE4 建立前全包含（等宽互含），永不建立。
    let ce4 = [mk(0, 0, 5), mk(1, 0, 5), mk(2, 1, 4), mk(3, 0, 6)];
    // CE5 建立前包含 + 同价极值竞争（RD-02 根集合）跨重折。
    let ce5 = [mk(0, 2, 9), mk(1, 3, 9), mk(2, 4, 10), mk(3, 4, 10), mk(4, 1, 5)];
    for (name, bars) in [("CE1", &ce1[..]), ("CE2", &ce2[..]), ("CE3", &ce3[..]), ("CE4", &ce4[..]), ("CE5", &ce5[..])] {
        for k in 0..=bars.len() {
            let label = format!("{name}[..{k}]");
            let s = fresh(&bars[..k]);
            let f = s.materialize_facts();
            assert_eq!(f, process_inclusion_with_facts(&bars[..k]), "{label}: 共享=展开");
            check_against_oracle(&bars[..k], &f, &label);
        }
    }
}

#[test]
fn wreview_random_prefix_rewind_and_witness() {
    let mut rng = Lcg(0x1393_1419);
    let mut cases = 0usize;
    for _ in 0..1500 {
        let n = 1 + rng.next(9) as usize;
        let span = 3 + rng.next(6);
        let bars = random_bars(&mut rng, 0, n, span);
        // 逐前缀：变化见证 + oracle + 发证规则
        let mut s = IncrFactsState::empty();
        for k in 0..n {
            let before = s.materialize_facts();
            let ch = s.append_with_changes(bars[k]);
            let after = s.materialize_facts();
            let label = format!("seq{cases} prefix{}", k + 1);
            assert_eq!(after, process_inclusion_with_facts(&bars[..=k]), "{label}");
            check_against_oracle(&bars[..=k], &after, &label);
            let established_now = before.initial_direction_unsettled && !after.initial_direction_unsettled;
            assert_eq!(ch.replayed_from_zero(), established_now, "{label}: 重折见证");
            assert_eq!(s.replayed_from_zero(), established_now, "{label}");
            let sd = ch.step_dirty_from();
            assert_eq!(sd, if established_now { 0 } else { k }, "{label}");
            assert_eq!(&before.steps[..sd], &after.steps[..sd], "{label}: 步前缀不变");
            let gd = ch.group_dirty_from().unwrap_or(before.groups.len());
            assert_eq!(&before.groups[..gd], &after.groups[..gd], "{label}: 组前缀不变");
            let md = ch.merged_dirty_from().unwrap_or(before.merged.len());
            assert_eq!(&before.merged[..md], &after.merged[..md], "{label}: merged 前缀不变");
            let want_conf = if after.initial_direction_unsettled || established_now { 0 } else { after.merged.len() - 1 };
            assert_eq!(s.merged_confirmed_len(), want_conf, "{label}: 发证");
            // 已发证前缀此后永不改写（下一根检查）
        }
        // 回退到每个 rank（含跨建立点），再接异样后缀，对新鲜重放。
        for cut in 0..=n {
            let mut r = fresh(&bars);
            r.rewind_to(cut).unwrap();
            let label = format!("seq{cases} rewind{cut}");
            let f0 = fresh(&bars[..cut]);
            assert_eq!(r.materialize_facts(), f0.materialize_facts(), "{label}");
            assert_eq!(r.merged_confirmed_len(), f0.merged_confirmed_len(), "{label}: 回退后发证");
            assert_eq!(r.replayed_from_zero(), f0.replayed_from_zero(), "{label}");
            assert_eq!(r.unsettled(), f0.unsettled(), "{label}");
            let start = if cut == 0 { 0 } else { bars[cut - 1].source_index };
            let tl = 1 + rng.next(5) as usize;
            let tail = random_bars(&mut rng, start, tl, span);
            let mut combined = bars[..cut].to_vec();
            for t in &tail {
                r.append(*t);
                combined.push(*t);
                let f = r.materialize_facts();
                assert_eq!(f, process_inclusion_with_facts(&combined), "{label}+{}", combined.len());
                check_against_oracle(&combined, &f, &format!("{label}+{}", combined.len()));
                let fr = fresh(&combined);
                assert_eq!(r.merged_confirmed_len(), fr.merged_confirmed_len(), "{label}: 发证一致");
            }
        }
        cases += 1;
    }
}

#[test]
fn wreview_confirmed_prefix_never_rewritten() {
    let mut rng = Lcg(0xfeed_0019);
    for _ in 0..800 {
        let n = 2 + rng.next(10) as usize;
        let sp = 3 + rng.next(5);
        let bars = random_bars(&mut rng, 0, n, sp);
        let mut s = IncrFactsState::empty();
        let mut issued: Vec<Bar> = vec![];
        for b in &bars {
            s.append(*b);
            let m = s.merged_slice();
            assert!(m.len() >= issued.len() || s.unsettled(), "已发证后组数不减");
            assert_eq!(&m[..issued.len()], &issued[..], "已发证前缀被改写");
            issued = m[..s.merged_confirmed_len()].to_vec();
        }
    }
}
