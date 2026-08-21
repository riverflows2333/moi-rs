use moi_bridge::BridgeOptimizer;
use moi_core::{
    AffineTerm, AttrValue, ModelSense, OptimizerAttr, ScalarAffineFn, ScalarFunctionType,
    ScalarSetType, SolveStatus, VarId,
};
use moi_model_dummy::DummyModel;
use moi_solver_api::{BoundType, ModelLike, Optimizer};
use moi_solver_copt::{CoptApi, CoptEnv, CoptOptimizer, find_library};
use std::sync::{Arc, Mutex};

type TestEnv = Arc<Mutex<CoptEnv>>;

fn configured_environment() -> Option<TestEnv> {
    let path = find_library()?;
    let api = Arc::new(CoptApi::new(path).ok()?);
    Some(Arc::new(Mutex::new(CoptEnv::new(api).ok()?)))
}

fn configured_optimizer(env: &TestEnv) -> Option<CoptOptimizer> {
    let mut optimizer = CoptOptimizer::new(env.clone()).ok()?;
    optimizer
        .set_optimizer_attr(OptimizerAttr::Silent, AttrValue::Bool(true))
        .ok()?;
    Some(optimizer)
}

fn next_value(state: &mut u64, upper: u32) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1);
    ((*state >> 32) as u32 % upper + 1) as f64
}

fn affine(coefficients: &[f64]) -> ScalarFunctionType {
    ScalarFunctionType::Affine(ScalarAffineFn {
        terms: coefficients
            .iter()
            .enumerate()
            .map(|(index, coefficient)| AffineTerm {
                var: VarId(index),
                coeff: *coefficient,
            })
            .collect(),
        constant: 0.0,
    })
}

fn brute_force_knapsack(profits: &[f64], weights: &[f64], capacity: f64) -> f64 {
    (0..(1usize << profits.len()))
        .filter_map(|mask| {
            let weight = weights
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1 << index) != 0)
                .map(|(_, value)| value)
                .sum::<f64>();
            (weight <= capacity).then(|| {
                profits
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| mask & (1 << index) != 0)
                    .map(|(_, value)| value)
                    .sum::<f64>()
            })
        })
        .fold(0.0, f64::max)
}

fn fractional_knapsack(profits: &[f64], weights: &[f64], capacity: f64) -> f64 {
    let mut order = (0..profits.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| {
        (profits[*right] / weights[*right]).total_cmp(&(profits[*left] / weights[*left]))
    });
    let mut remaining = capacity;
    let mut objective = 0.0;
    for index in order {
        let value = remaining.min(weights[index]);
        objective += value * profits[index] / weights[index];
        remaining -= value;
        if remaining <= 0.0 {
            break;
        }
    }
    objective
}

#[test]
fn generated_small_milps_match_recording_and_brute_force_results() {
    let Some(env) = configured_environment() else {
        eprintln!("skipping COPT differential test: native environment is unavailable");
        return;
    };
    let mut random_state = 0x5eed_u64;

    for case in 0..10 {
        let profits = (0..6)
            .map(|_| next_value(&mut random_state, 10))
            .collect::<Vec<_>>();
        let weights = (0..6)
            .map(|_| next_value(&mut random_state, 8))
            .collect::<Vec<_>>();
        let capacity = (weights.iter().sum::<f64>() * 0.55).floor();
        let expected = brute_force_knapsack(&profits, &weights, capacity);

        let (recording, handle) = DummyModel::new();
        let mut recorded = BridgeOptimizer::new();
        recorded
            .add_variables(
                6,
                None,
                Some(vec!['B'; 6]),
                Some(BoundType::Single(0.0)),
                Some(BoundType::Single(1.0)),
            )
            .unwrap();
        recorded
            .add_constraint(
                affine(&weights),
                ScalarSetType::LessThan(capacity),
                Some(format!("capacity-{case}")),
            )
            .unwrap();
        recorded
            .set_objective(affine(&profits), ModelSense::Maximize)
            .unwrap();
        recorded.attach_backend(Box::new(recording)).unwrap();

        let snapshot = handle.snapshot().unwrap();
        assert_eq!(snapshot.variables.len(), 6);
        assert_eq!(snapshot.constraints.len(), 1);
        assert_eq!(snapshot.objective, Some(affine(&profits)));
        assert_eq!(snapshot.sense, Some(ModelSense::Maximize));

        let mut solved = BridgeOptimizer::new();
        solved
            .add_variables(
                6,
                None,
                Some(vec!['B'; 6]),
                Some(BoundType::Single(0.0)),
                Some(BoundType::Single(1.0)),
            )
            .unwrap();
        solved
            .add_constraint(affine(&weights), ScalarSetType::LessThan(capacity), None)
            .unwrap();
        solved
            .set_objective(affine(&profits), ModelSense::Maximize)
            .unwrap();
        solved
            .attach_backend(Box::new(configured_optimizer(&env).unwrap()))
            .unwrap();

        assert_eq!(solved.optimize().unwrap(), SolveStatus::Optimal);
        let actual = solved.get_objective_value().unwrap().unwrap();
        assert!(
            (actual - expected).abs() < 1e-7,
            "case {case}: expected {expected}, got {actual}"
        );
    }
}

#[test]
fn generated_small_lps_match_recording_and_fractional_optima() {
    let Some(env) = configured_environment() else {
        eprintln!("skipping COPT differential test: native environment is unavailable");
        return;
    };
    let mut random_state = 0x1a2b_3c4d_u64;

    for case in 0..10 {
        let profits = (0..6)
            .map(|_| next_value(&mut random_state, 10))
            .collect::<Vec<_>>();
        let weights = (0..6)
            .map(|_| next_value(&mut random_state, 8))
            .collect::<Vec<_>>();
        let capacity = weights.iter().sum::<f64>() * 0.55;
        let expected = fractional_knapsack(&profits, &weights, capacity);

        let (recording, handle) = DummyModel::new();
        let mut model = BridgeOptimizer::new();
        model
            .add_variables(
                6,
                None,
                Some(vec!['C'; 6]),
                Some(BoundType::Single(0.0)),
                Some(BoundType::Single(1.0)),
            )
            .unwrap();
        model
            .add_constraint(
                affine(&weights),
                ScalarSetType::LessThan(capacity),
                Some(format!("capacity-{case}")),
            )
            .unwrap();
        model
            .set_objective(affine(&profits), ModelSense::Maximize)
            .unwrap();
        model.attach_backend(Box::new(recording)).unwrap();
        let snapshot = handle.snapshot().unwrap();
        assert!(
            snapshot
                .variables
                .iter()
                .all(|variable| variable.vtype == 'C')
        );
        assert_eq!(snapshot.constraints.len(), 1);

        let mut solved = BridgeOptimizer::new();
        solved
            .add_variables(
                6,
                None,
                Some(vec!['C'; 6]),
                Some(BoundType::Single(0.0)),
                Some(BoundType::Single(1.0)),
            )
            .unwrap();
        solved
            .add_constraint(affine(&weights), ScalarSetType::LessThan(capacity), None)
            .unwrap();
        solved
            .set_objective(affine(&profits), ModelSense::Maximize)
            .unwrap();
        solved
            .attach_backend(Box::new(configured_optimizer(&env).unwrap()))
            .unwrap();

        assert_eq!(solved.optimize().unwrap(), SolveStatus::Optimal);
        let actual = solved.get_objective_value().unwrap().unwrap();
        assert!(
            (actual - expected).abs() < 1e-7,
            "case {case}: expected {expected}, got {actual}"
        );
    }
}
