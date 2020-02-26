use crate::helpers::*;
use crate::json::problem::*;
use crate::json::solution::*;

#[test]
fn can_use_sequence_and_flexible_relation_for_one_vehicle() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![1., 0.]),
                create_delivery_job("job2", vec![2., 0.]),
                create_delivery_job("job3", vec![3., 0.]),
                create_delivery_job("job4", vec![4., 0.]),
                create_delivery_job("job5", vec![5., 0.]),
                create_delivery_job("job6", vec![6., 0.]),
                create_delivery_job("job7", vec![7., 0.]),
            ],
            relations: Some(vec![
                Relation {
                    type_field: RelationType::Sequence,
                    jobs: to_strings(vec!["departure", "job4", "job2", "job6"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                    shift_index: None,
                },
                Relation {
                    type_field: RelationType::Flexible,
                    jobs: to_strings(vec!["job1", "job3"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                    shift_index: None,
                },
            ]),
        },
        fleet: Fleet { types: vec![create_default_vehicle_type()], profiles: create_default_profiles() },
        config: None,
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 61.,
                distance: 22,
                duration: 29,
                times: Timing { driving: 22, serving: 7, waiting: 0, break_time: 0 },
            },
            tours: vec![Tour {
                vehicle_id: "my_vehicle_1".to_string(),
                type_id: "my_vehicle".to_string(),
                shift_index: 0,
                stops: vec![
                    create_stop_with_activity(
                        "departure",
                        "departure",
                        (0., 0.),
                        7,
                        ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                        0
                    ),
                    create_stop_with_activity(
                        "job4",
                        "delivery",
                        (4., 0.),
                        6,
                        ("1970-01-01T00:00:04Z", "1970-01-01T00:00:05Z"),
                        4
                    ),
                    create_stop_with_activity(
                        "job2",
                        "delivery",
                        (2., 0.),
                        5,
                        ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                        6
                    ),
                    create_stop_with_activity(
                        "job6",
                        "delivery",
                        (6., 0.),
                        4,
                        ("1970-01-01T00:00:12Z", "1970-01-01T00:00:13Z"),
                        10
                    ),
                    create_stop_with_activity(
                        "job7",
                        "delivery",
                        (7., 0.),
                        3,
                        ("1970-01-01T00:00:14Z", "1970-01-01T00:00:15Z"),
                        11
                    ),
                    create_stop_with_activity(
                        "job5",
                        "delivery",
                        (5., 0.),
                        2,
                        ("1970-01-01T00:00:17Z", "1970-01-01T00:00:18Z"),
                        13
                    ),
                    create_stop_with_activity(
                        "job1",
                        "delivery",
                        (1., 0.),
                        1,
                        ("1970-01-01T00:00:22Z", "1970-01-01T00:00:23Z"),
                        17
                    ),
                    create_stop_with_activity(
                        "job3",
                        "delivery",
                        (3., 0.),
                        0,
                        ("1970-01-01T00:00:25Z", "1970-01-01T00:00:26Z"),
                        19
                    ),
                    create_stop_with_activity(
                        "arrival",
                        "arrival",
                        (0., 0.),
                        0,
                        ("1970-01-01T00:00:29Z", "1970-01-01T00:00:29Z"),
                        22
                    )
                ],
                statistic: Statistic {
                    cost: 61.,
                    distance: 22,
                    duration: 29,
                    times: Timing { driving: 22, serving: 7, waiting: 0, break_time: 0 },
                },
            }],
            unassigned: vec![],
            extras: None,
        }
    );
}

