#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:06}..{:06}", self.start, self.end)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Option<Span>,
}

impl<T: PartialEq> PartialEq<Spanned<T>> for Spanned<T> {
    fn eq(&self, other: &Spanned<T>) -> bool {
        if self.span.is_none() || other.span.is_none() {
            self.value == other.value
        } else {
            self.value == other.value && self.span == other.span
        }
    }
}

pub trait SpannedExt: Sized {
    fn sbetween(self, start: usize, end: usize) -> Spanned<Self> {
        Spanned { value: self, span: Some(Span { start, end: end.max(start + 1) }) }
    }

    #[cfg(test)]
    fn snone(self) -> Spanned<Self> {
        Spanned { value: self, span: None }
    }

    fn sinherit<T>(self, parent: &Spanned<T>) -> Spanned<Self> {
        Spanned { value: self, span: parent.span }
    }

    fn sjoin<T, U>(self, start: &Spanned<T>, end: &Spanned<U>) -> Spanned<Self> {
        if let (Some(start), Some(end)) = (start.span, end.span) {
            self.sbetween(start.start, end.end)
        } else {
            Spanned { value: self, span: None }
        }
    }
}

impl<T> SpannedExt for T {}
