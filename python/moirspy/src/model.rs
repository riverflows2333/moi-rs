use crate::constr::Constr;
use crate::expr::LinExpr;
use crate::moi::*;
use crate::utils::*;
use crate::var::*;
use bincode::config;
use moi_bridge::BridgeOptimizer;
use moi_core::*;
use moi_solver_api::*;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyTuple};
use std::sync::{Arc, RwLock};
#[pyclass]
#[derive(Debug)]
pub struct Model {
    name: String,
    model: SharedBridge,
    backend: Option<Py<PyAny>>,
}

#[pymethods]
impl Model {
    #[new]
    fn new(name: String) -> Self {
        Model {
            name,
            model: Arc::new(RwLock::new(BridgeOptimizer::new())),
            backend: None,
        }
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
        let mut model = self.model.write().unwrap();
        let var_id = model.add_variable(
            Some(&name),
            vtype.map(|t| match t {
                VarType::CONTINUOUS => 'C',
                VarType::BINARY => 'B',
                VarType::INTEGER => 'I',
            }),
            Some(lb),
            Some(ub),
        );
        let mut var = Var::new(var_id.0);
        var.set_bridge(&self.model);
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
        let mut model = self.model.write().unwrap();
        let varids = model.add_variables(
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
        let mut vars = Vars::new(shape_vec.clone(), var_ids);
        vars.set_bridge(&self.model);
        Ok(vars)
    }

    #[pyo3(signature = (constr, name=None),name="addConstr")]
    fn add_constr(&mut self, constr: &Bound<'_, Constr>, name: Option<&str>) -> PyResult<()> {
        let constr: Constr = constr.extract()?;

        let mut model = self.model.write().unwrap();
        model.add_constraint(constr.get_f(), constr.get_s(), name.map(|s| s.to_string()));
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

        let mut model = self.model.write().unwrap();
        model.add_constraints(fs, ss, Some(name_param.to_vec(Some(count))));
        Ok(())
    }
    #[pyo3(signature = (expr, sense),name="setObjective")]
    fn set_objective(&mut self, expr: &Bound<'_, PyAny>, sense: Sense) -> PyResult<()> {
        let obj_expr = expr.extract::<LinExpr>()?;
        let mut model = self.model.write().unwrap();
        let _ = model.set_model_attr(
            ModelAttr::ObjectiveFunction,
            AttrValue::ScalarFn(ScalarFunctionType::Affine(obj_expr.get_fn())),
        );
        let _ = model.set_model_attr(
            ModelAttr::ObjectiveSense,
            AttrValue::ModelSense(match sense {
                Sense::MINIMIZE => ModelSense::Minimize,
                Sense::MAXIMIZE => ModelSense::Maximize,
            }),
        );
        Ok(())
    }
    // 选择求解器后端
    #[pyo3(name = "setBackend")]
    fn set_backend(&mut self, py: Python, backend: &str) {
        // 通过Python attach搜索库中包含的moirspy-后端名称的模块，并调用其Model类创建对象；
        let model_instance = py.import(&format!("moirspy_{}", backend))
            .and_then(|module| module.getattr("Model"))
            .and_then(|model_class| model_class.call0())
            .expect(&format!("Failed to set backend to '{}'. Please ensure the corresponding module is available.", backend));
        // 将BridgeOptimizer进行编码，在后端Model当中进行解码并更新模型；
        let encoded_model = self.encode().expect("Failed to encode model");
        model_instance
            .call_method1("decode_and_update", (encoded_model,))
            .expect("Failed to update model in backend");
        // 将后端Model的实例保存到当前Model的backend属性当中，以便后续调用求解等接口时使用
        self.backend = Some(model_instance.into());
    }
    // 调用底层求解器进行优化
    fn optimize(&mut self, py: Python) -> PyResult<()> {
        if let Some(ref backend) = self.backend {
            let result_obj = backend
                .call_method0(py, "optimize")
                .expect("Failed to call optimize on backend");
            if let Ok(result_dict) = result_obj.cast_bound::<PyDict>(py) {
                // 2. 解析返回的载荷
                let _: String = result_dict
                    .get_item("status")?
                    .unwrap()
                    .extract::<String>()?;
                let obj_val: f64 = result_dict
                    .get_item("objval")?
                    .unwrap()
                    .extract::<f64>()?;
                let x_values: Vec<f64> = result_dict
                    .get_item("x_values")?
                    .unwrap()
                    .extract::<Vec<f64>>()?;

                // 3. 核心：获取写锁，更新 moipy 自己的 BridgeOptimizer
                let mut bridge = self.model.write().unwrap(); // 这里的 bridge 是 SharedBridge
                bridge.objval = Some(obj_val);
                // 将结果回填到各个变量的 VarInfo 中
                for (i, &val) in x_values.iter().enumerate() {
                    bridge.vars.get_mut(i).map(|var_info| {
                        var_info.value = Some(val);
                    });
                }
                // dbg!(&bridge.vars.iter().map(|v| v.value).collect::<Vec<_>>());
            } else {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Backend optimize() did not return a dictionary with results.",
                ));
            }
            Ok(())
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "No backend set. Please call setBackend() before optimizing.",
            ))
        }
    }
    fn __str__(&self) -> PyResult<String> {
        Ok(format!("Model(name={})", self.name))
    }
    #[getter]
    #[pyo3(name = "ObjVal")]
    pub fn get_objval(&self) -> PyResult<Option<f64>> {
        let bridge = self.model.read().unwrap();
        Ok(bridge.objval)
    }
}

impl Model {
    pub fn encode(&self) -> PyResult<Vec<u8>> {
        let config = config::standard();
        bincode::encode_to_vec(&self.model, config).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("Serialization error: {}", e))
        })
    }
}
