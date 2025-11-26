use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::iter::Peekable;
use std::{fmt::Debug, fmt::Display, rc::Rc, str::Chars};

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum DAG<T: Clone> {
    Leaf(T),
    Branch(Rc<DAG<T>>, Rc<DAG<T>>),
}

struct TreeMap<T: Clone>(HashMap<*const DAG<T>, Rc<DAG<T>>>);

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

pub struct TreeTransformation<T: Clone> {
    source_pattern_table: HashMap<(usize, usize), usize>,
    target_to_source: TreeMap<T>,
    source: Rc<DAG<T>>,
    target: Rc<DAG<T>>,
}

impl<T: Clone + Display> Display for TreeTransformation<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            writeln!(f, "{:#} -> {:#}", self.source, self.target)?;
            writeln!(f, "with mapping: ")?;
            for (domain, image) in &self.target_to_source.0 {
                writeln!(f, "\t{:p} -> {:p}", *domain, *image)?;
            }
            Ok(())
        } else {
            let mut labels = HashMap::new();

            let formatted_source = self.source.map(&mut |leaf, _value| {
                let label = labels.len();
                labels.insert(leaf.as_ref() as *const DAG<T>, label);
                format!("<{}>", label)
            });

            let formatted_target = self.target.map(&mut |leaf, _value| {
                labels
                    .get(&(self.target_to_source.lookup(leaf).as_ref() as *const DAG<T>))
                    .map_or("<unk>".into(), |label| format!("<{}>", label))
            });

            write!(f, "{} -> {}", formatted_source, formatted_target)
        }
    }
}

pub struct TreeEquivalence<T: Clone> {
    left: Rc<DAG<T>>,
    right: Rc<DAG<T>>,
    pub left_to_right: Rc<TreeTransformation<T>>,
    pub right_to_left: Rc<TreeTransformation<T>>,
}

struct TreeEquivalenceClass<T: Clone> {
    trees: HashSet<Rc<DAG<T>>>,
    origin: Rc<DAG<T>>,
    equivalences: HashMap<Rc<DAG<T>>, Rc<TreeEquivalence<T>>>,
}

impl<T: Clone + Eq + Hash> TreeEquivalence<T> {
    pub fn from_labels(left: Rc<DAG<T>>, right: Rc<DAG<T>>) -> Self {
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

        let mut tree_map = TreeMap::new();

        before.walk_leaves(&mut |leaf, label| {
            tree_map.insert(leaf, label_map.get(label).unwrap().clone());
        });

        tree_map
    }
    fn from_labels(before: &Rc<DAG<T>>, after: &Rc<DAG<T>>) -> Self {
        TreeTransformation {
            source_pattern_table: before.build_pattern_table(),
            target_to_source: Self::embedding_from_labels(&after, &before),
            target: after.clone(),
            source: before.clone(),
        }
    }

    pub fn all_matches(&self, haystack: &Rc<DAG<T>>) -> Vec<Rc<DAG<T>>> {
        haystack.all_matches(&self.source_pattern_table)
    }
}

impl<T: Clone> DAG<T> {
    fn reduce<S, F: FnMut(&Rc<Self>, S, S) -> S, L: FnMut(&Rc<Self>, &T) -> S>(
        self: &Rc<Self>,
        reduction: &mut F,
        labeler: &mut L,
    ) -> S {
        match self.as_ref() {
            Self::Leaf(value) => labeler(self, value),
            Self::Branch(left, right) => {
                let result_left = left.reduce(reduction, labeler);
                let result_right = right.reduce(reduction, labeler);

                reduction(self, result_left, result_right)
            }
        }
    }

    fn propagate<S, F: FnMut(&Rc<Self>, S) -> (S, S), L: FnMut(&Rc<Self>, &T, S)>(
        self: &Rc<Self>,
        value: S,
        propagation: &mut F,
        finalizer: &mut L,
    ) {
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
        self.reduce(
            &mut #[inline(always)]
            |_, _, _| (),
            &mut #[inline(always)]
            |leaf, value| {
                visitor(leaf, value);
            },
        )
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
        self.reduce(
            &mut #[inline(always)]
            |_, left, right| Rc::new(DAG::Branch(left, right)),
            transformer,
        )
    }

    pub fn map<R: Clone, F: FnMut(&Rc<Self>, &T) -> R>(
        self: &Rc<Self>,
        transformer: &mut F,
    ) -> Rc<DAG<R>> {
        self.reduce(
            &mut #[inline(always)]
            |_, left, right| Rc::new(DAG::Branch(left, right)),
            &mut #[inline(always)]
            |leaf, value| Rc::new(DAG::Leaf(transformer(leaf, value))),
        )
    }

