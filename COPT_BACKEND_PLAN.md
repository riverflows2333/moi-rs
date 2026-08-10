# COPT 8.0 MILP 后端实现计划

> 本文档用于指导 `moi-solver-copt` 和 `moirspy-copt` 的第一版实现。第一版范围限定为线性规划（LP）与混合整数线性规划（MILP），不包含二次、锥、非线性、回调、IIS 和 MIP start。

## 1. 当前框架与重构完成度

截至 2026-08-09，现有框架已经可以开始实现 COPT 后端，但原 `REFACTOR_PLAN.md` 尚不能视为“全部完成”。

### 已完成

- `ModelLike` 的变量、约束写操作已经统一返回 `Result<_, MoiError>`。
- 单条添加默认委托给批量添加，后端只需实现批量路径。
- `scalar_function_to_linear`、`scalar_set_to_bounds` 已放在 `moi-solver-api`，COPT 可以直接复用。
- `BridgeOptimizer` 已支持 attach 前缓存、attach 时同步、attach 后增量转发。
- Python `PyBackend` 已固定低层 backend 方法名、参数顺序和状态整数协议。
- `moi-model-dummy` 已成为 recording backend，可以在没有求解器时测试 Bridge 同步、参数传递和失败原子性。
- Gurobi 的 Rust/Python backend 已覆盖当前 LP/MILP 所需的完整纵向路径。
- 当前 `cargo fmt --all -- --check`、`cargo check --workspace --all-targets`、`cargo test --workspace` 均通过。

### 尚未完成，但不阻塞 COPT 第一版

- getter 仍使用 `Option`，无法区分“没有结果”和“native 查询失败”。COPT 第一版应在 `optimize()` 内查询并缓存状态、解向量和目标值，使错误仍能通过 `Result` 返回，getter 只读取缓存。
- 变量类型仍使用 `char`，尚未引入 solver-independent `VariableType`。COPT crate 内先严格接受并映射 `'C'`、`'B'`、`'I'`。
- backend ID 映射仍假定原生列/行索引与 `VarId`/`ConstrId` 连续一致。第一版只支持追加，不实现删除和重排。
- Python 运算符中的 `panic!` 尚未全面迁移为 `PyTypeError`。
- Python backend 协议还没有抽成单独的共享常量/类型模块；COPT 第一版必须严格复制现有 `moirspy-gurobi` 的 Python 方法签名。
- `cargo clippy --workspace --all-targets -- -D warnings` 尚未通过。目前是既有 Gurobi/Python 风格问题，不是 COPT blocker；新增 COPT crate 自身必须做到 clippy clean。
- 原重构计划的阶段 5 尚未完成，临时计划文档暂不删除。

## 2. 官方版本与 API 选择

计划以官方 COPT 8.0 文档和安装包中的 `copt.h` 为准。本机已安装 COPT 8.0.6，后续绑定以 `D:\env\copt80\include\copt.h` 为准。

官方资料：

- 用户手册：https://guide.coap.online/copt/en-doc/
- C API：https://guide.coap.online/copt/en-doc/capiref.html
- C 接口示例：https://guide.coap.online/copt/en-doc/cinterface.html
- 安装与许可证：https://guide.coap.online/copt/en-doc/install.html
- 参数：https://guide.coap.online/copt/en-doc/parameter.html
- 属性：https://guide.coap.online/copt/en-doc/attribute.html

第一版直接动态加载 C API，不依赖 `coptpy`，也不在构建期链接 COPT：

- Windows：优先检查 `%COPT_HOME%\bin\copt.dll`。
- Linux：优先检查 `$COPT_HOME/lib/libcopt.so`。
- macOS：优先检查 `$COPT_HOME/lib/libcopt.dylib`。
- 所有平台都允许显式传入 native library 完整路径。
- 运行时版本信息使用 `COPT_GetBanner` 检查；绑定常量必须从实际安装包的 `include/copt.h` 核对，不能只根据网页手抄数值。

## 3. 总体架构

```text
moirspy.Model
    │ setBackend("copt", env=...)
    ▼
PyBackend（通用 Python 协议）
    ▼
moirspy_copt.Model（PyO3 低层适配）
    ▼
CoptOptimizer（ModelLike + Optimizer）
    ▼
CoptApi（libloading 函数表）
    ▼
copt.dll / libcopt.so / libcopt.dylib
```

职责边界：

