class Var:
    def __init__(self, id: int) -> None: ...

class Vars:
    def __init__(self, shape: list[int], vars: list[int]) -> None: ...
    def __getitem__(self, index: int) -> Var: ...

class LinExpr:
    pass


class Constr:
    pass

class MOI:
    def __init__(self) -> None:
        self.CONTINUOUS = "CONTINUOUS"
        self.BINARY = "BINARY"
        self.INTEGER = "INTEGER"
        self.MINIMIZE = "MINIMIZE"
        self.MAXIMIZE = "MAXIMIZE"


class Model:
    def __init__(self, name: str) -> None: ...
    def addVar(
        self,
        lb: float = 0.0,
        ub: float = float("inf"),
        obj: float = 0.0,
        vtype: str = "C",
        name: str = "",
    ) -> "Var": ...
    def addVars(
        self,
        *args: int,
        lb: float = 0.0,
        ub: float = float("inf"),
        obj: float = 0.0,
        vtype: str = "C",
        name: str = "",
    ) -> "Vars": ...
    def addConstr(
        self,
        constr: Constr,
        name: str = "",
    ) -> None: ...
    def addConstrs(self, generator: list[Constr], name: str = "") -> None: ...
    def setObjective(self, expr: LinExpr, sense: MOI) -> None: ...
    def set_backend(self, backend: str) -> None: ...
    def optimize(self) -> None: ...
