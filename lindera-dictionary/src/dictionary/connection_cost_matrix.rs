use crate::{LinderaResult, error::LinderaErrorKind, util::Data};

use byteorder::{ByteOrder, LittleEndian};

/// Byte length of the transposed format header:
/// `[i16 -1][i16 forward_size][i16 backward_size]`.
const NEW_FORMAT_HEADER_LEN: usize = 6;

/// Byte length of the legacy format header: `[i16 forward_size][i16 backward_size]`.
const OLD_FORMAT_HEADER_LEN: usize = 4;

/// A `*const i16` that is safe to share across threads.
///
/// A bare raw pointer is `!Send + !Sync`, which would strip both auto traits
/// from [`ConnectionCostMatrix`] and, through `Arc<ConnectionCostMatrix>`,
/// from `Dictionary` and every binding wrapper (`lindera-binding-core` has a
/// compile-time `assert_send_sync` for exactly this). Confining the assertion
/// to the pointer instead of blanket-asserting it for the whole struct keeps
/// a future non-`Send` field catchable by the compiler.
#[derive(Clone, Copy)]
struct CostsPtr(*const i16);

// SAFETY: the pointee is a `[i16]` that stays immutable for the lifetime of
// the owning `ConnectionCostMatrix` (its `storage` field keeps the allocation
// alive and no `&mut` path to it exists), and `i16` is `Send`. Sending the
// pointer therefore exposes no more than sending a `&'static [i16]` would.
unsafe impl Send for CostsPtr {}

// SAFETY: same as `Send`. The pointee is never mutated, so concurrent shared
// reads through this pointer cannot race, and `i16` is `Sync`.
unsafe impl Sync for CostsPtr {}

/// Owner of the cost values behind [`ConnectionCostMatrix`]'s cached pointer.
///
/// After [`ConnectionCostMatrix::load`] returns there is exactly one live
/// variant and it never changes. The enum exists only so the borrowed and
/// owned cases can share a single pointer-derivation site
/// ([`ConnectionCostMatrix::from_storage`]); the hot path never matches on it.
enum CostStorage {
    /// Zero-copy: the payload is read in place from `matrix.mtx`'s bytes.
    ///
    /// Only constructed after [`ConnectionCostMatrix::is_borrowable`] has
    /// confirmed a little-endian host and an `i16`-aligned payload start,
    /// which together mean the on-disk bytes already *are* the in-memory
    /// values.
    Borrowed(Data),
    /// Fallback: values decoded into an owned buffer, used when the payload
    /// cannot be viewed in place (unaligned base address, big-endian host, or
    /// the legacy non-transposed format, which needs a transpose anyway).
    /// `Vec<i16>` is inherently `i16`-aligned, so no explicit alignment work
    /// is needed here.
    Owned(Vec<i16>),
}

impl CostStorage {
    /// Returns the cost values this storage holds.
    ///
    /// Called only from [`ConnectionCostMatrix::from_storage`] at construction
    /// time; the hot path reads the cached pointer instead, so this `match` is
    /// never executed per lookup.
    ///
    /// # Returns
    ///
    /// The flat, transposed cost table.
    fn costs(&self) -> &[i16] {
        match self {
            // SAFETY: `ConnectionCostMatrix::is_borrowable` verified a
            // little-endian host and an `i16`-aligned payload start, and
            // `load` verified the buffer is at least `NEW_FORMAT_HEADER_LEN`
            // bytes long, so the subslice below cannot panic. `len() / 2`
            // rounds down, so every element lies fully inside the backing
            // allocation, whose size is by construction at most `isize::MAX`
            // bytes. The bytes are never mutated (`Data` is reached through
            // `&self` and no `&mut` path exists), a slice's `as_ptr()` is
            // never null, and `i16` has no invalid bit patterns.
            Self::Borrowed(data) => unsafe {
                let payload = &data[NEW_FORMAT_HEADER_LEN..];
                core::slice::from_raw_parts(payload.as_ptr().cast::<i16>(), payload.len() / 2)
            },
            Self::Owned(costs) => costs,
        }
    }
}

