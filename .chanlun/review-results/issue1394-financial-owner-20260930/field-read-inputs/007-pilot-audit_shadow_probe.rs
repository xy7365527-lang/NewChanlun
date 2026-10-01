//! #1394 cfg(test) shadow observation only; no resource/admission policy.
use std::{cell::RefCell, collections::BTreeSet, time::Instant};
use newchan_rust::s_content as sc;
use serde_json::{json, Value};

#[derive(Default)]
struct Counts {
    enabled: bool,
    parse_calls: u64,
    parse_ns: u128,
    parse_events: u64,
    stored_calls: u64,
    stored_ns: u128,
    source_reads: u64,
    source_body_bytes: u64,
    source_framed_bytes: u64,
    unique: BTreeSet<sc::NodeDigest>,
    f_base: u64,
    reuse_hits: u64,
    reuse_misses: u64,
    reuse_mismatches: u64,
    admission_reject_entries: u64,
    admission_reject_bytes: u64,
    admission_reject_capacity: u64,
    retained_entries_peak: usize,
    retained_capacity_peak: usize,
    reuse_marker_inner_ns: u128,
    reuse_marker_calls: u64,
    public_role_success_roots: u64,
    template_role_success_roots: u64,
}
thread_local! { static COUNTS: RefCell<Counts> = RefCell::new(Counts::default()); }
pub(crate) fn reset() {
    COUNTS.with(|c| *c.borrow_mut()=Counts {enabled:true, f_base:sc::frame_calls_on_thread(), ..Counts::default()});
}
pub(crate) fn source_read(d: &sc::NodeDigest, bytes: usize) {
    COUNTS.with(|c| { let mut c=c.borrow_mut(); if c.enabled {
        c.source_reads+=1; c.source_body_bytes+=bytes as u64;
        c.source_framed_bytes+=(sc::DOMAIN_SEPARATOR.len()+9+bytes) as u64; c.unique.insert(*d);
    }});
}
// Additional candidate-only observations. The measured duration excludes the TLS lookup,
// Instant construction and call overhead, so it is a lower bound, not correction to walltime.
fn mark(update: impl FnOnce(&mut Counts)) {
    let start=Instant::now();
    COUNTS.with(|cell| { let mut c=cell.borrow_mut(); if c.enabled {
        update(&mut c);
        c.reuse_marker_calls+=1;
        c.reuse_marker_inner_ns+=start.elapsed().as_nanos();
    }});
}
pub(crate) fn reuse_hit() { mark(|c| c.reuse_hits+=1); }
pub(crate) fn reuse_miss() { mark(|c| c.reuse_misses+=1); }
pub(crate) fn reuse_mismatch() { mark(|c| c.reuse_mismatches+=1); }
pub(crate) fn admission_reject_entries() { mark(|c| c.admission_reject_entries+=1); }
pub(crate) fn admission_reject_bytes() { mark(|c| c.admission_reject_bytes+=1); }
pub(crate) fn admission_reject_capacity() { mark(|c| c.admission_reject_capacity+=1); }
pub(crate) fn retention(entries:usize, capacity:usize) { mark(|c| {
    c.retained_entries_peak=c.retained_entries_peak.max(entries);
    c.retained_capacity_peak=c.retained_capacity_peak.max(capacity);
}); }
pub(crate) fn public_role_success(roots:usize) {
    COUNTS.with(|cell| { let mut c=cell.borrow_mut(); if c.enabled { c.public_role_success_roots+=roots as u64; }});
}
pub(crate) fn template_role_success(roots:usize) {
    COUNTS.with(|cell| { let mut c=cell.borrow_mut(); if c.enabled { c.template_role_success_roots+=roots as u64; }});
}
pub(crate) fn snapshot() -> Value {
    COUNTS.with(|c| { let c=c.borrow(); json!({"parse_calls":c.parse_calls,"parse_ns":c.parse_ns.to_string(),
        "parse_event_checks":c.parse_events,"stored_clock_calls":c.stored_calls,"stored_clock_ns":c.stored_ns.to_string(),
        "source_read_calls":c.source_reads,"source_body_bytes":c.source_body_bytes,"source_framed_bytes":c.source_framed_bytes,
        "unique_source_digests":c.unique.len(),"s_content_f_calls":sc::frame_calls_on_thread()-c.f_base,
        "reuse_hits":c.reuse_hits,"reuse_misses":c.reuse_misses,"reuse_mismatches":c.reuse_mismatches,
        "admission_reject_entries":c.admission_reject_entries,"admission_reject_bytes":c.admission_reject_bytes,
        "admission_reject_capacity":c.admission_reject_capacity,
        "retained_entries_peak":c.retained_entries_peak,"retained_capacity_peak":c.retained_capacity_peak,
        "reuse_marker_calls":c.reuse_marker_calls,"reuse_marker_inner_ns_lower_bound":c.reuse_marker_inner_ns.to_string(),
        "public_role_success_roots":c.public_role_success_roots,
        "template_role_success_roots":c.template_role_success_roots}) })
}
pub(crate) struct Span { start:Instant, parse:bool, events:usize }
impl Span {
    pub(crate) fn parse(events:usize)->Self { Self{start:Instant::now(),parse:true,events} }
    pub(crate) fn stored()->Self { Self{start:Instant::now(),parse:false,events:0} }
}
impl Drop for Span {
    fn drop(&mut self) { let ns=self.start.elapsed().as_nanos(); COUNTS.with(|c| {
        let mut c=c.borrow_mut(); if c.enabled { if self.parse {
            c.parse_calls+=1; c.parse_ns+=ns; c.parse_events+=self.events as u64;
        } else { c.stored_calls+=1; c.stored_ns+=ns; } }
    }); }
}
