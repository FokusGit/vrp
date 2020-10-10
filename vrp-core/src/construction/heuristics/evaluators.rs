#[cfg(test)]
#[path = "../../../tests/unit/construction/heuristics/evaluators_test.rs"]
mod evaluators_test;

use std::sync::Arc;

use crate::construction::constraints::{ActivityConstraintViolation, ConstraintPipeline};
use crate::construction::heuristics::*;
use crate::models::common::Cost;
use crate::models::problem::{Job, Multi, Single};
use crate::models::solution::{Activity, Place};
use std::iter::repeat;

/// Specifies allowed insertion position in route for the job.
#[derive(Copy, Clone)]
pub enum InsertionPosition {
    /// Job can be inserted anywhere in the route.
    Any,
    /// Job can be inserted only at the leg with the concrete index.
    Concrete(usize),
    /// Job can be inserted only to the end of the route.
    Last,
}

/// Evaluates possibility to preform insertion from given insertion context in all available
/// routes at given position constraint.
pub fn evaluate_job_insertion(
    job: &Job,
    ctx: &InsertionContext,
    route_selector: &(dyn RouteSelector + Send + Sync),
    position: InsertionPosition,
) -> InsertionResult {
    route_selector.select(ctx, job).fold(InsertionResult::make_failure(), |acc, route_ctx| {
        evaluate_job_insertion_in_route(job, ctx, &route_ctx, position, acc)
    })
}

/// Evaluates possibility to preform insertion from given insertion context in given route
/// at given position constraint.
pub fn evaluate_job_insertion_in_route(
    job: &Job,
    ctx: &InsertionContext,
    route_ctx: &RouteContext,
    position: InsertionPosition,
    alternative: InsertionResult,
) -> InsertionResult {
    let constraint = &ctx.problem.constraint;

    if let Some(violation) = constraint.evaluate_hard_route(&ctx.solution, &route_ctx, job) {
        return InsertionResult::choose_best_result(
            alternative,
            InsertionResult::make_failure_with_code(violation.code, Some(job.clone())),
        );
    }

    let route_costs = constraint.evaluate_soft_route(&ctx.solution, &route_ctx, &job);
    let best_known_cost = match &alternative {
        InsertionResult::Success(success) => Some(success.cost),
        _ => None,
    };

    if let Some(best_known_cost) = best_known_cost {
        if best_known_cost < route_costs {
            return alternative;
        }
    }

    InsertionResult::choose_best_result(
        alternative,
        match job {
            Job::Single(single) => {
                evaluate_single(job, single, constraint, &route_ctx, position, route_costs, best_known_cost)
            }
            Job::Multi(multi) => {
                evaluate_multi(job, multi, constraint, &route_ctx, position, route_costs, best_known_cost)
            }
        },
    )
}

/// Evaluates possibility to preform insertion in route context only.
pub fn evaluate_job_constraint_in_route(
    job: &Job,
    constraint: &ConstraintPipeline,
    route_ctx: &RouteContext,
    position: InsertionPosition,
) -> InsertionResult {
    match job {
        Job::Single(single) => evaluate_single(job, single, constraint, &route_ctx, position, 0., None),
        Job::Multi(multi) => evaluate_multi(job, multi, constraint, &route_ctx, position, 0., None),
    }
}

fn evaluate_single(
    job: &Job,
    single: &Arc<Single>,
    constraint: &ConstraintPipeline,
    route_ctx: &RouteContext,
    position: InsertionPosition,
    route_costs: Cost,
    best_known_cost: Option<Cost>,
) -> InsertionResult {
    let insertion_idx = get_insertion_index(route_ctx, position);
    let mut activity = Activity::new_with_job(single.clone());

    let result = analyze_insertion_in_route(
        constraint,
        route_ctx,
        insertion_idx,
        single,
        &mut activity,
        SingleContext::new(best_known_cost, 0),
    );

    if result.is_success() {
        activity.place = result.place.unwrap();
        let activities = vec![(activity, result.index)];
        InsertionResult::make_success(result.cost.unwrap() + route_costs, job.clone(), activities, route_ctx.clone())
    } else {
        InsertionResult::make_failure_with_code(result.violation.map_or(0, |v| v.code), Some(job.clone()))
    }
}