impl Clone for CostStorage {
    /// Clones the storage, re-establishing the borrow invariant rather than
    /// duplicating the variant blindly.
    ///
    /// `Data::Static` and `Data::Map` clone to the very same bytes (a copied
    /// `'static` reference, and an `Arc` sharing one mapping), so a borrow
    /// survives. `Data::Vec` does not: cloning it allocates a fresh buffer,
    /// and `Vec<u8>` guarantees only 1-byte alignment, so the copy can land
    /// at an address where the payload is no longer `i16`-aligned. Borrowing
    /// from such a buffer would be undefined behaviour, so this decodes into
    /// an owned buffer instead -- correct, just no longer zero-copy.
    ///
    /// # Returns
    ///
    /// Storage holding the same cost values, borrowed when that stays sound.
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(data) => {
                let cloned = data.clone();
                if ConnectionCostMatrix::is_borrowable(&cloned) {
                    Self::Borrowed(cloned)
                } else {
                    Self::Owned(self.costs().to_vec())
                }
            }
            Self::Owned(costs) => Self::Owned(costs.clone()),
        }
    }
}

/// The connection cost matrix, in transposed layout
/// (`costs[forward_id + backward_id * forward_size]`).
///
/// The values are borrowed from their backing bytes whenever possible (see
/// [`ConnectionCostMatrix::is_zero_copy`]); for UniDic that avoids a 71 MB
/// copy and 71 MB of anonymous RSS on every load.
///
/// `#[repr(C)]` is load-bearing: it pins the four fields the Viterbi inner
/// loop touches into the first 24 bytes, i.e. a single cache line, and keeps
/// the much larger `storage` -- which is read exactly once, at construction --
/// behind them. Letting the default layout interleave them measured as a
/// ~4.7% tokenize regression on IPADIC.
#[repr(C)]
pub struct ConnectionCostMatrix {
    /// Pointer to the first cost value, derived once in
    /// [`ConnectionCostMatrix::from_storage`].
    ///
    /// The address is stable across moves of `self`: moving `CostStorage`
    /// moves only the `Data`/`Vec` headers, while the pointee lives in
    /// `.rodata` (`Data::Static`), a stable heap allocation (`Data::Vec` and
    /// `CostStorage::Owned`), or an `Arc`-owned mapping (`Data::Map`).
    costs_ptr: CostsPtr,
    /// Number of `i16` cost values reachable from `costs_ptr`.
    costs_len: usize,
    /// Number of forward (left-context) ids, i.e. the length of one row.
    pub forward_size: u32,
    /// Number of backward (right-context) ids, i.e. the number of rows.
    pub backward_size: u32,
    /// Keeps the cost values alive. Never mutated after construction and
    /// never handed out by reference. Cold: nothing on the hot path reads it.
    storage: CostStorage,
}

