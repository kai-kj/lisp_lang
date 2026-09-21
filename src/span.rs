#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:06}..{:06}", self.start, self.end)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

pub trait SpannedExt: Sized {
    fn sbetween(self, start: usize, end: usize) -> Spanned<Self> {
        Spanned { value: self, span: Span::new(start, end.max(start + 1)) }
    }

    fn scopy(self, span: Span) -> Spanned<Self> {
        Spanned { value: self, span }
    }

    fn sjoin<T, U>(self, start: &Spanned<T>, end: &Spanned<U>) -> Spanned<Self> {
        self.sbetween(start.span.start, end.span.end)
    }
}

impl<T> SpannedExt for T {}

pub trait ResultSpannedExt<T, E> {
    fn err_sbetween(self, start: usize, end: usize) -> Result<T, Spanned<E>>;
    fn err_scopy(self, span: Span) -> Result<T, Spanned<E>>;
    fn err_sjoin<A, B>(self, start: &Spanned<A>, end: &Spanned<B>) -> Result<T, Spanned<E>>;
}

impl<T, E> ResultSpannedExt<T, E> for Result<T, E> {
    fn err_sbetween(self, start: usize, end: usize) -> Result<T, Spanned<E>> {
        self.map_err(|err| err.sbetween(start, end))
    }

    fn err_scopy(self, span: Span) -> Result<T, Spanned<E>> {
        self.map_err(|err| err.scopy(span))
    }

    fn err_sjoin<A, B>(self, start: &Spanned<A>, end: &Spanned<B>) -> Result<T, Spanned<E>> {
        self.map_err(|err| err.sjoin(start, end))
    }
}
