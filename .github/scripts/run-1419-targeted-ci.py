#!/usr/bin/env python3
"""#1419 三条版本门及 codec 合同：仅精确远端候选，失败不洗绿。"""
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import signal
import stat
import subprocess
import sys
import time

REVIEWED_SOURCE = "187488552a6d4158be5bb44139dc2b36baaeccd4"
REVIEWED_TREE = "d3f4f29864ecb4e423c59991ef3b9b9942c8c9ab"
CONTRACT_READY = False  # 两 CI 文件独审、源码绑定和尾差复核后由主控显式开启。
CI_FILES = {".github/scripts/run-1419-targeted-ci.py", ".github/workflows/ci.yml"}
LEGACY_RUNNER = ".github/scripts/run-1488-delta-ci.py"
LEGACY_SHA256 = "2c94e9e8354bd3fe369067210054db903df019cf9ce8c1345a6ec6d107b297dc"
FEATURES = "s_session,s_capture_native_testonly"
DISK_MIN_BYTES = 2147483648
TESTS = ['v3::source_audit::tests::structure_rule_rejects_new_kinds_under_legacy_rules', 'v3::source_audit::tests::structure_rule_accepts_declared_versions_and_checks_payload', 'v3::content_structure::tests::codec_bridge_preserves_json_types_and_number_domain']

CANCELLATION = {"count": 0, "first_signal": None}
OWNED_GROUPS: dict[int, subprocess.Popen] = {}
FINAL_STATE = {"sealed": False, "artifacts": None}
FINAL_EXCLUSIONS = {
    "FILES.json", "FILES.SEAL.json", "RESULT.json", "LATE.json", "CANCELLED.json",
    "CANCELLED-AFTER-SEAL.json", "FINALIZATION-REJECTED.json",
    "control/inventory.stdout", "control/inventory.stderr", "control/inventory.json",
}

class Cancelled(RuntimeError):
    pass


