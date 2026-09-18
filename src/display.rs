pub struct WithDisplayContext<'v, T> {
    pub value: &'v T,
    pub indent_size: Option<usize>,
    pub indent_level: usize,
}

impl<'v, T> WithDisplayContext<'v, T> {
    pub fn set_indent(self, indent_size: usize) -> Self {
        Self { indent_size: Some(indent_size), ..self }
    }

    pub fn indent(self) -> Self {
        Self { indent_level: self.indent_level + 1, ..self }
    }

    pub fn no_indent(self) -> Self {
        Self { indent_size: None, ..self }
    }

    pub fn make_indent(&self) -> String {
        " ".repeat(self.indent_level * self.indent_size.unwrap_or(0))
    }
}

pub trait WithDisplayContextExt<'v, T> {
    fn disp(&'v self) -> WithDisplayContext<'v, T>;
}

impl<'v, T> WithDisplayContextExt<'v, T> for T {
    fn disp(&'v self) -> WithDisplayContext<'v, T> {
        WithDisplayContext { value: self, indent_size: None, indent_level: 0 }
    }
}
