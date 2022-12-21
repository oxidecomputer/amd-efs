use crate::amdfletcher32::AmdFletcher32;
use crate::ondisk::header_from_collection;
use crate::ondisk::header_from_collection_mut;
use crate::ondisk::DirectoryEntrySerde;
pub use crate::ondisk::ProcessorGeneration;
use crate::ondisk::EFH_POSITION;
use crate::ondisk::{
    mmio_decode, mmio_encode, AddressMode, BhdDirectoryEntry,
    BhdDirectoryEntryType, BhdDirectoryHeader, ComboDirectoryEntry,
    ComboDirectoryHeader, DirectoryAdditionalInfo, DirectoryEntry,
    DirectoryHeader, Efh, EfhBulldozerSpiMode, EfhNaplesSpiMode,
    EfhRomeSpiMode, PspDirectoryEntry, PspDirectoryEntryType,
    PspDirectoryHeader, ValueOrLocation, WEAK_ADDRESS_MODE,
};
use crate::types::Error;
use crate::types::Result;
use amd_flash::{
    ErasableLocation, ErasableRange, FlashRead, FlashWrite, Location,
};
use core::convert::TryInto;
use core::mem::size_of;
use zerocopy::AsBytes;
use zerocopy::FromBytes;

// TODO: split into Directory and DirectoryContents (disjunct) if requested in additional_info.
pub struct Directory<
    MainHeader,
    Item: DirectoryEntry + FromBytes + AsBytes + Default,
    const MAIN_HEADER_SIZE: usize,
    const ITEM_SIZE: usize,
> {
    directory_address_mode: AddressMode,
    header: MainHeader,
    directory_headers_size: u32,
    // On AMD, this field specifies how much of the memory area under
    // address 2**32 (towards lower addresses) is used to memory-map
    // Flash. This is used in order to store pointers to other
    // areas on Flash (with ValueOrLocation::PhysicalAddress).
    amd_physical_mode_mmio_size: Option<u32>,
    entries: [Item; 64],
}

