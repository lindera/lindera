use crate::LinderaResult;

pub trait CharacterFilter {
    fn apply(&self, text: &mut String) -> LinderaResult<(Vec<usize>, Vec<i64>)>;
}

pub fn correct_offset(offset: usize, offsets: &[usize], diffs: &[i64]) -> usize {
    // If `offsets` is empty, the `offset` specified is the correct offset.
    if offsets.is_empty() {
        return offset;
    }

    // Finds the `index` containing the specified `offset` from the `offsets`.
    let index = match offsets.binary_search(&offset) {
        Ok(i) => i,
        Err(i) => {
            if i != 0 {
                // If `i` is greater than `0`, then `i - 1` is the `index` for the `diff` of the specified `offset`.
                i - 1
            } else {
                // If the `offset` is not found and `i` is 0,
                // the specified `offset` is the correct offset.
                return offset;
            }
        }
    };

    // The correct offset value can be calculated by adding `diff[index]` to the given `offset`.
    (offset as i64 + diffs[index]) as usize
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_correct_offset() {
        let offsets = vec![2, 3, 7, 10, 11, 12];
        let diffs = vec![-1, -2, -3, -4, -5, -6];

        let offset = 2;
        let correct_offset = super::correct_offset(offset, &offsets, &diffs);
        assert_eq!(1, correct_offset);
    }
}
