
use amd_flash::{FlashRead, FlashWrite};
use crate::ondisk::EMBEDDED_FIRMWARE_STRUCTURE_POSITION;
use crate::ondisk::Efh;
use crate::types::Result;
use crate::types::Error;
use zerocopy::LayoutVerified;
use crate::ondisk::header_from_collection_mut;

// TODO: Borrow storage.
pub struct Efs<T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> {
    storage: T,
}

impl<T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> Efs<T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
    pub fn load(storage: T) -> Result<Self> {
        Ok(Self {
            storage,
        })
    }
    pub fn create(mut storage: T) -> Result<Self> {
        // FIXME
        let mut buf: [u8; RW_BLOCK_SIZE] = [0xFF; RW_BLOCK_SIZE];
        match header_from_collection_mut(&mut buf[..]) {
            Some(item) => {
                let efh: Efh = Efh::default();
                *item = efh;
            }
            None => {
            },
        }

        storage.write_block(0x20_000, &buf)?;
        Self::load(storage)
    }
    // TODO: Extra arguments to filter by version
    pub fn embedded_firmware_structure(&self) -> Result<Efh> {
        for position in EMBEDDED_FIRMWARE_STRUCTURE_POSITION.iter() {
            let mut xbuf: [u8; RW_BLOCK_SIZE] = [0; RW_BLOCK_SIZE];

            self.storage.read_block(*position, &mut xbuf)?;
            let item = LayoutVerified::<_, Efh>::new_from_prefix(&xbuf[..]);
            match item {
                Some((item, _)) => {
                    let item = item.into_ref();
                    // TODO: item.compatible_with_processor_generation(0) for Milan; earlier processor generations don't have this, though.
                    if item.signature.get() == 0x55AA55AA && item.second_gen_efs() {
                        return Ok(*item);
                    }
                },
                None => {
                },
            }
        }
        Err(Error::HeaderNotFound)
    }
}
