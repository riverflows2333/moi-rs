use moi_core::attributes::{AttrValue, ModelAttr};
use moi_core::*;
use moi_solver_api::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Refactored BridgeOptimizer to hold model state.
#[derive(Clone, Debug,Serialize,Deserialize)]
pub struct BridgeOptimizer {
    pub vars: Vec<VarInfo>,
    pub constrs: HashMap<ConstrId, ConstrInfo>,
    pub obj: Option<ScalarFunctionType>,
    pub sense: Option<ModelSense>,
    pub needs_update: bool,
}

impl BridgeOptimizer {
    pub fn new() -> Self {
        Self {
            vars: Vec::new(),
            constrs: HashMap::new(),
            obj: None,
            sense: None,
            needs_update: false,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.vars.is_empty() && self.constrs.is_empty()
    }

    pub fn empty(&mut self) {
        self.vars.clear();
        self.constrs.clear();
        self.obj = None;
        self.sense = None;
        self.needs_update = false;
    }

    pub fn get_var_name_by_id(&self, id: VarId) -> Option<String> {
        self.vars.get(id.0).map(|var| var.name.clone())
    }
}

impl BridgeOptimizer {
    /// Explicit API: add an affine scalar-bound constraint.
    /// This converts the bound to a standard constraint and adds it.
    pub fn add_affine_bound(&mut self, f: ScalarAffineFn, s: ScalarSetType) -> ConstrId {
        let fty = ScalarFunctionType::Affine(f);
        self.add_constraint(fty, s, None)
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
        self.vars.push(VarInfo {
            col_index: var_id,
            lb: lb.unwrap_or(0.0),
            ub: ub.unwrap_or(f64::INFINITY),
            vtype: vtype.unwrap_or('C'),
            name: name.unwrap_or("").to_string(),
            value: None,
        });
        self.needs_update = true;
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
        let lb_vec = match lb {
            Some(BoundType::Single(val)) => vec![val; n],
            Some(BoundType::Vector(vec)) => vec,
            None => vec![0.0; n],
        };
        let ub_vec = match ub {
            Some(BoundType::Single(val)) => vec![val; n],
            Some(BoundType::Vector(vec)) => vec,
            None => vec![f64::INFINITY; n],
        };

        let get_vtype = |i: usize| -> char {
            match &vtype {
                Some(v) => v[i],
                None => 'C',
            }
        };

        for i in 0..n {
            let var_id = start_id + i;
            let current_name = match &name {
                Some(NameType::Single(s)) => {
                    if s.is_empty() {
                        format!("var_x_{}", var_id)
                    } else {
                        format!("{}_{}", s, i)
                    }
                }
                Some(NameType::Vector(vec)) => vec[i].clone(),
                None => "".to_string(),
            };
            self.vars.push(VarInfo {
                col_index: var_id,
                lb: lb_vec[i],
                ub: ub_vec[i],
                vtype: get_vtype(i),
                name: current_name,
                value: None,
            });
        }
        self.needs_update = true;
        (start_id..start_id + n).map(VarId).collect()
    }

    fn add_constraint(
        &mut self,
        f: ScalarFunctionType,
        s: ScalarSetType,
        name: Option<String>,
    ) -> ConstrId {
        let constr_id = self.constrs.len();
        self.constrs.insert(
            ConstrId(constr_id),
            ConstrInfo {
                row_index: constr_id,
                name: name.unwrap_or_default(),
                f,
                s,
            },
        );
        self.needs_update = true;
        ConstrId(constr_id)
    }

    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Vec<ConstrId> {
        let names = names.unwrap_or_else(|| vec!["".to_string(); fs.len()]);
        fs.into_iter()
            .zip(ss.into_iter())
            .zip(names.into_iter())
            .map(|((f, s), n)| self.add_constraint(f, s, Some(n)))
            .collect()
    }

    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue> {
        match attr {
            ModelAttr::ObjectiveSense => self.sense.map(AttrValue::ModelSense),
            ModelAttr::ObjectiveFunction => self.obj.clone().map(AttrValue::ScalarFn),
            _ => None,
        }
    }

    fn set_model_attr(&mut self, attr: ModelAttr, value: AttrValue) -> Result<(), MoiError> {
        match attr {
            ModelAttr::ObjectiveSense => {
                if let AttrValue::ModelSense(sense) = value {
                    self.sense = Some(sense);
                    self.needs_update = true;
                    Ok(())
                } else {
                    Err(MoiError::Msg(
                        "Invalid attribute value type for ObjectiveSense".to_string(),
                    ))
                }
            }
            ModelAttr::ObjectiveFunction => {
                if let AttrValue::ScalarFn(f) = value {
                    self.obj = Some(f);
                    self.needs_update = true;
                    Ok(())
                } else {
                    Err(MoiError::Msg(
                        "Invalid attribute value type for ObjectiveFunction".to_string(),
                    ))
                }
            }
            _ => Err(MoiError::Msg(
                "Setting this model attribute is not supported".to_string(),
            )),
        }
    }
}
