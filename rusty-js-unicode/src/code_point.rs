#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CodePoint(pub(crate) char);

impl From<char> for CodePoint {
    fn from(c: char) -> Self {
        CodePoint(c)
    }
}

impl From<CodePoint> for char{
    fn from(c: CodePoint) -> Self {
        c.0
    }
}

impl CodePoint{
    pub fn as_u32(self) -> u32{
        self.0 as u32
    }
}

impl AsRef<char> for CodePoint{
    fn as_ref(&self) -> &char {
        &self.0
    }
}