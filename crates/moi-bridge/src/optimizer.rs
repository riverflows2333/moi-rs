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
    pub backend: Option<Box<dyn Optimizer>>,
    pub raw_params: HashMap<OptimizerAttr, AttrValue>,
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
        }
    }

    /// 关联后端求解器并执行“大冲刷”（一次性同步所有本地缓存的状态）
    pub fn attach_backend(&mut self, mut backend: Box<dyn Optimizer>) -> Result<(), MoiError> {
        // 1. 同步变量
        if !self.vars.is_empty() {
            let n = self.vars.len();
            let names: Vec<String> = self.vars.iter().map(|v| v.name.clone()).collect();
            let vtypes: Vec<char> = self.vars.iter().map(|v| v.vtype).collect();
            let lbs: Vec<f64> = self.vars.iter().map(|v| v.lb).collect();
            let ubs: Vec<f64> = self.vars.iter().map(|v| v.ub).collect();

            backend.add_variables(
                n,
                Some(NameType::Vector(names)),
                Some(vtypes),
                Some(BoundType::Vector(lbs)),
                Some(BoundType::Vector(ubs)),
            );
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

            backend.add_constraints(fs, ss, Some(names));
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
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.vars.is_empty() && self.constrs.is_empty()
    }

    pub fn get_var_name_by_id(&self, id: VarId) -> Option<String> {
        self.vars.get(id.0).map(|var| var.name.clone())
    }
}

impl ModelLike for BridgeOptimizer {
    fn add_variable(
        &mut self,
        name: Option<&str>,
        vtype: Option<char>,
        lb: Option<f64>,
        ub: Option<f64>,
    ) -> VarId {
        let var_id = self.vars.len();
        let info = VarInfo {
            col_index: var_id,
            lb: lb.unwrap_or(0.0),
            ub: ub.unwrap_or(f64::INFINITY),
            vtype: vtype.unwrap_or('C'),
            name: name.unwrap_or("").to_string(),
            value: None,
        };
        self.vars.push(info.clone());

        if self.status == BridgeState::Synced {
            if let Some(ref mut b) = self.backend {
                b.add_variable(name, vtype, lb, ub);
            }
        }
        VarId(var_id)
    }

    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Vec<VarId> {
        let start_id = self.vars.len();
        // 简化实现：循环调用 add_variable 以确保逻辑一致
        // 在高性能需求下，这里可以优化为直接构建批量数据并同步至 backend
        let mut ids = Vec::with_capacity(n);
        for i in 0..n {
            let cur_name = match &name {
                Some(NameType::Single(s)) if !s.is_empty() => Some(format!("{}_{}", s, i)),
                Some(NameType::Vector(v)) => Some(v[i].clone()),
                _ => None,
            };
            let cur_vtype = vtype.as_ref().map(|v| v[i]);
            let cur_lb = match &lb {
                Some(BoundType::Single(val)) => Some(*val),
                Some(BoundType::Vector(v)) => Some(v[i]),
                None => None,
            };
            let cur_ub = match &ub {
                Some(BoundType::Single(val)) => Some(*val),
                Some(BoundType::Vector(v)) => Some(v[i]),
                None => None,
            };
            ids.push(self.add_variable(cur_name.as_deref(), cur_vtype, cur_lb, cur_ub));
        }
        ids
    }

    fn add_constraint(
        &mut self,
        f: ScalarFunctionType,
        s: ScalarSetType,
        name: Option<String>,
    ) -> ConstrId {
        let constr_id = ConstrId(self.constrs.len());
        let info = ConstrInfo {
            row_index: constr_id.0,
            name: name.clone().unwrap_or_default(),
            f: f.clone(),
            s: s.clone(),
        };
        self.constrs.insert(constr_id, info);

        if self.status == BridgeState::Synced {
            if let Some(ref mut b) = self.backend {
                b.add_constraint(f, s, name);
            }
        }
        constr_id
    }

    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Vec<ConstrId> {
        let mut ids = Vec::with_capacity(fs.len());
        let names_vec = names.unwrap_or_else(|| vec!["".to_string(); fs.len()]);
        for ((f, s), n) in fs
            .into_iter()
            .zip(ss.into_iter())
            .zip(names_vec.into_iter())
        {
            ids.push(self.add_constraint(f, s, Some(n)));
        }
        ids
    }

    fn set_objective(&mut self, f: ScalarFunctionType, sense: ModelSense) -> Result<(), MoiError> {
        self.obj = Some(f.clone());
        self.sense = Some(sense);
        if self.status == BridgeState::Synced {
            if let Some(ref mut b) = self.backend {
                b.set_objective(f, sense)?;
            }
        }
        Ok(())
    }

    fn update(&mut self) -> Result<(), MoiError> {
        if let Some(ref mut b) = self.backend {
            b.update()?;
        }
        Ok(())
    }

    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue> {
        if let Some(ref b) = self.backend {
            b.get_model_attr(attr)
        } else {
            match attr {
                ModelAttr::ObjectiveSense => self.sense.map(AttrValue::ModelSense),
                ModelAttr::ObjectiveFunction => self.obj.clone().map(AttrValue::ScalarFn),
                _ => None,
            }
        }
    }

    fn set_model_attr(&mut self, attr: ModelAttr, value: AttrValue) -> Result<(), MoiError> {
        match attr {
            ModelAttr::ObjectiveSense => {
                if let AttrValue::ModelSense(s) = value {
                    self.set_objective(self.obj.clone().unwrap(), s)?;
                }
            }
            ModelAttr::ObjectiveFunction => {
                if let AttrValue::ScalarFn(f) = value {
                    self.set_objective(f, self.sense.unwrap_or(ModelSense::Minimize))?;
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
        if let Some(ref b) = self.backend {
            b.get_optimizer_attr(attr)
        } else {
            self.raw_params.get(&attr).cloned()
        }
    }

    fn set_optimizer_attr(
        &mut self,
        attr: OptimizerAttr,
        value: AttrValue,
    ) -> Result<(), MoiError> {
        self.raw_params.insert(attr.clone(), value.clone());
        if let Some(ref mut b) = self.backend {
            b.set_optimizer_attr(attr, value)?;
        }
        Ok(())
    }
}

impl Optimizer for BridgeOptimizer {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError> {
        match self.backend {
            Some(ref mut b) => b.optimize(),
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
        self.backend.as_ref().and_then(|b| b.get_var_value(var_id))
    }

    fn get_objective_value(&self) -> Option<f64> {
        self.backend.as_ref().and_then(|b| b.get_objective_value())
    }
}
