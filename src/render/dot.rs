use std::borrow::Cow;
use std::io::Write;

use hashbrown::HashMap;

use crate::automata::Dfa;
use crate::solver::iterate_elements;

type Node = usize;
type Edge = (Node, Node);

struct Graph {
    edges: Vec<Edge>,
    nodes: Vec<Box<[usize]>>,
}

/// Renders a DFA representing a graph to DOT format.
/// The DFA must have an even number of tracks.
/// The first half of tracks represents source nodes, the second half represents target nodes.
pub fn render_set_dot<W: Write>(dfa: &Dfa, output: &mut W) {
    assert_eq!(dfa.n_tracks() % 2, 0);

    let mut edges = vec!();
    let mut nodes = vec!();
    let mut node_map: HashMap<Box<[usize]>, usize> = HashMap::new();
    let half = dfa.n_tracks() / 2;

    let mut insert = |nodes: &mut Vec<Box<[usize]>>, element: &[usize]| -> usize {
        match node_map.get(element) {
            Some(v) => *v,
            None => {
                let boxed = Box::<[usize]>::from(element);
                let index = nodes.len();
                nodes.push(boxed.clone());
                node_map.insert(boxed, index);
                index
            }
        }
    };

    iterate_elements(&dfa, None, |element| {
        let source = &element.values[0..half];
        let target = &element.values[half..];

        let source_index = insert(&mut nodes, source);
        let target_index = insert(&mut nodes, target);
        edges.push((source_index, target_index));
    });

    let graph = Graph {
        edges,
        nodes,
    };
    dot::render(&graph, output).unwrap();
}

impl<'a> dot::Labeller<'a, Node, Edge> for Graph {
    fn graph_id(&'a self) -> dot::Id<'a> {
        dot::Id::new("dfa").unwrap()
    }
    fn node_id(&'a self, node: &Node) -> dot::Id<'a> {
        dot::Id::new(format!("n{}", node)).unwrap()
    }
}

impl<'a> dot::GraphWalk<'a, Node, Edge> for Graph {
    fn nodes(&self) -> dot::Nodes<'a, Node> {
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for i in 0..nodes.len() {
            nodes.push(i);
        }
        Cow::Owned(nodes)
    }

    fn edges(&'a self) -> dot::Edges<'a, Edge> {
        Cow::Borrowed(&self.edges[..])
    }

    fn source(&self, e: &Edge) -> Node { e.0 }
    fn target(&self, e: &Edge) -> Node { e.1 }
}

#[cfg(test)]
mod tests {
    use crate::highlevel::parser::parse_setdef;
    use crate::solver::eval::evaluate_formula;

    use super::*;

    #[test]
    #[should_panic]
    fn reject_odd_number_of_tracks() {
        let mut dfa = Dfa::trivial(false);
        dfa.add_track();
        render_set_dot(&dfa, &mut Vec::new());
    }

    #[test]
    fn render_1d_loops() {
        let dfa = make_dfa("{ x, y | x < 10 and y < 10 and x == y }");
        let mut output = Vec::new();
        render_set_dot(&dfa, &mut output);
        assert_eq!(String::from_utf8(output).unwrap(), r#"digraph dfa {
    n0 -> n0[label=""];
    n1 -> n1[label=""];
    n2 -> n2[label=""];
    n3 -> n3[label=""];
    n4 -> n4[label=""];
    n5 -> n5[label=""];
    n6 -> n6[label=""];
    n7 -> n7[label=""];
    n8 -> n8[label=""];
    n9 -> n9[label=""];
}
"#)
    }

    #[test]
    fn render_2d_grid() {
        let dfa = make_dfa("{x, y, a, b | a < 4 and b < 4 and x < 4 and y < 4 and (x + 1 == a and y == b or x == a and y + 1 == b)}");
        let mut output = Vec::new();
        render_set_dot(&dfa, &mut output);
        assert_eq!(String::from_utf8(output).unwrap(), r#"digraph dfa {
    n0 -> n1[label=""];
    n2 -> n1[label=""];
    n3 -> n0[label=""];
    n3 -> n2[label=""];
    n4 -> n0[label=""];
    n5 -> n3[label=""];
    n6 -> n2[label=""];
    n7 -> n3[label=""];
    n8 -> n4[label=""];
    n5 -> n4[label=""];
    n9 -> n8[label=""];
    n9 -> n5[label=""];
    n10 -> n5[label=""];
    n11 -> n9[label=""];
    n7 -> n6[label=""];
    n12 -> n6[label=""];
    n13 -> n7[label=""];
    n13 -> n12[label=""];
    n10 -> n7[label=""];
    n14 -> n13[label=""];
    n11 -> n10[label=""];
    n14 -> n10[label=""];
    n15 -> n11[label=""];
    n15 -> n14[label=""];
}
"#)
    }

    fn make_dfa(setdef: &str) -> Dfa {
        evaluate_formula(&parse_setdef(setdef).formula().make_lo_formula()).into_dfa()
    }
}