impl<
        MainHeader: Copy + DirectoryHeader + FromBytes + AsBytes + Default,
        Item: Copy
            + DirectoryEntrySerde
            + DirectoryEntry
            + core::fmt::Debug
            + FromBytes
            + AsBytes
            + Default,
        const MAIN_HEADER_SIZE: usize,
        const ITEM_SIZE: usize,
    > Directory<MainHeader, Item, MAIN_HEADER_SIZE, ITEM_SIZE>
{
    const MAX_DIRECTORY_ENTRIES: usize = 64; // FIXME

    pub fn header(&self) -> MainHeader {
        self.header
    }
    pub fn directory_address_mode(&self) -> AddressMode {
        self.directory_address_mode
    }
    pub fn minimal_directory_size(total_entries: u32) -> Result<u32> {
        size_of::<MainHeader>()
            .checked_add(
                size_of::<Item>()
                    .checked_mul(total_entries as usize)
                    .ok_or(Error::DirectoryRangeCheck)?,
            )
            .ok_or(Error::DirectoryRangeCheck)?
            .try_into()
            .map_err(|_| Error::DirectoryRangeCheck)
    }

    /// Note: Caller has to check whether it is the right cookie (possibly afterwards)!
    pub fn load<'a, T: FlashRead>(
        storage: &'a T,
        location: Location,
        amd_physical_mode_mmio_size: Option<u32>,
    ) -> Result<Self> {
        let mut buf: [u8; MAIN_HEADER_SIZE] = [0xff; MAIN_HEADER_SIZE];
        assert_eq!(MAIN_HEADER_SIZE, size_of::<MainHeader>()); // TODO: move to compile-time
        storage.read_exact(location, &mut buf)?;
        match header_from_collection::<MainHeader>(&buf[..]) {
            Some(header) => {
                let cookie = header.cookie();
                if cookie == *b"$PSP"
                    || cookie == *b"$PL2"
                    || cookie == *b"$BHD"
                    || cookie == *b"$BL2"
                    || cookie == *b"2PSP"
                {
                    let directory_address_mode =
                        header.additional_info().address_mode();
                    match directory_address_mode {
                        AddressMode::PhysicalAddress
                        | AddressMode::EfsRelativeOffset
                        | AddressMode::DirectoryRelativeOffset => {}
                        _ => return Err(Error::DirectoryTypeMismatch),
                    }
                    let mut entries = [Item::default(); 64];
                    for i in 0..(header.total_entries() as usize) {
                        if i < 64 {
                            let mut buf: [u8; ITEM_SIZE] = [0xff; ITEM_SIZE];
                            assert_eq!(ITEM_SIZE, size_of::<Item>()); // TODO: move to compile-time
                            storage.read_exact(
                                location + (i as u32) * (ITEM_SIZE as u32),
                                &mut buf,
                            )?;
                            match header_from_collection::<Item>(&buf[..]) {
                                Some(entry) => {
                                    entries[i] = *entry;
                                }
                                None => return Err(Error::Marshal),
                            }
                        } else {
                            return Err(Error::DirectoryRangeCheck);
                        }
                    }
                    Ok(Self {
                        directory_address_mode,
                        header: *header,
                        directory_headers_size: Self::minimal_directory_size(
                            header.total_entries(),
                        )?,
                        amd_physical_mode_mmio_size,
                        entries,
                    })
                } else {
                    Err(Error::Marshal)
                }
            }
            None => Err(Error::Marshal),
        }
    }
    fn create(
        directory_address_mode: AddressMode,
        cookie: [u8; 4],
        amd_physical_mode_mmio_size: Option<u32>,
        payloads_beginning: Option<ErasableLocation>,
        entries: &[Item],
    ) -> Result<Self> {
        // FIXME: handle directory_address_mode
        let mut header = MainHeader::default();
        header.set_cookie(cookie);
        let payloads_beginning = match payloads_beginning {
            Some(x) => Location::from(x),
            None => {
                todo!()
            }
        };
        let mut result = Self {
            directory_address_mode,
            header,
            directory_headers_size: Self::minimal_directory_size(
                Self::MAX_DIRECTORY_ENTRIES as u32,
            )?,
            amd_physical_mode_mmio_size,
            entries: [Item::default(); 64], // FIXME
        };
        for entry in entries {
            result.add_entry_direct(entry)?;
        }
        Ok(result)
    }
    /// Updates the main header checksum.  Also updates total_entries (in the same header) to TOTAL_ENTRIES.
    /// Precondition: Since the checksum is over the entire directory, that means that all the directory entries needs to be correct already.
    fn update_main_header(&mut self, total_entries: u32) -> Result<()> {
        let mut checksummer = AmdFletcher32::new();
        //let mut skip: usize = 12; // Skip fields "signature", "checksum" and "total_entries"
        checksummer.update(&[
            (total_entries & 0xffff) as u16,
            (total_entries >> 16) as u16,
        ]);
        let additional_info = u32::from(self.header.additional_info());
        checksummer.update(&[
            (additional_info & 0xffff) as u16,
            (additional_info >> 16) as u16,
        ]);
        assert!(ITEM_SIZE % 2 == 0);
        for i in 0..(self.header.total_entries() as usize) {
            let entry = &self.entries[i];
            let bytes = entry.as_bytes();
            let block = bytes
                .chunks(2)
                .map(|bytes| u16::from_le_bytes(bytes.try_into().unwrap()));
            // TODO: Optimize performance
            block.clone().for_each(|item: u16| checksummer.update(&[item]));
        }

        let checksum = checksummer.value().value();
        self.header.set_checksum(checksum);
        Ok(())
    }
    #[cfg(feature = "std")]
    pub fn save<T: FlashRead + FlashWrite>(
        &mut self,
        destination: &T,
        range: ErasableRange,
        payloads_beginning: ErasableLocation,
    ) -> Result<ErasableRange> {
        let total_entries = self.header.total_entries();
        //let additional_info = self.header.additional_info();
        let additional_info = DirectoryAdditionalInfo::new()
            .with_max_size_checked(
                DirectoryAdditionalInfo::try_into_unit(
                    range
                        .capacity()
                        .try_into()
                        .map_err(|_| Error::DirectoryRangeCheck)?,
                )
                .ok_or(Error::DirectoryRangeCheck)?,
            )
            .map_err(|_| Error::DirectoryRangeCheck)?
            .with_spi_block_size_checked(
                DirectoryAdditionalInfo::try_into_unit(
                    destination.erasable_block_size() as usize,
                )
                .ok_or(Error::DirectoryRangeCheck)?, // .try_into()
                                                     // .map_err(|_| Error::DirectoryRangeCheck)?,
            )
            .map_err(|_| Error::DirectoryRangeCheck)?
            .with_base_address(
                DirectoryAdditionalInfo::try_into_unit(
                    Location::from(payloads_beginning)
                        .try_into()
                        .map_err(|_| Error::DirectoryRangeCheck)?,
                )
                .ok_or(Error::DirectoryRangeCheck)?,
            )
            .with_address_mode(AddressMode::EfsRelativeOffset);
        self.header.set_additional_info(additional_info);
        self.header.set_additional_info(additional_info);
        self.update_main_header(total_entries)?;
        let size = Self::minimal_directory_size(total_entries)?;
        let (range, rest) = range.split_at_least(size as usize);
        let mut result = Vec::<u8>::new();
        result.extend_from_slice(self.header.as_bytes());
        for entry in &self.entries[..total_entries as usize] {
            result.extend_from_slice(entry.as_bytes());
        }
        destination.erase_and_write_blocks(range.beginning, &result)?;
        Ok(rest)
    }
    pub fn entries(&self) -> impl Iterator<Item = Item> + '_ {
        let mut index = 0usize;
        core::iter::from_fn(move || {
            if index < self.header.total_entries() as usize {
                let result = self.entries[index];
                index = index + 1;
                Some(result)
            } else {
                None
            }
        })
    }

    pub(crate) fn add_entry_direct(&mut self, entry: &Item) -> Result<()> {
        let total_entries = self
            .header
            .total_entries()
            .checked_add(1)
            .ok_or(Error::DirectoryRangeCheck)?;
        self.entries[total_entries as usize - 1] = *entry;
        self.header.set_total_entries(total_entries);
        Ok(())
    }
}

