use crate::construction::heuristics::InsertionContext;
use crate::construction::Quota;
use crate::models::{Problem, Solution};
use crate::solver::evolution::{EvolutionConfig, InitialConfig, OffspringConfig, PopulationConfig};
use crate::solver::mutation::*;
use crate::solver::telemetry::{Telemetry, TelemetryMode};
use crate::solver::termination::*;
use crate::solver::Solver;
use crate::utils::{get_cpus, DefaultRandom, TimeQuota};
use std::ops::Range;
use std::sync::Arc;

/// Provides configurable way to build Vehile Routing Problem [`Solver`] instance using fluent
/// interface style.
///
/// A newly created builder instance is pre-configured with some reasonable defaults for mid-size
/// problems (~200), so there is no need to call any of its methods.
///
/// [`Solver`]: ./struct.Solver.html
///
/// # Examples
///
/// This example shows how to override some of default metaheuristic parameters using fluent
/// interface methods:
///
/// ```
/// # use vrp_core::models::examples::create_example_problem;
/// # use std::sync::Arc;
/// use vrp_core::solver::Builder;
/// use vrp_core::models::Problem;
///
/// // create your VRP problem
/// let problem: Arc<Problem> = create_example_problem();
/// // build solver using builder with overridden parameters
/// let solver = Builder::new(problem)
///     .with_max_time(Some(60))
///     .with_max_generations(Some(100))
///     .build()?;
/// // run solver and get the best known solution within its cost.
/// let (solution, cost, _) = solver.solve()?;
///
/// assert_eq!(cost, 42.);
/// assert_eq!(solution.routes.len(), 1);
/// assert_eq!(solution.unassigned.len(), 0);
/// # Ok::<(), String>(())
/// ```
pub struct Builder {
    /// A max amount generations in evolution.
    pub max_generations: Option<usize>,

    /// A max seconds to run evolution.
    pub max_time: Option<usize>,

    /// A cost variation parameters for termination criteria.
    pub cost_variation: Option<(usize, f64)>,

    /// A randomization seed
    pub seed: Option<u64>,

    /// An evolution configuration..
    pub config: EvolutionConfig,
}

impl Builder {
    /// Creates a new instance of `Builder`.
    pub fn new(problem: Arc<Problem>) -> Self {
        Self {
            max_generations: None,
            max_time: None,
            cost_variation: None,
            seed: None,
            config: EvolutionConfig {
                problem: problem.clone(),
                mutation: Box::new(RuinAndRecreateMutation::new_from_problem(problem)),
                termination: Box::new(CompositeTermination::new(vec![
                    Box::new(MaxTime::new(300.)),
                    Box::new(MaxGeneration::new(3000)),
                ])),
                quota: None,
                random: Arc::new(DefaultRandom::default()),
                telemetry: Telemetry::new(TelemetryMode::None),
                population: PopulationConfig {
                    size: 4,
                    initial: InitialConfig {
                        size: 1,
                        methods: vec![(Box::new(RecreateWithCheapest::default()), 10)],
                        individuals: vec![],
                    },
                    offspring: OffspringConfig { size: get_cpus(), chance: 0.001, generations: 4..6, steepness: 1.5 },
                },
            },
        }
    }
}

impl Builder {
    /// Sets telemetry. Default telemetry is set to do nothing.
    pub fn with_telemetry(mut self, telemetry: Telemetry) -> Self {
        self.config.telemetry = telemetry;
        self
    }

    /// Sets max generations to be run by evolution. Default is 3000.
    pub fn with_max_generations(mut self, limit: Option<usize>) -> Self {
        self.max_generations = limit;
        self
    }

    /// Sets cost variation termination criteria. Default is None.
    pub fn with_cost_variation(mut self, variation: Option<(usize, f64)>) -> Self {
        self.cost_variation = variation;
        self
    }

    /// Sets max running time limit for evolution. Default is 300 seconds.
    pub fn with_max_time(mut self, limit: Option<usize>) -> Self {
        self.max_time = limit;
        self
    }

    /// Sets initial parameters used to construct initial population.
    pub fn with_init_params(
        mut self,
        size: Option<usize>,
        initial_methods: Option<Vec<(Box<dyn Recreate + Send + Sync>, usize)>>,
    ) -> Self {
        self.config.telemetry.log("configured to use custom initial population parameters");

        if let Some(size) = size {
            self.config.population.initial.size = size;
        }

        if let Some(initial_methods) = initial_methods {
            self.config.population.initial.methods = initial_methods;
        }

        self
    }

