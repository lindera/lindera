use crate::{LinderaResult, error::LinderaErrorKind, util::Data};

use byteorder::{ByteOrder, LittleEndian};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

#[derive(Clone, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct ConnectionCostMatrix {
    /// The connection cost matrix data.
    /// Previously, this was `Data` (byte array) and costs were read using `LittleEndian::read_i16` at runtime.
    /// Changed to `Vec<i16>` to enable direct array indexing and avoid deserialization overhead during tokenization.
    pub costs_data: Vec<i16>,
    pub backward_size: u32,
    pub forward_size: u32,
}

impl ConnectionCostMatrix {
    /// Load a `ConnectionCostMatrix` from raw binary data.
    ///
    /// Supports both the new transposed format (header marker `-1`) and the old format.
    ///
    /// # Arguments
    ///
    /// * `conn_data` - Raw binary data for the connection cost matrix.
    ///
    /// # Returns
    ///
    /// A `ConnectionCostMatrix`, or an error if the data is too short or malformed.
    pub fn load(conn_data: impl Into<Data>) -> LinderaResult<ConnectionCostMatrix> {
        let conn_data = conn_data.into();
        if conn_data.len() < 4 {
            return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                "Connection cost matrix data too short: {} bytes",
                conn_data.len()
            )));
        }

        let first_v = LittleEndian::read_i16(&conn_data[0..2]);

        if first_v == -1 {
            // New format (transposed)
            if conn_data.len() < 6 {
                return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                    "Connection cost matrix header too short for new format: {} bytes",
                    conn_data.len()
                )));
            }
            let forward_size = LittleEndian::read_i16(&conn_data[2..4]) as u32;
            let backward_size = LittleEndian::read_i16(&conn_data[4..6]) as u32;
            let size = conn_data.len() / 2 - 3;
            let mut costs_data = vec![0i16; size];
            LittleEndian::read_i16_into(&conn_data[6..], &mut costs_data);

            Ok(ConnectionCostMatrix {
                costs_data,
                backward_size,
                forward_size,
            })
        } else {
            // Old format
            let forward_size = first_v as u32;
            let backward_size = LittleEndian::read_i16(&conn_data[2..4]) as u32;
            let size = conn_data.len() / 2 - 2;
            let mut old_costs_data = vec![0i16; size];
            LittleEndian::read_i16_into(&conn_data[4..], &mut old_costs_data);

            // Transpose to new layout in memory
            let mut costs_data = vec![0i16; size];
            for f in 0..forward_size {
                for b in 0..backward_size {
                    let old_id = (b + f * backward_size) as usize;
                    let new_id = (f + b * forward_size) as usize;
                    costs_data[new_id] = old_costs_data[old_id];
                }
            }

            Ok(ConnectionCostMatrix {
                costs_data,
                backward_size,
                forward_size,
            })
        }
    }

    #[inline]
    pub fn cost(&self, forward_id: u32, backward_id: u32) -> i32 {
        let cost_id = (forward_id + backward_id * self.forward_size) as usize;
        self.costs_data[cost_id] as i32
    }
}

impl ArchivedConnectionCostMatrix {
    #[inline]
    pub fn cost(&self, forward_id: u32, backward_id: u32) -> i32 {
        let cost_id = (forward_id + backward_id * self.forward_size) as usize;
        self.costs_data[cost_id].to_native() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{LittleEndian, WriteBytesExt};

    #[test]
    fn test_load_transposed() {
        let mut data = Vec::new();
        data.write_i16::<LittleEndian>(-1).unwrap(); // version
        data.write_i16::<LittleEndian>(2).unwrap(); // forward_size
        data.write_i16::<LittleEndian>(3).unwrap(); // backward_size
        // [forward_id + backward_id * forward_size]
        // [0][0], [1][0], [0][1], [1][1], [0][2], [1][2]
        data.write_i16::<LittleEndian>(10).unwrap();
        data.write_i16::<LittleEndian>(11).unwrap();
        data.write_i16::<LittleEndian>(12).unwrap();
        data.write_i16::<LittleEndian>(13).unwrap();
        data.write_i16::<LittleEndian>(14).unwrap();
        data.write_i16::<LittleEndian>(15).unwrap();

        let matrix = ConnectionCostMatrix::load(data).unwrap();
        assert_eq!(matrix.forward_size, 2);
        assert_eq!(matrix.backward_size, 3);
        assert_eq!(matrix.cost(0, 0), 10);
        assert_eq!(matrix.cost(1, 0), 11);
        assert_eq!(matrix.cost(0, 1), 12);
        assert_eq!(matrix.cost(1, 1), 13);
        assert_eq!(matrix.cost(0, 2), 14);
        assert_eq!(matrix.cost(1, 2), 15);
    }

    #[test]
    fn test_load_old_format() {
        let mut data = Vec::new();
        data.write_i16::<LittleEndian>(2).unwrap(); // forward_size
        data.write_i16::<LittleEndian>(3).unwrap(); // backward_size
        // Old layout: [backward_id + forward_id * backward_size]
        // [0][0], [1][0], [2][0], [0][1], [1][1], [2][1]
        data.write_i16::<LittleEndian>(10).unwrap();
        data.write_i16::<LittleEndian>(12).unwrap();
        data.write_i16::<LittleEndian>(14).unwrap();
        data.write_i16::<LittleEndian>(11).unwrap();
        data.write_i16::<LittleEndian>(13).unwrap();
        data.write_i16::<LittleEndian>(15).unwrap();

        let matrix = ConnectionCostMatrix::load(data).unwrap();
        assert_eq!(matrix.forward_size, 2);
        assert_eq!(matrix.backward_size, 3);
        assert_eq!(matrix.cost(0, 0), 10);
        assert_eq!(matrix.cost(1, 0), 11);
        assert_eq!(matrix.cost(0, 1), 12);
        assert_eq!(matrix.cost(1, 1), 13);
        assert_eq!(matrix.cost(0, 2), 14);
        assert_eq!(matrix.cost(1, 2), 15);
    }

    #[test]
    fn test_load_data_too_short() {
        let data: Vec<u8> = vec![0x01, 0x02];
        let result = ConnectionCostMatrix::load(data);
        assert!(result.is_err());
    }
}
