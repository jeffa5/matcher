use std::collections::{HashMap, HashSet};

#[derive(Debug)]
struct Person {
    id: u32,
    name: String,
    email: String,
}

#[derive(Default, Debug)]
struct Graph {
    nodes: HashMap<usize, Person>,
    edges: Vec<Vec<u32>>,
}

impl Graph {
    pub fn add_edge(&mut self, id1: usize, id2: usize, weight: u32) {
        assert!(self.nodes.contains_key(&id1));
        assert!(self.nodes.contains_key(&id2));
        self.edges[id1][id2] = weight;
        self.edges[id2][id1] = weight;
    }

    pub fn add_node(&mut self, person: Person) -> usize {
        let id = self.nodes.len();
        self.nodes.insert(id, person);
        for edge_row in &mut self.edges {
            edge_row.push(0);
        }
        self.edges.push(vec![0; self.nodes.len()]);
        id
    }

    pub fn nodes(&self) -> Vec<&Person> {
        self.nodes.values().collect()
    }

    pub fn edges_for(&self, id: usize) -> impl Iterator<Item = (usize, u32)> + '_ {
        self.edges[id].iter().enumerate().map(|(b, w)| (b, *w))
    }

    pub fn update_from_matching(&mut self, matching: &Vec<(usize, usize)>) {
        for (a, b) in matching {
            self.edges[*a][*b] += 1;
            self.edges[*b][*a] += 1;
        }
    }
}

fn maximal_matching(graph: &Graph) -> Vec<(usize, usize)> {
    let mut matchings = Vec::new();
    let mut seen = HashSet::new();
    let nodes = graph.nodes();

    for id in 0..nodes.len() {
        if seen.contains(&id) {
            continue;
        }

        let other_weights: Vec<_> = graph
            .edges_for(id)
            .filter(|e| !seen.contains(&e.0))
            .filter(|e| id != e.0)
            .collect();

        let other_node = other_weights.iter().min_by_key(|e| e.1);
        if let Some(other_node) = other_node {
            if id < other_node.0 {
                matchings.push((id, other_node.0));
            } else {
                matchings.push((other_node.0, id));
            }
            seen.insert(id);
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

    graph.add_edge(0, 1, 10);

    dbg!(graph.nodes());
    dbg!(&graph.edges);

    for _ in 0..20 {
        let matching1 = maximal_matching(&graph);
        println!("matching {:?}", matching1);
        graph.update_from_matching(&matching1);
        for row in &graph.edges {
            println!("{:?}", row);
        }
    }
}
