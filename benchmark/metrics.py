from __future__ import annotations

import ctypes
import importlib.util
import importlib.metadata
import json
import os
import platform
import subprocess
import sys
from dataclasses import asdict, dataclass
from datetime import datetime
from pathlib import Path


@dataclass(frozen=True, slots=True)
class TimingSummary:
    median: float
    p95: float
    minimum: float
    maximum: float
    samples: int

    @classmethod
    def from_samples(cls, samples: list[float]) -> "TimingSummary":
        if not samples:
            raise ValueError("at least one timing sample is required")
        ordered = sorted(samples)
        return cls(
            median=percentile(ordered, 0.50),
            p95=percentile(ordered, 0.95),
            minimum=ordered[0],
            maximum=ordered[-1],
            samples=len(ordered),
        )


def percentile(ordered: list[float], fraction: float) -> float:
    """Linearly interpolated percentile for an already sorted non-empty list."""

    if not ordered:
        raise ValueError("percentile requires at least one value")
    if not 0.0 <= fraction <= 1.0:
        raise ValueError("percentile fraction must be between zero and one")
    position = (len(ordered) - 1) * fraction
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    weight = position - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def process_memory_bytes() -> tuple[int | None, int | None]:
    """Return current RSS and process-lifetime peak RSS using only stdlib APIs."""

    if sys.platform == "win32":
        return _windows_memory_bytes()
    try:
        import resource

        peak = resource.getrusage(resource.RUSAGE_SELF).ru_maxrss
        peak_bytes = peak if sys.platform == "darwin" else peak * 1024
    except (ImportError, OSError):
        peak_bytes = None
    current_bytes = None
    statm = Path("/proc/self/statm")
    if statm.is_file():
        try:
            resident_pages = int(statm.read_text(encoding="ascii").split()[1])
            current_bytes = resident_pages * os.sysconf("SC_PAGE_SIZE")
        except (OSError, ValueError, IndexError):
            pass
    return current_bytes, peak_bytes


def _windows_memory_bytes() -> tuple[int | None, int | None]:
    from ctypes import wintypes

    size_t = ctypes.c_size_t

    class ProcessMemoryCountersEx(ctypes.Structure):
        _fields_ = [
            ("cb", wintypes.DWORD),
            ("PageFaultCount", wintypes.DWORD),
            ("PeakWorkingSetSize", size_t),
            ("WorkingSetSize", size_t),
            ("QuotaPeakPagedPoolUsage", size_t),
            ("QuotaPagedPoolUsage", size_t),
            ("QuotaPeakNonPagedPoolUsage", size_t),
            ("QuotaNonPagedPoolUsage", size_t),
            ("PagefileUsage", size_t),
            ("PeakPagefileUsage", size_t),
            ("PrivateUsage", size_t),
        ]

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    psapi = ctypes.WinDLL("psapi", use_last_error=True)
    get_current_process = kernel32.GetCurrentProcess
    get_current_process.argtypes = []
    get_current_process.restype = wintypes.HANDLE
    get_process_memory_info = psapi.GetProcessMemoryInfo
    get_process_memory_info.argtypes = [
        wintypes.HANDLE,
        ctypes.POINTER(ProcessMemoryCountersEx),
        wintypes.DWORD,
    ]
    get_process_memory_info.restype = wintypes.BOOL

    counters = ProcessMemoryCountersEx()
    counters.cb = ctypes.sizeof(counters)
    ok = get_process_memory_info(
        get_current_process(),
        ctypes.byref(counters),
        counters.cb,
    )
    if not ok:
        return None, None
    return int(counters.WorkingSetSize), int(counters.PeakWorkingSetSize)


def environment_metadata() -> dict[str, object]:
    packages: dict[str, str] = {}
    for distribution in ("moirspy", "moirspy-copt", "coptpy"):
        try:
            packages[distribution] = importlib.metadata.version(distribution)
        except importlib.metadata.PackageNotFoundError:
            packages[distribution] = "not-installed"

    artifacts = {
        module: _extension_artifact(module)
        for module in ("moirspy.moirspy", "moirspy_copt.moirspy_copt")
    }
    return {
        "timestamp": datetime.now().astimezone().isoformat(timespec="seconds"),
        "platform": platform.platform(),
        "machine": platform.machine(),
        "processor": platform.processor() or os.environ.get("PROCESSOR_IDENTIFIER", "unknown"),
        "logical_cpus": os.cpu_count(),
        "python": platform.python_version(),
        "python_implementation": platform.python_implementation(),
        "python_executable": sys.executable,
        "rustc": _command_version(["rustc", "--version"]),
        "packages": packages,
        "copt_home": os.environ.get("COPT_HOME"),
        "copt_version": _copt_version(),
        "extension_artifacts": artifacts,
        "threads_env": {
            name: os.environ.get(name)
            for name in ("OMP_NUM_THREADS", "MKL_NUM_THREADS", "OPENBLAS_NUM_THREADS")
        },
    }


def _extension_artifact(module: str) -> dict[str, object] | None:
    try:
        spec = importlib.util.find_spec(module)
    except (ImportError, AttributeError):
        return None
    if spec is None or spec.origin is None:
        return None
    path = Path(spec.origin)
    try:
        size = path.stat().st_size
    except OSError:
        size = None
    stem = module.rsplit(".", 1)[-1]
    profile = "unknown"
    for candidate_profile in ("release", "debug"):
        profile_dir = Path.cwd() / "target" / candidate_profile
        candidates = (
            profile_dir / f"{stem}.dll",
            profile_dir / f"lib{stem}.so",
            profile_dir / f"lib{stem}.dylib",
        )
        if size is not None and any(
            candidate.is_file() and candidate.stat().st_size == size for candidate in candidates
        ):
            profile = candidate_profile
            break
    return {"path": str(path), "bytes": size, "inferred_profile": profile}


def _copt_version() -> str:
    home = os.environ.get("COPT_HOME")
    if not home:
        return "unknown"
    notes = Path(home) / "release-notes.txt"
    try:
        return next(line.strip() for line in notes.read_text(errors="replace").splitlines() if line.strip())
    except (OSError, StopIteration):
        return "unknown"


def _command_version(command: list[str]) -> str:
    try:
        completed = subprocess.run(
            command,
            check=False,
            capture_output=True,
            text=True,
            timeout=5,
        )
    except (OSError, subprocess.TimeoutExpired):
        return "unavailable"
    return completed.stdout.strip() or completed.stderr.strip() or "unavailable"


def json_dumps(value: object) -> str:
    return json.dumps(value, ensure_ascii=False, indent=2, default=asdict)
