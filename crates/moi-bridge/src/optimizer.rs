use moi_core::attributes::{AttrValue, ModelAttr, OptimizerAttr};
use moi_core::*;
use moi_solver_api::*;
use std::collections::HashMap;

/// 表示桥接优化器的同步状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeState {
    /// 尚未关联后端求解器，仅在本地缓存模型变更
    NotAttached,
    /// 已关联后端求解器，所有变更将实时同步
    Synced,
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
    solution_valid: bool,
}

impl BridgeOptimizer {
    pub fn new() -> Self {
        Self {
            vars: Vec::new(),
            constrs: HashMap::new(),
            obj: None,
            sense: None,
            status: BridgeState::NotAttached,
            backend: None,
            raw_params: HashMap::new(),
            solution_valid: false,
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
        self.status = BridgeState::Synced;
        self.solution_valid = false;
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.vars.is_empty() && self.constrs.is_empty()
    }

    pub fn get_var_name_by_id(&self, id: VarId) -> Option<String> {
        self.vars.get(id.0).map(|var| var.name.clone())
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
        let linear = scalar_function_to_linear(function)?;
        if let Some(var) = linear
            .variables
            .into_iter()
            .find(|var| var.0 >= self.vars.len())
        {
            return Err(MoiError::InvalidVariableIndex(var.0));
        }
        Ok(())
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

        let ids = (start_id..start_id + n).map(VarId).collect::<Vec<_>>();
        if let Some(ref mut backend) = self.backend {
            let backend_ids = backend.add_variables(
                n,
                Some(NameType::Vector(names.clone())),
                Some(vtypes.clone()),
                Some(BoundType::Vector(lbs.clone())),
                Some(BoundType::Vector(ubs.clone())),
            )?;
            Self::ensure_expected_var_ids(&backend_ids, start_id)?;
        }

        self.vars.extend((0..n).map(|i| VarInfo {
            col_index: start_id + i,
            lb: lbs[i],
            ub: ubs[i],
            vtype: vtypes[i],
            name: names[i].clone(),
            value: None,
        }));
        self.solution_valid = false;
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
        for function in &fs {
            self.validate_function(function)?;
        }

        let start_id = self.constrs.len();
        let ids = (start_id..start_id + fs.len())
            .map(ConstrId)
            .collect::<Vec<_>>();
        if let Some(ref mut backend) = self.backend {
            let backend_ids =
                backend.add_constraints(fs.clone(), ss.clone(), Some(names.clone()))?;
            Self::ensure_expected_constr_ids(&backend_ids, start_id)?;
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
        self.solution_valid = false;
        Ok(ids)
    }

    fn set_objective(&mut self, f: ScalarFunctionType, sense: ModelSense) -> Result<(), MoiError> {
        self.validate_function(&f)?;
        if let Some(ref mut backend) = self.backend {
            backend.set_objective(f.clone(), sense)?;
        }
        self.obj = Some(f.clone());
        self.sense = Some(sense);
        self.solution_valid = false;
        Ok(())
    }

    fn update(&mut self) -> Result<(), MoiError> {
        if let Some(ref mut b) = self.backend {
            b.update()?;
        }
        Ok(())
    }

    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue> {
        match attr {
            ModelAttr::ObjectiveSense => self.sense.map(AttrValue::ModelSense),
            ModelAttr::ObjectiveFunction => self.obj.clone().map(AttrValue::ScalarFn),
            ModelAttr::NumberOfVariables => Some(AttrValue::Usize(self.vars.len())),
            ModelAttr::NumberOfConstraints => Some(AttrValue::Usize(self.constrs.len())),
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
        if let Some(ref mut b) = self.backend {
            b.set_optimizer_attr(attr.clone(), value.clone())?;
        }
        self.raw_params.insert(attr, value);
        self.solution_valid = false;
        Ok(())
    }
}

impl Optimizer for BridgeOptimizer {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError> {
        match self.backend {
            Some(ref mut b) => {
                let status = b.optimize()?;
                self.solution_valid = true;
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

    fn get_var_value(&self, var_id: VarId) -> Option<f64> {
        self.solution_valid
            .then(|| self.backend.as_ref().and_then(|b| b.get_var_value(var_id)))
            .flatten()
    }

    fn get_objective_value(&self) -> Option<f64> {
        self.solution_valid
            .then(|| self.backend.as_ref().and_then(|b| b.get_objective_value()))
            .flatten()
    }
}
