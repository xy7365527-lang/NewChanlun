//! #1505：固定容量、字节级 overlay。无 SQLite 格式解析、无源写、无回调内堆增长。
#![deny(unsafe_op_in_unsafe_fn)]
use serde::Serialize;
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;
use std::ptr::NonNull;

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
static OWNED_CURRENT: AtomicU64 = AtomicU64::new(0);
static OWNED_PEAK: AtomicU64 = AtomicU64::new(0);
static OWNED_REQUESTS: AtomicU64 = AtomicU64::new(0);
static OWNED_ALLOCS: AtomicU64 = AtomicU64::new(0);
static OWNED_FREES: AtomicU64 = AtomicU64::new(0);
static OWNED_FAILURES: AtomicU64 = AtomicU64::new(0);
static FAIL_NEXT: AtomicBool = AtomicBool::new(false);
pub fn fail_next_allocation() {
    FAIL_NEXT.store(true, Ordering::SeqCst);
}
fn allocate(layout: Layout, zeroed: bool) -> io::Result<NonNull<u8>> {
    OWNED_REQUESTS.fetch_add(layout.size() as u64, Ordering::SeqCst);
    let ptr = if FAIL_NEXT.swap(false, Ordering::SeqCst) {
        std::ptr::null_mut()
    } else {
        unsafe {
            if zeroed {
                alloc_zeroed(layout)
            } else {
                std::alloc::alloc(layout)
            }
        }
    };
    let Some(ptr) = NonNull::new(ptr) else {
        OWNED_FAILURES.fetch_add(1, Ordering::SeqCst);
        return Err(io::Error::from(io::ErrorKind::OutOfMemory));
    };
    OWNED_ALLOCS.fetch_add(1, Ordering::SeqCst);
    let now =
        OWNED_CURRENT.fetch_add(layout.size() as u64, Ordering::SeqCst) + layout.size() as u64;
    OWNED_PEAK.fetch_max(now, Ordering::SeqCst);
    Ok(ptr)
}
unsafe fn release(ptr: NonNull<u8>, layout: Layout) {
    unsafe { dealloc(ptr.as_ptr(), layout) };
    OWNED_CURRENT.fetch_sub(layout.size() as u64, Ordering::SeqCst);
    OWNED_FREES.fetch_add(1, Ordering::SeqCst);
}
pub fn allocation_snapshot() -> serde_json::Value {
    serde_json::json!({"request_layout_bytes":OWNED_REQUESTS.load(Ordering::SeqCst),
    "actual_owned_layout_current_bytes":OWNED_CURRENT.load(Ordering::SeqCst),"actual_owned_layout_peak_bytes":OWNED_PEAK.load(Ordering::SeqCst),
    "allocation_calls":OWNED_ALLOCS.load(Ordering::SeqCst),"free_calls":OWNED_FREES.load(Ordering::SeqCst),"failures":OWNED_FAILURES.load(Ordering::SeqCst),
    "allocator_private_overhead":"NOT_MEASURED","scope":"fixed VFS payload/metadata/trace/owner allocations, not driver result buffers or process RSS"})
}

pub const CHUNK: usize = 4096;
pub const MAIN: i32 = 1;
pub const JOURNAL: i32 = 2;

