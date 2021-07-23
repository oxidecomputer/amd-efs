
use amd_flash::Flash;
use crate::ondisk::embedded_firmware_structure_position;
use crate::ondisk::Efh;
use crate::types::Result;
use crate::types::Error;
use zerocopy::LayoutVerified;

// TODO: Borrow storage.
struct Efs<T: Flash> {
    storage: T,
}

/*
impl Flash for Efs {
        fn block_size() -> usize;
        fn read_block(location: Location, buffer: &mut [u8]) -> Result<()>;
        fn write_block(location: Location, buffer: &[u8]) -> Result<()>;
        fn erase_block(location: Location) -> Result<()>;
}
*/

impl<T: Flash> Efs<T> {
    pub fn new(storage: T) -> Self {
        Self {
            storage,
        }
    }
    // TODO: Extra arguments to filter by version
    pub fn embedded_firmware_structure(&self) -> Result<Efh> {
        for position in embedded_firmware_structure_position.iter() {
            let mut xbuf: [u8; 4096] = [0; 4096];

            self.storage.read_block(*position, &mut xbuf[..])?;
            let item = LayoutVerified::<_, Efh>::new(&xbuf[..]);
            match item {
                Some(item) => {
                    let item = item.into_ref();
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
