use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::iter::Peekable;
use std::{fmt::Debug, fmt::Display, rc::Rc, str::Chars};

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum DAG<T: Clone> {
    Leaf(T),
    Branch(Rc<DAG<T>>, Rc<DAG<T>>),
}

type TreeMap<T> = DAG<Rc<DAG<T>>>;

struct TreeTransformation<T: Clone> {
    source_pattern_table: HashMap<(usize, usize), usize>,
    source_to_target: Rc<TreeMap<T>>,
    target: Rc<DAG<T>>,
}

struct TreeEquivalence<T: Clone> {
    left: Rc<DAG<T>>,
    right: Rc<DAG<T>>,
    left_to_right: Rc<TreeTransformation<T>>,
    right_to_left: Rc<TreeTransformation<T>>,
}

struct TreeEquivalenceClass<T: Clone> {
    trees: HashSet<Rc<DAG<T>>>,
    equivalences: HashMap<Rc<DAG<T>>, Rc<TreeEquivalence<T>>>,
}

impl<T: Clone + Eq + Hash> TreeEquivalence<T> {
    fn from_labels(left: Rc<DAG<T>>, right: Rc<DAG<T>>) -> Self {
        TreeEquivalence {
            left_to_right: Rc::new(TreeTransformation::from_labels(&left, &right)),
            right_to_left: Rc::new(TreeTransformation::from_labels(&right, &left)),
            left,
            right,
        }
    }
}

impl<T: Clone + Eq + Hash> TreeTransformation<T> {
    fn embedding_from_labels(before: &Rc<DAG<T>>, after: &Rc<DAG<T>>) -> Rc<TreeMap<T>> {
        let mut label_map = HashMap::new();
        after.compute_label_table(after, &|label| label.clone(), &mut label_map);
        before.map(&mut (), &|_, value: &T, _| {
            label_map.get(value).unwrap().clone()
        })
    }

    fn from_labels(before: &Rc<DAG<T>>, after: &Rc<DAG<T>>) -> Self {
        TreeTransformation {
            source_pattern_table: before.build_pattern_table(),
            source_to_target: Self::embedding_from_labels(&before, &after),
            target: after.clone(),
        }
    }

    fn all_matches(&self, haystack: &Rc<DAG<T>>) -> Vec<Rc<DAG<T>>> {
        haystack.all_matches(&self.source_pattern_table)
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
        let embedding_map = transform.source_to_target.compute_embedding_table(root);

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

    fn compute_label_table<K: Eq + Hash, S: Clone, F: Fn(&T) -> K>(
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
                    left.compute_label_table(left_root, labeler, label_map);
                    right.compute_label_table(right_root, labeler, label_map);
                } else {
                    panic!("Self not embedded at this location")
                }
            }
        };
    }
}

impl<T: Clone> TreeMap<T> {
    fn compute_embedding_table(
        self: &Rc<Self>,
        root: &Rc<DAG<T>>,
    ) -> HashMap<*const DAG<T>, Rc<DAG<T>>> {
        let mut label_map = HashMap::new();

        self.compute_label_table(
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

mod test {
    use super::*;

    #[test]
    fn test_eq() {
        assert_eq!(DAG::parse("a * (b * c)"), DAG::parse("a  * (b * c)"));
        assert_ne!(DAG::parse("a * (b * c)"), DAG::parse("a * (d * c)"));
    }

    #[test]
    fn test_match() {
        let haystack = DAG::parse("a * (c * (b * d))");

        let left = DAG::parse("a * (b * c)");
        let right: Rc<DAG<String>> = DAG::parse("(b * a) * c");

        let equivalence = TreeEquivalence::from_labels(left, right);

        let matched = equivalence.left_to_right.all_matches(&haystack);

        let first_match = &matched[1];

        let replaced = haystack.substitue(first_match, &equivalence.left_to_right);

        println!("{:?}", matched);
        println!("{:?}", replaced);

        let class: TreeEquivalenceClass<String> = TreeEquivalenceClass {
            trees: HashSet::new(),
            equivalences: HashMap::new(),
        };
    }
}
