use crate::util::Data;

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
    pub fn load(conn_data: impl Into<Data>) -> ConnectionCostMatrix {
        let conn_data = conn_data.into();
        let forward_size = LittleEndian::read_i16(&conn_data[0..2]);
        let backward_size = LittleEndian::read_i16(&conn_data[2..4]);
        let size = conn_data.len() / 2 - 2;
        let mut costs_data = Vec::with_capacity(size);
        // SAFETY: The vector is allocated with capacity `size`.
        // We set the length to `size` to allow `read_i16_into` to write into it.
        // `read_i16_into` writes to the entire slice, so no uninitialized memory remains (assuming correct input size).
        // However, `read_i16_into` takes &mut [i16], and we are casting from uninitialized memory.
        // It is generally safer to populate with default values, but here we prioritize performance.
        // Since i16 carries no drop logic, this is sound as long as we fill it before reading.
        unsafe {
            costs_data.set_len(size);
        }
        LittleEndian::read_i16_into(&conn_data[4..], &mut costs_data);

        ConnectionCostMatrix {
            costs_data,
            backward_size: backward_size as u32,
            forward_size: forward_size as u32,
        }
    }

    pub fn cost(&self, forward_id: u32, backward_id: u32) -> i32 {
        let cost_id = (forward_id + backward_id * self.forward_size) as usize;
        self.costs_data[cost_id] as i32
    }
}

impl ArchivedConnectionCostMatrix {
    pub fn cost(&self, forward_id: u32, backward_id: u32) -> i32 {
        let cost_id = (forward_id + backward_id * self.forward_size) as usize;
        self.costs_data[cost_id].to_native() as i32
    }
}
