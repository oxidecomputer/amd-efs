use amd_flash::{FlashRead, FlashWrite, Location, ErasableLocation};
use crate::ondisk::EMBEDDED_FIRMWARE_STRUCTURE_POSITION;
use crate::ondisk::{BhdDirectoryHeader, Efh, PspDirectoryHeader, PspDirectoryEntry, PspDirectoryEntryAttrs, BhdDirectoryEntry, BhdDirectoryEntryAttrs, BhdDirectoryEntryType, PspDirectoryEntryType, DirectoryAdditionalInfo, AddressMode, DirectoryHeader, DirectoryEntry};
pub use crate::ondisk::ProcessorGeneration;
use crate::types::Result;
use crate::types::Error;
use crate::types::ValueOrLocation;
use crate::ondisk::header_from_collection;
use crate::ondisk::header_from_collection_mut;
use core::mem::size_of;
use core::convert::TryFrom;
use core::convert::TryInto;
use core::marker::PhantomData;
use crate::amdfletcher32::AmdFletcher32;
use zerocopy::FromBytes;
use zerocopy::AsBytes;

pub struct DirectoryIter<'a, Item, T: FlashRead<ERASABLE_BLOCK_SIZE>, const ERASABLE_BLOCK_SIZE: usize> {
    storage: &'a T,
    beginning: Location, // current item (directory entry)
    end: Location,
    total_entries: u32,
    index: u32,
    _item: PhantomData<Item>,
}

impl<'a, Item: FromBytes + Copy, T: FlashRead<ERASABLE_BLOCK_SIZE>, const ERASABLE_BLOCK_SIZE: usize> Iterator for DirectoryIter<'a, Item, T, ERASABLE_BLOCK_SIZE> {
    type Item = Item;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.index < self.total_entries {
            let mut buf: [u8; ERASABLE_BLOCK_SIZE] = [0xff; ERASABLE_BLOCK_SIZE];
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
pub struct Directory<'a, MainHeader, Item: FromBytes, T: FlashRead<ERASABLE_BLOCK_SIZE> + FlashWrite<ERASABLE_BLOCK_SIZE>, Attrs: Sized, const _SPI_BLOCK_SIZE: usize, const ERASABLE_BLOCK_SIZE: usize> {
    storage: &'a T,
    location: Location, // ideally ErasableLocation<ERASABLE_BLOCK_SIZE>--but that's impossible with AMD-generated images.
    pub header: MainHeader, // FIXME: make read-only
    directory_headers_size: u32,
    _attrs: PhantomData<Attrs>,
    _item: PhantomData<Item>,
}

impl<'a, MainHeader: Copy + DirectoryHeader + FromBytes + AsBytes + Default, Item: Copy + FromBytes + AsBytes + DirectoryEntry + core::fmt::Debug, T: 'a + FlashRead<ERASABLE_BLOCK_SIZE> + FlashWrite<ERASABLE_BLOCK_SIZE>, Attrs: Sized, const SPI_BLOCK_SIZE: usize, const ERASABLE_BLOCK_SIZE: usize> Directory<'a, MainHeader, Item, T, Attrs, SPI_BLOCK_SIZE, ERASABLE_BLOCK_SIZE> {
    const SPI_BLOCK_SIZE: usize = SPI_BLOCK_SIZE;
    const MAX_DIRECTORY_HEADERS_SIZE: u32 = SPI_BLOCK_SIZE as u32; // AMD says 0x400; but then good luck with modifying the first entry payload without clobbering the directory that comes right before it.
    const MAX_DIRECTORY_ENTRIES: usize = ((Self::MAX_DIRECTORY_HEADERS_SIZE as usize) - size_of::<MainHeader>()) / size_of::<Item>();

    fn minimal_directory_headers_size(total_entries: u32) -> Result<u32> {
        Ok(size_of::<MainHeader>().checked_add(size_of::<Item>().checked_mul(total_entries as usize).ok_or(Error::DirectoryRangeCheck)?).ok_or(Error::DirectoryRangeCheck)?.try_into().map_err(|_| Error::DirectoryRangeCheck)?)
    }

