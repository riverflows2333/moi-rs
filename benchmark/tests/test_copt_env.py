from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

from benchmark.copt_env import copt_config_values, load_env_file


class CoptEnvironmentTests(unittest.TestCase):
    def test_file_does_not_override_process_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / ".env"
            path.write_text(
                "COPT_OEM_NAME=file-name\n"
                'COPT_OEM_SIGNATURE="signature\\nline"\n'
                "COPT_ENV_NO_BANNER=1\n",
                encoding="utf-8",
            )
            environ = {"COPT_OEM_NAME": "process-name"}

            loaded = load_env_file(path, environ=environ, required=True)

        self.assertEqual(loaded, path.resolve())
        self.assertEqual(environ["COPT_OEM_NAME"], "process-name")
        self.assertEqual(environ["COPT_OEM_SIGNATURE"], "signature\nline")
        self.assertEqual(environ["COPT_ENV_NO_BANNER"], "1")

    def test_oem_license_can_be_constructed_from_metadata(self):
        values = copt_config_values(
            {
                "COPT_OEM_NAME": "example-user",
                "COPT_OEM_SIGNATURE": "example-signature",
                "COPT_OEM_VERSION": "1.2.3",
                "COPT_OEM_EXPIRY": "123456",
                "COPT_OEM_TYPE": "OEM",
            }
        )

        self.assertEqual(values["OEM"], "example-user")
        self.assertEqual(values["Signature"], "example-signature")
        self.assertIn("USER = example-user", values["License"])
        self.assertIn("VERSION = 1.2.3", values["License"])

    def test_explicit_license_and_client_config_are_forwarded(self):
        values = copt_config_values(
            {
                "COPT_OEM_NAME": "example-user",
                "COPT_OEM_SIGNATURE": "example-signature",
                "COPT_OEM_LICENSE": "license-payload",
                "COPT_CLIENT_CAFILE": "ca.pem",
                "COPT_CLIENT_WEBLICENSEID": "license-id",
            }
        )

        self.assertEqual(values["License"], "license-payload")
        self.assertEqual(values["CaFile"], "ca.pem")
        self.assertEqual(values["WebLicenseId"], "license-id")

    def test_partial_oem_configuration_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "COPT_OEM_SIGNATURE"):
            copt_config_values({"COPT_OEM_NAME": "example-user"})

    def test_missing_explicit_file_is_rejected(self):
        with self.assertRaises(FileNotFoundError):
            load_env_file("does-not-exist.env", required=True)


if __name__ == "__main__":
    unittest.main()
