use crate::util::Data;

use byteorder::{ByteOrder, LittleEndian};

#[derive(Clone)]
pub struct ConnectionCostMatrix {
    costs_data: Data,
    backward_size: u32,
}

impl ConnectionCostMatrix {
    pub fn load(conn_data: impl Into<Data>) -> ConnectionCostMatrix {
        let conn_data = conn_data.into();
        let backward_size = LittleEndian::read_i16(&conn_data[2..4]);
        ConnectionCostMatrix {
            costs_data: conn_data,
            backward_size: backward_size as u32,
        }
    }

    pub fn cost(&self, forward_id: u32, backward_id: u32) -> i32 {
        let cost_id = (backward_id + forward_id * self.backward_size) as usize;
        LittleEndian::read_i16(&self.costs_data[4 + cost_id * 2..]) as i32
    }
}