- `moi-core`：不出现任何 COPT 常量或状态码。
- `moi-solver-api`：继续提供通用 lowering 和 trait，不为 COPT 增加 solver-specific 方法。
- `moi-solver-copt`：动态加载、C API、RAII、数据转换、状态映射、参数和解缓存。
- `moirspy-copt`：只负责 PyO3 类型转换和满足 `PyBackend` 协议。
- `moirspy`：无需新增 COPT 分支；现有动态导入 `moirspy_{backend}` 已可加载 `moirspy_copt`。

## 4. `moi-solver-copt` 文件结构

```text
crates/moi-solver-copt/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── bindings/
│   │   ├── mod.rs
│   │   └── gen80.rs
│   ├── dynamic/
│   │   ├── mod.rs
│   │   ├── api.rs
│   │   └── loader.rs
│   └── wrapper/
│       ├── mod.rs
│       ├── env.rs
│       ├── optimizer.rs
│       └── utils.rs
└── tests/
    ├── test_api.rs
    ├── test_moi.rs
    └── test_bridge.rs
```

### 4.1 `bindings/gen80.rs`

只保存第一版需要的 opaque 类型、常量和名称：

- `copt_env`、`copt_prob` 对应 opaque pointer。
- 变量类型：`COPT_CONTINUOUS`、`COPT_BINARY`、`COPT_INTEGER`。
- 目标方向：`COPT_MINIMIZE`、`COPT_MAXIMIZE`。
- `COPT_INFINITY`。
- 统一状态常量：`COPT_STATUS_*`。
- 属性名称：`Status`、`HasSol`、`IsMIP`、`ObjVal`、`Cols`、`Rows`。
- 参数名称：`TimeLimit`、`Logging`。
- 推荐消息缓冲区大小 `COPT_BUFFSIZE`。

绑定来源必须注明 COPT 8.0 `copt.h`。如果使用 bindgen，只提交稳定生成结果，不要求最终用户安装 clang。

### 4.2 `dynamic/api.rs`

`CoptApi` 持有 `libloading::Library` 和最小函数表：

```rust
pub struct CoptApi {
    _lib: Library,

    // version / diagnostics
    pub COPT_GetBanner: ...,
    pub COPT_GetRetcodeMsg: ...,

    // environment
    pub COPT_CreateEnv: ...,
    pub COPT_CreateEnvWithPath: ...,
    pub COPT_DeleteEnv: ...,
    pub COPT_GetLicenseMsg: ...,

    // problem lifetime
    pub COPT_CreateProb: ...,
    pub COPT_DeleteProb: ...,
    pub COPT_Update: ...,

    // LP/MILP construction
    pub COPT_AddCols: ...,
    pub COPT_AddRows: ...,
    pub COPT_SetColObj: ...,
    pub COPT_SetObjConst: ...,
    pub COPT_SetObjSense: ...,

    // parameters / attributes
    pub COPT_SetIntParam: ...,
    pub COPT_SetDblParam: ...,
    pub COPT_GetIntAttr: ...,
    pub COPT_GetDblAttr: ...,

    // solve / solution
    pub COPT_Solve: ...,
    pub COPT_GetSolution: ...,
    pub COPT_GetLpSolution: ...,
}
```

注意：官方网页中个别 synopsis 可能存在排版类型错误，例如 `COPT_GetDblAttr` 的输出指针；实现必须以安装包 `copt.h` 为最终依据。

### 4.3 `dynamic/loader.rs`

实现顺序：

1. 显式 native library 路径。
2. `COPT_HOME`。
3. 少量官方默认安装目录：Windows `C:\Program Files\copt80`、Unix `/opt/copt80`。
4. 找不到时返回包含已检查路径的明确错误。

不要扫描并误载 `copt_cpp.dll`；只接受精确 C library 文件名。loader 单元测试不依赖安装 COPT。

### 4.4 `wrapper/env.rs`

```rust
pub struct CoptEnv {
    api: Arc<CoptApi>,
    raw: *mut c_void,
}

pub type SharedCoptEnv = Arc<Mutex<CoptEnv>>;
```

构造接口：

```rust
pub fn new(api: Arc<CoptApi>) -> Result<Self, MoiError>;
pub fn with_license_dir(
    api: Arc<CoptApi>,
    license_dir: &str,
) -> Result<Self, MoiError>;
```