SOURCE_FILES = {'rust/.chanlun/.stop-guard-counter': 'fa186b3f41f78c3c3ea46b78beedee11e22bf75b9ba6dd8bdda19b74fd238e12', 'rust/.chanlun/review-results/on2-truehotspot-20260630.md': 'a4a49ba598fd2fc8bef813796705d2a03ec83ee50c448b5ae63f2ced74897c3d', 'rust/Cargo.lock': 'af8449bbc52d59d7ed9fbdb62c27150bcfa00f7ff061e16d28b3b7bd017c2a3b', 'rust/Cargo.toml': 'f991a4bcb9e1238eae93357fbe4c960ae482aaf559b58457b374dc9b224e2cbb', 'rust/pyproject.toml': '3ac63162aa050d2fca920cb7ac88b310a9c99667c81ae62f3da086ec8744e67d', 'rust/src/bi_engine.rs': '31ce870209a9ae803966c4032a4179eb2382188c613ea1b74ff636c78656e931', 'rust/src/bi_zhongshu_bsp.rs': 'b5d1484cf8a5eb04632ff618704ba9eff1b55576306ad43c060af5fa9de93c84', 'rust/src/bin/b_chong_session.rs': '34adfbb8456b5e81ea35c81b6cb11ebc83fe85f4aa571ea669115adf3ff56af1', 'rust/src/bin/cp_capability_smoke.rs': 'c733bb16a95a90ea319c019c88bc84cb9e45afcc285ead3dfb3aa114f1601c3c', 'rust/src/bin/e_economic_session.rs': '866dbe66d598648b5c324ad77eaa825c2a856caa37a19b41d4145f7f251e7005', 'rust/src/bin/issue550_event_battery.rs': 'a90c1dabc7bdb4fa60e3274e31d55df77c51820b34d0da1a0a7ec27eb63f88de', 'rust/src/bin/lav_seal_samples.rs': '3bf72686fbdc1144456d267510ee40bac35ef90857cfdd60a849a9d8efda6ac1', 'rust/src/bin/p100_cert_bsp_recon.rs': '5c7e6d61f86b9b2c941c6f3d363d0d0e28c408d24f753f168d4816bd14124944', 'rust/src/bin/p101_cert_bsp_tag.rs': '7534ff2d708bf151f07b53f7e88eb5ac20c2f7ccca50e6d70eeed8c3db251dbd', 'rust/src/bin/p102_b_long_zero_probe.rs': '3ee310ff25728354a5fada00275065cffc7cb3c4e18d9104dfa22c4c420c68b1', 'rust/src/bin/p1035_type1_lead.rs': 'd648463826f37f55680eeb8e36b45fd04b01897ed17cb06e1605a37995f59e25', 'rust/src/bin/p104_trend_div_funnel.rs': '09be7d8ad325a6786591da7bb9ca0c76dafb0e21b26a510f7ec60c7cf0481213', 'rust/src/bin/p105_cert_level_spectrum.rs': '8479788327c10fa85f3e896d89ededbb4e8497b3c222acab7ba6cd64efccf0a7', 'rust/src/bin/p106_cert_forward_pnl.rs': '36d5c4e56fbf9767a974a4e220b76e46eb4e34cb386d1f9c9bc0629b10079769', 'rust/src/bin/p107_level_calib.rs': '20d89ce81bb71275f11a7418588016c3fbee89a8cef69f4c9c40ef52aebaf401', 'rust/src/bin/p107_level_funnel_audit.rs': '620452578e746a47efa4e59b4d98e3781cba7ba233e7569db7020d8ef601c956', 'rust/src/bin/p108_dparent_caliber_counterfactual.rs': 'a7d729bdc7ba291ed7f91e9432e8fc0bc99306912c76b278c79924035c957e87', 'rust/src/bin/p108_interval_probe.rs': 'f10136fae6626235677782fa8cf4c41405ed69f00582373fc1da9f937f219330', 'rust/src/bin/p109_chain157_gate_attrition.rs': '5d76e678b28dbafb8e3bf1328ff104d60612f84b3359de4ce0c85aeff08f65c3', 'rust/src/bin/p111_parent_divergence_validity.rs': '3ab60e3881a30c60679289a31d584429ab26b583b0f8a109b9e1962e4530bcae', 'rust/src/bin/p112_trend_predicate_caseaudit.rs': 'd5ec5f043cc16367bbd8ddf2c870df17c7a906b558c86dbfd802751d73d033f8', 'rust/src/bin/p113_pan_caseaudit.rs': '8cc79dea5c8d5668e6c5a25da9f867e48e248d52c386fd984a4c15ee20914615', 'rust/src/bin/p116_turnpoint_anchor_existence.rs': '5f040bb3af673f3a3f94381c8a7531f69951645ebbff70f91e8977e97025626b', 'rust/src/bin/p117_h4_attribution.rs': '7d3f0a45349fa02fe2d5f72b539a7093ab52d30cf42debe3a45135c1fe8e4d12', 'rust/src/bin/p117_s1a_level_recheck.rs': 'af9177056bea71f8c2c6fd42c4fd323788e8cd591751c1bdd6f6202328c8749c', 'rust/src/bin/p119_l5_decompose.rs': 'aa7a61434894bf5a297a04b6ed9f033c57fbbcff340f98ca2687dbb0c5d63708', 'rust/src/bin/p122_replay_profile.rs': '55b1cdf15cc69dc2b05d56234ed3e26efff50c9cd7775e7f285ba7338cced7cb', 'rust/src/bin/p123_fast_replay.rs': '5acfcf39a5a90e6f4894f463598e224c3c45bd962716521e50843893c7c96eb1', 'rust/src/bin/p124_merge.rs': 'f2b381dfe4526f7db899e77ad990aaff2704e2da2f1fd320989caa237708f418', 'rust/src/bin/p124_shard_replay.rs': '36f96ec69f89804e364c2e849bd070502400c7252960d1cf147c9be99a1c09de', 'rust/src/bin/p125_endorsement_audit.rs': '2a775a52d46ce206011d21eb29a858d7aaea57351dfe08b87a8f0ce926a6ad99', 'rust/src/bin/p126_d3_descending_clock.rs': 'cd831a4a086557cbd02b95fcc38c157e667f9b39436c66aa5f443f119f9cc4b6', 'rust/src/bin/p127_skip_impact.rs': 'f013fcbea71f7d4c6d856060dabdef49c473b6931134a9519678b28db988f92e', 'rust/src/bin/p129_44ke_strict_skip.rs': '2d4bbc2c16f74c1612f8b33eb8c1a669fdda6d0c1a6ae98bf355b57247e642f2', 'rust/src/bin/p409_pan_live_probe.rs': 'a62357afd48541200331dc87f36d3e17f4d9d3128a1b2aec1cfba11c2cb7ef07', 'rust/src/bin/p76_case2_replay.rs': '0656ea1fd46c0ddaef3744c2daf5d560e225d1d43d68583376e54591f6e8cc45', 'rust/src/bin/p83_yield_remeasure.rs': 'f7abcaf59da89119e4d6cdb0d01d4b1476fd0bec9cc78938c0c974b83b3925c0', 'rust/src/bin/p84_failclosed_audit.rs': '7d5553c1f61376ee2321ae7961f3b6c61dfd401d1dd76ff85ba71bab5e3c11fa', 'rust/src/bin/p856_ed_dominance.rs': 'a622662721ac7a99a63f81f17503a5d51a5b23641e2f76b948dd7b7c1fb117f1', 'rust/src/bin/p86_anchor_hypothesis.rs': '7a76767f3abdaa17709627223a6696d4155fe387662de079f64f211093eb1249', 'rust/src/bin/p89_dual_core_audit.rs': 'f6d7729ea6e749686c824e6afdc096390a0383c486431276051a58503772d95d', 'rust/src/bin/p907_cost_gate_level.rs': 'cc01c16e60d85500bdfc2a2760b5069cbd56b09b4c92f073c70acc8c3ab716f4', 'rust/src/bin/p915_capacity_frequency.rs': 'e860b48647fbdc22a5bd9a2e263004e668f93ed3a2320578d6d53d2420128413', 'rust/src/bin/p92_nest_replay_postruling.rs': 'f9d5bd984bb3dc5546409939194018be51649db6f079f96404c778b12c19d409', 'rust/src/bin/p938_bound_center_openness.rs': 'ae99206fe19f3c320bf190e1e3b74e7c4397fca24db6c9ca041e897993954d30', 'rust/src/bin/p938_seed_moved_composition.rs': 'fdc62152f0c1567263294353e36f84ee8fb150d8bf49c96974da48d4376a925b', 'rust/src/bin/p93_invalidseed_probe.rs': '4b02f8157594b605dad822052b8feb51800985b18ef054df65b100e960bb982a', 'rust/src/bin/p940_struct_input_abc.rs': '27b216ce9f1590c63ad0eab4173b75acb60d86eddeff48d642b6b139dae5f1cf', 'rust/src/bin/p944_bsp_layer_mismatch.rs': 'b7e64c4016ede641b2d4138d05a4986c679ae42e57716c4016c006b68c98ca60', 'rust/src/bin/p946_seg_structure_alignment.rs': '7d63dc20652fd60767f87aa32621186c84775507c80261448b7ed368e610b508', 'rust/src/bin/p95_carriedonly_ab.rs': '5740bd458bf1d0d98f05710e9cd7b3afc1daf505a741af37bf4b6a8fcc2027ef', 'rust/src/bin/p977_force_def_validation.rs': '2d3217d6201016572478bcf52b254ed24897a68a3e077e17e91c1482bb82f84a', 'rust/src/bin/p985_s8_consume_baseline.rs': 'f1d1aa9c2c481180d2c3b2076909c12b7d9869b65d65ad873b0c3f6e2a64cd90', 'rust/src/bin/p985_tv_endpoint_blunt.rs': '339bb2a459ff20216a990a5768ba504cb5e70e610fccf03fbd88f56154e390c9', 'rust/src/bin/p992_gauge_diff.rs': '91f81585fa4b86737927ce9d8799c8934e2b1babe1677419f2a354ef41421e30', 'rust/src/bin/p_issue668_bsp_bridge_battery.rs': 'ac8caf644e210782626a936d5bb5a4d2419073ad2a37028f52d64a4c277c3723', 'rust/src/bin/p_issue668_bsp_key_truth.rs': '355307c7f08b8196f51d275fffb9e259d68bdc83b0a9410bcd8404257a902616', 'rust/src/bin/pi_bsp_timing.rs': 'ed7a950b5f60da4bf0f0a079812917ff985074d3e646b10fe7e45227380015be', 'rust/src/bin/pi_bsp_timing/cli_args_tests.rs': '0b8335db0875fe045634c3399d7cb542611c3799667e00dc99c619e9d1901401', 'rust/src/bin/pure_bsp_timing.rs': 'ec932d7b5d08202008f13bf145449113f91018f0c9814674777e0f5f2712f843', 'rust/src/bin/s2_lean_mirror_extract.rs': 'b82450a2d401bd5697756d23a029a91ebd74616ff8855111d46459e94cc63e7d', 'rust/src/bin/s_session_v2/audit.rs': '3f9b030dc7bcf5945f20ec3f52f82dd6b34f5497653d0e33e7681b1d396f9752', 'rust/src/bin/s_session_v2/cache.rs': 'e39a86d3d41079cd34aec4c7f1c02a84f2a9a8a32b3f1ee5e9478db8f7a7c998', 'rust/src/bin/s_session_v2/content_bi.rs': 'b3ed67cce4f3c5a315ed69f76a03b441cc6dcab317abd4b16b1023b8a8855335', 'rust/src/bin/s_session_v2/content_manifest.rs': '75473618d47f4560c005e164da492b160bc6bb792ff8ef87c61ec591818857f1', 'rust/src/bin/s_session_v2/content_producer.rs': '57c02cf2c4a8810e4031048533826f5174f80c88af97fe6154718bfd4ee0eacd', 'rust/src/bin/s_session_v2/content_publication.rs': 'e8fe94d6d9d7f8f78fe378abb5e80dab643539d0be01f4a79c8aa2fa200a88fd', 'rust/src/bin/s_session_v2/content_store.rs': '71ff328b4041ec0de63887b48961ca05c9b37a81469b47b6f4d13c828a6b8d95', 'rust/src/bin/s_session_v2/delta_storage.rs': '6a5c7ea1518aff8d35abc51a3ffe0424390e27cc580eaa63f82260aad7966fc9', 'rust/src/bin/s_session_v2/evidence_store.rs': '8e24d03822670015445a51d371a68ee9a066ee8c5ebf18fcb9d030eff0a3d3f6', 'rust/src/bin/s_session_v2/mod.rs': '9f7264accc4f65c4fa435990f8afa278ccde69be6422b3c9e02b3379d6a5e1a6', 'rust/src/bin/s_session_v2/s_fact_cache.rs': '8676d216c88379e30eb8681dd62b5eb7fb5ad6f3d84d33892a44f4a3cba765da', 'rust/src/bin/s_session_v2/staged_nodes.rs': 'bc60cd67f22aa6acda6690b46080a5a8c9b13b2ed131ba3728cf9a9d7da70f05', 'rust/src/bin/s_session_v2/tb02a.rs': '4ca3c7fccbc278caa140cf31629c1c193f4aa5645557651d3532a0d18d3d424d', 'rust/src/bin/s_session_v2/tb02a_facts.rs': 'f33aa9e513191f689b18539b06faf97b159484bbe9f8d88aefef5f8a92d4c5b4', 'rust/src/bin/s_session_v2/tb02b.rs': '4d4db9df229bb5768ba5890e445cae272977694349c37128fec4fe32bb81732f', 'rust/src/bin/s_session_v2/tb02c.rs': '3742db857c20e2d46be21893eeaf2770e8b2c315bc26f41e18ce4194a3eeebe4', 'rust/src/bin/s_session_v2/tb02e.rs': '9701f93d114eb62a8321cb558b6117a50b6653a0c81d7d7666fe5aedef369b60', 'rust/src/bin/s_session_v2/tb02f.rs': '417be8f52a8b6613deb2edfe181b6ecc495e21920969cac6794ac1c17c3fcee0', 'rust/src/bin/s_session_v3/audit_budget.rs': '62fb2c5d0b8e93417875764b3bbe33058dae5524186076c38aa13fd19e3bb837', 'rust/src/bin/s_session_v3/audit_header.rs': 'fc3939e7abc74c43be628f492eeb0fdf15475a528e9aa15b0dffc403ebeb31da', 'rust/src/bin/s_session_v3/audit_owner.rs': 'ca3447894800ac210d4ef8de9744bc67ddb921d3c6784c43cc1221978e633117', 'rust/src/bin/s_session_v3/audit_owner/allocation_probe.rs': '59191296ec6b6f7b03bd130d14a0f4d7995b7edaaec7eb144118d59c3775dd17', 'rust/src/bin/s_session_v3/audit_owner/b2_source_fault_tests.rs': '104e697bb5681978d52a5b4de3606d2474b844d922cdfd65927985e29ee76758', 'rust/src/bin/s_session_v3/audit_owner/effects.rs': '6094df399b27a1982ea781795aacef102736dc2b46c9cd0bd6f0b10ea1afde12', 'rust/src/bin/s_session_v3/audit_owner/native.rs': '93dead162e797328e46b1c039592ca204768f1655e7f10293354c04d739b01bf', 'rust/src/bin/s_session_v3/audit_owner/q.rs': '2e4453b4e47059f78b862d23c236c3ecb0673180c854a58934545da458d84548', 'rust/src/bin/s_session_v3/audit_retention.rs': 'aaf0898c25d2d4cb8f7403820346937152e23365517c78964fe1317718f45fb3', 'rust/src/bin/s_session_v3/audit_store.rs': 'c9caabc103a2fa432fb05a1358270a7c854db9a5b3a1dd02f5059580c7465661', 'rust/src/bin/s_session_v3/aux_index.rs': '61d2813121d8299a493c1b4a2b04ffbfc63554c0fb23051e01c409707d8d4687', 'rust/src/bin/s_session_v3/business_json.rs': '3eeabc2908418021890042488ae8a3fa7aa5064dc5320804da68bd5ccc8fa7da', 'rust/src/bin/s_session_v3/capture.rs': 'ff9cbb2646747426d741c149d8a5d55ca6e8d99c760aee7c753213ce24c450b7', 'rust/src/bin/s_session_v3/catalog_evidence.rs': 'c200ce86557b8d3c54a623a70a5dea5a8f951a02125474a529e0941f81f04023', 'rust/src/bin/s_session_v3/commit.rs': '4167a0a3c6897295a3198eca31a74a2b718df397636b2457e24d349d47f6e02a', 'rust/src/bin/s_session_v3/construction_set.rs': '7d8e1af1172b700d36710f981c873f1f740666a3e9a1ce8cd2cf6a6e9ee8cb97', 'rust/src/bin/s_session_v3/construction_state.rs': '60ec6efe71ce0c54643385626d09b79669ea82b406537c4988a1bd87bdbe4960', 'rust/src/bin/s_session_v3/content_structure.rs': '3e7803272c1503ef9579ab812a81e7b69f5e26a9c4896bef71fd06520d50ca65', 'rust/src/bin/s_session_v3/control.rs': 'bbf1141c19763c61fcdd9a3c3e97d4a22fe6cc8f2bbf095424bd07f215212d83', 'rust/src/bin/s_session_v3/control_audit.rs': '8ae5ea7d4a364030744415686ccebf88b002a280c3ec26a28569d200ad2d085c', 'rust/src/bin/s_session_v3/cross_request_source_cache.rs': 'ef82e6b2d4a47c58e0072795fb70da17d3a383531ac229659860b3c6f0ea3e4c', 'rust/src/bin/s_session_v3/driver.rs': '79e70d88877f5bc0238c38f4bb2565bb9692417446d2b05479451e1c2d3377b1', 'rust/src/bin/s_session_v3/financial_reader.rs': '0b7a010162341e1d72a978bd9aed8095bdbf6dc804fa7c46eae2f52a2fc328f2', 'rust/src/bin/s_session_v3/financial_session.rs': 'ded7f8c61a21910586d5cba9bfa81afc6378723055bd6b2f3ab0c77de1eaa0a0', 'rust/src/bin/s_session_v3/financial_session_tests.rs': 'a025c1dfacc3b56310b90dcdd3bd439f2029f015819e8695f19d915e6f789ec9', 'rust/src/bin/s_session_v3/generation.rs': '471825f38f5a41796346b286386d64f5ad25e06bfbd01ac51ea77248bdda4c0f', 'rust/src/bin/s_session_v3/history.rs': '88af90c50dcc303fbfb02f47db21bae1006fd30899599dee2f5bf9625a5e1291', 'rust/src/bin/s_session_v3/mod.rs': '7c7483132ae093cbf14c28fa50debcccafbbd25d3e6bd0d5cab7789df5bc0022', 'rust/src/bin/s_session_v3/native_record_fields.rs': '45be1dcf7fb223448b8b2c9232a6ac7fc08277c7b27646141b061a0e3b3ba47d', 'rust/src/bin/s_session_v3/object_diff.rs': '96e38ef37ba740ec6c8db829f20418a60b941840f9cbc7360048ffb12ee1da8c', 'rust/src/bin/s_session_v3/object_index.rs': '6dc03d398393c8f2705aae5815ad82fd2ae0c2fbe94f59a05ec811bbd36a2f87', 'rust/src/bin/s_session_v3/object_records.rs': '01cb55955db4d35721ad043056cbc5eee4b351f9c7ffd5c0a2631db2a574dae5', 'rust/src/bin/s_session_v3/ordinary_materials.rs': '06ef7960692af0543e6222c5f393f45daf2b0c09ee8ac69ac6aa51f7b5a7c745', 'rust/src/bin/s_session_v3/ordinary_materials_tests.rs': 'f80fd18de147d4c7f99eaf32e0fb0f81a18e77f39e1019d733e52b71aa0b1f6d', 'rust/src/bin/s_session_v3/pending_source_tests.rs': '63d00f9da8b1f6bab25804f507ef4b74d5de78f10a3bd6213cb7b08cd54cd1a2', 'rust/src/bin/s_session_v3/prepared.rs': 'dc1cdab4f60bd804ebe679e006ff790a9ca253f0d4a29ba97b9f4ff73893b97a', 'rust/src/bin/s_session_v3/projection.rs': '58059cc9a54fd9f6c06cf8f29aacbee6e8bc13945eca2029716fe8bcb295d48c', 'rust/src/bin/s_session_v3/q_allocation.rs': '93b7a79d948fc0fb768f7bfb224ba5b9ac23831371b82bebe62281928d1c494d', 'rust/src/bin/s_session_v3/q_content.rs': '6012e58b647f3f826641c6cb5352ce62343ebdc6df8b7123c22ef9a69e4f19fb', 'rust/src/bin/s_session_v3/q_delta.rs': '202349bec3ac799e4db5f02d3eb488817f07edad86f6306a888d1168845abbc9', 'rust/src/bin/s_session_v3/q_executor.rs': '7cd63fe8acbb112d33131258ec53554d0a35c0ae6e0f1a059cbfc77b6327f805', 'rust/src/bin/s_session_v3/q_http_input.rs': 'fed1f97b7a17a129263939837ef0039ab9634bb8727ef4a7f111c95542a32a24', 'rust/src/bin/s_session_v3/q_lease.rs': 'f159b78e4838d6d68b0662582b6bc712f0c71ed4aeba31bc9418639a7991dcf7', 'rust/src/bin/s_session_v3/q_listener.rs': '6e18b3e8f7d48d546dabdd9038aadbcbbaedd5713b84491aa1d2547fb37ac446', 'rust/src/bin/s_session_v3/q_resources.rs': 'b072c225319b5e0b8847f2a5eb51d0fbf2e3e0d8287123f21e2e557d0a9f8de1', 'rust/src/bin/s_session_v3/q_retained.rs': '67bd29c27d0f436038188ca4ee527bb622b6eb9e2ca9cd0e9a90acad44c71d6d', 'rust/src/bin/s_session_v3/q_runtime.rs': 'e316142febbbafe24791aa84e729d1d189443b67b2af015dfafc86cd836b9bdd', 'rust/src/bin/s_session_v3/q_serve.rs': 'e96208f3274ac4bf8e146ac3757a5e22b165de1de46ccf08ffce4488e43faf25', 'rust/src/bin/s_session_v3/q_snapshot.rs': '6f6632673208897b21046989c8e896081468c293866a8db0fce13da2f5cdd1a9', 'rust/src/bin/s_session_v3/q_watch.rs': '2653b1603d5ceed016aec3b5511440881637e8e00de1859faef25e9b4e191a92', 'rust/src/bin/s_session_v3/q_wire.rs': 'f4b2afc858001a5b8a62da73fced7d3283640b2834dd4995aaeef39a15ffb6f1', 'rust/src/bin/s_session_v3/raw_input.rs': '3e02b40deb3341085096238d55ce2f7d9e11d6af9fadf8de8dfde83624cebe45', 'rust/src/bin/s_session_v3/reader.rs': '74e387f0f8f5c2d0b921422fb012663a5a66a912cf46934cbcb1b9a615b76fa6', 'rust/src/bin/s_session_v3/request_audit.rs': '55f85676ffb087042874cc50d3b11405a603953f931f9bf12827ebf26c6f17cd', 'rust/src/bin/s_session_v3/s_allocation.rs': '8b3d517ae8a5f00e4875bdbff190e4343590f4a81ef4015b25b7447fc5e90b5b', 'rust/src/bin/s_session_v3/source_audit.rs': '1d2d85ed50e6b5b103fe1ce1a2e118c6172d7ad11769e313480d95288ca4a182', 'rust/src/bin/s_session_v3/source_audit_oracle.rs': 'e1f66b0d31f365188e7baabcbf32a17770c56f837d44702f33c2491c521296aa', 'rust/src/bin/s_session_v3/source_contributions.rs': '507a66ef81681f02f7ae580fac2493488b54a6de218b42e88b5e224a83c307ce', 'rust/src/bin/s_session_v3/source_delta.rs': 'f5975d169300e27d4e094454736dd64f71aef5897631f1c54d16133a08f4b810', 'rust/src/bin/s_session_v3/source_order.rs': '1d8aec0b2e5966e2f053b486b59f26f8b27dcff1111c5416470760f56252b834', 'rust/src/bin/s_session_v3/source_pending.rs': 'ec80d4e7784e23ffa87fcb30e54467bcac7941f1bff8dea064c3af63ed707633', 'rust/src/bin/s_session_v3/source_pending_oracle.rs': 'ec524a0b23cba07d8c1b16a1b18b06a25b079c33f3c6a57f708e37d41bb517d3', 'rust/src/bin/s_session_v3/source_prefix.rs': '7f9b3eea173c6276ba5dfaec543f848e46b21917a2663da27c0ca6e7d6675fca', 'rust/src/bin/s_session_v3/source_structure.rs': '5dd619d794f97a06800d4b92210f253c7afc0ddad84ae80b83c3516685255111', 'rust/src/bin/s_session_v3/sqlite_pager_probe.rs': '3c58ae756b23de9a203ab4f67263ad5b664001c0ffdedbc63b05832744105fa4', 'rust/src/bin/s_session_v3/state.rs': 'd40af81496dcf45803048baf2c26309e13f64f17565a84f01ec9d109f53a58f8', 'rust/src/bin/s_session_v3/writer_cli.rs': 'e412dd79940562e44d93738ecf2333c6be899a1dc197fccbc2e8f4b4bbf65b3d', 'rust/src/bin/s_structure_session.rs': 'fc75d59e8a81cde6ebef41fca035b0b599bc68b2c00e71f5f18e9aae48439907', 'rust/src/bin/strict_nest_check.rs': 'f313d51dc9ff60c3261730060778c2013f182e6084bb65f1ef61185339f79d4e', 'rust/src/bin/tangency_probe.rs': 'd96b795bdba2f267e563ca13a709710d598945b0f777f3e82ddb04caf20a2603', 'rust/src/bin/theta_accept.rs': '29f6ea67036e72c1acb64cb1ca0feaefdc7fe01e633c76a79abcd52032102500', 'rust/src/bin/theta_backtest.rs': 'cb228c18216a00402f81fed225833a19901ecf157cda25f9ddf9865d88a036f0', 'rust/src/bin/theta_m1_dual.rs': '429b5192eb817243a7dae9032ff301066be0cd53e6a054089e7df6f1a04470d1', 'rust/src/bin/theta_overlay.rs': 'ca28818741e2e62f66da749f2ca958e0fe6055c0e97c6da12c32352a2ff5edff', 'rust/src/bin/theta_replay.rs': 'a099e3c1d2ad4d757275ba567574a1df5d68404687bb43bc53158e38e4defbe4', 'rust/src/buysellpoint.rs': 'f9001ddffe19484989b6ecd9ee376b3b6e085825996316c93d6474ff7057a247', 'rust/src/c_segment_verify.rs': '587d6e36178846f36b157ced6b6dd903969a26e19ede87567294103b7f0046c0', 'rust/src/divergence.rs': 'ce13e7c7ebe2c062348004ffda6815c22468a78fc8512a372c253eebeb2de5f9', 'rust/src/economic_session/book.rs': 'e6dbd45a64c4c93f6de59c1f4362a89c2a4b134106ea2a57bc470fb020e7a09e', 'rust/src/economic_session/closure_replica_tests.rs': 'c292507a3e92b9e35dd6d3f0504c7d5e5835c279ed4fe75874b73a4d3f11e6fa', 'rust/src/economic_session/closure_tests.rs': '8d585835fe89f1d2db7b1b02a49e06c66bd673136ddbb0f642a81a155f097271', 'rust/src/economic_session/content_store.rs': 'bc65cfc015bffeb96c5f8f9b70cd9c520c650f56629ed1e13c8dfddbe2ee0450', 'rust/src/economic_session/evidence.rs': '77cac6ff99a793d142c3f3701f29d4a24c722cbdcc4b2926bd54181661f0b0ad', 'rust/src/economic_session/finance.rs': 'fcef033b813822609e225e5208a8d6484b9dcfb52c1b90387af01433bf940d87', 'rust/src/economic_session/finance_tests.rs': '7e40b47d0ee5529852f0ec65a8021b5334f0b518fb39648aa13c7794f8e9e2c2', 'rust/src/economic_session/finance_transport.rs': '9004fb38811742b93b1f65049f317f34b5e6ae15827dd31a2b291533e134f1da', 'rust/src/economic_session/financial_content.rs': 'a2091b7dd2bda8862d0716c8dbffbf19eb7e94e7e293134e7c25817ed93494e6', 'rust/src/economic_session/financial_reader_resources.rs': '74c0dd50f6d69054b449c919c5ae15385248c034eafe96bc04e73ff61cb596b1', 'rust/src/economic_session/mod.rs': '3e18af5d87be88040d54a5d8d463e894aa74c2581407e8347a7180c739237d2d', 'rust/src/economic_session/reading_tests.rs': 'cf6d22041cd2862de1d2ddd34cf63b5f432af1b635403ad91104d95047f1a0a0', 'rust/src/economic_session/service.rs': 'f5f685e6844544af8ee1b0bb50dc6c89122dc1fad23c9d50bb0279e5e2703413', 'rust/src/economic_session/store.rs': 'b7db658d28496374567eca0b6cba3b250669161f685363af037d688908bbfc16', 'rust/src/economic_session/voice.rs': '47320079744485b8b6f2b229bb7441602e4a9b11a9a92eaeb9057f6fe543b218', 'rust/src/economic_session/voice_tests.rs': '47509489a874f37d51afef33af67ba1ce68ad652426a9b135db57bc21d1640d7', 'rust/src/fractal.rs': 'da8e968fea94c12d1e5e075db161c192556acc5f7865609fded301a44fadcbf7', 'rust/src/fugue_v3/accounting.rs': 'bd63a3c30c0ecc36330b1f16206ac8fba3421745117cb0bcc683c2a209f44276', 'rust/src/fugue_v3/axis.rs': 'c804ccaf9e24b280413254ad7c2e7fb9e426731250381758266c6ef48a71ab5b', 'rust/src/fugue_v3/cycle.rs': 'db42f51de264fe05fcc426feb1341c7873ce4ebbf507200fe42ddc31ba852291', 'rust/src/fugue_v3/engine.rs': 'c7366f9d21910a435aa9fa359461273bd73e513db34b6b188d9bf860f7e5fdc9', 'rust/src/fugue_v3/ffi.rs': 'adaf89d4fd4c61b43bded48d20dbd890937e7f6cf744a040a2815272ebce129b', 'rust/src/fugue_v3/layer.rs': 'adb1f7489019169636e60ec79272fc33f2dfaa50d40f532ac7db4ad362a4e0bf', 'rust/src/fugue_v3/mod.rs': 'a4be842797c4fe77c78c373ff0dcca0fe2b668380442e4cdf603a91b85b677df', 'rust/src/fugue_v3/morphology.rs': 'aa74ec7117892a016196abd8578c039c55a6ed5e726c10e92a7f91b86c994800', 'rust/src/fugue_v3/observe.rs': 'dde286e2222d4112c741f71642ad49d67a69ad4525098148665e78dc6f4afa8d', 'rust/src/fugue_v3/operate.rs': '0bdf52b4b65a4fde253f9d7058646f2a8c2a180112f0726348a19596cb983a33', 'rust/src/fugue_v3/prove.rs': '024fb48ba5d25a503da609b260e4c9f8d073d51a6604165dc605343ac53dc545', 'rust/src/lav_seal/bundle.rs': '4888840adf2fd7eb6eb09e9ea51352f735caf7008d9a8395f22f34902df294d4', 'rust/src/lav_seal/comparison_set.rs': 'af8cfca3149276b66d53419c22d0e4667900b409b206a176e4322c09a35ec3a6', 'rust/src/lav_seal/digest.rs': '3df4890115c74f3d7ac5db9ec8f5ae113fff8b2fdf09cdcc3da4a6099645c7dd', 'rust/src/lav_seal/jcs.rs': 'fa4e984ad548da4b0b87693e5fe59de9fc29dde65184e15d0f9cd4d398fc8970', 'rust/src/lav_seal/lead_baseline.rs': '9d5cb1421cd0a959b7716d3b8f0741dbf86f6e3ce64ef3bd11a6ef31bc1d46c0', 'rust/src/lav_seal/mod.rs': '9357ae833ed0dc2e41f3ed00cdb8b738b4f6d1504656664063637628da8806c7', 'rust/src/lav_seal/samples.rs': 'de24d9a2de0d51b87a06d8c66a37b34830578b3d28ea765227eda1b5bd82bed9', 'rust/src/level.rs': '9b9716f902973f5c01aecc9ed10a54281638acbb187ed53e3a6ba78426cf07a8', 'rust/src/lib.rs': 'fb95d0650b35b65682adedbf85c8d6d432faf4c4e40913ada954e60e9e1b5710', 'rust/src/macd.rs': '2802ccdf2f159c498378153696603fb5515c6db1a831e0739bd47674f88ee1f2', 'rust/src/moves.rs': 'f97cd4dfaebc9d614ba1b80a5d2328bf5f6cc4fd9f3028d767f581ee76a26bd1', 'rust/src/orchestrator.rs': '0ef1a1391fe45a225b889cf701f95789fa6170017b5d8cb1f1c17535e563e167', 'rust/src/ph.rs': '0c43e0f50bbddd4582b8594cd1eae76a118748c99c7a7ae65583ba667b7d1f4e', 'rust/src/recursive_t/backtest.rs': '16cfd8ca72d48198639d05f5e6fb0a88b03eadfeee95e2adab3988db52b44ff4', 'rust/src/recursive_t/backtest_run.rs': '7164ccdbf9b3ac4ad687a75439176ef7428ccd61c8f10c9cdf713007496cbfdc', 'rust/src/recursive_t/center.rs': '8e56ba3e54ad4bcd0a847de71ad5c96c602e2a2aa034954f66b1a2855c0d0087', 'rust/src/recursive_t/divergence.rs': '245558b9203803bb865f9a93ec1fa6564a2862430b64a5906913576cdb6aa95b', 'rust/src/recursive_t/ffi.rs': '75d282f181d30d7f1da75c6817eb0626270e1980113a704067152bfaef644461', 'rust/src/recursive_t/mod.rs': '90196d73b5d06d38efb9490770bbc1a4fb748bbb9070efe1cfe573e5d810c9a5', 'rust/src/recursive_t/operator.rs': 'ddbbfb73876edbc9370c37a8dc3f2dac13a2609243e98f2cbfba86c6b75f63a8', 'rust/src/recursive_t/prove_guards.rs': 'ffbb4e771417f6aa0796099dd3e354d68f406a7594732a676ffc328e486824c1', 'rust/src/recursive_t/rec_driver.rs': '8c829a4e465b54a4eb9ae2eae991f156806b22f9491675a06f6d3abf2cc3b410', 'rust/src/recursive_t/rec_engine.rs': 'c976aa9138fb469d672331e9dc35944a402c14081691b507dd3663fca57caa37', 'rust/src/recursive_t/rec_stream.rs': '8dd8cb1fad08b545a75520f47a23df7ad2d15a212b9b71875a963729f1760e3d', 'rust/src/recursive_t/stream.rs': '99fac4c6eb5d45016414b7163803f8688fe7620ff34820e154a9130113b032b5', 'rust/src/recursive_t/t_engine.rs': '5f5c5e920e498944856a309901d39b14d942824fb3978b6d4666e590507232af', 'rust/src/recursive_t/t_engine_run.rs': 'b9cc91ef273c267513df4a159a2b74ff6676917dfdb22ae941a697bbff3ead9f', 'rust/src/recursive_t/trend.rs': 'e5b80b742c4beed4d552811141698381f933295f08d7a841006f2201646055dc', 'rust/src/recursive_t/types.rs': '72c5af0108bf0cd00ad983b60a4a8af98ea61eb8206139803cd4c804ddb8658b', 'rust/src/s_content.rs': '3879abb91c095a2a0b39bbc10b34414d605bf0e7bed2fd1205d766f110a22f51', 'rust/src/s_content/access.rs': '50123038fbb3ebaf87ed8831a62be2435a697c6486247f392790074721ed6914', 'rust/src/s_content/capture_allocation.rs': '8a9fb283efefeda43e2d4aebed945df4f9a94bad24772a56971a780a6dd7a755', 'rust/src/s_content/capture_allocation/audit_json.rs': 'dce00e8bfab16864f345bc38af6d5ae541e05c5f7fb032895bc7501a4ed25c93', 'rust/src/s_content/capture_allocation/audit_json/number.rs': '23ef4ed233e09a50e35ab8db434972f9da5dd0852d8c2a691c54f7e96328eb5d', 'rust/src/s_content/capture_allocation/audit_json/strict.rs': '4a6d021e0b7db1d7004482de04b4ceb335f390dc773db89af747f7acb48b2e1c', 'rust/src/s_content/capture_allocation/body.rs': '99fc470ed50db4e806bf12a36cf62286b635c832c3485a6f6944d28c0d912352', 'rust/src/s_content/capture_allocation/dispatch.rs': '4fe18cce147806d621cc24ad40188bd8c444a68aa6e7bc8c19728b4b21665b4a', 'rust/src/s_content/capture_allocation/overlay.rs': '7362ed65d843e3dd8916f0d911e6634b8b3cc9ec8551be7f67559bd3ffe6b8d9', 'rust/src/s_content/capture_allocation/scratch.rs': '97224d8f99376c78e9736e9501fb1e5d3e1e28af70b1e60ff5360e236fb2c483', 'rust/src/s_content/capture_allocation/sqlite.rs': '52fb297ca3aafa03f592ca3715ebe7959be9e7c12a923e228ff8719e45746086', 'rust/src/s_content/capture_allocation/work.rs': 'a7f30347f89470776bd6e9c2166cc4f02fa810e31eb535ccd90633d2c6bd80b2', 'rust/src/s_content/closed.rs': '6a258beee600959987beebe5043bfe28813135cfcef09276427188e16e8a9c7a', 'rust/src/s_content/fragment.rs': 'fdcfc3dea8e09ee11aac0fce9bca30f5310ff1f3c9fee204918269bc219f335a', 'rust/src/s_content/job.rs': '1659a774e681f67ec9761d45adb298a164722a7e363e72d2e60f7e79256ca220', 'rust/src/s_content/job_wire.rs': 'bf5d31b3cad0b4769f9d34ecdfa3801836f52b8544b25e55ab8b8c801d1b5f63', 'rust/src/s_content/json_writer.rs': '47620b239f6c6c18187b3ad4ec1b82c29babf6cdd071b857f06eb22a86c8959b', 'rust/src/s_content/path.rs': '633df8d61a4f3ed47f8b3d1eebcbd84345eb65ea6d0f2c2c12d97ec41a673871', 'rust/src/s_content/ref_owner.rs': 'e3794c5479ee3d2b591417b26a0c1241f807a51bedf135cc3fed1d789bac09fa', 'rust/src/s_content/sequence.rs': '5ec92ede39d2a991c3b03090837884e27f8160067bbe49a09b93cc787a8d152e', 'rust/src/segment.rs': 'e6a39b5770e1b79cf3d7b4b5f2ac9738fb67242eb19e75b6162da75fa3050d54', 'rust/src/segment_layers.rs': '4acfbcf1560744a84d4b467fd4762b948d921a40c4d8b0406a3b2a5767d54646', 'rust/src/segment_tangency_tests.rs': 'a94f75f271cebaa25e4b55bcc720441fab58498e43c8e4d9d6d591806d639a55', 'rust/src/session_protocol.rs': '1420f14c93d9f3bfa6df3834abf079676fb36a860a18c7bc4017f419b70fb15f', 'rust/src/session_transport.rs': '48a3a985640091eb3abb81ee0c5b934f2719c641fc7732583b795ce51157857a', 'rust/src/spiral/accounting.rs': '6dae31f2174e0b1660fa65eba93623ca200ce8e25cca82aadbb35944f2bd2bad', 'rust/src/spiral/closure.rs': '3749c762809f22f05038ef341402931aa6277af5b1e7245834671a5f8c2d5deb', 'rust/src/spiral/engine.rs': 'c15a90a07c372b56d8df4d8b92f38be7ea5b6482ee03f95227a86ecf3d36c787', 'rust/src/spiral/ffi.rs': 'e3e824cd4388179129e35d381c17048e9d24930c8c7006c005680b19b2fe603a', 'rust/src/spiral/mod.rs': 'f9b171991bd5458aa8be82a3d689740e73f5a79a8707ac7f21841c81838e11d2', 'rust/src/spiral/operation.rs': '4a5b229c5707d363971aa91f34a9a01d9d2b70f015f1dfaa669f1d2d8eb36a2d', 'rust/src/spiral/params.rs': '79843a792bebcd2b16f44cc7d08df5528dc8e333e707781bf003ccd87142779e', 'rust/src/spiral/prove.rs': '478ee3ef9c063a8b421f858f24a822a8f8151f12f3d6b6101ddc12d5549dd291', 'rust/src/spiral/result.rs': '88e6252531f353535288955a85827323812b1704d7d6eb6d1d1ea254916966f4', 'rust/src/spiral/signal.rs': '496fee28cafbcd77dd07fcbbd1cbf262d7a58050c11ec0cdb728bf61284c232c', 'rust/src/spiral/state.rs': '71c42c7ab91c56bc15d7e5bea57e172ae15253a49024fa58b893d050d48367c2', 'rust/src/spiral/voice.rs': '10f9be4f54566810ab17bc37d3b0c1afad9fb10f181dd6b67eae36f82436cd3c', 'rust/src/sqlite_checked.rs': '756c214d4108cd9eea5bc102a2006d2a53ee4b2762b1aedf6d5d4b032df1327e', 'rust/src/stroke.rs': '6ea8f41cacc714b6cbe89528ecca9e15498125d97c2174ba47a8c0af7e396ef2', 'rust/src/theta_v0/backtest/admission.rs': '762a05dced30b2e6cd89baf93f71c6536e29ff33e74ff5b4e589612ee5ae2aea', 'rust/src/theta_v0/backtest/capture_oos.rs': '5166ed81da3b99d58a64aa22d4368236390d8d142537006886ac6e70092e44d5', 'rust/src/theta_v0/backtest/data.rs': '725e76d1966bce3569544ee189d2e150b4524e507d1eb0fc1e21aecae5ce7539', 'rust/src/theta_v0/backtest/decontam.rs': 'f4a4b9a295a9608bba7fdf6f34c72c42b7f5b9b7bc8e27c70371032c40f978bf', 'rust/src/theta_v0/backtest/diff_capture.rs': '9b87fdf0549079159ec1ea862f9e719ff51eb1c9c5acbef9fa86f0c7441e68eb', 'rust/src/theta_v0/backtest/dual_ledger.rs': 'f1385677469b7bd5747858210df2edceaa2795c8337550c1255a95a00d9f92bf', 'rust/src/theta_v0/backtest/econ_positive.rs': '5b9509e3bf253d1d628372cff383de19c469ab56fd0773867efdb94008a32042', 'rust/src/theta_v0/backtest/econ_positive/cand_delta.rs': '33264802cd72534ff19300f4ee552eea70ce61bb27c92c4b84586ee9669d7b41', 'rust/src/theta_v0/backtest/econ_positive/descend.rs': 'ad1d82131e399df298b3e011550a0170662d0783910ddd5207f560ef6c806e61', 'rust/src/theta_v0/backtest/econ_positive/spread.rs': '07f4629158c9e20b799f0b201856fc9a22d81cee901811931fefe2bf7b5015d2', 'rust/src/theta_v0/backtest/econ_positive/tests.rs': 'b185cbb6c39cda220e2620dd181be7b4c526349cc33f6b5d00db433786d862f8', 'rust/src/theta_v0/backtest/econ_positive/xzd.rs': 'e6019aefdb0e5994236e5bffc260abddce01f59f1b8337df663eeeb14a1ce591', 'rust/src/theta_v0/backtest/fill.rs': '408a8b7c02c14679d3cf6660434b76eccb18985af3e9a32542bf4c421ce72c4a', 'rust/src/theta_v0/backtest/gamma_dump.rs': 'a9df6b235078c93ae942a5d3702528ed563a24afcd2669e9d0e086a9461577a8', 'rust/src/theta_v0/backtest/highlow_mu.rs': '083d50d37560737cfd5b6267d07dbfb17c64610429079ae0b1fdbba3c33d6771', 'rust/src/theta_v0/backtest/incremental.rs': 'b60d7bf792279b6755edddbf90e1d0264945e022fd755986956ae4b3d2bd7243', 'rust/src/theta_v0/backtest/issue837_probe.rs': 'c7b5c13c2bbd4153086e7eff677373ea9646f950c84e4866ad40bfcb422a0178', 'rust/src/theta_v0/backtest/issue841_probe.rs': 'bc8faba3f08c0e47202859b832e0917d2eca0b591922f22feafc42a392cfd9d5', 'rust/src/theta_v0/backtest/l3_delta_r_alpha.rs': '0d80672e883da8062ea8e8b66a32424e17bf0f485b971c7c6291daad8034119b', 'rust/src/theta_v0/backtest/l3_fullwindow.rs': '6bcefc365f32676140d733e73b5148ab7f9953db7cff5f4000d2eccda0ccdaff', 'rust/src/theta_v0/backtest/l3_pi_depth_diag.rs': 'bdc498b83dbe8dabb3ce9283226a422322c00ad7e1bc9cab6bc5d41f789e251a', 'rust/src/theta_v0/backtest/l3_pi_falsify.rs': 'c00558665faaa6f62f1ebdcfada6c72081f0bad50c92ee71536232681ba02a51', 'rust/src/theta_v0/backtest/l3_pi_probe.rs': 'ce825d7e09471f77044f36f40d3494526d831ce0562555ed6d0780297f825037', 'rust/src/theta_v0/backtest/ledger.rs': '766093df32c6ee5f80aef4f110319c459319f0e182abca16eb41136cb19bd752', 'rust/src/theta_v0/backtest/m1_dual.rs': '1dd4df6a7239e554bc13167ef32aac95a0d56d7becfb5aed0664e40d5cdfa5f4', 'rust/src/theta_v0/backtest/metrics.rs': '8302df51552768b0f8057766b0be766aaf0f27fd13b395e96e209788bf3bce4e', 'rust/src/theta_v0/backtest/mod.rs': 'd5977f6842441fa5e2ade0a2adb4e68b072793c1c04f9dfac128aeff2d075833', 'rust/src/theta_v0/backtest/mu_estimator.rs': '6f60208fde012864c0df5d2b4376de5072162165c9533ec942e7ca2dddc49892', 'rust/src/theta_v0/backtest/open_ledger.rs': '91bc4553731c5e041c10f20c40c41dffc7866e191d3dc42539d2a95b1cacfd65', 'rust/src/theta_v0/backtest/opsem_dump.rs': 'fd597183b86bbcabf72e2d7c974110d4f8238f11c730c82485fc176a8667d1f2', 'rust/src/theta_v0/backtest/pan_div.rs': '41f2e699a7fe56d5405416da9b9b8253c16a8f2c9baa7bd9f401c3efc3d3b5c9', 'rust/src/theta_v0/backtest/perm_test.rs': '17cd57fe630045b7621bd613ab0c28bca14e8d8b25386b12f2e29d1b3f0f4e1a', 'rust/src/theta_v0/backtest/pooling_icc.rs': '1d9fd2cb2683e5f841d01bc480929d30e17b476169d8846bc523bcbac2be5685', 'rust/src/theta_v0/backtest/prereg_windows.rs': '5a093865e264b144804dc24c4eb623552e329d90c8ef3f00e3ff61d445fc26f9', 'rust/src/theta_v0/backtest/replay_dump.rs': '3de1fa809017b7b3a4a716b2bc704ad616167be67842f68c10f53f4dcca7040b', 'rust/src/theta_v0/backtest/runner.rs': '405e18ada2a44613c7adb520674533367aab1944ccf0ec133434db4a3244db8e', 'rust/src/theta_v0/backtest/runner_tests.rs': '1230539e0cd2c6f2af30c118f770fe42a480f4b5056074ea4d2d3a0c3306c22b', 'rust/src/theta_v0/backtest/segment_gn.rs': 'b6b5446c1c986fbea0a5733e813ffb92b4c9a9f8bbf1b6570370b35ac9f249d9', 'rust/src/theta_v0/backtest/selector.rs': 'a289de1df32e0a162b0244bb0715b16e5bc0ebffe479eb79051ac260a652314f', 'rust/src/theta_v0/backtest/signal.rs': '8cb8c2a226fd89e356d1384823435b7dc44883bb1577b4254e089e292af1ac55', 'rust/src/theta_v0/backtest/theta_pi_diff.rs': '2b32cfe9d1c869913b4eeb913d6b82b5a34e580f3304d1289e8fc316198a27d1', 'rust/src/theta_v0/backtest/tick.rs': 'd02e730b264f1e9b9c646f9f202139af1b2361d2a03c0364856ac6b50950fd46', 'rust/src/theta_v0/backtest/treasury.rs': 'a5d444e9e871a63d1b5a64c6bff77a0b38bbd37a804ad6c1ec72fdd8457a8e9b', 'rust/src/theta_v0/backtest/wverify_run.rs': '0597e850c4a03dc193bd2f236564894555120ade23e7766668cc8ac891396455', 'rust/src/theta_v0/classifier/bsp.rs': '06585715af51cdcf239815ac646eb1791fde1d5ce12680b9b17a1ec31cf80eb8', 'rust/src/theta_v0/classifier/bsp_bridge.rs': 'd079e482b33de7387cdd3d4fec25342d123768361b6164c4a2be4767f9e0dcf9', 'rust/src/theta_v0/classifier/bsp_bridge/tests.rs': '366eb2e7b2fe7a9f563f3fbe0857fa34ccbd53278406f2cf1eea23d6f4362361', 'rust/src/theta_v0/classifier/cand_event/book.rs': '156b01c5ca72f99e6edc72cdd4665edd9d3d4f5584cdfbbbc89d5a01c0b0f2f3', 'rust/src/theta_v0/classifier/cand_event/delta.rs': '56a60db0bada42ba44562b0ab71db0634eae645e037faee9e156d978efd8a500', 'rust/src/theta_v0/classifier/cand_event/fixtures.rs': 'b072067cac8b28280b7d660fea9621570c6d81fc889fb921097d17d4190cde23', 'rust/src/theta_v0/classifier/cand_event/key.rs': 'cdc175bbdf8d39d26a8712ad126efe425a8c5a372d150c0c0c9856f56554cf85', 'rust/src/theta_v0/classifier/cand_event/mod.rs': '38c0afe9cff5ed9cfa5fb764165e361acf86590e061c49389e6e9a778031b97d', 'rust/src/theta_v0/classifier/cand_event/observe.rs': '6198f19655fa6074b2c3c96c2488e28c3d039cc17cbdfc6083496c5701940045', 'rust/src/theta_v0/classifier/cand_predicate.rs': 'ba538f46435848e24e8d5f0f6bfd0b0ad66bd38b5f0538e764b86c09921b6901', 'rust/src/theta_v0/classifier/cand_sub.rs': 'c13a5fa66326125cf6ae73bceebe88a27159a7a8f6e4ada4168e89b0af0d13d5', 'rust/src/theta_v0/classifier/center.rs': '97e7c3033debbdda65cbd5897b63473a957b71146b377e2409ef92c8f2698f74', 'rust/src/theta_v0/classifier/center_lifecycle.rs': '6b16d49b2bdac4c9fac82ec4d8910859f995a8655e9529dd5711275831a0f85c', 'rust/src/theta_v0/classifier/center_scan.rs': 'd90d3d6b630cc41eecaf1faec2881d9634a098afde32184705b30b92354fb77c', 'rust/src/theta_v0/classifier/chain_cert/mod.rs': '748df93a112549a462c4a4678286c3cd0ac71c52965c9c79b00cd83d242de66b', 'rust/src/theta_v0/classifier/chain_cert/tests.rs': 'dad9ea558de093d485e216cc0810e27630dddeb441c58783dd88028d736a146c', 'rust/src/theta_v0/classifier/consume_at.rs': '50c5e33005a3a687970d91c3307d32fad100b240d9387a82b92f4049842c27d8', 'rust/src/theta_v0/classifier/decompose.rs': '95779daefb44ffb5883d9ff04741056be93f5c23f62acc5f94dad5d624bed0d1', 'rust/src/theta_v0/classifier/descend.rs': '3c914782557bd2d6ea16d9800eac26babbee209e065cb1284a971f429a30d197', 'rust/src/theta_v0/classifier/diag/cand_delta.rs': 'c81663eb22f4d0ca18a000e90786b0cece4fc8f8a2f3a1c4b67880da29b7d702', 'rust/src/theta_v0/classifier/diag/cp_recall_audit.rs': '866ef330ee179bb04943b7e9e223ba3313a352acdfdc0c504b35b02a0bd973b9', 'rust/src/theta_v0/classifier/diag/cp_replay_diagnostics.rs': '22ae89e694fb4b86b3b5ad65feb44976760f4b6a65ee3b727b0801a563e548b5', 'rust/src/theta_v0/classifier/diag/mod.rs': 'a766bffbbf4eef0a1b6e67ff8ce0cc4372ed201248f1cb23680ad7813eb28164', 'rust/src/theta_v0/classifier/diag/oracle_probe.rs': '653df0927a65e1f4d8d77aabe1674dba245930edb06605974c824b2284589e7b', 'rust/src/theta_v0/classifier/diag/s2_mirror_capture.rs': '25cbfbdf8272808dfb70f675563e83f5196ae5fb68c3290182688bcd1d26e65a', 'rust/src/theta_v0/classifier/diag/stage_profile.rs': '5eea7a50b5a00225a5f9e83bc79736aee5b2bf4cc3976b5f181121f55862119d', 'rust/src/theta_v0/classifier/divergence.rs': 'f843069254f3c2895bf79df4ad60a6229874338a208c0b3595cd3ccf1746818c', 'rust/src/theta_v0/classifier/e2eo.rs': '912c48b2bf26a602e7530a2ff431c10314fd8dd7aff871e6c43bca806d50b17b', 'rust/src/theta_v0/classifier/interval_necessity.rs': '34e37a4e63ad2ee1c805167fa6ffb75304ac773e977724f245c9ab14e61bb0e0', 'rust/src/theta_v0/classifier/lean_mirror.rs': '1f41c88bd25b94008b276879f0b2c45a70455587d4418605a1c6904b67aaae94', 'rust/src/theta_v0/classifier/ledger_kernel/mod.rs': '56d6416646da123aa8886c84ba1b2688e48ab13ab3c5dd19e66d1ff7b3be3664', 'rust/src/theta_v0/classifier/ledger_kernel/tests.rs': '36c50a2367d183142b20d7566ac2e3139ec0761f8a0292a5bf40cd5a5798df3e', 'rust/src/theta_v0/classifier/level_state.rs': 'ebcfe8b6742f1e969222f000ea53c3dd27cbef9e5dbd421869f041c35e3dd08e', 'rust/src/theta_v0/classifier/level_view/confirm.rs': '8b36df933f78f22b104fe1b183429b7d8e3571dafeb75c708ac75cab82b6e2c8', 'rust/src/theta_v0/classifier/level_view/mod.rs': '2d3e616654f1a76bf650ec113e26ca10f5b995707dc560c35350fbf3bd3b8e4f', 'rust/src/theta_v0/classifier/level_view/pan.rs': '3dc166c6973994f9e448bc76e00421fdb621eba64d3bf67e7c5413b6c7bd266e', 'rust/src/theta_v0/classifier/level_view/pan_provider.rs': '32d961f31bc0b3d0c4fd50676a109cfba938bc9972aa30943cf487fa2c73dbb5', 'rust/src/theta_v0/classifier/level_view/projection.rs': '906e2db47af52cbd91e98f7c19544862dba0ed3bc907b8ff1155ffcb6fb2bdc7', 'rust/src/theta_v0/classifier/level_view/seed_mapping.rs': 'e7fe166c43b3f6df3f22a59e09d46e56f98de2ae12c30cf4cc4e0bfadabbda6b', 'rust/src/theta_v0/classifier/level_view/tests/confirm.rs': 'e9f24e5cf70c012d286f0fcc6c8f2beb49b93a68a6c5b68a2c4443aaa38dc805', 'rust/src/theta_v0/classifier/level_view/tests/feature_gating.rs': '517c16515aae236a4bc66cc289e1c977d41f386392cf1d65f89f5cde260892ce', 'rust/src/theta_v0/classifier/level_view/tests/fixtures.rs': '951314c132bb7214f1946a6cf29f94abcdeab33e8497c5ac8ae263dc936b5f38', 'rust/src/theta_v0/classifier/level_view/tests/mod.rs': 'd5766f5157fe485b6a6334d4047658593837bf83257b2b870ff01a17f47afce7', 'rust/src/theta_v0/classifier/level_view/tests/pan_memo_a.rs': 'c93cd9884ead596e3f67dd270785eb8b7dcbe3e2fd833b99ed2c3cb4ea08c398', 'rust/src/theta_v0/classifier/level_view/tests/pan_memo_b.rs': 'b74c66071a7d378d82ca58d012106a13ab8082af7d25d82b986a37b0531c7e4f', 'rust/src/theta_v0/classifier/level_view/tests/pan_memo_fixtures.rs': '844960621b07ee001a4506441c9b496e2fde932ba67641f41fc8fcea89928af3', 'rust/src/theta_v0/classifier/level_view/tests/projection_pairing.rs': '21d7ac63b83c14dcc2cdc54283010d883a1c767b7b9c1148edcb9e6eb58b57a5', 'rust/src/theta_v0/classifier/level_view/tests/seed_mapping.rs': 'a64939ee038fa455a7475aa74a52a88b7d6421e4297f6c0dada03da80f763a79', 'rust/src/theta_v0/classifier/level_view/tests/triple_anchor.rs': 'c4f495f390cf89287c0717d33c876d21254303cbd583ea97ce1e329b4c3ec5cb', 'rust/src/theta_v0/classifier/level_view_store.rs': '89a33979ebd73735057d40ad4881e511ab49bf6a258b682625c2530e9e7dfb14', 'rust/src/theta_v0/classifier/local_shape.rs': '57537f3a90c5efc30c8f8449bba6727d0fb69e810fd2bc4ad3ad6f9834f3ae97', 'rust/src/theta_v0/classifier/mod.rs': '8025167c5483e577fceb0c3b7b12344e9e61b5c4746a85980407654a83e859e2', 'rust/src/theta_v0/classifier/nest.rs': 'bf7ee5c7d6e0ecb918e7208273de7407fbb3fe90ed19c4469dd55408fff20300', 'rust/src/theta_v0/classifier/nest_index.rs': '8406a621c9252a56a8ebfb4b15c65c3698ed976d04216d1bc543dcf8097f34ae', 'rust/src/theta_v0/classifier/nest_lifecycle.rs': 'f2b1134caf867846e1f79745022484edcd9ef4c18408379e49774ee53989729b', 'rust/src/theta_v0/classifier/nest_lifecycle/book.rs': '906296f39c03574c533e694272340f6250d2eae49727106ec88d7883afcead74', 'rust/src/theta_v0/classifier/nest_lifecycle/entry.rs': '872120193b5a510e3a639cd9b87cef72b2d4437e0055644187f099c3734a6c5f', 'rust/src/theta_v0/classifier/nest_lifecycle/feed.rs': '29499015f939f03a350ae0ba1a856dbfc047a5d9de4063ab13ef26081588bf5e', 'rust/src/theta_v0/classifier/nest_lifecycle/key.rs': '24b3a0fba23b04df4294ad4fe6209d153230829280b39711a0fa3c41fc55b134', 'rust/src/theta_v0/classifier/nest_lifecycle/observation.rs': '8c0e5df650f7ffc5d1b157027e13686b6859d2b78bb5612f12114daef34926a1', 'rust/src/theta_v0/classifier/nest_lifecycle/pan.rs': '4b819809fb41c958bbf97e78206bfa432f80496fb3f17961aeff9905a1419e97', 'rust/src/theta_v0/classifier/nest_lifecycle/stats.rs': 'a52a7dfea2ca1689580013fc1e6e26c92c4c99226990b3760c62ee401d499227', 'rust/src/theta_v0/classifier/nest_lifecycle/window.rs': '2eeef3b3ad2b37fc1bbbcaf968de566d7c5cd7bac208e1c985215cffb78284f8', 'rust/src/theta_v0/classifier/operation.rs': '4d076382a3adc3b61876ca60084699503b84743125411ed894c05139bd3150c6', 'rust/src/theta_v0/classifier/pipeline.rs': '40a1b5f490da8af491be146c438761ad21f464bada55b0f9b20f41d047947edb', 'rust/src/theta_v0/classifier/projection.rs': 'f2e3b8cdd379e0c801f069e22cce43f26036aa950f052c819719e7f68602135a', 'rust/src/theta_v0/classifier/rebase_txn.rs': '1906bd810beb1476a11e28e09dfd6e2da4ea824a74d711ebcbf8c4030b86de9e', 'rust/src/theta_v0/classifier/recursive_tower.rs': '4f1a7f875460807f23baff731ee799ef90bb738e0e1584e7f9fafd577fed6112', 'rust/src/theta_v0/classifier/ref_v1.rs': '0d2424989c3ab9a44c788b0ad79420e856d8cc4269c5e64f8e01e44267b8a056', 'rust/src/theta_v0/classifier/retrace_ledger/adapter.rs': '5f6222ce7f778d3fa7f0269c6587c99bd5ae8d4713612ba6c05bf2f1a67ce948', 'rust/src/theta_v0/classifier/retrace_ledger/audit.rs': 'a9507689cf8e4555cdb0f3ca999f3b9f5a405c1c50944fc18234790e5111301a', 'rust/src/theta_v0/classifier/retrace_ledger/book.rs': '0f11f3a94489841bc24ccfff898d429fcad34921c8a05053be1f5c3984ea5ea6', 'rust/src/theta_v0/classifier/retrace_ledger/established.rs': '92b679d43a2a4b43bf0f24cc5c8d13f29548f29c3987f49031a990388bb47584', 'rust/src/theta_v0/classifier/retrace_ledger/log.rs': 'e32a57db2a148bd283cddc560a0e3d43b5d4b7c3e9cf8ef5479392a8af0fb4bb', 'rust/src/theta_v0/classifier/retrace_ledger/mod.rs': '95d5c12bace46015444a426caeee10966cea71f121748442a0854fb915390ba6', 'rust/src/theta_v0/classifier/retrace_ledger/portal.rs': '4ba7ade8e256e2fecd2bed83e07036bb54ae5d21059c6b2378a3462eba24084d', 'rust/src/theta_v0/classifier/retrace_ledger/tests/clocks.rs': '144c92c9528e68566fccabfed9a3312159da4c90a6d5bec1de8d3be6b2b0735a', 'rust/src/theta_v0/classifier/retrace_ledger/tests/dead_center.rs': '183896c11cecdb06e514b9400e553fd77c0b7859b80b006cbce339e00892fda5', 'rust/src/theta_v0/classifier/retrace_ledger/tests/golden_log.rs': '1ea9a271bc9d00d2e7c1425333a4d8390a72b0a20a34015f314653d1ef37028a', 'rust/src/theta_v0/classifier/retrace_ledger/tests/log_replay.rs': 'db0aea6bcb78bf90747453eba948b07a6359bbba1d8a0cda31615d5143a2ad6b', 'rust/src/theta_v0/classifier/retrace_ledger/tests/mod.rs': '5e4794da80db128e9a18cfb33fd93885e1b540bc3a31fbed0b4f16b9122ecb43', 'rust/src/theta_v0/classifier/retrace_ledger/tests/portal.rs': 'c9bffa56caae851e5887d809c0d9bea8b2ba8d672ea8359dc421ce845d73d363', 'rust/src/theta_v0/classifier/retrace_ledger/tests/rebase.rs': 'eb028e862d6e532f1893fd377e567df16e6b265ac912df5244b25f8d7036d81b', 'rust/src/theta_v0/classifier/retrace_ledger/tests/rebase_audit.rs': 'c2068958c0fd34be80b46faca6b06a6c8a8b96c6528c0d167bdcf64bf1734915', 'rust/src/theta_v0/classifier/retrace_ledger/tests/recovery_guard.rs': '0c4a1334a648ea6cf3b3ac3bf6cff39ad0378fad4eb3cc6c6209aaf7a67c805c', 'rust/src/theta_v0/classifier/retrace_ledger/tests/replay_parity.rs': '9d0cdc1576d5b0afcd0475f5c7692d4e1e15c8743b443f2a537101ebaf43775f', 'rust/src/theta_v0/classifier/retrace_ledger/tests/short_retrace.rs': 'a20768975caf4726cbd8c3bb61eee41cb01c12539b9c3b5497142cf449cd8bb6', 'rust/src/theta_v0/classifier/retrace_ledger/tests/standby.rs': '4a6cc870243e37436645d86c2f29579a375a7c166fa428a7129858c79cc92868', 'rust/src/theta_v0/classifier/retrace_ledger/tests/state_machine.rs': 'c6f80b97566596acb08105b1e4bd049b0163d13824d3aca09d5435c2e6dd3d29', 'rust/src/theta_v0/classifier/retrace_ledger/tests/two_pass_evidence.rs': 'a72b9c4f3cc7a4caaecf0342da11e4eb56622a5475c5434ab597db5f30b8ac79', 'rust/src/theta_v0/classifier/rmove_compose.rs': '91978d1fbb30e8794676a737f2b7b5902771c55b02c1f41ebdde51ca3d441e0a', 'rust/src/theta_v0/classifier/scan.rs': '856288de02d9b27e4f599f3dc3ce88c13237d213bf8804b85f975cbfb2d16a12', 'rust/src/theta_v0/classifier/scan/wire.rs': '39a443a38c255abec5b1b10773e014367927becc4a3ba576cb3adf34cc85c5c6', 'rust/src/theta_v0/classifier/signal.rs': '095931a397d8308eb06e0333eb3d8853b1f03112ef62394aa0dcf716725b9e28', 'rust/src/theta_v0/classifier/signal/cert.rs': '3d2e642d4a6d3af63d99251bab4493f55c8fa8f7250e7ce89c694b85758c18f5', 'rust/src/theta_v0/classifier/signal/gates.rs': '957c236a7fbb2e7b9eddecddc965019763c515de56447d2429eb68c422e4d662', 'rust/src/theta_v0/classifier/signal/pan_div_short.rs': '0cc0ed5074373b4a1fecf53e3eec5c5bf36308c34e61b7467041c2ca992884bf', 'rust/src/theta_v0/classifier/six_state.rs': '8f172502ffc2bb743b23d893e07c5998b8ab260915eaf58fd6034b6f55b905a4', 'rust/src/theta_v0/classifier/streaming.rs': 'b6d6c70a3a1aa86a4b648a3c8088ba8f0a914289709a066b26f16c5b05fdb590', 'rust/src/theta_v0/classifier/tests/center_pair_1424.rs': '582f0f6436110df1a02ce142eca53291a10b97a653ba4a918925c11ca6599cc8', 'rust/src/theta_v0/classifier/tests/center_scan_1419.rs': '97b4380b7a981ad60e2d2b9479744b6f21accc795544e03161e0bec1acda1994', 'rust/src/theta_v0/classifier/tests/fixtures.rs': '16d98b6abba12c788badcd2ee722f520b0cec04444b79daa576ba02a52880ec1', 'rust/src/theta_v0/classifier/tests/incremental_parity.rs': '5b740f9d90031ce93fbc36e81af8121d82763910eb869286f8fdf68863457f99', 'rust/src/theta_v0/classifier/tests/incremental_profile.rs': 'a763beafbc0382b64a13dfb93312b9dadeaf4524818b7cc78a8a3f7b1037b75e', 'rust/src/theta_v0/classifier/tests/mod.rs': '61a6e5e52193ffb78ddbb5d169439d03af64f1b9cba96ed7730344401e1cc469', 'rust/src/theta_v0/classifier/tests/operation.rs': 'ddfc5697c554b4364b1694272f53e7b97160047479f14684414dfb75e66c9fbc', 'rust/src/theta_v0/classifier/tests/pipeline_geometry.rs': 'd673a656ff3a26e297985347d06a59b67daba0e452c3f89cddaf8fb24b5c9388', 'rust/src/theta_v0/classifier/tests/rebase_txn.rs': '3f725a87e0b81e9d1b7902e0ac1ac79e8b40b20a3529f5dc7d30179e5e7b0a32', 'rust/src/theta_v0/classifier/tests/third_class_anchor.rs': 'c49f964e115ee907f782f343a4d3abf604e161bb0165e31d2a608f4ddaceec6c', 'rust/src/theta_v0/classifier/tower_cache.rs': 'd4d373edfda463bc71d9041db7651a43fc0e5d5d4f9f80bfc633f7d0bd079b04', 'rust/src/theta_v0/classifier/turn_class.rs': 'e4aff64b5f20dd4fc0b82080e81f07be12084d23bf819e4959385074e95837cc', 'rust/src/theta_v0/classifier/unassigned_ledger.rs': 'dadcfb04d6a897e25d9afe34cae24404d80f035c252049827f51f44b993c03d3', 'rust/src/theta_v0/classifier/voice_eat.rs': '6b35aca651e547ca192a1bc8f604cb93c8f2bddfdd6579992a23db5148e22135', 'rust/src/theta_v0/closed_loop/buy.rs': 'a074c93438520ef576c490508b45c09f4a95793822f689a947560cb96845e42c', 'rust/src/theta_v0/closed_loop/conformance.rs': 'f5c310488d6dac1e8294cb8324a5e11a4c282b74807ea0a31e91b645459e34b8', 'rust/src/theta_v0/closed_loop/mod.rs': '629f9e42b534787125e60176e103aefc08fd430d54953d99177e7590e6e6fbdd', 'rust/src/theta_v0/closed_loop/state.rs': 'cd78e3a3995298148508e4b6bd96c0d6a06c7ed1ba6202aa125d233f2a1a6af2', 'rust/src/theta_v0/closed_loop/transition.rs': 'b4ad9afa451d78c21b9eb6cff44a7ab2141f642aa83e95acf563060cc3b5d8d5', 'rust/src/theta_v0/complete/event.rs': '67a1cc6a49619ccf4f0430b5f901f501d33082462476c2824d9310cb61fdb4cb', 'rust/src/theta_v0/complete/mod.rs': 'd82ab96c7e70235a23867d2a3ea715433f2f9a2852684c1f1341b1e53ada9ace', 'rust/src/theta_v0/complete/state.rs': 'be0e4bf3cec1b084d7783425bbeeb43efc2151fa77085241e4f43e3d4d39ab39', 'rust/src/theta_v0/config.rs': 'd7bf588e7605a2574c7651aa81ed64400d1f2a680a41a4e529d755a941576b6d', 'rust/src/theta_v0/env_registry.rs': '9b6367071fde7834ccf3540c10a0cc27d53829c57c25c54122cb91031d1e74ab', 'rust/src/theta_v0/ffi.rs': '7b210e5d26952d967009311eadb56218c01de184228c5bd513433af846b06e55', 'rust/src/theta_v0/ledger/mod.rs': 'e44d60e46b017dbe9a9de4fb2827a5e8155aea45b7bbefba97a833b953cb2ac2', 'rust/src/theta_v0/ledger/separate.rs': 'f6efdc3e57c0712167d08437804de01532efdf8408ef16fdd6b4dc147d32681c', 'rust/src/theta_v0/lineage_book.rs': '5c0b75cf476950185a5b3ffe0092a14339bd0ac1d261150780af195a57989006', 'rust/src/theta_v0/mod.rs': '5a525d35166b3244a65c3cdbac9de7e50b18c3f388952172a4c0398e32590c05', 'rust/src/theta_v0/nautilus/account_adapter.rs': '16dfc39391e5ffff01df541226b4710cd100a49e47d3d3f15a5bc09f7f232b67', 'rust/src/theta_v0/nautilus/backtest_engine.rs': '974e7d6a6b75de8d619741abadcadb24ded5485a050ad78b7dc4342c44e6895f', 'rust/src/theta_v0/nautilus/bar_adapter.rs': 'c7a2f2b5d72ea19ec19eb71f100598c607227e61e81d986fa126ef464288eda1', 'rust/src/theta_v0/nautilus/mod.rs': 'a0818a8195042afb17d366f90295ce81cb59b57af0b638e8c0dc5df075d7c3fe', 'rust/src/theta_v0/nautilus/order_adapter.rs': 'bf30683af3a48bb75cde917d52bb4559ed8e07d3f946b93b8b6abc2f7594aab6', 'rust/src/theta_v0/nautilus/strategy.rs': '31425429a9c823b7b958a803a8a0520e306693383e95d13b8cab5e045b3506d8', 'rust/src/theta_v0/nautilus/theta_pi_strategy.rs': 'f6b046f148767e50273ec440cbb78f35f792115b4420b252d28cedbed0b394d7', 'rust/src/theta_v0/nautilus/theta_strategy.rs': '927d5e9c3b12cefa72f1f740ba613631ed4fa04a3f3fbd44ee69f432a6e4d072', 'rust/src/theta_v0/parser/canonical.rs': '24f2eac08aeb62dae7d038e8467814423a46210f82fcf311aaad2c6973e62d58', 'rust/src/theta_v0/parser/evidence.rs': '13807ed10822de27b570d7026b9955ffa647c987b8b07bc9638fcf1d2efb32ac', 'rust/src/theta_v0/parser/feature_seq.rs': '56a8e3d43abdbee7c79b627d70b6d222dfdb52cf5fbe7917fa8e91c5c3ec3aab', 'rust/src/theta_v0/parser/fractal.rs': '3d4c7c40d74225366d7488522d795dafbf24a1149bdf8903de5dbfd11970401c', 'rust/src/theta_v0/parser/gap_overlap_fixture.rs': '05973c5d7546ff77af23e61544bea7055471ada2b979daf03623649e99e96af3', 'rust/src/theta_v0/parser/inclusion.rs': '775e064c7716eb34815c0ac9acadd4bdcfb9b7b1032332af2d456531c0d39b7e', 'rust/src/theta_v0/parser/mod.rs': '6da7182d75d60f9e0131e3112c5dba3c465a5e13e7d484f5a0e43f11732bc2f7', 'rust/src/theta_v0/parser/profile.rs': '1ccc9654bafa70408352adf0da2f9d639d934d93d99553f489e314ee7e5725a7', 'rust/src/theta_v0/parser/second_fold.rs': '25faeb1b9af214704258433c513394df120f5087a0ef16f99421a7f56ce4f0c1', 'rust/src/theta_v0/parser/second_kind.rs': 'f4e0ca7308074d63b4c742be900f0e461d86e9707cc226fbe71587a2e6a1c906', 'rust/src/theta_v0/parser/segment.rs': 'd0399baca9a87a3ef39a4f47f6aa94d48e450cde393879abf16f5516b970c9e7', 'rust/src/theta_v0/parser/stroke.rs': 'e55feecfa62d4d8937c725be7cc49b65c7c1fd959700a2bee58ba34b03fb80d2', 'rust/src/theta_v0/parser/stroke_tb02v_tests.rs': '01b8540e36971b34a8ae78972154caa25fa7f44074e907180ca657d3ef2349b2', 'rust/src/theta_v0/parser/tail.rs': '6bbf0bce124d23e48c15a2194ad5c7504aebbeed7a2fef804e9359c97181b840', 'rust/src/theta_v0/parser/tb02c_tests.rs': '8c841d436410bc88abc94d4e97462dd899d4356649436453fdad869d24787ea5', 'rust/src/theta_v0/parser/tb02d_tests.rs': 'cfb74f1c4dd7506cdd8f77136c0ece887377737b882bd016387746cfd8f12d4f', 'rust/src/theta_v0/parser/tb02w_tests.rs': '8ae874f3ec62955204a8aa27f1f378915cf7c2ac5465e05603ca2c3be489a899', 'rust/src/theta_v0/strategy/account.rs': 'da08addc531230f15dec0a5e4f82d1f4e0709196a8d113df9099dd9feea04c0b', 'rust/src/theta_v0/strategy/campaign_book.rs': '2c3df885094d4ba1a489876908d86977836de2882750b7ff3c9235d7a9c7b135', 'rust/src/theta_v0/strategy/campaign_ledger.rs': '9f0762150a8d37c0bf7338b3eef86670baad00b241bfb1340643ec572f6059c3', 'rust/src/theta_v0/strategy/center_oscillation_trade.rs': '4f6a89c13880bd973bde13f501e133f969a1ab22ce4ca3c4e652b7fef7a943de', 'rust/src/theta_v0/strategy/channel.rs': 'c23841324126146d480fd10310235dc828c687f10c7b28a9cbb132d7036d5556', 'rust/src/theta_v0/strategy/chong.rs': '10f275046901df9143548c1b400a73dc6386824ed12e86f83173e5cb2a376e4d', 'rust/src/theta_v0/strategy/consume_router.rs': 'f2d1b90f9e084c689b717a27ce82b9dfc94eaac0558605b1ef9bacdef43706cf', 'rust/src/theta_v0/strategy/coverage/ancok.rs': 'fb728ed1fb91ef507687fe6b625da7d8f1c03b13a0124ea97d251eb75c5e7ba8', 'rust/src/theta_v0/strategy/coverage/ancok_tests.rs': '9f1b3882cd1aaa45e0fdfe6fa1292be1fc0a446d03d4633114c8b5530126c109', 'rust/src/theta_v0/strategy/coverage/compose.rs': 'e6018e22d4aebebd91b0739f276b595d2e87cbd60f17ff8dcfd2fbaeefa140ed', 'rust/src/theta_v0/strategy/coverage/compose_tests_1.rs': 'ceba22c57950a7870e5496d1bf2c06bfca7991e436de8d4ae10dd21f5a597a41', 'rust/src/theta_v0/strategy/coverage/compose_tests_2.rs': '9ea4a3dcbb5b4a0c641d334a6d4bf6f9b3c7b42bcba8d7679ac7fb9a227053dd', 'rust/src/theta_v0/strategy/coverage/element.rs': 'e4a11ab7f0061cb55bc8db90f34ce2671a38f374efe77aaf504a235df13a2be3', 'rust/src/theta_v0/strategy/coverage/element_tests.rs': '71201c49063583ed47ac69a1ca10df2226a97edffa2c60f0bbf4cfed327aa298', 'rust/src/theta_v0/strategy/coverage/held.rs': '3248c93058b1f1127155a698fb0b1260fb9e6cfcc568316af8901965b058c9c0', 'rust/src/theta_v0/strategy/coverage/held_tests_1.rs': '1e1ec83f1f935c530370a77add83c2fcc8c3d379a6e13c5dbf5718b05ac76f01', 'rust/src/theta_v0/strategy/coverage/held_tests_2.rs': '14f88a4371649892db6284977e555192f503c72dc8dd51db6fd7505c93bb3727', 'rust/src/theta_v0/strategy/coverage/leg.rs': 'c2bd2ee715b4913d328c2096420b63da7cabe7bc6514fa60313185bf44cf31ab', 'rust/src/theta_v0/strategy/coverage/leg_tests.rs': '73021c33f3f2c73c509528c951c1f20c3bb58736c4fcfcc62855affa36688026', 'rust/src/theta_v0/strategy/coverage/mod.rs': 'a6639bda3cdbda589e47e18eba5a3210f63503bebb26d443ec0d577ff7520ca3', 'rust/src/theta_v0/strategy/coverage/role.rs': '1c49d26c030eef202ab3e4948006d46455385bef19ae2514af462031d16f5136', 'rust/src/theta_v0/strategy/coverage/role_tests.rs': '8d4472c87cece44c7bd6546c35e505b0687ebf10259a0a59b40d7fc0838467da', 'rust/src/theta_v0/strategy/coverage/sizing.rs': 'a990f632fd13e29909fd6e0feb510bb71218eb0d5e275ea1ccc7ebf2ae70f525', 'rust/src/theta_v0/strategy/coverage/sizing_tests_1.rs': '41df93391785bc097382e8abc8b7c12ce8f345407756650a61064a5d3a461e9c', 'rust/src/theta_v0/strategy/coverage/sizing_tests_2.rs': 'a4b3b151024248db82902514e80a227da21cafabc5fd7675a4a3f72decef6c1d', 'rust/src/theta_v0/strategy/coverage/step.rs': 'c9c9df763e274219b06da5f358d499c9aaf0540ed459d47445e817cea93fd7f9', 'rust/src/theta_v0/strategy/coverage/step_tests.rs': 'ceb66cceba78621fbad1d0e003dcbce4ee1ea4847f06889f2258c18bdd505bc6', 'rust/src/theta_v0/strategy/coverage/test_support.rs': 'e2f1d240d26747c4bf589eb42714b96882931d85d4926e5f4758b5577072d403', 'rust/src/theta_v0/strategy/exec.rs': 'aa720c1e8a2a87a610a7ab6a5adadda0a36429eb5a64d8d9bc0f9abab5dc6f70', 'rust/src/theta_v0/strategy/exit.rs': '79ef83b1110e9958028afff89d7f4403b7f010d27215c06a43c3b7fadbf8e092', 'rust/src/theta_v0/strategy/intent.rs': '1404d7ca7ca32dabc01564cdc34041791923e0cf146f961e91dbc792adc4bae0', 'rust/src/theta_v0/strategy/interp.rs': 'b9b41a0252f31f788e5837bae484b467ddc1afafe25bd1116c7bc08437221a11', 'rust/src/theta_v0/strategy/interp_cert.rs': '6e25815cbdd9529b963c4f61de812e3d15378b9d21f0b6026f1476d00ec01f09', 'rust/src/theta_v0/strategy/ledger.rs': '7c218a26af9a7ad0fd2cbe5cfc51511cc3dc98a402541281ab3590280c911f24', 'rust/src/theta_v0/strategy/level_attrib.rs': '689afe3aa69e01a1dd2888154e31646063e2e7aaae69e19cbcf825557961048f', 'rust/src/theta_v0/strategy/level_clock.rs': 'b6f1766cbebc9df82ecb22447e21e7463fbab7e3eb5af55c1255ca5a835c459b', 'rust/src/theta_v0/strategy/level_ledger.rs': 'a7ee2d57a6f6be712d3e7c8339eade951ac877773927a73be43c00938c7e788d', 'rust/src/theta_v0/strategy/level_order.rs': '52e1900c8f17cdd226a391baa9704f3cbbfb0039e3d00a150c474de57270744c', 'rust/src/theta_v0/strategy/level_risk.rs': '98cc92f244de4a06f4d8f31e90cf9cae5dced3044b80f4e991c3b906094aba9b', 'rust/src/theta_v0/strategy/mod.rs': 'eec96eef96cbe55cd032166cfa42cfa7df1bb28399948afa6a3c5f33d84665a2', 'rust/src/theta_v0/strategy/mutex.rs': '885c19127f52348ef69bede467fb94fcee422e8b631d6e0b346e05bca6dd77f9', 'rust/src/theta_v0/strategy/nest.rs': '7a978e644f956d50f0850152368c7605d2e08b34c49d238227c2bd44ebc84623', 'rust/src/theta_v0/strategy/oscillation.rs': '46b45a5b0a3a1c8c7d480008af94953bb0a62040fbb905f1abe13dca87f2f8ff', 'rust/src/theta_v0/strategy/oscillation_campaign.rs': 'd4fe125da343c9c028dc5446b71e2ab2a87a6b9a4a46a215311b7a5c64bdebd1', 'rust/src/theta_v0/strategy/overlay_state.rs': 'fc4195a1a279ff9ab7a54f79156ba0ca32b9bd1c38608d19d4ab413dc65a3ea5', 'rust/src/theta_v0/strategy/persistent.rs': '3b41df94cd8b66636f2096fe25d938927c7002c80f971bbdcd54a403dc33e604', 'rust/src/theta_v0/strategy/protocol.rs': '87802100416db0531fc887a5d196dfea393bcc3a5415084440b22fda743c5121', 'rust/src/theta_v0/strategy/risk.rs': 'b1df1b6a9ca96dd6d3c76c529c19d567ab83d89962c3ec7bccd99be8b2d65222', 'rust/src/theta_v0/strategy/shadow.rs': '9e5d22cba18269ec647fc70f721549508f28d914fbf67336eab37d4d9dd7c51a', 'rust/src/theta_v0/strategy/short_diff_bucket.rs': '8bb0936deec2b26c03c67338d78a42809cbf0549d0e359d007722cc9c1f36f3d', 'rust/src/theta_v0/strategy/voice.rs': 'e3b14ca4a25d0b753bc1cbbdf1e94ee26a3d2f3016e3162a5c8da1bd4aa42bd5', 'rust/src/theta_v0/stream.rs': '7363d915f0908e7c25771c79ae9a5366438e5aca052f338a570103bf8d2014e1', 'rust/src/theta_v0/types.rs': '4370651d94e1be89d83d26c2d8ab00a7f7cd961693f071749afb10bf95f6bca0', 'rust/src/theta_v0/venue_fee.rs': 'c07ef8d7df9c32fab117731e4e783c68e2056e3b32666916ee0f94f40088ec82', 'rust/src/theta_v0/venue_fee/datum_io.rs': 'cef7a6a133fba712d19ae0613f16721e960a0fc7ef862575ec4564ca447025b4', 'rust/src/trading/allocator.rs': '2e35ebf483f4de7a76d5002cf9bea44a4ca2d8fc63f98c3c22ae5856cd87a559', 'rust/src/trading/axiom_voice.rs': '75b769f167083bdf40c03a797e53c58d7e98406b5b2859954b6195857d914310', 'rust/src/trading/center_book.rs': '00e1c36e48a5b417528736c63868f184dae857d0b62a7e337e0f812b55f231a2', 'rust/src/trading/config.rs': '4eef382fdfe264b9f499c9dc38d2cdb5a6e0e5867dbe9498d89c21abed71054e', 'rust/src/trading/consolidation_ablation.rs': 'e0addc59627a81fd7a2bd9ae153a63fb2a2d21568ebf7b18133b7844996e2ffc', 'rust/src/trading/depth_ref.rs': '75e24539dfc28702c5474a58f4d69c673745bcd23fd8dd8a8f505b6097406333', 'rust/src/trading/dual_voice.rs': 'f9fa0684b23cb3bdbec2eb40d169242b94ea87b1bbba6737d3621cd86db9937d', 'rust/src/trading/fatigue_gate.rs': '317ced1e7898fb4983be9bf5370b73112e9e00f1969a488f17b1b18d000ef2d2', 'rust/src/trading/gate_state_dump.rs': '6af5ac90f39c90b1538fbf9bc600f7611a0cbebed87be13bf585c3b2727d6764', 'rust/src/trading/graph_chain_destack_parity.rs': '2c32290f3dd262f32f6aa2b0ec8971fde4ebf1c5b346fb26d02f06756c744e50', 'rust/src/trading/isolated_fugue.rs': '2b6de2ec735fb99aaef399a2fad2c3882e71f136059d58bbc6d745a91af9c544', 'rust/src/trading/ledger.rs': '60423d2234d84249e8bf7c12d64cf238608b500066caa9a502fa25d1c82b12f0', 'rust/src/trading/level_operating_unit.rs': 'c61392edb5111acf33ff42b5d1e283205362bff91b9aa3f02b9581618e9dcec4', 'rust/src/trading/level_operating_unit_tests.rs': '7e9a2b0be57a928993be620593b56474a29e23f41b72981f98214e9c10bb0de2', 'rust/src/trading/level_sub_lou.rs': '2c2cc6ecd51b21332aab1ad1d967aab2bd6ec143bd2b12c41fd244b771eaa0a0', 'rust/src/trading/master.rs': 'e44af71eef9db29f617bd76fc66054af0a9f26bc0af712a6a9dfa75a4976d871', 'rust/src/trading/mod.rs': 'a6977d4f735b36b44857e4c80e0b6b41260370c6c48bf0179bf5ffb6bb5995eb', 'rust/src/trading/nested_fugue.rs': '51b41430b8f5aba670b4162a7879844be2dcb599b6838b5f6a4c59a91bdfe93d', 'rust/src/trading/nested_interval_fugue.rs': '047c60092cb00b7149c8a7e3351ea0350bae9802fcb15d4e114bc296be880334', 'rust/src/trading/positional.rs': '144c1d817e7a2ab94bea24c2dabbd0df4ac3cbf68b7d338480c69a86ac95ef61', 'rust/src/trading/positional_fusion.rs': 'dd54cf7f28efcc5cf8caad5f91309eb9f0018b0f611d2fdf1464cef0cd64434d', 'rust/src/trading/positioning_chain_fugue.rs': '4c5d20e390ff5695b21453991ab71a5724c898a5052a0480a58b65c2c9b41234', 'rust/src/trading/recursive_nested_fugue.rs': '615736baadec8bb4bfbc7c87fedb21f520a58146e83d7641e0dc6f27e3946cff', 'rust/src/trading/recursive_position.rs': '53488e40fac2f709346da742b3cc1944ed0bebad2ac9c07ef2e67cf446722f9b', 'rust/src/trading/runner.rs': '5382678afbf8b1a7ab2a5434444b15254d93eba0ba83af3eae69559307eaccd3', 'rust/src/trading/sublevel_confirmation_ablation.rs': '1d8351b53413164099ee99a0e56d8626e8594a44848406fac9875a7d74f9c39c', 'rust/src/trading/tape.rs': '5fdc01ac3d1b2c8093c61b77abac1c2a802c8bc8f7cc77ee2699d2767fa389b5', 'rust/src/trading/third_point_book.rs': 'ed9a715c44d4e79887a0732e21e440435a3ea66f5b2371303606cbed3e9ad243', 'rust/src/trading/trade_behavior.rs': '806f8e998ef2c7d7733e23702aa51e4e010c5d1b584435dd3c058f416fc3ea61', 'rust/src/trading/trend_exhaustion.rs': 'c4fe60c278e1c88ae10ec05dd0232898a212906fced02c1ebe4245ee47f745dc', 'rust/src/trading/types.rs': 'b372e7a8ce068f64e5089a1c28785c066cf3a4d7bd7d0f2affcbb5349e3acccb', 'rust/src/trading/unified_necessity.rs': '4dc44a067e9f994cb6f96fbb83ca7d40b3456267bc7bc50a3170e9ff1343757e', 'rust/src/trading/unified_osc.rs': '3ddb150fd48c34b3057f4346feeeb0c4d78e96169664a8f9bcee1f309ceac2f3', 'rust/src/trading/unified_recursive.rs': '0f67c7f9d0bdb1bf18dc8b1b7ba0632b9d97144919f39800a0f0b436f1ff4ead', 'rust/src/trading/unified_voice.rs': '48f187091a1ebbf150c094a31212df5e09d589d935ef09a44084fe039bcc3c6a', 'rust/src/zhongshu.rs': '45970e594510e14c5f7feaaebfa21461b16ae8f17add35f925a29da8b40a1803', 'rust/tests/bsp_predicate_lean_parity.rs': '90daecd0a7504447c271f48b3922aa4ef06beaee5f75d833e4adc59d54432098', 'rust/tests/certificate_chain_single_source.rs': '7755d071393a030730a28b982bc72e67db45b54f3667962bf5a40143e0d866fe', 'rust/tests/econ_oddeven_diagnosis.rs': 'd7d273632b8469db540f86223485b92a4feab189305b0e92fa701e7a984c9a3b', 'rust/tests/fixtures/btc_agg_trades_20000.json': 'c37b8cedf722d443787f76a3b8717369ab7769610621fbacca60b79ea86f9fe7', 'rust/tests/fixtures/center_pair_1424_lean.json': '0802c65c41a20a268b7d3e7bd7f94cc12def98fb5dbb357dbb518cd70d662c78', 'rust/tests/fixtures/center_pair_1424_worked_examples.json': '3dcbb7b09cb7e7c5e539b3232ee222bcf23ba44ec87bb8cfcb5bffae8e41821c', 'rust/tests/fixtures/certificate_chain_parity.json': '99338ec969dd9d94c78431d5e969721cd9ea6754ad66b27c255385bdeeead040', 'rust/tests/fixtures/issue533_m8_p3fold_tower_events.golden.jsonl': '83f45a36ab422a92d198d5d9289c40db94e95fe2cba7af2322e2d37aa8e718c2', 'rust/tests/fixtures/issue533_m8_p3fold_trades.golden.jsonl': '8196a2941bd3560f4af2adaec3609079291302eaa2aede3bbcf3fda1ad23d5e5', 'rust/tests/fixtures/issue533_m8_wf7_tower_events.golden.jsonl': 'aa96b3e836be5a43514c425d572c5312d15d8c6950d991d094416e4d61af1cbb', 'rust/tests/fixtures/issue533_m8_wf7_trades.golden.jsonl': '868e415081c78520e45fbd876681022960de02cc80c4124ac27ddb93a8ffbf01', 'rust/tests/fixtures/issue533_m8_wf8_tower_events.golden.jsonl': '1d8dff0d29e8925fd2931e05259fad11b71745354482c413f4138d8123dfb34f', 'rust/tests/fixtures/issue533_m8_wf8_trades.golden.jsonl': 'b265c52ed27967d6f6c0601db814af7d482347344588e830fbfab858098eb2ad', 'rust/tests/fixtures/issue533_p123_100000.sha256': '469b14100b2e83b5edf9ddecbfc4ed701c7c3b8700050644ec4c0ace0e5788b3', 'rust/tests/fixtures/issue533_p123_100000_p116.golden.txt': '8a7327feb3b9ba29f69fa824af1f1ecc137d9303637b47ad8465b6d638705f84', 'rust/tests/fixtures/issue533_p123_20000.sha256': '6345dc8122bc2fd421186819dba0f848ba0f89ca39be4586dd55827a9e1a576f', 'rust/tests/fixtures/issue533_p123_20000_p116.golden.txt': 'fcc8016a9a01a1098736b9ee3b348614296bb97aec817a5c172c9a0723427a40', 'rust/tests/fixtures/issue533_p123_2000_dump.golden.txt': 'e52d80c675fd00f56b3cfc64d83a01d503233f1d59743d89a530f4b4f1602585', 'rust/tests/fixtures/issue533_p123_2000_p116.golden.txt': 'a1445d215fb69472b3df88a8599ae4d52cff1b62cdb5cb222ad58d16acf6a7a6', 'rust/tests/fixtures/issue533_p123_2000_stdout.golden.txt': '8e0aef6dd802dbf12e7d2469929ac08f4af174a4437475817e6c8e604f9e9edf', 'rust/tests/fixtures/issue624_retrace_ledger_events.golden.jsonl': '76f84367639b9c734439ba2f717f31bbf732ffc44fdfe7bfaaa46d1609571f45', 'rust/tests/fixtures/tb02v_same_price/PROVENANCE.md': '5d007fd4315d8ed4d34de37f424f9317c86f00121be14b30951bc033fd6323bc', 'rust/tests/fixtures/tb02v_same_price/real_A_bars14_30_formed_then_popped.json': 'b4eaec3ad3d1fc3b0c925bbd4e51088051dc1320a91711880d0663c72a678859', 'rust/tests/fixtures/tb02v_same_price/real_B_bars25_30_minimal_eq.json': 'e3c30584b1c55cc0b253ebed114a024a093231a27de3e955ecd4ac2ecb426dd5', 'rust/tests/fixtures/tb02v_same_price/real_C_bars14_40_multi_root_second_gate.json': 'e5dff3622d8d2d42b029c62127f59b6a735170bf4f0b04f03cc3d7a5087c5779', 'rust/tests/fixtures/tb02v_same_price/syn1_same_kind_equal_price.json': '47a800ee471185cf304ff9e883cb91209df1ec133d1c1bc2b0774864ef155b77', 'rust/tests/fixtures/tb02v_same_price/syn2_multi_root_group.json': '1c22bdee093569fcfc9cf73e1fb2efba76cfd32cf77ac273940a197906ace9e0', 'rust/tests/fixtures/tb02v_same_price/syn3_multi_root_replace.json': 'ad67f939ed11ea214f922c6f497033a6ab6aac9ce1ea08b820665dd4c44a8412', 'rust/tests/fixtures/theta_v0_center_parity.json': 'ce3008233d89a2c49bf52bd8ec887e3bc004a188dae12a405d64a1c39e055d21', 'rust/tests/fixtures/theta_v0_parity.json': '7b25bab963711956de6d100d61a2070aa5ef4947e3ceb851c1e5f2ceba1df74b', 'rust/tests/issue1087_scan_lean_parity.rs': 'e38c8a94a9adea19190dab7ec0d7a550423d85984eee6659fe35d44145a8737e', 'rust/tests/issue533_p123_byte_guardrail.rs': '0a82bcad51a47b90388c0f308f4bf43a9bec6f396da2f18f611524ba228eac6a', 'rust/tests/nest_isolation_guard.rs': '56d1573ad9fd0bca40c07dc6a232f3f1b208bf2de82f673f14614a723b1575dd', 'rust/tests/q_owned_strict_decoder_contract.rs': '1857268a53b09bac40e4f69213d39ac9f0afd9734dce749a5f99465dd6d735a3', 'rust/tests/s_content_array_scan.rs': 'a9ca4c23a6b2db8fa05354703dafd98f6d4945f881f557ccbb923cbd4fa6245f', 'rust/tests/s_content_contract.rs': '3d123b96585272bd702987ffba328fee668340662ed619aa2547886ab5d35e50', 'rust/tests/s_content_fragment.rs': '4e1b757f128674d6e14a03c7823b5ff1a229b27930658aada59c150035e45836', 'rust/tests/s_content_job.rs': '45a688fed16a603452cd6e1e7bb89e913a65d5efa6defe57a0d368465cbe0cba', 'rust/tests/s_content_key_subset.rs': '1841d380bd4c178b7b51ef2e6937b17468b0fffa8e217cc006e731b16c4ba9a8', 'rust/tests/s_content_path.rs': '6b13ec018d0cce607fe31fffb79fbee95ef4ab660179a4a517f48f04b9604fa0', 'rust/tests/s_content_sequence.rs': '18454a4cbcf19413acda48134c53a6e5ac2466d5eddbbc45305ab59198f86ec5', 'rust/tests/s_content_wire.rs': '6ed8d5f7b7683e7f22dafd20cdc6cd3a4ced55e90a674bedc8e894610c116c59', 'rust/tests/theta_pi_stream.rs': '1903ed2182db01b646274f0fe433203a9469e18338f30e783e3cf77eb214c660', 'rust/tests/theta_v0_07b_gating.rs': 'a97bc7e17ada120ac20a0cdf0d520bf13640303d0a88056f700303f6ad7d1497', 'rust/tests/theta_v0_buy_parity.rs': '96506ab1299c115d49d829360a82daa6464092d9873fd09e4d24aabbe6e724a6', 'rust/tests/theta_v0_center_parity.rs': 'a6c069ca6bb19df1299293bd676fff7dc313ab73d48aee16327a67c61610e16b', 'rust/tests/theta_v0_classifier_parity.rs': 'a95c66b2fc557aa20122ac608f945dc0733436cbca1db7cb1d310d0a29f520d0', 'rust/tests/theta_v0_fixture_drift.rs': '93b36937a15d29a93aff3e44a1a599f3c500952d6481ac2937e0001770b5922f', 'rust/tests/theta_v0_lean_parity.rs': 'c0629878b394a27ba49bc0d2981cced8ed5756d8a2ead3031a98b4f6366d2fd6', 'rust/tests/theta_v0_perf_profile.rs': 'eec91c2a97e9a89670a5fdf5ca54d7ada96cd753693f83a4c18e7cfb934658af', 's_session/tests/fixtures/q3-complete-onebar.json': 'ce4efe9dcaac287abebdf1bd4c647d5c1a264482faaa86f05c087b22c17af9c2', 's_session/tests/fixtures/q3-complete-s16-cuts345.json': '0ad66776f85000abdbbe8d3e8d12965f053545496a9e1f517210c2ce022c8338', 's_session/tests/fixtures/r12_consumer.json': '1b0020a69a897da000056666907ef606a6d0b2ed35bac97ec65d625131e25745', 's_session/tests/fixtures/tb01c/MANIFEST.json': '6e41985da0791592163187a454d24651493843369689862320b4f507aa069b29', 's_session/tests/fixtures/tb01c/RUNTIME-MANIFEST.json': 'ce8b7dc5cdbf8436b69bdcdcf6de4b268503b21f4f279536d33d57af0fae2991', 's_session/tests/fixtures/tb01c/clock-plan.json': '3d7d0470f9e6e358e645b3e757b85ed212d45bb0024f9a90b6919bebc4001709', 's_session/tests/fixtures/tb01c/live-input.json': 'c6d5b7bbdad2b4e3be0980159b2810fee0a1cc460b9395deb23ece79d876345f', 's_session/tests/fixtures/tb01c/messages.jsonl': '29a202bf4a94e69ed503db54e1e30c52d052616447bbd1c4f37488169cc8ac58', 's_session/tests/fixtures/tb01c/operation-schedule.json': '34904c1c774ab5088ea43870b204a999b48234b6cc629de301af705d6d8e6f87', 's_session/tests/fixtures/tb01c/operation-schedule.jsonl': 'daffb9ecf2c63d757bca49dbc34dc65bf926c4f4394078ff578bc3f0602f5b88', 's_session/tests/fixtures/tb01c/profile.json': '2cd50e43e659dc87498eebba76d88deb4cde276cd51c47fa0c083b07dffbdcfc', 's_session/tests/fixtures/tb01c/query-resources.json': '36b188ae42be8e24e7fdbb44e7d3a7068200f7c942f50975337d07595ac62e64', 's_session/tests/fixtures/tb01c/raw-input.json': 'b83542e6ad6af3a4da27768601c7f9a2e019a6cae77e4c8785e5f0e9edf6772a', 's_session/tests/fixtures/tb01c/raw-input.jsonl': 'bc94f1e7fddde4c726fac9b60a12178420cd55fb37a4e9a1c598ef20bd18eb1d', 's_session/tests/fixtures/tb01c/runtime-plan.json': '18e9b3cd6cb5f17ba715b09405995b68b627bcd696ba457de790dfa9a51125c3', 's_session/tests/fixtures/tb01c/seed-input.json': '423dc00069462ba23fdc32c07310d1553f29e3e226893814df5185efff31b23f', 's_session/tests/fixtures/tb02a/ORACLE.md': 'b27838118952c418fc16b8d0d67c4a7dc9be316f6c4bb1253e9d514fcfd2cd9b', 's_session/tests/fixtures/tb02a/hand-oracle.json': 'dae7ab55694f858e4c52c642d31f405459fa8f004a9f952efd670bda3c5f7ff2', 's_session/tests/fixtures/tb02a/raw-ledger.json': 'b321c037723cc00d597b441cd5eca6736249240d812dde2ede03e28e44fca7bf', 's_session/tests/fixtures/tb02b/ORACLE.md': '1b5fd9c3532f5f93b03a397917bd06dc64792d4f62ebc74ef50696c818dd138f', 's_session/tests/fixtures/tb02b/hand-oracle.json': 'b152d9afb7a4758179ae086674060e0aa409b84c0a8fb50585a10bc64ed29d1e', 's_session/tests/fixtures/tb02b/independent-source-oracle.json': '0450031d1201f64e1cfcd5f347a2597070e6c61f31f9376434382ac09245ac56', 's_session/tests/fixtures/tb02b/raw-ledger.json': '0caafdb71b5df667be3c58781aa28011e5ec949bb128686623929c63078c5169', 's_session/tests/fixtures/tb02c/ORACLE.md': '2e68be25a816f48d4b8251f57a54e05cfb1e98f45126564ebfc9085691ba4146', 's_session/tests/fixtures/tb02c/contract-seed.json': '7d2bb7a2939cb49aff5b722c0a131e6ec715763309510f02cec012f46ce21cd9', 's_session/tests/fixtures/tb02c/first-cut-logical-slots.json': 'e924233c77661f4cd8eade8695276fd351b74269d4221b9bbc54b6faa8dc03f7', 's_session/tests/fixtures/tb02c/hand-oracle.json': '0be95af141b6b4d42de4e5c84f6cab165af67eb859928f096be51381fd90b332', 's_session/tests/fixtures/tb02c/raw-ledger.json': '32d00df0b3e349411b79e34e284242bce5b63dce97a03b6cb2eb17a8dec27aa4', 's_session/tests/fixtures/tb02d/MANIFEST.json': 'f056451baa3a149f6cdf4b7ad92538666234f817b4271587c166056a0d37f3ae', 's_session/tests/fixtures/tb02d/generate-input.py': 'a8ef0768d2f626d671f26d2e4d832d3635dbc1f708d2cbba5ee5bd416b3cdd16', 's_session/tests/fixtures/tb02d/hand-oracle.json': '9140b7827d958f3e558100c124abe9f31f314f63b7e3e7295759dddfc239e454', 's_session/tests/fixtures/tb02d/raw-ledger.json': '8040caa6ab4e10a7fcc374ec6e71c6a28af9553fb16fa452a727bccd7b6bc377', 's_session/tests/fixtures/tb02e-real/btc-tail50k-first600.json': '0df2091a8723b040988527bcd05612c9643530f9724be68edb52acbcf77c108a', 's_session/tests/fixtures/tb03b-changes-only-browser.json': '56c9ae63748d8fc1668ad214b4e10adaa42d944d616c6a1eb2ca94edc2716b1d'}


