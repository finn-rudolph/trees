use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
    mem,
    ops::{Index, Mul, MulAssign},
    rc::Rc,
};

use crate::{transform::TreeTransform, tree::DAG};

#[derive(Clone)]
pub struct TreeMap<T: Clone>(HashMap<*const DAG<T>, Rc<DAG<T>>>);

impl<T: Clone> TreeMap<T> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn upgrade(self, source: &Rc<DAG<T>>) -> TreeBijection<T> {
        let mut inverse_map = TreeMap::new();

        source.walk_leaves(&mut |leaf, _| {
            inverse_map.insert(self[leaf.as_ref()].as_ref(), leaf.clone());
        });

        TreeBijection {
            left_to_right: self,
            right_to_left: inverse_map,
        }
    }

    pub fn insert(&mut self, left: &DAG<T>, right: Rc<DAG<T>>) {
        self.0.insert(left as *const DAG<T>, right);
    }

    pub fn from_embedding(needle: &Rc<DAG<T>>, haystack: &Rc<DAG<T>>) -> Self {
        let mut embedding = TreeMap::new();

        needle.propagate(
            haystack,
            &mut |_, embedded| {
                if let DAG::Branch(left, right) = embedded.as_ref() {
                    return (left, right);
                }
                panic!("pattern not embedded at this location");
            },
            &mut |leaf, _, embedded| {
                embedding.insert(leaf.as_ref(), embedded.clone());
            },
        );

        embedding
    }
}

impl<T: Clone> Index<&DAG<T>> for TreeMap<T> {
    type Output = Rc<DAG<T>>;

    fn index(&self, index: &DAG<T>) -> &Self::Output {
        self.0.get(&(index as *const DAG<T>)).unwrap()
    }
}

impl<T: Clone> Debug for TreeMap<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "TreeMap: ")?;
        for (domain, image) in &self.0 {
            writeln!(f, "\t{:p} -> {:p}", *domain, *image)?;
        }
        Ok(())
    }
}

impl<T: Clone> Mul<&TreeMap<T>> for &TreeMap<T> {
    type Output = TreeMap<T>;
    fn mul(self, rhs: &TreeMap<T>) -> Self::Output {
        let mut result_map = HashMap::new();

        for (k, v) in &self.0 {
            result_map.insert(*k, rhs[v.as_ref()].clone());
        }

        TreeMap(result_map)
    }
}

impl<T: Clone> MulAssign<&TreeMap<T>> for TreeMap<T> {
    fn mul_assign(&mut self, rhs: &TreeMap<T>) {
        for v in self.0.values_mut() {
            *v = rhs[v.as_ref()].clone();
        }
    }
}

#[derive(Clone)]
pub struct TreeBijection<T: Clone> {
    pub left_to_right: TreeMap<T>,
    pub right_to_left: TreeMap<T>,
}

impl<T: Clone> Mul<&TreeBijection<T>> for &TreeBijection<T> {
    type Output = TreeBijection<T>;
    fn mul(self, rhs: &TreeBijection<T>) -> Self::Output {
        TreeBijection {
            left_to_right: &self.left_to_right * &rhs.left_to_right,
            right_to_left: &rhs.right_to_left * &self.right_to_left,
        }
    }
}

impl<T: Clone> MulAssign<TreeBijection<T>> for TreeBijection<T> {
    fn mul_assign(&mut self, mut rhs: TreeBijection<T>) {
        self.left_to_right *= &rhs.left_to_right;
        rhs.right_to_left *= &self.right_to_left;
        self.right_to_left = rhs.right_to_left;
    }
}

impl<T: Clone> Debug for TreeBijection<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "TreeBijection: ")?;
        for (domain, image) in &self.left_to_right.0 {
            writeln!(f, "\t{:p} -> {:p}", *domain, *image)?;
        }
        for (domain, image) in &self.right_to_left.0 {
            writeln!(f, "\t{:p} <- {:p}", *domain, *image)?;
        }
        Ok(())
    }
}

impl<T: Clone> TreeBijection<T> {
    pub fn new() -> Self {
        Self {
            left_to_right: TreeMap::new(),
            right_to_left: TreeMap::new(),
        }
    }

    pub fn left_to_right(self) -> TreeMap<T> {
        self.left_to_right
    }

    pub fn lookup_left(&self, node: &DAG<T>) -> &Rc<DAG<T>> {
        &self.left_to_right[node]
    }

    pub fn lookup_right(&self, node: &DAG<T>) -> &Rc<DAG<T>> {
        &self.right_to_left[node]
    }

    pub fn insert(&mut self, left: Rc<DAG<T>>, right: Rc<DAG<T>>) {
        self.left_to_right.insert(left.as_ref(), right.clone());
        self.right_to_left.insert(right.as_ref(), left);
    }

    pub fn invert(&mut self) {
        mem::swap(&mut self.left_to_right, &mut self.right_to_left);
    }

    pub fn is_idenity(&self) -> bool {
        for (left, right) in &self.left_to_right.0 {
            if right.as_ref() as *const DAG<T> != *left {
                return false;
            }
        }
        true
    }

    pub fn track_clone(&mut self, tree: &Rc<DAG<T>>) -> Rc<DAG<T>> {
        tree.replace_leaves(&mut |leaf, value| {
            let result = Rc::new(DAG::Leaf(value.clone()));
            self.insert(result.clone(), leaf.clone());
            result
        })
    }

