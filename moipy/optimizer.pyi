from typing import List, Protocol

class SolverBackend(Protocol):
    def addVars(
        self, num: int, lb: List[float], ub: List[float], types: str
    ) -> None: ...
    def addConstr(
        self, indices: List[int], coeffs: List[float], sense: str, rhs: float
    ) -> None: ...
    def optimize(self) -> None: ...