def clock_ns() -> int:
    """与 Rust libc CLOCK_MONOTONIC 使用同一显式时钟域。"""
    return time.clock_gettime_ns(time.CLOCK_MONOTONIC)


def clock_seconds() -> float:
    return clock_ns() / 1_000_000_000


def remember_signal(signum: int, _frame: object) -> None:
    CANCELLATION["count"] += 1
    if CANCELLATION["first_signal"] is None:
        CANCELLATION["first_signal"] = signum


def check_cancelled() -> None:
    if CANCELLATION["count"]:
        raise Cancelled("监督器收到 SIGTERM/SIGINT")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def save(path: Path, value: object) -> None:
    # 同卷原子且不覆盖已有回执；中断不得暴露半份 PASS。
    temporary = path.with_name("." + path.name + ".tmp")
    created = False
    try:
        with temporary.open("x") as out:
            created = True
            json.dump(value, out, ensure_ascii=False, indent=2)
            out.write("\n")
        os.link(temporary, path)
    finally:
        if created:
            temporary.unlink(missing_ok=True)


def group_exists(pgid: int) -> bool:
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False


def reap(proc: subprocess.Popen, deadline: float) -> dict:
    """未 reap 的直属 leader 固定 PID/PGID；已回收或消失后绝不补发信号。

    SIGCHLD 必须为默认，单线程且无其他 waitpid 回收者。TERM 后不能 poll/wait，
    保留 leader（包括 zombie）直到最后一次 group signal，防止 PGID 复用。
    正常 wait 已回收的 leader 只查组消失；残留归属不明即失败保留原件。
    """
    receipt = {"group_reaped": False, "leader_reaped": proc.returncode is not None,
               "signals_sent": [], "ownership": "unreaped_direct_child"}
    if proc.returncode is not None:
        receipt["ownership"] = "leader_already_reaped_no_more_signals"
        receipt["group_reaped"] = not group_exists(proc.pid)
        return receipt
    disappeared = False
    for sig, grace in [(signal.SIGTERM, 2.0), (signal.SIGKILL, 8.0)]:
        if disappeared or clock_seconds() >= deadline:
            break
        # 不调用 poll/wait；在这里 returncode 为空的 leader 仍由本监督器唯一持有。
        try:
            os.killpg(proc.pid, sig)
            receipt["signals_sent"].append(int(sig))
        except ProcessLookupError:
            disappeared = True
            break
        except Exception as exc:
            receipt["signal_error"] = str(exc)
            break  # 归属/权限异常后不再发送下一信号；仍尝试 wait 直属子进程。
        if sig == signal.SIGKILL:
            break
        end = min(deadline, clock_seconds() + grace)
        while clock_seconds() < end:
            try:
                if not group_exists(proc.pid):
                    disappeared = True
                    break
            except Exception as exc:
                receipt["signal_error"] = "group observation: " + str(exc)
                break
            time.sleep(min(0.1, max(0.0, end - clock_seconds())))
        if "signal_error" in receipt:
            break
    remaining = deadline - clock_seconds()
    if remaining > 0:
        try:
            proc.wait(timeout=remaining)
        except subprocess.TimeoutExpired:
            receipt["ownership"] = "unreaped_direct_child_cleanup_timeout"
            return receipt
    receipt["leader_reaped"] = proc.returncode is not None
    # 此后只能观测，不能重新对历史 PGID 发送任何信号。
    while receipt["leader_reaped"] and not disappeared and clock_seconds() < deadline:
        if not group_exists(proc.pid):
            disappeared = True
            break
        time.sleep(min(0.1, max(0.0, deadline - clock_seconds())))
    receipt["group_reaped"] = receipt["leader_reaped"] and disappeared and "signal_error" not in receipt
    return receipt


