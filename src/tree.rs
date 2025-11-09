use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::iter::Peekable;
use std::{fmt::Debug, fmt::Display, rc::Rc, str::Chars};

#[derive(Clone)]
pub enum DAG<T: Clone> {
    Leaf(T),
    Branch(Rc<DAG<T>>, Rc<DAG<T>>),
}

struct TreeTransformation<T: Clone> {
    source_to_target: Rc<DAG<Rc<DAG<T>>>>,
    target: Rc<DAG<T>>,
}

impl<T: Clone + Eq + Hash> TreeTransformation<T> {
    fn from_labels(left: Rc<DAG<T>>, right: Rc<DAG<T>>) -> Self {
        let mut label_map = HashMap::new();
        right.compute_label_map(&right, &|label| label.clone(), &mut label_map);
        let embedding = left.map(&mut (), &|_, value: &T, _| {
            label_map.get(value).unwrap().clone()
        });

        TreeTransformation {
            source_to_target: embedding,
            target: right,
        }
    }
}

impl<T: Clone> DAG<T> {
    fn map<S, R: Clone, F: Fn(&mut S, &T, &Self) -> R>(
        self: &Rc<Self>,
        state: &mut S,
        transformer: &F,
    ) -> Rc<DAG<R>> {
        self.replace_leaves(state, &|state, value, leaf| {
            Rc::new(DAG::<R>::Leaf(transformer(state, value, leaf)))
        })
    }

    fn replace_leaves<S, R: Clone, F: Fn(&mut S, &T, &Self) -> Rc<DAG<R>>>(
        self: &Rc<Self>,
        state: &mut S,
        transformer: &F,
    ) -> Rc<DAG<R>> {
        match self.as_ref() {
            DAG::Leaf(value) => transformer(state, value, self.as_ref()),
            DAG::Branch(left, right) => Rc::new(DAG::<R>::Branch(
                left.replace_leaves(state, transformer),
                right.replace_leaves(state, transformer),
            )),
        }
    }

    fn all_matches(
        self: &Rc<Self>,
        pattern_table: &HashMap<(usize, usize), usize>,
    ) -> Vec<Rc<Self>> {
        let mut matched = Vec::new();
        self.all_matches_inner(pattern_table, &mut matched);
        matched
    }

    fn all_matches_inner(
        self: &Rc<Self>,
        pattern_table: &HashMap<(usize, usize), usize>,
        matched: &mut Vec<Rc<Self>>,
    ) -> HashSet<usize> {
        let mut labels = HashSet::<usize>::new();
        labels.insert(0);
        match self.as_ref() {
            DAG::<T>::Leaf(_) => labels,
            DAG::<T>::Branch(left, right) => {
                let left_lables = left.all_matches_inner(pattern_table, matched);
                let right_lables = right.all_matches_inner(pattern_table, matched);
                for ((left_label, right_label), label) in pattern_table {
                    if left_lables.contains(left_label) && right_lables.contains(right_label) {
                        if *label == pattern_table.len() {
                            matched.push(self.clone());
                        }
                        labels.insert(*label);
                    }
                }
                labels
            }
        }
    }

    fn build_pattern_table(self: &Rc<Self>) -> HashMap<(usize, usize), usize> {
        let mut table = HashMap::new();
        self.build_pattern_table_inner(&mut table);
        table
    }

    fn build_pattern_table_inner(
        self: &Rc<Self>,
        table: &mut HashMap<(usize, usize), usize>,
    ) -> usize {
        match self.as_ref() {
            DAG::<T>::Leaf(_) => 0,
            DAG::<T>::Branch(left, right) => {
                let left_label = left.build_pattern_table_inner(table);
                let right_label = right.build_pattern_table_inner(table);
                match table.get(&(left_label, right_label)) {
                    Some(label) => *label,
                    None => {
                        table.insert((left_label, right_label), table.len() + 1);
                        table.len()
                    }
                }
            }
        }
    }