    /// Note: Caller has to check whether it is the right cookie!
    fn load(storage: &'a T, location: Location) -> Result<Self> {
        let mut buf: [u8; ERASABLE_BLOCK_SIZE] = [0xff; ERASABLE_BLOCK_SIZE];
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
    fn create(storage: &'a mut T, beginning: ErasableLocation<ERASABLE_BLOCK_SIZE>, end: ErasableLocation<ERASABLE_BLOCK_SIZE>, cookie: [u8; 4]) -> Result<Self> {
        let mut buf: [u8; ERASABLE_BLOCK_SIZE] = [0xFF; ERASABLE_BLOCK_SIZE];
        match header_from_collection_mut::<MainHeader>(&mut buf[..]) {
            Some(item) => {
                *item = MainHeader::default();
                item.set_cookie(cookie);
                // Note: It is valid that ERASABLE_BLOCK_SIZE <= SPI_BLOCK_SIZE.
                if Self::SPI_BLOCK_SIZE % ERASABLE_BLOCK_SIZE != 0 {
                    return Err(Error::DirectoryRangeCheck);
                }
                let additional_info = DirectoryAdditionalInfo::new()
                  .with_max_size_checked(DirectoryAdditionalInfo::try_into_unit(ErasableLocation::<ERASABLE_BLOCK_SIZE>::extent(beginning, end).try_into().map_err(|_| Error::DirectoryRangeCheck)?).ok_or_else(|| Error::DirectoryRangeCheck)?).map_err(|_| Error::DirectoryRangeCheck)?
                  .with_spi_block_size_checked(DirectoryAdditionalInfo::try_into_unit(Self::SPI_BLOCK_SIZE).ok_or_else(|| Error::DirectoryRangeCheck)?.try_into().map_err(|_| Error::DirectoryRangeCheck)?).map_err(|_| Error::DirectoryRangeCheck)?
                  // We put the actual payload at some distance from the directory, but still close-by--in order to be able to grow the directory later (when there's already payload)
                  .with_base_address(DirectoryAdditionalInfo::try_into_unit((Location::from(beginning).checked_add(Self::MAX_DIRECTORY_HEADERS_SIZE).ok_or_else(|| Error::DirectoryRangeCheck)?).try_into().map_err(|_| Error::DirectoryRangeCheck)?).ok_or_else(|| Error::DirectoryRangeCheck)?.try_into().map_err(|_| Error::DirectoryRangeCheck)?)
                  .with_address_mode(AddressMode::EfsRelativeOffset);
                item.set_additional_info(additional_info);
                storage.erase_and_write_block(beginning, &buf)?;
                Self::load(storage, Location::from(beginning))
            }
            None => {
                Err(Error::Marshal)
            },
        }
    }
    /// Updates the main header checksum.  Also updates total_entries (in the same header) to TOTAL_ENTRIES.
    /// Precondition: Since the checksum is over the entire directory, that means that all the directory entries needs to be correct already.
    fn update_main_header(&mut self, total_entries: u32) -> Result<()> {
        let old_total_entries = self.header.total_entries();
        let flash_input_block_size = Self::minimal_directory_headers_size(total_entries)?;
        let mut flash_input_block_address: Location = self.location.into();
        let mut buf = [0xFFu8; ERASABLE_BLOCK_SIZE];
        let mut flash_input_block_remainder = flash_input_block_size;
        let mut checksummer = AmdFletcher32::new();
        // Good luck with that: assert!(((flash_input_block_size as usize) % ERASABLE_BLOCK_SIZE) == 0);
        let mut skip: usize = 12; // Skip fields "signature", "checksum" and "total_entries"
        // Note: total_entries on the flash has not been updated yet--so manually account for it.
        checksummer.update(&[(total_entries & 0xffff) as u16, (total_entries >> 16) as u16]);
        while flash_input_block_remainder > 0 {
            self.storage.read_exact(flash_input_block_address, &mut buf)?;
            let mut count = ERASABLE_BLOCK_SIZE as u32;
            if count > flash_input_block_remainder {
                count = flash_input_block_remainder;
            }
            assert!(count % 2 == 0);
            assert!(count as usize >= skip);
            let block = &buf[skip..count as usize].chunks(2).map(|bytes| { u16::from_le_bytes(bytes.try_into().unwrap()) });
            skip = 0;
            // TODO: Optimize performance
            block.clone().for_each(|item: u16|
                checksummer.update(&[item]));
            flash_input_block_remainder -= count;
            flash_input_block_address = flash_input_block_address.checked_add(count).ok_or(Error::DirectoryRangeCheck)?;
        }

        let checksum = checksummer.value().value();
        self.header.set_checksum(checksum);
        let flash_input_block_address = ErasableLocation::<ERASABLE_BLOCK_SIZE>::try_from(self.location)?;
        self.storage.read_erasable_block(flash_input_block_address, &mut buf)?;
        // Write main header--and at least the directory entries that are "in the way"
        match header_from_collection_mut::<MainHeader>(&mut buf[..size_of::<MainHeader>()]) {
            Some(item) => {
                self.header.set_total_entries(total_entries); // Note: reverted on error--see below
                *item = self.header;
            },
            None => {
                return Err(Error::DirectoryRangeCheck);
            },
        }
        match self.storage.erase_and_write_block(flash_input_block_address, &buf) {
            Ok(()) => {
                Ok(())
            },
            Err(e) => {
                self.header.set_total_entries(old_total_entries);
                Err(Error::from(e))
            },
        }
    }
    fn directory_beginning(&self) -> Location {
        let additional_info = self.header.additional_info();
        let contents_base = DirectoryAdditionalInfo::try_from_unit(additional_info.base_address()).unwrap();
        let location: Location = self.location.into();
        location + size_of::<MainHeader>() as Location // FIXME: range check
    }
    /// This will return whatever space is allocated--whether it's in use or not!
    fn directory_end(&self) -> Location {
        let headers_size = self.directory_headers_size;
        assert!((headers_size as usize) >= size_of::<MainHeader>());
        let location: Location = self.location.into();
        // Note: should be erasable (but we don't care on this side; on the other hand, see contents_beginning)
        location + headers_size // FIXME: range check
    }
    fn contents_beginning(&self) -> Location {
        let additional_info = self.header.additional_info();
        let contents_base = DirectoryAdditionalInfo::try_from_unit(additional_info.base_address()).unwrap();
        if contents_base == 0 {
            self.directory_end().try_into().unwrap()
        } else {
            let base: u32 = contents_base.try_into().unwrap();
            base
            // We'd like to do ErasableLocation::<ERASABLE_BLOCK_SIZE>::try_from(base).unwrap(), but AMD-provided images do not actually have the first payload content aligned.  So we don't.
        }
    }
    fn contents_end(&self) -> ErasableLocation<ERASABLE_BLOCK_SIZE> {
        let additional_info = self.header.additional_info();
        let size: u32 = DirectoryAdditionalInfo::try_from_unit(additional_info.max_size()).unwrap().try_into().unwrap();
        let location = Location::from(self.contents_beginning());
        // Assumption: SIZE includes the size of the main directory header.
        // FIXME: What happens in the case contents_base != 0 ?  I think then it doesn't include it.
        let end = location + size - self.directory_headers_size; // FIXME: range check
        ErasableLocation::<ERASABLE_BLOCK_SIZE>::try_from(end).unwrap()
    }
    pub fn entries(&self) -> DirectoryIter<Item, T, ERASABLE_BLOCK_SIZE> {
        let additional_info = self.header.additional_info();

        DirectoryIter::<Item, T, ERASABLE_BLOCK_SIZE> {
            storage: self.storage,
            beginning: self.directory_beginning(),
            end: self.directory_end(), // actually, much earlier--this here is the allocation, not the actual size
            total_entries: self.header.total_entries(),
            index: 0u32,
            _item: PhantomData,
        }
    }

    pub(crate) fn find_payload_empty_slot(&self, size: u32) -> Result<ErasableLocation<ERASABLE_BLOCK_SIZE>> {
        let mut entries = self.entries();
        let contents_beginning = Location::from(self.contents_beginning()) as u64;
        let contents_end = Location::from(self.contents_end()) as u64;
        let mut frontier: u64 = contents_beginning;
        // TODO: Also use gaps between entries
        for ref entry in entries {
            let size = match entry.size() {
                None => continue,
                Some(x) => x as u64
            };
            match entry.source() {
                ValueOrLocation::Location(x) => {
                    if x >= contents_beginning && x + size <= contents_end {

                        let new_frontier = x + size; // FIXME bounds check
                        if new_frontier > frontier {
                            frontier = new_frontier;
                        }
                    }
                },
                _ => {
                },
            }
        }
        let frontier: Location = frontier.try_into().map_err(|_| Error::DirectoryPayloadRangeCheck)?;
        let frontier_end = frontier.checked_add(size).ok_or(Error::DirectoryPayloadRangeCheck)?;
        let (_, frontier) = T::grow_to_erasable_block(frontier, frontier);
        Ok(frontier.try_into()?)
    }

    pub(crate) fn write_directory_entry(&mut self, directory_entry_position: Location, entry: &Item) -> Result<()> {
        let mut buf: [u8; ERASABLE_BLOCK_SIZE] = [0xFF; ERASABLE_BLOCK_SIZE];
        let buf_index = (directory_entry_position as usize) % ERASABLE_BLOCK_SIZE;
        let beginning = directory_entry_position - (buf_index as Location); // align
        let beginning = beginning.try_into().map_err(|_| Error::Misaligned)?;
        self.storage.read_erasable_block(beginning, &mut buf)?;
        // FIXME: what if this straddles two different blocks?
        match header_from_collection_mut::<Item>(&mut buf[buf_index..buf_index + size_of::<Item>()]) {
            Some(item) => {
                *item = *entry;
                self.storage.erase_and_write_block(beginning, &buf)?;
            },
            None => {
                return Err(Error::DirectoryRangeCheck);
            }
        }
        Ok(())
    }
    /// PAYLOAD_POSITION: If you have a position on the Flash that you want this fn to use, specify it.  Otherwise, one will be calculated.
    /// ENTRY: The directory entry to put.  Note that we WILL set entry.source = (maybe calculated) payload_position in the copy we save on Flash.
    /// Result: Location where to put the payload.
    pub(crate) fn add_entry(&mut self, payload_position: Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>, entry: &Item) -> Result<Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>> {
        let total_entries = self.header.total_entries().checked_add(1).ok_or(Error::DirectoryRangeCheck)?;
        if Self::minimal_directory_headers_size(total_entries)? <= self.directory_headers_size { // there's still space for the directory entry
            let result: Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>;
            match entry.size() {
                None => {
                    result = None
                },
                Some(size) => {
                    if size == 0 {
                        result = None
                    } else {
                        let beginning = self.find_payload_empty_slot(size)?;
                        result = Some(beginning)
                    }
                }
            }
            let mut entry = *entry;
            match result {
                None => {
                },
                Some(beginning) => {
                    entry.set_source(ValueOrLocation::Location(Location::from(beginning).into()));
                }
            }
            let location: Location = self.location.into();
            self.write_directory_entry(location + Self::minimal_directory_headers_size(self.header.total_entries())?, &entry)?; // FIXME check bounds
            self.update_main_header(total_entries)?;
            Ok(result)
        } else {
            Err(Error::DirectoryRangeCheck)
        }
    }

    /// Repeatedly calls GENERATE_CONTENTS, which fills it's passed buffer as much as possible, as long as the total <= SIZE.
    /// GENERATE_CONTENTS can only return a number (of u8 that are filled in BUF) smaller than the possible size if the blob is ending.
    pub(crate) fn add_payload(&mut self, payload_position: ErasableLocation<ERASABLE_BLOCK_SIZE>, size: usize, generate_contents: &mut dyn FnMut(&mut [u8]) -> Result<usize>) -> Result<()> {
        let mut buf: [u8; ERASABLE_BLOCK_SIZE] = [0xFF; ERASABLE_BLOCK_SIZE];
        let mut remaining_size = size;
        let mut payload_position = payload_position;
        let contents_end = Location::from(self.contents_end()) as usize;
        while remaining_size > 0 {
            let mut count = generate_contents(&mut buf)?;
            if count == 0 {
                break;
            }
            if count > remaining_size {
                count = remaining_size;
            }
            if count < buf.len() {
                for i in count..buf.len() {
                    buf[i] = 0xFF;
                }
            }

            let end = (Location::from(payload_position) as usize).checked_add(ERASABLE_BLOCK_SIZE).ok_or(Error::DirectoryPayloadRangeCheck)?;
            if end > contents_end as usize {
                return Err(Error::DirectoryPayloadRangeCheck);
            }
            remaining_size = remaining_size.checked_sub(count).ok_or(Error::DirectoryPayloadRangeCheck)?;
            self.storage.erase_and_write_block(payload_position, &buf)?;
            payload_position = payload_position.advance(ERASABLE_BLOCK_SIZE)?;
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

pub type PspDirectory<'a, T, const ERASABLE_BLOCK_SIZE: usize> = Directory<'a, PspDirectoryHeader, PspDirectoryEntry, T, PspDirectoryEntryAttrs, 0x3000, ERASABLE_BLOCK_SIZE>;
pub type BhdDirectory<'a, T, const ERASABLE_BLOCK_SIZE: usize> = Directory<'a, BhdDirectoryHeader, BhdDirectoryEntry, T, BhdDirectoryEntryAttrs, 0x1000, ERASABLE_BLOCK_SIZE>;

impl<'a, T: 'a + FlashRead<ERASABLE_BLOCK_SIZE> + FlashWrite<ERASABLE_BLOCK_SIZE>, const SPI_BLOCK_SIZE: usize, const ERASABLE_BLOCK_SIZE: usize> Directory<'a, PspDirectoryHeader, PspDirectoryEntry, T, PspDirectoryEntryAttrs, SPI_BLOCK_SIZE, ERASABLE_BLOCK_SIZE> {
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

    pub fn add_blob_entry(&mut self, payload_position: Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>, attrs: &PspDirectoryEntryAttrs, size: u32, iterative_contents: &mut dyn FnMut(&mut [u8]) -> Result<usize>) -> Result<ErasableLocation<ERASABLE_BLOCK_SIZE>> {
        let xpayload_position = self.add_entry(payload_position, &PspDirectoryEntry::new_payload(attrs, size, match payload_position {
            None => 0,
            Some(x) => x.into()
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

impl<'a, T: 'a + FlashRead<ERASABLE_BLOCK_SIZE> + FlashWrite<ERASABLE_BLOCK_SIZE>, const SPI_BLOCK_SIZE: usize, const ERASABLE_BLOCK_SIZE: usize> Directory<'a, BhdDirectoryHeader, BhdDirectoryEntry, T, BhdDirectoryEntryAttrs, SPI_BLOCK_SIZE, ERASABLE_BLOCK_SIZE> {
    pub(crate) fn add_entry_with_destination(&mut self, payload_position: Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>, attrs: &BhdDirectoryEntryAttrs, size: u32, destination_location: u64) -> Result<Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>> {
        self.add_entry(payload_position, &BhdDirectoryEntry::new_payload(attrs, size, 0, Some(destination_location))?)
    }

    pub fn add_apob_entry(&mut self, payload_position: Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>, type_: BhdDirectoryEntryType, ram_destination_address: u64) -> Result<()> {
        let attrs = BhdDirectoryEntryAttrs::new().with_type_(type_);
        match self.add_entry_with_destination(payload_position, &attrs, 0, ram_destination_address)? {
            None => {
                Ok(())
            },
            _ => {
                Err(Error::BhdDirectoryEntryTypeMismatch)
            }
        }
    }

    pub fn add_blob_entry(&mut self, payload_position: Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>, attrs: &BhdDirectoryEntryAttrs, size: u32, destination_location: Option<u64>, iterative_contents: &mut dyn FnMut(&mut [u8]) -> Result<usize>) -> Result<ErasableLocation<ERASABLE_BLOCK_SIZE>> {
        let xpayload_position = self.add_entry(payload_position, &BhdDirectoryEntry::new_payload(attrs, size, match payload_position {
            None => 0,
            Some(x) => x.into()
        }, destination_location)?)?;
        match xpayload_position {
            None => {
                Err(Error::BhdDirectoryEntryTypeMismatch)
            },
            Some(pos) => {
                self.add_payload(pos, size as usize, iterative_contents)?;
                Ok(pos)
            },
        }
    }
}

pub struct EfhBhdsIterator<'a, T: FlashRead<ERASABLE_BLOCK_SIZE> + FlashWrite<ERASABLE_BLOCK_SIZE>, const ERASABLE_BLOCK_SIZE: usize> {
    storage: &'a T,
    positions: [u32; 4], // 0xffff_ffff: invalid
    index_into_positions: usize,
}

impl<'a, T: FlashRead<ERASABLE_BLOCK_SIZE> + FlashWrite<ERASABLE_BLOCK_SIZE>, const ERASABLE_BLOCK_SIZE: usize> Iterator for EfhBhdsIterator<'a, T, ERASABLE_BLOCK_SIZE> {
   type Item = BhdDirectory<'a, T, ERASABLE_BLOCK_SIZE>;
   fn next(&mut self) -> Option<<Self as Iterator>::Item> {
       while self.index_into_positions < self.positions.len() {
           let position = self.positions[self.index_into_positions];
           self.index_into_positions += 1;
           if position != 0xffff_ffff && position != 0 /* sigh.  Some images have 0 as "invalid" mark */ {
               match BhdDirectory::load(self.storage, position) {
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
pub struct Efs<T: FlashRead<ERASABLE_BLOCK_SIZE> + FlashWrite<ERASABLE_BLOCK_SIZE>, const ERASABLE_BLOCK_SIZE: usize> {
    storage: T,
    efh_beginning: ErasableLocation<ERASABLE_BLOCK_SIZE>,
    efh: Efh,
}

impl<T: FlashRead<ERASABLE_BLOCK_SIZE> + FlashWrite<ERASABLE_BLOCK_SIZE>, const ERASABLE_BLOCK_SIZE: usize> Efs<T, ERASABLE_BLOCK_SIZE> {
    // TODO: If we wanted to, we could also try the whole thing on the top 16 MiB again (I think it would be better to have the user just construct two different Efs instances in that case)
    pub(crate) fn efh_beginning(storage: &T, processor_generation: Option<ProcessorGeneration>) -> Result<ErasableLocation<ERASABLE_BLOCK_SIZE>> {
        for position in EMBEDDED_FIRMWARE_STRUCTURE_POSITION.iter() {
            let mut xbuf: [u8; ERASABLE_BLOCK_SIZE] = [0; ERASABLE_BLOCK_SIZE];
            storage.read_exact(*position, &mut xbuf)?;
            match header_from_collection::<Efh>(&xbuf[..]) {
                Some(item) => {
                    // Note: only one Efh with second_gen_efs()==true allowed in entire Flash!
                    if item.signature.get() == 0x55AA55AA && item.second_gen_efs() && match processor_generation {
                        Some(x) => item.compatible_with_processor_generation(x),
                        None => true,
                    } {
                        return Ok(ErasableLocation::<ERASABLE_BLOCK_SIZE>::try_from(*position)?);
                    }
                },
                None => {
                },
            }
        }
        // Old firmware header is better than no firmware header; TODO: Warn.
        for position in EMBEDDED_FIRMWARE_STRUCTURE_POSITION.iter() {
            let mut xbuf: [u8; ERASABLE_BLOCK_SIZE] = [0; ERASABLE_BLOCK_SIZE];
            storage.read_exact(*position, &mut xbuf)?;
            match header_from_collection::<Efh>(&xbuf[..]) {
                Some(item) => {
                    if item.signature.get() == 0x55AA55AA && !item.second_gen_efs() && match processor_generation {
                        //Some(x) => item.compatible_with_processor_generation(x),
                        None => true,
                        _ => false,
                    } {
                        return Ok(ErasableLocation::<ERASABLE_BLOCK_SIZE>::try_from(*position)?);
                    }
                },
                None => {
                },
            }
        }
        Err(Error::EfsHeaderNotFound)
    }

    pub fn load(storage: T, processor_generation: Option<ProcessorGeneration>) -> Result<Self> {
        let efh_beginning = Self::efh_beginning(&storage, processor_generation)?;
        let mut xbuf: [u8; ERASABLE_BLOCK_SIZE] = [0; ERASABLE_BLOCK_SIZE];
        storage.read_erasable_block(efh_beginning, &mut xbuf)?;
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
        let mut buf: [u8; ERASABLE_BLOCK_SIZE] = [0xFF; ERASABLE_BLOCK_SIZE];
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

        storage.erase_and_write_block(ErasableLocation::<ERASABLE_BLOCK_SIZE>::try_from(0x20_000u32)?, &buf)?;
        Self::load(storage, Some(processor_generation))
    }

    pub fn psp_directory(&self) -> Result<PspDirectory<T, ERASABLE_BLOCK_SIZE>> {
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

    pub fn secondary_psp_directory(&self) -> Result<PspDirectory<T, ERASABLE_BLOCK_SIZE>> {
        let main_directory = self.psp_directory()?;
        for entry in main_directory.entries() {
            if entry.type_() == PspDirectoryEntryType::SecondLevelDirectory {
                match entry.source() {
                    ValueOrLocation::Location(psp_directory_table_location) => {
                        if psp_directory_table_location >= 0x1_0000_0000 {
                            return Err(Error::PspDirectoryEntryTypeMismatch)
                        } else {
                            let directory = PspDirectory::load(&self.storage, psp_directory_table_location.try_into().map_err(|_| Error::DirectoryRangeCheck)?)?;
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
    pub fn bhd_directories(&self) -> Result<EfhBhdsIterator<T, ERASABLE_BLOCK_SIZE>> {
        let efh = &self.efh;
        let positions = [efh.bhd_directory_table_milan.get(), efh.bhd_directory_tables[2].get() & 0x00ff_ffff, efh.bhd_directory_tables[1].get() & 0x00ff_ffff, efh.bhd_directory_tables[0].get() & 0x00ff_ffff]; // the latter are physical addresses
        Ok(EfhBhdsIterator {
            storage: &self.storage,
            positions: positions,
            index_into_positions: 0,
        })
    }

    // Make sure there's no overlap (even when rounded to entire erasure blocks)
    fn ensure_no_overlap(&self, beginning: Location, end: Location) -> Result<()> {
        let (beginning, end) = T::grow_to_erasable_block(beginning, end);
        // FIXME: check EFH no-overlap!
        match self.psp_directory() {
            Ok(psp_directory) => {
                let (reference_beginning, reference_end) = T::grow_to_erasable_block(psp_directory.directory_beginning(), psp_directory.directory_end());
                let intersection_beginning = beginning.max(reference_beginning);
                let intersection_end = end.min(reference_end);
                if intersection_beginning < intersection_end {
                    return Err(Error::Overlap);
                }
                let (reference_beginning, reference_end) = (Location::from(psp_directory.contents_beginning()), Location::from(psp_directory.contents_end()));
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
        let bhd_directories = self.bhd_directories()?;
        for bhd_directory in bhd_directories {
            let (reference_beginning, reference_end) = T::grow_to_erasable_block(bhd_directory.directory_beginning(), bhd_directory.directory_end());
            let intersection_beginning = beginning.max(reference_beginning);
            let intersection_end = end.min(reference_end);
            if intersection_beginning < intersection_end {
                return Err(Error::Overlap);
            }
            let (reference_beginning, reference_end) = (Location::from(bhd_directory.contents_beginning()), Location::from(bhd_directory.contents_end()));
            let intersection_beginning = beginning.max(reference_beginning);
            let intersection_end = end.min(reference_end);
            if intersection_beginning < intersection_end {
                return Err(Error::Overlap);
            }
        }
        Ok(())
    }

    fn write_efh(&mut self) -> Result<()> {
        let mut buf: [u8; ERASABLE_BLOCK_SIZE] = [0xFF; ERASABLE_BLOCK_SIZE];
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
    /// Note: We always create the directory and the contents adjacent, with gap in order to allow creating new directory entries when there are already contents.
    pub fn create_bhd_directory(&mut self, beginning: ErasableLocation<ERASABLE_BLOCK_SIZE>, end: ErasableLocation<ERASABLE_BLOCK_SIZE>) -> Result<BhdDirectory<'_, T, ERASABLE_BLOCK_SIZE>> {
        match self.bhd_directories() {
            Ok(items) => {
                for directory in items {
                    // TODO: Ensure that we don't have too many similar ones
                }
            },
            Err(e) => {
                return Err(e);
            },
        }
        self.ensure_no_overlap(Location::from(beginning), Location::from(end))?;
        if self.efh.compatible_with_processor_generation(ProcessorGeneration::Milan) {
            self.efh.bhd_directory_table_milan.set(beginning.into());
            // FIXME: ensure that the others are unset?
        } else {
            self.efh.bhd_directory_tables[2].set(beginning.into());
            // FIXME: ensure that the others are unset?
        }
        self.write_efh()?;
        let result = BhdDirectory::create(&mut self.storage, beginning, end, *b"$BHD")?;
        Ok(result)
    }

    // Note: BEGINNING, END are coordinates (in Byte).
    pub fn create_psp_directory(&mut self, beginning: ErasableLocation<ERASABLE_BLOCK_SIZE>, end: ErasableLocation<ERASABLE_BLOCK_SIZE>) -> Result<PspDirectory<'_, T, ERASABLE_BLOCK_SIZE>> {
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
        self.ensure_no_overlap(Location::from(beginning), Location::from(end))?;
        // TODO: Boards older than Rome have 0xff at the top bits.  Depends on address_mode maybe.  Then, also psp_directory_table_location_naples should be set, instead.
        self.efh.psp_directory_table_location_zen.set(beginning.into());
        self.write_efh()?;
        let result = PspDirectory::create(&mut self.storage, beginning, end, *b"$PSP")?;
        Ok(result)
    }
}