pub type PspDirectory = Directory<
    PspDirectoryHeader,
    PspDirectoryEntry,
    { size_of::<PspDirectoryHeader>() },
    { size_of::<PspDirectoryEntry>() },
>;
pub type BhdDirectory = Directory<
    BhdDirectoryHeader,
    BhdDirectoryEntry,
    { size_of::<BhdDirectoryHeader>() },
    { size_of::<BhdDirectoryEntry>() },
>;
pub type ComboDirectory = Directory<
    ComboDirectoryHeader,
    ComboDirectoryEntry,
    { size_of::<ComboDirectoryHeader>() },
    { size_of::<ComboDirectoryEntry>() },
>;

impl
    Directory<
        PspDirectoryHeader,
        PspDirectoryEntry,
        { size_of::<PspDirectoryHeader>() },
        { size_of::<PspDirectoryEntry>() },
    >
{
    // TODO: Type-check value
    pub fn add_value_entry(
        &mut self,
        entry: &mut PspDirectoryEntry,
    ) -> Result<()> {
        if let ValueOrLocation::Value(_) = entry.source(WEAK_ADDRESS_MODE)? {
            self.add_entry_direct(entry)?;
            Ok(())
        } else {
            Err(Error::EntryTypeMismatch)
        }
    }
}

pub struct EfhBhdsIterator<'a, T: FlashRead + FlashWrite> {
    storage: &'a T,
    physical_address_mode: bool,
    positions: [u32; 4], // 0xffff_ffff: invalid
    index_into_positions: usize,
    amd_physical_mode_mmio_size: Option<u32>,
}

