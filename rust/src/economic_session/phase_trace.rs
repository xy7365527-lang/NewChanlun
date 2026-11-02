//! #1716：同一经济写操作的阶段计时。测试构建按线程记录每段结束时刻相对操作起点的累计 wall time 与
//! 距操作期限的剩余预算，供真实 S 正控在断言前落盘；非测试构建全部为空操作。阶段名表示该段已结束，
//! 失败段即最后一个已记阶段之后的下一段。记录不读写期限，也不触碰共同 Job。
use std::time::Instant;

#[cfg(test)]
#[derive(Default)]
struct Trace {
    start: Option<Instant>,
    deadline: Option<Instant>,
    marks: Vec<(String, Instant)>,
}
#[cfg(test)]
thread_local! {
    static TRACE: std::cell::RefCell<Trace> = std::cell::RefCell::new(Trace::default());
}

pub(super) fn begin() {
    #[cfg(test)]
    TRACE.with(|t| {
        *t.borrow_mut() = Trace {
            start: Some(Instant::now()),
            ..Trace::default()
        }
    });
}
#[cfg_attr(not(test), allow(unused_variables))]
pub(super) fn deadline(at: Instant) {
    #[cfg(test)]
    TRACE.with(|t| t.borrow_mut().deadline = Some(at));
}
#[cfg_attr(not(test), allow(unused_variables))]
pub(super) fn mark(phase: &str, owner: &str) {
    #[cfg(test)]
    TRACE.with(|t| {
        let mut t = t.borrow_mut();
        if t.start.is_some() {
            t.marks.push((format!("{phase}@{owner}"), Instant::now()));
        }
    });
}
#[cfg_attr(not(test), allow(unused_variables))]
pub(super) fn mark_seq(phase: &str, owner: &str, seq: i64) {
    #[cfg(test)]
    TRACE.with(|t| {
        let mut t = t.borrow_mut();
        if t.start.is_some() {
            t.marks
                .push((format!("{phase}{seq}@{owner}"), Instant::now()));
        }
    });
}

/// 取走并清空本线程最近一次操作的记录；未开始过返回 Null。毫秒为三位小数文本，剩余预算可为负。
#[cfg(test)]
pub(super) fn take() -> serde_json::Value {
    let t = TRACE.with(|t| std::mem::take(&mut *t.borrow_mut()));
    let Some(start) = t.start else {
        return serde_json::Value::Null;
    };
    let ms = |from: Instant, to: Instant| -> String {
        if to >= from {
            format!("{:.3}", (to - from).as_secs_f64() * 1e3)
        } else {
            format!("-{:.3}", (from - to).as_secs_f64() * 1e3)
        }
    };
    let mut previous = start;
    let phases: Vec<serde_json::Value> = t
        .marks
        .iter()
        .map(|(phase, at)| {
            let row = serde_json::json!({"phase":phase,"segment_ms":ms(previous,*at),
                "cumulative_ms":ms(start,*at),"remaining_ms":t.deadline.map(|d| ms(*at,d))});
            previous = *at;
            row
        })
        .collect();
    serde_json::json!({"deadline_after_start_ms":t.deadline.map(|d| ms(start,d)),"phases":phases})
}
