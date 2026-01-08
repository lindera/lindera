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
}

impl ConnectionCostMatrix {
    pub fn load(conn_data: impl Into<Data>) -> ConnectionCostMatrix {
        let conn_data = conn_data.into();
        let backward_size = LittleEndian::read_i16(&conn_data[2..4]);
        let size = conn_data.len() / 2 - 2;
        let mut costs_data = vec![0i16; size];
        LittleEndian::read_i16_into(&conn_data[4..], &mut costs_data);

        ConnectionCostMatrix {
            costs_data,
            backward_size: backward_size as u32,
        }
    }

    pub fn cost(&self, forward_id: u32, backward_id: u32) -> i32 {
        let cost_id = (backward_id + forward_id * self.backward_size) as usize;
        self.costs_data[cost_id] as i32
    }
}

impl ArchivedConnectionCostMatrix {
    pub fn cost(&self, forward_id: u32, backward_id: u32) -> i32 {
        let cost_id = (backward_id + forward_id * self.backward_size) as usize;
        self.costs_data[cost_id].to_native() as i32
    }
}