fn evaluate_multi(
    job: &Job,
    multi: &Arc<Multi>,
    constraint: &ConstraintPipeline,
    route_ctx: &RouteContext,
    position: InsertionPosition,
    route_costs: Cost,
    best_known_cost: Option<Cost>,
) -> InsertionResult {
    let insertion_idx = get_insertion_index(route_ctx, position).unwrap_or(0);
    // 1. analyze permutations
    let result = unwrap_from_result(multi.permutations().into_iter().try_fold(
        MultiContext::new(best_known_cost, insertion_idx),
        |acc_res, services| {
            let mut shadow = ShadowContext::new(constraint, &route_ctx);
            let perm_res = unwrap_from_result(repeat(0).try_fold(MultiContext::new(None, insertion_idx), |out, _| {
                if out.is_failure(route_ctx.route.tour.activity_count()) {
                    return Result::Err(out);
                }
                shadow.restore(job);

                // 2. analyze inner jobs
                let sq_res = unwrap_from_result(services.iter().try_fold(out.next(), |in1, service| {
                    if in1.violation.is_some() {
                        return Result::Err(in1);
                    }
                    let mut activity = Activity::new_with_job(service.clone());
                    // 3. analyze legs
                    let srv_res = analyze_insertion_in_route(
                        &constraint,
                        &shadow.ctx,
                        None,
                        service,
                        &mut activity,
                        SingleContext::new(None, in1.next_index),
                    );

                    if srv_res.is_success() {
                        activity.place = srv_res.place.unwrap();
                        let activity = shadow.insert(activity, srv_res.index);
                        let activities = concat_activities(in1.activities, (activity, srv_res.index));
                        return MultiContext::success(in1.cost.unwrap_or(0.) + srv_res.cost.unwrap(), activities);
                    }

                    MultiContext::fail(srv_res, in1)
                }));

                MultiContext::promote(sq_res, out)
            }));

            MultiContext::promote(perm_res, acc_res)
        },
    ));

    if result.is_success() {
        let activities = result.activities.unwrap();
        InsertionResult::make_success(result.cost.unwrap() + route_costs, job.clone(), activities, route_ctx.clone())
    } else {
        InsertionResult::make_failure_with_code(result.violation.map_or(0, |v| v.code), Some(job.clone()))
    }
}

#[inline(always)]
fn analyze_insertion_in_route(
    constraint: &ConstraintPipeline,
    route_ctx: &RouteContext,
    insertion_idx: Option<usize>,
    single: &Single,
    target: &mut Activity,
    init: SingleContext,
) -> SingleContext {
    unwrap_from_result(match insertion_idx {
        Some(idx) => {
            if let Some(concrete_leg) = route_ctx.route.tour.legs().nth(idx) {
                analyze_insertion_in_route_leg(constraint, route_ctx, concrete_leg, single, target, init)
            } else {
                Ok(init)
            }
        }
        None => {
            route_ctx.route.tour.legs().skip(init.index).try_fold(init, |out, leg| {
                analyze_insertion_in_route_leg(constraint, route_ctx, leg, single, target, out)
            })
        }
    })
}

#[inline(always)]
fn analyze_insertion_in_route_leg<'a>(
    constraint: &ConstraintPipeline,
    route_ctx: &RouteContext,
    leg: (&'a [Activity], usize),
    single: &Single,
    target: &mut Activity,
    out: SingleContext,
) -> Result<SingleContext, SingleContext> {
    let (items, index) = leg;
    let (prev, next) = match items {
        [prev] => (prev, None),
        [prev, next] => (prev, Some(next)),
        _ => panic!("Unexpected route leg configuration."),
    };
    let start_time = route_ctx.route.tour.start().unwrap().schedule.departure;
    // analyze service details
    single.places.iter().try_fold(out, |in1, detail| {
        // analyze detail time windows
        detail.times.iter().try_fold(in1, |in2, time| {
            target.place = Place {
                location: detail.location.unwrap_or(prev.place.location),
                duration: detail.duration,
                time: time.to_time_window(start_time),
            };

            let activity_ctx = ActivityContext { index, prev, target: &target, next };

            if let Some(violation) = constraint.evaluate_hard_activity(route_ctx, &activity_ctx) {
                return SingleContext::fail(violation, in2);
            }

            let costs = constraint.evaluate_soft_activity(route_ctx, &activity_ctx);

            if costs < in2.cost.unwrap_or(std::f64::MAX) {
                SingleContext::success(activity_ctx.index, costs, target.place.clone())
            } else {
                SingleContext::skip(in2)
            }
        })
    })
}

fn get_insertion_index(route_ctx: &RouteContext, position: InsertionPosition) -> Option<usize> {
    match position {
        InsertionPosition::Any => None,
        InsertionPosition::Concrete(idx) => Some(idx),
        InsertionPosition::Last => Some(route_ctx.route.tour.legs().count().max(1) - 1),
    }
}

/// Stores information needed for single insertion.
#[derive(Debug)]
struct SingleContext {
    /// Constraint violation.
    pub violation: Option<ActivityConstraintViolation>,
    /// Insertion index.
    pub index: usize,
    /// Best cost.
    pub cost: Option<Cost>,
    /// Activity place.
    pub place: Option<Place>,
}

impl SingleContext {
    /// Creates a new empty context with given cost.
    fn new(cost: Option<Cost>, index: usize) -> Self {
        Self { violation: None, index, cost, place: None }
    }

    fn fail(violation: ActivityConstraintViolation, other: SingleContext) -> Result<Self, Self> {
        let stopped = violation.stopped;
        let ctx = Self { violation: Some(violation), index: other.index, cost: other.cost, place: other.place };
        if stopped {
            Result::Err(ctx)
        } else {
            Result::Ok(ctx)
        }
    }