impl<'a, T: FlashRead + FlashWrite> Iterator for EfhBhdsIterator<'a, T> {
    type Item = BhdDirectory;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        while self.index_into_positions < self.positions.len() {
            let position = self.positions[self.index_into_positions];
            self.index_into_positions += 1;
            if position != 0xffff_ffff && position != 0
            /* sigh.  Some images have 0 as "invalid" mark */
            {
                return BhdDirectory::load(
                    self.storage,
                    position,
                    self.amd_physical_mode_mmio_size,
                )
                .ok(); // FIXME: error check
            }
        }
        None
    }
}

pub const fn preferred_efh_location(
    processor_generation: ProcessorGeneration,
) -> Location {
    match processor_generation {
        ProcessorGeneration::Naples => 0x2_0000,
        ProcessorGeneration::Rome | ProcessorGeneration::Milan => 0xFA_0000,
    }
}

pub struct Efs<'a, T: FlashRead + FlashWrite> {
    storage: &'a T,
    efh_beginning: ErasableLocation,
    pub efh: Efh,
    amd_physical_mode_mmio_size: Option<u32>,
}

impl<'a, T: FlashRead + FlashWrite> Efs<'a, T> {
    // FIXME maybe not public
    pub fn erasable_location(
        &self,
        location: Location,
    ) -> Option<ErasableLocation> {
        self.storage.erasable_location(location)
    }
    pub fn erasable_location_mut(
        &mut self,
        location: Location,
    ) -> Option<ErasableLocation> {
        self.storage.erasable_location(location)
    }
    // TODO: If we wanted to, we could also try the whole thing on the top 16 MiB again
    // (I think it would be better to have the user just construct two
    // different Efs instances in that case)
    const EFH_SIZE: u32 = 0x200;
    pub(crate) fn efh_beginning(
        storage: &T,
        processor_generation: Option<ProcessorGeneration>,
    ) -> Result<ErasableLocation> {
        for position in EFH_POSITION.iter() {
            let mut xbuf: [u8; size_of::<Efh>()] = [0; size_of::<Efh>()];
            storage.read_exact(*position, &mut xbuf)?;
            if let Some(item) = header_from_collection::<Efh>(&xbuf[..]) {
                // Note: only one Efh with second_gen_efs() allowed in entire Flash!
                if item.signature().ok().unwrap_or(0) == 0x55AA55AA
                    && item.second_gen_efs()
                    && match processor_generation {
                        Some(x) => item.compatible_with_processor_generation(x),
                        None => true,
                    }
                {
                    return Ok(storage
                        .erasable_location(*position)
                        .ok_or(Error::Misaligned)?);
                }
            }
        }
        // Old firmware header is better than no firmware header; TODO: Warn.
        for position in EFH_POSITION.iter() {
            let mut xbuf: [u8; size_of::<Efh>()] = [0; size_of::<Efh>()];
            storage.read_exact(*position, &mut xbuf)?;
            if let Some(item) = header_from_collection::<Efh>(&xbuf[..]) {
                if item.signature().ok().unwrap_or(0) == 0x55AA55AA
                    && !item.second_gen_efs()
                    && match processor_generation {
                        //Some(x) => item.compatible_with_processor_generation(x),
                        None => true,
                        Some(ProcessorGeneration::Naples) => true,
                        _ => false,
                    }
                {
                    return Ok(storage
                        .erasable_location(*position)
                        .ok_or(Error::Misaligned)?);
                }
            }
        }
        Err(Error::EfsHeaderNotFound)
    }

    pub fn physical_address_mode(&self) -> bool {
        self.efh.physical_address_mode()
    }

