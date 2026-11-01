import importlib.util
import pathlib
import unittest

SPEC = importlib.util.spec_from_file_location(
    "run_1488_delta_ci", pathlib.Path(__file__).with_name("run-1488-delta-ci.py")
)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)

NATIVE = ["default", "s_capture_native_testonly", "s_incremental", "s_session"]


class FeatureClosureTest(unittest.TestCase):
    def test_native_exact_closure_accepted(self):
        MODULE.check_feature_closure("Native", NATIVE)

    def test_native_extra_feature_rejected(self):
        with self.assertRaisesRegex(ValueError, "features 不符"):
            MODULE.check_feature_closure("Native", NATIVE + ["s_allocation_json_testonly"])

    def test_native_missing_incremental_rejected(self):
        with self.assertRaisesRegex(ValueError, "features 不符"):
            MODULE.check_feature_closure(
                "Native", ["default", "s_capture_native_testonly", "s_session"]
            )

    def test_native_legacy_closure_rejected(self):
        legacy = {"default", *MODULE.FEATURES["Native"].split(",")}
        with self.assertRaisesRegex(ValueError, "features 不符"):
            MODULE.check_feature_closure("Native", sorted(legacy))

    def test_plain_and_owned_accepted(self):
        MODULE.check_feature_closure("Plain", ["default", "s_session"])
        MODULE.check_feature_closure(
            "Owned", ["default", "s_allocation_json_testonly", "s_session"]
        )

    def test_closures_cover_requested_features(self):
        self.assertEqual(set(MODULE.FEATURE_CLOSURES), set(MODULE.FEATURES))
        for mode, requested in MODULE.FEATURES.items():
            self.assertLessEqual(
                {"default", *requested.split(",")}, MODULE.FEATURE_CLOSURES[mode]
            )


if __name__ == "__main__":
    unittest.main()
