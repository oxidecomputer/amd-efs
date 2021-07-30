
use amd_flash::{FlashRead, FlashWrite, Location};
use crate::ondisk::EMBEDDED_FIRMWARE_STRUCTURE_POSITION;
use crate::ondisk::{BiosDirectoryHeader, Efh};
pub use crate::ondisk::ProcessorGeneration;
use crate::types::Result;
use crate::types::Error;
use zerocopy::LayoutVerified;
use crate::ondisk::header_from_collection;
use crate::ondisk::header_from_collection_mut;

pub struct PspDirectory<'a, T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> {
    storage: &'a T,

}

pub struct EfhPspIterator<'a, T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> {
    storage: &'a T,
    current_position: Location,
}

impl<'a, T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> Iterator for EfhPspIterator<'a, T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
   type Item = PspDirectory<'a, T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>;
   fn next(&mut self) -> Option<<Self as Iterator>::Item> {
       Some(PspDirectory {
           storage: self.storage,
       })
   }
}

pub struct BiosDirectory<'a, T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> {
    storage: &'a T,
    location: Location,
    pub header: BiosDirectoryHeader,
}

impl<'a, T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> BiosDirectory<'a, T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
    fn new(storage: &'a T, location: Location) -> Result<Self> {
       let mut xbuf: [u8; RW_BLOCK_SIZE] = [0; RW_BLOCK_SIZE];
       storage.read_block(location, &mut xbuf)?;
       let (item, _) = LayoutVerified::<_, BiosDirectoryHeader>::new_from_prefix(&xbuf[..]).ok_or_else(|| Error::Marshal)?;
       let header = item.into_ref();
       Ok(Self {
           storage,
           location,
           header: *header,
       })
   }
}

pub struct EfhBiosIterator<'a, T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> {
    storage: &'a T,
    positions: [u32; 4], // 0xffff_ffff: invalid
    index_into_positions: usize,
}

impl<'a, T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> Iterator for EfhBiosIterator<'a, T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
   type Item = BiosDirectory<'a, T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>;
   fn next(&mut self) -> Option<<Self as Iterator>::Item> {
       while self.index_into_positions < self.positions.len() {
           let position = self.positions[self.index_into_positions];
           self.index_into_positions += 1;
           if position != 0xffff_ffff && position != 0 /* sigh.  Some images have 0 as "invalid" mark */ {
               match BiosDirectory::new(self.storage, position) {
                   Ok(e) => {
                       return Some(e);
                   },
                   Err(e) => {
                       return None; // FIXME: error check
                   },
               }
           }
       }
       None
   }
}

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
    // TODO: If we wanted to, we could also try the whole thing on the top 16 MiB again (I think it would be better to have the user just construct two different Efs instances in that case)
    pub fn embedded_firmware_structure(&self, processor_generation: Option<ProcessorGeneration>) -> Result<Efh> {
        for position in EMBEDDED_FIRMWARE_STRUCTURE_POSITION.iter() {
            let mut xbuf: [u8; RW_BLOCK_SIZE] = [0; RW_BLOCK_SIZE];

            self.storage.read_block(*position, &mut xbuf)?;
            let item = LayoutVerified::<_, Efh>::new_from_prefix(&xbuf[..]);
            match item {
                Some((item, _)) => {
                    let item = item.into_ref();
                    // Note: only one Efh with second_gen_efs()==true allowed in entire Flash!
                    if item.signature.get() == 0x55AA55AA && item.second_gen_efs() && match processor_generation {
                        Some(x) => item.compatible_with_processor_generation(x),
                        None => true,
                    } {
                        // TODO: if (fuse_is_clear(FUSE_2ND_GEN_EFS) || check_2nd_gen_efs(offset)) check_2nd_gen_efs(offset) bit at 0x24
                        return Ok(*item);
                    }
                },
                None => {
                },
            }
        }
        Err(Error::HeaderNotFound)
    }

    pub fn psp_directories(&self, embedded_firmware_structure: &Efh) -> EfhPspIterator<T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
        EfhPspIterator {
            storage: &self.storage,
        }
    }

    pub fn bios_directories(&self, embedded_firmware_structure: &Efh) -> EfhBiosIterator<T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
        let positions = [embedded_firmware_structure.bios_directory_table_milan.get(), embedded_firmware_structure.bios_directory_tables[2].get(), embedded_firmware_structure.bios_directory_tables[1].get(), embedded_firmware_structure.bios_directory_tables[0].get()];
        EfhBiosIterator {
            storage: &self.storage,
            positions: positions,
            index_into_positions: 0,
        }
    }
}