#[test]
fn can_use_sequence_and_flexible_relation_for_two_vehicles() {
    let problem = Problem {
        plan: Plan {
            jobs: vec![
                create_delivery_job("job1", vec![1., 0.]),
                create_delivery_job("job2", vec![2., 0.]),
                create_delivery_job("job3", vec![3., 0.]),
                create_delivery_job("job4", vec![4., 0.]),
                create_delivery_job("job5", vec![5., 0.]),
                create_delivery_job("job6", vec![6., 0.]),
                create_delivery_job("job7", vec![7., 0.]),
                create_delivery_job("job8", vec![8., 0.]),
            ],
            relations: Some(vec![
                Relation {
                    type_field: RelationType::Sequence,
                    jobs: to_strings(vec!["departure", "job1", "job6"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                    shift_index: None,
                },
                Relation {
                    type_field: RelationType::Flexible,
                    jobs: to_strings(vec!["job3", "job7"]),
                    vehicle_id: "my_vehicle_1".to_string(),
                    shift_index: None,
                },
                Relation {
                    type_field: RelationType::Sequence,
                    jobs: to_strings(vec!["departure", "job2", "job8"]),
                    vehicle_id: "my_vehicle_2".to_string(),
                    shift_index: None,
                },
                Relation {
                    type_field: RelationType::Flexible,
                    jobs: to_strings(vec!["job4", "job5"]),
                    vehicle_id: "my_vehicle_2".to_string(),
                    shift_index: None,
                },
            ]),
        },
        fleet: Fleet {
            types: vec![VehicleType {
                id: "my_vehicle".to_string(),
                profile: "car".to_string(),
                costs: create_default_vehicle_costs(),
                shifts: vec![create_default_open_vehicle_shift()],
                capacity: vec![5],
                amount: 2,
                skills: None,
                limits: None,
            }],
            profiles: create_default_profiles(),
        },
        config: None,
    };
    let matrix = create_matrix_from_problem(&problem);

    let solution = solve_with_metaheuristic(problem, vec![matrix]);

    assert_eq!(
        solution,
        Solution {
            statistic: Statistic {
                cost: 80.,
                distance: 26,
                duration: 34,
                times: Timing { driving: 26, serving: 8, waiting: 0, break_time: 0 },
            },
            tours: vec![
                Tour {
                    vehicle_id: "my_vehicle_1".to_string(),
                    type_id: "my_vehicle".to_string(),
                    shift_index: 0,
                    stops: vec![
                        create_stop_with_activity(
                            "departure",
                            "departure",
                            (0., 0.),
                            4,
                            ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                            0
                        ),
                        create_stop_with_activity(
                            "job1",
                            "delivery",
                            (1., 0.),
                            3,
                            ("1970-01-01T00:00:01Z", "1970-01-01T00:00:02Z"),
                            1
                        ),
                        create_stop_with_activity(
                            "job6",
                            "delivery",
                            (6., 0.),
                            2,
                            ("1970-01-01T00:00:07Z", "1970-01-01T00:00:08Z"),
                            6
                        ),
                        create_stop_with_activity(
                            "job3",
                            "delivery",
                            (3., 0.),
                            1,
                            ("1970-01-01T00:00:11Z", "1970-01-01T00:00:12Z"),
                            9
                        ),
                        create_stop_with_activity(
                            "job7",
                            "delivery",
                            (7., 0.),
                            0,
                            ("1970-01-01T00:00:16Z", "1970-01-01T00:00:17Z"),
                            13
                        )
                    ],
                    statistic: Statistic {
                        cost: 40.,
                        distance: 13,
                        duration: 17,
                        times: Timing { driving: 13, serving: 4, waiting: 0, break_time: 0 },
                    },
                },
                Tour {
                    vehicle_id: "my_vehicle_2".to_string(),
                    type_id: "my_vehicle".to_string(),
                    shift_index: 0,
                    stops: vec![
                        create_stop_with_activity(
                            "departure",
                            "departure",
                            (0., 0.),
                            4,
                            ("1970-01-01T00:00:00Z", "1970-01-01T00:00:00Z"),
                            0
                        ),
                        create_stop_with_activity(
                            "job2",
                            "delivery",
                            (2., 0.),
                            3,
                            ("1970-01-01T00:00:02Z", "1970-01-01T00:00:03Z"),
                            2
                        ),
                        create_stop_with_activity(
                            "job8",
                            "delivery",
                            (8., 0.),
                            2,
                            ("1970-01-01T00:00:09Z", "1970-01-01T00:00:10Z"),
                            8
                        ),
                        create_stop_with_activity(
                            "job4",
                            "delivery",
                            (4., 0.),
                            1,
                            ("1970-01-01T00:00:14Z", "1970-01-01T00:00:15Z"),
                            12
                        ),
                        create_stop_with_activity(
                            "job5",
                            "delivery",
                            (5., 0.),
                            0,
                            ("1970-01-01T00:00:16Z", "1970-01-01T00:00:17Z"),
                            13
                        )
                    ],
                    statistic: Statistic {
                        cost: 40.,
                        distance: 13,
                        duration: 17,
                        times: Timing { driving: 13, serving: 4, waiting: 0, break_time: 0 },
                    },
                }
            ],
            unassigned: vec![],
            extras: None,
        }
    );
}
