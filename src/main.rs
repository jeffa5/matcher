use std::collections::{HashMap, HashSet};

#[derive(Debug)]
struct Person {
    id: u32,
    name: String,
    email: String,
}

#[derive(Default, Debug)]
struct Graph {
    nodes: HashMap<u32, Person>,
    edges: HashMap<(u32, u32), u32>,
}

impl Graph {
    pub fn add_edge(&mut self, id1: u32, id2: u32, weight: u32) {
        assert!(self.nodes.contains_key(&id1));
        assert!(self.nodes.contains_key(&id2));
        if id1 > id2 {
            self.edges.insert((id2, id1), weight);
        } else {
            self.edges.insert((id1, id2), weight);
        }
    }

    pub fn add_node(&mut self, person: Person) {
        self.nodes.insert(person.id, person);
    }

    pub fn nodes(&self) -> Vec<&Person> {
        self.nodes.values().collect()
    }

    pub fn default_edges(&mut self) {
        let keys: Vec<_> = self.nodes.keys().copied().to_owned().collect();
        for node1 in &keys {
            for node2 in &keys {
                if node2 >= node1 {
                    continue;
                }
                if !self.edges.contains_key(&(*node1, *node2)) {
                    self.add_edge(*node1, *node2, 0);
                }
            }
        }
    }

    pub fn edges_for(&self, id: u32) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.edges
            .iter()
            .filter(move |((a, b), _)| *a == id || *b == id)
            .map(move |((a, b), w)| if *a == id { (*b, *w) } else { (*a, *w) })
    }

    pub fn contains_node(&self, id: u32) -> bool {
        self.nodes.contains_key(&id)
    }

    pub fn update_from_matching(&mut self, matching: &Vec<(u32, u32)>) {
        for (a, b) in matching {
            *self.edges.entry((*a, *b)).or_default() += 1;
        }
    }
}

fn maximal_matching(graph: &Graph) -> Vec<(u32, u32)> {
    let mut matchings = Vec::new();
    let mut seen = HashSet::new();
    let nodes = graph.nodes();

    for node in nodes {
        if seen.contains(&node.id) {
            continue;
        }

        let other_node = graph
            .edges_for(node.id)
            .filter(|e| !seen.contains(&e.0))
            .min_by_key(|e| e.1);
        if let Some(other_node) = other_node {
            if node.id == other_node.0 {
                continue;
            }
            if node.id < other_node.0 {
                matchings.push((node.id, other_node.0));
            } else {
                matchings.push((other_node.0, node.id));
            }
            seen.insert(node.id);
            seen.insert(other_node.0);
        }
    }

    matchings
}

fn main() {
    let mut graph = Graph::default();
    graph.add_node(Person {
        id: 1,
        name: "a".to_owned(),
        email: String::new(),
    });
    graph.add_node(Person {
        id: 2,
        name: "b".to_owned(),
        email: String::new(),
    });
    graph.add_node(Person {
        id: 3,
        name: "c".to_owned(),
        email: String::new(),
    });
    graph.add_node(Person {
        id: 4,
        name: "d".to_owned(),
        email: String::new(),
    });

    dbg!(graph.nodes());
    dbg!(&graph.edges);

    let matching1 = maximal_matching(&graph);
    dbg!(&matching1);
    graph.update_from_matching(&matching1);
    dbg!(&graph.edges);

    graph.default_edges();
    dbg!(&graph.edges);

    let matching2 = maximal_matching(&graph);
    dbg!(&matching2);
    graph.update_from_matching(&matching2);
    dbg!(&graph.edges);

    let matching3 = maximal_matching(&graph);
    dbg!(&matching3);
    graph.update_from_matching(&matching3);
    dbg!(&graph.edges);

    let matching4 = maximal_matching(&graph);
    dbg!(&matching4);
}
