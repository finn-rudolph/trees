use std::{collections::HashMap, fmt::Display, hash::Hash, iter::Peekable, rc::Rc, str::Chars};

use crate::{
    bidag::{BinaryChildren, FromChildren},
    maps::{TermBijection, TermMap},
    term::{Term, TermRef},
};

pub type LabeledTermRef<T> = Rc<LabeledTerm<T>>;

pub enum LabeledTerm<T> {
    Variable(T),
    Operation(Rc<LabeledTerm<T>>, Rc<LabeledTerm<T>>),
}

impl<T> LabeledTerm<T> {
    pub fn label(&self) -> Option<&T> {
        match self {
            Self::Variable(v) => Some(v),
            Self::Operation(_, _) => None,
        }
    }

    pub fn skeleton(&self) -> TermRef {
        self.map(&mut |_| ())
    }
}

impl LabeledTerm<String> {
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
            Some(x @ ('a'..='z' | 'A'..='Z')) => Rc::new(Self::Variable(x.to_string())),
            _ => panic!(),
        };

        match input.peek() {
            Some('*') => {
                input.next();
                let right = Self::parse_inner(input);
                Rc::new(Self::Operation(left, right))
            }
            _ => left,
        }
    }
}

impl<T: Clone + Hash + PartialEq + Eq> LabeledTerm<T> {
    pub fn map_to(self: LabeledTermRef<T>, target: LabeledTermRef<T>) -> TermMap<'static> {
        let mut target_labels = HashMap::new();

        target.walk_leaves(&mut |leaf| {
            target_labels.insert(leaf.label().unwrap().clone(), target_labels.len());
        });

        let mut map = Vec::new();

        self.walk_leaves(&mut |leaf| map.push(target_labels[leaf.label().unwrap()]));

        TermMap::new(self.skeleton(), target.skeleton(), map.into())
    }
}

impl<T> BinaryChildren for LabeledTerm<T> {
    fn children(&self) -> Option<(&Self, &Self)> {
        match self {
            LabeledTerm::Variable(_) => None,
            LabeledTerm::Operation(left, right) => Some((left, right)),
        }
    }
}

impl<T> BinaryChildren for Rc<LabeledTerm<T>> {
    fn children(&self) -> Option<(&Self, &Self)> {
        match self.as_ref() {
            LabeledTerm::Variable(_) => None,
            LabeledTerm::Operation(left, right) => Some((left, right)),
        }
    }
}

impl<T> FromChildren<T> for Rc<LabeledTerm<T>> {
    fn from_children(left: Self, right: Self) -> Self {
        Rc::new(LabeledTerm::Operation(left, right))
    }

    fn from_leaf(value: T) -> Self {
        Rc::new(LabeledTerm::Variable(value))
    }
}

impl<T: Display> Display for LabeledTerm<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.display_helper(
            f,
            &mut |node, f| write!(f, "("),
            &mut |node, f| write!(f, ")"),
            &mut |_, f| write!(f, " * "),
            &mut |leaf, f| write!(f, "{}", leaf.label().unwrap()),
        )
    }
}
