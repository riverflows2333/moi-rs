# Correctness 与后端接口重构计划（临时）

> 本文档只用于跟踪本轮重构。所有阶段完成、验收通过且必要内容转移到正式文档后，删除本文件。

## 执行记录（2026-08-07）

阶段 0–3 的最小功能重构已经完成：

- 阶段 0：格式化、workspace 编译和 solver-independent/Gurobi 分层测试已完成；Gurobi 不可用或动态库版本不兼容时相关测试会跳过。完整 `cargo test --workspace` 仍会被尚未实现的 `moi-model-dummy::DummyModel` 阻塞，这一项随阶段 4 处理。
- 阶段 1：已修复 `addVar` bridge 丢失、批量变量参数丢失、单约束协议、状态码混用、Boolean 参数识别、Gurobi environment 生命周期、C 字符串和重复 backend attach 等已知 correctness 问题。
- 阶段 2：`ModelLike` 的四个添加方法已经迁移为 `Result`；Bridge、Gurobi、PyBackend 和两个 Python crate 已完成错误传播；批量长度、变量引用、unsupported interval 和 native 返回码均已校验。
- 阶段 3：单条添加已改为批量添加的默认实现；公共线性函数 lowering 和 scalar-set bounds lowering 已加入 `moi-solver-api`。变量类型 enum、backend index 映射和完整 Python 协议模块保留为后续设计任务。

本轮新增的回归测试位于 `crates/moi-bridge/tests/correctness.rs`，core/status/lowering 测试位于对应模块内。

仍保留到后续阶段的项目：

- `moi-model-dummy` recording backend 及由它承载的完整 Bridge attach/失败注入测试。
- Python 运算符中既有的 `panic!` 全面迁移为 `PyTypeError`。
- getter 从 `Option` 迁移到能够区分“无结果”和“查询失败”的接口。
- `VariableType` enum、稳定 backend ID 映射及删除/重建模型语义。

## 目标与范围

本轮工作的目标是先稳定现有 LP/MILP 路径，为后续 COPT 后端提供可靠的公共接口：

1. 修复当前已知的非设计层面 correctness bug。
2. 将 `ModelLike` 的可失败写操作改为显式返回 `Result`，不再吞掉后端错误。
3. 逐层消除 FFI、PyO3 和 Bridge 路径中的 `unwrap`、`panic` 与静默失败。
4. 在较后阶段恢复 `moi-model-dummy`，将其实现为可检查 Bridge 同步行为的 recording backend。

本轮暂不包含：

- COPT 动态库加载与求解器实现。
- 二次、锥、非线性函数支持。
- 删除变量/约束、generational ID 或完整 capability 系统。
- Python 建模 API 的大规模改名或不兼容调整。
- `moi-cache`、`moi-expr`、`moi-nlp` 等占位 crate 的功能实现。

## 总体原则

- 每个阶段结束时保持工作区可编译，不进行一次性大爆炸式迁移。
- 不允许 native solver 调用失败后仍生成本地 ID 或递增计数。
- Rust 错误在 Rust 层使用 `MoiError`；跨 PyO3 边界时转换为合适的 Python exception。
- `panic!` 只用于真正不可恢复的内部不变量；用户输入、求解器错误和不支持的功能必须返回错误。
- 在没有真实 Gurobi 安装的环境中，也应能测试 core、Bridge 和参数同步逻辑。
- 每修复一个已知 bug，先增加或同时增加能够复现该 bug 的回归测试。

## 阶段 0：建立基线与测试分层

优先级：P0

- [ ] 记录当前构建基线：`cargo check --workspace`。
- [ ] 修复 `cargo fmt --all -- --check` 当前失败的问题，只做格式化，不混入逻辑修改。
- [ ] 将测试分成不依赖求解器的单元测试和依赖 Gurobi 的集成测试。
- [ ] 为 Gurobi 集成测试增加显式环境检测；没有动态库或许可证时应跳过，而不是因为硬编码 `/opt/gurobi1203` 失败。
- [ ] 确认 CI/default test 不要求安装 Gurobi。

验收标准：

- `cargo fmt --all -- --check` 通过。
- `cargo check --workspace` 通过。
- 无 Gurobi 环境时，solver-independent 测试可以独立运行。

## 阶段 1：修复现有 correctness bug

优先级：P0

这一阶段尽量不改变 `ModelLike` 的签名，先修复可以局部完成的问题。

