from pathlib import Path
import sys
from tarfile import open as open_tar
from zipfile import ZipFile


def is_forbidden(path: str) -> bool:
    normalized = path.replace("\\", "/").lower()
    name = normalized.rsplit("/", 1)[-1]
    return (
        name.endswith(".lic")
        or name == "copt.dll"
        or name == "libcopt.so"
        or name.startswith("libcopt.so.")
        or (name.startswith("libcopt") and name.endswith(".dylib"))
        or (name.startswith("gurobi") and name.endswith(".dll"))
        or (name.startswith("libgurobi") and ".so" in name)
        or (name.startswith("libgurobi") and name.endswith(".dylib"))
        or normalized.endswith("/crates/moi-solver-copt/src/bindings/gen.rs")
    )


def artifacts_under(arguments: list[str]) -> list[Path]:
    artifacts: list[Path] = []
    for argument in arguments:
        path = Path(argument)
        if path.is_dir():
            artifacts.extend(path.glob("*.whl"))
            artifacts.extend(path.glob("*.tar.gz"))
        elif path.suffix == ".whl" or path.name.endswith(".tar.gz"):
            artifacts.append(path)
    return sorted(set(artifacts))


def members(artifact: Path) -> list[str]:
    if artifact.suffix == ".whl":
        with ZipFile(artifact) as archive:
            return archive.namelist()
    with open_tar(artifact, "r:gz") as archive:
        return archive.getnames()


def main() -> int:
    artifacts = artifacts_under(sys.argv[1:])
    if not artifacts:
        print("No wheel or source distribution was found to audit.", file=sys.stderr)
        return 1

    rejected: list[str] = []
    for artifact in artifacts:
        for member in members(artifact):
            if is_forbidden(member):
                rejected.append(f"{artifact}: {member}")

    if rejected:
        print("Forbidden generated sources, solver runtimes, or licenses were bundled:", file=sys.stderr)
        print("\n".join(rejected), file=sys.stderr)
        return 1

    print(f"Audited {len(artifacts)} artifact(s); no forbidden file was bundled.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