    /// This loads the Embedded Firmware Structure (EFS) from STORAGE.
    /// Should the EFS be old enough to still use physical mmio addresses
    /// for pointers on the Flash, AMD_PHYSICAL_MODE_MMIO_SIZE is required.
    /// Otherwise, AMD_PHYSICAL_MODE_MMIO_SIZE is allowed to be None.
    pub fn load(
        storage: &'a T,
        processor_generation: Option<ProcessorGeneration>,
        amd_physical_mode_mmio_size: Option<u32>,
    ) -> Result<Self> {
        let efh_beginning =
            Self::efh_beginning(&storage, processor_generation)?;
        let mut xbuf: [u8; size_of::<Efh>()] = [0; size_of::<Efh>()];
        storage.read_exact(efh_beginning.into(), &mut xbuf)?;
        let efh = header_from_collection::<Efh>(&xbuf[..])
            .ok_or(Error::EfsHeaderNotFound)?;
        if efh.signature().ok().unwrap_or(0) != 0x55aa_55aa {
            return Err(Error::EfsHeaderNotFound);
        }

        Ok(Self {
            storage,
            efh_beginning,
            efh: *efh,
            amd_physical_mode_mmio_size,
        })
    }
    pub fn create(
        storage: &'a T,
        processor_generation: ProcessorGeneration,
        efh_beginning: Location,
        amd_physical_mode_mmio_size: Option<u32>,
    ) -> Result<Self> {
        if !EFH_POSITION.contains(&efh_beginning)
            || preferred_efh_location(processor_generation) != efh_beginning
        {
            return Err(Error::EfsRangeCheck);
        }

        let mut buf: [u8; size_of::<Efh>()] = [0xFF; size_of::<Efh>()]; // FIXME should be: EFH_SIZE == size_of::<Efh>()
        match header_from_collection_mut(&mut buf[..]) {
            Some(item) => {
                let mut efh: Efh = Efh::default();
                efh.efs_generations.set(
                    Efh::efs_generations_for_processor_generation(
                        processor_generation,
                    ),
                );
                *item = efh;
            }
            None => {
                return Err(Error::Marshal);
            }
        }

        storage.erase_and_write_blocks(
            storage
                .erasable_location(efh_beginning)
                .ok_or(Error::Misaligned)?,
            &buf,
        )?;
        Self::load(
            storage,
            Some(processor_generation),
            amd_physical_mode_mmio_size,
        )
    }

    /// Note: Either psp_directory or psp_combo_directory will succeed--but not both.
    pub fn psp_directory(&self) -> Result<PspDirectory> {
        let psp_directory_table_location = self
            .efh
            .psp_directory_table_location_zen()
            .ok()
            .unwrap_or(0xffff_ffff);
        if psp_directory_table_location == 0xffff_ffff
            || psp_directory_table_location == 0
        {
            Err(Error::PspDirectoryHeaderNotFound)
        } else {
            match PspDirectory::load(
                self.storage,
                psp_directory_table_location,
                self.amd_physical_mode_mmio_size,
            ) {
                Ok(directory) => {
                    if directory.header.cookie == *b"$PSP" {
                        // level 1 PSP header should have "$PSP" cookie
                        return Ok(directory);
                    }
                }
                Err(Error::Marshal) => {}
                Err(e) => {
                    return Err(e);
                }
            };

            // That's the same fallback AMD does on Naples:

            let psp_directory_table_location = {
                let addr = self
                    .efh
                    .psp_directory_table_location_naples()
                    .ok()
                    .unwrap_or(0xffff_ffff);
                if addr == 0xffff_ffff {
                    addr
                } else {
                    addr & 0x00ff_ffff
                }
            };
            if psp_directory_table_location == 0xffff_ffff
                || psp_directory_table_location == 0
            {
                Err(Error::PspDirectoryHeaderNotFound)
            } else {
                let directory = PspDirectory::load(
                    self.storage,
                    psp_directory_table_location,
                    self.amd_physical_mode_mmio_size,
                )?;
                if directory.header.cookie == *b"$PSP" {
                    // level 1 PSP header should have "$PSP" cookie
                    Ok(directory)
                } else {
                    Err(Error::Marshal)
                }
            }
        }
    }

