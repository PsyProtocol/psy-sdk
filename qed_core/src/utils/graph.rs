use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BidirectionalGraph<T: Eq + Hash + Debug> {
    edges: HashMap<T, IndexSet<T>>,
    reverse_edges: HashMap<T, IndexSet<T>>,
}

impl<T: Eq + Hash + Clone + Debug> BidirectionalGraph<T> {
    pub fn new() -> Self {
        BidirectionalGraph {
            edges: HashMap::new(),
            reverse_edges: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.edges.clear();
        self.reverse_edges.clear();
    }

    pub fn add_edge(&mut self, from: T, to: T) {
        self.edges
            .entry(from.clone())
            .or_default()
            .insert(to.clone());
        self.reverse_edges.entry(to).or_default().insert(from);
    }

    pub fn add_node(&mut self, node: T) {
        self.edges.entry(node.clone()).or_default();
        self.reverse_edges.entry(node).or_default();
    }

    pub fn get_dependencies(&self, node: &T) -> Option<&IndexSet<T>> {
        self.edges.get(node)
    }

    pub fn get_dependents(&self, node: &T) -> Option<&IndexSet<T>> {
        self.reverse_edges.get(node)
    }

    pub fn ts_order(&self) -> Vec<Vec<T>> {
        let mut in_degrees: HashMap<T, usize> = self
            .edges
            .keys()
            .map(|node| {
                let degree = self.edges.get(node).map_or(0, |deps| deps.len());
                (node.clone(), degree)
            })
            .collect();

        let mut current_layer: Vec<T> = in_degrees
            .iter()
            .filter_map(|(node, &degree)| {
                if degree == 0 {
                    Some(node.clone())
                } else {
                    None
                }
            })
            .collect();

        let mut sorted_layers = Vec::new();
        let mut processed_tasks_count = 0;

        while !current_layer.is_empty() {
            sorted_layers.push(current_layer.clone());
            processed_tasks_count += current_layer.len();
            let mut next_layer = Vec::new();
            for item in current_layer.iter() {
                if let Some(reverse_edge) = self.reverse_edges.get(item) {
                    for reverse_edge in reverse_edge.iter() {
                        let degree = in_degrees.get_mut(reverse_edge).unwrap();
                        *degree -= 1;
                        if *degree == 0 {
                            next_layer.push(reverse_edge.clone());
                        }
                    }
                }
            }

            current_layer = next_layer;
        }

        if processed_tasks_count != self.edges.len() {
            panic!("Cycle detected in the task graph.");
        } else {
            sorted_layers
        }
    }
}
