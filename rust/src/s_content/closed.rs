//! #1394 Root 2026-10-02 内部实施边界：具体同源owner，非新增外部协议/额度合同。
//! 不实现NodeSource，不暴露body Vec；普通路径与本owner复用同一codec判断。
use super::capture_allocation::{overlay::OverlaySlab, OwnedBudget};
use super::*;

/// lib→bin 窄实现接口。字段/构造私有，不能拼接dyn source或另一份budget。
pub struct ClosedCapturedOwner<'capture> {
    capture: &'capture AuditCapture,
    budget: OwnedBudget,
    overlay: OverlaySlab,
    max_render_bytes: Option<u64>,
    full_arrays: bool,
    read_trace: Option<fn(bool)>,
    put_trace: Option<fn()>,
    overlay_peak_trace: Option<fn((usize, usize))>,
}
impl AuditCapture {
    #[doc(hidden)]
    pub fn is_allocation_prototype(&self) -> bool {
        matches!(self.nodes, CaptureNodes::AllocationPrototype(_))
    }
    #[doc(hidden)]
    pub fn closed_codec_for_allocation_prototype(
        &self,
        slots: usize,
        bytes: usize,
        max_render_bytes: Option<u64>,
    ) -> Result<ClosedCapturedOwner<'_>, CodecError> {
        let CaptureNodes::AllocationPrototype(slab) = &self.nodes else {
            return Err(CodecError::new(
                capture_allocation::INCOMPLETE,
                "closed codec必须来自AllocationPrototype capture",
            ));
        };
        let budget = slab.owned_budget();
        let overlay = OverlaySlab::new(slots, bytes, &budget)?;
        Ok(ClosedCapturedOwner {
            capture: self,
            budget,
            overlay,
            max_render_bytes,
            full_arrays: false,
            read_trace: None,
            put_trace: None,
            overlay_peak_trace: None,
        })
    }
}
impl ClosedCapturedOwner<'_> {
    pub fn content_ref_of(&mut self, root: &str) -> Result<ContentRef, CodecError> {
        content_ref_ctx(&mut CodecContext::Closed(self), root)
    }
    pub fn lookup_path(
        &mut self,
        r: &ContentRef,
        p: &[path::PathStep],
    ) -> Result<ContentRef, CodecError> {
        path::lookup_path_ctx(&mut CodecContext::Closed(self), r, p)
    }
    pub fn content_ref_owned(&mut self, root: &str) -> Result<RefOwner, CodecError> {
        let digest = parse_root(root)?;
        let (kind, extent) = path::reference_header(&mut CodecContext::Closed(self), &digest)?;
        Ok(RefOwner::new(kind, digest, extent))
    }
    pub fn lookup_path_owned(
        &mut self,
        r: RefView<'_>,
        p: &[path::PathStep],
    ) -> Result<RefOwner, CodecError> {
        path::lookup_path_owned_ctx(self, r, p)
    }
    pub fn lookup_array_items_owned(
        &mut self,
        r: RefView<'_>,
    ) -> Result<CodecVec<RefOwner>, CodecError> {
        path::lookup_array_items_owned_ctx(self, r)
    }
    pub fn lookup_object_fields_owned<'names>(
        &mut self,
        r: RefView<'_>,
        n: &[&'names str],
    ) -> Result<OwnedRefFields<'names>, CodecError> {
        path::lookup_object_fields_owned_ctx(self, r, n, true)
    }
    pub fn lookup_object_fields_subset_owned<'names>(
        &mut self,
        r: RefView<'_>,
        n: &[&'names str],
    ) -> Result<OwnedRefFields<'names>, CodecError> {
        path::lookup_object_fields_owned_ctx(self, r, n, false)
    }
    pub fn allocation_budget(&self) -> OwnedBudget {
        self.budget.clone()
    }
    pub fn write_value_view(&mut self, v: ValueView<'_>) -> Result<Built, CodecError> {
        write_value_view_ctx(&mut CodecContext::Closed(self), v, Mode::Public)
    }
    pub fn write_template_view(&mut self, v: ValueView<'_>) -> Result<Built, CodecError> {
        write_value_view_ctx(&mut CodecContext::Closed(self), v, Mode::Template)
    }
    pub fn replace_batch_placeholder_owned(
        &mut self,
        r: RefView<'_>,
        path: &[path::PathStep],
        batch: &str,
    ) -> Result<RefOwner, CodecError> {
        path::replace_batch_placeholder_owned_ctx(self, r, path, batch)
    }
    pub fn new_owned_ref_result(&self) -> CodecVec<RefOwner> {
        WorkVec::new(Some(&self.budget))
    }
    pub fn render_canonical_owned(
        &mut self,
        r: RefView<'_>,
        out: &mut dyn Write,
    ) -> Result<(), CodecError> {
        render_refview_ctx(CodecContext::Closed(self), r, out)
    }
    pub fn verify_fresh_owned(&mut self, r: RefView<'_>, mode: Mode) -> Result<(), CodecError> {
        Verifier::with_context(CodecContext::Closed(self), mode, AuditMemo::default())
            .verify_refview(r)
            .map(|_| ())
    }
    pub fn verify_cached_owned(&mut self, r: RefView<'_>, mode: Mode) -> Result<(), CodecError> {
        if self.capture.contains(&r.digest()?) {
            self.capture.verify_refview(r, mode)
        } else {
            self.verify_fresh_owned(r, mode)
        }
    }
    pub fn verify_many_fresh_owned<'a>(
        &mut self,
        refs: impl IntoIterator<Item = RefView<'a>>,
        mode: Mode,
    ) -> Result<(), CodecError> {
        let mut verifier =
            Verifier::with_context(CodecContext::Closed(self), mode, AuditMemo::default());
        for r in refs {
            verifier.verify_refview(r)?;
        }
        Ok(())
    }
    pub fn verify_many_cached_owned<'a>(
        &mut self,
        refs: impl IntoIterator<Item = RefView<'a>>,
        mode: Mode,
    ) -> Result<(), CodecError> {
        for r in refs {
            self.verify_cached_owned(r, mode)?;
        }
        Ok(())
    }
    pub fn new_ref_result(&self) -> CodecVec<ContentRef> {
        WorkVec::new(Some(&self.budget))
    }
    pub fn lookup_array_items(
        &mut self,
        r: &ContentRef,
    ) -> Result<CodecVec<ContentRef>, CodecError> {
        path::lookup_array_items_ctx(&mut CodecContext::Closed(self), r)
    }
    pub fn lookup_object_fields(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<CodecFields, CodecError> {
        path::lookup_object_fields_ctx(&mut CodecContext::Closed(self), r, n)
    }
    pub fn lookup_object_fields_subset(
        &mut self,
        r: &ContentRef,
        n: &[&str],
    ) -> Result<CodecFields, CodecError> {
        path::lookup_object_fields_subset_ctx(&mut CodecContext::Closed(self), r, n)
    }
    pub fn render_canonical(
        &mut self,
        r: &ContentRef,
        out: &mut dyn Write,
    ) -> Result<(), CodecError> {
        render_ctx(CodecContext::Closed(self), r, out)
    }
    pub fn verify_fresh(&mut self, r: &ContentRef, mode: Mode) -> Result<(), CodecError> {
        Verifier::with_context(CodecContext::Closed(self), mode, AuditMemo::default())
            .verify_reference(r)
            .map(|_| ())
    }
    pub fn verify_many_fresh(
        &mut self,
        refs: impl IntoIterator<Item = ContentRef>,
        mode: Mode,
    ) -> Result<(), CodecError> {
        let mut verifier =
            Verifier::with_context(CodecContext::Closed(self), mode, AuditMemo::default());
        for r in refs {
            verifier.verify_reference(&r)?;
        }
        Ok(())
    }
    pub fn verify_cached(&mut self, r: &ContentRef, mode: Mode) -> Result<(), CodecError> {
        // 与原HashReader完全同一root选择：纯capture根复用capture各自Public/Template memo。
        if self.capture.contains(&r.digest()?) {
            self.capture.verify(r, mode)
        } else {
            self.verify_fresh(r, mode)
        }
    }
    pub fn verify_many_cached(
        &mut self,
        refs: impl IntoIterator<Item = ContentRef>,
        mode: Mode,
    ) -> Result<(), CodecError> {
        for r in refs {
            self.verify_cached(&r, mode)?;
        }
        Ok(())
    }
    pub fn replace_batch_placeholder(
        &mut self,
        r: &ContentRef,
        p: &[path::PathStep],
        b: &str,
    ) -> Result<ContentRef, CodecError> {
        path::replace_batch_placeholder_ctx(&mut CodecContext::Closed(self), r, p, b)
    }
    pub fn write_json_view(
        &mut self,
        v: capture_allocation::audit_json::JsonView<'_>,
    ) -> Result<Built, CodecError> {
        write_json_view_ctx(&mut CodecContext::Closed(self), v)
    }
    pub fn write_value(&mut self, v: &Value) -> Result<Built, CodecError> {
        write_in_mode(&mut CodecContext::Closed(self), v, Mode::Public)
    }
    pub fn write_template(&mut self, v: &Value) -> Result<Built, CodecError> {
        write_in_mode(&mut CodecContext::Closed(self), v, Mode::Template)
    }
    pub fn raw_put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), CodecError> {
        let material = FrameMaterial::checked(t, b);
        if material.digest != d {
            return Err(CodecError::new(
                code::NODE_DIGEST,
                "closed overlay put摘要与实际F不符",
            ));
        }
        self.put_material(material)
    }
    pub fn clear_overlay(&mut self) {
        self.overlay.clear();
    }
    pub fn overlay_peak_used(&self) -> (usize, usize) {
        self.overlay.peak_used()
    }
    pub fn set_audit_full_array_scan(&mut self, enabled: bool) {
        self.full_arrays = enabled;
    }
    /// 仅测量回调，无返回值、body/source/预算输入；不能改变读取分派或提供成功证明。
    pub fn set_trace_callbacks(&mut self, read: Option<fn(bool)>, put: Option<fn()>) {
        self.read_trace = read;
        self.put_trace = put;
    }
    pub fn set_overlay_peak_callback(&mut self, trace: Option<fn((usize, usize))>) {
        self.overlay_peak_trace = trace;
    }
    fn read(&mut self, d: &NodeDigest) -> Result<(u8, VerifierBody), CodecError> {
        let hit = self.overlay.original(d);
        if let Some(trace) = self.read_trace {
            trace(hit.is_some());
        }
        let (t, b) = if let Some(row) = hit {
            row
        } else {
            #[cfg(test)]
            trace_capture_body_read(d);
            self.capture.reads.set(self.capture.reads.get() + 1);
            self.capture.original(d).ok_or_else(|| {
                CodecError::new(code::NODE_MISSING, format!("节点缺失：{}", root_string(d)))
            })?
        };
        let body = super::capture_allocation::body::MeteredBody::copy_from(b, &self.budget)?;
        let actual = frame(t, &body);
        if actual != *d {
            return Err(CodecError::new(
                code::NODE_DIGEST,
                format!(
                    "节点 (tag, body) 与摘要不符：声称 {}，实算 {}；未核摘要不成帧",
                    root_string(d),
                    root_string(&actual)
                ),
            ));
        }
        Ok((t, VerifierBody::Metered(body)))
    }
    fn put_material(&mut self, m: FrameMaterial<'_>) -> Result<(), CodecError> {
        if let Some(trace) = self.put_trace {
            trace();
        }
        if let Some((t, b)) = self.capture.original(&m.digest) {
            if t != m.tag || b != m.body {
                return Err(CodecError::new(
                    code::NODE_DIGEST,
                    "派生overlay不能遮蔽不可变捕获原件",
                ));
            }
        }
        self.overlay.put(m.digest, m.tag, m.body)?;
        if let Some(trace) = self.overlay_peak_trace {
            trace(self.overlay.peak_used());
        }
        Ok(())
    }
}