    pub fn track_insert(
        &mut self,
        root: &Rc<DAG<T>>,
        at: &Rc<DAG<T>>,
        what: &Rc<DAG<T>>,
    ) -> Rc<DAG<T>> {
        root.pre_reduce(
            &mut |node| {
                if Rc::ptr_eq(node, at) {
                    Some(what.clone())
                } else {
                    None
                }
            },
            &mut |_node, left, right| Rc::new(DAG::Branch(left, right)),
            &mut |leaf, value| {
                let result = Rc::new(DAG::Leaf(value.clone()));
                self.insert(result.clone(), leaf.clone());
                result
            },
        )
    }
}

impl<T: Clone + Hash + Eq + PartialEq> TreeBijection<T> {
    pub fn from_labels(left: &Rc<DAG<T>>, right: &Rc<DAG<T>>) -> Self {
        let mut label_map = HashMap::new();

        right.walk_leaves(&mut |leaf, label| {
            label_map.insert(label.clone(), leaf.clone());
        });

        let mut bijection = TreeBijection::new();

        left.walk_leaves(&mut |leaf, label| {
            bijection.insert(leaf.clone(), label_map.get(label).unwrap().clone());
        });

        bijection
    }
}

#[derive(Clone)]
pub struct TreeEquivalence<T: Clone> {
    pub left: Rc<DAG<T>>,
    pub right: Rc<DAG<T>>,
    pub left_pattern_table: HashMap<(usize, usize), usize>,
    pub right_pattern_table: HashMap<(usize, usize), usize>,
    pub bijection: TreeBijection<T>,
}

impl<T: Clone> TreeEquivalence<T> {
    pub fn left_to_right<'a>(&'a self) -> TreeTransform<'a, T> {
        TreeTransform::new(self, true)
    }

    pub fn right_to_left<'a>(&'a self) -> TreeTransform<'a, T> {
        TreeTransform::new(self, true)
    }

    pub fn left_to_right_map(&self) -> &TreeMap<T> {
        &self.bijection.left_to_right
    }

    pub fn right_to_left_map(&self) -> &TreeMap<T> {
        &self.bijection.right_to_left
    }

    // TODO: This is so stupid. Why is even nessary?
    pub fn remove_labels(self) -> TreeEquivalence<()> {
        let mut right_embedding = HashMap::<*const DAG<T>, Rc<DAG<()>>>::new();

        let new_right = self.right.replace_leaves(&mut |leaf, _| {
            let new_leaf = Rc::new(DAG::Leaf(()));
            right_embedding.insert(leaf.as_ref() as *const DAG<T>, new_leaf.clone());
            new_leaf
        });

        let mut new_bijection = TreeBijection::new();

        let new_left = self.left.replace_leaves(&mut |leaf, _| {
            let new_leaf = Rc::new(DAG::Leaf(()));
            new_bijection.insert(
                new_leaf.clone(),
                right_embedding
                    .get(&(self.bijection.lookup_left(&leaf).as_ref() as *const DAG<T>))
                    .unwrap()
                    .clone(),
            );
            new_leaf
        });

        TreeEquivalence {
            left: new_left,
            right: new_right,
            left_pattern_table: self.left_pattern_table,
            right_pattern_table: self.right_pattern_table,
            bijection: new_bijection,
        }
    }

    pub fn new(left: Rc<DAG<T>>, right: Rc<DAG<T>>, bijection: TreeBijection<T>) -> Self {
        TreeEquivalence {
            left_pattern_table: left.build_pattern_table(),
            right_pattern_table: right.build_pattern_table(),
            bijection,
            left,
            right,
        }
    }

    pub fn invert(&mut self) {
        self.bijection.invert();
        mem::swap(&mut self.left, &mut self.right);
        mem::swap(&mut self.left_pattern_table, &mut self.right_pattern_table);
    }

    pub fn is_idenity(&self) -> bool {
        Rc::ptr_eq(&self.left, &self.right) && self.bijection.is_idenity()
    }
}

impl<T: Clone + Eq + Hash> TreeEquivalence<T> {
    pub fn from_labels(left: Rc<DAG<T>>, right: Rc<DAG<T>>) -> Self {
        TreeEquivalence {
            bijection: TreeBijection::from_labels(&left, &right),
            left_pattern_table: left.build_pattern_table(),
            right_pattern_table: right.build_pattern_table(),
            left,
            right,
        }
    }
}

impl<T: Clone + Debug> Debug for TreeEquivalence<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            writeln!(f, "{:#?} -> {:#?}", self.left, self.right)?;
            writeln!(f, "with mapping: ")?;
            for (domain, image) in &self.bijection.left_to_right.0 {
                writeln!(f, "\t{:p} -> {:p}", *domain, *image)?;
            }
            Ok(())
        } else {
            let mut labels = HashMap::new();

            let formatted_source = self.right.map(&mut |leaf, _value| {
                let label = labels.len();
                labels.insert(leaf.as_ref() as *const DAG<T>, label);
                format!("<{}>", label)
            });

            let formatted_target = self.left.map(&mut |leaf, _value| {
                labels
                    .get(&(self.bijection.left_to_right[leaf].as_ref() as *const DAG<T>))
                    .map_or("<unk>".into(), |label| format!("<{}>", label))
            });

            write!(f, "{} -> {}", formatted_source, formatted_target)
        }
    }
}

impl<T: Clone> Mul<&TreeEquivalence<T>> for &TreeEquivalence<T> {
    type Output = TreeEquivalence<T>;
    fn mul(self, rhs: &TreeEquivalence<T>) -> Self::Output {
        assert!(Rc::ptr_eq(&self.right, &rhs.left));

        TreeEquivalence {
            bijection: &self.bijection * &rhs.bijection,
            left: self.left.clone(),
            right: rhs.right.clone(),
            left_pattern_table: self.left_pattern_table.clone(),
            right_pattern_table: rhs.right_pattern_table.clone(),
        }
    }
}
