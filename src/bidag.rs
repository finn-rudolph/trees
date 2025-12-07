use crate::maps::NodeIndex;

pub trait BinaryChildren {
    fn children(&self) -> Option<(&Self, &Self)>;

    fn is_leaf(&self) -> bool {
        self.children().is_none()
    }

    fn reduce<S, F: FnMut(&Self, S, S) -> S, L: FnMut(&Self) -> S>(
        &self,
        reduction: &mut F,
        labeler: &mut L,
    ) -> S {
        match self.children() {
            None => labeler(self),
            Some((left, right)) => {
                let result_left = left.reduce(reduction, labeler);
                let result_right = right.reduce(reduction, labeler);

                reduction(self, result_left, result_right)
            }
        }
    }

    fn propagate<S, F: FnMut(&Self, S) -> (S, S), L: FnMut(&Self, S)>(
        &self,
        value: S,
        propagation: &mut F,
        finalizer: &mut L,
    ) {
        match self.children() {
            None => finalizer(self, value),
            Some((left, right)) => {
                let (left_prop, right_prop) = propagation(self, value);
                left.propagate(left_prop, propagation, finalizer);
                right.propagate(right_prop, propagation, finalizer);
            }
        }
    }

    fn walk_leaves<F: FnMut(&Self)>(&self, visitor: &mut F) {
        self.reduce(
            &mut #[inline(always)]
            |_, _, _| (),
            visitor,
        )
    }

    // cannot be reduced to reduce, because would need to have double mut borrow to visior
    fn walk<F: FnMut(&Self)>(&self, visitor: &mut F) {
        match self.children() {
            None => visitor(self),
            Some((left, right)) => {
                left.walk(visitor);
                right.walk(visitor);

                visitor(self)
            }
        }
    }

    fn replace_leaves<S, R: FromChildren<S>, F: FnMut(&Self) -> R>(
        &self,
        transformer: &mut F,
    ) -> R {
        self.reduce(
            &mut #[inline(always)]
            |_, left, right| R::from_children(left, right),
            transformer,
        )
    }

    fn counted_replace_leaves<S, R: FromChildren<S>, F: FnMut(&Self, NodeIndex) -> R>(
        &self,
        transformer: &mut F,
    ) -> R {
        let mut counter = 0;
        self.reduce(
            &mut #[inline(always)]
            |_, left, right| R::from_children(left, right),
            &mut #[inline(always)]
            |leaf| {
                let result = transformer(leaf, counter);
                counter += 1;
                result
            },
        )
    }

    fn try_map<S, R: FromChildren<S>, F: FnMut(&Self) -> Option<S>>(
        &self,
        transformer: &mut F,
    ) -> Option<R> {
        match self.children() {
            None => transformer(self).map(R::from_leaf),
            Some((left, right)) => {
                if let Some(left_result) = left.try_map(transformer)
                    && let Some(right_result) = right.try_map(transformer)
                {
                    return Some(R::from_children(left_result, right_result));
                }
                None
            }
        }
    }

    fn map<S, R: FromChildren<S>, F: FnMut(&Self) -> S>(&self, transformer: &mut F) -> R {
        self.reduce(
            &mut #[inline(always)]
            |_, left, right| R::from_children(left, right),
            &mut #[inline(always)]
            |leaf| R::from_leaf(transformer(leaf)),
        )
    }

    fn display_helper<
        T,
        E,
        S,
        FE: FnMut(&Self, &mut S) -> Result<T, E>,
        FL: FnMut(&Self, &mut S) -> Result<T, E>,
        FC: FnMut(&Self, &mut S) -> Result<T, E>,
        L: FnMut(&Self, &mut S) -> Result<T, E>,
    >(
        &self,
        state: &mut S,
        enter: &mut FE,
        leave: &mut FL,
        combine: &mut FC,
        leaf: &mut L,
    ) -> Result<T, E> {
        match self.children() {
            None => leaf(self, state),
            Some((left, right)) => {
                if !left.is_leaf() {
                    enter(left, state)?;
                    left.display_helper(state, enter, leave, combine, leaf)?;
                    leave(left, state)?;
                } else {
                    leaf(left, state)?;
                }
                combine(self, state)?;
                if !right.is_leaf() {
                    enter(right, state)?;
                    right.display_helper(state, enter, leave, combine, leaf)?;
                    leave(right, state)
                } else {
                    leaf(right, state)
                }
            }
        }
    }
}

pub trait FromChildren<T>: BinaryChildren {
    fn from_leaf(value: T) -> Self;
    fn from_children(left: Self, right: Self) -> Self;
}
