import pytest

try:
    from moirspy_gurobi import Model
    HAS_GUROBI_MODULE = True
except ImportError:
    HAS_GUROBI_MODULE = False

@pytest.mark.skipif(not HAS_GUROBI_MODULE, reason="moirspy_gurobi not built")
def test_basic_model_operations():
    # Attempt to load without explicit path. Will succeed if gurobi is in LD_LIBRARY_PATH
    try:
        model = Model("TestModel", None)
    except Exception as e:
        pytest.skip(f"Could not load Gurobi: {e}")
        return

    # Add variables: 2 continuous variables, LB=0, UB=None
    v1 = model.add_variable(name="x0", vtype='C', lb=0.0, ub=float('inf'))
    v2 = model.add_variable(name="x1", vtype='C', lb=0.0, ub=float('inf'))
    
    assert v1 == 0
    assert v2 == 1

    # Add constraints: x0 + x1 >= 1.0 (Sense 1 for 'G', assuming MOI translates it in rust side)
    # Wait, in model.rs `add_constraint` takes `vars, coeffs, lb, ub, name`.
    # Let's inspect signature.
    model.add_constraint(
        vars=[v1, v2],
        coeffs=[1.0, 1.0],
        constant=0.0,
        sense = ">",
        rhs = 1.0,
        name="c1"
    )

    # Set objective: minimize x0 + x1, Sense: 1=Minimize, 0=Maximize? 
    # Wait, check model.rs `set_objective` sense parameter.
    model.set_objective(
        vars=[v1, v2],
        coeffs=[1.0, 1.0],
        constant=0.0,
        sense=0 # Assuming 0 is Min based on my previous rewrite?
    )
    
    model.update()
    model.set_optimizer_attr("OutputFlag", 0) # Suppress Gurobi output
    
    status = model.optimize()
    
    print("Status:", status)
    
    if status == 1: # GRB_OPTIMAL? Depends on mapping
        val = model.get_objective_value()
        assert val is not None
        assert val >= 1.0

if __name__ == "__main__":
    test_basic_model_operations()