### 1.1 Python 建模对象

- [ ] 修复 `Model::add_var`：返回已经绑定 `SharedBridge` 的 `Var`，而不是重新创建未绑定对象。
- [ ] 添加回归测试：`addVar()` 与 `addVars()[...]` 获得解值时行为一致。
- [ ] 调整 Python 参数类型识别顺序，优先识别 `bool`，避免把 Python `bool` 当作整数。
- [ ] `set_objective` 不再忽略两次 `set_model_attr` 的返回值，统一转换为 `PyRuntimeError` 或更具体的异常。
- [ ] `set_backend` 返回 `PyResult<()>`，导入失败、构造失败和同步失败不再通过 `expect` 终止进程。

涉及文件：

- `python/moirspy/src/model.rs`
- `python/moirspy/src/var.rs`
- `python/moirspy/tests/`

### 1.2 PyBackend 参数协议

- [ ] 修复 `add_variables` 丢弃 vector names/lower bounds/upper bounds 的问题。
- [ ] 修复单约束 `add_constraint` 的参数顺序，使其与 `moirspy-gurobi::Model.add_constraint` 完全一致。
- [ ] 为批量变量、单约束、批量约束分别增加协议测试。
- [ ] 删除未使用的 `name_list`、`lb_list`、`ub_list` 分支变量，或者统一构造成一个实际传递值。
- [ ] 不再依赖 `SolveStatus as u32` 的隐式枚举序号；定义稳定的跨 Python 状态协议。
- [ ] 至少覆盖 `Unknown`、`Optimal`、`Infeasible`、`Unbounded`、`Feasible` 的往返映射。

推荐的临时稳定协议：后端 Python 方法返回固定字符串，例如 `"optimal"`；若继续使用整数，则必须定义显式协议枚举并禁止使用 solver native status code。

涉及文件：

- `python/moirspy/src/py_backend.rs`
- `python/moirspy-gurobi/src/model.rs`
- `crates/moi-core/src/attributes.rs`

### 1.3 Gurobi 资源与字符串安全

- [ ] 让 `GurobiOptimizer` 持有 `Arc<GurobiEnv>`，明确保证 model 的生命周期不超过 environment。
- [ ] 检查构造失败路径：如果 `GRBnewmodel` 失败，不得保存空或无效 model 指针。
- [ ] 将模型名、变量名、约束名、Raw 参数名和字符串参数值统一通过 `CString::new` 转换。
- [ ] 将字符串中的内嵌 NUL 转换为明确的 `MoiError`/构造错误，而不是 `unwrap`。
- [ ] 复核 `unsafe impl Send/Sync`：在没有清晰线程安全保证前，优先移除不必要的 `Sync`，或用互斥保护并记录安全依据。
- [ ] 为 FFI 返回码建立统一 helper，例如 `check_grb(code, context)`；真正接入所有写操作放在阶段 2 完成。

涉及文件：

- `crates/moi-solver-gurobi/src/wrapper/wrapper.rs`
- `crates/moi-solver-gurobi/src/dynamic/api.rs`
- `python/moirspy-gurobi/src/model.rs`

### 1.4 Bridge 局部状态一致性

- [ ] 设置 `ObjectiveSense` 且目标函数尚不存在时不再 `unwrap`；选择“零目标函数”或返回明确错误，并用测试固定语义。
- [ ] 后端已挂载时，Bridge 自己持有的静态模型属性仍能读取；求解结果类属性再委托后端。
- [ ] 优化后修改变量、约束、目标或参数时清除/标记旧求解结果，避免读取过期值。
- [ ] 明确并测试重复 `attach_backend` 的行为：拒绝、替换或重新同步，只允许一种语义。

涉及文件：

- `crates/moi-bridge/src/optimizer.rs`
- `crates/moi-bridge/tests/`

阶段 1 验收标准：

- 每个上述 bug 都有不依赖真实 Gurobi 的回归测试，资源生命周期测试除外可使用局部 mock。
- Python/Rust 桥接路径不再因普通用户输入使用 `panic!` 或 `expect`。
- 单条添加和批量添加得到等价的数据。

## 阶段 2：重塑 `ModelLike` 的错误返回

优先级：P0

### 2.1 修改公共 trait

- [ ] 将以下方法改为返回 `Result`：

