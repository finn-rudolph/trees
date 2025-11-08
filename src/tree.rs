use std::iter::Peekable;
use std::{fmt::Display, rc::Rc, str::Chars};

pub enum DAG<T: Clone> {
    Leaf(T),
    Branch(Rc<DAG<T>>, Rc<DAG<T>>),
}

impl<T: Clone> DAG<T> {
    fn substitue(self: &Rc<Self>, root: &Rc<DAG<T>>, replacement: &Rc<DAG<T>>) -> Rc<DAG<T>> {
        if Rc::ptr_eq(&self, &root) {
            return replacement.clone();
        }
        Rc::new({
            match self.as_ref() {
                DAG::Leaf(value) => DAG::Leaf(value.clone()),
                DAG::Branch(left, right) => DAG::Branch(
                    left.substitue(root, replacement),
                    right.substitue(root, replacement),
                ),
            }
        })
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
}
