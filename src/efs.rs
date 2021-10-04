use amd_flash::{FlashRead, FlashWrite, Location};
use crate::ondisk::EMBEDDED_FIRMWARE_STRUCTURE_POSITION;
use crate::ondisk::{BiosDirectoryHeader, Efh, PspDirectoryHeader, PspDirectoryEntry, PspDirectoryEntryAttrs, BiosDirectoryEntry, BiosDirectoryEntryAttrs, PspDirectoryEntryType, DirectoryAdditionalInfo, AddressMode, DirectoryHeader};
pub use crate::ondisk::ProcessorGeneration;
use crate::types::Result;
use crate::types::Error;
use crate::types::ValueOrLocation;
use crate::ondisk::header_from_collection;
use crate::ondisk::header_from_collection_mut;
use core::mem::size_of;
use core::convert::TryInto;
use core::marker::PhantomData;
use crate::amdfletcher32::AmdFletcher32;
use zerocopy::FromBytes;
use zerocopy::AsBytes;

pub struct DirectoryIter<'a, Item, T: FlashRead<RW_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize> {
    storage: &'a T,
    beginning: Location, // current item (entry)
    end: Location,
    total_entries: u32,
    index: u32,
    _item: PhantomData<Item>,
}

impl<'a, Item: FromBytes + Copy, T: FlashRead<RW_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize> Iterator for DirectoryIter<'a, Item, T, RW_BLOCK_SIZE> {
    type Item = Item;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.index < self.total_entries {
            // This is very inefficient reading!
            let mut buf: [u8; RW_BLOCK_SIZE] = [0; RW_BLOCK_SIZE];
            // FIXME: range check so we don't fall off the end!
            let rw_block_size = RW_BLOCK_SIZE as u32;
            self.storage.read_block(self.beginning - (self.beginning % rw_block_size), &mut buf).ok()?;
            let beginning = self.beginning as usize;
            let end = self.end as usize;
            let buf = &buf[(beginning % RW_BLOCK_SIZE) .. (beginning % RW_BLOCK_SIZE) + size_of::<Item>()];
            let result = header_from_collection::<Item>(buf)?; // TODO: Check for errors
            self.beginning += size_of::<Item>() as u32; // FIXME: range check
            self.index += 1;
            Some(*result)
        } else {
            None
        }
    }
}

// TODO: split into Directory and DirectoryContents (disjunct) if requested in additional_info.
pub struct Directory<'a, MainHeader, Item: FromBytes, T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, Attrs: Sized, const _SPI_BLOCK_SIZE: usize, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> {
    storage: &'a T,
    location: Location,
    pub header: MainHeader, // FIXME: make read-only
    directory_headers_size: u32,
    _attrs: PhantomData<Attrs>,
    _item: PhantomData<Item>,
}

