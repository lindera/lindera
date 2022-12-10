use crate::LinderaResult;

pub trait CharacterFilter {
    fn name(&self) -> &str;
    fn apply(&self, text: &str) -> LinderaResult<(String, Vec<usize>, Vec<i64>)>;
}

pub fn add_offset_diff(offsets: &mut Vec<usize>, diffs: &mut Vec<i64>, offset: usize, diff: i64) {
    match offsets.last() {
        Some(&last_offset) => {
            if last_offset == offset {
                // Replace the last diff.
                diffs.pop();
                diffs.push(diff);
            } else {
                offsets.push(offset);
                diffs.push(diff);
            }
        }
        None => {
            // First offset.
            offsets.push(offset);
            diffs.push(diff);
        }
    }
}

pub fn correct_offset(offset: usize, offsets: &[usize], diffs: &[i64], text_len: usize) -> usize {
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
            } else if i >= text_len {
                text_len
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
        let text = "ABCDEFG";
        let filterd_text = "AbbbCdddFgggg";

        let text_len = filterd_text.len();
        let offsets = vec![2, 3, 7, 10, 11, 12];
        let diffs = vec![-1, -2, -3, -4, -5, -6];

        let start_b = 1;
        let end_b = 4;
        assert_eq!("bbb", &filterd_text[start_b..end_b]);
        let correct_start_b = super::correct_offset(start_b, &offsets, &diffs, text_len);
        let correct_end_b = super::correct_offset(end_b, &offsets, &diffs, text_len);
        assert_eq!(1, correct_start_b);
        assert_eq!(2, correct_end_b);
        assert_eq!("B", &text[correct_start_b..correct_end_b]);

        let start_g = 9;
        let end_g = 13;
        assert_eq!("gggg", &filterd_text[start_g..end_g]);
        let correct_start_g = super::correct_offset(start_g, &offsets, &diffs, text_len);
        let correct_end_g = super::correct_offset(end_g, &offsets, &diffs, text_len);
        assert_eq!(6, correct_start_g);
        assert_eq!(7, correct_end_g);
        assert_eq!("G", &text[correct_start_g..correct_end_g]);

        let start = 0;
        let end = 13;
        assert_eq!("AbbbCdddFgggg", &filterd_text[start..end]);
        let correct_start = super::correct_offset(start, &offsets, &diffs, text_len);
        let correct_end = super::correct_offset(end, &offsets, &diffs, text_len);
        assert_eq!(0, correct_start);
        assert_eq!(7, correct_end);
        assert_eq!("ABCDEFG", &text[correct_start..correct_end]);
    }
}