impl ConnectionCostMatrix {
    /// Load a `ConnectionCostMatrix` from raw binary data.
    ///
    /// Supports the transposed format (header marker `-1`) and the legacy
    /// format. The transposed format is borrowed in place when the host is
    /// little-endian and the payload is `i16`-aligned; otherwise, and always
    /// for the legacy format, the values are decoded into an owned buffer.
    ///
    /// # Arguments
    ///
    /// * `conn_data` - Raw binary data for the connection cost matrix.
    ///
    /// # Returns
    ///
    /// A `ConnectionCostMatrix`, or an error if the data is too short, the
    /// header is malformed, or the axis sizes do not fit the payload.
    pub fn load(conn_data: impl Into<Data>) -> LinderaResult<ConnectionCostMatrix> {
        let conn_data = conn_data.into();
        if conn_data.len() < OLD_FORMAT_HEADER_LEN {
            return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                "Connection cost matrix data too short: {} bytes",
                conn_data.len()
            )));
        }

        let first_v = LittleEndian::read_i16(&conn_data[0..2]);

        if first_v == -1 {
            // New format (transposed)
            if conn_data.len() < NEW_FORMAT_HEADER_LEN {
                return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                    "Connection cost matrix header too short for new format: {} bytes",
                    conn_data.len()
                )));
            }
            let forward_size = LittleEndian::read_i16(&conn_data[2..4]) as u32;
            let backward_size = LittleEndian::read_i16(&conn_data[4..6]) as u32;
            // Round down: a trailing odd byte is not a whole cost value.
            let costs_len = (conn_data.len() - NEW_FORMAT_HEADER_LEN) / 2;
            Self::validate_axes(forward_size, backward_size, costs_len)?;

            let storage = if Self::is_borrowable(&conn_data) {
                CostStorage::Borrowed(conn_data)
            } else {
                let mut costs_data = vec![0i16; costs_len];
                // Slice exactly `costs_len * 2` bytes: `read_i16_into` panics
                // unless `src.len() == 2 * dst.len()`, which `&conn_data[6..]`
                // violates for an odd-length buffer.
                let end = NEW_FORMAT_HEADER_LEN + costs_len * 2;
                LittleEndian::read_i16_into(
                    &conn_data[NEW_FORMAT_HEADER_LEN..end],
                    &mut costs_data,
                );
                CostStorage::Owned(costs_data)
            };

            Ok(Self::from_storage(storage, forward_size, backward_size))
        } else {
            // Old format: laid out as `[backward_id + forward_id *
            // backward_size]`, so it must be transposed and can never be
            // viewed in place.
            let forward_size = first_v as u32;
            let backward_size = LittleEndian::read_i16(&conn_data[2..4]) as u32;
            let costs_len = (conn_data.len() - OLD_FORMAT_HEADER_LEN) / 2;
            Self::validate_axes(forward_size, backward_size, costs_len)?;

            let mut old_costs_data = vec![0i16; costs_len];
            let end = OLD_FORMAT_HEADER_LEN + costs_len * 2;
            LittleEndian::read_i16_into(
                &conn_data[OLD_FORMAT_HEADER_LEN..end],
                &mut old_costs_data,
            );

            // Transpose to new layout in memory
            let mut costs_data = vec![0i16; costs_len];
            for f in 0..forward_size {
                for b in 0..backward_size {
                    let old_id = (b + f * backward_size) as usize;
                    let new_id = (f + b * forward_size) as usize;
                    costs_data[new_id] = old_costs_data[old_id];
                }
            }

            Ok(Self::from_storage(
                CostStorage::Owned(costs_data),
                forward_size,
                backward_size,
            ))
        }
    }

    /// Builds a matrix from already-validated storage, deriving the cached
    /// costs pointer from it exactly once.
    ///
    /// This is the only site that computes `costs_ptr`. [`Clone`] routes
    /// through it as well, so a clone can never keep a pointer into the
    /// source's buffer.
    ///
    /// # Arguments
    ///
    /// * `storage` - Validated backing storage for the cost values.
    /// * `forward_size` - Number of forward context ids.
    /// * `backward_size` - Number of backward context ids.
    ///
    /// # Returns
    ///
    /// The constructed `ConnectionCostMatrix`.
    fn from_storage(storage: CostStorage, forward_size: u32, backward_size: u32) -> Self {
        let costs = storage.costs();
        // The borrow ends here; moving `storage` into the struct below moves
        // only the `Data`/`Vec` header, never the pointee, so this address
        // stays valid.
        let costs_ptr = CostsPtr(costs.as_ptr());
        let costs_len = costs.len();
        Self {
            storage,
            costs_ptr,
            costs_len,
            backward_size,
            forward_size,
        }
    }

    /// Whether the transposed payload inside `conn_data` can be viewed as
    /// `[i16]` in place.
    ///
    /// # Arguments
    ///
    /// * `conn_data` - The whole matrix buffer, header included. Must be at
    ///   least [`NEW_FORMAT_HEADER_LEN`] bytes long.
    ///
    /// # Returns
    ///
    /// `true` when the host is little-endian and the payload start is
    /// `i16`-aligned. The payload stores little-endian `i16` values, so on
    /// such a host the on-disk bytes already are the in-memory values.
    fn is_borrowable(conn_data: &Data) -> bool {
        // On a big-endian host every value would need a byte swap, which only
        // the owning path can do.
        if !cfg!(target_endian = "little") {
            return false;
        }
        // Pure address arithmetic; nothing is dereferenced here. `mmap` bases
        // are page-aligned and embedded data is aligned by
        // `include_bytes_aligned!`, so this holds on every shipped path. A
        // `Vec<u8>` from `read_file` satisfies it in practice too, but that is
        // not guaranteed, hence the runtime check.
        conn_data[NEW_FORMAT_HEADER_LEN..]
            .as_ptr()
            .cast::<i16>()
            .is_aligned()
    }

    /// Rejects a header whose axis sizes do not fit the payload.
    ///
    /// [`Self::row`] and [`Self::cost`] index a `costs_len`-long slice, so an
    /// oversized header could otherwise only surface as a panic at tokenize
    /// time. This also catches a `forward_size` above `i16::MAX`, which the
    /// `as u32` cast in the header parse wraps to roughly four billion.
    ///
    /// # Arguments
    ///
    /// * `forward_size` - Number of forward context ids from the header.
    /// * `backward_size` - Number of backward context ids from the header.
    /// * `costs_len` - Number of whole `i16` values in the payload.
    ///
    /// # Returns
    ///
    /// `Ok(())` when the payload holds at least `forward_size *
    /// backward_size` values, otherwise an error.
    fn validate_axes(forward_size: u32, backward_size: u32, costs_len: usize) -> LinderaResult<()> {
        let required = (forward_size as usize)
            .checked_mul(backward_size as usize)
            .ok_or_else(|| {
                LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                    "Connection cost matrix axes overflow: forward_size={forward_size}, backward_size={backward_size}"
                ))
            })?;
        if costs_len < required {
            return Err(LinderaErrorKind::Deserialize.with_error(anyhow::anyhow!(
                "Connection cost matrix payload holds {costs_len} values but the header requires {required} (forward_size={forward_size}, backward_size={backward_size})"
            )));
        }
        Ok(())
    }

    /// Returns the whole cost table as a flat slice in transposed layout.
    ///
    /// Replaces the former `costs_data` field, which forced every load to
    /// materialize an owned `Vec<i16>`.
    ///
    /// # Returns
    ///
    /// The cost values, indexed by `forward_id + backward_id * forward_size`.
    #[inline(always)]
    pub fn costs(&self) -> &[i16] {
        // SAFETY: `costs_ptr`/`costs_len` were derived in `from_storage` from
        // `self.storage`, which this `&self` borrow keeps alive. `storage`,
        // `costs_ptr` and `costs_len` are private and no method takes
        // `&mut self`, so the pointee is never mutated or reallocated. Moving
        // `self` does not move the pointee (`Data::Static` lives in `.rodata`,
        // `Data::Vec` and `CostStorage::Owned` in a stable heap allocation,
        // `Data::Map` in an `Arc`-owned mapping), and `Clone` re-derives the
        // pointer through `from_storage` rather than copying it. The
        // remaining preconditions were established by `CostStorage::costs`.
        unsafe { core::slice::from_raw_parts(self.costs_ptr.0, self.costs_len) }
    }

    /// Whether the cost values are read in place from the backing bytes,
    /// i.e. no copy was made at load time.
    ///
    /// Exposed so tests and diagnostics can assert that the mmap and embedded
    /// paths actually take the zero-copy branch.
    ///
    /// # Returns
    ///
    /// `true` when the matrix borrows its values, `false` when it owns a
    /// decoded copy.
    pub fn is_zero_copy(&self) -> bool {
        matches!(self.storage, CostStorage::Borrowed(_))
    }

    /// Returns the contiguous cost row for a fixed backward (right-context)
    /// id, so callers relaxing many forward ids against the same backward id
    /// pay the offset computation and bounds check once.
    ///
    /// # Arguments
    ///
    /// * `backward_id` - The backward context id selecting the row.
    ///
    /// # Returns
    ///
    /// A `forward_size`-long slice indexed directly by forward context id.
    #[inline]
    pub fn row(&self, backward_id: u32) -> &[i16] {
        let start = (backward_id * self.forward_size) as usize;
        &self.costs()[start..start + self.forward_size as usize]
    }

    #[inline]
    pub fn cost(&self, forward_id: u32, backward_id: u32) -> i32 {
        // Context-id access profiling (feature `ctxfreq`); compiled out by default.
        #[cfg(feature = "ctxfreq")]
        crate::builder::context_id_remap::record_access(forward_id, backward_id);

        let cost_id = (forward_id + backward_id * self.forward_size) as usize;
        self.costs()[cost_id] as i32
    }
}

