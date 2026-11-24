//! The canonical writer's pieces (docs/contract.md section 3, "Bytes written"):
//! one line, no white space, only `"` and `\` escaped, everything else raw UTF-8.

use super::StoreError;

/// The largest `Int` the store carries: 2^53.
pub const INT_LIMIT: i64 = 1 << 53;

/// One buffer for the whole document, handed to one `write`.
#[derive(Debug, Default)]
pub struct Writer {
    text: String,
}

impl Writer {
    pub fn raw(&mut self, text: &str) {
        self.text.push_str(text);
    }

    /// An integer, or `Full` when the store cannot carry it.
    pub fn int(&mut self, value: i64) -> Result<(), StoreError> {
        if !(0..=INT_LIMIT).contains(&value) {
            return Err(StoreError::Full);
        }
        self.text.push_str(&value.to_string());
        Ok(())
    }

    pub fn string(&mut self, value: &str) {
        self.text.push('"');
        let mut rest = value;
        while let Some(at) = rest.find(['"', '\\']) {
            let (plain, special) = rest.split_at(at);
            self.text.push_str(plain);
            self.text.push('\\');
            let mut chars = special.chars();
            if let Some(c) = chars.next() {
                self.text.push(c);
            }
            rest = chars.as_str();
        }
        self.text.push_str(rest);
        self.text.push('"');
    }

    /// `[item, item, ...]`.
    pub fn array<T>(
        &mut self,
        items: impl IntoIterator<Item = T>,
        mut item: impl FnMut(&mut Self, T) -> Result<(), StoreError>,
    ) -> Result<(), StoreError> {
        self.text.push('[');
        for (index, value) in items.into_iter().enumerate() {
            if index > 0 {
                self.text.push(',');
            }
            item(self, value)?;
        }
        self.text.push(']');
        Ok(())
    }

    pub fn finish(mut self) -> Vec<u8> {
        self.text.push('\n');
        self.text.into_bytes()
    }
}
