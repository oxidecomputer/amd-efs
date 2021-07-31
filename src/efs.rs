
use amd_flash::{FlashRead, FlashWrite, Location};
use crate::ondisk::EMBEDDED_FIRMWARE_STRUCTURE_POSITION;
use crate::ondisk::{BiosDirectoryHeader, Efh, PspDirectoryHeader};
pub use crate::ondisk::ProcessorGeneration;
use crate::types::Result;
use crate::types::Error;
use crate::ondisk::header_from_collection;
use crate::ondisk::header_from_collection_mut;

pub struct PspDirectory<'a, T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> {
    storage: &'a T,
    location: Location,
    pub header: PspDirectoryHeader,
}

impl<'a, T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> PspDirectory<'a, T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
    fn load(storage: &'a T, location: Location) -> Result<Self> {
        let mut buf: [u8; RW_BLOCK_SIZE] = [0; RW_BLOCK_SIZE];
        storage.read_block(location, &mut buf)?;
        match header_from_collection::<PspDirectoryHeader>(&buf[..]) {
            Some(header) => {
                if header.cookie == *b"$PSP" || header.cookie == *b"$PL2" {
                     Ok(Self {
                         storage,
                         location,
                         header: *header,
                     })
                } else {
                     Err(Error::Marshal)
                }
            },
            None => {
                Err(Error::Marshal)
            },
        }
    }
    fn create(storage: &'a mut T, beginning: Location, end: Location, cookie: [u8; 4]) -> Result<Self> {
        let mut buf: [u8; RW_BLOCK_SIZE] = [0xFF; RW_BLOCK_SIZE];
        match header_from_collection_mut::<PspDirectoryHeader>(&mut buf[..]) {
            Some(item) => {
                *item = PspDirectoryHeader::default();
                item.cookie = cookie;
                storage.write_block(beginning, &buf)?;
                Self::load(storage, beginning)
            }
            None => {
                Err(Error::Marshal)
            },
        }
    }
    fn beginning(&self) -> Location {
        self.location
    }
    fn end(&self) -> Location {
        self.location // FIXME
    }
}

pub struct BiosDirectory<'a, T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> {
    storage: &'a T,
    location: Location,
    pub header: BiosDirectoryHeader,
}

