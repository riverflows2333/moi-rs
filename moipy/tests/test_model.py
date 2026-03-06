from moipy import MOI, Model, quicksum
from unittest import TestCase


class TestModel(TestCase):
    def test_model_import(self):
        model = Model("test_model")
        print(model)

    def test_add_var(self):
        model = Model("test_model")
        a = model.addVar(name="a")
        print(a)

    def test_add_vars(self):
        model = Model("test_model")
        x = model.addVars(2, 3, lb=0.0, ub=10.0, name="x", vtype=MOI.CONTINUOUS)
        print(x[0, 2])

    def test_add_vars_hybrid(self):
        model = Model("test_model")
        x = model.addVars(
            3,
            lb=[0.0, 1.0, 2.0],
            ub=[10.0, 20.0, 30.0],
            name="x",
            vtype=[MOI.CONTINUOUS, MOI.INTEGER, MOI.BINARY],
        )
        y = model.addVar(name="y")
        model.addConstr(x[0] + x[1] + x[2] <= 10.0, name="c1")
        model.addConstrs((x[i] >= 0.0 for i in range(3)), name="c2")
        print(x[2])
        print(y)

    def test_optimize(self):
        model = Model("test_model")
        x = model.addVars(3, name="x", vtype=MOI.BINARY)
        model.addConstr(x[0] + 2 * x[1] + 3 * x[2] <= 4.0, name="c1")
        model.addConstr(x[0] + x[1] >= 1.0, name="c2")
        model.setObjective(x[0] + x[1] + 2 * x[2], sense=MOI.MAXIMIZE)
        model.setBackend("gurobi")
        model.optimize()
        print(x[0].x)
        print(x[1].x)
        print(x[2].x)


    def test_quicksum(self):
        model = Model("test_model")
        x = model.addVars(3, name="x", vtype=MOI.BINARY)
        model.addConstr(quicksum(x[i] for i in range(3)) <= 2.0, name="c1")
        model.setObjective(quicksum(x[i] for i in range(3)), sense=MOI.MAXIMIZE)
        model.setBackend("gurobi")
        model.optimize()
        

if __name__ == "__main__":
    import unittest

    unittest.main()
