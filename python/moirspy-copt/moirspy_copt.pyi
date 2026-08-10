from typing import Optional

class Env:
    def __init__(
        self,
        dll_path: Optional[str] = None,
        license_dir: Optional[str] = None,
    ) -> None: ...

class Model:
    def __init__(
        self,
        name: Optional[str] = None,
        dll_path: Optional[str] = None,
        env: Optional[Env] = None,
        license_dir: Optional[str] = None,
    ) -> None: ...
