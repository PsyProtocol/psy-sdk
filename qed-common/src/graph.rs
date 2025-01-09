use std::collections::{HashMap, VecDeque};
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TraverseOrder {
    Enter,
    Visit,
    Exit,
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
        parent: Option<&'a T>,
        visitor: &mut impl FnMut(&'a T, Option<&'a T>, TraverseOrder),
    ) {
        visitor(node, parent, TraverseOrder::Enter);

        visitor(node, parent, TraverseOrder::Visit);

        if let Some(neighbors) = self.nodes.get(&node) {
            for neighbor in neighbors {
                self.dfs(neighbor, Some(node), visitor);
            }
        }

        visitor(node, parent, TraverseOrder::Exit);
    }

    pub fn bfs<'a>(
        &'a self,
        node: &'a T,
        visited: &mut HashMap<&'a T, bool>,
        visitor: &mut impl FnMut(&'a T),
    ) {
        let mut queue = VecDeque::new();
        queue.push_back(node);
        visited.insert(node, true);

        while let Some(node) = queue.pop_front() {
            visitor(node);

            if let Some(neighbors) = self.nodes.get(node) {
                for neighbor in neighbors {
                    if !visited.contains_key(neighbor) {
                        visited.insert(neighbor, true);
                        queue.push_back(neighbor);
                    }
                }
            }
        }
    }

    pub fn ts<'a>(
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
                        self.ts(neighbor, colors, visitor)?;
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

            if self.ts(node, &mut colors, &mut visitor).is_err() {
                return true;
            }
        }

        false
    }
}
