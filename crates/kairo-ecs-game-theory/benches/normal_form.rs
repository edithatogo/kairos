use std::hint::black_box;
use std::time::Instant;

use kairo_ecs_game_theory::normal_form::{
    BestResponseSolver, PayoffMatrix, PureNashSolver, StrategySpace, StrictDominanceSolver, Utility,
};

fn main() {
    let quick = std::env::args().any(|arg| arg == "--quick");
    let iterations = if quick { 64 } else { 4_096 };
    let matrix = fixture_matrix();

    let start = Instant::now();
    let mut best_response_count = 0usize;
    let mut equilibrium_count = 0usize;
    let mut dominated_count = 0usize;
    for iteration in 0..iterations {
        let opponent_choice = iteration % 3;
        best_response_count +=
            BestResponseSolver::best_responses(&matrix, 0, &[opponent_choice, 1, 2])
                .expect("valid best-response query")
                .len();
        equilibrium_count += PureNashSolver::equilibria(&matrix).len();
        dominated_count += StrictDominanceSolver::strictly_dominated_strategies(&matrix, 0)
            .expect("valid dominated-strategy query")
            .len();
    }
    let elapsed = start.elapsed();

    black_box(best_response_count);
    black_box(equilibrium_count);
    black_box(dominated_count);

    println!(
        "{{\"benchmark\":\"normal_form_solver_quick\",\"iterations\":{iterations},\"elapsed_ns\":{},\"best_response_count\":{best_response_count},\"equilibrium_count\":{equilibrium_count},\"dominated_count\":{dominated_count}}}",
        elapsed.as_nanos()
    );
}

fn fixture_matrix() -> PayoffMatrix {
    let strategies = StrategySpace::from_counts(vec![3, 3, 3, 3]).expect("valid strategy counts");
    let mut payoffs = Vec::with_capacity(strategies.profile_count() * strategies.player_count());

    for a in 0..3 {
        for b in 0..3 {
            for c in 0..3 {
                for d in 0..3 {
                    let profile = [a, b, c, d];
                    for player in 0..4 {
                        let own = profile[player] as f64;
                        let neighbor = profile[(player + 1) % 4] as f64;
                        let coordination_bonus =
                            if profile.iter().all(|choice| *choice == profile[0]) {
                                2.0
                            } else {
                                0.0
                            };
                        payoffs.push(
                            Utility::new((own * 1.5) - neighbor + coordination_bonus)
                                .expect("finite utility"),
                        );
                    }
                }
            }
        }
    }

    PayoffMatrix::new(strategies, payoffs).expect("valid payoff matrix")
}