def supervised(name: str, argv: list[str], cwd: Path, env: dict,
               deadline: float, cleanup_deadline: float, artifacts: Path,
               *, allow_cancel: bool = True) -> dict:
    started = clock_seconds()
    proc = None
    result = {"argv": argv, "pid": None, "exit": None, "timed_out": False,
              "group_reaped": True, "cancelled": False, "cwd": str(cwd),
              "started_monotonic": started, "deadline": deadline,
              "cleanup_deadline": cleanup_deadline, "pgid": None}
    try:
        if allow_cancel:
            check_cancelled()
        require(clock_seconds() < deadline, name + " 启动前期限到达")
        with (artifacts / (name + ".stdout")).open("xb") as out, (artifacts / (name + ".stderr")).open("xb") as err:
            # 信号只记录：Popen 返回与登记之间也不会跳过登记/回收。
            proc = subprocess.Popen(argv, cwd=cwd, env=env, stdout=out, stderr=err, start_new_session=True)
            OWNED_GROUPS[proc.pid] = proc
            result["pid"] = proc.pid
            result["pgid"] = proc.pid
            result["group_reaped"] = False
            while True:
                if allow_cancel:
                    check_cancelled()
                remaining = deadline - clock_seconds()
                if remaining <= 0:
                    result["timed_out"] = True
                    break
                try:
                    result["exit"] = proc.wait(timeout=min(0.1, remaining))
                    result["timed_out"] = clock_seconds() > deadline
                    break
                except subprocess.TimeoutExpired:
                    continue
    except Exception as exc:
        result["error"] = str(exc)
        result["cancelled"] = isinstance(exc, Cancelled)
    finally:
        if proc is not None:
            try:
                cleanup = reap(proc, cleanup_deadline)
                result["cleanup"] = cleanup
                result["group_reaped"] = cleanup["group_reaped"]
            except Exception as exc:
                result["reap_error"] = str(exc)
            if result["group_reaped"]:
                OWNED_GROUPS.pop(proc.pid, None)
        if proc is not None:
            result["exit"] = proc.returncode
        result["ended_monotonic"] = clock_seconds()
        result["elapsed_seconds"] = clock_seconds() - started
        result["signals_observed"] = dict(CANCELLATION)
        save(artifacts / (name + ".json"), result)
    if allow_cancel:
        check_cancelled()
    require(not result.get("error") and not result["timed_out"] and result["exit"] == 0
            and result["group_reaped"], name + " 失败/超时/进程未回收")
    return result


