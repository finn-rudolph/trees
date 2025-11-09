use std::collections::{HashMap, HashSet};
use std::iter::Peekable;
use std::{fmt::Debug, fmt::Display, rc::Rc, str::Chars};

pub enum DAG<T: Clone> {
    Leaf(T),
    Branch(Rc<DAG<T>>, Rc<DAG<T>>),
}

pub enum LabelTree<T: Clone, S: Clone> {
    Leaf(T, Rc<DAG<S>>),
    Branch(Rc<LabelTree<T, S>>, Rc<LabelTree<T, S>>, T, Rc<DAG<S>>),
}

impl<T: Clone, S: Clone> LabelTree<S, T> {
    fn value(&self) -> &S {
        match self {
            Self::Leaf(value, _) => value,
            Self::Branch(_, _, value, _) => value,
        }
    }
}

impl<T: Clone> LabelTree<usize, T> {}

impl<T: Clone> LabelTree<HashSet<usize>, T> {
    fn fill_iso_classes(self) -> ! {
        todo!()
    }
}

impl<T: Clone> DAG<T> {
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

    fn insert(self: &Rc<Self>, root: &Rc<DAG<T>>, subtree: &Rc<DAG<T>>) -> Rc<DAG<T>> {
        if Rc::ptr_eq(self, root) {
            return subtree.clone();
        }
        Rc::new({
            match self.as_ref() {
                DAG::Leaf(value) => DAG::Leaf(value.clone()),
                DAG::Branch(left, right) => {
                    DAG::Branch(left.insert(root, subtree), right.insert(root, subtree))
                }
            }
        })
    }

    fn substitue(
        self: &Rc<Self>,
        root: &Rc<Self>,
        pattern: &Rc<Self>,
        replacement: &Rc<DAG<Rc<Self>>>, // map from replacement to pattern
    ) {
        let embedding = self.pattern_embedding(root, pattern).unwrap();
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
        let needle = DAG::parse("a * (b * c)");

        let table = needle.build_pattern_table();
        let matched = haystack.all_matches(&table);

        let first_match = &matched[0];

        let replaced = haystack.substitue(first_match, &needle);

        println!("{:?}", replaced);
        println!("{:?}", matched);

        let after_op = DAG::parse("");
    }
}