/// 私有材料证明只能在本次真实F入口生成，借用同一完整body直到同步copy完成。
/// 它不证明Flavor、闭包、来源业务资格或全heap准入。
struct FrameMaterial<'body> {
    digest: NodeDigest,
    tag: u8,
    body: &'body [u8],
}
impl<'body> FrameMaterial<'body> {
    fn checked(tag: u8, body: &'body [u8]) -> Self {
        Self {
            digest: frame(tag, body),
            tag,
            body,
        }
    }
}

/// 私有静态分派；任意公开NodeSource不能自行提供Closed分支。
pub(super) enum CodecContext<'read, 'capture> {
    Source(&'read mut dyn NodeSource),
    Store(&'read mut dyn NodeStore),
    Sink(&'read mut dyn NodeSink, Option<OwnedBudget>),
    Capture(&'read AuditCapture),
    Closed(&'read mut ClosedCapturedOwner<'capture>),
}
impl<'read, 'capture> CodecContext<'read, 'capture> {
    pub(super) fn budget(&self) -> Option<OwnedBudget> {
        match self {
            Self::Capture(c) => match &c.nodes {
                CaptureNodes::AllocationPrototype(s) => Some(s.owned_budget()),
                _ => None,
            },
            Self::Closed(c) => Some(c.budget.clone()),
            Self::Sink(_, b) => b.clone(),
            _ => None,
        }
    }
    pub(super) fn reborrow(&mut self) -> CodecContext<'_, 'capture> {
        match self {
            Self::Source(s) => CodecContext::Source(&mut **s),
            Self::Store(s) => CodecContext::Store(&mut **s),
            Self::Sink(s, b) => CodecContext::Sink(&mut **s, b.clone()),
            Self::Capture(c) => CodecContext::Capture(c),
            Self::Closed(c) => CodecContext::Closed(&mut **c),
        }
    }
    pub(super) fn read(&mut self, d: &NodeDigest) -> Result<(u8, VerifierBody), CodecError> {
        match self {
            Self::Source(s) => read_checked(&mut **s, d).map(|(t, b)| (t, VerifierBody::Plain(b))),
            Self::Store(s) => read_checked(&mut **s, d).map(|(t, b)| (t, VerifierBody::Plain(b))),
            Self::Capture(c) => {
                let mut reader = CaptureReader(c);
                if c.is_allocation_prototype() {
                    read_body_checked(&mut reader, Some(c), d)
                } else {
                    read_checked(&mut reader, d).map(|(t, b)| (t, VerifierBody::Plain(b)))
                }
            }
            Self::Closed(c) => c.read(d),
            Self::Sink(_, _) => Err(CodecError::new(code::SOURCE, "只写context不可读节点")),
        }
    }
    pub(super) fn emit(&mut self, tag: u8, body: &[u8]) -> Result<NodeDigest, CodecError> {
        let m = FrameMaterial::checked(tag, body);
        let digest = m.digest;
        match self {
            Self::Store(s) => s.put(m.digest, m.tag, m.body)?,
            Self::Sink(s, _) => s.put(m.digest, m.tag, m.body)?,
            Self::Closed(c) => c.put_material(m)?,
            Self::Source(_) | Self::Capture(_) => {
                return Err(CodecError::new(code::SOURCE, "只读context不可写节点"))
            }
        }
        Ok(digest)
    }
    pub(super) fn before_path_index(
        &mut self,
        p: &[path::PathStep],
        i: u64,
    ) -> Result<(), CodecError> {
        match self {
            Self::Source(s) => s.before_path_index(p, i),
            Self::Store(s) => s.before_path_index(p, i),
            _ => Ok(()),
        }
    }
    pub(super) fn audit_full_array_scan(&self) -> bool {
        match self {
            Self::Source(s) => s.audit_full_array_scan(),
            Self::Store(s) => s.audit_full_array_scan(),
            Self::Closed(c) => c.full_arrays,
            _ => false,
        }
    }
    pub(super) fn max_render_bytes(&self) -> Option<u64> {
        match self {
            Self::Source(s) => s.max_render_bytes(),
            Self::Store(s) => s.max_render_bytes(),
            Self::Closed(c) => c.max_render_bytes,
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn capture(nodes: MemoryNodes, extra: usize) -> (AuditCapture, OwnedBudget, usize) {
        let count = nodes.nodes.len();
        let bytes = nodes.nodes.values().map(|(_, b)| b.len()).sum();
        let base = capture_allocation::CaptureSlab::requested_bytes(count, bytes).unwrap();
        let budget = OwnedBudget::new(base + extra);
        let mut slab = capture_allocation::CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, (t, b)) in nodes.nodes {
            slab.push(d, t, &b).unwrap();
        }
        (
            AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap()),
            budget,
            base,
        )
    }
    #[test]
    fn closed_codec_real_two_bodies_share_same_overlay_ledger_until_drop() {
        let mut nodes = MemoryNodes::new();
        let body = vec![b'a'; 4096];
        let d = frame(tag::BYTES_CHUNK, &body);
        nodes.insert_unchecked(d, tag::BYTES_CHUNK, body);
        let overlay = OverlaySlab::requested_bytes(1, 4).unwrap();
        let (cap, budget, base) = capture(nodes, overlay + 4096);
        let mut owner = cap
            .closed_codec_for_allocation_prototype(1, 4, None)
            .unwrap();
        let baseline = base + overlay;
        let before = budget.snapshot();
        let first = owner.read(&d).unwrap();
        assert_eq!(
            budget.snapshot().live_requested_layout_bytes,
            baseline + 4096
        );
        let second = owner.read(&d);
        assert!(second.is_err());
        assert_eq!(second.err().unwrap().code(), capture_allocation::LIMIT);
        assert_eq!(
            budget.snapshot().allocated_layouts,
            before.allocated_layouts + 1
        );
        assert_eq!(
            budget.snapshot().allocation_attempts,
            before.allocation_attempts + 1
        );
        drop(first);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, baseline);
        println!(
            "CLOSED_TWO_BODY_RECEIPT {}",
            serde_json::to_string(&budget.snapshot()).unwrap()
        );
        drop(owner);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        drop(cap);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn closed_codec_render_path_replace_and_encoder_same_material_f_order() {
        let mut nodes = MemoryNodes::new();
        let template = write_template(
            &mut nodes,
            &Value::Object(vec![
                ("batch_id".into(), Value::BatchSelf),
                ("text".into(), Value::string("汉".repeat(2000))),
                (
                    "view".into(),
                    Value::JsonText(Box::new(JsonValue::Object(vec![(
                        "inside".into(),
                        JsonValue::String("v".into()),
                    )]))),
                ),
            ]),
        )
        .unwrap();
        // 匹配旧history HashReader写层：编码器已算F，动态overlay只保存原材料。
        struct HistoryWriteLayer(MemoryNodes);
        impl NodeSource for HistoryWriteLayer {
            fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
                self.0.read(d)
            }
        }
        impl NodeSink for HistoryWriteLayer {
            fn put(&mut self, d: NodeDigest, t: u8, b: &[u8]) -> Result<(), CodecError> {
                self.0.insert_unchecked(d, t, b.to_vec());
                Ok(())
            }
        }
        let mut old = HistoryWriteLayer(nodes.clone());
        let overlay = OverlaySlab::requested_bytes(128, 16384).unwrap();
        let (cap, budget, base) = capture(nodes, overlay + 32768);
        let mut owner = cap
            .closed_codec_for_allocation_prototype(128, 16384, None)
            .unwrap();
        let baseline = base + overlay;
        let root = template.content_ref();
        let steps = [path::PathStep::Field("text".into())];
        let f = frame_calls_on_thread();
        let ordinary = path::lookup_path(&mut old, &root, &steps).unwrap();
        let n = frame_calls_on_thread() - f;
        let f = frame_calls_on_thread();
        let bounded = owner.lookup_path(&root, &steps).unwrap();
        assert_eq!(bounded, ordinary);
        assert_eq!(frame_calls_on_thread() - f, n);
        let mut plain = Vec::new();
        let f = frame_calls_on_thread();
        render_canonical(&mut old, &ordinary, &mut plain).unwrap();
        let n = frame_calls_on_thread() - f;
        let mut output = Vec::new();
        let f = frame_calls_on_thread();
        owner.render_canonical(&bounded, &mut output).unwrap();
        assert_eq!(output, plain);
        assert_eq!(frame_calls_on_thread() - f, n);
        let nested = [
            path::PathStep::Field("view".into()),
            path::PathStep::JsonText,
            path::PathStep::Field("inside".into()),
        ];
        assert_eq!(
            owner.lookup_path(&root, &nested).unwrap(),
            path::lookup_path(&mut old, &root, &nested).unwrap()
        );
        let batch = format!("batch3-{}", "1".repeat(64));
        let target = [path::PathStep::Field("batch_id".into())];
        let f = frame_calls_on_thread();
        let plain = path::replace_batch_placeholder(&mut old, &root, &target, &batch).unwrap();
        let n = frame_calls_on_thread() - f;
        let f = frame_calls_on_thread();
        let bounded = owner
            .replace_batch_placeholder(&root, &target, &batch)
            .unwrap();
        assert_eq!(bounded, plain);
        assert_eq!(frame_calls_on_thread() - f, n);
        owner.verify_fresh(&bounded, Mode::Public).unwrap();
        owner.verify_cached(&root, Mode::Template).unwrap();
        assert_eq!(
            owner.verify_fresh(&root, Mode::Public).unwrap_err().code(),
            code::TEMPLATE_PLACEHOLDER
        );
        let f = frame_calls_on_thread();
        let built = owner.write_value(&Value::Bool(true)).unwrap();
        assert_eq!(frame_calls_on_thread() - f, 1);
        assert_eq!(
            owner.content_ref_of(&built.root()).unwrap(),
            built.content_ref()
        );
        assert_eq!(budget.snapshot().live_requested_layout_bytes, baseline);
        println!(
            "CLOSED_SURFACE_RECEIPT {}",
            serde_json::to_string(&budget.snapshot()).unwrap()
        );
        drop(owner);
        drop(cap);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn closed_codec_bad_f_capture_conflict_missing_and_rejected_put_keep_old_bytes() {
        let mut nodes = MemoryNodes::new();
        let valid = write_value(&mut nodes, &Value::Bool(false)).unwrap();
        let invalid = frame(tag::BYTES_CHUNK, b"real");
        nodes.insert_unchecked(invalid, tag::BYTES_CHUNK, b"wrong".to_vec());
        let overlay = OverlaySlab::requested_bytes(1, 4).unwrap();
        let (cap, budget, base) = capture(nodes, overlay + 8192);
        let mut owner = cap
            .closed_codec_for_allocation_prototype(1, 4, None)
            .unwrap();
        let baseline = base + overlay;
        assert_eq!(
            owner.read(&invalid).err().unwrap().code(),
            code::NODE_DIGEST
        );
        assert_eq!(
            owner
                .raw_put(invalid, tag::BYTES_CHUNK, b"real")
                .unwrap_err()
                .code(),
            code::NODE_DIGEST
        );
        assert_eq!(
            owner
                .raw_put([1; 32], tag::BOOL_TRUE, &[])
                .unwrap_err()
                .code(),
            code::NODE_DIGEST
        );
        let d = frame(tag::BYTES_CHUNK, b"abcd");
        owner.raw_put(d, tag::BYTES_CHUNK, b"abcd").unwrap();
        let e = frame(tag::BYTES_CHUNK, b"other");
        assert_eq!(
            owner
                .raw_put(e, tag::BYTES_CHUNK, b"other")
                .unwrap_err()
                .code(),
            capture_allocation::LIMIT
        );
        assert_eq!(&*owner.read(&d).unwrap().1, b"abcd");
        assert_eq!(
            owner.read(&[2; 32]).err().unwrap().code(),
            code::NODE_MISSING
        );
        assert_eq!(
            owner.content_ref_of(&valid.root()).unwrap(),
            valid.content_ref()
        );
        owner.clear_overlay();
        assert_eq!(budget.snapshot().live_requested_layout_bytes, baseline);
        assert_eq!(owner.read(&d).err().unwrap().code(), code::NODE_MISSING);
        let unrelated = OwnedBudget::new(0);
        assert_eq!(unrelated.snapshot().allocated_layouts, 0);
        assert!(AuditCapture::new(MemoryNodes::new())
            .closed_codec_for_allocation_prototype(0, 0, None)
            .is_err());
        drop(owner);
        drop(cap);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        assert_eq!(unrelated.snapshot().allocated_layouts, 0);
    }
}

#[cfg(test)]
mod ordinary_sink_regression {
    use super::*;
    #[test]
    fn closed_codec_normal_memorynodes_keeps_second_f_and_bad_digest_refusal() {
        let mut nodes = MemoryNodes::new();
        let before = frame_calls_on_thread();
        let value = write_value(&mut nodes, &Value::Bool(true)).unwrap();
        assert_eq!(
            frame_calls_on_thread() - before,
            2,
            "普通MemoryNodes原encoder+sink两次实际F必须保留"
        );
        assert_eq!(
            content_ref_of(&mut nodes, &value.root()).unwrap(),
            value.content_ref()
        );
        let wrong = frame(tag::BOOL_FALSE, &[]);
        let before = frame_calls_on_thread();
        let error = nodes.put(wrong, tag::BOOL_TRUE, &[]).unwrap_err();
        assert_eq!(error.code(), code::SINK);
        assert_eq!(frame_calls_on_thread() - before, 1);
        assert!(!nodes.contains(&wrong));
    }
}

#[cfg(test)]
mod codec_workset_growth_probe {
    use super::*;
    fn make_capture(nodes: MemoryNodes, extra: usize) -> (AuditCapture, OwnedBudget, usize) {
        let count = nodes.len();
        let bytes = nodes.iter().map(|(_, _, b)| b.len()).sum();
        let base = capture_allocation::CaptureSlab::requested_bytes(count, bytes).unwrap();
        let budget = OwnedBudget::new(base + extra);
        let mut slab = capture_allocation::CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, t, b) in nodes.iter() {
            slab.push(*d, t, b).unwrap();
        }
        (
            AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap()),
            budget,
            base,
        )
    }
    #[test]
    fn codec_path_stack_next_layout_held_old_block_refuses_before_growth() {
        let mut nodes = MemoryNodes::new();
        let array = write_value(
            &mut nodes,
            &Value::Array(vec![Value::Null, Value::Bool(true)]),
        )
        .unwrap();
        let (cap, budget, base) = make_capture(nodes, 215);
        let mut owner = cap
            .closed_codec_for_allocation_prototype(0, 0, None)
            .unwrap();
        let got = owner.lookup_array_items(&array.content_ref());
        assert!(got.is_err());
        assert_eq!(got.err().unwrap().code(), capture_allocation::LIMIT);
        let probe = budget.allocation_request_probe();
        assert_eq!(probe.refused_bytes, 192);
        assert_eq!(probe.live_before_refusal, base + 48);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        println!(
            "PATH_GROW_LAYOUT_RECEIPT {}",
            serde_json::to_string(&probe).unwrap()
        );
        drop(owner);
        drop(cap);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn codec_leaf_iterator_stack_counts_old_new_body_live_before_growth() {
        let mut nodes = MemoryNodes::new();
        let value = write_value(&mut nodes, &Value::string("x".repeat(8192))).unwrap();
        let seq = digest_at(nodes.get(&value.digest).unwrap().1, 8);
        let (cap, budget, base) = make_capture(nodes, 167);
        let mut owner = cap
            .closed_codec_for_allocation_prototype(0, 0, None)
            .unwrap();
        let mut source = CodecContext::Closed(&mut owner);
        let mut iter = LeafIter::new(seq, source.budget().as_ref()).unwrap();
        let got = iter.next_context(&mut source);
        assert!(got.is_err());
        assert_eq!(got.err().unwrap().code(), capture_allocation::LIMIT);
        let probe = budget.allocation_request_probe();
        assert_eq!(probe.refused_bytes, 128);
        assert_eq!(probe.live_before_refusal, base + 104);
        drop(iter);
        drop(source);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        println!(
            "LEAF_GROW_LAYOUT_RECEIPT {}",
            serde_json::to_string(&probe).unwrap()
        );
        drop(owner);
        drop(cap);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn codec_large_ordered_fields_keep_key_storage_and_logarithmic_lookup() {
        let mut nodes = MemoryNodes::new();
        let count = 2048;
        let fields = (0..count)
            .rev()
            .map(|i| (format!("键-{i:05}"), Value::Null))
            .collect();
        let object = write_value(&mut nodes, &Value::Object(fields)).unwrap();
        let names = (0..count).map(|i| format!("键-{i:05}")).collect::<Vec<_>>();
        let wanted = names.iter().map(String::as_str).collect::<Vec<_>>();
        let mut plain = nodes.clone();
        let f = frame_calls_on_thread();
        let expected =
            path::lookup_object_fields(&mut plain, &object.content_ref(), &wanted).unwrap();
        let plain_f = frame_calls_on_thread() - f;
        let (cap, budget, base) = make_capture(nodes, 4 * 1024 * 1024);
        let mut owner = cap
            .closed_codec_for_allocation_prototype(0, 0, None)
            .unwrap();
        let f = frame_calls_on_thread();
        let started = std::time::Instant::now();
        let actual = owner
            .lookup_object_fields(&object.content_ref(), &wanted)
            .unwrap();
        let elapsed = started.elapsed().as_nanos();
        assert_eq!(frame_calls_on_thread() - f, plain_f);
        assert_eq!(actual.len(), count);
        for name in &wanted {
            assert_eq!(actual.get(name), expected.get(*name));
        }
        drop(owner);
        assert!(
            budget.snapshot().live_requested_layout_bytes > base,
            "返还离开codec后的字段entry+UTF8 key必须仍真实计量"
        );
        println!(
            "ORDERED_FIELDS_RECEIPT {}",
            serde_json::json!({"fields":count,"complexity":"amortized append O(1), canonical sort O(n log n), query O(log n)","f_calls":plain_f,"elapsed_ns":elapsed.to_string(),"meter":budget.snapshot()})
        );
        drop(actual);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        drop(cap);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn codec_renderer_task_growth_old_tasks_leaf_stack_and_body_are_live() {
        let mut nodes = MemoryNodes::new();
        let value = write_value(&mut nodes, &Value::Array(vec![Value::Null])).unwrap();
        let task = std::mem::size_of::<RTask>();
        let (cap, budget, base) = make_capture(nodes, 5 * task + 71);
        let mut owner = cap
            .closed_codec_for_allocation_prototype(0, 0, None)
            .unwrap();
        let mut output = Vec::new();
        let result = Renderer {
            source: CodecContext::Closed(&mut owner),
            out: &mut output,
            escaping: false,
            esc_decoder: Utf8Decoder::default(),
        }
        .run(value.digest);
        assert_eq!(result.unwrap_err().code(), capture_allocation::LIMIT);
        let probe = budget.allocation_request_probe();
        assert_eq!(probe.refused_bytes, 4 * task);
        assert_eq!(probe.live_before_refusal, base + task + 72);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
        assert!(output.is_empty());
        println!(
            "RENDER_TASK_GROW_RECEIPT {}",
            serde_json::to_string(&probe).unwrap()
        );
        drop(owner);
        drop(cap);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}

#[cfg(test)]
mod writer_error_cause_probe {
    use super::*;
    #[test]
    fn normal_external_writer_oom_stays_io_owned_internal_quota_stays_limit() {
        struct OomWriter;
        impl Write for OomWriter {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(std::io::ErrorKind::OutOfMemory))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut nodes = MemoryNodes::new();
        let value = write_value(&mut nodes, &Value::Bool(true)).unwrap();
        let error = render_canonical(&mut nodes, &value.content_ref(), &mut OomWriter).unwrap_err();
        assert_eq!(
            error,
            CodecError::new(
                code::IO,
                std::io::Error::from(std::io::ErrorKind::OutOfMemory).to_string()
            )
        );
        let b = OwnedBudget::new(1);
        let mut owned = WorkVec::with_capacity(1, Some(&b)).unwrap();
        let error = write_escaped_char_work('\u{1}', &mut owned).unwrap_err();
        assert_eq!(error.code(), capture_allocation::LIMIT);
        assert!(matches!(error.detail, std::borrow::Cow::Borrowed(_)));
        assert_eq!(owned.len(), 0);
        assert_eq!(b.snapshot().allocated_layouts, 1);
        drop(owned);
        assert_eq!(b.snapshot().live_requested_layout_bytes, 0);
    }
}

#[cfg(test)]
mod owned_ref_tests {
    use super::*;
    fn capture(nodes: MemoryNodes, extra: usize) -> (AuditCapture, OwnedBudget, usize) {
        let count = nodes.nodes.len();
        let bytes = nodes.nodes.values().map(|(_, b)| b.len()).sum();
        let base = capture_allocation::CaptureSlab::requested_bytes(count, bytes).unwrap();
        let budget = OwnedBudget::new(base + extra);
        let mut slab = capture_allocation::CaptureSlab::new(count, bytes, &budget).unwrap();
        for (d, (t, b)) in nodes.nodes {
            slab.push(d, t, &b).unwrap();
        }
        (
            AuditCapture::from_slab_for_allocation_prototype(slab.finish().unwrap()),
            budget,
            base,
        )
    }
    fn equal_fields(actual: &RefOwner, expected: &ContentRef) {
        assert_eq!(actual.format(), expected.format);
        assert_eq!(actual.root(), expected.root);
        assert_eq!(actual.kind(), expected.kind);
        assert_eq!(actual.extent(), expected.extent);
        assert_eq!(actual.digest(), expected.digest().unwrap());
    }
    fn traced<T>(f: impl FnOnce() -> T) -> (T, Vec<(NodeDigest, u64)>, u64) {
        let start = frame_calls_on_thread();
        CAPTURE_BODY_READ_TRACE.with(|t| {
            assert!(t.borrow().is_none());
            *t.borrow_mut() = Some((start, Vec::new()));
        });
        let value = f();
        let reads = CAPTURE_BODY_READ_TRACE.with(|t| t.borrow_mut().take().unwrap().1);
        (value, reads, frame_calls_on_thread() - start)
    }
    struct TracedSource {
        nodes: MemoryNodes,
        start: u64,
        reads: Vec<(NodeDigest, u64)>,
    }
    impl NodeSource for TracedSource {
        fn read(&mut self, d: &NodeDigest) -> Result<NodeRead, CodecError> {
            self.reads.push((*d, frame_calls_on_thread() - self.start));
            self.nodes.read(d)
        }
    }
    #[test]
    fn owned_lookup_matches_normal_fields_read_order_f_and_errors() {
        let values = [
            Value::Array(vec![
                Value::Null,
                Value::Bool(true),
                Value::string("汉x".repeat(2500)),
            ]),
            Value::Object(vec![
                ("a".into(), Value::Array(vec![Value::Null])),
                ("b".into(), Value::Bool(false)),
                ("c".into(), Value::string("x")),
            ]),
            Value::JsonText(Box::new(JsonValue::Array(vec![JsonValue::Null]))),
        ];
        for (case, value) in values.into_iter().enumerate() {
            let mut nodes = MemoryNodes::new();
            let root = write_value(&mut nodes, &value).unwrap().content_ref();
            let (cap, budget, base) = capture(nodes.clone(), 32768);
            let mut closed = cap
                .closed_codec_for_allocation_prototype(0, 0, None)
                .unwrap();
            let paths = match case {
                0 => vec![
                    vec![],
                    vec![path::PathStep::Index(2)],
                    vec![path::PathStep::Index(3)],
                    vec![path::PathStep::Field("a".into())],
                ],
                1 => vec![
                    vec![],
                    vec![path::PathStep::Field("a".into()), path::PathStep::Index(0)],
                    vec![path::PathStep::Field("missing".into())],
                ],
                _ => vec![
                    vec![],
                    vec![path::PathStep::JsonText, path::PathStep::Index(0)],
                    vec![path::PathStep::JsonText, path::PathStep::JsonText],
                ],
            };
            for steps in paths {
                let start = frame_calls_on_thread();
                let mut plain = TracedSource {
                    nodes: nodes.clone(),
                    start,
                    reads: Vec::new(),
                };
                let expected = path::lookup_path(&mut plain, &root, &steps);
                let calls = frame_calls_on_thread() - start;
                let (actual, reads, f) =
                    traced(|| closed.lookup_path_owned((&root).into(), &steps));
                match (actual, expected) {
                    (Ok(a), Ok(e)) => equal_fields(&a, &e),
                    (Err(a), Err(e)) => assert_eq!(a, e),
                    pair => panic!("result mismatch {pair:?}"),
                }
                assert_eq!(reads, plain.reads);
                assert_eq!(f, calls);
            }
            // Invalid incoming reference keeps format/root/kind/extent precedence.
            for part in 0..4 {
                let mut invalid = root.clone();
                match part {
                    0 => invalid.format = "bad".into(),
                    1 => invalid.root = "bad".into(),
                    2 => invalid.kind = Kind::TemplateBatch,
                    _ => invalid.extent = Some(99),
                };
                let start = frame_calls_on_thread();
                let mut plain = TracedSource {
                    nodes: nodes.clone(),
                    start,
                    reads: Vec::new(),
                };
                let expected = path::lookup_path(&mut plain, &invalid, &[]).unwrap_err();
                let calls = frame_calls_on_thread() - start;
                let (actual, reads, f) =
                    traced(|| closed.lookup_path_owned((&invalid).into(), &[]));
                assert_eq!(actual.unwrap_err(), expected);
                assert_eq!(reads, plain.reads);
                assert_eq!(f, calls);
            }
            if case == 0 {
                let start = frame_calls_on_thread();
                let mut plain = TracedSource {
                    nodes: nodes.clone(),
                    start,
                    reads: Vec::new(),
                };
                let expected = path::lookup_array_items(&mut plain, &root).unwrap();
                let calls = frame_calls_on_thread() - start;
                let (actual, reads, f) = traced(|| closed.lookup_array_items_owned((&root).into()));
                let actual = actual.unwrap();
                for (a, e) in actual.iter().zip(&expected) {
                    equal_fields(a, e);
                }
                assert_eq!(actual.len(), expected.len());
                assert_eq!(reads, plain.reads);
                assert_eq!(f, calls);
                drop(actual);
            }
            if case == 1 {
                for (names, exact) in [
                    (["c", "a", "b"].as_slice(), true),
                    (["c", "a"].as_slice(), false),
                ] {
                    let start = frame_calls_on_thread();
                    let mut plain = TracedSource {
                        nodes: nodes.clone(),
                        start,
                        reads: Vec::new(),
                    };
                    let expected = if exact {
                        path::lookup_object_fields(&mut plain, &root, names)
                    } else {
                        path::lookup_object_fields_subset(&mut plain, &root, names)
                    }
                    .unwrap();
                    let calls = frame_calls_on_thread() - start;
                    let (actual, reads, f) = traced(|| {
                        if exact {
                            closed.lookup_object_fields_owned((&root).into(), names)
                        } else {
                            closed.lookup_object_fields_subset_owned((&root).into(), names)
                        }
                    });
                    let actual = actual.unwrap();
                    for (name, r) in actual.iter() {
                        equal_fields(r, &expected[name]);
                    }
                    assert_eq!(actual.len(), expected.len());
                    assert_eq!(reads, plain.reads);
                    assert_eq!(f, calls);
                    drop(actual);
                }
                for (names, exact) in [
                    (["missing"].as_slice(), false),
                    (["a", "a", "c"].as_slice(), true),
                    (["a"].as_slice(), true),
                ] {
                    let mut plain = nodes.clone();
                    let expected = if exact {
                        path::lookup_object_fields(&mut plain, &root, names)
                    } else {
                        path::lookup_object_fields_subset(&mut plain, &root, names)
                    }
                    .err()
                    .unwrap();
                    let actual = if exact {
                        closed.lookup_object_fields_owned((&root).into(), names)
                    } else {
                        closed.lookup_object_fields_subset_owned((&root).into(), names)
                    }
                    .err()
                    .unwrap();
                    assert_eq!(actual, expected);
                }
            }
            assert_eq!(budget.snapshot().live_requested_layout_bytes, base);
            drop(closed);
            drop(cap);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn owned_lookup_target_bad_material_and_missing_errors_match_normal() {
        for missing in [false, true] {
            let mut nodes = MemoryNodes::new();
            let built = write_value(&mut nodes, &Value::Array(vec![Value::Bool(true)])).unwrap();
            let child = frame(tag::BOOL_TRUE, &[]);
            if missing {
                nodes.nodes.remove(&child);
            } else {
                nodes.insert_unchecked(child, tag::BOOL_TRUE, vec![1]);
            }
            let reference = built.content_ref();
            let mut plain = nodes.clone();
            let expected =
                path::lookup_path(&mut plain, &reference, &[path::PathStep::Index(0)]).unwrap_err();
            let (cap, budget, _) = capture(nodes, 8192);
            let mut closed = cap
                .closed_codec_for_allocation_prototype(0, 0, None)
                .unwrap();
            assert_eq!(
                closed
                    .lookup_path_owned((&reference).into(), &[path::PathStep::Index(0)])
                    .unwrap_err(),
                expected
            );
            drop(closed);
            drop(cap);
            assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        }
    }
    #[test]
    fn owned_ref_verify_render_and_nested_json_flavor_share_normal_kernel() {
        let mut nodes = MemoryNodes::new();
        let built = write_value(
            &mut nodes,
            &Value::Object(vec![
                (
                    "a".into(),
                    Value::Array(vec![Value::Null, Value::Bool(true)]),
                ),
                ("b".into(), Value::string("汉")),
            ]),
        )
        .unwrap();
        let reference = built.content_ref();
        let legacy = AuditCapture::new(nodes.clone());
        let mut plain = nodes.clone();
        let (cap, budget, _) = capture(nodes, 32768);
        let mut closed = cap
            .closed_codec_for_allocation_prototype(0, 0, None)
            .unwrap();
        let owned = closed.content_ref_owned(&reference.root).unwrap();
        let f = frame_calls_on_thread();
        verify(&mut plain, &reference).unwrap();
        let calls = frame_calls_on_thread() - f;
        let f = frame_calls_on_thread();
        closed
            .verify_fresh_owned(owned.view(), Mode::Public)
            .unwrap();
        assert_eq!(frame_calls_on_thread() - f, calls);
        for _ in 0..2 {
            let f = frame_calls_on_thread();
            legacy.verify(&reference, Mode::Public).unwrap();
            let calls = frame_calls_on_thread() - f;
            let f = frame_calls_on_thread();
            closed
                .verify_cached_owned(owned.view(), Mode::Public)
                .unwrap();
            assert_eq!(frame_calls_on_thread() - f, calls);
        }
        let mut ordinary = Vec::new();
        let mut actual = Vec::new();
        let f = frame_calls_on_thread();
        render_canonical(&mut plain, &reference, &mut ordinary).unwrap();
        let calls = frame_calls_on_thread() - f;
        let f = frame_calls_on_thread();
        closed
            .render_canonical_owned(owned.view(), &mut actual)
            .unwrap();
        assert_eq!(actual, ordinary);
        assert_eq!(frame_calls_on_thread() - f, calls);
        drop(closed);
        drop(cap);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);

        // Each node has a valid F, but a nested 0x07 inside JsonInner is forbidden.
        let mut nodes = MemoryNodes::new();
        let null = frame(tag::NULL, &[]);
        nodes.insert_unchecked(null, tag::NULL, vec![]);
        let nested = frame(tag::JSON_TEXT, &null);
        nodes.insert_unchecked(nested, tag::JSON_TEXT, null.to_vec());
        let seq = frame(tag::SEQ_LEAF, &nested);
        nodes.insert_unchecked(seq, tag::SEQ_LEAF, nested.to_vec());
        let mut array = 1u64.to_be_bytes().to_vec();
        array.extend_from_slice(&seq);
        let inner = frame(tag::ARRAY, &array);
        nodes.insert_unchecked(inner, tag::ARRAY, array);
        let outer = frame(tag::JSON_TEXT, &inner);
        nodes.insert_unchecked(outer, tag::JSON_TEXT, inner.to_vec());
        let reference = ContentRef::new(Kind::JsonText, &outer, None);
        let path = [path::PathStep::JsonText, path::PathStep::Index(0)];
        let mut plain = nodes.clone();
        let expected = path::lookup_path(&mut plain, &reference, &path).unwrap_err();
        let (cap, budget, _) = capture(nodes, 32768);
        let mut closed = cap
            .closed_codec_for_allocation_prototype(0, 0, None)
            .unwrap();
        let owned = closed.content_ref_owned(&reference.root).unwrap();
        assert_eq!(
            closed.lookup_path_owned(owned.view(), &path).unwrap_err(),
            expected
        );
        let expected = verify(&mut plain, &reference).unwrap_err();
        assert_eq!(
            closed
                .verify_fresh_owned(owned.view(), Mode::Public)
                .unwrap_err(),
            expected
        );
        let mut ordinary = Vec::new();
        let mut actual = Vec::new();
        let expected = render_canonical(&mut plain, &reference, &mut ordinary).unwrap_err();
        assert_eq!(
            closed
                .render_canonical_owned(owned.view(), &mut actual)
                .unwrap_err(),
            expected
        );
        assert!(actual.is_empty());
        assert!(ordinary.is_empty());
        drop(closed);
        drop(cap);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[test]
    fn owned_ref_results_share_capture_ledger_on_move_clone_growth_and_drop() {
        let size = std::mem::size_of::<RefOwner>();
        let (cap, budget, _) = capture(MemoryNodes::new(), 3 * size - 1);
        let closed = cap
            .closed_codec_for_allocation_prototype(0, 0, None)
            .unwrap();
        let mut refs = closed.new_owned_ref_result();
        refs.reserve_exact(1).unwrap();
        refs.try_push(RefOwner::new(Kind::Null, [1; 32], None))
            .unwrap();
        let before = budget.snapshot();
        assert_eq!(
            refs.reserve_exact(1).unwrap_err().code(),
            capture_allocation::LIMIT
        );
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].digest(), [1; 32]);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, size);
        assert_eq!(
            budget.snapshot().allocated_layouts,
            before.allocated_layouts
        );
        drop(closed);
        drop(cap);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, size);
        let moved = refs;
        drop(moved);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
        let (cap, budget, _) = capture(MemoryNodes::new(), 16 * size);
        let closed = cap
            .closed_codec_for_allocation_prototype(0, 0, None)
            .unwrap();
        let mut refs = closed.new_owned_ref_result();
        refs.try_push(RefOwner::new(Kind::Null, [2; 32], None))
            .unwrap();
        let copy = refs.try_clone().unwrap();
        drop(closed);
        drop(cap);
        drop(refs);
        assert!(budget.snapshot().live_requested_layout_bytes > 0);
        assert_eq!(copy[0].digest(), [2; 32]);
        drop(copy);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
    #[cfg(feature = "s_allocation_json_testonly")]
    #[test]
    fn owned_lookup_global_observer_has_only_real_ledger_allocations() {
        use capture_allocation::audit_json::tests::observed;
        let mut nodes = MemoryNodes::new();
        let object = write_value(
            &mut nodes,
            &Value::Object(vec![
                ("a".into(), Value::Null),
                ("b".into(), Value::Bool(true)),
            ]),
        )
        .unwrap();
        let array = write_value(
            &mut nodes,
            &Value::Array(vec![Value::Null, Value::Bool(true)]),
        )
        .unwrap();
        let object_root = object.root();
        let array_root = array.root();
        let (cap, budget, _) = capture(nodes, 32768);
        let mut closed = cap
            .closed_codec_for_allocation_prototype(0, 0, None)
            .unwrap();
        frame_calls_on_thread();
        for operation in 0..5 {
            let attempts = budget.snapshot().allocation_attempts;
            let ((), calls) = observed(|| {
                let r = closed.content_ref_owned(&object_root).unwrap();
                match operation {
                    0 => {}
                    1 => {
                        closed.lookup_path_owned(r.view(), &[]).unwrap();
                    }
                    2 => {
                        closed
                            .lookup_object_fields_owned(r.view(), &["a", "b"])
                            .unwrap();
                    }
                    3 => {
                        closed
                            .lookup_object_fields_subset_owned(r.view(), &["a"])
                            .unwrap();
                    }
                    _ => {
                        let r = closed.content_ref_owned(&array_root).unwrap();
                        closed.lookup_array_items_owned(r.view()).unwrap();
                    }
                }
            });
            assert_eq!(
                calls,
                budget.snapshot().allocation_attempts - attempts,
                "operation {operation}: unledgered alloc"
            );
        }
        drop(closed);
        drop(cap);
        assert_eq!(budget.snapshot().live_requested_layout_bytes, 0);
    }
}