impl<'a, T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> BiosDirectory<'a, T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
    fn load(storage: &'a T, location: Location) -> Result<Self> {
        let mut buf: [u8; RW_BLOCK_SIZE] = [0; RW_BLOCK_SIZE];
        storage.read_block(location, &mut buf)?;
        match header_from_collection::<BiosDirectoryHeader>(&buf[..]) {
            Some(header) => {
                if header.cookie == *b"$BHD" || header.cookie == *b"$BL2" {
                     Ok(Self {
                         storage,
                         location,
                         header: *header,
                     })
                } else {
                     Err(Error::Marshal)
                }
            },
            None => {
                Err(Error::Marshal)
            },
        }
    }
    fn create(storage: &'a mut T, beginning: Location, end: Location, cookie: [u8; 4]) -> Result<Self> {
        let mut buf: [u8; RW_BLOCK_SIZE] = [0xFF; RW_BLOCK_SIZE];
        match header_from_collection_mut::<BiosDirectoryHeader>(&mut buf[..]) {
            Some(item) => {
                *item = BiosDirectoryHeader::default();
                item.cookie = cookie;
                storage.write_block(beginning, &buf)?;
                Self::load(storage, beginning)
            }
            None => {
                Err(Error::Marshal)
            },
        }
    }
    fn beginning(&self) -> Location {
        self.location
    }
    fn end(&self) -> Location {
        self.location // FIXME
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
               match BiosDirectory::load(self.storage, position) {
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
            match header_from_collection::<Efh>(&xbuf[..]) {
                Some(item) => {
                    // Note: only one Efh with second_gen_efs()==true allowed in entire Flash!
                    if item.signature.get() == 0x55AA55AA && item.second_gen_efs() && match processor_generation {
                        Some(x) => item.compatible_with_processor_generation(x),
                        None => true,
                    } {
                        return Ok(*item);
                    }
                },
                None => {
                },
            }
        }
        Err(Error::HeaderNotFound)
    }

    pub fn psp_directory(&self, embedded_firmware_structure: &Efh) -> Result<PspDirectory<T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>> {
        let mut xbuf: [u8; RW_BLOCK_SIZE] = [0; RW_BLOCK_SIZE];
        if embedded_firmware_structure.psp_directory_table_location_zen.get() == 0xffff_ffff {
            Err(Error::HeaderNotFound)
        } else {
            let directory = PspDirectory::load(&self.storage, embedded_firmware_structure.psp_directory_table_location_zen.get())?;
            if directory.header.cookie == *b"$PSP" { // level 1 PSP header should have "$PSP" cookie
                Ok(directory)
            } else {
                Err(Error::Marshal)
            }
        }
    }

    /// Returns an iterator over level 1 BIOS directories
    pub fn bios_directories(&self, embedded_firmware_structure: &Efh) -> Result<EfhBiosIterator<T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>> {
        let positions = [embedded_firmware_structure.bios_directory_table_milan.get(), embedded_firmware_structure.bios_directory_tables[2].get(), embedded_firmware_structure.bios_directory_tables[1].get(), embedded_firmware_structure.bios_directory_tables[0].get()];
        Ok(EfhBiosIterator {
            storage: &self.storage,
            positions: positions,
            index_into_positions: 0,
        })
    }

    // Note: BEGINNING, END are coordinates (in Byte).
    pub fn create_bios_directory(&mut self, embedded_firmware_structure: &Efh, beginning: Location, end: Location) -> Result<BiosDirectory<'_, T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>> {
        // Make sure there's no overlap
        let psp_directory = self.psp_directory(embedded_firmware_structure)?;
        let intersection_beginning = beginning.max(psp_directory.beginning());
        let intersection_end = end.min(psp_directory.end());
        if intersection_beginning < intersection_end {
            return Err(Error::Overlap);
        }
        let bios_directories = self.bios_directories(embedded_firmware_structure)?;
        for bios_directory in bios_directories {
            let intersection_beginning = beginning.max(bios_directory.beginning());
            let intersection_end = end.min(bios_directory.end());
            if intersection_beginning < intersection_end {
                return Err(Error::Overlap);
            }
        }
        let result = BiosDirectory::create(&mut self.storage, beginning, end, *b"$BHD")?;
        if embedded_firmware_structure.compatible_with_processor_generation(ProcessorGeneration::Milan) {
            // FIXME: embedded_firmware_structure.bios_directory_table_milan.set(); ensure that the others are unset?
        } else {
            // FIXME: embedded_firmware_structure.bios_directory_tables[2].set() or embedded_firmware_structure.bios_directory_tables[1].set() or embedded_firmware_structure.bios_directory_tables[0].set()
        }
        Ok(result)
    }

    // Note: BEGINNING, END are coordinates (in Byte).
    pub fn create_psp_directory(&mut self, embedded_firmware_structure: &Efh, beginning: Location, end: Location) -> Result<PspDirectory<'_, T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>> {
        // Make sure there's no overlap
        match self.psp_directory(embedded_firmware_structure) {
            Err(Error::HeaderNotFound) => {
            },
            Err(e) => {
                return Err(e);
            }
            Ok(_) => {
                // FIXME: Create level 2 PSP Directory
                return Err(Error::Duplicate);
            }
        }
        let bios_directories = self.bios_directories(embedded_firmware_structure)?;
        for bios_directory in bios_directories {
            let intersection_beginning = beginning.max(bios_directory.beginning());
            let intersection_end = end.min(bios_directory.end());
            if intersection_beginning < intersection_end {
                return Err(Error::Overlap);
            }
        }
        let result = PspDirectory::create(&mut self.storage, beginning, end, *b"$PSP")?;
        // FIXME: embedded_firmware_structure.psp_directory_table_location_zen.set(); and self.storage.write_block(right location, efh)
        Ok(result)
    }
}
