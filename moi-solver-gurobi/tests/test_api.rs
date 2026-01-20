use moi_solver_gurobi::bindings::*;
use moi_solver_gurobi::dynamic::api::GurobiApi;
use std::ffi::{c_char, c_double, c_int, c_void};
use std::path::PathBuf;
use std::ptr::{null, null_mut};

#[test]
fn test_api_mip() {
    let api = GurobiApi::new(PathBuf::from("/usr/local/gurobi1203/lib/libgurobi120.so")).unwrap();
    unsafe {
        let mut ret = 0;
        // 创建环境
        let mut env: *mut c_void = null_mut();
        ret = (api.GRBloadenv)(&mut env as *mut *mut c_void, null());
        assert_eq!(ret, 0);
        // 创建模型
        let mut model: *mut c_void = null_mut();
        ret = (api.GRBnewmodel)(
            env,
            &mut model as *mut *mut c_void,
            b"test_model\0".as_ptr() as *const c_char,
            0,
            null(),
            null(),
            null(),
            null(),
            null(),
        );
        assert_eq!(ret, 0);
        let obj = [1., 1., 2.];
        // 添加变量
        let vtype = [GRB_BINARY,GRB_BINARY,GRB_BINARY];
        ret = (api.GRBaddvars)(
            model,
            3,
            0,
            null(),
            null(),
            null(),
            obj.as_ptr(),
            null(),
            null(),
            vtype.as_ptr() as *const i8,
            null(),
        );
        assert_eq!(ret, 0);
        // 最大化
        ret = (api.GRBsetintattr)(
            model,
            GRB_INT_ATTR_MODELSENSE.as_ptr() as *const c_char,
            GRB_MAXIMIZE,
        );
        assert_eq!(ret, 0);
        // 添加约束
        let ind = [0, 1, 2];
        let val = [1., 2., 3.];
        ret = (api.GRBaddconstr)(
            model,
            3,
            ind.as_ptr(),
            val.as_ptr(),
            GRB_LESS_EQUAL as i8,
            4.,
            "c0".as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);
        let ind = [0, 1];
        let val = [1., 1.];
        ret = (api.GRBaddconstr)(
            model,
            2,
            ind.as_ptr(),
            val.as_ptr(),
            GRB_GREATER_EQUAL as i8,
            1.,
            "c1".as_ptr() as *const c_char,
        );
        assert_eq!(ret, 0);
        // 优化
        ret = (api.GRBoptimize)(model);
        assert_eq!(ret, 0);
        // 求解信息
        let mut status = 0;
        ret = (api.GRBgetintattr)(
            model,
            GRB_INT_ATTR_STATUS.as_ptr() as *const c_char,
            &mut status,
        );
        assert_eq!(ret, 0);
        let mut objval: c_double = 0.;
        ret = (api.GRBgetdblattr)(
            model,
            GRB_DBL_ATTR_OBJVAL.as_ptr() as *const c_char,
            &mut objval as *mut c_double,
        );
        assert_eq!(ret, 0);
        let mut x = [0.; 3];
        ret = (api.GRBgetdblattrarray)(
            model,
            GRB_DBL_ATTR_X.as_ptr() as *const c_char,
            0,
            3,
            x.as_mut_ptr(),
        );
        assert_eq!(ret, 0);
        if status as u32 == GRB_OPTIMAL {
            println!("Optimal objective value: {}", objval);
            println!("Optimal solution: {:?}", x);
        } else {
            panic!("No optimal solution found");
        }
        (api.GRBfreemodel)(model);
        (api.GRBfreeenv)(env);
    }
}