    fn pattern_embedding(
        self: &Rc<Self>,
        root: &Rc<Self>,
        pattern: &Rc<Self>,
    ) -> Option<Rc<DAG<Rc<Self>>>> {
        if !Rc::ptr_eq(self, root) {
            return match self.as_ref() {
                Self::Leaf(_) => None,
                Self::Branch(left, right) => left
                    .pattern_embedding(root, pattern)
                    .or(right.pattern_embedding(root, pattern)),
            };
        }

        Some(Rc::new(match (self.as_ref(), pattern.as_ref()) {
            (DAG::Leaf(_), DAG::Leaf(_)) => DAG::Leaf(self.clone()),
            (DAG::Branch(left, right), DAG::Branch(left_pattern, right_pattern)) => DAG::Branch(
                left.pattern_embedding(left, left_pattern).unwrap(),
                right.pattern_embedding(right, right_pattern).unwrap(),
            ),
            _ => panic!("Pattern is not embedded at this location"),
        }))
    }

    fn copying_insert(self: &Rc<Self>, root: &Rc<DAG<T>>, subtree: &Rc<DAG<T>>) -> Rc<DAG<T>> {
        if Rc::ptr_eq(self, root) {
            return subtree.clone();
        }
        Rc::new({
            match self.as_ref() {
                DAG::Leaf(value) => DAG::Leaf(value.clone()),
                DAG::Branch(left, right) => DAG::Branch(
                    left.copying_insert(root, subtree),
                    right.copying_insert(root, subtree),
                ),
            }
        })
    }

    fn substitue(self: &Rc<Self>, root: &Rc<Self>, transform: &TreeTransformation<T>) -> Rc<Self> {
        let embedding_map = transform.source_to_target.compute_embedding_map(root);

        let replacement = transform.target.replace_leaves(&mut (), &|_, _, leaf| {
            // be sure to really make a copy of the tree not just the references
            Rc::new(
                embedding_map
                    .get(&(leaf as *const DAG<T>))
                    .unwrap()
                    .as_ref()
                    .clone(),
            )
        });

        self.copying_insert(root, &replacement)
    }

    fn serialize(self: &Rc<Self>, result: &mut Vec<u8>) {
        match self.as_ref() {
            // avoid string allocations
            DAG::Branch(left, right) => {
                result.push(0);
                left.serialize(result);
                right.serialize(result);
            }
            DAG::Leaf(_) => {
                result.push(1);
            }
        }
    }

    fn ith_preorder_node(self: &Rc<Self>, i: usize) -> Result<usize, Rc<Self>> {
        if i == 0 {
            return Err(self.clone());
        }

        match self.as_ref() {
            DAG::Leaf(_) => Ok(i - 1),
            DAG::Branch(left, right) => right.ith_preorder_node(left.ith_preorder_node(i - 1)?),
        }
    }

    fn find_as_subtree(self: Rc<Self>, pattern: Rc<Self>) -> Rc<Self> {
        let mut s = Vec::<u8>::new();
        pattern.serialize(&mut s);
        let needle_len = s.len();
        s.push(42);
        self.serialize(&mut s);
        println!("{:?}", s);
        self.ith_preorder_node(kmp(&s, needle_len).unwrap())
            .expect_err("too few nodes :(")
    }
}

impl<T: Clone> DAG<T> {
    fn compute_label_map<K: Eq + Hash, S: Clone, F: Fn(&T) -> K>(
        self: &Rc<Self>,
        embedded_root: &Rc<DAG<S>>,
        labeler: &F,
        label_map: &mut HashMap<K, Rc<DAG<S>>>,
    ) {
        match self.as_ref() {
            Self::Leaf(value) => {
                label_map.insert(labeler(value), embedded_root.clone());
            }
            Self::Branch(left, right) => {
                if let DAG::<S>::Branch(left_root, right_root) = embedded_root.as_ref() {
                    left.compute_label_map(left_root, labeler, label_map);
                    right.compute_label_map(right_root, labeler, label_map);
                } else {
                    panic!("Self not embedded at this location")
                }
            }
        };
    }
}

