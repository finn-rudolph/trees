pub trait BinaryDirectedAcyclicGraph<T> {
    fn children(&self) -> Option<(&Self, &Self)>;

    fn from_leaf(value: T) -> Self;
    fn from_children(left: Self, right: Self) -> Self;

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

    fn replace_leaves_with_count<
        S,
        R: BinaryDirectedAcyclicGraph<S>,
        F: FnMut(&Self, usize) -> R,
    >(
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
    fn replace_leaves<S, R: BinaryDirectedAcyclicGraph<S>, F: FnMut(&Self) -> R>(
        &self,
        transformer: &mut F,
    ) -> R {
        self.reduce(
            &mut #[inline(always)]
            |_, left, right| R::from_children(left, right),
            transformer,
        )
    }

    fn try_map<S, R: BinaryDirectedAcyclicGraph<S>, F: FnMut(&Self) -> Option<S>>(
        &self,
        transformer: &mut F,
    ) -> Option<R> {
        match self.children() {
            None => transformer(self).map(R::from_leaf),
            Some((left, right)) => {
                if let Some(left_result) = left.try_map(transformer) {
                    if let Some(right_result) = right.try_map(transformer) {
                        return Some(R::from_children(left_result, right_result));
                    }
                }
                None
            }
        }
    }

    fn map<S, R: BinaryDirectedAcyclicGraph<S>, F: FnMut(&Self) -> S>(
        &self,
        transformer: &mut F,
    ) -> R {
        self.reduce(
            &mut #[inline(always)]
            |_, left, right| R::from_children(left, right),
            &mut #[inline(always)]
            |leaf| R::from_leaf(transformer(leaf)),
        )
    }
}

pub trait BinaryChildren {
    fn children(&self) -> Option<(&Self, &Self)>;
}
