#!/usr/bin/env python3
"""#1488：按 cargo JSON 原测试二进制逐名列举和执行 A/B/C 锁。"""

from __future__ import annotations

import argparse
from collections import Counter
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time

FEATURES = {"Plain": "s_session", "Owned": "s_session,s_allocation_json_testonly", "Native": "s_session,s_capture_native_testonly"}
FEATURE_CLOSURES = {
    "Plain": frozenset({"default", "s_session"}),
    "Owned": frozenset({"default", "s_session", "s_allocation_json_testonly"}),
    "Native": frozenset({"default", "s_session", "s_incremental", "s_capture_native_testonly"}),
}


def check_feature_closure(mode: str, features) -> None:
    if set(features) != FEATURE_CLOSURES[mode]:
        raise ValueError(f"实际编译 features 不符：{features}")


# 源码包：A 878bd13a6b / B a24ce62cae / C df9898b928；共38条独立锁。
LOCKS = [
    {
        "lane": "A",
        "name": "v3::generation::tests::delta_raw_plain_append_revision_swap_and_roots_match_full_oracle",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs"
    },
    {
        "lane": "A",
        "name": "v3::generation::tests::delta_raw_owned_append_revision_swap_and_roots_match_full_oracle",
        "target": "bin",
        "modes": [
            "Owned"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs"
    },
    {
        "lane": "A",
        "name": "v3::generation::tests::delta_raw_expired_foreign_domain_and_rollback_cannot_reuse_pending_state",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs"
    },
    {
        "lane": "A",
        "name": "v3::generation::tests::delta_raw_initial_write_failure_revokes_token_and_requires_new_owner_domain",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs"
    },
    {
        "lane": "A",
        "name": "v3::generation::tests::delta_sequence_plain_rank_splice_matches_independent_full_writer",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs"
    },
    {
        "lane": "A",
        "name": "v3::generation::tests::delta_sequence_owned_rank_splice_matches_independent_full_writer",
        "target": "bin",
        "modes": [
            "Owned"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs"
    },
    {
        "lane": "A",
        "name": "v3::generation::tests::delta_sequence_plain_failures_do_not_advance_keys_or_root",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs"
    },
    {
        "lane": "A",
        "name": "v3::generation::tests::delta_sequence_owned_failures_do_not_advance_keys_or_root",
        "target": "bin",
        "modes": [
            "Owned"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs"
    },
    {
        "lane": "A",
        "name": "v3::generation::tests::delta_sequence_foreign_budget_and_allocation_failure_preserve_binding",
        "target": "bin",
        "modes": [
            "Owned"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs"
    },
    {
        "lane": "A",
        "name": "s_fact_cache::delta_tests::delta_lane_reads_only_changed_winners_and_replays_only_numeric_suffix",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v2/s_fact_cache.rs"
    },
    {
        "lane": "A",
        "name": "s_fact_cache::delta_tests::delta_lane_same_ohlc_revision_invalidates_real_fact_cache",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v2/s_fact_cache.rs"
    },
    {
        "lane": "A",
        "name": "s_fact_cache::delta_tests::delta_lane_mismatched_old_ordinal_poison_has_no_complete_fallback",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v2/s_fact_cache.rs"
    },
    {
        "lane": "A",
        "name": "theta_v0::parser::inclusion::tests::delta_append_frontiers_match_full_kernel_and_preserve_untouched_prefixes",
        "target": "lib",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/theta_v0/parser/inclusion.rs"
    },
    {
        "lane": "A",
        "name": "theta_v0::parser::inclusion::tests::delta_append_distinguishes_group_confirmation_merge_and_waiting",
        "target": "lib",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/theta_v0/parser/inclusion.rs"
    },
    {
        "lane": "A",
        "name": "theta_v0::parser::inclusion::tests::delta_append_nonincreasing_coordinate_cannot_issue_shared_frontier",
        "target": "lib",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/theta_v0/parser/inclusion.rs"
    },
    {
        "lane": "A",
        "name": "theta_v0::parser::inclusion::tests::delta_rewind_every_cut_restores_metadata_and_matches_full_kernel",
        "target": "lib",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/theta_v0/parser/inclusion.rs"
    },
    {
        "lane": "A",
        "name": "theta_v0::parser::inclusion::tests::delta_rewind_revision_can_change_initial_direction_and_equal_extreme_sources",
        "target": "lib",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/theta_v0/parser/inclusion.rs"
    },
    {
        "lane": "A",
        "name": "theta_v0::parser::inclusion::tests::delta_rewind_releases_removed_evidence_and_rejects_invalid_rank_atomically",
        "target": "lib",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/theta_v0/parser/inclusion.rs"
    },
    {
        "lane": "A",
        "name": "theta_v0::parser::evidence::tests::delta_chunked_pop_preserves_shared_snapshots_at_block_boundaries",
        "target": "lib",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/theta_v0/parser/evidence.rs"
    },
    {
        "lane": "A",
        "name": "theta_v0::parser::evidence::tests::delta_coords_remove_last_preserves_avl_and_old_snapshots",
        "target": "lib",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/theta_v0/parser/evidence.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::nonempty_root_and_unchanged_cut_match_independent_oracle",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::four_cuts_add_replace_remove_observations_keep_multiset_counts",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::missing_and_duplicate_witness_slots_match_frozen_rejection",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::duplicate_object_identity_matches_frozen_rejection",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::source_revision_with_unchanged_prices_invalidates_old_facts",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::mismatched_previous_root_does_not_produce_pending_state",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::owned_contribution_containers_charge_same_ledger_and_reject_foreign_reader",
        "target": "bin",
        "modes": [
            "Owned"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::construction_tests::owner_raw_real_effects_only_promote_after_real_commit",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::construction_tests::owner_raw_rollback_unknown_write_and_unclosed_commit_revoke",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::construction_tests::owner_raw_commit_hook_failure_never_promotes",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::construction_tests::owner_raw_external_commit_rejects_equal_length_material",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::construction_tests::owner_raw_schema_read_domain_and_consumer_failure_revoke",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_effects_node_insert_and_repeated_noop_use_actual_write_counts",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_effects_unhooked_without_rowid_write_and_trigger_side_effect_reject",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_effects_strict_schema_rejects_rowid_node_substitute",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_effects_absent_budget_never_yields_coverage_receipt",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::construction_state::tests::owner_raw_fixture_domains_and_invalidation_are_not_prefix_lengths",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/construction_state.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::construction_state::tests::owner_raw_edits_are_final_identity_winners_and_allow_coordinate_swap",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/construction_state.rs"
    }
]

# #1488 B2 / C1 正控补锁：保留原38锁，另加7个首错锁和4个真实owner锁。
LOCKS.extend([
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::first_occurrence_payload_identity_precedes_later_duplicate_id",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::first_occurrence_group_index_conflict_does_not_mask_intervening_identity_error",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::first_occurrence_witness_payload_precedes_later_duplicate_slot",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::dirty_duplicate_object_rechecks_first_occurrence_before_later_fault",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::dirty_duplicate_witness_rechecks_first_occurrence_before_later_fault",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::legal_object_and_witness_reorder_reuses_contributions_across_cuts",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "B",
        "name": "v3::source_audit::source_delta::tests::legal_group_metadata_replacement_preserves_prefix_groups_and_next_cut",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_delta.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::construction_tests::owner_raw_formal_accept_probe_is_scoped_to_owner_and_thread",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::construction_tests::owner_raw_formal_accept_append_commits_before_nonempty_delta",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::construction_tests::owner_raw_formal_accept_revision_commits_before_nonempty_delta",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs"
    },
    {
        "lane": "C",
        "name": "v3::audit_owner::construction_tests::owner_raw_nonempty_consumer_failure_stales_after_formal_accept_commit",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs"
    }
])


# #1488 A2：新具名锁追加；原49锁与全部运行/退出判定原样保留。
# modes表示构建配置；公共锁使用Plain原件/fixture，不代表完整Owned或Native Runtime。
LOCKS.extend([
    {
        "lane": "A2",
        "name": "s_fact_cache::delta_tests::delta_effective_view_rank_range_and_parser_are_same_bound_without_projection",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v2/s_fact_cache.rs",
        "input_evidence": "Plain input with RawAdvanceFixture; Native build is not Runtime evidence"
    },
    {
        "lane": "A2",
        "name": "s_fact_cache::delta_tests::delta_effective_view_rejects_self_reported_parse_even_with_valid_raw_binding",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v2/s_fact_cache.rs",
        "input_evidence": "Plain input with RawAdvanceFixture; Native build is not Runtime evidence"
    },
    {
        "lane": "A2",
        "name": "s_fact_cache::delta_tests::delta_prepared_rejects_equal_but_foreign_parser_and_untracked_source_epoch",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v2/s_fact_cache.rs",
        "input_evidence": "Plain input with RawAdvanceFixture; Native build is not Runtime evidence"
    },
    {
        "lane": "A2",
        "name": "v3::generation::tests::delta_full_six_roots_revision_swap_insertion_and_metadata_match_full",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs",
        "input_evidence": "Plain input with RawAdvanceFixture; Native build is not Runtime evidence"
    },
    {
        "lane": "A2",
        "name": "v3::generation::tests::delta_full_bi_prior_tail_revision_and_reactivated_windows_match_full",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs",
        "input_evidence": "Plain input with RawAdvanceFixture; Native build is not Runtime evidence"
    },
    {
        "lane": "A2",
        "name": "v3::generation::tests::delta_full_direction_correction_competing_extremes_and_coordinate_removal_match_full",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs",
        "input_evidence": "Plain input with RawAdvanceFixture; Native build is not Runtime evidence"
    },
    {
        "lane": "A2",
        "name": "v3::generation::tests::delta_full_cold_matches_independent_sealed_v2_oracle",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs",
        "input_evidence": "Plain input with RawAdvanceFixture; Native build is not Runtime evidence"
    },
    {
        "lane": "A2",
        "name": "v3::generation::tests::delta_full_foreign_expired_and_consumer_failure_revoke_without_fallback",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs",
        "input_evidence": "Plain input with RawAdvanceFixture; Native build is not Runtime evidence"
    },
    {
        "lane": "A2",
        "name": "v3::generation::tests::delta_full_node_write_failures_revoke_pending_generation_and_all_six_memos",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs",
        "input_evidence": "Plain input with RawAdvanceFixture; Native build is not Runtime evidence"
    },
    {
        "lane": "A2",
        "name": "v3::generation::tests::delta_full_owned_is_explicitly_rejected_and_lease_revoked",
        "target": "bin",
        "modes": [
            "Owned"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs",
        "input_evidence": "real Owned rejection only"
    }
])

# #1488 A2 v2：跨代来源资格与身份首错返修锁，原59锁及执行器保持。
LOCKS.extend([
    {
        "lane": "A2v2",
        "name": "s_fact_cache::delta_tests::delta_owner_rejects_generic_history_on_later_no_edit_generation_without_intermediate_view",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v2/s_fact_cache.rs",
        "input_evidence": "Plain originals and RawAdvanceFixture; cross-generation negative controls and first-error identity proof only"
    },
    {
        "lane": "A2v2",
        "name": "s_fact_cache::delta_tests::delta_owner_rejects_intervening_generic_revision_before_other_winner_append",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v2/s_fact_cache.rs",
        "input_evidence": "Plain originals and RawAdvanceFixture; cross-generation negative controls and first-error identity proof only"
    },
    {
        "lane": "A2v2",
        "name": "v3::generation::tests::delta_full_identity_preflight_preserves_full_first_error_and_zero_write_attempts",
        "target": "bin",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/generation.rs",
        "input_evidence": "Plain originals and RawAdvanceFixture; cross-generation negative controls and first-error identity proof only"
    }
])


# #1488 C2 v2：追加18条Native具名锁；原62锁顺序与全部执行函数保持。
LOCKS.extend([
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::construction_set::tests::owner_set_sequence_exhaustive_occurrences_and_order",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/construction_set.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::construction_set::tests::owner_set_append_skips_shared_prefix_and_preserves_duplicate_occurrence",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/construction_set.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::construction_set::tests::owner_set_full_material_diff_covers_witness_observation_and_nested_requests",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/construction_set.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::construction_set::tests::owner_set_wrong_root_owner_and_unclosed_domain_reject",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/construction_set.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::construction_set::tests::owner_set_diff_budget_failure_does_not_reissue_success",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/construction_set.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::construction_tests::owner_set_real_publish_stage_effects_commit_promotes",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::construction_tests::owner_set_real_publish_commit_failure_revokes_everything",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::construction_tests::owner_set_real_publish_unscoped_same_value_update_rejects",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_set_ordered_node_receipts_keep_repeated_noop_and_unreachable_nodes",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_set_future_stage_cannot_repair_missing_current_original",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_set_stage_drop_foreign_connection_and_omitted_business_row_reject",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_set_duplicate_withdrawal_same_final_row_poisons_ordered_seal",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_set_distinct_withdrawals_each_consume_one_actual_row",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_set_source_cut_rejects_foreign_only_closure_and_cannot_reissue",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_set_none_scope_shares_journal_cap_and_failure_cannot_reset",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_set_exact_work_cap_lives_through_effects_then_releases_only_its_charge",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_set_failed_source_drops_new_cap_without_erasing_existing_codec_retain",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    },
    {
        "lane": "C2v2",
        "name": "v3::audit_owner::effects::construction_coverage_tests::owner_set_actual_transaction_progress_cancellation_is_preserved",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/effects.rs"
    }
])


# #1488 B2：冻结 codec 14 + M/I 5 + 来源组件 5 + 正式坏件 4，均仅 Native。
# 另加既有 BSP 扫描门（三模式）；原80锁/166实例与执行器保持。
# 连同 workflow 既有固定字段37实例：203 + 31 = 234；不代表全图验收。
LOCKS.extend([
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_cross_scope_preserves_only_success_and_charges_current_parent",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_append_delta_skips_successful_sequence_subtrees",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_failed_partial_closure_never_becomes_reusable",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_same_digest_different_body_revokes_existing_success",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_mode_and_json_role_do_not_share_success",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_sequence_and_field_proofs_keep_json_role",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_full_verify_and_value_root_seed_do_not_mint_owner_proof",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_checks_headers_and_revision_even_on_warm_success",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_old_new_and_scratch_share_total_cap_before_growth",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_parent_invalidation_rejects_warm_hit_before_read",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_avl_fixed_delta_keeps_old_addresses_and_logarithmic_paths",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_avl_duplicate_and_one_byte_refusal_preserve_original_tree",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_successful_verify_then_partial_promotion_failure_revokes_all",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-codec",
        "name": "s_content::native_owner_codec_bridge_tests::b2_owner_codec_avl_order_equality_matches_all_context_components",
        "target": "lib",
        "modes": [
            "Native"
        ],
        "file": "rust/src/s_content.rs",
        "input_evidence": "Native feature 下 codec/WorkScope 组件锁；不代表生产 Runtime 准入"
    },
    {
        "lane": "B2-MI",
        "name": "v3::audit_owner::construction_tests::b2_native_warm_cold_and_ordinary_match_all_rows_nodes_roots_receipts_and_counts",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs",
        "input_evidence": "Native 显式候选的真实 owner/事务/两轮流程锁；完整 Runtime 与 Owned 仍未验收"
    },
    {
        "lane": "B2-MI",
        "name": "v3::audit_owner::construction_tests::b2_native_two_nonempty_formal_rounds_match_full_oracle_at_every_commit",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs",
        "input_evidence": "Native 显式候选的真实 owner/事务/两轮流程锁；完整 Runtime 与 Owned 仍未验收"
    },
    {
        "lane": "B2-MI",
        "name": "v3::audit_owner::construction_tests::b2_native_formal_failure_rollback_and_commit_failure_revoke_all_state",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs",
        "input_evidence": "Native 显式候选的真实 owner/事务/两轮流程锁；完整 Runtime 与 Owned 仍未验收"
    },
    {
        "lane": "B2-MI",
        "name": "v3::audit_owner::construction_tests::b2_native_warm_missing_or_stale_originals_reject_before_full_source",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs",
        "input_evidence": "Native 显式候选的真实 owner/事务/两轮流程锁；完整 Runtime 与 Owned 仍未验收"
    },
    {
        "lane": "B2-MI",
        "name": "v3::audit_owner::construction_tests::b2_native_carry_budget_rejects_without_losing_committed_frontier",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner.rs",
        "input_evidence": "Native 显式候选的真实 owner/事务/两轮流程锁；完整 Runtime 与 Owned 仍未验收"
    },
    {
        "lane": "B2-source",
        "name": "v3::source_audit::source_contributions::component_tests::b2_source_retained_tree_uses_actual_string_capacity_and_symmetric_release",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_contributions.rs",
        "input_evidence": "Native feature 下 MemoryNodes 容器计费与首错组件锁；不代表真实事务验收"
    },
    {
        "lane": "B2-source",
        "name": "v3::source_audit::source_contributions::component_tests::b2_source_nested_reverse_index_charges_each_member_and_clears_empty_root",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_contributions.rs",
        "input_evidence": "Native feature 下 MemoryNodes 容器计费与首错组件锁；不代表真实事务验收"
    },
    {
        "lane": "B2-source",
        "name": "v3::source_audit::source_contributions::component_tests::b2_source_duplicate_key_is_reported_at_second_current_occurrence",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_contributions.rs",
        "input_evidence": "Native feature 下 MemoryNodes 容器计费与首错组件锁；不代表真实事务验收"
    },
    {
        "lane": "B2-source",
        "name": "v3::source_audit::source_contributions::source_order::tests::b2_order_avl_stable_handles_survive_prefix_middle_and_reverse_edits",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_order.rs",
        "input_evidence": "Native feature 下 MemoryNodes 顺序树组件锁；不代表真实事务验收"
    },
    {
        "lane": "B2-source",
        "name": "v3::source_audit::source_contributions::source_order::tests::b2_order_rejects_wrong_original_and_stale_handle_before_reuse",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/source_order.rs",
        "input_evidence": "Native feature 下 MemoryNodes 顺序树组件锁；不代表真实事务验收"
    },
    {
        "lane": "B2-source-fault",
        "name": "v3::audit_owner::construction_tests::b2_native_source_phase_errors_precede_later_bad_material",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/b2_source_fault_tests.rs",
        "input_evidence": "include! 注入 construction_tests 的真实来源坏件/首错/回滚锁；12场景并非94分支穷举"
    },
    {
        "lane": "B2-source-fault",
        "name": "v3::audit_owner::construction_tests::b2_native_source_global_dependencies_match_full_oracle_and_rollback",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/b2_source_fault_tests.rs",
        "input_evidence": "include! 注入 construction_tests 的真实来源坏件/首错/回滚锁；12场景并非94分支穷举"
    },
    {
        "lane": "B2-source-fault",
        "name": "v3::audit_owner::construction_tests::b2_native_source_future_duplicates_preserve_earlier_actual_fault",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/b2_source_fault_tests.rs",
        "input_evidence": "include! 注入 construction_tests 的真实来源坏件/首错/回滚锁；12场景并非94分支穷举"
    },
    {
        "lane": "B2-source-fault",
        "name": "v3::audit_owner::construction_tests::b2_native_source_unsettled_rechecks_added_nonzero_group_before_partition",
        "target": "bin",
        "modes": [
            "Native"
        ],
        "file": "rust/src/bin/s_session_v3/audit_owner/b2_source_fault_tests.rs",
        "input_evidence": "include! 注入 construction_tests 的真实来源坏件/首错/回滚锁；12场景并非94分支穷举"
    },
    {
        "lane": "B2-BSP",
        "name": "theta_v0::classifier::bsp::tests::no_stray_bsp_binding_rewrites_outside_registry",
        "target": "lib",
        "modes": [
            "Plain",
            "Owned",
            "Native"
        ],
        "file": "rust/src/theta_v0/classifier/bsp.rs",
        "input_evidence": "既有 BSP 绑定扫描门；保留原扫描与 stray/stale 双断言，三模式逐名执行"
    }
])

def required_locks(mode: str, target: str) -> list[dict]:
    """清单来自冻结源码包；不按测试输出反推预期，防止零测试假绿。"""
    return [lock for lock in LOCKS if mode in lock["modes"] and lock["target"] == target]


def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def checkout_identity(repo: Path) -> dict:
    """实际构建源码必须属于该提交；运行前后都核验，禁止脏树冒充HEAD。"""
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip()
    tree = subprocess.check_output(["git", "rev-parse", "HEAD^{tree}"], cwd=repo, text=True).strip()
    subprocess.check_call(["git", "diff", "--quiet", "HEAD", "--"], cwd=repo)
    return {
        "head": head,
        "tree": tree,
        "source_sha256": {name: sha256(repo / name) for name in sorted({x["file"] for x in LOCKS})},
        "runner_sha256": sha256(repo / ".github/scripts/run-1488-delta-ci.py"),
        "tracked_worktree_clean": True,
    }


def select_binary(log: str, rust: Path, target_dir: Path, target: str, mode: str) -> dict:
    """只接受本次成功 cargo JSON 指出的唯一当前包测试二进制。"""
    messages = []
    for line in log.splitlines():
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            messages.append(value)
    finished = [m.get("success") for m in messages if m.get("reason") == "build-finished"]
    if finished != [True]:
        raise ValueError(f"cargo 未唯一声明成功完成：{finished}")
    artifacts = [m for m in messages if m.get("reason") == "compiler-artifact"
                 and m.get("profile", {}).get("test") and m.get("executable")]
    if len(artifacts) != 1:
        raise ValueError(f"预期唯一测试 executable，实际 {len(artifacts)}")
    artifact = artifacts[0]
    meta = artifact.get("target", {})
    expected_name = "newchan_rust" if target == "lib" else "s_structure_session"
    expected_source = rust / ("src/lib.rs" if target == "lib" else "src/bin/s_structure_session.rs")
    kinds = set(meta.get("kind", []))
    correct_kind = bool(kinds & {"lib", "rlib", "cdylib"}) if target == "lib" else kinds == {"bin"}
    if (meta.get("name") != expected_name or not correct_kind
            or Path(meta.get("src_path", "")).resolve() != expected_source.resolve()
            or Path(artifact.get("manifest_path", "")).resolve() != (rust / "Cargo.toml").resolve()):
        raise ValueError("cargo executable 的 target/source/manifest 不属当前目标")
    check_feature_closure(mode, artifact.get("features", []))
    binary = Path(artifact["executable"]).resolve()
    if not binary.is_relative_to(target_dir.resolve()) or not binary.is_file() or not os.access(binary, os.X_OK):
        raise ValueError("测试 executable 不存在、不可执行或不属本 job target")
    return {"path": str(binary), "sha256": sha256(binary),
            "features": artifact["features"], "fresh": artifact.get("fresh"),
            "target": meta, "manifest_path": artifact["manifest_path"]}


def require_listed(log: str, names: list[str]) -> None:
    listed = Counter(line.removesuffix(": test") for line in log.splitlines() if line.endswith(": test"))
    missing = [name for name in names if listed[name] != 1]
    if missing:
        raise ValueError(f"当前二进制未逐名唯一列出必需测试：{missing}")


def require_passed(log: str, name: str) -> None:
    if log.splitlines().count(f"test {name} ... ok") != 1:
        raise ValueError(f"测试未逐名唯一成功：{name}")
    summaries = re.findall(
        r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured;.*$",
        log, re.MULTILINE,
    )
    if summaries != [("1", "0", "0", "0")]:
        raise ValueError(f"exact运行必须恰1 passed且无failed/ignored/measured：{summaries}")


def same_binary(binary: dict) -> None:
    if sha256(Path(binary["path"])) != binary["sha256"]:
        raise ValueError("列举/执行之间测试二进制已改变")


def run_logged(argv: list[str], cwd: Path, log_path: Path, receipts: list[dict]) -> int:
    """保留原命令退出码；不通过shell，不能让tee或日志成功掩盖命令失败。"""
    started = time.monotonic()
    record = {"argv": argv, "cwd": str(cwd), "log": str(log_path), "exit_code": None}
    receipts.append(record)
    print(json.dumps({"command": argv, "log": str(log_path)}, ensure_ascii=False), flush=True)
    with log_path.open("w", encoding="utf-8") as log:
        try:
            with subprocess.Popen(argv, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                                  text=True, encoding="utf-8", errors="replace") as child:
                assert child.stdout is not None
                for line in child.stdout:
                    log.write(line)
                    print(line, end="", flush=True)
                code = child.wait()
        except OSError as exc:
            record["error"] = str(exc)
            code = 127
        finally:
            record["elapsed_seconds"] = time.monotonic() - started
    record["exit_code"] = code
    record["log_sha256"] = sha256(log_path)
    print(json.dumps({"exit_code": code, "log": str(log_path)}, ensure_ascii=False), flush=True)
    return code


def run_target(rust: Path, target_dir: Path, output: Path, target: str, mode: str,
               report: dict) -> bool:
    locks = required_locks(mode, target)
    result = {"required": [lock["name"] for lock in locks], "commands": [],
              "tests": [], "success": False}
    report["targets"][target] = result
    try:
        args = ["--lib"] if target == "lib" else ["--bin", "s_structure_session"]
        build_log = output / f"{target}-build.log"
        command = ["cargo", "test", "--offline", "--locked", "-j", "6", "--features",
                   FEATURES[mode], "--no-run", "--message-format=json", *args]
        if run_logged(command, rust, build_log, result["commands"]) != 0:
            raise ValueError("cargo编译失败；不得寻找或执行旧二进制")
        binary = select_binary(build_log.read_text(), rust, target_dir, target, mode)
        result["binary"] = binary
        names = result["required"]
        if not names:
            raise ValueError("此目标预期锁为空，拒绝空运行")
        same_binary(binary)
        list_log = output / f"{target}-list.log"
        if run_logged([binary["path"], "--list", "--format", "pretty"], rust, list_log,
                      result["commands"]) != 0:
            raise ValueError("测试二进制列举失败")
        require_listed(list_log.read_text(), names)
        same_binary(binary)
        failures = []
        for number, lock in enumerate(locks):
            name = lock["name"]
            test_result = {"name": name, "lane": lock["lane"], "success": False}
            result["tests"].append(test_result)
            same_binary(binary)
            test_log = output / f"{target}-{number:02d}.log"
            code = run_logged([binary["path"], name, "--exact", "--format", "pretty",
                               "--color", "never", "--show-output"], rust, test_log, result["commands"])
            test_result.update({"exit_code": code, "log": str(test_log)})
            same_binary(binary)
            try:
                if code != 0:
                    raise ValueError(f"测试命令退出码 {code}")
                require_passed(test_log.read_text(), name)
                test_result["success"] = True
            except ValueError as exc:
                test_result["error"] = str(exc)
                failures.append(name)
        same_binary(binary)
        result["binary_hash_after_run"] = sha256(Path(binary["path"]))
        if failures:
            raise ValueError(f"具名锁失败：{failures}")
        result["success"] = True
    except (ValueError, OSError) as exc:
        result["error"] = str(exc)
        print(f"::error::{target}: {exc}", flush=True)
    return result["success"]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", required=True, choices=FEATURES)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--print-plan", action="store_true", help="只打印具名清单，不执行Rust")
    args = parser.parse_args(argv)
    plan = {"mode": args.mode, "features": FEATURES[args.mode],
            "locks": [lock for lock in LOCKS if args.mode in lock["modes"]]}
    if args.print_plan:
        print(json.dumps(plan, ensure_ascii=False, indent=2))
        return 0
    if os.environ.get("GITHUB_ACTIONS") != "true":
        parser.error("实际Rust执行仅限已授权GitHub CI；本机只允许--print-plan")
    if args.output is None or not args.output.is_absolute():
        parser.error("--output必须是runner上的绝对路径")
    if os.environ.get("FEATURES") != FEATURES[args.mode]:
        parser.error("FEATURES与所选模式不一致")
    raw_target = os.environ.get("CARGO_TARGET_DIR")
    if not raw_target or not Path(raw_target).is_absolute():
        parser.error("CARGO_TARGET_DIR必须沿现job设置为绝对路径")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    repo = Path(__file__).resolve().parents[2]
    rust = repo / "rust"
    report = {"schema": "1488-abc-named-locks/1", **plan, "targets": {}, "success": False}
    try:
        before = checkout_identity(repo)
        report.update(before)
        if report["head"] != os.environ.get("GITHUB_SHA"):
            raise ValueError("当前HEAD与GitHub本次run SHA不符")
        report["run_id"] = os.environ.get("GITHUB_RUN_ID")
        report["run_attempt"] = os.environ.get("GITHUB_RUN_ATTEMPT")
        report["target_dir"] = raw_target
        # 保留现有-j6/工作目录/feature/预算/超时；两个目标顺序构建避免额外内存峰值。
        passed = [run_target(rust, Path(raw_target), output, target, args.mode, report)
                  for target in ("lib", "bin")]
        report["checkout_after"] = checkout_identity(repo)
        if report["checkout_after"] != before:
            raise ValueError("构建/测试期间HEAD、tree或源码发生漂移")
        report["success"] = all(passed)
    except (ValueError, OSError, subprocess.CalledProcessError) as exc:
        report["error"] = str(exc)
        print(f"::error::{exc}", flush=True)
    finally:
        (output / "RESULT.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return 0 if report["success"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
