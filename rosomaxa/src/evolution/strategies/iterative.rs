use super::*;
use crate::utils::Timer;
use std::marker::PhantomData;

/// A simple evolution algorithm which maintains a single population and improves it iteratively.
pub struct Iterative<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    desired_solutions_amount: usize,
    _marker: (PhantomData<C>, PhantomData<O>, PhantomData<S>),
}

impl<C, O, S> Iterative<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    /// Creates a new instance of `RunSimple`.
    pub fn new(desired_solutions_amount: usize) -> Self {
        Self { desired_solutions_amount, _marker: (Default::default(), Default::default(), Default::default()) }
    }
}

impl<C, O, S> EvolutionStrategy for Iterative<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    type Context = C;
    type Objective = O;
    type Solution = S;

    fn run(
        &self,
        heuristic_ctx: Self::Context,
        heuristic: Box<
            dyn HyperHeuristic<Context = Self::Context, Objective = Self::Objective, Solution = Self::Solution>,
        >,
        termination: Box<dyn Termination<Context = Self::Context, Objective = Self::Objective>>,
    ) -> EvolutionResult<Self::Solution> {
        let mut heuristic_ctx = heuristic_ctx;
        let mut heuristic = heuristic;

        loop {
            let is_terminated = termination.is_termination(&mut heuristic_ctx);
            let is_quota_reached = heuristic_ctx.environment().quota.as_ref().map_or(false, |q| q.is_reached());

            if is_terminated || is_quota_reached {
                break;
            }

            let generation_time = Timer::start();

            let parents = heuristic_ctx.population().select().collect::<Vec<_>>();

            let diverse_offspring = if heuristic_ctx.population().selection_phase() == SelectionPhase::Exploitation {
                Vec::default()
            } else {
                heuristic.diversify(&heuristic_ctx, parents.clone())
            };

            let search_offspring = heuristic.search(&heuristic_ctx, parents);

            let offspring = search_offspring.into_iter().chain(diverse_offspring).collect::<Vec<_>>();

            let termination_estimate = termination.estimate(&heuristic_ctx);

            heuristic_ctx.on_generation(offspring, termination_estimate, generation_time);
        }

        // NOTE give a chance to report internal state of heuristic
        (heuristic_ctx.environment().logger)(&format!("{heuristic}"));

        let (population, telemetry_metrics) = heuristic_ctx.on_result()?;

        let solutions =
            population.ranked().map(|(solution, _)| solution.deep_copy()).take(self.desired_solutions_amount).collect();

        Ok((solutions, telemetry_metrics))
    }
}

impl<C, O, S> Default for Iterative<C, O, S>
where
    C: HeuristicContext<Objective = O, Solution = S>,
    O: HeuristicObjective<Solution = S>,
    S: HeuristicSolution,
{
    fn default() -> Self {
        Self::new(1)
    }
}
