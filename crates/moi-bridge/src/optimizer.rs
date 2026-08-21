use moi_core::attributes::{AttrValue, ModelAttr, OptimizerAttr};
use moi_core::*;
use moi_solver_api::*;
use std::collections::HashMap;

/// 表示桥接优化器的同步状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeState {
    /// Model data is being cached without an attached backend.
    Building,
    /// A backend is attached and no valid solution is cached.
    Attached,
    /// The last optimize call completed and result getters may be queried.
    Solved,
    /// A solved model was modified; the previous solution is invalid.
    Dirty,
}

/// BridgeOptimizer 作为模型状态的中心缓存，并动态分发任务给后端求解器
pub struct BridgeOptimizer {
    pub vars: Vec<VarInfo>,
    pub constrs: HashMap<ConstrId, ConstrInfo>,
    pub obj: Option<ScalarFunctionType>,
    pub sense: Option<ModelSense>,
    pub status: BridgeState,
    pub backend: Option<Box<dyn Optimizer + Send>>,
    pub raw_params: HashMap<OptimizerAttr, AttrValue>,
    termination_status: Option<SolveStatus>,
}

impl BridgeOptimizer {
    pub fn new() -> Self {
        Self {
            vars: Vec::new(),
            constrs: HashMap::new(),
            obj: None,
            sense: None,
            status: BridgeState::Building,
            backend: None,
            raw_params: HashMap::new(),
            termination_status: None,
        }
    }

    /// 关联后端求解器并执行“大冲刷”（一次性同步所有本地缓存的状态）
    pub fn attach_backend(
        &mut self,
        mut backend: Box<dyn Optimizer + Send>,
    ) -> Result<(), MoiError> {
        if self.backend.is_some() {
            return Err(MoiError::BackendState(
                "a backend is already attached".to_string(),
            ));
        }

        // 1. 同步变量
        if !self.vars.is_empty() {
            let n = self.vars.len();
            let names: Vec<String> = self.vars.iter().map(|v| v.name.clone()).collect();
            let vtypes: Vec<char> = self.vars.iter().map(|v| v.vtype).collect();
            let lbs: Vec<f64> = self.vars.iter().map(|v| v.lb).collect();
            let ubs: Vec<f64> = self.vars.iter().map(|v| v.ub).collect();

            let ids = backend.add_variables(
                n,
                Some(NameType::Vector(names)),
                Some(vtypes),
                Some(BoundType::Vector(lbs)),
                Some(BoundType::Vector(ubs)),
            )?;
            Self::ensure_expected_var_ids(&ids, 0)?;
        }

        // 2. 同步约束
        if !self.constrs.is_empty() {
            let mut sorted_constrs: Vec<_> = self.constrs.iter().collect();
            sorted_constrs.sort_by_key(|(id, _)| id.0);

            let fs: Vec<ScalarFunctionType> = sorted_constrs
                .iter()
                .map(|(_, info)| info.f.clone())
                .collect();
            let ss: Vec<ScalarSetType> = sorted_constrs
                .iter()
                .map(|(_, info)| info.s.clone())
                .collect();
            let names: Vec<String> = sorted_constrs
                .iter()
                .map(|(_, info)| info.name.clone())
                .collect();

            let ids = backend.add_constraints(fs, ss, Some(names))?;
            Self::ensure_expected_constr_ids(&ids, 0)?;
        }

        // 3. 同步目标函数和优化方向
        if let (Some(obj), Some(sense)) = (&self.obj, self.sense) {
            backend.set_objective(obj.clone(), sense)?;
        }

        // 4. 同步优化器参数
        for (attr, value) in &self.raw_params {
            backend.set_optimizer_attr(attr.clone(), value.clone())?;
        }

        self.backend = Some(backend);
        self.status = BridgeState::Attached;
        self.termination_status = None;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.vars.is_empty() && self.constrs.is_empty()
    }

    pub fn get_var_name_by_id(&self, id: VarId) -> Option<String> {
        self.vars.get(id.0).map(|var| var.name.clone())
    }

    fn invalidate_solution(&mut self) {
        self.termination_status = None;
        self.status = match self.status {
            BridgeState::Building => BridgeState::Building,
            BridgeState::Attached => BridgeState::Attached,
            BridgeState::Solved | BridgeState::Dirty => BridgeState::Dirty,
        };
    }

    fn detach_failed_backend(&mut self) {
        self.backend = None;
        self.status = BridgeState::Building;
        self.termination_status = None;
    }

