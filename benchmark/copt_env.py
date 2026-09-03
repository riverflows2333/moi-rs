"""Create COPT environments from process variables and a local ``.env`` file.

This module deliberately has no dependency on ``python-dotenv`` so benchmark
environments remain small and reproducible. Process environment variables take
precedence over values from the file, and secret values are never logged.
"""

from __future__ import annotations

import os
import re
from pathlib import Path
from typing import Any, Mapping, MutableMapping


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_ENV_FILE = ROOT / ".env"
_ENV_NAME = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")

_CONFIG_ENV_NAMES = {
    "COPT_ENV_NO_BANNER": "NoBanner",
    "COPT_CLIENT_CAFILE": "CaFile",
    "COPT_CLIENT_CERTFILE": "CertFile",
    "COPT_CLIENT_CERTKEYFILE": "CertKeyFile",
    "COPT_CLIENT_CLUSTER": "Cluster",
    "COPT_CLIENT_FLOATING": "Floating",
    "COPT_CLIENT_PASSWORD": "PassWord",
    "COPT_CLIENT_PORT": "Port",
    "COPT_CLIENT_PRIORITY": "Priority",
    "COPT_CLIENT_WAITTIME": "WaitTime",
    "COPT_CLIENT_WEBSERVER": "WebServer",
    "COPT_CLIENT_WEBLICENSEID": "WebLicenseId",
    "COPT_CLIENT_WEBACCESSKEY": "WebAccessKey",
    "COPT_CLIENT_WEBTOKENDURATION": "WebTokenDuration",
}


def load_env_file(
    path: str | Path | None = None,
    *,
    environ: MutableMapping[str, str] | None = None,
    required: bool = False,
) -> Path | None:
    """Load a dotenv-style file without overriding existing variables."""

    target = Path(path).expanduser().resolve() if path is not None else DEFAULT_ENV_FILE
    if not target.is_file():
        if required:
            raise FileNotFoundError(f"COPT environment file does not exist: {target}")
        return None

    destination = os.environ if environ is None else environ
    for line_number, raw_line in enumerate(
        target.read_text(encoding="utf-8-sig").splitlines(), 1
    ):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        if line.startswith("export "):
            line = line[7:].lstrip()
        name, separator, raw_value = line.partition("=")
        name = name.strip()
        if not separator or not _ENV_NAME.fullmatch(name):
            raise ValueError(f"{target}:{line_number}: invalid environment assignment")
        destination.setdefault(name, _decode_value(raw_value.strip(), target, line_number))
    return target


def _decode_value(value: str, path: Path, line_number: int) -> str:
    if not value:
        return ""
    if value[0] == "'":
        if len(value) < 2 or value[-1] != "'":
            raise ValueError(f"{path}:{line_number}: unterminated single-quoted value")
        return value[1:-1]
    if value[0] == '"':
        if len(value) < 2 or value[-1] != '"':
            raise ValueError(f"{path}:{line_number}: unterminated double-quoted value")
        return _decode_escapes(value[1:-1])
    comment = re.search(r"\s+#", value)
    if comment is not None:
        value = value[: comment.start()].rstrip()
    return value


def _decode_escapes(value: str) -> str:
    replacements = {"n": "\n", "r": "\r", "t": "\t", "\\": "\\", '"': '"'}
    result: list[str] = []
    index = 0
    while index < len(value):
        if value[index] == "\\" and index + 1 < len(value):
            escaped = value[index + 1]
            replacement = replacements.get(escaped)
            if replacement is not None:
                result.append(replacement)
                index += 2
                continue
        result.append(value[index])
        index += 1
    return "".join(result)


def copt_config_values(environ: Mapping[str, str] | None = None) -> dict[str, str]:
    """Return validated COPT environment configuration without exposing secrets."""

    source = os.environ if environ is None else environ
    values = {
        config_name: value
        for env_name, config_name in _CONFIG_ENV_NAMES.items()
        if (value := source.get(env_name, ""))
    }

    oem = source.get("COPT_OEM_NAME", "")
    signature = source.get("COPT_OEM_SIGNATURE", "")
    license_payload = source.get("COPT_OEM_LICENSE", "")
    oem_requested = any((oem, signature, license_payload))
    if not oem_requested:
        return values
    missing = [
        name
        for name, value in (
            ("COPT_OEM_NAME", oem),
            ("COPT_OEM_SIGNATURE", signature),
        )
        if not value
    ]
    if missing:
        raise ValueError(
            "incomplete COPT OEM configuration; missing " + ", ".join(missing)
        )
    if not license_payload:
        version = source.get("COPT_OEM_VERSION", "")
        expiry = source.get("COPT_OEM_EXPIRY", "")
        license_type = source.get("COPT_OEM_TYPE", "")
        metadata_missing = [
            name
            for name, value in (
                ("COPT_OEM_VERSION", version),
                ("COPT_OEM_EXPIRY", expiry),
                ("COPT_OEM_TYPE", license_type),
            )
            if not value
        ]
        if metadata_missing:
            raise ValueError(
                "COPT_OEM_LICENSE is empty and license metadata is incomplete; missing "
                + ", ".join(metadata_missing)
            )
        license_payload = (
            "#### COPT OEM LICENSE DATA ####\n\n"
            f"USER = {oem}\n"
            f"VERSION = {version}\n"
            f"EXPIRY = {expiry}\n"
            f"TYPE={license_type}\n\n"
        )
    values.update({"OEM": oem, "License": license_payload, "Signature": signature})
    return values


def create_copt_env(env_file: str | Path | None = None) -> Any:
    """Create the built-in ``moirspy.CoptEnv`` used by high-level models."""

    from moirspy import CoptEnv, CoptEnvConfig

    return _create_copt_env(CoptEnv, CoptEnvConfig, env_file)


def create_legacy_copt_env(env_file: str | Path | None = None) -> Any:
    """Create ``moirspy_copt.Env`` for the standalone low-level benchmark."""

    from moirspy_copt import Env, EnvrConfig

    return _create_copt_env(Env, EnvrConfig, env_file)


def _create_copt_env(
    env_type: Any, config_type: Any, env_file: str | Path | None
) -> Any:
    """Load dotenv values and instantiate the selected COPT environment type."""

    load_env_file(env_file, required=env_file is not None)
    values = copt_config_values()
    dll_path = os.environ.get("COPT_DLL_PATH") or None
    if not values and dll_path is None:
        return env_type()
    config = config_type(dll_path)
    for name, value in values.items():
        config.set(name, value)
    return env_type(config=config)