- `new` 调用 `COPT_CreateEnv`。
- `with_license_dir` 调用 `COPT_CreateEnvWithPath`。
- `Drop` 调用 `COPT_DeleteEnv(&mut raw)`。
- 环境创建失败时，尽可能使用 `COPT_GetLicenseMsg` 补充诊断。
- 暂不暴露 `empty/start`：COPT 环境创建时即验证许可证，没有对应 Gurobi empty-env 流程。
- 暂不提供 `Env.setParam`：COPT 8.0 的普通求解参数 API 接收 `copt_prob*`，参数属于 Model/Problem。
- floating/cluster client 所需 `COPT_CreateEnvConfig` 系列留到第二阶段。
- 保守使用 `Arc<Mutex<_>>`，不要给裸环境声明 `Sync`；optimizer 只实现确有需要的 `Send`。

### 4.5 `wrapper/utils.rs`

需要实现：

```rust
fn normalize_bound(value: f64) -> f64;
fn map_variable_type(value: char) -> Result<c_char, MoiError>;
fn build_copt_rows(...) -> Result<CoptRows, MoiError>;
fn map_copt_status(native: c_int, has_solution: bool) -> SolveStatus;
fn checked_c_int(value: usize, field: &str) -> Result<c_int, MoiError>;
```

`CoptRows` 使用具名结构体，不返回长 tuple：

```rust
struct CoptRows {
    beg: Vec<c_int>,
    count: Vec<c_int>,
    index: Vec<c_int>,
    value: Vec<f64>,
    lower: Vec<f64>,
    upper: Vec<f64>,
    names: CStringArray,
}
```

约束推荐使用 `COPT_AddRows` 的 row-bound 模式：传 `rowSense = NULL`，将 `ScalarSetType` 转成 lower/upper。这样可以原生支持：

- `GreaterThan`：`[rhs, +∞]`
- `LessThan`：`[-∞, rhs]`
- `EqualTo`：`[rhs, rhs]`
- `Interval`：`[lower, upper]`

仿射函数常数必须从上下界两端减去。所有 `usize -> c_int` 转换必须检查溢出，禁止直接 `as c_int`。

### 4.6 `wrapper/optimizer.rs`

```rust
pub struct CoptOptimizer {
    _env: SharedCoptEnv,
    api: Arc<CoptApi>,
    prob: *mut c_void,
    num_vars: usize,
    num_constrs: usize,
    objective: Option<ScalarFunctionType>,
    sense: Option<ModelSense>,
    cached_status: Option<SolveStatus>,
    cached_solution: Option<Vec<f64>>,
    cached_objective: Option<f64>,
}
```

所有模型修改成功后清除三个求解缓存。native 调用成功后才递增计数或提交 Rust 状态。

## 5. `ModelLike` / `Optimizer` 接口映射

| Rust 接口 | COPT C API | 第一版语义 |
|---|---|---|
| `add_variables` | `COPT_AddCols` | 无矩阵系数，目标系数初始为 0；支持 C/B/I、bounds、names |
| `add_constraints` | `COPT_AddRows` | CRS；使用 row lower/upper 模式；支持四种 scalar set |
| `set_objective` | `COPT_SetColObj` + `COPT_SetObjConst` + `COPT_SetObjSense` | 先构造完整目标系数数组，重复项累加，覆盖旧目标 |
| `update` | `COPT_Update` | 检查返回码 |
| `set_optimizer_attr(TimeLimit)` | `COPT_SetDblParam("TimeLimit")` | 仅接受 float，非负 |
| `set_optimizer_attr(Silent)` | `COPT_SetIntParam("Logging")` | `true -> 0`，`false -> 1` |
| `set_optimizer_attr(Raw)` | `COPT_SetIntParam` / `COPT_SetDblParam` | bool/int -> int，float -> double；第一版拒绝 string |
| `optimize` | `COPT_Solve` | 同时处理 LP/MILP，之后统一查询 Status/HasSol/ObjVal |
| `get_var_value` | 读取缓存 | optimize 中用 `GetSolution` 或 `GetLpSolution` 一次性缓存 |
| `get_objective_value` | 读取缓存 | optimize 中查询统一 `ObjVal` |
| `compute_conflict` | 暂不实现 | 返回明确 unsupported；后续接 `COPT_ComputeIIS` |

### 状态映射