    fn ensure_expected_var_ids(ids: &[VarId], start: usize) -> Result<(), MoiError> {
        for (offset, id) in ids.iter().enumerate() {
            let actual = id.0;
            let expected = start + offset;
            if actual != expected {
                return Err(MoiError::BackendProtocol(format!(
                    "backend returned variable ID {actual}, expected {expected}"
                )));
            }
        }
        Ok(())
    }

    fn ensure_expected_constr_ids(ids: &[ConstrId], start: usize) -> Result<(), MoiError> {
        for (offset, id) in ids.iter().enumerate() {
            let actual = id.0;
            let expected = start + offset;
            if actual != expected {
                return Err(MoiError::BackendProtocol(format!(
                    "backend returned constraint ID {actual}, expected {expected}"
                )));
            }
        }
        Ok(())
    }

    fn validate_function(&self, function: &ScalarFunctionType) -> Result<(), MoiError> {
        match function {
            ScalarFunctionType::Variable(variable) => {
                if variable.0 >= self.vars.len() {
                    return Err(MoiError::InvalidVariableIndex(variable.0));
                }
            }
            ScalarFunctionType::Affine(function) => {
                if !function.constant.is_finite() {
                    return Err(MoiError::InvalidInput(
                        "affine function constant must be finite".into(),
                    ));
                }
                for term in &function.terms {
                    if term.var.0 >= self.vars.len() {
                        return Err(MoiError::InvalidVariableIndex(term.var.0));
                    }
                    if !term.coeff.is_finite() {
                        return Err(MoiError::InvalidInput(format!(
                            "affine coefficient for variable {} must be finite",
                            term.var.0
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_set(set: &ScalarSetType) -> Result<(), MoiError> {
        match set {
            ScalarSetType::GreaterThan(value)
            | ScalarSetType::LessThan(value)
            | ScalarSetType::EqualTo(value) => {
                if !value.is_finite() {
                    return Err(MoiError::InvalidInput(
                        "constraint bound must be finite".into(),
                    ));
                }
            }
            ScalarSetType::Interval(lower, upper) => {
                if !lower.is_finite() || !upper.is_finite() {
                    return Err(MoiError::InvalidInput(
                        "interval bounds must be finite".into(),
                    ));
                }
                if lower > upper {
                    return Err(MoiError::InvalidInput(format!(
                        "interval lower bound {lower} exceeds upper bound {upper}"
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_name(name: &str, field: &str) -> Result<(), MoiError> {
        if name.contains('\0') {
            Err(MoiError::InvalidName(format!(
                "{field} contains an embedded NUL byte"
            )))
        } else {
            Ok(())
        }
    }
}

impl Default for BridgeOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelLike for BridgeOptimizer {
    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Result<Vec<VarId>, MoiError> {
        let start_id = self.vars.len();
        let end_id = start_id
            .checked_add(n)
            .ok_or_else(|| MoiError::InvalidInput("variable count overflow".into()))?;
        let names = match name {
            Some(NameType::Single(base)) => {
                (0..n).map(|i| format!("{base}_{i}")).collect::<Vec<_>>()
            }
            Some(NameType::Vector(values)) => {
                ensure_len(values.len(), n, "variable names")?;
                values
            }
            None => vec![String::new(); n],
        };
        let vtypes = match vtype {
            Some(values) => {
                ensure_len(values.len(), n, "variable types")?;
                values
            }
            None => vec!['C'; n],
        };
        let lbs = match lb {
            Some(BoundType::Single(value)) => vec![value; n],
            Some(BoundType::Vector(values)) => {
                ensure_len(values.len(), n, "lower bounds")?;
                values
            }
            None => vec![0.0; n],
        };
        let ubs = match ub {
            Some(BoundType::Single(value)) => vec![value; n],
            Some(BoundType::Vector(values)) => {
                ensure_len(values.len(), n, "upper bounds")?;
                values
            }
            None => vec![f64::INFINITY; n],
        };

        for (index, name) in names.iter().enumerate() {
            Self::validate_name(name, &format!("variable name at index {index}"))?;
        }
        for (index, variable_type) in vtypes.iter().enumerate() {
            if !matches!(variable_type, 'C' | 'B' | 'I') {
                return Err(MoiError::InvalidInput(format!(
                    "variable type at index {index} must be 'C', 'B', or 'I', got '{variable_type}'"
                )));
            }
        }
        for (index, (lower, upper)) in lbs.iter().zip(&ubs).enumerate() {
            if lower.is_nan() || upper.is_nan() {
                return Err(MoiError::InvalidInput(format!(
                    "variable bounds at index {index} cannot contain NaN"
                )));
            }
            if lower > upper {
                return Err(MoiError::InvalidInput(format!(
                    "lower bound {lower} exceeds upper bound {upper} for variable {index}"
                )));
            }
        }

        let ids = (start_id..end_id).map(VarId).collect::<Vec<_>>();
        if let Some(backend) = self.backend.as_mut() {
            let result = backend.add_variables(
                n,
                Some(NameType::Vector(names.clone())),
                Some(vtypes.clone()),
                Some(BoundType::Vector(lbs.clone())),
                Some(BoundType::Vector(ubs.clone())),
            );
            let backend_ids = match result {
                Ok(ids) => ids,
                Err(error) => {
                    self.detach_failed_backend();
                    return Err(error);
                }
            };
            if let Err(error) = Self::ensure_expected_var_ids(&backend_ids, start_id) {
                self.detach_failed_backend();
                return Err(error);
            }
        }

        self.vars.extend((0..n).map(|i| VarInfo {
            col_index: start_id + i,
            lb: lbs[i],
            ub: ubs[i],
            vtype: vtypes[i],
            name: names[i].clone(),
            value: None,
        }));
        self.invalidate_solution();
        Ok(ids)
    }

    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<ConstrId>, MoiError> {
        ensure_len(ss.len(), fs.len(), "constraint sets")?;
        let names = match names {
            Some(values) => {
                ensure_len(values.len(), fs.len(), "constraint names")?;
                values
            }
            None => vec![String::new(); fs.len()],
        };
        for (index, name) in names.iter().enumerate() {
            Self::validate_name(name, &format!("constraint name at index {index}"))?;
        }
        for (function, set) in fs.iter().zip(&ss) {
            self.validate_function(function)?;
            Self::validate_set(set)?;
        }

        let start_id = self.constrs.len();
        let end_id = start_id
            .checked_add(fs.len())
            .ok_or_else(|| MoiError::InvalidInput("constraint count overflow".into()))?;
        let ids = (start_id..end_id).map(ConstrId).collect::<Vec<_>>();
        if let Some(backend) = self.backend.as_mut() {
            let result = backend.add_constraints(fs.clone(), ss.clone(), Some(names.clone()));
            let backend_ids = match result {
                Ok(ids) => ids,
                Err(error) => {
                    self.detach_failed_backend();
                    return Err(error);
                }
            };
            if let Err(error) = Self::ensure_expected_constr_ids(&backend_ids, start_id) {
                self.detach_failed_backend();
                return Err(error);
            }
        }

        for (offset, ((f, s), name)) in fs.into_iter().zip(ss).zip(names).enumerate() {
            let id = ConstrId(start_id + offset);
            self.constrs.insert(
                id,
                ConstrInfo {
                    row_index: id.0,
                    name,
                    f,
                    s,
                },
            );
        }
        self.invalidate_solution();
        Ok(ids)
    }

    fn set_objective(&mut self, f: ScalarFunctionType, sense: ModelSense) -> Result<(), MoiError> {
        self.validate_function(&f)?;
        if let Some(backend) = self.backend.as_mut() {
            let result = backend.set_objective(f.clone(), sense);
            if let Err(error) = result {
                self.detach_failed_backend();
                return Err(error);
            }
        }
        self.obj = Some(f);
        self.sense = Some(sense);
        self.invalidate_solution();
        Ok(())
    }

    fn update(&mut self) -> Result<(), MoiError> {
        if let Some(backend) = self.backend.as_mut() {
            let result = backend.update();
            if let Err(error) = result {
                self.detach_failed_backend();
                return Err(error);
            }
        }
        Ok(())
    }

    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue> {
        match attr {
            ModelAttr::ObjectiveSense => self.sense.map(AttrValue::ModelSense),
            ModelAttr::ObjectiveFunction => self.obj.clone().map(AttrValue::ScalarFn),
            ModelAttr::NumberOfVariables => Some(AttrValue::Usize(self.vars.len())),
            ModelAttr::NumberOfConstraints => Some(AttrValue::Usize(self.constrs.len())),
            ModelAttr::ListOfVariableIndices => {
                Some(AttrValue::VecUsize((0..self.vars.len()).collect()))
            }
            ModelAttr::TerminationStatus => self.termination_status.map(AttrValue::Status),
            ModelAttr::ResultCount => Some(AttrValue::Usize(usize::from(
                self.status == BridgeState::Solved
                    && self
                        .backend
                        .as_ref()
                        .and_then(|backend| backend.get_objective_value().ok().flatten())
                        .is_some(),
            ))),
            ModelAttr::ObjectiveValue if self.status == BridgeState::Solved => self
                .backend
                .as_ref()
                .and_then(|backend| backend.get_objective_value().ok().flatten())
                .map(AttrValue::Float),
            ModelAttr::ObjectiveValue => None,
            _ => self.backend.as_ref().and_then(|b| b.get_model_attr(attr)),
        }
    }

    fn set_model_attr(&mut self, attr: ModelAttr, value: AttrValue) -> Result<(), MoiError> {
        match attr {
            ModelAttr::ObjectiveSense => {
                if let AttrValue::ModelSense(s) = value {
                    let objective = self
                        .obj
                        .clone()
                        .unwrap_or_else(|| ScalarFunctionType::Affine(ScalarAffineFn::default()));
                    self.set_objective(objective, s)?;
                } else {
                    return Err(MoiError::InvalidInput(
                        "ObjectiveSense requires AttrValue::ModelSense".to_string(),
                    ));
                }
            }
            ModelAttr::ObjectiveFunction => {
                if let AttrValue::ScalarFn(f) = value {
                    self.set_objective(f, self.sense.unwrap_or(ModelSense::Minimize))?;
                } else {
                    return Err(MoiError::InvalidInput(
                        "ObjectiveFunction requires AttrValue::ScalarFn".to_string(),
                    ));
                }
            }
            _ => {
                if let Some(ref mut b) = self.backend {
                    b.set_model_attr(attr, value)?;
                }
            }
        }
        Ok(())
    }

    fn get_optimizer_attr(&self, attr: OptimizerAttr) -> Option<AttrValue> {
        self.backend
            .as_ref()
            .and_then(|backend| backend.get_optimizer_attr(attr.clone()))
            .or_else(|| self.raw_params.get(&attr).cloned())
    }

    fn set_optimizer_attr(
        &mut self,
        attr: OptimizerAttr,
        value: AttrValue,
    ) -> Result<(), MoiError> {
        if let OptimizerAttr::Raw(name) = &attr {
            Self::validate_name(name, "optimizer parameter name")?;
        }
        match &value {
            AttrValue::Float(value) if !value.is_finite() => {
                return Err(MoiError::InvalidInput(
                    "optimizer parameter value must be finite".into(),
                ));
            }
            AttrValue::String(value) => {
                Self::validate_name(value, "optimizer parameter string value")?;
            }
            _ => {}
        }
        if let Some(ref mut b) = self.backend
            && let Err(error) = b.set_optimizer_attr(attr.clone(), value.clone())
        {
            self.detach_failed_backend();
            return Err(error);
        }
        self.raw_params.insert(attr, value);
        self.invalidate_solution();
        Ok(())
    }
}

impl Optimizer for BridgeOptimizer {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError> {
        self.invalidate_solution();
        match self.backend {
            Some(ref mut b) => {
                let status = b.optimize()?;
                self.termination_status = Some(status);
                self.status = BridgeState::Solved;
                Ok(status)
            }
            None => Err(MoiError::Msg("No backend solver attached".to_string())),
        }
    }

    fn compute_conflict(&mut self) -> Result<(), MoiError> {
        match self.backend {
            Some(ref mut b) => b.compute_conflict(),
            None => Err(MoiError::Msg("No backend solver attached".to_string())),
        }
    }

    fn get_var_value(&self, var_id: VarId) -> Result<Option<f64>, MoiError> {
        if var_id.0 >= self.vars.len() {
            return Err(MoiError::InvalidVariableIndex(var_id.0));
        }
        if self.status != BridgeState::Solved {
            return Ok(None);
        }
        self.backend
            .as_ref()
            .ok_or_else(|| MoiError::BackendState("solved model has no attached backend".into()))?
            .get_var_value(var_id)
    }

    fn get_objective_value(&self) -> Result<Option<f64>, MoiError> {
        if self.status != BridgeState::Solved {
            return Ok(None);
        }
        self.backend
            .as_ref()
            .ok_or_else(|| MoiError::BackendState("solved model has no attached backend".into()))?
            .get_objective_value()
    }
}