    /// Note: Either psp_directory or psp_combo_directory will succeed--but not both.
    pub fn psp_combo_directory(&self) -> Result<ComboDirectory> {
        let psp_directory_table_location = self
            .efh
            .psp_directory_table_location_zen()
            .ok()
            .unwrap_or(0xffff_ffff);
        if psp_directory_table_location == 0xffff_ffff {
            Err(Error::PspDirectoryHeaderNotFound)
        } else {
            match ComboDirectory::load(
                self.storage,
                psp_directory_table_location,
                self.amd_physical_mode_mmio_size,
            ) {
                Ok(directory) => {
                    if directory.header.cookie == *b"2PSP" {
                        return Ok(directory);
                    }
                }
                Err(Error::Marshal) => {}
                Err(e) => {
                    return Err(e);
                }
            };

            // That's the same fallback AMD does on Naples:

            let psp_directory_table_location = {
                let addr = self
                    .efh
                    .psp_directory_table_location_naples()
                    .ok()
                    .unwrap_or(0xffff_ffff);
                if addr == 0xffff_ffff {
                    addr
                } else {
                    addr & 0x00ff_ffff
                }
            };
            if psp_directory_table_location == 0xffff_ffff
                || psp_directory_table_location == 0
            {
                Err(Error::PspDirectoryHeaderNotFound)
            } else {
                let directory = ComboDirectory::load(
                    self.storage,
                    psp_directory_table_location,
                    self.amd_physical_mode_mmio_size,
                )?;
                if directory.header.cookie == *b"2PSP" {
                    Ok(directory)
                } else {
                    Err(Error::Marshal)
                }
            }
        }
    }

    /// Returns an iterator over level 1 BHD directories.
    /// If PROCESSOR_GENERATION is Some, then only return the directories
    /// matching that generation.
    pub fn bhd_directories(
        &self,
        processor_generation: Option<ProcessorGeneration>,
    ) -> Result<EfhBhdsIterator<T>> {
        /// Given V which is possibly a MMIO address (from inside a
        /// directory entry), convert it to a regular offset
        /// relative to the beginning of the flash.
        fn de_mmio(v: u32, amd_physical_mode_mmio_size: Option<u32>) -> u32 {
            if v == 0xffff_ffff || v == 0 {
                0xffff_ffff
            } else if let Some(amd_physical_mode_mmio_size) =
                amd_physical_mode_mmio_size
            {
                match mmio_decode(v, amd_physical_mode_mmio_size) {
                    Ok(v) => v,
                    Err(Error::DirectoryTypeMismatch) => {
                        // Rome is a grey-area that supports both MMIO addresses and offsets
                        if v < amd_physical_mode_mmio_size {
                            v
                        } else {
                            0xffff_ffff
                        }
                    }
                    Err(_) => 0xffff_ffff,
                }
            } else {
                0xffff_ffff
            }
        }
        let efh = &self.efh;
        let amd_physical_mode_mmio_size = self.amd_physical_mode_mmio_size;
        let invalid_position = 0xffff_ffffu32;
        let positions = match processor_generation {
            Some(ProcessorGeneration::Milan) => [
                efh.bhd_directory_table_milan()
                    .ok()
                    .unwrap_or(invalid_position),
                invalid_position,
                invalid_position,
                invalid_position,
            ],
            Some(ProcessorGeneration::Rome) => [
                de_mmio(
                    efh.bhd_directory_tables[2].get(),
                    amd_physical_mode_mmio_size,
                ),
                invalid_position,
                invalid_position,
                invalid_position,
            ],
            Some(ProcessorGeneration::Naples) => [
                de_mmio(
                    efh.bhd_directory_tables[0].get(),
                    amd_physical_mode_mmio_size,
                ),
                invalid_position,
                invalid_position,
                invalid_position,
            ],
            None => [
                // allow all (used for example for overlap checking)
                efh.bhd_directory_table_milan()
                    .ok()
                    .unwrap_or(invalid_position),
                de_mmio(
                    efh.bhd_directory_tables[2].get(),
                    amd_physical_mode_mmio_size,
                ),
                de_mmio(
                    efh.bhd_directory_tables[1].get(),
                    amd_physical_mode_mmio_size,
                ),
                de_mmio(
                    efh.bhd_directory_tables[0].get(),
                    amd_physical_mode_mmio_size,
                ),
            ],
        };
        Ok(EfhBhdsIterator {
            storage: self.storage,
            physical_address_mode: self.physical_address_mode(),
            positions,
            index_into_positions: 0,
            amd_physical_mode_mmio_size: self.amd_physical_mode_mmio_size,
        })
    }

