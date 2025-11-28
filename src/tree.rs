use std::collections::HashMap;
use std::hash::Hash;
use std::iter::Peekable;
use std::{fmt::Debug, fmt::Display, rc::Rc, str::Chars};

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum DAG<T: Clone> {
    Leaf(T),
    Branch(Rc<DAG<T>>, Rc<DAG<T>>),
}

impl<T: Clone> DAG<T> {
    pub fn reduce<S, F: FnMut(&Rc<Self>, S, S) -> S, L: FnMut(&Rc<Self>, &T) -> S>(
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

    pub fn pre_reduce<
        S,
        P: FnMut(&Rc<Self>) -> Option<S>,
        F: FnMut(&Rc<Self>, S, S) -> S,
        L: FnMut(&Rc<Self>, &T) -> S,
    >(
        self: &Rc<Self>,
        pre_reduction: &mut P,
        post_reduction: &mut F,
        labeler: &mut L,
    ) -> S {
        match self.as_ref() {
            Self::Leaf(value) => labeler(self, value),
            Self::Branch(left, right) => {
                let pre_result = pre_reduction(self);

                if pre_result.is_some() {
                    pre_result.unwrap()
                } else {
                    let result_left = left.pre_reduce(pre_reduction, post_reduction, labeler);
                    let result_right = right.pre_reduce(pre_reduction, post_reduction, labeler);

                    post_reduction(self, result_left, result_right)
                }
            }
        }
    }

    pub fn propagate<S, F: FnMut(&Rc<Self>, S) -> (S, S), L: FnMut(&Rc<Self>, &T, S)>(
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

    pub fn walk_leaves<F: FnMut(&Rc<Self>, &T)>(self: &Rc<Self>, visitor: &mut F) {
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
    pub fn walk<F: FnMut(&Rc<Self>)>(self: &Rc<Self>, visitor: &mut F) {
        match self.as_ref() {
            Self::Leaf(_) => visitor(self),
            Self::Branch(left, right) => {
                left.walk(visitor);
                right.walk(visitor);

                visitor(self)
            }
        }
    }

    pub fn replace_leaves<R: Clone, F: FnMut(&Rc<Self>, &T) -> Rc<DAG<R>>>(
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

    pub fn build_pattern_table(self: &Rc<Self>) -> HashMap<(usize, usize), usize> {
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
}
