from typing import List, Optional, Union

import numpy as np
import numpy.typing as npt

ParamValue = Union[bool, int, float, str]

class Env:
    def __init__(
        self,
        dll_path: Optional[str] = ...,
        empty: bool = ...,
    ) -> None: ...
    def setParam(self, name: str, value: ParamValue) -> None: ...
    def start(self) -> None: ...
    @property
    def started(self) -> bool: ...

class Model:
    def __init__(
        self,
        name: Optional[str] = ...,
        dll_path: Optional[str] = ...,
        env: Optional[Env] = ...,
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
    def add_constraints_flat(
        self,
        row_offsets: npt.NDArray[np.uintp],
        columns: npt.NDArray[np.uintp],
        values: npt.NDArray[np.float64],
        constants: npt.NDArray[np.float64],
        senses: npt.NDArray[np.uint8],
        rhss: npt.NDArray[np.float64],
        names: Optional[List[str]] = ...,
    ) -> tuple[int, int]: ...
    def set_objective(
        self,
        vars: List[int],
        coeffs: List[float],
        constant: float,
        sense: int,
    ) -> None: ...
    def set_objective_flat(
        self,
        vars: npt.NDArray[np.uintp],
        coeffs: npt.NDArray[np.float64],
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
        value: ParamValue,
    ) -> None: ...