impl<'a, MainHeader: Copy + DirectoryHeader + FromBytes + AsBytes + Default, Item: FromBytes + AsBytes, T: 'a + FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, Attrs: Sized, const SPI_BLOCK_SIZE: usize, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> Directory<'a, MainHeader, Item, T, Attrs, SPI_BLOCK_SIZE, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
    const SPI_BLOCK_SIZE: usize = SPI_BLOCK_SIZE;
    const MAX_DIRECTORY_HEADERS_SIZE: usize = 0x400;
    const MAX_DIRECTORY_ENTRIES: usize = (Self::MAX_DIRECTORY_HEADERS_SIZE - size_of::<MainHeader>()) / size_of::<Item>();
    /// Note: Caller has to check whether it is the right cookie!
    fn load(storage: &'a T, location: Location) -> Result<Self> {
        let mut buf: [u8; RW_BLOCK_SIZE] = [0; RW_BLOCK_SIZE];
        storage.read_block(location, &mut buf)?;
        match header_from_collection::<MainHeader>(&buf[..]) {
            Some(header) => {
                let cookie = header.cookie();
                if cookie == *b"$PSP" || cookie == *b"$PL2" || cookie == *b"$BHD" || cookie == *b"$BL2" {
                    let contents_base = DirectoryAdditionalInfo::try_from_unit(header.additional_info().base_address()).unwrap();
                    Ok(Self {
                        storage,
                        location,
                        header: *header,
                        directory_headers_size: if contents_base == 0 {
                            size_of::<MainHeader>() as u32 + size_of::<Item>() as u32 * header.total_entries() // FIXME: Range check
                        } else {
                            Self::MAX_DIRECTORY_HEADERS_SIZE.try_into().unwrap()
                        },
                        _attrs: PhantomData,
                        _item: PhantomData,
                    })
                } else {
                    Err(Error::Marshal)
                }
            },
            None => {
                Err(Error::Marshal)
            },
        }
/*
let mut fletcher = AmdFletcher32::new();
fletcher.update([1,2,3]);
fletcher.value().value()
*/
    }
    fn create(storage: &'a mut T, beginning: Location, end: Location, cookie: [u8; 4]) -> Result<Self> {
        let mut buf: [u8; RW_BLOCK_SIZE] = [0xFF; RW_BLOCK_SIZE];
        match header_from_collection_mut::<MainHeader>(&mut buf[..]) {
            Some(item) => {
                *item = MainHeader::default();
                // FIXME: item.cookie = cookie;
                // FIXME: Erase
                // Note: It is valid that ERASURE_BLOCK_SIZE <= SPI_BLOCK_SIZE.
                if Self::SPI_BLOCK_SIZE % ERASURE_BLOCK_SIZE != 0 {
                    return Err(Error::DirectoryRangeCheck);
                }

                let additional_info = DirectoryAdditionalInfo::new()
                  .with_max_size_checked(DirectoryAdditionalInfo::try_into_unit((end - beginning).try_into().map_err(|_| Error::DirectoryRangeCheck)?).ok_or_else(|| Error::DirectoryRangeCheck)?).map_err(|_| Error::DirectoryRangeCheck)?
                  .with_spi_block_size_checked(DirectoryAdditionalInfo::try_into_unit(Self::SPI_BLOCK_SIZE).ok_or_else(|| Error::DirectoryRangeCheck)?.try_into().map_err(|_| Error::DirectoryRangeCheck)?).map_err(|_| Error::DirectoryRangeCheck)?
                  .with_base_address(0)
                  .with_address_mode(AddressMode::EfsRelativeOffset);
                item.set_additional_info(additional_info);
                storage.write_block(beginning, &buf)?;
                Self::load(storage, beginning)
            }
            None => {
                Err(Error::Marshal)
            },
        }
    }
    fn directory_beginning(&self) -> Location {
        let additional_info = self.header.additional_info();
        let contents_base = DirectoryAdditionalInfo::try_from_unit(additional_info.base_address()).unwrap();
        if contents_base == 0 { // skip Main Header
            self.location + size_of::<MainHeader>() as Location // FIXME: range check
        } else { // The main header is somewhere completely different
            self.location
        }
    }
    fn directory_end(&self) -> Location {
        let headers_size: Location = self.directory_headers_size.try_into().unwrap();
        self.location + headers_size // FIXME: range check; TODO: maybe sometimes, provide much less (see contents_beginning)
    }
    fn contents_beginning(&self) -> Location {
        let additional_info = self.header.additional_info();
        let contents_base = DirectoryAdditionalInfo::try_from_unit(additional_info.base_address()).unwrap();
        if contents_base == 0 {
            self.directory_end()
        } else {
            contents_base.try_into().unwrap()
        }
    }
    fn contents_end(&self) -> Location {
        let additional_info = self.header.additional_info();
        let size: u32 = DirectoryAdditionalInfo::try_from_unit(additional_info.max_size()).unwrap().try_into().unwrap();
        self.contents_beginning() + size // FIXME: range check
    }
    pub fn entries(&self) -> DirectoryIter<Item, T, RW_BLOCK_SIZE> {
        let additional_info = self.header.additional_info();

        DirectoryIter::<Item, T, RW_BLOCK_SIZE> {
            storage: self.storage,
            beginning: self.directory_beginning(),
            end: self.directory_end(), // actually, much earlier--when total_entries is over.
            total_entries: self.header.total_entries(),
            index: 0u32,
            _item: PhantomData,
        }
    }

    pub(crate) fn add_entry(&mut self, attrs: &Attrs, size: usize) -> Result<()> {
        // FIXME: Actually increase header.total_entries (if there's still space)
        let entry_size = size_of::<Item>();
        todo!();
    }

    /// Repeatedly calls ITERATIVE_CONTENTS, which fills BUF as much as possible.  Returns the number of u8 that are filled in BUF.
    /// It is only allowed to return a number of u8 that are filled in BUF smaller than the possible size if the blob is ending.
    pub fn add_blob_entry(&mut self, attrs: &Attrs, size: usize, iterative_contents: &mut dyn FnMut(&mut [u8]) -> Result<usize>) -> Result<()> {
        let mut buf: [u8; ERASURE_BLOCK_SIZE] = [0xFF; ERASURE_BLOCK_SIZE];
        loop {
            let count = iterative_contents(&mut buf)?;
            if count == 0 {
                break;
            }
            // FIXME: actually add BUF to the directory.
            if count < buf.len() {
                break;
            }
        }
        self.add_entry(attrs, size)
    }
}

