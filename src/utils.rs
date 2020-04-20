pub trait IterUtils<S, E> {
    fn transpose(self) -> Result<Vec<S>, E>;
    fn collect_errors(self) -> Result<(), Vec<E>>;
}

impl<I: IntoIterator<Item = Result<S, E>>, S, E> IterUtils<S, E> for I {
    fn transpose(self) -> Result<Vec<S>, E> {
        let list = self.into_iter();

        list.fold(Ok(Vec::new()), |acc: Result<Vec<S>, E>, a| match (acc, a) {
            (Ok(mut acc), Ok(a)) => {
                acc.push(a);
                Ok(acc)
            }
            (Err(e), _) => Err(e),
            (_, Err(e)) => Err(e),
        })
    }

    fn collect_errors(self) -> Result<(), Vec<E>> {
        let list = self.into_iter();

        list.fold(Ok(()), |acc: Result<(), Vec<E>>, a| match (acc, a) {
            (Err(mut acc), Err(e)) => {
                acc.push(e);
                Err(acc)
            }
            (Ok(_), Err(e)) => Err(vec![e]),
            (Err(acc), Ok(_)) => Err(acc),
            (Ok(_), Ok(_)) => Ok(()),
        })
    }
}
