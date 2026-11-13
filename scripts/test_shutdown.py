"""Exercise the built server with real shutdown signals and an active download."""

import os
import http.client
import re
import signal
import socket
import stat
import subprocess
import tempfile
import time
import unittest
from contextlib import contextmanager
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOWNLOAD_SIZE = 16 * 1024 * 1024


class ShutdownTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.site = Path(self.temp.name) / "site"
        self.data = Path(self.temp.name) / "admin"
        self.site.mkdir()
        (self.site / "download.bin").write_bytes(b"x" * DOWNLOAD_SIZE)

    @contextmanager
    def server(self):
        environment = os.environ.copy()
        environment.pop("PORKMAIL_PWD", None)
        environment.pop("GOOGLE_CLIENT_ID", None)
        environment.pop("GOOGLE_CLIENT_SECRET", None)
        environment.update(
            APP_ENVIRONMENT="development",
            MAJ_APP_ORIGIN="http://localhost:8080",
            MAJ_DATA_DIR=str(self.data),
            LEPTOS_SITE_ADDR="127.0.0.1:0",
            LEPTOS_SITE_ROOT=str(self.site),
            RUST_LOG="info",
        )
        with tempfile.TemporaryFile() as log:
            process = subprocess.Popen(
                [str(ROOT / "target/debug/leptos-website-maj")],
                cwd=ROOT,
                env=environment,
                stdout=log,
                stderr=subprocess.STDOUT,
            )
            try:
                deadline = time.monotonic() + 10
                while time.monotonic() < deadline:
                    log.seek(0)
                    match = re.search(
                        rb"Listening on http://127\.0\.0\.1:(\d+)", log.read()
                    )
                    if match:
                        yield process, int(match[1])
                        break
                    if process.poll() is not None:
                        log.seek(0)
                        self.fail("server exited before listening: " + log.read().decode(errors="replace"))
                    time.sleep(0.02)
                else:
                    self.fail("server did not start within 10 seconds")
            finally:
                if process.poll() is None:
                    process.kill()
                process.wait(timeout=5)

    @contextmanager
    def download(self, port):
        with socket.socket() as connection:
            connection.setsockopt(socket.SOL_SOCKET, socket.SO_RCVBUF, 64 * 1024)
            connection.settimeout(5)
            connection.connect(("127.0.0.1", port))
            connection.sendall(
                b"GET /download.bin HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
            )
            response = b""
            while b"\r\n\r\n" not in response:
                chunk = connection.recv(4096)
                self.assertTrue(chunk, "download closed before its headers")
                response += chunk
            headers, body = response.split(b"\r\n\r\n", 1)
            self.assertIn(b"200 OK", headers)
            self.assertIn(f"content-length: {DOWNLOAD_SIZE}".encode(), headers.lower())
            yield connection, len(body)

    def test_interrupt_and_terminate_finish_active_downloads(self):
        for shutdown_signal in [signal.SIGINT, signal.SIGTERM]:
            with self.subTest(signal=shutdown_signal), self.server() as (process, port):
                with self.download(port) as (connection, received):
                    process.send_signal(shutdown_signal)
                    while chunk := connection.recv(64 * 1024):
                        received += len(chunk)
                    self.assertEqual(received, DOWNLOAD_SIZE)
                self.assertEqual(process.wait(timeout=5), 0)

    def test_private_storage_permissions_umask_and_http_isolation(self):
        private_content = b"private-storage-test-canary"
        with self.server() as (process, port):
            self.assertIn("Umask:\t0077", Path(f"/proc/{process.pid}/status").read_text())
            for relative in ["", "gallery", "gallery/originals", "gallery/thumbnails", "staging"]:
                self.assertEqual(stat.S_IMODE((self.data / relative).stat().st_mode), 0o700)
            database = self.data / "admin.sqlite3"
            self.assertEqual(stat.S_IMODE(database.stat().st_mode), 0o600)
            self.assertEqual(database.stat().st_size, 0)
            database.write_bytes(private_content)
            staging_file = self.data / "staging/private.txt"
            staging_file.write_bytes(private_content)
            staging_file.chmod(0o600)
            for path in ["/admin.sqlite3", "/data/admin.sqlite3", "/staging/private.txt", "/../admin/admin.sqlite3", "/%2e%2e/admin/admin.sqlite3"]:
                with self.subTest(path=path):
                    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=5)
                    try:
                        connection.request("GET", path)
                        response = connection.getresponse()
                        self.assertIn(response.status, [400, 404])
                        self.assertNotIn(private_content, response.read())
                    finally:
                        connection.close()
        self.assertEqual(database.read_bytes(), private_content)

    def test_stalled_download_cannot_hold_shutdown_open(self):
        with self.server() as (process, port), self.download(port):
            started = time.monotonic()
            process.send_signal(signal.SIGTERM)
            self.assertEqual(process.wait(timeout=28), 0)
            self.assertGreaterEqual(time.monotonic() - started, 24)


if __name__ == "__main__":
    unittest.main()
