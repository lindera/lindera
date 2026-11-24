//! Writing and reading the bytes of a model: numbers in little endian, and
//! strings with their length.
//!
//! The model file, its parts and the schema inside it share these.

use crate::error::{Error, Result};

/// Appends a number as 4 bytes.
///
/// # Arguments
///
/// * `bytes` - The bytes to append to.
/// * `value` - The number.
pub(crate) fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

/// Appends a string: its length in bytes as 4 bytes, then its UTF-8.
///
/// # Arguments
///
/// * `bytes` - The bytes to append to.
/// * `value` - The string.
pub(crate) fn push_string(bytes: &mut Vec<u8>, value: &str) {
    push_u32(bytes, value.len() as u32);
    bytes.extend_from_slice(value.as_bytes());
}

/// Appends a list of strings: their number as 4 bytes, then each string.
///
/// # Arguments
///
/// * `bytes` - The bytes to append to.
/// * `values` - The strings.
pub(crate) fn push_strings(bytes: &mut Vec<u8>, values: &[String]) {
    push_u32(bytes, values.len() as u32);
    for value in values {
        push_string(bytes, value);
    }
}

/// Reads bytes from the first to the last.
pub(crate) struct Reader<'a> {
    /// The bytes.
    bytes: &'a [u8],
    /// The number of bytes read.
    position: usize,
    /// What the bytes are, for the messages: `model`.
    what: &'static str,
    /// Makes the error of a message.
    error: fn(String) -> Error,
}

impl<'a> Reader<'a> {
    /// Makes a reader at the first byte.
    ///
    /// # Arguments
    ///
    /// * `bytes` - The bytes.
    /// * `what` - What the bytes are, for the messages: `model`.
    /// * `error` - Makes the error of a message.
    ///
    /// # Returns
    ///
    /// The reader.
    pub(crate) fn new(bytes: &'a [u8], what: &'static str, error: fn(String) -> Error) -> Self {
        Self {
            bytes,
            position: 0,
            what,
            error,
        }
    }

    /// The error of something wrong in the bytes.
    ///
    /// # Arguments
    ///
    /// * `problem` - What is wrong, to follow `the <what> `.
    ///
    /// # Returns
    ///
    /// The error.
    pub(crate) fn error(&self, problem: impl std::fmt::Display) -> Error {
        (self.error)(format!("the {} {problem}", self.what))
    }

    /// The number of bytes read so far.
    ///
    /// # Returns
    ///
    /// The position of the next byte, from the first byte of the reader.
    pub(crate) fn position(&self) -> usize {
        self.position
    }

    /// The number of bytes not read yet.
    ///
    /// # Returns
    ///
    /// The number of bytes left.
    pub(crate) fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    /// Takes the next bytes.
    ///
    /// # Arguments
    ///
    /// * `length` - The number of bytes.
    ///
    /// # Returns
    ///
    /// The bytes, or an error when fewer are left.
    pub(crate) fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .position
            .checked_add(length)
            .filter(|end| *end <= self.bytes.len())
            .ok_or_else(|| {
                self.error(format_args!(
                    "ends early: {length} bytes expected at {}, {} left",
                    self.position,
                    self.remaining()
                ))
            })?;
        let taken = &self.bytes[self.position..end];
        self.position = end;
        Ok(taken)
    }

    /// Takes the next bytes as an array.
    ///
    /// # Returns
    ///
    /// The array, or the error of [`Reader::take`].
    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut array = [0; N];
        array.copy_from_slice(self.take(N)?);
        Ok(array)
    }

    /// Reads a byte.
    ///
    /// # Returns
    ///
    /// The byte, or the error of [`Reader::take`].
    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    /// Reads a number of 4 bytes.
    ///
    /// # Returns
    ///
    /// The number, or the error of [`Reader::take`].
    pub(crate) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    /// Reads a number of 8 bytes.
    ///
    /// # Returns
    ///
    /// The number, or the error of [`Reader::take`].
    pub(crate) fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    /// Reads a weight.
    ///
    /// # Returns
    ///
    /// The weight, or the error of [`Reader::take`].
    pub(crate) fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_le_bytes(self.array()?))
    }

    /// Reads a string.
    ///
    /// # Returns
    ///
    /// The string, or an error when the bytes end early or are not UTF-8.
    pub(crate) fn string(&mut self) -> Result<String> {
        let length = self.u32()? as usize;
        let bytes = self.take(length)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| self.error("has a string that is not UTF-8"))
    }

    /// Reads a list of strings.
    ///
    /// # Returns
    ///
    /// The strings, or the error of the first one that cannot be read.
    pub(crate) fn strings(&mut self) -> Result<Vec<String>> {
        let count = self.u32()?;
        // Nothing is reserved for the count: bytes that claim a huge list
        // end early instead.
        let mut values = Vec::new();
        for _ in 0..count {
            values.push(self.string()?);
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A reader of test bytes, with the errors of the chunker.
    fn reader(bytes: &[u8]) -> Reader<'_> {
        Reader::new(bytes, "test bytes", Error::Chunker)
    }

    #[test]
    fn numbers_and_strings_come_back_as_they_were_written() {
        let mut bytes = Vec::new();
        push_u32(&mut bytes, 0x0102_0304);
        push_string(&mut bytes, "文節");
        push_strings(&mut bytes, &["a".to_string(), String::new()]);
        // Little endian, and the length of a string in bytes.
        assert_eq!(bytes[..4], [4, 3, 2, 1]);
        assert_eq!(bytes[4..8], [6, 0, 0, 0]);

        let mut reader = reader(&bytes);
        assert_eq!(reader.u32().unwrap(), 0x0102_0304);
        assert_eq!(reader.string().unwrap(), "文節");
        assert_eq!(reader.strings().unwrap(), ["a", ""]);
        assert_eq!(reader.remaining(), 0);
    }

    #[test]
    fn bytes_that_end_early_or_are_not_utf8_are_errors() {
        let message = reader(&[1, 2, 3]).u32().unwrap_err().to_string();
        assert!(
            message.contains("the test bytes ends early: 4 bytes expected at 0, 3 left"),
            "{message}"
        );

        // A string of 2 bytes that are not UTF-8.
        let message = reader(&[2, 0, 0, 0, 0xff, 0xfe])
            .string()
            .unwrap_err()
            .to_string();
        assert!(message.contains("not UTF-8"), "{message}");

        // A list that claims more strings than there are bytes.
        let mut bytes = Vec::new();
        push_u32(&mut bytes, u32::MAX);
        assert!(reader(&bytes).strings().is_err());
    }
}