/// 只把已初始化的前 len 项作为 T 暴露。零填 backing 不构造未初始化 T（如 Event 的引用）。
pub struct Fixed<T: Copy> {
    ptr: NonNull<T>,
    layout: Layout,
    len: usize,
    capacity: usize,
}
impl<T: Copy> Fixed<T> {
    pub fn new(bytes: usize) -> io::Result<Self> {
        if std::mem::size_of::<T>() == 0 || bytes < std::mem::size_of::<T>() {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }
        let layout = Layout::from_size_align(bytes, std::mem::align_of::<T>())
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        // SAFETY: 有效且非零 Layout；失败返回 NULL，绝不调用 handle_alloc_error/abort。
        let ptr = allocate(layout, true)?.cast::<T>();
        Ok(Self {
            ptr,
            layout,
            len: 0,
            capacity: bytes / std::mem::size_of::<T>(),
        })
    }
    pub fn push(&mut self, value: T) -> bool {
        if self.len == self.capacity {
            return false;
        }
        // SAFETY: len < capacity；此槽尚未作为 T 暴露，T: Copy 无 drop 所有权。
        unsafe {
            self.ptr.as_ptr().add(self.len).write(value);
        }
        self.len += 1;
        true
    }
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn bytes(&self) -> usize {
        self.layout.size()
    }
    pub fn accounting(&self) -> serde_json::Value {
        serde_json::json!({"request_bytes":self.layout.size(),"layout_bytes":self.layout.size(),
            "alignment":self.layout.align(),"capacity_elements":self.capacity,
            "capacity_bytes":self.capacity*std::mem::size_of::<T>(),"held_allocation_bytes":self.layout.size(),
            "initialized_elements":self.len,"allocator_private_overhead":"NOT_MEASURED"})
    }
}
impl<T: Copy> Drop for Fixed<T> {
    fn drop(&mut self) {
        unsafe {
            release(self.ptr.cast(), self.layout);
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
struct Slot {
    kind: i32,
    index: u64,
    state: u8,
}
#[derive(Clone, Copy, Debug, Serialize)]
pub struct FileState {
    pub logical_len: u64,
    pub visible_base_end: u64,
    pub exists: bool,
}
#[derive(Debug)]
pub enum Error {
    Invalid,
    Capacity,
    Io(io::Error),
    Sticky,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl From<io::Error> for Error {
    fn from(x: io::Error) -> Self {
        Self::Io(x)
    }
}

pub struct Overlay {
    payload: Fixed<u8>,
    slots: Fixed<Slot>,
    files: [FileState; 2],
    pub sticky: bool,
    pub write_calls: u64,
    pub truncate_calls: u64,
    pub delete_calls: u64,
    pub requested_write_bytes: u64,
    pub slot_peak: usize,
    pub failures: u64,
}
impl Overlay {
    pub fn new(
        payload: usize,
        metadata: usize,
        main: u64,
        journal: Option<u64>,
    ) -> Result<Self, Error> {
        let count = payload / CHUNK;
        if count == 0
            || metadata / std::mem::size_of::<Slot>() < count
            || main > i64::MAX as u64
            || journal.is_some_and(|n| n > i64::MAX as u64)
        {
            return Err(Error::Invalid);
        }
        let mut bytes = Fixed::<u8>::new(payload)?;
        // allocate(zeroed=true) 已初始化每个u8；无逐字节push，也不把未初始化泛型T暴露。
        bytes.len = bytes.capacity;
        let mut slots = Fixed::new(metadata)?;
        for _ in 0..count {
            if !slots.push(Slot {
                kind: 0,
                index: 0,
                state: 0,
            }) {
                return Err(Error::Invalid);
            }
        }
        Ok(Self {
            payload: bytes,
            slots,
            files: [
                FileState {
                    logical_len: main,
                    visible_base_end: main,
                    exists: true,
                },
                FileState {
                    logical_len: journal.unwrap_or(0),
                    visible_base_end: journal.unwrap_or(0),
                    exists: journal.is_some(),
                },
            ],
            sticky: false,
            write_calls: 0,
            truncate_calls: 0,
            delete_calls: 0,
            requested_write_bytes: 0,
            slot_peak: 0,
            failures: 0,
        })
    }
    fn ordinal(kind: i32) -> Result<usize, Error> {
        match kind {
            MAIN => Ok(0),
            JOURNAL => Ok(1),
            _ => Err(Error::Invalid),
        }
    }
    pub fn state(&self, kind: i32) -> Result<FileState, Error> {
        Ok(self.files[Self::ordinal(kind)?])
    }
    fn slot(&self, kind: i32, index: u64) -> Option<usize> {
        self.slots
            .as_slice()
            .iter()
            .position(|s| s.state != 0 && s.kind == kind && s.index == index)
    }
    fn fail<T>(&mut self, e: Error) -> Result<T, Error> {
        self.sticky = true;
        self.failures += 1;
        Err(e)
    }
    pub fn read(&self, kind: i32, base: &File, off: u64, out: &mut [u8]) -> Result<bool, Error> {
        let st = self.state(kind)?;
        if self.sticky {
            return Err(Error::Sticky);
        }
        let end = off.checked_add(out.len() as u64).ok_or(Error::Invalid)?;
        if end > i64::MAX as u64 {
            return Err(Error::Invalid);
        }
        out.fill(0);
        let valid = end.min(st.logical_len);
        let mut at = off;
        while at < valid {
            let index = at / CHUNK as u64;
            let inner = (at % CHUNK as u64) as usize;
            let n = ((valid - at) as usize).min(CHUNK - inner);
            let dst = (at - off) as usize;
            if let Some(slot) = self.slot(kind, index) {
                out[dst..dst + n].copy_from_slice(
                    &self.payload.as_slice()[slot * CHUNK + inner..slot * CHUNK + inner + n],
                );
            } else if at < st.visible_base_end {
                let nbase = n.min((st.visible_base_end - at) as usize);
                read_exact(base, &mut out[dst..dst + nbase], at)?;
            }
            at += n as u64;
        }
        Ok(end > st.logical_len)
    }
    pub fn write(&mut self, kind: i32, base: &File, off: u64, input: &[u8]) -> Result<(), Error> {
        if self.sticky {
            return Err(Error::Sticky);
        }
        let ordinal = Self::ordinal(kind)?;
        if input.is_empty() {
            return Ok(());
        }
        let Some(end) = off
            .checked_add(input.len() as u64)
            .filter(|n| *n <= i64::MAX as u64)
        else {
            return self.fail(Error::Invalid);
        };
        let first = off / CHUNK as u64;
        let last = (end - 1) / CHUNK as u64;
        if last - first + 1 > self.slots.len() as u64 {
            return self.fail(Error::Capacity);
        }
        let missing = (first..=last)
            .filter(|&i| self.slot(kind, i).is_none())
            .count();
        let free = self
            .slots
            .as_slice()
            .iter()
            .filter(|s| s.state == 0)
            .count();
        if missing > free {
            return self.fail(Error::Capacity);
        }
        // 先准备全部新 chunk；任何预读失败只撤回 state=2，不修改已发布的槽或长度。
        let visible = self.files[ordinal].visible_base_end;
        for index in first..=last {
            if self.slot(kind, index).is_some() {
                continue;
            }
            let slot = self
                .slots
                .as_slice()
                .iter()
                .position(|s| s.state == 0)
                .ok_or(Error::Capacity)?;
            self.slots.as_mut_slice()[slot] = Slot {
                kind,
                index,
                state: 2,
            };
            let buf = &mut self.payload.as_mut_slice()[slot * CHUNK..(slot + 1) * CHUNK];
            buf.fill(0);
            let start = index * CHUNK as u64;
            if start < visible {
                let n = (visible - start).min(CHUNK as u64) as usize;
                if let Err(e) = read_exact(base, &mut buf[..n], start) {
                    for s in self.slots.as_mut_slice() {
                        if s.state == 2 {
                            s.state = 0;
                        }
                    }
                    return self.fail(Error::Io(e));
                }
            }
        }
        for s in self.slots.as_mut_slice() {
            if s.state == 2 {
                s.state = 1;
            }
        }
        let mut done = 0;
        while done < input.len() {
            let at = off + done as u64;
            let inner = (at % CHUNK as u64) as usize;
            let n = (input.len() - done).min(CHUNK - inner);
            let slot = self.slot(kind, at / CHUNK as u64).ok_or(Error::Invalid)?;
            self.payload.as_mut_slice()[slot * CHUNK + inner..slot * CHUNK + inner + n]
                .copy_from_slice(&input[done..done + n]);
            done += n;
        }
        self.files[ordinal].logical_len = self.files[ordinal].logical_len.max(end);
        self.write_calls += 1;
        self.requested_write_bytes += input.len() as u64;
        self.slot_peak = self.slot_peak.max(
            self.slots
                .as_slice()
                .iter()
                .filter(|s| s.state != 0)
                .count(),
        );
        Ok(())
    }
    pub fn truncate(&mut self, kind: i32, n: u64) -> Result<(), Error> {
        if self.sticky {
            return Err(Error::Sticky);
        }
        if n > i64::MAX as u64 {
            return self.fail(Error::Invalid);
        }
        let k = Self::ordinal(kind)?;
        if n < self.files[k].logical_len {
            self.files[k].visible_base_end = self.files[k].visible_base_end.min(n);
            for (slot, s) in self.slots.as_mut_slice().iter_mut().enumerate() {
                if s.state == 0 || s.kind != kind {
                    continue;
                }
                let start = s.index * CHUNK as u64;
                if start >= n {
                    s.state = 0;
                } else if n - start < CHUNK as u64 {
                    self.payload.as_mut_slice()
                        [slot * CHUNK + (n - start) as usize..(slot + 1) * CHUNK]
                        .fill(0);
                }
            }
        }
        self.files[k].logical_len = n;
        self.truncate_calls += 1;
        Ok(())
    }
    pub fn delete_journal(&mut self) -> Result<(), Error> {
        if self.sticky {
            return Err(Error::Sticky);
        }
        self.files[1].exists = false;
        self.delete_calls += 1;
        Ok(())
    }
    pub fn view_fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        for f in self.files {
            h.update(f.logical_len.to_le_bytes());
            h.update(f.visible_base_end.to_le_bytes());
            h.update([u8::from(f.exists)]);
        }
        for (i, s) in self.slots.as_slice().iter().enumerate() {
            if s.state == 1 {
                h.update(s.kind.to_le_bytes());
                h.update(s.index.to_le_bytes());
                h.update(&self.payload.as_slice()[i * CHUNK..(i + 1) * CHUNK]);
            }
        }
        format!("{:x}", h.finalize())
    }
    pub fn report(&self) -> serde_json::Value {
        serde_json::json!({"files":self.files,"payload":self.payload.accounting(),"metadata":self.slots.accounting(),
            "chunk_bytes":CHUNK,"slot_count":self.slots.len(),"slot_peak":self.slot_peak,
            "write_calls":self.write_calls,"truncate_calls":self.truncate_calls,"delete_calls":self.delete_calls,
            "requested_write_bytes":self.requested_write_bytes,"failures":self.failures,"sticky":self.sticky,
            "free_slots_do_not_release_capacity":true})
    }
}
fn read_exact(file: &File, mut out: &mut [u8], mut off: u64) -> io::Result<()> {
    let mut eintr = 0;
    while !out.is_empty() {
        match file.read_at(out, off) {
            Ok(0) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
            Ok(n) => {
                off += n as u64;
                out = &mut out[n..];
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted && eintr < 3 => eintr += 1,
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// 与 Box 相同的唯一所有权；分配失败可返回，不进入 Rust 全局 OOM abort。
pub struct FallibleBox<T> {
    ptr: NonNull<T>,
}
impl<T> FallibleBox<T> {
    pub fn new(value: T) -> io::Result<Self> {
        let layout = Layout::new::<T>();
        if layout.size() == 0 {
            return Err(io::Error::from(io::ErrorKind::InvalidInput));
        }
        let ptr = allocate(layout, false)?.cast::<T>();
        unsafe {
            ptr.as_ptr().write(value);
        }
        Ok(Self { ptr })
    }
}
impl<T> std::ops::Deref for FallibleBox<T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { self.ptr.as_ref() }
    }
}
impl<T> std::ops::DerefMut for FallibleBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { self.ptr.as_mut() }
    }
}
impl<T> Drop for FallibleBox<T> {
    fn drop(&mut self) {
        unsafe {
            self.ptr.as_ptr().drop_in_place();
            release(self.ptr.cast(), Layout::new::<T>());
        }
    }
}