def read_small(path: Path) -> dict:
    with path.open("rb") as stream:
        raw = stream.read(1048577)
    require(len(raw) <= 1048576, "监督器只读取有限元数据")
    value = json.loads(raw)
    require(isinstance(value, dict), "元数据必须为对象")
    return value


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def regular_files(directory: Path):
    require(stat.S_ISDIR(directory.lstat().st_mode), "原件根不是目录")
    with os.scandir(directory) as entries:
        children = sorted(entries, key=lambda item: item.name)
    for child in children:
        path = Path(child.path)
        mode = child.stat(follow_symlinks=False).st_mode
        if stat.S_ISDIR(mode):
            yield from regular_files(path)
        else:
            require(stat.S_ISREG(mode), "原件内含链接/特殊文件")
            yield path


def load_legacy(root: Path):
    path = root / LEGACY_RUNNER
    require(digest(path) == LEGACY_SHA256, "旧核验器身份变化")
    spec = importlib.util.spec_from_file_location("driver1488_legacy", path)
    require(spec is not None and spec.loader is not None, "旧核验器无法加载")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def gate() -> None:
    require(CONTRACT_READY, "CI 合同未独审，禁止实际运行")
    require(os.environ.get("GITHUB_ACTIONS") == "true" and platform.system() == "Linux", "仅 GitHub Linux runner")
    require(os.environ.get("GITHUB_REF") == "refs/heads/codex/1419-targeted-ci-20261018", "专用分支不符")
    require(os.environ.get("GITHUB_EVENT_NAME") == "workflow_dispatch", "仅专用 workflow_dispatch")


