//! #1394 实际审计返回头；normal DTO保持，返回存储不擦除owned文本/引用。
use super::{
    audit_store::{CodecRead, NativeAdmission, RefValue},
    business_json::TextValue,
};
use crate::content_manifest::{MetadataView, RootsView, SourceCommitView};

pub(crate) struct AuditSourceCommit {
    pub source_namespace: TextValue,
    pub source_epoch: TextValue,
    pub message_id: TextValue,
    pub payload_hash: TextValue,
    pub producer_id: TextValue,
    pub producer_epoch: TextValue,
    pub clock_event_id: TextValue,
    pub attempt_index: i64,
}

pub(crate) struct AuditMetadata {
    pub session_id: TextValue,
    pub session_generation: TextValue,
    pub writer_epoch: TextValue,
    pub base_cut: TextValue,
    pub structure_cut: TextValue,
    pub clock_plan_hash: TextValue,
    pub semantic_commit_ns: TextValue,
    pub profile_id: TextValue,
    pub profile_hash: TextValue,
    pub rule_revision: TextValue,
    pub catalog_revision: TextValue,
    pub generation: i64,
    pub input_frontier: i64,
    pub source_commit: AuditSourceCommit,
    pub scope: RefValue,
}
pub(crate) struct AuditRoots {
    pub generated_state: RefValue,
    pub publication_template: RefValue,
    pub delta_template: RefValue,
}
impl AuditSourceCommit {
    pub fn view(&self) -> SourceCommitView<'_> {
        SourceCommitView {
            source_namespace: self.source_namespace.as_str(),
            source_epoch: self.source_epoch.as_str(),
            message_id: self.message_id.as_str(),
            payload_hash: self.payload_hash.as_str(),
            producer_id: self.producer_id.as_str(),
            producer_epoch: self.producer_epoch.as_str(),
            clock_event_id: self.clock_event_id.as_str(),
            attempt_index: self.attempt_index,
        }
    }
}
impl AuditMetadata {
    pub fn view(&self) -> MetadataView<'_> {
        MetadataView {
            session_id: self.session_id.as_str(),
            session_generation: self.session_generation.as_str(),
            writer_epoch: self.writer_epoch.as_str(),
            base_cut: self.base_cut.as_str(),
            structure_cut: self.structure_cut.as_str(),
            clock_plan_hash: self.clock_plan_hash.as_str(),
            semantic_commit_ns: self.semantic_commit_ns.as_str(),
            profile_id: self.profile_id.as_str(),
            profile_hash: self.profile_hash.as_str(),
            rule_revision: self.rule_revision.as_str(),
            catalog_revision: self.catalog_revision.as_str(),
            generation: self.generation,
            input_frontier: self.input_frontier,
            source_commit: self.source_commit.view(),
            scope: self.scope.view(),
        }
    }
}
impl AuditRoots {
    pub fn view(&self) -> RootsView<'_> {
        RootsView {
            generated_state: self.generated_state.view(),
            publication_template: self.publication_template.view(),
            delta_template: self.delta_template.view(),
        }
    }
}
impl AuditMetadata {
    #[allow(dead_code, reason = "#1608：类型配套访问器，当前无调用方；保留不删")]
    pub fn try_clone(&self) -> Result<Self, String> {
        self.try_clone_admitted(&NativeAdmission(None))
    }
    pub fn try_clone_admitted(&self, store: &(impl CodecRead + ?Sized)) -> Result<Self, String> {
        Ok(Self {
            session_id: self.session_id.try_clone_admitted(store.codec_work())?,
            session_generation: self
                .session_generation
                .try_clone_admitted(store.codec_work())?,
            writer_epoch: self.writer_epoch.try_clone_admitted(store.codec_work())?,
            base_cut: self.base_cut.try_clone_admitted(store.codec_work())?,
            structure_cut: self.structure_cut.try_clone_admitted(store.codec_work())?,
            clock_plan_hash: self
                .clock_plan_hash
                .try_clone_admitted(store.codec_work())?,
            semantic_commit_ns: self
                .semantic_commit_ns
                .try_clone_admitted(store.codec_work())?,
            profile_id: self.profile_id.try_clone_admitted(store.codec_work())?,
            profile_hash: self.profile_hash.try_clone_admitted(store.codec_work())?,
            rule_revision: self.rule_revision.try_clone_admitted(store.codec_work())?,
            catalog_revision: self
                .catalog_revision
                .try_clone_admitted(store.codec_work())?,
            generation: self.generation,
            input_frontier: self.input_frontier,
            source_commit: AuditSourceCommit {
                source_namespace: self
                    .source_commit
                    .source_namespace
                    .try_clone_admitted(store.codec_work())?,
                source_epoch: self
                    .source_commit
                    .source_epoch
                    .try_clone_admitted(store.codec_work())?,
                message_id: self
                    .source_commit
                    .message_id
                    .try_clone_admitted(store.codec_work())?,
                payload_hash: self
                    .source_commit
                    .payload_hash
                    .try_clone_admitted(store.codec_work())?,
                producer_id: self
                    .source_commit
                    .producer_id
                    .try_clone_admitted(store.codec_work())?,
                producer_epoch: self
                    .source_commit
                    .producer_epoch
                    .try_clone_admitted(store.codec_work())?,
                clock_event_id: self
                    .source_commit
                    .clock_event_id
                    .try_clone_admitted(store.codec_work())?,
                attempt_index: self.source_commit.attempt_index,
            },
            scope: store
                .native_ref_clone(&self.scope)
                .map_err(|e| e.to_string())?,
        })
    }
}
impl std::fmt::Debug for AuditSourceCommit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceCommit")
            .field("source_namespace", &self.source_namespace)
            .field("source_epoch", &self.source_epoch)
            .field("message_id", &self.message_id)
            .field("payload_hash", &self.payload_hash)
            .field("producer_id", &self.producer_id)
            .field("producer_epoch", &self.producer_epoch)
            .field("clock_event_id", &self.clock_event_id)
            .field("attempt_index", &self.attempt_index)
            .finish()
    }
}
impl std::fmt::Debug for AuditMetadata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManifestMetadata")
            .field("session_id", &self.session_id)
            .field("session_generation", &self.session_generation)
            .field("writer_epoch", &self.writer_epoch)
            .field("generation", &self.generation)
            .field("base_cut", &self.base_cut)
            .field("structure_cut", &self.structure_cut)
            .field("input_frontier", &self.input_frontier)
            .field("source_commit", &self.source_commit)
            .field("clock_plan_hash", &self.clock_plan_hash)
            .field("semantic_commit_ns", &self.semantic_commit_ns)
            .field("profile_id", &self.profile_id)
            .field("profile_hash", &self.profile_hash)
            .field("rule_revision", &self.rule_revision)
            .field("catalog_revision", &self.catalog_revision)
            .field("scope", &self.scope)
            .finish()
    }
}
impl std::fmt::Debug for AuditRoots {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManifestRoots")
            .field("generated_state", &self.generated_state)
            .field("publication_template", &self.publication_template)
            .field("delta_template", &self.delta_template)
            .finish()
    }
}

#[cfg(feature = "s_incremental")]
mod owner_retention {
    use super::*;
    use crate::v3::audit_retention::retained_struct;

    retained_struct!(AuditSourceCommit {
        source_namespace,
        source_epoch,
        message_id,
        payload_hash,
        producer_id,
        producer_epoch,
        clock_event_id,
        attempt_index
    });
    retained_struct!(AuditMetadata {
        session_id,
        session_generation,
        writer_epoch,
        base_cut,
        structure_cut,
        clock_plan_hash,
        semantic_commit_ns,
        profile_id,
        profile_hash,
        rule_revision,
        catalog_revision,
        generation,
        input_frontier,
        source_commit,
        scope
    });
    retained_struct!(AuditRoots {
        generated_state,
        publication_template,
        delta_template
    });
}
