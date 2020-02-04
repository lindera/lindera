use byteorder::{ByteOrder, LittleEndian};
use lindera_ipadic::connection_data;

pub struct ConnectionCostMatrix {
    pub costs_data: &'static [u8],
    pub backward_size: u32,
}

impl ConnectionCostMatrix {
    pub fn load_default() -> ConnectionCostMatrix {
        let backward_size = LittleEndian::read_i16(&connection_data()[..2]);
        ConnectionCostMatrix {
            costs_data: &connection_data()[4..],
            backward_size: backward_size as u32,
        }
    }

    pub fn cost(&self, backward_id: u32, forward_id: u32) -> i32 {
        let cost_id = (forward_id + backward_id * self.backward_size) as usize;
        LittleEndian::read_i16(&self.costs_data[cost_id * 2..]) as i32
    }
}