- `COPT_STATUS_OPTIMAL` → `SolveStatus::Optimal`
- `COPT_STATUS_INFEASIBLE` → `SolveStatus::Infeasible`
- `COPT_STATUS_UNBOUNDED` → `SolveStatus::Unbounded`
- `NODELIMIT`、`TIMEOUT`、`INTERRUPTED`、`IMPRECISE`：有解时 → `Feasible`，无解时 → `Unknown`
- `INF_OR_UNB`、`NUMERICAL`、`UNFINISHED`、`UNSTARTED` → `Unknown`

只查询官方推荐的统一属性 `Status`、`HasSol`、`ObjVal`，不要使用已弃用的 `LpStatus/MipStatus/HasLpSol/HasMipSol/LpObjval/BestObj`。

### LP 与 MILP 解向量

`COPT_Solve` 后：

1. 查询 `HasSol`，无解则清空 solution/objective cache。
2. 查询 `IsMIP`。
3. MIP 使用 `COPT_GetSolution`。
4. LP 使用 `COPT_GetLpSolution(prob, values, NULL, NULL, NULL)`。
5. 查询统一 `ObjVal`。

所有这些查询都发生在 `optimize() -> Result` 内，避免 getter 的 `Option` 吞掉 native error。

## 6. `moirspy-copt` 文件结构

```text
python/moirspy-copt/
├── Cargo.toml
├── pyproject.toml
├── README.md
├── moirspy_copt.pyi
├── src/
│   ├── lib.rs
│   ├── loader.rs
│   ├── env.rs
│   └── model.rs
└── tests/
    └── test_model.py
```

### 6.1 Python `Env`

```python
from moirspy_copt import Env

env = Env()                          # COPT_CreateEnv
env = Env(dll_path="...")           # 显式 native library
env = Env(license_dir="...")        # COPT_CreateEnvWithPath
```

构造签名建议：

```python
Env(dll_path: str | None = None, license_dir: str | None = None)
```

第一版不提供 `setParam/start/started`，README 明确说明 COPT 参数属于 problem。

### 6.2 Python `Model`

为了兼容现有通用 `PyBackend`，方法签名必须与 `moirspy_gurobi.Model` 一致：

```python
Model(name=None, dll_path=None, env=None, license_dir=None)

add_variable(name=None, vtype=None, lb=None, ub=None) -> int
add_variables(n, names=None, vtypes=None, lbs=None, ubs=None) -> list[int]
add_constraint(vars, coeffs, constant, sense, rhs, name=None) -> int
add_constraints(fs_vars, fs_coeffs, fs_consts, senses, rhss, names=None) -> list[int]
set_objective(vars, coeffs, constant, sense) -> None
update() -> None
optimize() -> int
get_var_value(var_id) -> float | None
get_objective_value() -> float | None
set_optimizer_attr(attr, value) -> None
```

约束：

- `env` 与 `dll_path/license_dir` 不允许同时提供。
- Python bool 必须先于 int 提取。
- 所有 Rust `MoiError` 转成 `PyRuntimeError`；参数形状错误转成 `PyValueError`；类型错误转成 `PyTypeError`。
- `optimize` 返回 `SolveStatus::code()`，禁止返回 COPT native status。

### 6.3 高层使用

现有 `moirspy` 不需要 solver-specific import：

```python
from moirspy import Model, MOI
from moirspy_copt import Env

env = Env()
model = Model("milp")
x = model.addVars(3, vtype=MOI.BINARY, name="x")
model.addConstr(x[0] + 2*x[1] + 3*x[2] <= 4)
model.setObjective(x[0] + x[1] + 2*x[2], MOI.MAXIMIZE)
model.setParam("Logging", 0)
model.setBackend("copt", env=env)
model.optimize()
```

`setBackend("copt", env=env)` 已由通用层透明转发，不需要让 `moirspy` 认识 `CoptEnv` 类型。

## 7. 分阶段实施

### 阶段 A：前置清理与骨架

- [x] 在 workspace dependencies 增加 `moi-solver-copt`。
- [x] 创建两个 crate 和空模块结构。
- [x] 从 COPT 8.0.6 `copt.h` 生成/核对最小 bindings。
- [x] 两个 crate 在没有 COPT 安装时也能 `cargo check`。

验收：`cargo check --workspace --all-targets`。

### 阶段 B：loader、API 与 RAII

- [x] 实现跨平台 loader 和精确库名过滤。
- [x] 加载最小 `CoptApi` 函数表。
- [x] 实现 `CoptEnv`、`SharedCoptEnv` 和 `CoptOptimizer` 构造/析构。
- [x] 用 `COPT_GetBanner` 做动态库 smoke test。
- [x] 所有创建失败路径释放已创建资源。