```rust
fn add_variable(...) -> Result<VarId, MoiError>;
fn add_variables(...) -> Result<Vec<VarId>, MoiError>;
fn add_constraint(...) -> Result<ConstrId, MoiError>;
fn add_constraints(...) -> Result<Vec<ConstrId>, MoiError>;
```

- [ ] 保留 `set_objective`、`update`、属性 setter 和 `optimize` 的既有 `Result` 形式。
- [ ] 评估 getter 的 `Option`：本轮暂不强制全部改为 `Result<Option<_>>`，但必须区分“结果不存在”和“后端查询失败”；无法区分的地方记录为后续接口任务。
- [ ] 为常见失败补充结构化 `MoiError`，至少包括：
  - 参数长度不一致；
  - 非法变量/约束 ID；
  - 非法名称（内嵌 NUL）；
  - 不支持的函数/集合组合；
  - native solver error code；
  - backend protocol/Python exception；
  - backend 已挂载/未挂载状态错误。

### 2.2 按依赖方向迁移实现

迁移顺序固定为：

1. `moi-solver-api` trait 与错误类型。
2. `moi-bridge` 实现。
3. `moi-solver-gurobi` 实现。
4. `python/moirspy/src/py_backend.rs`。
5. `python/moirspy-gurobi`。
6. `python/moirspy` 建模 API。
7. tests/examples。

每迁移一层都运行 `cargo check`，避免错误在多层堆积后才处理。

### 2.3 输入校验和原子性

- [ ] `add_variables(n, ...)` 校验 names、vtypes、lbs、ubs 的 vector 长度均为 `n`。
- [ ] `add_constraints` 校验 functions、sets、names 数量一致。
- [ ] 每个 affine expression 校验变量 ID 已存在。
- [ ] 批量调用失败时不得部分递增本地计数或返回虚假 ID。
- [ ] Bridge 在后端调用成功后才提交本地状态；或者明确实现可恢复的两阶段操作。
- [ ] `attach_backend` 同步时检查后端返回的 ID 数量及映射，不再假设调用必然成功。
- [ ] unsupported `Interval`、function type 或 attribute 返回 `MoiError::Unsupported...`，不再静默忽略或 panic。

### 2.4 接入 Gurobi 返回码

- [ ] 检查 `GRBaddvars`、`GRBaddconstr`、objective setters、parameter setters、update 和 optimize 的返回码。
- [ ] 只有 native 调用成功后才修改 `num_vars`、`num_constrs`。
- [ ] 尽可能读取并附加 solver error message；至少保留 API 名称、错误码和操作上下文。
- [ ] `set_model_attr`、`get_optimizer_attr` 等未实现分支返回明确的不支持错误或 `None` 语义，不再无条件 `Ok(())`。
- [ ] `compute_conflict` 暂未实现时返回 `MoiError::Unsupported...`，不再使用 `unimplemented!()`。

阶段 2 验收标准：

- 所有 `ModelLike` 实现和调用点完成迁移。
- 后端失败不会产生 ID、计数或 Bridge 状态漂移。
- 对向量长度不一致、非法 ID、不支持集合和非法 C 字符串均有测试。
- `rg "unwrap\\(|expect\\(|panic!|unimplemented!"` 的剩余结果逐项审核并注明合理性。

## 阶段 3：减少协议重复并稳定后端边界

优先级：P1

这一阶段只做能降低 COPT 复制成本的小型重构，不扩展数学功能。

- [ ] 为单条添加方法提供基于批量方法的默认实现，减少 Bridge、Gurobi 和 PyBackend 的双份逻辑。
- [ ] 抽取 `ScalarFunctionType` 到 `(indices, coefficients, constant)` 的公共、可失败转换。
- [ ] 抽取 `ScalarSetType` 到稳定约束表示的公共转换，至少统一 `<`、`>`、`=` 和 `Interval` 的错误语义。
- [ ] Python backend 协议集中定义方法名、参数顺序和状态编码，Rust 两侧不再各自手写约定。
- [ ] 将 `char` 变量类型替换为 solver-independent enum 的工作单独评估；若会扩大本轮范围，可以记录到 COPT 前置任务而不在本轮实施。
- [ ] 评估 `VarInfo.col_index`、`ConstrInfo.row_index` 从 core 移到具体 backend；若暂不迁移，至少移除 Gurobi 特有注释并禁止其他逻辑依赖它们等于 backend index。

阶段 3 验收标准：