    pub fn all_matches(
        self: &Rc<Self>,
        pattern_table: &HashMap<(usize, usize), usize>,
    ) -> Vec<Rc<Self>> {
        let mut matched = Vec::new();
        self.reduce(
            &mut |node, left_labels, right_labels| -> HashSet<usize> {
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
            },
            &mut |_leaf, _value| [0].into(),
        );

        matched
    }

    fn build_pattern_table(self: &Rc<Self>) -> HashMap<(usize, usize), usize> {
        let mut table = HashMap::new();

        self.reduce(
            &mut |_node, left_label, right_label| {
                if let Some(label) = table.get(&(left_label, right_label)) {
                    *label
                } else {
                    table.insert((left_label, right_label), table.len() + 1);
                    table.len()
                }
            },
            &mut |_, _| 0,
        );
        table
    }

    fn insert_at(
        self: &Rc<Self>,
        root: &Rc<DAG<T>>,
        subtree: &Rc<DAG<T>>,
        transform: &mut TreeMap<T>,
    ) -> Rc<DAG<T>> {
        self.reduce(
            &mut |node, left, right| {
                if Rc::ptr_eq(node, root) {
                    return subtree.clone();
                }
                Rc::new(Self::Branch(left, right))
            },
            &mut |leaf, value| {
                let result = Rc::new(DAG::Leaf(value.clone()));
                transform.insert(&result, leaf.clone());
                result
            },
        )
    }

    fn tracked_clone(self: &Rc<Self>, transform: &mut TreeMap<T>) -> Rc<DAG<T>> {
        self.replace_leaves(&mut |leaf, value| {
            let result = Rc::new(DAG::Leaf(value.clone()));
            transform.insert(&result, leaf.clone());
            result
        })
    }

    pub fn substitue(
        self: &Rc<Self>,
        root: &Rc<Self>,
        transform: &TreeTransformation<T>,
    ) -> (Rc<Self>, TreeTransformation<T>) {
        let embedding_map = root.compute_embedding_table(&transform.source);
        let mut result_map = TreeMap::new();

        let replacement = transform.target.replace_leaves(&mut |leaf, _| {
            embedding_map
                .lookup(transform.target_to_source.lookup(leaf).as_ref())
                .tracked_clone(&mut result_map)
        });

        let result = self.insert_at(root, &replacement, &mut result_map);
        let result_transform = TreeTransformation {
            source: self.clone(),
            target: result.clone(),
            source_pattern_table: self.build_pattern_table(),
            target_to_source: result_map,
        };

        (result, result_transform)
    }

    fn compute_embedding_table(self: &Rc<Self>, pattern: &Rc<Self>) -> TreeMap<T> {
        let mut embedding = TreeMap::new();

        pattern.propagate(
            self,
            &mut |_, embedded| {
                if let Self::Branch(left, right) = embedded.as_ref() {
                    return (left, right);
                }
                panic!("pattern not embedded at this location");
            },
            &mut |leaf, _, embedded| {
                embedding.insert(leaf, embedded.clone());
            },
        );

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

// impl Debug for DAG<()> {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             DAG::Leaf(_) => {
//                 write!(f, "[-]")
//             }
//             DAG::Branch(left, right) => {
//                 write!(f, "(")?;
//                 Debug::fmt(left, f)?;
//                 write!(f, " * ")?;
//                 Debug::fmt(right, f)?;
//                 write!(f, ")")
//             }
//         }
//     }
// }

impl<T: Display + Clone> Display for DAG<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DAG::Leaf(val) => {
                if f.alternate() {
                    write!(f, "{:#x}", (self as *const Self).addr())
                } else {
                    write!(f, "{}", val)
                }
            }
            DAG::Branch(left, right) => {
                write!(f, "(")?;
                Display::fmt(left, f)?;
                write!(f, " * ")?;
                Display::fmt(right, f)?;
                write!(f, ")")
            }
        }
    }
}

impl<T: Debug + Clone> Debug for DAG<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DAG::Leaf(val) => {
                if f.alternate() {
                    write!(f, "{:#x}", (self as *const Self).addr())
                } else {
                    write!(f, "{:?}", val)
                }
            }
            DAG::Branch(left, right) => {
                write!(f, "(")?;
                Debug::fmt(left, f)?;
                write!(f, " * ")?;
                Debug::fmt(right, f)?;
                write!(f, ")")
            }
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

        let (substituted, new_equivalence) =
            haystack.substitue(&matched[0], &equivalence.left_to_right);

        assert_eq!(
            DAG::parse("(a * ((b * c) * d))"),
            haystack
                .substitue(&matched[0], &equivalence.left_to_right)
                .0
        );

        assert_eq!(
            DAG::parse("((c * a) * (b * d))"),
            haystack
                .substitue(&matched[1], &equivalence.left_to_right)
                .0
        );
    }
}