    fn success(index: usize, cost: Cost, place: Place) -> Result<Self, Self> {
        Result::Ok(Self { violation: None, index, cost: Some(cost), place: Some(place) })
    }

    fn skip(other: SingleContext) -> Result<Self, Self> {
        Result::Ok(other)
    }

    fn is_success(&self) -> bool {
        self.place.is_some()
    }
}

/// Stores information needed for multi job insertion.
struct MultiContext {
    /// Constraint violation.
    pub violation: Option<ActivityConstraintViolation>,
    /// Insertion index for first service.
    pub start_index: usize,
    /// Insertion index for next service.
    pub next_index: usize,
    /// Cost accumulator.
    pub cost: Option<Cost>,
    /// Activities with their indices.
    pub activities: Option<Vec<(Activity, usize)>>,
}

impl MultiContext {
    /// Creates new empty insertion context.
    fn new(cost: Option<Cost>, index: usize) -> Self {
        Self { violation: None, start_index: index, next_index: index, cost, activities: None }
    }

    /// Promotes insertion context by best price.
    fn promote(left: Self, right: Self) -> Result<Self, Self> {
        let index = left.start_index.max(right.start_index) + 1;
        let best = match (left.cost, right.cost) {
            (Some(left_cost), Some(right_cost)) => {
                if left_cost < right_cost {
                    left
                } else {
                    right
                }
            }
            (Some(_), None) => left,
            (None, Some(_)) => right,
            _ => {
                if left.violation.is_some() {
                    left
                } else {
                    right
                }
            }
        };

        let result = Self {
            violation: best.violation,
            start_index: index,
            next_index: index,
            cost: best.cost,
            activities: best.activities,
        };

        if result.violation.as_ref().map_or_else(|| false, |v| v.stopped) {
            Result::Err(result)
        } else {
            Result::Ok(result)
        }
    }

    /// Creates failed insertion context within reason code.
    fn fail(err_ctx: SingleContext, other_ctx: MultiContext) -> Result<Self, Self> {
        let (code, stopped) =
            err_ctx.violation.map_or((0, false), |v| (v.code, v.stopped && other_ctx.activities.is_none()));

        Result::Err(Self {
            violation: Some(ActivityConstraintViolation { code, stopped }),
            start_index: other_ctx.start_index,
            next_index: other_ctx.start_index,
            cost: None,
            activities: None,
        })
    }

    /// Creates successful insertion context.
    fn success(cost: Cost, activities: Vec<(Activity, usize)>) -> Result<Self, Self> {
        Result::Ok(Self {
            violation: None,
            start_index: activities.first().unwrap().1,
            next_index: activities.last().unwrap().1 + 1,
            cost: Some(cost),
            activities: Some(activities),
        })
    }

    /// Creates next insertion context from existing one.
    fn next(&self) -> Self {
        Self {
            violation: None,
            start_index: self.start_index,
            next_index: self.start_index,
            cost: None,
            activities: None,
        }
    }

    /// Checks whether insertion is found.
    fn is_success(&self) -> bool {
        self.violation.is_none() && self.cost.is_some() && self.activities.is_some()
    }

    /// Checks whether insertion is failed.
    fn is_failure(&self, index: usize) -> bool {
        self.violation.as_ref().map_or(false, |v| v.stopped) || (self.start_index > index)
    }
}

/// Provides the way to use copy on write strategy within route state context.
struct ShadowContext<'a> {
    is_mutated: bool,
    is_dirty: bool,
    constraint: &'a ConstraintPipeline,
    ctx: RouteContext,
}

impl<'a> ShadowContext<'a> {
    fn new(constraint: &'a ConstraintPipeline, ctx: &RouteContext) -> Self {
        Self {
            is_mutated: false,
            is_dirty: false,
            constraint,
            ctx: RouteContext { route: ctx.route.clone(), state: ctx.state.clone() },
        }
    }

    fn insert(&mut self, activity: Activity, index: usize) -> Activity {
        if !self.is_mutated {
            self.ctx = self.ctx.deep_copy();
            self.is_mutated = true;
        }

        self.ctx.route_mut().tour.insert_at(activity, index + 1);
        self.constraint.accept_route_state(&mut self.ctx);
        self.is_dirty = true;

        self.ctx.route.tour.get(index + 1).unwrap().deep_copy()
    }

    fn restore(&mut self, job: &Job) {
        if self.is_dirty {
            let (route, state) = self.ctx.as_mut();

            route.tour.all_activities().for_each(|a| state.remove_activity_states(a));
            route.tour.remove(job);

            self.constraint.accept_route_state(&mut self.ctx);
        }
    }
}

fn unwrap_from_result<T>(result: Result<T, T>) -> T {
    match result {
        Ok(result) => result,
        Err(result) => result,
    }
}

fn concat_activities(
    activities: Option<Vec<(Activity, usize)>>,
    activity: (Activity, usize),
) -> Vec<(Activity, usize)> {
    let mut activities = activities.unwrap_or_else(Vec::new);
    activities.push((activity.0, activity.1));

    activities
}