pub type PspDirectory<'a, T: FlashRead<RW_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> = Directory<'a, PspDirectoryHeader, PspDirectoryEntry, T, PspDirectoryEntryAttrs, 0x3000, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>;
pub type BiosDirectory<'a, T: FlashRead<RW_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> = Directory<'a, BiosDirectoryHeader, BiosDirectoryEntry, T, BiosDirectoryEntryAttrs, 0x1000, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>;

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
    efh_beginning: u32,
    efh: Efh,
}

impl<T: FlashRead<RW_BLOCK_SIZE> + FlashWrite<RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>, const RW_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> Efs<T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
    // TODO: If we wanted to, we could also try the whole thing on the top 16 MiB again (I think it would be better to have the user just construct two different Efs instances in that case)
    pub(crate) fn embedded_firmware_header_beginning(storage: &T, processor_generation: Option<ProcessorGeneration>) -> Result<u32> {
        for position in EMBEDDED_FIRMWARE_STRUCTURE_POSITION.iter() {
            let mut xbuf: [u8; RW_BLOCK_SIZE] = [0; RW_BLOCK_SIZE];
            storage.read_block(*position, &mut xbuf)?;
            match header_from_collection::<Efh>(&xbuf[..]) {
                Some(item) => {
                    // Note: only one Efh with second_gen_efs()==true allowed in entire Flash!
                    if item.signature.get() == 0x55AA55AA && item.second_gen_efs() && match processor_generation {
                        Some(x) => item.compatible_with_processor_generation(x),
                        None => true,
                    } {
                        return Ok(*position);
                    }
                },
                None => {
                },
            }
        }
        // Old firmware header is better than no firmware header; TODO: Warn.
        for position in EMBEDDED_FIRMWARE_STRUCTURE_POSITION.iter() {
            let mut xbuf: [u8; RW_BLOCK_SIZE] = [0; RW_BLOCK_SIZE];
            storage.read_block(*position, &mut xbuf)?;
            match header_from_collection::<Efh>(&xbuf[..]) {
                Some(item) => {
                    if item.signature.get() == 0x55AA55AA && !item.second_gen_efs() && match processor_generation {
                        //Some(x) => item.compatible_with_processor_generation(x),
                        None => true,
                        _ => false,
                    } {
                        return Ok(*position);
                    }
                },
                None => {
                },
            }
        }
        Err(Error::EfsHeaderNotFound)
    }

    pub fn load(storage: T, processor_generation: Option<ProcessorGeneration>) -> Result<Self> {
        let efh_beginning = Self::embedded_firmware_header_beginning(&storage, processor_generation)?;
        let mut xbuf: [u8; RW_BLOCK_SIZE] = [0; RW_BLOCK_SIZE];
        storage.read_block(efh_beginning, &mut xbuf)?;
        let efh = header_from_collection::<Efh>(&xbuf[..]).ok_or_else(|| Error::EfsHeaderNotFound)?;
        if efh.signature.get() != 0x55aa_55aa {
            return Err(Error::EfsHeaderNotFound);
        }

        Ok(Self {
            storage,
            efh_beginning,
            efh: *efh,
        })
    }
    pub fn create(mut storage: T, processor_generation: ProcessorGeneration) -> Result<Self> {
        let mut buf: [u8; RW_BLOCK_SIZE] = [0xFF; RW_BLOCK_SIZE];
        match header_from_collection_mut(&mut buf[..]) {
            Some(item) => {
                let mut efh: Efh = Efh::default();
                efh.second_gen_efs.set(Efh::second_gen_efs_for_processor_generation(processor_generation));
                *item = efh;
            }
            None => {
                return Err(Error::Marshal);
            },
        }

        storage.write_block(0x20_000, &buf)?;
        Self::load(storage, Some(processor_generation))
    }

    pub fn psp_directory(&self) -> Result<PspDirectory<T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>> {
        let mut xbuf: [u8; RW_BLOCK_SIZE] = [0; RW_BLOCK_SIZE];
        let psp_directory_table_location = self.efh.psp_directory_table_location_zen.get();
        if psp_directory_table_location == 0xffff_ffff {
            Err(Error::PspDirectoryHeaderNotFound)
        } else {
            let directory = match PspDirectory::load(&self.storage, psp_directory_table_location) {
                Ok(directory) => {
                    if directory.header.cookie == *b"$PSP" { // level 1 PSP header should have "$PSP" cookie
                        return Ok(directory);
                    }
                },
                Err(Error::Marshal) => {
                },
                Err(e) => {
                    return Err(e);
                },
            };

            // That's the same fallback AMD does on Naples:

            let psp_directory_table_location = {
                let addr = self.efh.psp_directory_table_location_naples.get();
                if addr == 0xffff_ffff {
                    addr
                } else {
                    addr & 0x00ff_ffff
                }
            };
            if psp_directory_table_location == 0xffff_ffff || psp_directory_table_location == 0 {
                Err(Error::PspDirectoryHeaderNotFound)
            } else {
                let directory = PspDirectory::load(&self.storage, psp_directory_table_location)?;
                if directory.header.cookie == *b"$PSP" { // level 1 PSP header should have "$PSP" cookie
                    Ok(directory)
                } else {
                    Err(Error::Marshal)
                }
            }
        }
    }

    pub fn secondary_psp_directory(&self) -> Result<PspDirectory<T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>> {
        let main_directory = self.psp_directory()?;
        for entry in main_directory.entries() {
            if entry.type_() == PspDirectoryEntryType::SecondLevelDirectory {
                match entry.source() {
                    ValueOrLocation::Location(psp_directory_table_location) => {
                        if psp_directory_table_location >= 0x1_0000_0000 {
                            return Err(Error::PspDirectoryEntryTypeMismatch)
                        } else {
                            let psp_directory_table_location = psp_directory_table_location as u32;
                            let directory = PspDirectory::load(&self.storage, psp_directory_table_location)?;
                            return Ok(directory);
                        }
                    },
                    _ => {
                        return Err(Error::PspDirectoryEntryTypeMismatch)
                    }
                }
            }
        }
        Err(Error::PspDirectoryHeaderNotFound)
    }

    /// Returns an iterator over level 1 BIOS directories
    pub fn bios_directories(&self) -> Result<EfhBiosIterator<T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>> {
        let embedded_firmware_structure = &self.efh;
        let positions = [embedded_firmware_structure.bios_directory_table_milan.get(), embedded_firmware_structure.bios_directory_tables[2].get() & 0x00ff_ffff, embedded_firmware_structure.bios_directory_tables[1].get() & 0x00ff_ffff, embedded_firmware_structure.bios_directory_tables[0].get() & 0x00ff_ffff]; // the latter are physical addresses
        Ok(EfhBiosIterator {
            storage: &self.storage,
            positions: positions,
            index_into_positions: 0,
        })
    }

    // Make sure there's no overlap (even when rounded to entire erasure blocks)
    fn ensure_no_overlap(&self, beginning: Location, end: Location) -> Result<()> {
        let (beginning, end) = T::grow_to_erasure_block(beginning, end);
        match self.psp_directory() {
            Ok(psp_directory) => {
                let (reference_beginning, reference_end) = T::grow_to_erasure_block(psp_directory.directory_beginning(), psp_directory.directory_end());
                let intersection_beginning = beginning.max(reference_beginning);
                let intersection_end = end.min(reference_end);
                if intersection_beginning < intersection_end {
                    return Err(Error::Overlap);
                }
                let (reference_beginning, reference_end) = T::grow_to_erasure_block(psp_directory.contents_beginning(), psp_directory.contents_end());
                let intersection_beginning = beginning.max(reference_beginning);
                let intersection_end = end.min(reference_end);
                if intersection_beginning < intersection_end {
                    return Err(Error::Overlap);
                }
            },
            Err(Error::PspDirectoryHeaderNotFound) => {
            },
            Err(e) => {
                return Err(e);
            },
        }
        let bios_directories = self.bios_directories()?;
        for bios_directory in bios_directories {
            let (reference_beginning, reference_end) = T::grow_to_erasure_block(bios_directory.directory_beginning(), bios_directory.directory_end());
            let intersection_beginning = beginning.max(reference_beginning);
            let intersection_end = end.min(reference_end);
            if intersection_beginning < intersection_end {
                return Err(Error::Overlap);
            }
            let (reference_beginning, reference_end) = T::grow_to_erasure_block(bios_directory.contents_beginning(), bios_directory.contents_end());
            let intersection_beginning = beginning.max(reference_beginning);
            let intersection_end = end.min(reference_end);
            if intersection_beginning < intersection_end {
                return Err(Error::Overlap);
            }
        }
        Ok(())
    }

    fn write_efh(&mut self) -> Result<()> {
        let mut buf: [u8; RW_BLOCK_SIZE] = [0xFF; RW_BLOCK_SIZE];
        match header_from_collection_mut(&mut buf[..]) {
            Some(item) => {
                *item = self.efh;
            }
            None => {
            },
        }

        self.storage.write_block(self.efh_beginning, &buf)?;
        Ok(())
    }

    /// Note: BEGINNING, END are coordinates (in Byte).
    /// Note: We always create the directory and the contents adjacent without gap.
    pub fn create_bios_directory(&mut self, beginning: Location, end: Location) -> Result<BiosDirectory<'_, T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>> {
        if T::grow_to_erasure_block(beginning, end) != (beginning, end) {
            return Err(Error::Misaligned);
        }
        match self.bios_directories() {
            Ok(items) => {
                for directory in items {
                    // TODO: Ensure that we don't have too many similar ones
                }
            },
            Err(e) => {
                return Err(e);
            },
        }
        self.ensure_no_overlap(beginning, end)?;
        if self.efh.compatible_with_processor_generation(ProcessorGeneration::Milan) {
            self.efh.bios_directory_table_milan.set(beginning);
            // FIXME: ensure that the others are unset?
        } else {
            self.efh.bios_directory_tables[2].set(beginning);
            // FIXME: ensure that the others are unset?
        }
        self.write_efh()?;
        let result = BiosDirectory::create(&mut self.storage, beginning, end, *b"$BHD")?;
        Ok(result)
    }

    // Note: BEGINNING, END are coordinates (in Byte).
    pub fn create_psp_directory(&mut self, beginning: Location, end: Location) -> Result<PspDirectory<'_, T, RW_BLOCK_SIZE, ERASURE_BLOCK_SIZE>> {
        if T::grow_to_erasure_block(beginning, end) != (beginning, end) {
            return Err(Error::Misaligned);
        }
        match self.psp_directory() {
            Err(Error::PspDirectoryHeaderNotFound) => {
            },
            Err(e) => {
                return Err(e);
            }
            Ok(_) => {
                // FIXME: Create level 2 PSP Directory
                return Err(Error::Duplicate);
            }
        }
        self.ensure_no_overlap(beginning, end)?;
        // TODO: Boards older than Rome have 0xff at the top bits.  Depends on address_mode maybe.  Then, also psp_directory_table_location_naples should be set, instead.
        self.efh.psp_directory_table_location_zen.set(beginning);
        self.write_efh()?;
        let result = PspDirectory::create(&mut self.storage, beginning, end, *b"$PSP")?;
        Ok(result)
    }
}