    /// Return the directory matching PROCESSOR_GENERATION,
    /// or any directory if that is None.
    pub fn bhd_directory(
        &self,
        processor_generation: Option<ProcessorGeneration>,
    ) -> Result<BhdDirectory> {
        if let Some(bhd_directory) =
            self.bhd_directories(processor_generation).unwrap().next()
        {
            return Ok(bhd_directory);
        }
        Err(Error::BhdDirectoryHeaderNotFound)
    }

    fn write_efh(&mut self) -> Result<()> {
        let mut buf: [u8; size_of::<Efh>()] = [0xFF; size_of::<Efh>()];
        if let Some(item) = header_from_collection_mut(&mut buf[..]) {
            *item = self.efh;
        }

        self.storage.erase_and_write_blocks(self.efh_beginning, &buf)?;
        Ok(())
    }

    pub fn spi_mode_bulldozer(&self) -> Result<EfhBulldozerSpiMode> {
        self.efh.spi_mode_bulldozer()
    }
    pub fn set_spi_mode_bulldozer(&mut self, value: EfhBulldozerSpiMode) {
        self.efh.set_spi_mode_bulldozer(value)
        // FIXME: write_efh ?
    }
    pub fn spi_mode_zen_naples(&self) -> Result<EfhNaplesSpiMode> {
        self.efh.spi_mode_zen_naples()
    }

    pub fn set_spi_mode_zen_naples(&mut self, value: EfhNaplesSpiMode) {
        self.efh.set_spi_mode_zen_naples(value)
        // FIXME: write_efh ?
    }

    pub fn spi_mode_zen_rome(&self) -> Result<EfhRomeSpiMode> {
        self.efh.spi_mode_zen_rome()
    }

    pub fn set_spi_mode_zen_rome(&mut self, value: EfhRomeSpiMode) {
        self.efh.set_spi_mode_zen_rome(value)
        // FIXME: write_efh ?
    }

