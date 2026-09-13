"""#1373/#1374：合并后的共享控制器布局与 owner/Q 接线；不是正式运行验收。"""

from contextlib import redirect_stdout
import io
import os
from pathlib import Path
import subprocess
import sys
import sysconfig
import tempfile
import time
import unittest
from unittest.mock import Mock, patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import s_service_control as control
from economic_service_control import EconomicController


class RuntimeIntegrationTests(unittest.TestCase):
    @staticmethod
    def controller(root, domain="S"):
        if domain == "S":
            return control.Controller({"db": str(root / "s.sqlite"), "python": sys.executable,
                                       "startup_timeout_ms": "2000"}, root / "s-state")
        return EconomicController(
            {"python": sys.executable, "startup_timeout_ms": "2000", "state_root": str(root / "owners")},
            {}, {"db": str(root / (domain.replace(":", "-") + ".sqlite")),
                 "socket": str(root / (domain.replace(":", "-") + ".sock"))}, domain)

    def configured_probe(self, values):
        def run(argv, **kwargs):
            self.assertEqual(argv[:2], [sys.executable, "-c"])
            self.assertTrue(kwargs["check"])
            self.assertGreater(kwargs["timeout"], 0)
            output = io.StringIO()
            with patch.object(sysconfig, "get_config_var", side_effect=values.__getitem__), redirect_stdout(output):
                exec(argv[2], {})
            return subprocess.CompletedProcess(argv, 0, output.getvalue().encode())
        return run

    def test_shared_and_framework_layouts_reach_s_and_each_owner(self):
        for framework in (False, True):
            for domain in ("S", "E", "B:fixture"):
                with self.subTest(framework=framework, domain=domain), tempfile.TemporaryDirectory() as directory:
                    root = Path(directory)
                    prefix = root / "Frameworks"
                    libdir = root / "lib"
                    name = "Python.framework/Versions/3.14/Python" if framework else "libpython3.14.so"
                    library = (prefix if framework else libdir) / name
                    library.parent.mkdir(parents=True)
                    library.write_bytes(b"fixture library path only")
                    values = {"LIBDIR": str(libdir), "LDLIBRARY": name,
                              "PYTHONFRAMEWORK": "Python" if framework else "",
                              "PYTHONFRAMEWORKPREFIX": str(prefix)}
                    controller = self.controller(root, domain)
                    try:
                        with patch.object(control.subprocess, "run", side_effect=self.configured_probe(values)), \
                             patch.dict(os.environ, {"LD_LIBRARY_PATH": "/prior/ld", "DYLD_LIBRARY_PATH": "/prior/dyld",
                                                     "DYLD_FRAMEWORK_PATH": "/prior/framework"}, clear=True):
                            environment = controller.runtime_environment(time.monotonic() + 2)
                        self.assertEqual(environment["PYO3_PYTHON"], sys.executable)
                        self.assertEqual(environment["LD_LIBRARY_PATH"], str(library.resolve().parent) + os.pathsep + "/prior/ld")
                        self.assertEqual(environment["DYLD_LIBRARY_PATH"], str(library.resolve().parent) + os.pathsep + "/prior/dyld")
                        expected = str(prefix) + os.pathsep + "/prior/framework" if framework else "/prior/framework"
                        self.assertEqual(environment["DYLD_FRAMEWORK_PATH"], expected)
                    finally:
                        controller.close()

    def test_missing_declared_library_stops_all_services_before_launch(self):
        for domain in ("S", "E", "B:fixture"):
            with self.subTest(domain=domain), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                controller = self.controller(root, domain)
                values = {"LIBDIR": str(root / "missing"), "LDLIBRARY": "libpython.so",
                          "PYTHONFRAMEWORK": "", "PYTHONFRAMEWORKPREFIX": ""}
                try:
                    with patch.object(control.subprocess, "run", side_effect=self.configured_probe(values)), \
                         patch.object(control.subprocess, "Popen") as spawn:
                        with self.assertRaisesRegex(ValueError, "共享运行库"):
                            controller.runtime_environment(time.monotonic() + 2)
                        spawn.assert_not_called()
                finally:
                    controller.close()

    def test_actual_explicit_interpreter_supplies_the_shared_runtime(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for domain in ("S", "E", "B:fixture"):
                controller = self.controller(root, domain)
                try:
                    environment = controller.runtime_environment(time.monotonic() + 2)
                    self.assertEqual(environment["PYO3_PYTHON"], sys.executable)
                    self.assertTrue(Path(environment["LD_LIBRARY_PATH"].split(os.pathsep)[0]).is_dir())
                finally:
                    controller.close()

    def test_owner_and_s_keep_distinct_process_nonce_names(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for domain in ("S", "E", "B:fixture"):
                controller = self.controller(root, domain)
                service = "s" if domain == "S" else "owner"
                try:
                    with patch.object(controller, "record"), \
                         patch.object(control.subprocess, "Popen", return_value=Mock(pid=123456)) as spawn:
                        controller.start_process(service, ["fixture-only"], environment={})
                    environment = spawn.call_args.kwargs["env"]
                    expected = "S_CONTROL_INSTANCE_ID" if domain == "S" else "SESSION_CONTROL_INSTANCE_ID"
                    other = "SESSION_CONTROL_INSTANCE_ID" if domain == "S" else "S_CONTROL_INSTANCE_ID"
                    self.assertTrue(environment[expected])
                    self.assertNotIn(other, environment)
                finally:
                    controller.close()

    def test_q_retains_explicit_economic_configuration_with_structure_query(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            controller = self.controller(root)
            controller.config.update({"port": "19003", "browser": str(root / "index.html"),
                                      "query_resource_config": str(root / "resources.json"),
                                      "economic_query_config": str(root / "economic-query.json")})
            try:
                with patch.object(controller, "start_process") as start, \
                     patch.object(controller, "wait_ready", return_value={"state": "ready"}) as ready:
                    self.assertEqual(controller.start_q("3"), {"state": "ready"})
                service, command = start.call_args.args
                self.assertEqual(service, "q")
                self.assertEqual(command[command.index("--resource-config") + 1], str(root / "resources.json"))
                self.assertEqual(command[command.index("--economic-config") + 1], str(root / "economic-query.json"))
                self.assertEqual(command[command.index("--producer-epoch") + 1], "3")
                self.assertEqual(start.call_args.kwargs["deadline"], ready.call_args.kwargs["deadline"])
                self.assertEqual(ready.call_args.kwargs["query_epoch"], "3")
            finally:
                controller.close()


if __name__ == "__main__":
    unittest.main()