    /// Sets initial solutions in population. Default is no solutions in population.
    pub fn with_init_solutions(mut self, solutions: Vec<Solution>) -> Self {
        self.config.telemetry.log(format!("provided {} initial solutions to start with", solutions.len()).as_str());
        self.config.population.initial.individuals = solutions
            .into_iter()
            .map(|solution| {
                InsertionContext::new_from_solution(
                    self.config.problem.clone(),
                    (solution, None),
                    Arc::new(DefaultRandom::default()),
                )
            })
            .collect();
        self
    }

    /// Sets max population size. Default is 4.
    pub fn with_population_size(mut self, size: usize) -> Self {
        self.config.telemetry.log(format!("configured to use max population size: {} ", size).as_str());
        self.config.population.size = size;
        self
    }

    /// Sets offspring configuration.
    pub fn with_offspring(
        mut self,
        size: Option<usize>,
        chance: Option<f64>,
        generations: Option<Range<usize>>,
        steepness: Option<f64>,
    ) -> Self {
        self.config.telemetry.log("configured to use custom offspring parameters");
        let old = self.config.population.offspring;
        self.config.population.offspring = OffspringConfig {
            size: size.unwrap_or(old.size),
            chance: chance.unwrap_or(old.chance),
            generations: generations.unwrap_or(old.generations),
            steepness: steepness.unwrap_or(old.steepness),
        };
        self
    }

    /// Sets mutation algorithm. Default is ruin and recreate.
    pub fn with_mutation(mut self, mutation: Box<dyn Mutation + Send + Sync>) -> Self {
        self.config.telemetry.log("configured to use custom mutation parameters");
        self.config.mutation = mutation;
        self
    }

    /// Sets termination algorithm. Default is max time and max generations.
    pub fn with_termination(mut self, termination: Box<dyn Termination>) -> Self {
        self.config.telemetry.log("configured to use custom termination parameters");
        self.config.termination = termination;
        self
    }

    /// Sets randomization seed.
    pub fn with_seed(mut self, seed: Option<u64>) -> Self {
        self.config.telemetry.log("configured to use custom seed parameters");
        self.seed = seed;
        self
    }

    /// Builds [`Solver`](./struct.Solver.html) instance.
    pub fn build(self) -> Result<Solver, String> {
        let problem = self.config.problem.clone();

        let (criterias, quota): (Vec<Box<dyn Termination>>, _) =
            match (self.max_generations, self.max_time, self.cost_variation) {
                (None, None, None) => {
                    self.config
                        .telemetry
                        .log("configured to use default max-generations (3000) and max-time (300secs)");
                    (vec![Box::new(MaxGeneration::new(3000)), Box::new(MaxTime::new(300.))], None)
                }
                _ => {
                    let mut criterias: Vec<Box<dyn Termination>> = vec![];

                    if let Some(limit) = self.max_generations {
                        self.config.telemetry.log(format!("configured to use max-generations: {}", limit).as_str());
                        criterias.push(Box::new(MaxGeneration::new(limit)))
                    }

                    let quota = if let Some(limit) = self.max_time {
                        self.config.telemetry.log(format!("configured to use max-time: {}s", limit).as_str());
                        criterias.push(Box::new(MaxTime::new(limit as f64)));
                        create_time_quota(limit)
                    } else {
                        None
                    };

                    if let Some((sample, threshold)) = self.cost_variation {
                        self.config.telemetry.log(
                            format!(
                                "configured to use cost variation with sample: {}, threshold: {}",
                                sample, threshold
                            )
                            .as_str(),
                        );
                        criterias.push(Box::new(CostVariation::new(sample, threshold)))
                    }

                    (criterias, quota)
                }
            };

        let mut config = self.config;
        config.termination = Box::new(CompositeTermination::new(criterias));
        config.quota = quota;

        config.random = Arc::new(if let Some(seed) = self.seed {
            config.telemetry.log(format!("configured to use seed: {}", seed).as_str());
            DefaultRandom::new_with_seed(seed)
        } else {
            DefaultRandom::default()
        });

        Ok(Solver { problem, config })
    }
}

fn create_time_quota(limit: usize) -> Option<Box<dyn Quota + Sync + Send>> {
    Some(Box::new(TimeQuota::new(limit as f64)))
}