def git(root: Path, deadline: float, *args: str) -> str:
    remaining = deadline - clock_seconds()
    require(remaining > 0, "git helper 已过期限")
    # helper 的 Git 子进程继承 helper 的组，外层监督器负责整组回收。
    return subprocess.check_output(["git", *args], cwd=root, text=True,
                                   timeout=min(15, remaining)).strip()


def identity(root: Path, deadline: float) -> dict:
    gate()
    head = git(root, deadline, "rev-parse", "HEAD")
    require(head == os.environ.get("GITHUB_SHA") == os.environ.get("EXPECTED_SHA"), "执行 SHA 不符")
    require(not git(root, deadline, "status", "--porcelain", "--untracked-files=all"), "checkout 不干净")
    require(git(root, deadline, "rev-parse", REVIEWED_SOURCE + "^{tree}") == REVIEWED_TREE, "已审来源不符")
    git(root, deadline, "merge-base", "--is-ancestor", REVIEWED_SOURCE, head)
    changed = set(git(root, deadline, "diff", "--name-only", REVIEWED_SOURCE, head).splitlines())
    require(changed == CI_FILES, "源码尾差必须恰为两份 CI 文件")
    sources = {name: digest(root / name) for name in SOURCE_FILES}
    require(sources == SOURCE_FILES, "冻结 Rust 与 fixture 源码身份漂移")
    require(digest(root / LEGACY_RUNNER) == LEGACY_SHA256, "旧核验器身份漂移")
    return {"head": head, "tree": git(root, deadline, "rev-parse", "HEAD^{tree}"),
            "reviewed_source": REVIEWED_SOURCE, "reviewed_tree": REVIEWED_TREE,
            "source_files": sources, "legacy_runner_sha256": LEGACY_SHA256,
            "ci_files": {name: digest(root / name) for name in sorted(CI_FILES)},
            "run_id": os.environ["GITHUB_RUN_ID"], "attempt": os.environ["GITHUB_RUN_ATTEMPT"]}


