//! #1481：已认证历史的只读来源材料，共用在线 LazySet 差分与来源贡献 Δ 步。
//! 不持 SQLite 写口、RawAdvanceBinding 或 SnapshotSeal，不可签发 owner 写资格。

use super::super::{
    audit_store::{CodecRead, CodecStore, RefValue},
    history::HashReader,
    source_audit::{NativeSourceState, SourceInput},
};
use super::{construction_set, native};
use newchan_rust::s_content::{self as sc, capture_allocation::scratch::WorkScope};

pub(crate) struct ColdSource<'reader, 'source> {
    reader: &'reader mut HashReader<'source>,
    budget: &'reader native::Budget,
    root: construction_set::Declaration,
    set: construction_set::LazySet,
}
impl<'reader, 'source> ColdSource<'reader, 'source> {
    pub(crate) fn new(
        reader: &'reader mut HashReader<'source>,
        budget: &'reader native::Budget,
        previous_root: Option<sc::RefView<'_>>,
        current: sc::RefView<'_>,
        previous: Option<&NativeSourceState>,
    ) -> Result<Self, String> {
        if reader.allocation_budget().is_some() || sc::NodeSource::codec_work(reader).is_none() {
            return Err(super::construction_state::stale(
                "冷来源要求真实 Native 工作账",
            ));
        }
        let root = construction_set::Declaration::read(current)?;
        let set = construction_set::LazySet::cold(reader, previous_root, current, previous)?;
        Ok(Self {
            reader,
            budget,
            root,
            set,
        })
    }
}
impl sc::NodeSource for ColdSource<'_, '_> {
    fn codec_work(&self) -> Option<&WorkScope> {
        sc::NodeSource::codec_work(self.reader)
    }
    fn audit_full_array_scan(&self) -> bool {
        sc::NodeSource::audit_full_array_scan(self.reader)
    }
    fn max_render_bytes(&self) -> Option<u64> {
        sc::NodeSource::max_render_bytes(self.reader)
    }
    fn read(&mut self, digest: &sc::NodeDigest) -> Result<sc::NodeRead, sc::CodecError> {
        sc::NodeSource::read(self.reader, digest)
    }
    fn read_scoped(
        &mut self,
        digest: &sc::NodeDigest,
    ) -> Result<sc::job::ScopedNodeRead, sc::CodecError> {
        sc::NodeSource::read_scoped(self.reader, digest)
    }
    fn checked_read(
        &mut self,
        digest: &sc::NodeDigest,
    ) -> Result<Option<sc::job::CheckedFrame>, sc::CodecError> {
        sc::NodeSource::checked_read(self.reader, digest)
    }
    fn checked_read_scoped(
        &mut self,
        digest: &sc::NodeDigest,
    ) -> Result<Option<sc::job::ScopedFrame>, sc::CodecError> {
        sc::NodeSource::checked_read_scoped(self.reader, digest)
    }
}
impl CodecRead for ColdSource<'_, '_> {
    fn native_measure(&self) -> Option<super::super::audit_budget::NativeMeasure<'_>> {
        self.reader.native_measure()
    }
    fn parse_rendered_plain(
        &self,
        bytes: &[u8],
    ) -> Result<super::super::business_json::PlainJson, String> {
        self.reader.parse_rendered_plain(bytes)
    }
    fn generic_source(&mut self) -> &mut dyn sc::NodeSource {
        self
    }
}
impl sc::NodeSink for ColdSource<'_, '_> {
    fn codec_sink_work(&self) -> Option<&WorkScope> {
        sc::NodeSource::codec_work(self)
    }
    fn put(&mut self, _: sc::NodeDigest, _: u8, _: &[u8]) -> Result<(), sc::CodecError> {
        Err(sc::CodecError::new(
            sc::code::SOURCE,
            "冷来源材料只读，禁止写入原件或派生覆盖",
        ))
    }
}
impl CodecStore for ColdSource<'_, '_> {
    fn generic_store(&mut self) -> &mut dyn sc::NodeStore {
        self
    }
}
impl SourceInput for ColdSource<'_, '_> {
    fn root(&self) -> sc::Built {
        self.root.built()
    }
    fn sequence_delta(
        &mut self,
        which: super::SourceArray,
    ) -> Result<&super::SequenceDelta, String> {
        self.set.delta(self.reader, self.budget, which)
    }
    fn request_delta(&mut self, ordinal: u64) -> Result<&super::SequenceDelta, String> {
        self.set.request_delta(self.reader, self.budget, ordinal)
    }
    fn array_member(
        &mut self,
        which: super::SourceArray,
        ordinal: u64,
    ) -> Result<RefValue, String> {
        self.set.member(self.reader, self.budget, which, ordinal)
    }
    fn request_member(&mut self, knowledge: u64, ordinal: u64) -> Result<RefValue, String> {
        self.set
            .request_member(self.reader, self.budget, knowledge, ordinal)
    }
}
