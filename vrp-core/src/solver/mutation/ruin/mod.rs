//! The ruin module contains various strategies to destroy small, medium or large parts of an
//! existing solution.

use crate::construction::heuristics::{InsertionContext, RouteContext, SolutionContext};
use crate::models::problem::Job;
use crate::models::Problem;
use crate::solver::RefinementContext;
use crate::utils::Random;
use hashbrown::HashMap;
use std::iter::{empty, once};
use std::sync::Arc;

/// A trait which specifies logic to destroy parts of solution.
pub trait Ruin {
    /// Ruins given solution and returns a new one with less jobs assigned.
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext;
}

mod adjusted_string_removal;
pub use self::adjusted_string_removal::AdjustedStringRemoval;

mod cluster_removal;
pub use self::cluster_removal::ClusterRemoval;

mod neighbour_removal;
pub use self::neighbour_removal::NeighbourRemoval;

mod random_route_removal;
pub use self::random_route_removal::RandomRouteRemoval;

mod random_job_removal;
pub use self::random_job_removal::RandomJobRemoval;

mod worst_jobs_removal;
pub use self::worst_jobs_removal::WorstJobRemoval;

/// A type which specifies a group of multiple ruin strategies with its probability.
pub type RuinGroup = (Vec<(Arc<dyn Ruin + Send + Sync>, f64)>, usize);

/// Provides the way to run multiple ruin methods one by one on the same solution.
pub struct CompositeRuin {
    ruins: Vec<Vec<(Arc<dyn Ruin + Send + Sync>, f64)>>,
    weights: Vec<usize>,
}

/// Specifies a limit for amount of jobs to be removed.
pub struct JobRemovalLimit {
    /// Specifies minimum amount of removed jobs.
    pub min: usize,
    /// Specifies maximum amount of removed jobs.
    pub max: usize,
    /// Specifies threshold ratio of maximum removed jobs.
    pub threshold: f64,
}

impl JobRemovalLimit {
    /// Creates a new instance of `JobRemovalLimit`.
    pub fn new(min: usize, max: usize, threshold: f64) -> Self {
        Self { min, max, threshold }
    }
}

impl Default for JobRemovalLimit {
    fn default() -> Self {
        Self { min: 8, max: 16, threshold: 0.1 }
    }
}

impl CompositeRuin {
    /// Creates a new instance of `CompositeRuin` with passed ruin methods.
    pub fn new(ruins: Vec<RuinGroup>) -> Self {
        let weights = ruins.iter().map(|(_, weight)| *weight).collect();
        let ruins = ruins.into_iter().map(|(ruin, _)| ruin).collect();

        Self { ruins, weights }
    }

    /// Creates a new instance of `CompositeRuin` with default ruin methods.
    pub fn new_from_problem(problem: Arc<Problem>) -> Self {
        let random_route = Arc::new(RandomRouteRemoval::default());
        let random_job = Arc::new(RandomJobRemoval::new(JobRemovalLimit::default()));

        Self::new(vec![
            (
                vec![
                    (Arc::new(AdjustedStringRemoval::default()), 1.),
                    (random_job.clone(), 0.05),
                    (random_route.clone(), 0.01),
                ],
                100,
            ),
            (vec![(Arc::new(AdjustedStringRemoval::new(20, 20, 0.1)), 1.), (random_job.clone(), 0.05)], 10),
            (
                vec![
                    (Arc::new(WorstJobRemoval::default()), 1.),
                    (random_job.clone(), 0.05),
                    (random_route.clone(), 0.01),
                ],
                10,
            ),
            (
                vec![
                    (Arc::new(NeighbourRemoval::default()), 1.),
                    (random_job.clone(), 0.05),
                    (random_route.clone(), 0.01),
                ],
                10,
            ),
            (vec![(random_job.clone(), 1.), (random_route.clone(), 0.1)], 5),
            (vec![(random_route.clone(), 1.), (random_job.clone(), 0.1)], 5),
            (
                vec![
                    (Arc::new(ClusterRemoval::new_with_defaults(problem)), 1.),
                    (random_job, 0.05),
                    (random_route, 0.01),
                ],
                1,
            ),
        ])
    }
}

impl Ruin for CompositeRuin {
    fn run(&self, refinement_ctx: &RefinementContext, insertion_ctx: InsertionContext) -> InsertionContext {
        if insertion_ctx.solution.routes.is_empty() {
            return insertion_ctx;
        }

        let random = insertion_ctx.random.clone();

        let index = insertion_ctx.random.weighted(self.weights.as_slice());

        let mut insertion_ctx = self
            .ruins
            .get(index)
            .unwrap()
            .iter()
            .filter(|(_, probability)| *probability > random.uniform_real(0., 1.))
            .fold(insertion_ctx, |ctx, (ruin, _)| ruin.run(refinement_ctx, ctx));

        insertion_ctx.restore();

        insertion_ctx
    }
}

fn get_removal_chunk_size(ctx: &InsertionContext, limit: &JobRemovalLimit) -> usize {
    let assigned = ctx.problem.jobs.size() - ctx.solution.unassigned.len() - ctx.solution.ignored.len();

    let max_limit = (assigned as f64 * limit.threshold).min(limit.max as f64).round() as usize;

    ctx.random.uniform_int(limit.min as i32, limit.max as i32).min(max_limit as i32) as usize
}

fn get_route_jobs(solution: &SolutionContext) -> HashMap<Job, RouteContext> {
    solution
        .routes
        .iter()
        .flat_map(|rc| rc.route.tour.jobs().collect::<Vec<_>>().into_iter().map(move |job| (job, rc.clone())))
        .collect()
}

/// Returns randomly selected job within all its neighbours.
fn select_seed_jobs<'a>(
    problem: &'a Problem,
    routes: &[RouteContext],
    random: &Arc<dyn Random + Send + Sync>,
) -> Box<dyn Iterator<Item = Job> + 'a> {
    let seed = select_seed_job(routes, random);

    if let Some((route_index, job)) = seed {
        return Box::new(
            once(job.clone()).chain(
                problem
                    .jobs
                    .neighbors(routes.get(route_index).unwrap().route.actor.vehicle.profile, &job, Default::default())
                    .map(|(job, _)| job)
                    .cloned(),
            ),
        );
    }

    Box::new(empty())
}

/// Selects seed job from existing solution
fn select_seed_job<'a>(routes: &'a [RouteContext], random: &Arc<dyn Random + Send + Sync>) -> Option<(usize, Job)> {
    if routes.is_empty() {
        return None;
    }

    let route_index = random.uniform_int(0, (routes.len() - 1) as i32) as usize;
    let mut ri = route_index;

    loop {
        let rc = routes.get(ri).unwrap();

        if rc.route.tour.has_jobs() {
            let job = select_random_job(rc, random);
            if let Some(job) = job {
                return Some((ri, job));
            }
        }

        ri = (ri + 1) % routes.len();
        if ri == route_index {
            break;
        }
    }

    None
}

fn select_random_job(rc: &RouteContext, random: &Arc<dyn Random + Send + Sync>) -> Option<Job> {
    let size = rc.route.tour.activity_count();
    if size == 0 {
        return None;
    }

    let activity_index = random.uniform_int(1, size as i32) as usize;
    let mut ai = activity_index;

    loop {
        let job = rc.route.tour.get(ai).and_then(|a| a.retrieve_job());

        if job.is_some() {
            return job;
        }

        ai = (ai + 1) % (size + 1);
        if ai == activity_index {
            break;
        }
    }

    None
}