    /// Note: BEGINNING, END are coordinates (in Byte).
    /// Note: We always create the directory and the contents adjacent, with gap in order to allow creating new directory entries when there are already contents.
    pub fn create_bhd_directory(
        &mut self,
        beginning: ErasableLocation,
        end: ErasableLocation,
        default_entry_address_mode: AddressMode,
        payloads_beginning: Option<ErasableLocation>,
        entries: &[BhdDirectoryEntry],
    ) -> Result<BhdDirectory> {
        match default_entry_address_mode {
            AddressMode::PhysicalAddress => {
                if !self.physical_address_mode() {
                    return Err(Error::DirectoryTypeMismatch);
                }
            }
            AddressMode::EfsRelativeOffset => {
                if self.physical_address_mode() {
                    return Err(Error::DirectoryTypeMismatch);
                }
            }
            _ => return Err(Error::DirectoryTypeMismatch),
        }
        match self.bhd_directories(None) {
            Ok(items) => {
                for directory in items {
                    // TODO: Ensure that we don't have too many similar ones
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
        if self
            .efh
            .compatible_with_processor_generation(ProcessorGeneration::Milan)
        {
            self.efh.set_bhd_directory_table_milan(beginning.into());
        // FIXME: ensure that the others are unset?
        } else {
            self.efh.bhd_directory_tables[2].set(beginning.into());
            // FIXME: ensure that the others are unset?
        }
        self.write_efh()?;
        let result = BhdDirectory::create(
            default_entry_address_mode,
            *b"$BHD",
            self.amd_physical_mode_mmio_size,
            payloads_beginning,
            entries,
        )?;
        Ok(result)
    }

    // Note: BEGINNING, END are coordinates (in Byte).
    pub fn create_psp_directory(
        &mut self,
        beginning: ErasableLocation,
        end: ErasableLocation,
        default_entry_address_mode: AddressMode,
        payloads_beginning: Option<ErasableLocation>,
        entries: &[PspDirectoryEntry],
    ) -> Result<PspDirectory> {
        match default_entry_address_mode {
            AddressMode::PhysicalAddress => {
                if !self.physical_address_mode() {
                    return Err(Error::DirectoryTypeMismatch);
                }
            }
            AddressMode::EfsRelativeOffset => {
                if self.physical_address_mode() {
                    return Err(Error::DirectoryTypeMismatch);
                }
            }
            _ => return Err(Error::DirectoryTypeMismatch),
        }
        match self.psp_directory() {
            Err(Error::PspDirectoryHeaderNotFound) => {}
            Err(e) => {
                return Err(e);
            }
            Ok(_) => {
                return Err(Error::Duplicate);
            }
        }
        // TODO: Boards older than Rome have 0xff at the top bits.  Depends on address_mode maybe.
        // Then, also psp_directory_table_location_naples should be set, instead.
        self.efh.set_psp_directory_table_location_zen(beginning.into());
        self.write_efh()?;
        let result = PspDirectory::create(
            default_entry_address_mode,
            *b"$PSP",
            self.amd_physical_mode_mmio_size,
            payloads_beginning,
            entries,
        )?;
        Ok(result)
    }

    pub fn create_psp_subdirectory(
        &self,
        directory: &mut PspDirectory,
        beginning: ErasableLocation,
        end: ErasableLocation,
        amd_physical_mode_mmio_size: Option<u32>,
        payloads_beginning: Option<ErasableLocation>,
        entries: &[PspDirectoryEntry],
    ) -> Result<PspDirectory> {
        if directory.header.cookie() != *b"$PSP" {
            return Err(Error::DirectoryTypeMismatch);
        }
        // TODO
        //		if let Err(Error::EntryNotFound) = self.psp_subdirectory(directory) {
        directory.add_entry_direct(&mut PspDirectoryEntry::new_payload(
            directory.directory_address_mode(),
            PspDirectoryEntryType::SecondLevelDirectory,
            Some(ErasableLocation::extent(beginning, end)),
            Some(ValueOrLocation::EfsRelativeOffset(beginning.into())),
        )?)?;
        PspDirectory::create(
            directory.directory_address_mode,
            *b"$PL2",
            amd_physical_mode_mmio_size,
            payloads_beginning,
            entries,
        )
        //		} else {
        //			Err(Error::Duplicate)
        //		}
    }

    pub fn create_second_level_psp_directory(
        &self,
        beginning: ErasableLocation,
        end: ErasableLocation,
        payloads_beginning: Option<ErasableLocation>,
        entries: &[PspDirectoryEntry],
    ) -> Result<PspDirectory> {
        let mut psp_directory = self.psp_directory()?;
        self.create_psp_subdirectory(
            &mut psp_directory,
            beginning,
            end,
            self.amd_physical_mode_mmio_size,
            payloads_beginning,
            entries,
        )
    }
}