def resource_observation(target: Path, artifacts: Path) -> dict:
    disk = os.statvfs(artifacts)
    memory = Path("/proc/meminfo").read_text()
    size = sum(path.stat().st_size for path in regular_files(target)) if target.exists() else 0
    return {"available_disk_bytes": disk.f_bavail * disk.f_frsize,
            "target_observed_bytes": size, "target_observation": "阶段采样，非连续峰值或硬界",
            "memory_raw": memory, "os_memory_hard_cap": False,
            "native_16mib_8mib_budgets": "UNCHANGED"}


def material_diagnostics(directory: Path) -> dict:
    rows = []
    for channel in ["stdout", "stderr"]:
        path = directory / ("test." + channel)
        with path.open("rb") as stream:
            for number, raw in enumerate(stream, 1):
                if b'NativeOwnerMaterialFailure' not in raw:
                    continue
                record = {"channel": channel, "line": number,
                          "raw_line_hex": raw.hex()}
                try:
                    value = json.loads(raw)
                    require(isinstance(value, dict) and value.get("diagnostic") == "NativeOwnerMaterialFailure",
                            "诊断行不是独立 JSON 对象")
                    fields = ["stage", "flags", "resource", "requested_bytes", "cap", "bytes", "peak",
                              "owner_seal", "parent", "scope"]
                    record["values"] = {key: value.get(key) for key in fields}
                    record["missing_fields"] = [key for key in fields if key not in value]
                    record["missing_flags"] = [key for key in ["unexpected", "overflow", "resources", "set", "stages", "autocommit"]
                                               if key not in value.get("flags", {})]
                except (RuntimeError, ValueError, TypeError) as exc:
                    record["parse_error"] = str(exc)
                rows.append(record)
    return {"rows": rows, "count": len(rows), "innermost_first_error": "UNAVAILABLE_UNLESS_RAW_REPORTED",
            "absence_means": "未观测到该行，不推断成功或额度不足"}


