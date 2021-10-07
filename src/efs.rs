use amd_flash::{FlashRead, FlashWrite, Location};
use crate::ondisk::EMBEDDED_FIRMWARE_STRUCTURE_POSITION;
use crate::ondisk::{BiosDirectoryHeader, Efh, PspDirectoryHeader, PspDirectoryEntry, PspDirectoryEntryAttrs, BiosDirectoryEntry, BiosDirectoryEntryAttrs, BiosDirectoryEntryType, PspDirectoryEntryType, DirectoryAdditionalInfo, AddressMode, DirectoryHeader, DirectoryEntry};
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

pub struct DirectoryIter<'a, Item, T: FlashRead<ERASURE_BLOCK_SIZE>, const ERASURE_BLOCK_SIZE: usize> {
    storage: &'a T,
    beginning: Location, // current item (entry)
    end: Location,
    total_entries: u32,
    index: u32,
    _item: PhantomData<Item>,
}

impl<'a, Item: FromBytes + Copy, T: FlashRead<ERASURE_BLOCK_SIZE>, const ERASURE_BLOCK_SIZE: usize> Iterator for DirectoryIter<'a, Item, T, ERASURE_BLOCK_SIZE> {
    type Item = Item;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.index < self.total_entries {
            let mut buf: [u8; ERASURE_BLOCK_SIZE] = [0xff; ERASURE_BLOCK_SIZE];
            let mut buf = &mut buf[..size_of::<Item>()];
            self.storage.read_exact(self.beginning, buf).ok()?;
            // FIXME: range check so we don't fall off the end!
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
pub struct Directory<'a, MainHeader, Item: FromBytes, T: FlashRead<ERASURE_BLOCK_SIZE> + FlashWrite<ERASURE_BLOCK_SIZE>, Attrs: Sized, const _SPI_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> {
    storage: &'a T,
    location: Location,
    pub header: MainHeader, // FIXME: make read-only
    directory_headers_size: u32,
    _attrs: PhantomData<Attrs>,
    _item: PhantomData<Item>,
}

impl<'a, MainHeader: Copy + DirectoryHeader + FromBytes + AsBytes + Default, Item: Copy + FromBytes + AsBytes + DirectoryEntry, T: 'a + FlashRead<ERASURE_BLOCK_SIZE> + FlashWrite<ERASURE_BLOCK_SIZE>, Attrs: Sized, const SPI_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> Directory<'a, MainHeader, Item, T, Attrs, SPI_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
    const SPI_BLOCK_SIZE: usize = SPI_BLOCK_SIZE;
    const MAX_DIRECTORY_HEADERS_SIZE: u32 = SPI_BLOCK_SIZE as u32; // AMD says 0x400; but good luck with modifying the first entry payload then.
    const MAX_DIRECTORY_ENTRIES: usize = ((Self::MAX_DIRECTORY_HEADERS_SIZE as usize) - size_of::<MainHeader>()) / size_of::<Item>();

    fn minimal_directory_headers_size(total_entries: u32) -> Result<u32> {
        Ok(size_of::<MainHeader>().checked_add(size_of::<Item>().checked_mul(total_entries as usize).ok_or(Error::DirectoryRangeCheck)?).ok_or(Error::DirectoryRangeCheck)?.try_into().map_err(|_| Error::DirectoryRangeCheck)?)
    }

