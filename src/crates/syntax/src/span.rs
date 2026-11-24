//! Source files, byte spans and their `file:line:col` locations.

use std::fmt;

/// Index of a file in a [`SourceMap`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileId(pub u32);

/// A half-open byte range `lo..hi` in one file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub lo: u32,
    pub hi: u32,
}

impl Span {
    pub fn new(file: FileId, lo: u32, hi: u32) -> Self {
        Span { file, lo, hi }
    }

    /// The smallest span covering `self` and `other` (same file).
    pub fn to(self, other: Span) -> Span {
        Span { file: self.file, lo: self.lo.min(other.lo), hi: self.hi.max(other.hi) }
    }
}

/// One source text. `first_line` is the line number of its first line, which
/// is not 1 for a code fence extracted from a Markdown file.
#[derive(Debug)]
pub struct SourceFile {
    pub path: String,
    pub text: String,
    pub first_line: u32,
    line_starts: Vec<u32>,
}

impl SourceFile {
    pub fn new(path: impl Into<String>, text: impl Into<String>, first_line: u32) -> Self {
        let text = text.into();
        let mut line_starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(to_u32(i + 1));
            }
        }
        SourceFile { path: path.into(), text, first_line, line_starts }
    }

    /// 1-based line and column (in characters) of a byte offset.
    pub fn line_col(&self, offset: u32) -> (u32, u32) {
        let idx = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let start = self.line_starts.get(idx).copied().unwrap_or(0) as usize;
        let end = (offset as usize).min(self.text.len());
        let col = self.text.get(start..end).map_or(0, |s| s.chars().count());
        (self.first_line + to_u32(idx), to_u32(col + 1))
    }

    /// The text a span covers, or `""` when it is out of range.
    pub fn slice(&self, span: Span) -> &str {
        self.text.get(span.lo as usize..span.hi as usize).unwrap_or("")
    }
}

/// Every file the toolchain has read, addressed by [`FileId`].
#[derive(Debug, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new() -> Self {
        SourceMap::default()
    }

    pub fn add(&mut self, file: SourceFile) -> FileId {
        self.files.push(file);
        FileId(to_u32(self.files.len() - 1))
    }

    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }

    pub fn location(&self, span: Span) -> Location {
        match self.get(span.file) {
            Some(f) => {
                let (line, col) = f.line_col(span.lo);
                Location { path: f.path.clone(), line, col }
            }
            None => Location { path: "<unknown>".into(), line: 0, col: 0 },
        }
    }

    pub fn slice(&self, span: Span) -> &str {
        self.get(span.file).map_or("", |f| f.slice(span))
    }
}

/// A human position: `path:line:col`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub path: String,
    pub line: u32,
    pub col: u32,
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{}", self.path, self.line, self.col)
    }
}

/// Offsets are `u32`: a source file larger than 4 GiB is refused by the loader.
pub(crate) fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_and_column_count_characters() {
        let f = SourceFile::new("a.slg", "ab\nčd\n", 10);
        assert_eq!(f.line_col(0), (10, 1));
        assert_eq!(f.line_col(3), (11, 1));
        assert_eq!(f.line_col(5), (11, 2));
    }
}
