use crate::constr::Constr;
use crate::moi::*;
use crate::utils::*;
use crate::var::*;
use moi_bridge::BridgeOptimizer;
use moi_core::*;
use moi_solver_api::*;
use pyo3::prelude::*;
use pyo3::types::{PyAny,PyTuple};

#[pyclass]
#[derive(Debug)]
pub struct Model {
    name: String,
    model: BridgeOptimizer,
    backend: Option<Py<PyAny>>,
}

#[pymethods]
impl Model {
    #[new]
    fn new(name: String) -> Self {
        Model {
            name,
            model: BridgeOptimizer::new(),
            backend: None,
        }
    }
    fn _set_backend(&mut self, backend: Py<PyAny>) {
        self.backend = Some(backend);
    }
    #[pyo3(signature = (lb=0., ub=std::f64::INFINITY, obj=0.0, vtype=None, name=""),name="addVar")]
    fn add_var(
        &mut self,
        lb: f64,
        ub: f64,
        obj: f64,
        vtype: Option<VarType>,
        name: &str,
    ) -> PyResult<Var> {
        let var_id = self.model.add_variable(
            Some(&name),
            vtype.map(|t| match t {
                VarType::CONTINUOUS => 'C',
                VarType::BINARY => 'B',
                VarType::INTEGER => 'I',
            }),
            Some(lb),
            Some(ub),
        );
        Ok(Var::new(var_id.0))
    }
    #[pyo3(signature = (*indices, lb=None, ub=None, obj=None, vtype=None, name=None),name="addVars")]
    fn add_vars<'py>(
        &mut self,
        indices: &Bound<'py, PyTuple>,
        lb: Option<&Bound<'py, PyAny>>,
        ub: Option<&Bound<'py, PyAny>>,
        obj: Option<&Bound<'py, PyAny>>,
        vtype: Option<&Bound<'py, PyAny>>,
        name: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Vars> {
        let shape_vec: Vec<usize> = indices.extract()?;

        let num_vars = shape_vec.iter().product();
        let lb_param = lb
            .map(|obj| Param::from_py(obj))
            .transpose()?
            .unwrap_or(Param::Vector(vec![0.0; num_vars]));
        let ub_param = ub
            .map(|obj| Param::from_py(obj))
            .transpose()?
            .unwrap_or(Param::Vector(vec![f64::INFINITY; num_vars]));
        // TODO：目前暂不实现添加目标函数当中的参数，后续可以考虑添加一个专门的接口来设置目标函数参数
        let _ = obj
            .map(|obj| Param::from_py(obj))
            .transpose()?
            .unwrap_or(Param::Vector(vec![0.0; num_vars]));
        let vtype_param = vtype
            .map(|obj| Param::from_py(obj))
            .transpose()?
            .unwrap_or(Param::Vector(vec![VarType::CONTINUOUS; num_vars]));
        let name_param = name
            .map(|obj| Param::from_py(obj))
            .transpose()?
            .unwrap_or(Param::Scalar("".to_string()));
        // 如果传入参数为单一字符串，则按照shape生成a[0],a[1]或a[0,0],a[0,1]等变量名称
        let name_param = if let Param::Scalar(s) = &name_param {
            let names = generate_names(s, &shape_vec);
            Param::Vector(names)
        } else {
            name_param
        };
        let varids = self.model.add_variables(
            num_vars,
            Some(NameType::Vector(name_param.to_vec(Some(num_vars)))),
            Some(
                vtype_param
                    .to_vec(Some(num_vars))
                    .iter()
                    .map(|t| match t {
                        VarType::CONTINUOUS => 'C',
                        VarType::BINARY => 'B',
                        VarType::INTEGER => 'I',
                    })
                    .collect(),
            ),
            Some(BoundType::Vector(lb_param.to_vec(Some(num_vars)))),
            Some(BoundType::Vector(ub_param.to_vec(Some(num_vars)))),
        );
        let var_ids = varids.into_iter().map(|id| VarId(id.0)).collect();
        Ok(Vars::new(shape_vec, var_ids))
    }

    #[pyo3(signature = (constr, name=None),name="addConstr")]
    fn add_constr(&mut self, constr: &Bound<'_, Constr>, name: Option<&str>) -> PyResult<()> {
        let constr: Constr = constr.extract()?;
        self.model
            .add_constraint(constr.get_f(), constr.get_s(), name.map(|s| s.to_string()));
        Ok(())
    }
    #[pyo3(signature = (generator, name=None),name="addConstrs")]
    fn add_constrs(
        &mut self,
        generator: &Bound<'_, PyAny>,
        name: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let mut fs = Vec::new();
        let mut ss = Vec::new();
        let mut count = 0;
        let _ = generator
            .try_iter()?
            .map(|item| {
                let constr: Constr = item?.extract()?;
                fs.push(constr.get_f());
                ss.push(constr.get_s());
                count += 1;
                Ok(())
            })
            .collect::<PyResult<Vec<_>>>()?;
        let shape_vec = vec![count];
        let name_param = name
            .map(|obj| Param::from_py(obj))
            .transpose()?
            .unwrap_or(Param::Scalar("Cons".to_string()));
        // 如果传入参数为单一字符串，则按照shape生成a[0],a[1]或a[0,0],a[0,1]等约束名称
        let name_param = if let Param::Scalar(s) = &name_param {
            let names = generate_names(s, &shape_vec);
            Param::Vector(names)
        } else {
            name_param
        };

        self.model
            .add_constraints(fs, ss, Some(name_param.to_vec(Some(count))));
        Ok(())
        
    }
    fn __str__(&self) -> PyResult<String> {
        Ok(format!("Model(name={})", self.name))
    }
}
