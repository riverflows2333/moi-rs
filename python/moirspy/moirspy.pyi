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
        lb: float = 0.0 | list[float],
        ub: float = float("inf") | list[float],
        obj: float = 0.0,
        vtype: str = "C" | list[str],
        name: str = "" | list[str],
    ) -> "Var": ...
    def addVars(
        self,
        *args: int,
        lb: float = 0.0 | list[float],
        ub: float = float("inf") | list[float],
        obj: float = 0.0,
        vtype: str = "C" | list[str],
        name: str = "" | list[str],
    ) -> "Vars": ...
    def addConstr(
        self,
        constr: Constr,
        name: str = "",
    ) -> None: ...
    def addConstrs(self, generator: list[Constr], name: str = "") -> None: ...
    def setObjective(self, expr: LinExpr, sense: MOI) -> None: ...
    def setBackend(self, backend: str) -> None: ...
    def optimize(self) -> None: ...

# 常用函数
def quicksum(generator: list[Var | LinExpr]) -> LinExpr: ...