    /// Note: Caller has to check whether it is the right cookie!
    fn load(storage: &'a T, location: Location) -> Result<Self> {
        let mut buf: [u8; ERASURE_BLOCK_SIZE] = [0xff; ERASURE_BLOCK_SIZE];
        storage.read_exact(location, &mut buf)?;
        match header_from_collection::<MainHeader>(&buf[..size_of::<MainHeader>()]) {
            Some(header) => {
                let cookie = header.cookie();
                if cookie == *b"$PSP" || cookie == *b"$PL2" || cookie == *b"$BHD" || cookie == *b"$BL2" {
                    let contents_base = DirectoryAdditionalInfo::try_from_unit(header.additional_info().base_address()).unwrap();
                    Ok(Self {
                        storage,
                        location,
                        header: *header,
                        directory_headers_size: if contents_base == 0 {
                            // Note: This means the number of entries cannot be changed (without moving ALL the payload--which we don't want).
                            Self::minimal_directory_headers_size(header.total_entries())?
                        } else {
                            // Note: This means the number of entries can be changed even when payload is already present.
                            // TODO: This is maybe still bad since we are only guaranteed 0x400 B of space, which is less than the following:
                            Self::MAX_DIRECTORY_HEADERS_SIZE
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
    }
    fn create(storage: &'a mut T, beginning: Location, end: Location, cookie: [u8; 4]) -> Result<Self> {
        let mut buf: [u8; ERASURE_BLOCK_SIZE] = [0xFF; ERASURE_BLOCK_SIZE];
        match header_from_collection_mut::<MainHeader>(&mut buf[..]) {
            Some(item) => {
                *item = MainHeader::default();
                item.set_cookie(cookie);
                // Note: It is valid that ERASURE_BLOCK_SIZE <= SPI_BLOCK_SIZE.
                if Self::SPI_BLOCK_SIZE % ERASURE_BLOCK_SIZE != 0 {
                    return Err(Error::DirectoryRangeCheck);
                }
                let additional_info = DirectoryAdditionalInfo::new()
                  .with_max_size_checked(DirectoryAdditionalInfo::try_into_unit((end - beginning).try_into().map_err(|_| Error::DirectoryRangeCheck)?).ok_or_else(|| Error::DirectoryRangeCheck)?).map_err(|_| Error::DirectoryRangeCheck)?
                  .with_spi_block_size_checked(DirectoryAdditionalInfo::try_into_unit(Self::SPI_BLOCK_SIZE).ok_or_else(|| Error::DirectoryRangeCheck)?.try_into().map_err(|_| Error::DirectoryRangeCheck)?).map_err(|_| Error::DirectoryRangeCheck)?
                  // We put the actual payload at some distance from the directory, but still close-by--in order to be able to grow the directory later (when there's already payload)
                  .with_base_address(DirectoryAdditionalInfo::try_into_unit((beginning.checked_add(Self::MAX_DIRECTORY_HEADERS_SIZE).ok_or_else(|| Error::DirectoryRangeCheck)?).try_into().map_err(|_| Error::DirectoryRangeCheck)?).ok_or_else(|| Error::DirectoryRangeCheck)?.try_into().map_err(|_| Error::DirectoryRangeCheck)?)
                  .with_address_mode(AddressMode::EfsRelativeOffset);
                item.set_additional_info(additional_info);
                storage.erase_and_write_block(beginning, &buf)?;
                Self::load(storage, beginning)
            }
            None => {
                Err(Error::Marshal)
            },
        }
    }
    fn update_main_header_checksum(&mut self) -> Result<()> {
        let checksum_input_skip_at_the_beginning: u32 = 8; // Fields "signature" and "checksum"
        let flash_input_block_size = Self::minimal_directory_headers_size(self.header.total_entries())?;
        let checksum_input_size = flash_input_block_size.checked_sub(checksum_input_skip_at_the_beginning).ok_or(Error::DirectoryRangeCheck)?;
        let mut flash_input_block_address = self.directory_beginning();
        let mut buf = [0xFF; ERASURE_BLOCK_SIZE];
        let mut flash_input_block_remainder = flash_input_block_size;
        let mut checksummer = AmdFletcher32::new();
        // Good luck with that: assert!(((flash_input_block_size as usize) % ERASURE_BLOCK_SIZE) == 0);
        while flash_input_block_remainder > 0 {
            self.storage.read_erasure_block(flash_input_block_address, &mut buf)?;
            let mut count = ERASURE_BLOCK_SIZE as u32;
            if count > flash_input_block_remainder {
                count = flash_input_block_remainder;
            }
            assert!(count % 2 == 0);
            let block = &buf[..count as usize].chunks(2).map(|bytes| { u16::from_le_bytes(bytes.try_into().unwrap()) });
            // TODO: Optimize performance
            block.clone().for_each(|item: u16|
                checksummer.update(&[item]));
            flash_input_block_remainder -= count;
            flash_input_block_address = flash_input_block_address.checked_add(count).ok_or(Error::DirectoryRangeCheck)?;
        }

        let checksum = checksummer.value().value();
        self.header.set_checksum(checksum);
        flash_input_block_address = self.directory_beginning();
        self.storage.read_erasure_block(flash_input_block_address, &mut buf)?;
        // Write main header--and at least the directory entries that are "in the way"
        match header_from_collection_mut::<MainHeader>(&mut buf[..size_of::<MainHeader>()]) {
            Some(item) => {
                *item = self.header;
            },
            None => {
                return Err(Error::DirectoryRangeCheck);
            },
        }
        self.storage.erase_and_write_block(flash_input_block_address, &buf)?;
        Ok(())
    }
    fn directory_beginning(&self) -> Location {
        let additional_info = self.header.additional_info();
        let contents_base = DirectoryAdditionalInfo::try_from_unit(additional_info.base_address()).unwrap();
        self.location + size_of::<MainHeader>() as Location // FIXME: range check
    }
    /// This will return whatever space is allocated--whether it's in use or not!
    fn directory_end(&self) -> Location {
        let headers_size = self.directory_headers_size;
        assert!((headers_size as usize) >= size_of::<MainHeader>());
        self.location + headers_size // FIXME: range check
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
        // Assumption: SIZE includes the size of the main directory header.
        // FIXME: What happens in the case contents_base != 0 ?  I think then it doesn't include it.
        self.contents_beginning() + size - self.directory_headers_size // FIXME: range check
    }
    pub fn entries(&self) -> DirectoryIter<Item, T, ERASURE_BLOCK_SIZE> {
        let additional_info = self.header.additional_info();

        DirectoryIter::<Item, T, ERASURE_BLOCK_SIZE> {
            storage: self.storage,
            beginning: self.directory_beginning(),
            end: self.directory_end(), // actually, much earlier--when total_entries is over.
            total_entries: self.header.total_entries(),
            index: 0u32,
            _item: PhantomData,
        }
    }

    pub(crate) fn find_payload_empty_slot(&self, size: u32) -> Result<(Location, Location)> {
        let mut entries = self.entries();
        let contents_beginning = self.contents_beginning() as u64;
        let contents_end = self.contents_end() as u64;
        let mut frontier: u64 = self.contents_beginning().into();
        for ref entry in entries {
            let size = match entry.size() {
                None => continue,
                Some(x) => x as u64
            };
            match entry.source() {
                ValueOrLocation::Location(x) => {
                    if x >= contents_beginning && x + size <= contents_end {
                        frontier = x + size; // FIXME bounds check
                    }
                },
                _ => {
                },
            }
        }
        let frontier: Location = frontier.try_into().map_err(|_| Error::DirectoryPayloadRangeCheck)?;
        let frontier_end = frontier.checked_add(size).ok_or(Error::DirectoryPayloadRangeCheck)?;
        let (beginning, end) = T::grow_to_erasure_block(frontier, frontier_end);
        if (end as u64) <= contents_end {
            Ok((beginning, end))
        } else {
            Err(Error::DirectoryPayloadRangeCheck)
        }
    }

    pub(crate) fn write_directory_entry(&mut self, directory_entry_position: Location, entry: &Item) -> Result<()> {
        let mut buf: [u8; ERASURE_BLOCK_SIZE] = [0xFF; ERASURE_BLOCK_SIZE];
        let buf_index = (directory_entry_position as usize) % ERASURE_BLOCK_SIZE;
        self.storage.read_erasure_block(directory_entry_position - (buf_index as Location), &mut buf)?;
        // FIXME: what this straddles two different blocks?
        match header_from_collection_mut::<Item>(&mut buf[buf_index..buf_index + size_of::<Item>()]) {
            Some(item) => {
                *item = *entry;
                self.storage.erase_and_write_block(directory_entry_position - (buf_index as Location), &buf)?;
            },
            None => {
                return Err(Error::DirectoryRangeCheck);
            }
        }
        Ok(())
    }
    /// ENTRY: The directory entry to put.  Note that we WILL set entry.source = payload_position in the copy we save on Flash.
    /// Result: Location where to put the payload.
    pub(crate) fn add_entry(&mut self, payload_position: Option<Location>, entry: &Item) -> Result<Option<Location>> {
        let total_entries = self.header.total_entries().checked_add(1).ok_or(Error::DirectoryRangeCheck)?;
        if Self::minimal_directory_headers_size(total_entries)? <= self.directory_headers_size { // there's still space for the directory entry
            let result: Option<Location>;
            match entry.size() {
                None => {
                    self.header.set_total_entries(total_entries);
                    self.update_main_header_checksum()?;
                    result = None
                },
                Some(size) => { // has payload
                    if size == 0 {
                        result = None
                    } else {
                        let (beginning, end) = self.find_payload_empty_slot(size)?;
                        self.header.set_total_entries(total_entries); // FIXME error handling
                        result = Some(beginning)
                    }
                }
            }
            let mut entry = *entry;
            match result {
                None => {
                },
                Some(beginning) => {
                    entry.set_source(ValueOrLocation::Location(beginning.into()));
                }
            }
            self.write_directory_entry(self.directory_beginning() + Self::minimal_directory_headers_size(self.header.total_entries())?, &entry)?; // FIXME check bounds
            self.update_main_header_checksum()?;
            Ok(result)
        } else {
            Err(Error::DirectoryRangeCheck)
        }
    }

    /// Repeatedly calls GENERATE_CONTENTS, which fills it's passed buffer as much as possible, as long as the total <= SIZE.
    /// GENERATE_CONTENTS can only return a number (of u8 that are filled in BUF) smaller than the possible size if the blob is ending.
    pub(crate) fn add_payload(&mut self, payload_position: Location, size: usize, generate_contents: &mut dyn FnMut(&mut [u8]) -> Result<usize>) -> Result<()> {
        let mut buf: [u8; ERASURE_BLOCK_SIZE] = [0xFF; ERASURE_BLOCK_SIZE];
        let mut remaining_size = size;
        let mut payload_position = payload_position;
        while remaining_size > 0 {
            let count = generate_contents(&mut buf)?;
            if count == 0 {
                break;
            }
// too magical
//            if count > remaining_size {
//                count = remaining_size;
//                for i in count..ERASURE_BLOCK_SIZE {
//                    buf[i] = 0xFF;
//                }
//            }

            let end = (payload_position as usize).checked_add(count).ok_or(Error::DirectoryPayloadRangeCheck)?;
            if end >= self.contents_end() as usize {
                return Err(Error::DirectoryPayloadRangeCheck);
            }
            remaining_size = remaining_size.checked_sub(count).ok_or(Error::DirectoryPayloadRangeCheck)?;
            self.storage.erase_and_write_block(payload_position, &buf);
            payload_position = end as Location;
            if count < buf.len() {
                break;
            }
        }
        if remaining_size == 0 {
            Ok(())
        } else {
            Err(Error::DirectoryPayloadRangeCheck)
        }
    }
}

pub type PspDirectory<'a, T: FlashRead<ERASURE_BLOCK_SIZE>, const ERASURE_BLOCK_SIZE: usize> = Directory<'a, PspDirectoryHeader, PspDirectoryEntry, T, PspDirectoryEntryAttrs, 0x3000, ERASURE_BLOCK_SIZE>;
pub type BiosDirectory<'a, T: FlashRead<ERASURE_BLOCK_SIZE>, const ERASURE_BLOCK_SIZE: usize> = Directory<'a, BiosDirectoryHeader, BiosDirectoryEntry, T, BiosDirectoryEntryAttrs, 0x1000, ERASURE_BLOCK_SIZE>;

impl<'a, T: 'a + FlashRead<ERASURE_BLOCK_SIZE> + FlashWrite<ERASURE_BLOCK_SIZE>, const SPI_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> Directory<'a, PspDirectoryHeader, PspDirectoryEntry, T, PspDirectoryEntryAttrs, SPI_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
    // FIXME: Type-check
    pub fn add_value_entry(&mut self, attrs: &PspDirectoryEntryAttrs, value: u64) -> Result<()> {
        match self.add_entry(None, &PspDirectoryEntry::new_value(attrs, value))? {
            None => {
                Ok(())
            },
            _ => {
                Err(Error::PspDirectoryEntryTypeMismatch)
            },
        }
    }

    pub fn add_blob_entry(&mut self, payload_position: Option<Location>, attrs: &PspDirectoryEntryAttrs, size: u32, iterative_contents: &mut dyn FnMut(&mut [u8]) -> Result<usize>) -> Result<Location> {
        let xpayload_position = self.add_entry(payload_position, &PspDirectoryEntry::new_payload(attrs, size, match payload_position {
            None => 0,
            Some(x) => x
        })?)?;
        match xpayload_position {
            None => {
                Err(Error::PspDirectoryEntryTypeMismatch)
            },
            Some(pos) => {
                self.add_payload(pos, size as usize, iterative_contents)?;
                Ok(pos)
            },
        }
    }
}

impl<'a, T: 'a + FlashRead<ERASURE_BLOCK_SIZE> + FlashWrite<ERASURE_BLOCK_SIZE>, const SPI_BLOCK_SIZE: usize, const ERASURE_BLOCK_SIZE: usize> Directory<'a, BiosDirectoryHeader, BiosDirectoryEntry, T, BiosDirectoryEntryAttrs, SPI_BLOCK_SIZE, ERASURE_BLOCK_SIZE> {
    pub(crate) fn add_entry_with_destination(&mut self, payload_position: Option<Location>, attrs: &BiosDirectoryEntryAttrs, size: u32, destination_location: u64) -> Result<Option<Location>> {
        self.add_entry(payload_position, &BiosDirectoryEntry::new_payload(attrs, size, 0, Some(destination_location))?)
    }

    pub fn add_apob_entry(&mut self, payload_position: Option<Location>, type_: BiosDirectoryEntryType, ram_destination_address: u64) -> Result<()> {
        let attrs = BiosDirectoryEntryAttrs::new().with_type_(type_);
        match self.add_entry_with_destination(payload_position, &attrs, 0, ram_destination_address)? {
            None => {
                Ok(())
            },
            _ => {
                Err(Error::BiosDirectoryEntryTypeMismatch)
            }
        }
    }

    pub fn add_blob_entry(&mut self, payload_position: Option<Location>, attrs: &BiosDirectoryEntryAttrs, size: u32, destination_location: Option<u64>, iterative_contents: &mut dyn FnMut(&mut [u8]) -> Result<usize>) -> Result<Location> {
        let xpayload_position = self.add_entry(payload_position, &BiosDirectoryEntry::new_payload(attrs, size, match payload_position {
            None => 0,
            Some(x) => x
        }, destination_location)?)?;
        match xpayload_position {
            None => {
                Err(Error::PspDirectoryEntryTypeMismatch)
            },
            Some(pos) => {
                self.add_payload(pos, size as usize, iterative_contents)?;
                Ok(pos)
            },
        }
    }
}

pub struct EfhBiosIterator<'a, T: FlashRead<ERASURE_BLOCK_SIZE> + FlashWrite<ERASURE_BLOCK_SIZE>, const ERASURE_BLOCK_SIZE: usize> {
    storage: &'a T,
    positions: [u32; 4], // 0xffff_ffff: invalid
    index_into_positions: usize,
}

impl<'a, T: FlashRead<ERASURE_BLOCK_SIZE> + FlashWrite<ERASURE_BLOCK_SIZE>, const ERASURE_BLOCK_SIZE: usize> Iterator for EfhBiosIterator<'a, T, ERASURE_BLOCK_SIZE> {
   type Item = BiosDirectory<'a, T, ERASURE_BLOCK_SIZE>;
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
pub struct Efs<T: FlashRead<ERASURE_BLOCK_SIZE> + FlashWrite<ERASURE_BLOCK_SIZE>, const ERASURE_BLOCK_SIZE: usize> {
    storage: T,
    efh_beginning: u32,
    efh: Efh,
}

impl<T: FlashRead<ERASURE_BLOCK_SIZE> + FlashWrite<ERASURE_BLOCK_SIZE>, const ERASURE_BLOCK_SIZE: usize> Efs<T, ERASURE_BLOCK_SIZE> {
    // TODO: If we wanted to, we could also try the whole thing on the top 16 MiB again (I think it would be better to have the user just construct two different Efs instances in that case)
    pub(crate) fn embedded_firmware_header_beginning(storage: &T, processor_generation: Option<ProcessorGeneration>) -> Result<u32> {
        for position in EMBEDDED_FIRMWARE_STRUCTURE_POSITION.iter() {
            let mut xbuf: [u8; ERASURE_BLOCK_SIZE] = [0; ERASURE_BLOCK_SIZE];
            storage.read_exact(*position, &mut xbuf)?;
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
            let mut xbuf: [u8; ERASURE_BLOCK_SIZE] = [0; ERASURE_BLOCK_SIZE];
            storage.read_exact(*position, &mut xbuf)?;
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
        let mut xbuf: [u8; ERASURE_BLOCK_SIZE] = [0; ERASURE_BLOCK_SIZE];
        storage.read_exact(efh_beginning, &mut xbuf)?;
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
        let mut buf: [u8; ERASURE_BLOCK_SIZE] = [0xFF; ERASURE_BLOCK_SIZE];
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

        storage.erase_and_write_block(0x20_000, &buf)?;
        Self::load(storage, Some(processor_generation))
    }

    pub fn psp_directory(&self) -> Result<PspDirectory<T, ERASURE_BLOCK_SIZE>> {
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

    pub fn secondary_psp_directory(&self) -> Result<PspDirectory<T, ERASURE_BLOCK_SIZE>> {
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
    pub fn bios_directories(&self) -> Result<EfhBiosIterator<T, ERASURE_BLOCK_SIZE>> {
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
        // FIXME: check EFH no-overlap!
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
        let mut buf: [u8; ERASURE_BLOCK_SIZE] = [0xFF; ERASURE_BLOCK_SIZE];
        match header_from_collection_mut(&mut buf[..]) {
            Some(item) => {
                *item = self.efh;
            }
            None => {
            },
        }

        self.storage.erase_and_write_block(self.efh_beginning, &buf)?;
        Ok(())
    }

    /// Note: BEGINNING, END are coordinates (in Byte).
    /// Note: We always create the directory and the contents adjacent without gap.
    pub fn create_bios_directory(&mut self, beginning: Location, end: Location) -> Result<BiosDirectory<'_, T, ERASURE_BLOCK_SIZE>> {
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
    pub fn create_psp_directory(&mut self, beginning: Location, end: Location) -> Result<PspDirectory<'_, T, ERASURE_BLOCK_SIZE>> {
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
