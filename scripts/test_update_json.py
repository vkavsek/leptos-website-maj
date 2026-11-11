import json
from pathlib import Path
import stat
import subprocess
import tempfile
import unittest
import warnings
import zipfile


SCRIPT = Path(__file__).with_name("update_json.sh").resolve()
SHOWS = b'[{"date":"29.02.2024","name":"Updated show"}]'
VIDEOS = b'[{"url":"https://www.youtube.com/embed/test"}]'


class ContentImportTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.content = self.root / "public/data_json"
        self.content.mkdir(parents=True)
        for name in ("shows.json", "yt_videos.json"):
            (self.content / name).write_bytes(b"[]")

    def run_import(self, entries):
        archive = self.root / "content.zip"
        with zipfile.ZipFile(archive, "w") as output, warnings.catch_warnings():
            warnings.simplefilter("ignore", UserWarning)
            for name, payload in entries:
                output.writestr(name, payload)
        return subprocess.run(
            ["bash", str(SCRIPT), str(archive)], cwd=self.root,
            capture_output=True, text=True,
        )

    def test_valid_files_and_partial_updates(self):
        result = self.run_import([("shows.json", SHOWS), ("yt_videos.json", VIDEOS)])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.content / "shows.json").read_bytes(), SHOWS)
        result = self.run_import([("yt_videos.json", b"[]")])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.content / "yt_videos.json").read_bytes(), b"[]")
        self.assertEqual((self.content / "shows.json").read_bytes(), SHOWS)

    def test_invalid_archives_leave_all_content_unchanged(self):
        link = zipfile.ZipInfo("yt_videos.json")
        link.create_system = 3
        link.external_attr = (stat.S_IFLNK | 0o777) << 16
        fifo = zipfile.ZipInfo("yt_videos.json")
        fifo.external_attr = (stat.S_IFIFO | 0o600) << 16
        cases = [
            [], [("../outside.json", b"[]")], [("/outside.json", b"[]")],
            [("nested/shows.json", SHOWS)], [("unexpected.json", b"[]")],
            [("folder/", b"")], [(link, b"../../private.json")], [(fifo, b"[]")],
            [("shows.json", SHOWS), ("shows.json", b"[]")],
        ]
        for payload in [b"{broken", b"{}", b"[null]", b'[{"url":3}]', b"[NaN]",
                        b'[{"url":"first","url":"second"}]', "[]".encode("utf-16")]:
            cases.append([("shows.json", SHOWS), ("yt_videos.json", payload)])
        for record in [{}, {"date": None}, {"date": "31.02.2024"},
                       {"date": "29.02.2024", "name": 1}]:
            cases.append([("shows.json", json.dumps([record]))])
        for entries in cases:
            with self.subTest(entries=entries):
                result = self.run_import(entries)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual((self.content / "shows.json").read_bytes(), b"[]")
                self.assertEqual((self.content / "yt_videos.json").read_bytes(), b"[]")
                self.assertEqual(len(list(self.content.iterdir())), 2)

    def test_replacement_does_not_follow_existing_destination_link(self):
        private = self.root / "private.json"
        private.write_bytes(b"private fixture")
        target = self.content / "shows.json"
        target.unlink()
        target.symlink_to(private)
        result = self.run_import([("shows.json", SHOWS)])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse(target.is_symlink())
        self.assertEqual(target.read_bytes(), SHOWS)
        self.assertEqual(private.read_bytes(), b"private fixture")


if __name__ == "__main__":
    unittest.main()
