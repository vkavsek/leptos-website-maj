"""Check the Fish launcher with dummy credentials and mocked clipboard/Cargo tools."""

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/dev-google-auth.fish"
CLIENT_ID = "312770222166-qtkr1gn7e22jpofqtt31vhbq88bj3ppb.apps.googleusercontent.com"
SECRET = "fixture-$()-`literal`-secret"


class LauncherTests(unittest.TestCase):
    def setUp(self):
        just = shutil.which("just")
        if just is None:
            self.fail("Install just before running the launcher tests")
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.bin = self.directory / "bin"
        self.bin.mkdir()
        self.log = self.directory / "calls"
        for name in ["cargo", "wl-copy", "xclip", "powershell.exe"]:
            command = self.bin / name
            command.write_text(
                "#!/usr/bin/python3\n"
                "import os, sys\n"
                "from pathlib import Path\n"
                "name = Path(sys.argv[0]).name\n"
                'with open(os.environ["LAUNCHER_TEST_LOG"], "a") as log:\n'
                '    log.write(name + " " + " ".join(sys.argv[1:]) + "\\n")\n'
                'if name != "cargo":\n'
                '    assert "GOOGLE_CLIENT_SECRET" not in os.environ\n'
                '    if name == "xclip":\n'
                '        assert sys.stdin.read() == ""\n'
                '    sys.exit(int(os.environ.get("LAUNCHER_TEST_CLIPBOARD_FAILURE", "0")))\n'
                f'assert os.environ["GOOGLE_CLIENT_SECRET"] == {SECRET!r}\n'
                f'assert os.environ["GOOGLE_CLIENT_ID"] == {CLIENT_ID!r}\n'
                'assert os.environ["APP_ENVIRONMENT"] == "development"\n'
                'assert os.environ["MAJ_APP_ORIGIN"] == "http://localhost:8080"\n'
                'assert "PORKMAIL_PWD" not in os.environ\n'
                f"assert Path.cwd() == Path({str(ROOT)!r})\n"
                'assert sys.argv[1:] == ["leptos", "watch"]\n'
                'sys.exit(int(os.environ.get("LAUNCHER_TEST_CARGO_STATUS", "0")))\n'
            )
            command.chmod(0o700)
        self.environment = os.environ.copy()
        for key in [
            "WSL_INTEROP",
            "WSL_DISTRO_NAME",
            "WAYLAND_DISPLAY",
            "DISPLAY",
            "GOOGLE_CLIENT_SECRET",
        ]:
            self.environment.pop(key, None)
        self.environment.update(
            PATH=str(self.bin) + ":" + str(Path(just).parent) + ":/usr/bin:/bin",
            GOOGLE_CLIENT_ID=CLIENT_ID,
            MAJ_DATA_DIR=str(self.directory / "data"),
            WAYLAND_DISPLAY="mock-wayland",
            LAUNCHER_TEST_LOG=str(self.log),
            XDG_CONFIG_HOME=str(self.directory / "config"),
            XDG_DATA_HOME=str(self.directory / "share"),
            XDG_CACHE_HOME=str(self.directory / "cache"),
        )

    def run_launcher(self, input_value=SECRET + "\n"):
        result = subprocess.run(
            ["fish", "--no-config", "--private", str(SCRIPT)],
            check=False,
            cwd=self.directory,
            env=self.environment,
            input=input_value,
            capture_output=True,
            text=True,
            timeout=10,
        )
        self.assertNotIn(SECRET, result.stdout + result.stderr)
        self.assertFalse(list((self.directory / "share").rglob("*_history")))
        return result

    def calls(self):
        return self.log.read_text().splitlines() if self.log.exists() else []

    def test_clear_clipboard_before_launch_on_each_platform(self):
        for backend, settings in [
            ("wl-copy", {}),
            ("xclip", {"WAYLAND_DISPLAY": "", "DISPLAY": ":99"}),
            ("powershell.exe", {"WSL_DISTRO_NAME": "mock-wsl"}),
        ]:
            with self.subTest(backend=backend):
                self.log.unlink(missing_ok=True)
                environment = self.environment.copy()
                self.environment.update(settings)
                if backend == "xclip":
                    self.environment.pop("WAYLAND_DISPLAY", None)
                result = self.run_launcher()
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertTrue(self.calls()[0].startswith(backend + " "))
                self.assertEqual(self.calls()[1], "cargo leptos watch")
                self.environment = environment

    def test_default_client_id_and_propagate_server_exit_status(self):
        self.environment.pop("GOOGLE_CLIENT_ID")
        self.environment["LAUNCHER_TEST_CARGO_STATUS"] = "7"
        self.assertEqual(
            self.run_launcher().returncode, 7
        )

    def test_clear_failure_prevents_launch(self):
        self.environment["LAUNCHER_TEST_CLIPBOARD_FAILURE"] = "1"
        self.assertNotEqual(self.run_launcher().returncode, 0)
        self.assertEqual(self.calls(), ["wl-copy --clear"])

    def test_empty_or_cancelled_input_still_clears_clipboard(self):
        for value in ["\n", ""]:
            with self.subTest(value=value):
                self.log.unlink(missing_ok=True)
                self.assertNotEqual(self.run_launcher(value).returncode, 0)
                self.assertEqual(self.calls(), ["wl-copy --clear"])

    def test_project_id_is_not_accepted_as_client_id(self):
        self.environment["GOOGLE_CLIENT_ID"] = "maj-webpage-admin-development"
        self.assertNotEqual(self.run_launcher().returncode, 0)
        self.assertEqual(self.calls(), [])

    def test_inherited_secret_is_not_sent_to_clipboard_command(self):
        self.environment["GOOGLE_CLIENT_SECRET"] = "old-fixture-secret"
        self.assertEqual(self.run_launcher().returncode, 0)

    def test_missing_clipboard_backend_prevents_prompt_and_launch(self):
        self.environment.pop("WAYLAND_DISPLAY")
        self.assertNotEqual(self.run_launcher().returncode, 0)
        self.assertEqual(self.calls(), [])


if __name__ == "__main__":
    unittest.main()