impl Clone for ConnectionCostMatrix {
    /// Clones the matrix, re-deriving the cached costs pointer from the cloned
    /// storage.
    ///
    /// A derived `Clone` would copy `costs_ptr` verbatim. For
    /// `CostStorage::Borrowed(Data::Static)` and `Data::Map` that would happen
    /// to be sound (the pointee is `'static` or `Arc`-shared), but for
    /// `Data::Vec` and `CostStorage::Owned` the clone allocates a fresh buffer
    /// while the copied pointer still refers to the source's -- a
    /// use-after-free as soon as the source is dropped. Deriving `Clone` on
    /// this type is therefore forbidden.
    ///
    /// Alignment is handled one level down, in `CostStorage`'s own `Clone`:
    /// a reallocated `Data::Vec` may no longer be `i16`-aligned, so the
    /// borrow is re-validated there and falls back to an owned buffer.
    ///
    /// # Returns
    ///
    /// An independent `ConnectionCostMatrix` holding the same costs.
    fn clone(&self) -> Self {
        Self::from_storage(self.storage.clone(), self.forward_size, self.backward_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use byteorder::{LittleEndian, WriteBytesExt};

    /// Backing buffer whose `body` starts at a 16-byte boundary, so the cost
    /// payload at offset 6 is `i16`-aligned and the borrowed path is taken.
    #[repr(C, align(16))]
    struct AlignedBuf<const N: usize> {
        body: [u8; N],
    }

    /// Backing buffer whose `body` starts one byte past a 2-byte boundary, so
    /// the cost payload at offset 6 lands on an odd address and the owning
    /// fallback is taken. This is deterministic, unlike relying on whatever
    /// alignment the allocator happens to give a `Vec<u8>`.
    #[repr(C, align(2))]
    struct MisalignedBuf<const N: usize> {
        _pad: u8,
        body: [u8; N],
    }

    /// `[-1, 2, 3]` header followed by six costs in transposed layout.
    ///
    /// Spelled out as a `const` rather than built at runtime so it can back
    /// the `static`s below; leaking a `Box` instead would trip Miri's leak
    /// checker. `transposed_bytes_match_the_const` guards the hand-written
    /// encoding.
    const TRANSPOSED: [u8; 18] = [
        0xff, 0xff, // -1: transposed-layout marker
        0x02, 0x00, // forward_size = 2
        0x03, 0x00, // backward_size = 3
        0x0a, 0x00, 0x0b, 0x00, 0x0c, 0x00, 0x0d, 0x00, 0x0e, 0x00, 0x0f, 0x00,
    ];

    /// [`TRANSPOSED`] with a trailing byte, so the payload holds a partial
    /// `i16` at the end.
    const TRANSPOSED_ODD: [u8; 19] = [
        0xff, 0xff, 0x02, 0x00, 0x03, 0x00, 0x0a, 0x00, 0x0b, 0x00, 0x0c, 0x00, 0x0d, 0x00, 0x0e,
        0x00, 0x0f, 0x00, 0x00,
    ];

    /// `[-1, 1, 1]` plus a single cost of 7, mirroring `lindera-cc-cedict`'s
    /// 8-byte `matrix.mtx`.
    const DEGENERATE: [u8; 8] = [0xff, 0xff, 0x01, 0x00, 0x01, 0x00, 0x07, 0x00];

    static ALIGNED_TRANSPOSED: AlignedBuf<18> = AlignedBuf { body: TRANSPOSED };
    static MISALIGNED_TRANSPOSED: MisalignedBuf<18> = MisalignedBuf {
        _pad: 0,
        body: TRANSPOSED,
    };
    static MISALIGNED_TRANSPOSED_ODD: MisalignedBuf<19> = MisalignedBuf {
        _pad: 0,
        body: TRANSPOSED_ODD,
    };
    static ALIGNED_DEGENERATE: AlignedBuf<8> = AlignedBuf { body: DEGENERATE };

    fn assert_sample_costs(matrix: &ConnectionCostMatrix) {
        assert_eq!(matrix.forward_size, 2);
        assert_eq!(matrix.backward_size, 3);
        assert_eq!(matrix.cost(0, 0), 10);
        assert_eq!(matrix.cost(1, 0), 11);
        assert_eq!(matrix.cost(0, 1), 12);
        assert_eq!(matrix.cost(1, 1), 13);
        assert_eq!(matrix.cost(0, 2), 14);
        assert_eq!(matrix.cost(1, 2), 15);
        assert_eq!(matrix.row(0), &[10, 11]);
        assert_eq!(matrix.row(1), &[12, 13]);
        assert_eq!(matrix.row(2), &[14, 15]);
    }

    #[test]
    fn test_load_transposed() {
        let matrix = ConnectionCostMatrix::load(TRANSPOSED.to_vec()).unwrap();
        assert_sample_costs(&matrix);
    }

    #[test]
    fn test_load_old_format() {
        let mut data = Vec::new();
        data.write_i16::<LittleEndian>(2).unwrap(); // forward_size
        data.write_i16::<LittleEndian>(3).unwrap(); // backward_size
        // Old layout: [backward_id + forward_id * backward_size]
        // [0][0], [1][0], [2][0], [0][1], [1][1], [2][1]
        for v in [10i16, 12, 14, 11, 13, 15] {
            data.write_i16::<LittleEndian>(v).unwrap();
        }

        let matrix = ConnectionCostMatrix::load(data).unwrap();
        assert_sample_costs(&matrix);
        // A transpose is required, so the legacy format can never be borrowed.
        assert!(!matrix.is_zero_copy());
    }

    #[test]
    fn test_load_data_too_short() {
        let data: Vec<u8> = vec![0x01, 0x02];
        let result = ConnectionCostMatrix::load(data);
        assert!(result.is_err());
    }

    #[test]
    fn transposed_bytes_match_the_const() {
        let mut expected = Vec::new();
        expected.write_i16::<LittleEndian>(-1).unwrap();
        expected.write_i16::<LittleEndian>(2).unwrap();
        expected.write_i16::<LittleEndian>(3).unwrap();
        for v in [10i16, 11, 12, 13, 14, 15] {
            expected.write_i16::<LittleEndian>(v).unwrap();
        }
        assert_eq!(TRANSPOSED.as_slice(), expected.as_slice());

        assert_eq!(&TRANSPOSED_ODD[..18], TRANSPOSED.as_slice());
        assert_eq!(TRANSPOSED_ODD[18], 0);
    }

    #[test]
    fn borrowed_and_owned_produce_identical_costs() {
        let borrowed = ConnectionCostMatrix::load(&ALIGNED_TRANSPOSED.body[..]).unwrap();
        let owned = ConnectionCostMatrix::load(&MISALIGNED_TRANSPOSED.body[..]).unwrap();

        assert!(borrowed.is_zero_copy(), "aligned payload must be borrowed");
        assert!(
            !owned.is_zero_copy(),
            "misaligned payload must fall back to an owned copy"
        );

        assert_sample_costs(&borrowed);
        assert_sample_costs(&owned);
        assert_eq!(borrowed.costs(), owned.costs());
    }

    #[test]
    fn clone_does_not_alias_the_source_buffer() {
        // `Data::Vec` is the variant a derived `Clone` would get wrong twice
        // over: the clone allocates a fresh buffer, so a bitwise-copied
        // pointer would dangle once the source is dropped, and the fresh
        // buffer is only guaranteed 1-byte aligned, so keeping the borrow
        // could produce an unaligned `&[i16]`. Miri catches both.
        let source = ConnectionCostMatrix::load(TRANSPOSED.to_vec()).unwrap();
        let cloned = source.clone();
        drop(source);
        assert_sample_costs(&cloned);
    }

    #[test]
    fn cloning_a_static_backed_matrix_stays_zero_copy() {
        // `Data::Static` clones to the very same bytes, so the borrow -- and
        // with it the alignment that `load` validated -- survives.
        let source = ConnectionCostMatrix::load(&ALIGNED_TRANSPOSED.body[..]).unwrap();
        assert!(source.is_zero_copy());

        let cloned = source.clone();
        assert!(cloned.is_zero_copy());
        assert_eq!(cloned.costs().as_ptr(), source.costs().as_ptr());
        assert_sample_costs(&cloned);
    }

    #[test]
    fn matrix_survives_move() {
        let matrix = ConnectionCostMatrix::load(TRANSPOSED.to_vec()).unwrap();
        let boxed = Box::new(matrix);
        assert_sample_costs(&boxed);

        // Force the holding `Vec` to reallocate, moving the struct itself.
        let mut holder = Vec::with_capacity(1);
        holder.push(*boxed);
        for _ in 0..64 {
            holder.push(ConnectionCostMatrix::load(TRANSPOSED.to_vec()).unwrap());
        }
        for matrix in &holder {
            assert_sample_costs(matrix);
        }
    }

    #[test]
    fn connection_cost_matrix_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ConnectionCostMatrix>();
    }

    #[test]
    fn rejects_axes_larger_than_payload() {
        let mut data = Vec::new();
        data.write_i16::<LittleEndian>(-1).unwrap();
        data.write_i16::<LittleEndian>(100).unwrap();
        data.write_i16::<LittleEndian>(100).unwrap();
        data.write_i16::<LittleEndian>(0).unwrap();

        let result = ConnectionCostMatrix::load(data);
        assert!(result.is_err());
    }

    #[test]
    fn odd_length_payload_does_not_panic() {
        // `read_i16_into` panics unless `src.len() == 2 * dst.len()`, so the
        // owning path must slice off the trailing odd byte.
        let matrix = ConnectionCostMatrix::load(&MISALIGNED_TRANSPOSED_ODD.body[..]).unwrap();
        assert!(!matrix.is_zero_copy());
        assert_sample_costs(&matrix);
    }

    #[test]
    fn degenerate_single_cell_matrix_is_borrowable() {
        // `lindera-cc-cedict` ships an 8-byte `matrix.mtx` (`[-1, 1, 1]` plus
        // a single cost).
        let matrix = ConnectionCostMatrix::load(&ALIGNED_DEGENERATE.body[..]).unwrap();
        assert!(matrix.is_zero_copy());
        assert_eq!(matrix.cost(0, 0), 7);
    }
}
