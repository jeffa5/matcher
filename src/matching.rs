#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct Graph {
    nodes: Vec<u32>,
    edges: Vec<Vec<u32>>,
}

impl Graph {
    pub fn add_edge(&mut self, id1: usize, id2: usize, weight: u32) {
        assert!(self.nodes.len() > id1);
        assert!(self.nodes.len() > id2);
        self.edges[id1][id2] = weight;
        self.edges[id2][id1] = weight;
    }

    pub fn add_node(&mut self, person: u32) -> usize {
        let id = self.nodes.len();
        self.nodes.push(person);
        for edge_row in &mut self.edges {
            edge_row.push(0);
        }
        self.edges.push(vec![0; self.nodes.len()]);
        id
    }

    pub fn nodes(&self) -> Vec<&u32> {
        self.nodes.iter().collect()
    }

    pub fn node(&self, id: usize) -> &u32 {
        &self.nodes[id]
    }

    pub fn edge(&self, id1: usize, id2: usize) -> u32 {
        self.edges[id1][id2]
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

    pub fn matching(&self) -> Vec<(usize, Option<usize>)> {
        let mut matchings = Vec::new();
        let nodes = self.nodes();
        let mut seen = vec![false; nodes.len()];

        for id in 0..nodes.len() {
            if seen[id] {
                continue;
            }

            let other_weights: Vec<_> = self
                .edges_for(id)
                .filter(|e| !seen[e.0])
                .filter(|e| id != e.0)
                .collect();

            let other_node = other_weights.iter().min_by_key(|e| e.1);
            if let Some(other_node) = other_node {
                if id < other_node.0 {
                    matchings.push((id, Some(other_node.0)));
                } else {
                    matchings.push((other_node.0, Some(id)));
                }
                seen[id] = true;
                seen[other_node.0] = true;
            } else {
                matchings.push((id, None))
            }
        }

        matchings
    }
}