- 单条与批量路径共用同一套转换和校验逻辑。
- Gurobi-specific 常量和状态码不出现在 `moi-core`、`moi-solver-api` 或通用 Python 桥中。
- 新建 COPT 后端时无需复制 PyBackend 的协议解释逻辑。

## 阶段 4：恢复 `moi-model-dummy`，实现 recording backend

优先级：P2

沿用现有 `moi-model-dummy` crate，不额外新建 crate。它首先是测试后端，不追求实际求解能力。

### 4.1 最小数据模型

- [ ] 实现可编译的 `DummyModel`，完整实现最新的 `ModelLike` 和 `Optimizer`。
- [ ] 保存变量、约束、目标函数、模型方向、optimizer attributes 和调用顺序。
- [ ] 对输入执行与真实后端一致的基础校验。
- [ ] `optimize` 返回可配置状态；默认状态明确记录在测试中。
- [ ] 支持配置 objective value 和 variable primal values，供 Bridge 查询转发测试使用。

### 4.2 Recording 能力

- [ ] 定义稳定的记录类型，例如：

```rust
enum RecordedCall {
    AddVariables { /* specs */ },
    AddConstraints { /* rows */ },
    SetObjective { /* function + sense */ },
    SetOptimizerAttr { /* attr + value */ },
    Update,
    Optimize,
}
```

- [ ] 提供在 backend 被移动进 `Box<dyn Optimizer>` 后仍可检查的共享 handle，例如 `Arc<Mutex<RecordingState>>`。
- [ ] 测试 `BridgeOptimizer::attach_backend` 的同步顺序与同步内容。
- [ ] 测试挂载后的增量添加路径。
- [ ] 测试后端注入失败时 Bridge 状态和本地模型是否保持一致。
- [ ] 测试参数在 attach 前缓存、attach 时同步、attach 后实时传递。

### 4.3 清理旧测试和 examples

- [ ] 修复当前引用不存在 `DummyModel` 的 tests/examples。
- [ ] 删除已经被注释掉且不再表达真实 API 的测试代码。
- [ ] 将 solver-independent 的核心回归测试尽量放到 dummy/recording backend 上。

阶段 4 验收标准：

- `cargo test -p moi-model-dummy` 通过。
- `cargo test -p moi-bridge` 不依赖 Gurobi 并覆盖完整 attach/sync/增量路径。
- `cargo test --workspace` 在无 Gurobi 环境中不会因为 Gurobi 缺失而失败。

## 阶段 5：最终验收与临时文档删除

优先级：P1

- [ ] `cargo fmt --all -- --check`。
- [ ] `cargo clippy --workspace --all-targets`，新增 warning 必须处理，FFI 生成代码可局部 allow。
- [ ] `cargo test --workspace`。
- [ ] 在可用环境执行 Gurobi 集成测试，覆盖 LP、MILP、infeasible、unbounded、参数设置和解值查询。
- [ ] 检查 README 中 Python API、状态语义、后端挂载时机与实际实现一致。
- [ ] 确认 `git diff` 中没有无关格式化或生成文件改动。
- [ ] 将仍需在 COPT 前完成的设计任务转移到 issue/正式文档。
- [ ] 删除本文件 `REFACTOR_PLAN.md`。

## 建议的提交拆分

为了便于回归和 review，建议至少拆成以下提交：

1. `test: establish solver-independent baseline`
2. `fix: repair Python model and backend protocol bugs`
3. `fix: enforce Gurobi resource and CString safety`
4. `refactor: make ModelLike mutations fallible`
5. `refactor: propagate backend errors across Bridge and PyO3`
6. `test: implement dummy recording optimizer`
7. `docs: align backend behavior and remove temporary plan`

不要把全仓库格式化、trait 迁移和功能性 bug 修复放进同一个提交。

## 当前已知风险清单

- `ModelLike` 签名变化会同时影响 Rust 后端、Bridge 和两个 Python crate，应严格按依赖顺序迁移。
- PyO3 backend 当前跨越 Rust → Python → Rust，错误链容易丢失，需要保留原始 Python exception 文本。
- 批量 native API 可能出现部分提交语义；在承诺 Bridge 原子性前，需要确认 Gurobi 行为或采用预校验/重建策略。
- `Send + Sync` trait object 约束可能迫使 FFI wrapper 声明不安全的线程能力，应先决定模型是否真的需要并发访问。
- 状态协议修改必须同步 Python `.pyi`、README 和测试，不能只改 Rust 实现。
