use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Deref;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Color {
    #[allow(dead_code)]
    White,
    Grey,
    Black,
}

#[derive(Clone, Debug)]
pub struct Graph<T> {
    nodes: HashMap<T, Vec<T>>,
}

impl<T> Deref for Graph<T> {
    type Target = HashMap<T, Vec<T>>;
    fn deref(&self) -> &Self::Target {
        &self.nodes
    }
}

impl<T: Clone + Eq + Hash> Graph<T> {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    pub fn add_edge(&mut self, from: T, to: T) {
        self.nodes.entry(from).or_default().push(to);
    }

    pub fn nodes(&self) -> Vec<&T> {
        self.nodes.keys().collect()
    }

    pub fn edges(&self, node: &T) -> Option<&Vec<T>> {
        self.nodes.get(node)
    }

    pub fn dfs<'a>(
        &'a self,
        node: &'a T,
        visited: &mut HashMap<&'a T, bool>,
        visitor: &mut impl FnMut(&'a T),
    ) {
        visited.insert(node, true);
        visitor(node);

        if let Some(neighbors) = self.nodes.get(&node) {
            for neighbor in neighbors {
                if !visited.contains_key(neighbor) {
                    self.dfs(neighbor, visited, visitor);
                }
            }
        }
    }

    fn toporder<'a>(
        &'a self,
        node: &'a T,
        colors: &mut HashMap<&'a T, Color>,
        visitor: &mut impl FnMut(&'a T),
    ) -> Result<(), ()> {
        colors.insert(node, Color::Grey);

        if let Some(neighbors) = self.nodes.get(&node) {
            for neighbor in neighbors {
                match colors.get(neighbor) {
                    Some(Color::Grey) => return Err(()),
                    None => {
                        self.toporder(neighbor, colors, visitor)?;
                    }
                    _ => {}
                }
            }
        }

        visitor(node);
        colors.insert(node, Color::Black);

        Ok(())
    }

    pub fn has_cycle(&self) -> bool {
        for node in self.nodes.keys() {
            let mut colors = HashMap::new();
            let mut visitor = |_: &T| {};

            if self.toporder(node, &mut colors, &mut visitor).is_err() {
                return true;
            }
        }

        false
    }

    pub fn topsort<'a>(&'a self, node: &'a T) -> Result<Vec<&'a T>, ()> {
        let mut colors = HashMap::new();
        let mut result = Vec::new();
        let mut visitor = |node: &'a T| {
            result.push(node);
        };

        self.toporder(node, &mut colors, &mut visitor)?;

        Ok(result)
    }
}
