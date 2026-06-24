from typing import List, Optional, Union

class Model:
    def __init__(
        self,
        name: Optional[str] = ...,
        dll_path: Optional[str] = ...,
    ) -> None: ...
    def add_variable(
        self,
        name: Optional[str] = ...,
        vtype: Optional[str] = ...,
        lb: Optional[float] = ...,
        ub: Optional[float] = ...,
    ) -> int: ...
    def add_variables(
        self,
        n: int,
        names: Optional[List[str]] = ...,
        vtypes: Optional[List[str]] = ...,
        lbs: Optional[List[float]] = ...,
        ubs: Optional[List[float]] = ...,
    ) -> List[int]: ...
    def add_constraint(
        self,
        vars: List[int],
        coeffs: List[float],
        constant: float,
        sense: str,
        rhs: float,
        name: Optional[str] = ...,
    ) -> int: ...
    def add_constraints(
        self,
        fs_vars: List[List[int]],
        fs_coeffs: List[List[float]],
        fs_consts: List[float],
        senses: List[str],
        rhss: List[float],
        names: Optional[List[str]] = ...,
    ) -> List[int]: ...
    def set_objective(
        self,
        vars: List[int],
        coeffs: List[float],
        constant: float,
        sense: int,
    ) -> None: ...
    def update(self) -> None: ...
    def optimize(self) -> int: ...
    def get_var_value(self, var_id: int) -> Optional[float]: ...
    def get_objective_value(self) -> Optional[float]: ...
    def set_optimizer_attr(
        self,
        attr: str,
        value: Union[bool, int, float, str],
    ) -> None: ...