impl<T: Clone> DAG<Rc<DAG<T>>> {
    fn compute_embedding_map(
        self: &Rc<Self>,
        root: &Rc<DAG<T>>,
    ) -> HashMap<*const DAG<T>, Rc<DAG<T>>> {
        let mut label_map = HashMap::new();

        self.compute_label_map(
            root,
            &|subtree: &Rc<DAG<T>>| subtree.as_ref() as *const DAG<T>,
            &mut label_map,
        );
        label_map
    }
}

impl DAG<String> {
    pub fn parse(input: &str) -> Rc<Self> {
        Self::parse_inner(&mut input.replace(" ", "").chars().peekable())
    }

    fn parse_inner(input: &mut Peekable<Chars>) -> Rc<Self> {
        let left = match input.next() {
            Some('(') => {
                let child = Self::parse_inner(input);
                assert_eq!(input.next(), Some(')'));
                child
            }
            Some(x @ ('a'..='z' | 'A'..='Z')) => Rc::new(Self::Leaf(x.to_string())),
            _ => panic!(),
        };

        match input.peek() {
            Some('*') => {
                input.next();
                let right = Self::parse_inner(input);
                Rc::new(Self::Branch(left, right))
            }
            _ => left,
        }
    }
}

impl DAG<()> {
    fn deserialize(input: &[u8]) -> Rc<Self> {
        let (tree, leftover) = Self::deserialize_inner(input);
        assert_eq!(leftover.len(), 0);
        tree
    }

    fn deserialize_inner(input: &[u8]) -> (Rc<Self>, &[u8]) {
        match input[0] {
            1 => (Rc::new(Self::Leaf(())), &input[1..]),
            0 => {
                let (lefttree, leftover) = Self::deserialize_inner(&input[1..]);
                let (righttree, rightover) = Self::deserialize_inner(leftover);
                (Rc::new(DAG::Branch(lefttree, righttree)), rightover)
            }
            _ => panic!(),
        }
    }
}

impl<T: Display + Clone> Display for DAG<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DAG::Leaf(val) => write!(f, "{}", val),
            DAG::Branch(left, right) => write!(f, "({} * {})", left, right),
        }
    }
}

impl<T: Display + Clone> Debug for DAG<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DAG::Leaf(val) => write!(f, "{}", val),
            DAG::Branch(left, right) => write!(f, "({:?} * {:?})", left, right),
        }
    }
}

fn kmp(s: &[u8], needle_len: usize) -> Option<usize> {
    let (mut i, mut j): (usize, usize) = (1, 0);
    let mut pi = vec![0usize; s.len()];

    while i < s.len() {
        while s[i] != s[j] && j != 0 {
            j = pi[j - 1];
        }
        j += (s[i] == s[j]) as usize;
        pi[i] = j;
        if pi[i] == needle_len {
            println!("i={}, {:?}", i, pi);
            println!("i - needle {}", i - needle_len);
            return Some(i - needle_len);
        }
        i += 1;
    }

    None
}

mod test {
    use super::*;

    #[test]
    fn test_serialize() {
        let s = &[0, 1, 0, 1, 0, 1, 1];
        let tree = DAG::deserialize(s);
        let mut serialized = Vec::<u8>::new();
        tree.serialize(&mut serialized);
        assert_eq!(serialized, s);
        println!("{:?}", serialized);
    }

    #[test]
    fn test_match() {
        let haystack = DAG::parse("a * (c * (b * d))");
        let before = DAG::parse("a * (b * c)");
        let after: Rc<DAG<String>> = DAG::parse("(b * a) * c");

        let table = before.build_pattern_table();
        let transform = TreeTransformation::from_labels(before, after);

        let matched = haystack.all_matches(&table);

        let first_match = &matched[1];

        let replaced = haystack.substitue(first_match, &transform);

        println!("{:?}", matched);
        println!("{:?}", replaced);
    }
}