验收：无安装时 loader 单测通过；有安装/许可证时 env/prob smoke test 通过。

### 阶段 C：LP/MILP 建模

- [ ] `add_variables`：C/B/I、bounds、names、长度检查、NUL 检查。
- [ ] `add_constraints`：CSR、常数平移、四种 scalar set、非法 VarId 检查。
- [ ] `set_objective`：完整覆盖旧目标、常数、方向。
- [ ] `update`。
- [ ] native 成功后才更新计数和缓存。

验收：直接 Rust API 构造 LP 和 MILP；dummy 对照测试确认传入数据一致。

### 阶段 D：求解、状态、参数与结果

- [ ] `COPT_Solve`。
- [ ] 统一 Status/HasSol/ObjVal 查询。
- [ ] LP/MIP solution cache。
- [ ] TimeLimit、Silent、Raw int/double 参数。
- [ ] infeasible/unbounded/time-limit-with-incumbent 状态映射测试。

验收：Rust 端 LP、MILP、infeasible、unbounded、参数和解值测试通过。

### 阶段 E：Python backend

- [ ] 实现 `moirspy_copt.Env`。
- [ ] 实现与 Gurobi 完全一致的低层 `Model` 协议。
- [ ] 补 `.pyi`、README、`pyproject.toml`。
- [ ] `maturin develop`。
- [ ] 验证低层 `moirspy_copt.Model`。
- [ ] 验证高层 `moirspy.Model.setBackend("copt")` 和显式 `env=`。

验收：Python LP、MILP、向量 bounds、增量建模、不可行模型和错误输入测试通过。

### 阶段 F：最终验收

- [ ] `cargo fmt --all -- --check`。
- [ ] `cargo clippy -p moi-solver-copt --all-targets -- -D warnings`。
- [ ] `cargo clippy -p moirspy-copt --all-targets -- -D warnings`。
- [ ] `cargo test --workspace`。
- [ ] 无 COPT 环境时 native tests 明确跳过，solver-independent tests 仍通过。
- [ ] Windows、Linux 至少各验证一次 loader；macOS 可先保留 CI/用户验证项。
- [ ] 检查动态库和许可证没有被打包进 wheel。

## 8. 第一版测试矩阵

### 不依赖 COPT

- loader 文件名和路径优先级。
- `'C'/'B'/'I'` 映射，非法字符错误。
- CSR 构造和 affine constant 平移。
- Interval row bounds。
- status + has_solution 映射。
- 长度不一致、非法 ID、内嵌 NUL、整数溢出。
- 使用 dummy backend 的通用 Bridge 协议回归。

### 依赖 COPT 动态库和许可证

- banner/env/prob 生命周期。
- LP 最小化。
- binary MILP 最大化。
- mixed continuous/integer MILP。
- infeasible。
- unbounded。
- `TimeLimit`、`Logging` 和至少一个 Raw integer 参数。
- objective constant。
- vector names/lb/ub/vtype。
- attach 前完整建模与 attach 后增量建模。

### Python

- 低层 `Model` 默认环境。
- 显式 `Env` 复用两个模型。
- `license_dir`。
- 高层 `setBackend("copt", env=env)`。
- Python 异常类型和状态码协议。

## 9. 第一版明确不做

- QP/QCP/SOCP/SDP/NLP。
- SOS、indicator constraints。
- callbacks、lazy constraints。
- MIP starts 和 solution pool。
- IIS/冲突分析。
- 删除变量/约束和 native index 重映射。
- problem copy、读写 MPS/LP、参数文件。
- floating/cluster `EnvConfig`。
- 把 COPT 官方 Python 包 `coptpy` 作为加载来源。

这些功能应在基础 MILP 路径稳定后按独立阶段增加，避免第一版扩大 FFI 和公共 trait 范围。

## 10. 实施前需要的本机信息

开始 native 集成测试前，需要确认：

- `COPT_HOME` 的实际路径。
- native C library 的实际文件名和位置。
- `$COPT_HOME/include/copt.h` 的版本。
- 本机许可证是默认发现，还是需要 `COPT_LICENSE_DIR` / 显式 license directory。

在这些信息不可用时，仍可完成 crate 骨架、bindings 审核之外的 loader/unit tests、数据 lowering 和 Python 类型层编译。
