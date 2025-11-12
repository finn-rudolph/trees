use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::iter::Peekable;
use std::{fmt::Debug, fmt::Display, rc::Rc, str::Chars};

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum DAG<T: Clone> {
    Leaf(T),
    Branch(Rc<DAG<T>>, Rc<DAG<T>>),
}

struct TreeMap<T: Clone>(HashMap<*const DAG<T>, Rc<DAG<T>>>); //DAG<Rc<DAG<T>>>;

impl<T: Clone> TreeMap<T> {
    fn new() -> Self {
        Self(HashMap::new())
    }


    fn lookup(&self, node: &DAG<T>) -> Rc<DAG<T>> {
        self.0.get(&(node as *const DAG<T>)).unwrap().clone()
    }

    fn insert(&mut self, key: &DAG<T>, value: Rc<DAG<T>>) {
        self.0.insert(key as *const DAG<T>, value);
    }
}

struct TreeTransformation<T: Clone> {
    source_pattern_table: HashMap<(usize, usize), usize>,
    target_to_source: TreeMap<T>,
    source: Rc<DAG<T>>,
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
    origin: Rc<DAG<T>>,
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
    fn embedding_from_labels(before: &Rc<DAG<T>>, after: &Rc<DAG<T>>) -> TreeMap<T> {
        let mut label_map = HashMap::new();

        after.walk_leaves(&mut |leaf, label| {
            label_map.insert(label.clone(), leaf.clone());
        });

        let mut tree_map: HashMap<*const DAG<T>, Rc<DAG<T>>> = HashMap::new();

        before.walk_leaves(&mut |leaf, label| {
            tree_map.insert(
                leaf.as_ref() as *const DAG<T>,
                label_map.get(label).unwrap().clone(),
            );
        });

        TreeMap(tree_map)
    }
    fn from_labels(before: &Rc<DAG<T>>, after: &Rc<DAG<T>>) -> Self {
        TreeTransformation {
            source_pattern_table: before.build_pattern_table(),
            target_to_source: Self::embedding_from_labels(&after, &before),
            target: after.clone(),
            source: before.clone(),
        }
    }

    fn all_matches(&self, haystack: &Rc<DAG<T>>) -> Vec<Rc<DAG<T>>> {
        haystack.all_matches(&self.source_pattern_table)
    }
}

impl<T: Clone> DAG<T> {

    fn reduce<S, F: FnMut(&Rc<Self>, S, S) -> S, L: FnMut(&Rc<Self>, &T) -> S>(self: &Rc<Self>, reduction: &mut F, labeler: &mut L) -> S {
        match self.as_ref() {
            Self::Leaf(value) => labeler(self, value),
            Self::Branch(left, right) => {
                let result_left = left.reduce(reduction, labeler);
                let result_right = right.reduce(reduction, labeler);

                reduction(self, result_left, result_right)
            }

        }
    }

    fn propagate<S, F: FnMut(&Rc<Self>, S) -> (S, S), L: FnMut(&Rc<Self>, &T, S)>(self: &Rc<Self>, value: S, propagation: &mut F, finalizer: &mut L) {
        match self.as_ref() {
            Self::Leaf(label) => finalizer(self, label, value),
            Self::Branch(left, right) => {
                let (left_prop, right_prop) = propagation(self, value);
                left.propagate(left_prop, propagation, finalizer);
                right.propagate(right_prop, propagation, finalizer);
            }

        }
    }

    fn walk_leaves<F: FnMut(&Rc<Self>, &T)>(self: &Rc<Self>, visitor: &mut F) {
        self.reduce(&mut #[inline(always)] |_, _, _| (), &mut #[inline(always)] |leaf, value| {visitor(leaf, value); ()})
    }

    // cannot be reduced to reduce, because would need to have double mut borrow to visior
    fn walk<F: FnMut(&Rc<Self>)>(self: &Rc<Self>, visitor: &mut F) {
        match self.as_ref() {
            Self::Leaf(_) => visitor(self),
            Self::Branch(left, right) => {
                left.walk(visitor);
                right.walk(visitor);

                visitor(self)
            }

        }
    }

    fn replace_leaves<R: Clone, F: FnMut(&Rc<Self>, &T) -> Rc<DAG<R>>>(
        self: &Rc<Self>,
        transformer: &mut F,
    ) -> Rc<DAG<R>> {
        self.reduce(&mut #[inline(always)] |_, left, right| Rc::new(DAG::Branch(left, right)), transformer)
    }

    fn map<R: Clone, F: FnMut(&Rc<Self>, &T) -> R>(self: &Rc<Self>,
        transformer: &mut F,
    ) -> Rc<DAG<R>> {
        self.reduce(&mut #[inline(always)] |_, left, right| Rc::new(DAG::Branch(left, right)), &mut #[inline(always)] |leaf, value| Rc::new(DAG::Leaf(transformer(leaf, value))))
    }

    fn all_matches(
        self: &Rc<Self>,
        pattern_table: &HashMap<(usize, usize), usize>,
    ) -> Vec<Rc<Self>> {
        let mut matched = Vec::new();
        self.reduce(&mut |node, left_labels, right_labels| -> HashSet<usize> {
            let mut labels = HashSet::<usize>::from([0]);
            for ((left_label, right_label), label) in pattern_table {
                if left_labels.contains(left_label) && right_labels.contains(right_label) {
                    if *label == pattern_table.len() {
                        matched.push(node.clone());
                    }
                    labels.insert(*label);
                }
            }
            labels
        }, &mut |_leaf, _value| [0].into());

        matched
    }

    fn build_pattern_table(self: &Rc<Self>) -> HashMap<(usize, usize), usize> {
        let mut table = HashMap::new();

        self.reduce(&mut |_node, left_label, right_label| {
            if let Some(label) = table.get(&(left_label, right_label)) {
                *label
            } else {
                table.insert((left_label, right_label), table.len() + 1);
                table.len()
            }
        }, &mut |_, _| 0);
        table
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
        let embedding_map = root.compute_embedding_table(&transform.source);

        let replacement = transform.target.replace_leaves(&mut |leaf, _| {
            // be sure to really make a copy of the tree not just the references
            Rc::new(
                embedding_map
                    .lookup(transform.target_to_source.lookup(leaf).as_ref())
                    .as_ref()
                    .clone(),
            )
        });

        self.copying_insert(root, &replacement)
    }

    fn compute_embedding_table(self: &Rc<Self>, pattern: &Rc<Self>) -> TreeMap<T> {
        let mut embedding = TreeMap::new();
        // self.compute_embedding_table_inner(pattern, &mut embedding_table);

        pattern.propagate(self, &mut |_, embedded| {
            if let Self::Branch(left, right) = embedded.as_ref() {
                return (left, right);
            }
            panic!("pattern not embedded at this location");
        }, &mut |leaf, _, embedded| {
            embedding.insert(leaf, embedded.clone());
        });

        embedding
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

        println!("matched {:?}", matched);

        assert_eq!(haystack.substitue(&matched[0], &equivalence.left_to_right), DAG::parse("(a * ((b * c) * d))"));
        assert_eq!(haystack.substitue(&matched[1], &equivalence.left_to_right), DAG::parse("((c * a) * (b * d))"));

    }
}