def receipt_status(receipt: dict) -> str:
    require(receipt.get("pid") is not None and receipt.get("pgid") == receipt["pid"], "缺真实直属进程身份")
    require(receipt.get("group_reaped") is True and receipt.get("cleanup", {}).get("leader_reaped") is True,
            "残留组或 leader 未回收，必须停止")
    require(not receipt.get("cancelled") and not receipt.get("signals_observed", {}).get("count"), "取消，必须停止")
    require(not receipt.get("error") and not receipt.get("reap_error"), "监督器故障，必须停止")
    require(type(receipt.get("exit")) is int, "没有真实 wait 退出码")
    if receipt.get("timed_out"):
        return "FAIL_TIMEOUT"
    return "PASS" if receipt["exit"] == 0 else "FAIL"


def helper(spec: dict) -> None:
    """所有 Git、完整 hash、大日志/目录遍历均在可回收 helper 内。"""
    root = Path(spec["root"])
    operation = spec["operation"]
    artifacts = Path(spec["artifacts"])
    target = Path(spec["target"])
    deadline = spec["deadline"]
    require(clock_seconds() < deadline, "helper 启动迟到")
    if operation == "inventory":
        entries = []
        for path in regular_files(artifacts):
            relative = path.relative_to(artifacts).as_posix()
            if relative not in FINAL_EXCLUSIONS:
                entries.append({"path": relative, "bytes": path.stat().st_size, "sha256": digest(path)})
        save(artifacts / "FILES.json", {"files": entries, "self_exclusions": sorted(FINAL_EXCLUSIONS)})
        save(artifacts / "FILES.SEAL.json", {"files_sha256": digest(artifacts / "FILES.json"),
                                              "count": len(entries), "self_exclusions": sorted(FINAL_EXCLUSIONS)})
        require(clock_seconds() < deadline, "全原件清单迟到")
        return
    result = {}
    legacy = load_legacy(root)
    if operation in {"identity", "before", "after", "final"}:
        current = identity(root, deadline)
        if "expected_source" in spec:
            require(current == spec["expected_source"], "checkout 与起点身份不同")
        result["source"] = current
    if operation == "identity":
        result["resources"] = resource_observation(target, artifacts)
        require(result["resources"]["available_disk_bytes"] >= DISK_MIN_BYTES, "初始可用磁盘不足 2 GiB")
    if operation == "select":
        result["binary"] = legacy.select_binary((artifacts / "control/build.stdout").read_text(),
                                                 root / "rust", target, "bin", "Native")
    if operation in {"before", "after", "final", "list"}:
        legacy.same_binary(spec["binary"])
        result["binary_unchanged"] = True
    if operation == "list":
        legacy.require_listed((artifacts / "control/list.stdout").read_text(), TESTS)
        result["listed_exact_once"] = TESTS
    if operation == "after":
        directory = Path(spec["test_directory"])
        receipt = read_small(directory / "test.json")
        status = receipt_status(receipt)
        if status == "PASS":
            # stderr 原件另留；libtest 的逐名成功和唯一汇总只能来自 stdout。
            try:
                legacy.require_passed((directory / "test.stdout").read_text(), spec["test_name"])
            except ValueError as exc:
                status = "FAIL_OUTPUT"
                result["output_error"] = str(exc)
        result.update({"name": spec["test_name"], "status": status,
                       "receipt": receipt, "diagnostics": material_diagnostics(directory)})
        result["scratch_remaining"] = [str(p.relative_to(directory / "tmp"))
                                        for p in regular_files(directory / "tmp")]
        result["resources"] = resource_observation(target, artifacts)
    if operation == "final":
        result["resources"] = resource_observation(target, artifacts)
        samples = [result["resources"]["target_observed_bytes"]]
        for index in range(1, len(TESTS) + 1):
            path = artifacts / "control" / f"after-{index:02d}.output.json"
            if path.exists():
                samples.append(read_small(path)["resources"]["target_observed_bytes"])
        result["resources"]["target_peak_sampled_bytes"] = max(samples)
        result["unreaped_groups"] = spec["unreaped_groups"]
        require(not spec["unreaped_groups"], "有残留组，不可验收")
    require(operation in {"identity", "select", "before", "after", "final", "list"}, "未知 helper")
    require(clock_seconds() < deadline, "helper 完成迟到")
    save(Path(spec["output"]), result)


def phase(t0: float, work: float, cleanup: float, cap: float) -> tuple[float, float]:
    deadline = min(t0 + work, clock_seconds() + cap)
    return deadline, min(t0 + cleanup, deadline + 10)


def run_helper(name: str, operation: str, context: dict, work: float, cleanup: float, **extra) -> dict:
    deadline, cleanup_deadline = phase(context["t0"], work, cleanup, 30)
    artifacts = Path(context["artifacts"])
    spec = {**context, **extra, "operation": operation, "deadline": deadline,
            "output": str(artifacts / "control" / (name + ".output.json"))}
    path = artifacts / "control" / (name + ".spec.json")
    save(path, spec)
    supervised(name, [sys.executable, "-B", str(Path(__file__).resolve()), "--helper", str(path)],
               Path(context["root"]), command_env(context), deadline, cleanup_deadline, artifacts / "control")
    return read_small(Path(spec["output"])) if operation != "inventory" else read_small(artifacts / "FILES.SEAL.json")


def command_env(context: dict, tmpdir: Path | None = None) -> dict:
    env = os.environ.copy()
    env.update({"CARGO_INCREMENTAL": "0", "CARGO_TARGET_DIR": context["target"],
                "PYTHONDONTWRITEBYTECODE": "1", "RUST_BACKTRACE": "1"})
    if tmpdir is not None:
        env["TMPDIR"] = str(tmpdir)
        env["TMP"] = str(tmpdir)
        env["TEMP"] = str(tmpdir)
    return env


def run_three(context: dict, source: dict, binary: dict, states: list[dict]) -> None:
    for index, name in enumerate(TESTS):
        check_cancelled()
        require(not OWNED_GROUPS, "存在未回收组，停止新测试")
        directory = Path(context["artifacts"]) / "tests" / f"{index + 1:02d}"
        directory.mkdir(parents=True)
        (directory / "tmp").mkdir()
        run_helper(f"before-{index + 1:02d}", "before", context, 2280, 2290,
                   expected_source=source, binary=binary)
        deadline, cleanup_deadline = phase(context["t0"], 2280, 2290, 180)
        states[index] = {"name": name, "status": "FAIL_INCOMPLETE", "directory": f"tests/{index + 1:02d}"}
        try:
            receipt = supervised("test", [binary["path"], name, "--exact", "--format", "pretty", "--color", "never", "--show-output", "--test-threads=1"],
                       Path(context["root"]) / "rust", command_env(context, directory / "tmp"),
                       deadline, cleanup_deadline, directory)
        except RuntimeError:
            # 只有真实保存且已回收的普通非零/超时才继续；不捕获写回执错误或取消。
            check_cancelled()
            receipt = read_small(directory / "test.json")
            status = receipt_status(receipt)
            require(status in {"FAIL", "FAIL_TIMEOUT"}, "不是可继续的普通测试失败")
        actual_status = receipt_status(receipt)
        states[index]["exit"] = receipt["exit"]
        if actual_status != "PASS":
            # 总阶段耗尽导致 post helper 未运行时，仍保留实际 FAIL/TIMEOUT。
            states[index]["status"] = actual_status
        require(not OWNED_GROUPS, "失败后仍有残留组")
        observed = run_helper(f"after-{index + 1:02d}", "after", context, 2280, 2290,
                              expected_source=source, binary=binary, test_name=name, test_directory=str(directory))
        states[index].update({"status": observed["status"], "exit": observed["receipt"]["exit"],
                              "evidence": f"control/after-{index + 1:02d}.output.json"})


def final_handler(signum: int, frame: object) -> None:
    remember_signal(signum, frame)
    if FINAL_STATE["sealed"]:
        # 封口后不可改写 RESULT；独立取消标记加非零退出共同拒绝候选。
        try:
            save(Path(FINAL_STATE["artifacts"]) / "CANCELLED-AFTER-SEAL.json", dict(CANCELLATION))
        finally:
            raise SystemExit(1)


def publish_result(context: dict, states: list[dict], result: dict, seal: dict | None) -> int:
    artifacts = Path(context["artifacts"])
    result.update({"tests": states, "failed": sum(s["status"].startswith("FAIL") for s in states),
                   "not_run": sum(s["status"] == "NOT_RUN" for s in states),
                   "passed": sum(s["status"] == "PASS" for s in states),
                   "unreaped_groups": sorted(OWNED_GROUPS), "signals_observed": dict(CANCELLATION),
                   "supervisor_sigterm_sigint_actual": "OBSERVED_NOT_FAULT_INJECTION_VALIDATED" if CANCELLATION["count"] else "NOT_RUN",
                   "test_timeouts_observed": sum(s["status"] == "FAIL_TIMEOUT" for s in states),
                   "files_sha256": seal.get("files_sha256") if seal else None,
                   "candidate_only": True, "acceptance_requires": ["process exit 0", "job success", "all original evidence",
                     "no late/cancel/finalization rejection markers", "independent remote evidence review"]})
    good = (all(s["status"] == "PASS" for s in states) and not result.get("error")
            and seal is not None and not OWNED_GROUPS and not CANCELLATION["count"]
            and clock_seconds() < context["t0"] + 2370)
    result["verdict"] = "CANDIDATE_THREE_PASS" if good else "FAIL_THREE_TARGETED"
    result["finished_monotonic"] = clock_seconds()
    try:
        if CANCELLATION["count"]:
            save(artifacts / "CANCELLED.json", dict(CANCELLATION))
        save(artifacts / "RESULT.json", result)
        FINAL_STATE.update({"artifacts": str(artifacts), "sealed": True})
        # 原子写仍可能迟到/收到信号，不能把封口前的判定带过最后一门。
        if CANCELLATION["count"] or clock_seconds() >= context["t0"] + 2370:
            name = "CANCELLED.json" if CANCELLATION["count"] else "LATE.json"
            if not (artifacts / name).exists():
                save(artifacts / name, {"signals_observed": dict(CANCELLATION), "at": clock_seconds()})
            return 1
        return 0 if good else 1
    except Exception as exc:
        try:
            save(artifacts / "FINALIZATION-REJECTED.json", {"error": str(exc), "at": clock_seconds()})
        except Exception:
            pass
        return 1


def main() -> int:
    t0 = clock_seconds()  # identity/fetch/build 之前，唯一时钟原点。
    gate()
    require(signal.getsignal(signal.SIGCHLD) == signal.SIG_DFL, "要求 SIGCHLD 默认、无外部 waitpid 回收者")
    signal.signal(signal.SIGTERM, final_handler)
    signal.signal(signal.SIGINT, final_handler)
    root = Path(os.environ["GITHUB_WORKSPACE"]).resolve()
    require(root == Path(__file__).resolve().parents[2], "必须执行当前 checkout 内 runner")
    temp = Path(os.environ["RUNNER_TEMP"]).resolve()
    task = temp / ("1419-targeted-" + os.environ["GITHUB_RUN_ID"] + "-" + os.environ["GITHUB_RUN_ATTEMPT"])
    task.mkdir(mode=0o700)  # 全新目录，不能继承前轮 target/原件。
    artifacts = task / "evidence"
    artifacts.mkdir()
    (artifacts / "control").mkdir()
    context = {"root": str(root), "artifacts": str(artifacts), "target": str(task / "target"), "t0": t0}
    states = [{"name": name, "status": "NOT_RUN"} for name in TESTS]
    result = {"scope": "#1419 three version-gate and codec tests only", "started_monotonic": t0,
              "full_234_and_ci": "NOT_REPLACED", "production_v4": "BLOCK_NOT_MERGEABLE",
              "output_byte_hard_cap": "NOT_IMPLEMENTED_IN_REUSED_SUPERVISOR; original logs preserved",
              "supervisor_os_eintr_actual": "NOT_RUN",
              "synchronous_io_hard_deadline": "NOT_QUALIFIED",
              "resource_qualification": "2GiB initial disk admission only; no OS heap/store global hard cap",
              "deadlines_seconds": {"identity_fetch": 180, "identity_cleanup": 190, "build": 1080,
                "build_cleanup": 1090, "list": 1140, "one_test": 180, "all_tests": 2280,
                "test_cleanup": 2290, "verify": 2340, "verify_cleanup": 2350, "seal": 2370}}
    source = binary = seal = None
    try:
        first = run_helper("identity-start", "identity", context, 180, 190)
        source = first["source"]
        result["source"] = source
        for name, argv in [("cargo-version", ["cargo", "-V"]), ("rustc-version", ["rustc", "-Vv"]),
                           ("rustup-version", ["rustup", "show", "active-toolchain"])]:
            deadline, cleanup = phase(t0, 180, 190, 30)
            supervised(name, argv, root / "rust", command_env(context), deadline, cleanup, artifacts / "control")
        deadline, cleanup = phase(t0, 180, 190, 180)
        supervised("fetch", ["cargo", "fetch", "--locked"], root / "rust", command_env(context), deadline, cleanup, artifacts / "control")
        deadline, cleanup = phase(t0, 1080, 1090, 1080)
        supervised("build", ["cargo", "test", "--offline", "--locked", "-j", "6", "--features", FEATURES,
                    "--no-run", "--message-format=json", "--bin", "s_structure_session"],
                   root / "rust", command_env(context), deadline, cleanup, artifacts / "control")
        binary = run_helper("verify-build", "select", context, 1130, 1140)["binary"]
        result["binary"] = binary
        deadline, cleanup = phase(t0, 1130, 1140, 30)
        supervised("list", [binary["path"], "--list", "--format", "pretty"], root / "rust",
                   command_env(context), deadline, cleanup, artifacts / "control")
        run_helper("verify-list", "list", context, 1130, 1140, binary=binary)
        run_three(context, source, binary, states)
    except Exception as exc:
        result["error"] = str(exc)
    # 每条 supervised 已消费原 10 秒清理窗口；失败后不续租、不再次发信号。
    # 未回收直属组仅留存身份，停止新进程，交 GitHub runner 外层回收。
    cleanup_rows = [{"pid": pid, "returncode": proc.returncode, "group_reaped": False,
                     "action": "STOP_NO_ADDITIONAL_SIGNAL_OR_CLEANUP_WINDOW"}
                    for pid, proc in OWNED_GROUPS.items()]
    try:
        save(artifacts / "FINAL-CLEANUP.json", {"groups": cleanup_rows, "unreaped": sorted(OWNED_GROUPS)})
        if not CANCELLATION["count"] and not OWNED_GROUPS:
            if source is not None and binary is not None:
                run_helper("verify-final", "final", context, 2340, 2350,
                           expected_source=source, binary=binary, unreaped_groups=[])
            else:
                require(result.get("error") is not None, "缺来源/二进制且无失败理由")
            seal = run_helper("inventory", "inventory", context, 2340, 2350)
    except Exception as exc:
        result["finalization_error"] = str(exc)
        result["error"] = result.get("error", "原件封存失败")
    return publish_result(context, states, result, seal)


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--helper":
        gate()
        helper(read_small(Path(sys.argv[2])))
    else:
        raise SystemExit(main())
