use crate::algorithms::gsom::{Input, Network, Storage};
use std::fmt::{Display, Formatter};

#[derive(Clone)]
pub struct Data {
    pub values: Vec<f64>,
}

impl Input for Data {
    fn weights(&self) -> &[f64] {
        self.values.as_slice()
    }
}

impl Data {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { values: vec![x, y, z] }
    }
}

pub struct DataStorage {
    pub data: Vec<Data>,
}

impl Storage for DataStorage {
    type Item = Data;

    fn add(&mut self, input: Self::Item) {
        self.data.clear();
        self.data.push(input);
    }

    fn drain(&mut self) -> Vec<Self::Item> {
        self.data.drain(0..).collect()
    }

    fn distance(&self, a: &[f64], b: &[f64]) -> f64 {
        f64::sqrt((a[0] - b[0]).powf(2.0) + (a[1] - b[1]).powf(2.0) + (a[2] - b[2]).powf(2.0))
    }
}

impl Default for DataStorage {
    fn default() -> Self {
        Self { data: Default::default() }
    }
}

impl Display for DataStorage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data.len())
    }
}

pub fn create_test_network() -> Network<Data, DataStorage> {
    Network::new(
        [
            Data::new(0.23052992, 0.95666552, 0.48200831),
            Data::new(0.40077599, 0.14291798, 0.55551944),
            Data::new(0.26027299, 0.17534256, 0.19371101),
            Data::new(0.18671211, 0.16638008, 0.77362103),
        ],
        0.25,
        0.1,
        0.25,
        0.1,
        500,
        Box::new(|| DataStorage::default()),
    )
}
