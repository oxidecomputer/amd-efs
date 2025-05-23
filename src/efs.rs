use crate::amdfletcher32::AmdFletcher32;
use crate::flash;
#[cfg(feature = "std")]
use crate::ondisk::DirectoryAdditionalInfo;
use crate::ondisk::DirectoryEntrySerde;
use crate::ondisk::EFH_POSITION;
pub use crate::ondisk::ProcessorGeneration;
use crate::ondisk::header_from_collection;
use crate::ondisk::header_from_collection_mut;
use crate::ondisk::{
    AddressMode, BhdDirectoryEntry, BhdDirectoryEntryType, BhdDirectoryHeader,
    ComboDirectoryEntry, ComboDirectoryHeader, DirectoryEntry, DirectoryHeader,
    Efh, EfhBulldozerSpiMode, EfhEspiConfiguration, EfhNaplesSpiMode,
    EfhRomeSpiMode, PspDirectoryEntry, PspDirectoryEntryType,
    PspDirectoryHeader, ValueOrLocation, WEAK_ADDRESS_MODE, mmio_decode,
};
use crate::types::Error;
use crate::types::Result;

use core::convert::TryInto;
use core::mem::size_of;
#[cfg(feature = "std")]
use flash::ErasableRange;
use flash::{ErasableLocation, FlashRead, FlashWrite, Location};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

// XXX: This is arbitrary.
const MAX_DIRECTORY_ENTRIES: usize = 64;

// TODO: split into Directory and DirectoryContents (disjunct) if requested in additional_info.
pub struct Directory<
    MainHeader,
    Item: DirectoryEntry + FromBytes + IntoBytes + Immutable + KnownLayout + Default,
    const MAIN_HEADER_SIZE: usize,
    const ITEM_SIZE: usize,
> {
    // Heuristics: It's the beginning of a PSP directory, if we want such a
    // connection from the directory we are actually reading.
    // Otherwise it's 0.
    mode3_base: Location,
    beginning: Location, // mostly to help following outward pointers
    directory_address_mode: AddressMode,
    header: MainHeader,
    // On AMD, this field specifies how much of the memory area under
    // address 2**32 (towards lower addresses) is used to memory-map
    // Flash. This is used in order to store pointers to other
    // areas on Flash (with ValueOrLocation::PhysicalAddress).
    amd_physical_mode_mmio_size: Option<u32>,
    entries: [Item; MAX_DIRECTORY_ENTRIES],
}

impl<
    MainHeader: Copy
        + DirectoryHeader
        + FromBytes
        + IntoBytes
        + Immutable
        + KnownLayout
        + Default,
    Item: Copy
        + DirectoryEntrySerde
        + DirectoryEntry
        + core::fmt::Debug
        + FromBytes
        + IntoBytes
        + Immutable
        + KnownLayout
        + Default,
    const MAIN_HEADER_SIZE: usize,
    const ITEM_SIZE: usize,
> Directory<MainHeader, Item, MAIN_HEADER_SIZE, ITEM_SIZE>
{
    pub fn header(&self) -> MainHeader {
        self.header
    }
    pub fn beginning(&self) -> Location {
        self.beginning
    }
    pub fn directory_address_mode(&self) -> AddressMode {
        self.directory_address_mode
    }
    pub fn minimal_directory_size(total_entries: usize) -> Result<usize> {
        size_of::<MainHeader>()
            .checked_add(
                size_of::<Item>()
                    .checked_mul(total_entries)
                    .ok_or(Error::DirectoryRangeCheck)?,
            )
            .ok_or(Error::DirectoryRangeCheck)
    }

    /// Note: Caller should check whether it is the right cookie (afterwards)
    /// This is only used to load the second-level directory when dumping.
    /// There are nicer accessors otherwise (bhd_directory, psp_directory etc)
    pub fn load<T: FlashRead>(
        storage: &T,
        beginning: Location,
        mode3_base: Location,
        amd_physical_mode_mmio_size: Option<u32>,
    ) -> Result<Self> {
        let mut buf: [u8; MAIN_HEADER_SIZE] = [0xff; MAIN_HEADER_SIZE];
        assert_eq!(MAIN_HEADER_SIZE, size_of::<MainHeader>());
        storage.read_exact(beginning, &mut buf)?;
        let header = header_from_collection::<MainHeader>(&buf[..])
            .ok_or(Error::Marshal)?;
        let cookie = header.cookie();
        if !MainHeader::ALLOWED_COOKIES.contains(&cookie) {
            return Err(Error::DirectoryTypeMismatch);
        }
        let directory_address_mode = header.additional_info().address_mode();
        let directory_type_matches = matches!(
            directory_address_mode,
            AddressMode::PhysicalAddress
                | AddressMode::EfsRelativeOffset
                | AddressMode::DirectoryRelativeOffset
        );
        if !directory_type_matches {
            return Err(Error::DirectoryTypeMismatch);
        }
        let mut entries = [Item::default(); MAX_DIRECTORY_ENTRIES];
        let mut cursor = beginning
            .checked_add(MAIN_HEADER_SIZE as u32)
            .ok_or(Error::DirectoryRangeCheck)?;
        for (i, ie) in
            entries.iter_mut().enumerate().take(header.total_entries() as usize)
        {
            if i >= MAX_DIRECTORY_ENTRIES {
                return Err(Error::DirectoryRangeCheck);
            }
            let mut buf: [u8; ITEM_SIZE] = [0xff; ITEM_SIZE];
            assert_eq!(ITEM_SIZE, size_of::<Item>()); // TODO: move to compile-time
            storage.read_exact(cursor, &mut buf)?;
            cursor = cursor
                .checked_add(ITEM_SIZE as u32)
                .ok_or(Error::DirectoryRangeCheck)?;
            *ie = *header_from_collection::<Item>(&buf[..])
                .ok_or(Error::Marshal)?;
        }
        Ok(Self {
            beginning,
            mode3_base,
            directory_address_mode,
            header: *header,
            amd_physical_mode_mmio_size,
            entries,
        })
    }
    fn create(
        beginning: Location,
        mode3_base: Location,
        directory_address_mode: AddressMode,
        cookie: [u8; 4],
        amd_physical_mode_mmio_size: Option<u32>,
        entries: &[Item],
    ) -> Result<Self> {
        // This DIRECTORY mode is currently unsupported by PSP ABL.
        if directory_address_mode == AddressMode::OtherDirectoryRelativeOffset {
            return Err(Error::DirectoryTypeMismatch);
        }
        let mut header = MainHeader::default();
        header.set_cookie(cookie);
        let mut result = Self {
            beginning,
            mode3_base,
            directory_address_mode,
            header,
            amd_physical_mode_mmio_size,
            entries: [Item::default(); MAX_DIRECTORY_ENTRIES],
        };
        for entry in entries {
            result.add_entry_direct(entry)?;
        }
        Ok(result)
    }
    /// Updates the main header checksum.  Also updates total_entries (in the same header) to TOTAL_ENTRIES.
    /// Precondition: Since the checksum is over the entire directory, that means that all the directory entries needs to be correct already.
    #[allow(dead_code)]
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
    pub fn save(
        &mut self,
        erasable_block_size: usize,
        range: &ErasableRange,
        payloads_beginning: ErasableLocation,
    ) -> Result<Vec<u8>> {
        let cookie = self.header.cookie();
        if !MainHeader::ALLOWED_COOKIES.contains(&cookie) {
            return Err(Error::DirectoryTypeMismatch);
        }

        let total_entries = self.header.total_entries();
        //let additional_info = self.header.additional_info();
        let additional_info = DirectoryAdditionalInfo::new()
            .with_max_size_checked(
                DirectoryAdditionalInfo::try_into_unit(range.capacity())
                    .ok_or(Error::DirectoryRangeCheck)?,
            )
            .map_err(|_| Error::DirectoryRangeCheck)?
            .with_spi_block_size_checked(
                DirectoryAdditionalInfo::try_into_unit(erasable_block_size)
                    .ok_or(Error::DirectoryRangeCheck)?,
            )
            .map_err(|_| Error::DirectoryRangeCheck)?
            .with_base_address(
                DirectoryAdditionalInfo::try_into_unit(
                    Location::from(payloads_beginning)
                        .try_into()
                        .map_err(|_| Error::DirectoryRangeCheck)?,
                )
                // If that happens it's likely that your payload flash_location
                // is not aligned 4 kiB.
                .ok_or(Error::DirectoryPayloadMisaligned)?,
            )
            .with_address_mode(AddressMode::EfsRelativeOffset);
        self.header.set_additional_info(additional_info);
        self.header.set_additional_info(additional_info);
        self.update_main_header(total_entries)?;
        //let size = Self::minimal_directory_size(total_entries)?;
        //let _ = range.take_at_least(size as usize);
        let mut result = Vec::<u8>::new();
        result.extend_from_slice(self.header.as_bytes());
        for entry in &self.entries[..total_entries as usize] {
            result.extend_from_slice(entry.as_bytes());
        }
        Ok(result)
    }
    pub fn entries(&self) -> impl Iterator<Item = Item> + '_ {
        let mut index = 0usize;
        core::iter::from_fn(move || {
            if index < self.header.total_entries() as usize {
                let result = self.entries[index];
                index += 1;
                Some(result)
            } else {
                None
            }
        })
    }
    pub fn location_of_source(
        &self,
        source: ValueOrLocation,
        entry_base_location: Location,
    ) -> Result<Location> {
        match source {
            ValueOrLocation::Value(_) => Err(Error::DirectoryTypeMismatch),
            ValueOrLocation::PhysicalAddress(y) => {
                // or unknown
                self.amd_physical_mode_mmio_size
                    .map(|size| {
                        mmio_decode(y, size).or(
                            // Older Zen models also allowed a flash offset
                            // here.  So allow that as well.
                            // TODO: Maybe thread through the processor
                            // generation and only do on Naples and Rome.
                            if y < size {
                                Ok(y)
                            } else {
                                Err(Error::EntryTypeMismatch)
                            },
                        )
                    })
                    .ok_or(Error::EntryTypeMismatch)?
            }
            ValueOrLocation::EfsRelativeOffset(x) => Ok(x),
            ValueOrLocation::DirectoryRelativeOffset(y) => Ok(self
                .beginning
                .checked_add(y)
                .ok_or(Error::DirectoryPayloadRangeCheck)?),
            ValueOrLocation::OtherDirectoryRelativeOffset(y) => Ok(y
                .checked_add(entry_base_location)
                .ok_or(Error::DirectoryPayloadRangeCheck)?),
        }
    }
    pub fn payload_beginning(&self, entry: &Item) -> Result<Location> {
        let source = entry.source(self.directory_address_mode)?;
        self.location_of_source(source, self.mode3_base)
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

pub const fn preferred_efh_location(
    processor_generation: ProcessorGeneration,
) -> Location {
    match processor_generation {
        ProcessorGeneration::Naples
        | ProcessorGeneration::Genoa
        | ProcessorGeneration::Turin => 0x2_0000,
        ProcessorGeneration::Rome | ProcessorGeneration::Milan => 0xFA_0000,
    }
}

pub struct Efs<'a, T: FlashRead + FlashWrite> {
    storage: &'a T,
    efh_beginning: ErasableLocation,
    efh: Efh,
    amd_physical_mode_mmio_size: Option<u32>,
}

impl<'a, T: FlashRead + FlashWrite> Efs<'a, T> {
    pub fn compatible_with_processor_generation(
        &self,
        processor_generation: ProcessorGeneration,
    ) -> bool {
        self.efh.compatible_with_processor_generation(processor_generation)
    }

    // TODO: If we wanted to, we could also try the whole thing on the top 16 MiB again
    // (I think it would be better to have the user just construct two
    // different Efs instances in that case)
    pub(crate) fn efh_beginning(
        storage: &T,
        processor_generation: Option<ProcessorGeneration>,
    ) -> Result<ErasableLocation> {
        let positions = if let Some(
            ProcessorGeneration::Genoa | ProcessorGeneration::Turin,
        ) = processor_generation
        {
            // Starting with Genoa, only one EFS offset is allowed.
            [0x2_0000].as_slice()
        } else {
            EFH_POSITION.as_slice()
        };
        for position in positions.iter() {
            let mut xbuf: [u8; size_of::<Efh>()] = [0; size_of::<Efh>()];
            storage.read_exact(*position, &mut xbuf)?;
            if let Some(item) = header_from_collection::<Efh>(&xbuf[..]) {
                // Note: only one Efh with second_gen_efs() allowed in entire Flash!
                if item.signature().ok().unwrap_or(0) == 0x55AA55AA
                    && !item.physical_address_mode()
                    && match processor_generation {
                        Some(x) => item.compatible_with_processor_generation(x),
                        None => true,
                    }
                {
                    return storage
                        .erasable_location(*position)
                        .map_err(|_| Error::Misaligned);
                }
            }
        }
        // Old firmware header is better than no firmware header; TODO: Warn.
        for position in positions.iter() {
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
                    return storage
                        .erasable_location(*position)
                        .map_err(|_| Error::Misaligned);
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
        let efh_beginning = Self::efh_beginning(storage, processor_generation)?;
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
        let item =
            header_from_collection_mut(&mut buf[..]).ok_or(Error::Marshal)?;
        let mut efh: Efh = Efh::default();
        efh.efs_generations.set(Efh::efs_generations_for_processor_generation(
            processor_generation,
        ));
        *item = efh;

        storage.erase_and_write_blocks(
            storage
                .erasable_location(efh_beginning)
                .map_err(|_| Error::Misaligned)?,
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
        if Efh::is_invalid_directory_table_location(
            psp_directory_table_location,
        ) {
            // Note: We could also check efh.psp_directory_location_naples(),
            // but not even a newer Naples did that.
            Err(Error::PspDirectoryHeaderNotFound)
        } else {
            let psp_directory_table_location = if self.physical_address_mode() {
                assert!(Efh::is_invalid_directory_table_location(
                    self.efh.psp_directory_table_location_naples()?
                ));
                Efh::de_mmio(
                    psp_directory_table_location,
                    self.amd_physical_mode_mmio_size,
                )
                .ok_or(Error::Marshal)?
            } else {
                assert!(Efh::is_likely_location(psp_directory_table_location));
                psp_directory_table_location
            };
            let directory = PspDirectory::load(
                self.storage,
                psp_directory_table_location,
                psp_directory_table_location,
                self.amd_physical_mode_mmio_size,
            )?;
            if directory.header.cookie != PspDirectoryHeader::FIRST_LEVEL_COOKIE
            {
                return Err(Error::DirectoryTypeMismatch);
            }
            Ok(directory)
        }
    }

    /// Note: Either psp_directory or psp_combo_directory will succeed--but not both.
    pub fn psp_combo_directory(&self) -> Result<ComboDirectory> {
        let psp_directory_table_location = self
            .efh
            .psp_directory_table_location_zen()
            .ok()
            .unwrap_or(0xffff_ffff);
        if Efh::is_invalid_directory_table_location(
            psp_directory_table_location,
        ) {
            Err(Error::PspDirectoryHeaderNotFound)
        } else {
            let psp_directory_table_location = if self.physical_address_mode() {
                assert!(Efh::is_invalid_directory_table_location(
                    self.efh.psp_directory_table_location_naples()?
                ));
                Efh::de_mmio(
                    psp_directory_table_location,
                    self.amd_physical_mode_mmio_size,
                )
                .ok_or(Error::Marshal)?
            } else {
                assert!(Efh::is_likely_location(psp_directory_table_location));
                psp_directory_table_location
            };
            let directory = ComboDirectory::load(
                self.storage,
                psp_directory_table_location,
                0,
                self.amd_physical_mode_mmio_size,
            )?;
            if directory.header.cookie != ComboDirectoryHeader::PSP_COOKIE {
                return Err(Error::DirectoryTypeMismatch);
            }
            Ok(directory)
        }
    }

    /// Returns an iterator over level 1 BHD directories.
    /// If PROCESSOR_GENERATION is Some, then only return the directories
    /// matching that generation.
    ///
    /// The thing at each Location can be one of those things:
    ///
    /// * A ComboDirectory with entries' payload of type BhdDirectory
    /// * A BhdDirectory
    ///
    /// Therefore, just return locations.
    pub fn bhd_directories(
        &self,
        processor_generation: Option<ProcessorGeneration>,
    ) -> Result<impl Iterator<Item = Location>> {
        let efh = &self.efh;
        let amd_physical_mode_mmio_size = self.amd_physical_mode_mmio_size;
        let positions = match processor_generation {
            Some(ProcessorGeneration::Genoa | ProcessorGeneration::Turin) => {
                [efh.bhd_directory_table_milan().ok(), None, None, None]
            }
            Some(ProcessorGeneration::Milan) => {
                [efh.bhd_directory_table_milan().ok(), None, None, None]
            }
            Some(ProcessorGeneration::Rome) => [
                Efh::de_mmio(
                    efh.bhd_directory_tables[2].get(),
                    amd_physical_mode_mmio_size,
                ),
                None,
                None,
                None,
            ],
            Some(ProcessorGeneration::Naples) => [
                Efh::de_mmio(
                    efh.bhd_directory_tables[0].get(),
                    amd_physical_mode_mmio_size,
                ),
                None,
                None,
                None,
            ],
            None => [
                // allow all (used for example for overlap checking)
                efh.bhd_directory_table_milan().ok(),
                Efh::de_mmio(
                    efh.bhd_directory_tables[2].get(),
                    amd_physical_mode_mmio_size,
                ),
                Efh::de_mmio(
                    efh.bhd_directory_tables[1].get(),
                    amd_physical_mode_mmio_size,
                ),
                Efh::de_mmio(
                    efh.bhd_directory_tables[0].get(),
                    amd_physical_mode_mmio_size,
                ),
            ],
        };
        Ok(IntoIterator::into_iter(positions).flatten())
    }

    /// Return the directory matching PROCESSOR_GENERATION,
    /// or any directory if the former is None.
    /// Note: Either bhd_directory or bhd_combo_directory will succeed--but not both.
    pub fn bhd_directory(
        &self,
        processor_generation: Option<ProcessorGeneration>,
    ) -> Result<BhdDirectory> {
        let bhd_directory_table_location = self
            .bhd_directories(processor_generation)?
            .next()
            .ok_or(Error::BhdDirectoryHeaderNotFound)?;
        let directory = BhdDirectory::load(
            self.storage,
            bhd_directory_table_location,
            0,
            self.amd_physical_mode_mmio_size,
        )?;
        if directory.header.cookie != BhdDirectoryHeader::FIRST_LEVEL_COOKIE {
            return Err(Error::DirectoryTypeMismatch);
        }
        Ok(directory)
    }

    /// Return the directory matching PROCESSOR_GENERATION,
    /// or any directory if the former is None.
    /// Note: Either bhd_directory or bhd_combo_directory will succeed--but not both.
    pub fn bhd_combo_directory(
        &self,
        processor_generation: Option<ProcessorGeneration>,
    ) -> Result<ComboDirectory> {
        let bhd_directory_table_location = self
            .bhd_directories(processor_generation)?
            .next()
            .ok_or(Error::BhdDirectoryHeaderNotFound)?;
        let directory = ComboDirectory::load(
            self.storage,
            bhd_directory_table_location,
            0,
            self.amd_physical_mode_mmio_size,
        )?;
        if directory.header.cookie != ComboDirectoryHeader::BHD_COOKIE {
            return Err(Error::DirectoryTypeMismatch);
        }
        Ok(directory)
    }

    fn write_efh(&mut self) -> Result<()> {
        let mut buf: [u8; size_of::<Efh>()] = [0xFF; size_of::<Efh>()];
        if let Some(item) = header_from_collection_mut(&mut buf[..]) {
            *item = self.efh;
        }

        self.storage.erase_and_write_blocks(self.efh_beginning, &buf)?;
        Ok(())
    }

    pub fn spi_mode_bulldozer(&self) -> Result<Option<EfhBulldozerSpiMode>> {
        self.efh.spi_mode_bulldozer()
    }

    pub fn set_spi_mode_bulldozer(
        &mut self,
        value: Option<EfhBulldozerSpiMode>,
    ) {
        self.efh.set_spi_mode_bulldozer(value)
    }
    pub fn spi_mode_zen_naples(&self) -> Result<Option<EfhNaplesSpiMode>> {
        self.efh.spi_mode_zen_naples()
    }

    pub fn set_spi_mode_zen_naples(&mut self, value: Option<EfhNaplesSpiMode>) {
        self.efh.set_spi_mode_zen_naples(value)
    }

    pub fn spi_mode_zen_rome(&self) -> Result<Option<EfhRomeSpiMode>> {
        self.efh.spi_mode_zen_rome()
    }

    pub fn set_spi_mode_zen_rome(&mut self, value: Option<EfhRomeSpiMode>) {
        self.efh.set_spi_mode_zen_rome(value)
    }

    pub fn espi0_configuration(&self) -> Result<Option<EfhEspiConfiguration>> {
        self.efh.espi0_configuration()
    }

    pub fn set_espi0_configuration(
        &mut self,
        value: Option<EfhEspiConfiguration>,
    ) {
        self.efh.set_espi0_configuration(value)
    }

    pub fn espi1_configuration(&self) -> Result<Option<EfhEspiConfiguration>> {
        self.efh.espi1_configuration()
    }

    pub fn set_espi1_configuration(
        &mut self,
        value: Option<EfhEspiConfiguration>,
    ) {
        self.efh.set_espi1_configuration(value)
    }

    /// Create a directory but don't set it as the Efs main bhd directory.
    /// The idea is to use this also for creating a second level directory.
    pub fn create_bhd_directory(
        &mut self,
        cookie: [u8; 4],
        beginning: ErasableLocation,
        end: ErasableLocation,
        default_entry_address_mode: AddressMode,
        entries: &[BhdDirectoryEntry],
    ) -> Result<BhdDirectory> {
        assert_eq!(beginning.erasable_block_size(), end.erasable_block_size());
        match default_entry_address_mode {
            AddressMode::PhysicalAddress => {
                if !self.physical_address_mode() {
                    return Err(Error::DirectoryTypeMismatch);
                }
            }
            AddressMode::EfsRelativeOffset
            | AddressMode::DirectoryRelativeOffset => {
                if self.physical_address_mode() {
                    return Err(Error::DirectoryTypeMismatch);
                }
            }
            _ => return Err(Error::DirectoryTypeMismatch),
        }
        BhdDirectory::create(
            beginning.into(),
            0,
            default_entry_address_mode,
            cookie,
            self.amd_physical_mode_mmio_size,
            entries,
        )
    }
    pub fn set_main_bhd_directory(
        &mut self,
        directory: &BhdDirectory,
    ) -> Result<()> {
        let beginning = directory.beginning;
        if self
            .efh
            .compatible_with_processor_generation(ProcessorGeneration::Milan)
            || self.efh.compatible_with_processor_generation(
                ProcessorGeneration::Genoa,
            )
            || self.efh.compatible_with_processor_generation(
                ProcessorGeneration::Turin,
            )
        {
            self.efh.set_bhd_directory_table_milan(beginning);
        // FIXME: ensure that the others are unset?
        } else {
            self.efh.bhd_directory_tables[2].set(beginning);
            // FIXME: ensure that the others are unset?
        }
        self.write_efh()?;
        Ok(())
    }

    // Note: BEGINNING, END are coordinates (in Byte).
    pub fn create_psp_directory(
        &mut self,
        cookie: [u8; 4],
        beginning: ErasableLocation,
        end: ErasableLocation,
        default_entry_address_mode: AddressMode,
        entries: &[PspDirectoryEntry],
    ) -> Result<PspDirectory> {
        assert_eq!(beginning.erasable_block_size(), end.erasable_block_size());
        match default_entry_address_mode {
            AddressMode::PhysicalAddress => {
                if !self.physical_address_mode() {
                    return Err(Error::DirectoryTypeMismatch);
                }
            }
            AddressMode::EfsRelativeOffset
            | AddressMode::DirectoryRelativeOffset => {
                if self.physical_address_mode() {
                    return Err(Error::DirectoryTypeMismatch);
                }
            }
            _ => return Err(Error::DirectoryTypeMismatch),
        }
        let result = PspDirectory::create(
            beginning.into(),
            beginning.into(),
            default_entry_address_mode,
            cookie,
            self.amd_physical_mode_mmio_size,
            entries,
        )?;
        Ok(result)
    }
    pub fn set_main_psp_directory(
        &mut self,
        directory: &PspDirectory,
    ) -> Result<()> {
        let beginning = directory.beginning;
        // TODO: Boards older than Rome have 0xff at the top bits.  Depends on address_mode maybe.
        self.efh.set_psp_directory_table_location_zen(beginning);
        self.write_efh()?;
        Ok(())
    }
    pub fn psp_combo_subdirectory(
        &self,
        directory: &ComboDirectory,
        entry: &ComboDirectoryEntry,
    ) -> Result<PspDirectory> {
        let beginning = directory.payload_beginning(entry)?;
        PspDirectory::load(
            self.storage,
            beginning,
            directory.beginning, // TODO: verify.
            self.amd_physical_mode_mmio_size,
        )
    }
    pub fn bhd_combo_subdirectory(
        &self,
        directory: &ComboDirectory,
        entry: &ComboDirectoryEntry,
    ) -> Result<BhdDirectory> {
        let beginning = directory.payload_beginning(entry)?;
        BhdDirectory::load(
            self.storage,
            beginning,
            directory.beginning, // TODO: verify.
            self.amd_physical_mode_mmio_size,
        )
    }
    pub fn psp_subdirectory(
        &self,
        directory: &PspDirectory,
    ) -> Result<PspDirectory> {
        for entry in directory.entries() {
            if let Ok(PspDirectoryEntryType::SecondLevelDirectory) =
                entry.typ_or_err()
            {
                let beginning = directory.payload_beginning(&entry)?;
                return PspDirectory::load(
                    self.storage,
                    beginning,
                    beginning,
                    self.amd_physical_mode_mmio_size,
                );
            }
        }
        Err(Error::EntryNotFound)
    }
    pub fn bhd_subdirectory(
        &self,
        directory: &BhdDirectory,
    ) -> Result<BhdDirectory> {
        for entry in directory.entries() {
            if let Ok(BhdDirectoryEntryType::SecondLevelDirectory) =
                entry.typ_or_err()
            {
                let beginning = directory.payload_beginning(&entry)?;
                return BhdDirectory::load(
                    self.storage,
                    beginning,
                    beginning,
                    self.amd_physical_mode_mmio_size,
                );
            }
        }
        Err(Error::EntryNotFound)
    }
    /// Given a PSP directory, find a second level BHD directory (if any)
    /// that is a payload of the former and return that.
    pub fn psp_ab_bhd_subdirectory(
        &self,
        directory: &PspDirectory,
    ) -> Result<BhdDirectory> {
        for entry in directory.entries() {
            if let Ok(PspDirectoryEntryType::SecondLevelBhdDirectory) =
                entry.typ_or_err()
            {
                let beginning = directory.payload_beginning(&entry)?;
                return BhdDirectory::load(
                    self.storage,
                    beginning,
                    directory.beginning,
                    self.amd_physical_mode_mmio_size,
                );
            }
        }
        Err(Error::EntryNotFound)
    }
    pub fn create_psp_subdirectory(
        &self,
        directory: &mut PspDirectory,
        beginning: ErasableLocation,
        end: ErasableLocation,
        amd_physical_mode_mmio_size: Option<u32>,
        entries: &[PspDirectoryEntry],
    ) -> Result<PspDirectory> {
        if directory.header.cookie() != PspDirectoryHeader::FIRST_LEVEL_COOKIE {
            return Err(Error::DirectoryTypeMismatch);
        }
        // TODO
        // if let Err(Error::EntryNotFound) = self.psp_subdirectory(directory) {
        directory.add_entry_direct(&PspDirectoryEntry::new_payload(
            directory.directory_address_mode(),
            PspDirectoryEntryType::SecondLevelDirectory,
            Some(ErasableLocation::extent(beginning, end)),
            Some(ValueOrLocation::EfsRelativeOffset(beginning.into())),
        )?)?;
        PspDirectory::create(
            beginning.into(),
            beginning.into(),
            directory.directory_address_mode,
            *b"$PL2",
            amd_physical_mode_mmio_size,
            entries,
        )
        // } else {
        //     Err(Error::Duplicate)
        // }
    }

    pub fn create_second_level_psp_directory(
        &self,
        beginning: ErasableLocation,
        end: ErasableLocation,
        entries: &[PspDirectoryEntry],
    ) -> Result<PspDirectory> {
        let mut psp_directory = self.psp_directory()?;
        self.create_psp_subdirectory(
            &mut psp_directory,
            beginning,
            end,
            self.amd_physical_mode_mmio_size,
            entries,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{EfhBulldozerSpiMode, EfhNaplesSpiMode, EfhRomeSpiMode};
    use crate::Efh;
    use crate::Efs;
    use crate::Error;
    use crate::flash;
    use crate::ondisk::{
        SpiFastSpeedNew, SpiNaplesMicronMode, SpiReadMode, SpiRomeMicronMode,
    };
    use flash::{ErasableLocation, FlashAlign, FlashRead, FlashWrite};

    struct Storage {}
    impl FlashAlign for Storage {
        fn erasable_block_size(&self) -> usize {
            16
        }
    }
    impl FlashRead for Storage {
        fn read_exact(
            &self,
            _: u32,
            _: &mut [u8],
        ) -> core::result::Result<(), flash::Error> {
            todo!()
        }
    }
    impl FlashWrite for Storage {
        fn erase_block(
            &self,
            _: ErasableLocation,
        ) -> core::result::Result<(), flash::Error> {
            todo!()
        }
        fn erase_and_write_block(
            &self,
            _: ErasableLocation,
            _: &[u8],
        ) -> core::result::Result<(), flash::Error> {
            todo!()
        }
    }

    fn setup_efs_test(storage: &Storage) -> Efs<Storage> {
        let efh_beginning = storage.erasable_location(0).unwrap();
        Efs {
            amd_physical_mode_mmio_size: None,
            efh: Efh::default(),
            efh_beginning,
            storage: &storage,
        }
    }

    #[test]
    fn test_spi_mode_all_off() {
        let storage = Storage {};
        let mut setup = setup_efs_test(&storage);
        assert!(setup.spi_mode_bulldozer().unwrap().is_none());
        assert!(setup.spi_mode_zen_naples().unwrap().is_none());
        assert!(setup.spi_mode_zen_rome().unwrap().is_none());
        setup.set_spi_mode_bulldozer(None);
        setup.set_spi_mode_zen_naples(None);
        setup.set_spi_mode_zen_rome(None);
        assert!(setup.spi_mode_bulldozer().unwrap().is_none());
        assert!(setup.spi_mode_zen_naples().unwrap().is_none());
        assert!(setup.spi_mode_zen_rome().unwrap().is_none());
    }

    #[test]
    fn test_spi_mode_bulldozer() -> Result<(), Error> {
        let storage = Storage {};
        let mut setup = setup_efs_test(&storage);
        let spi_mode = EfhBulldozerSpiMode {
            read_mode: SpiReadMode::Dual112,
            fast_speed_new: SpiFastSpeedNew::_33_33MHz,
        };
        setup.set_spi_mode_bulldozer(Some(spi_mode));
        assert!(
            setup.spi_mode_bulldozer()?.unwrap().read_mode
                == SpiReadMode::Dual112
        );
        assert!(setup.spi_mode_zen_naples()?.is_none());
        assert!(setup.spi_mode_zen_rome()?.is_none());
        Ok(())
    }

    #[test]
    fn test_spi_mode_zen_naples() -> Result<(), Error> {
        let storage = Storage {};
        let mut setup = setup_efs_test(&storage);
        let spi_mode = EfhNaplesSpiMode {
            read_mode: SpiReadMode::Dual112,
            fast_speed_new: SpiFastSpeedNew::_33_33MHz,
            micron_mode: SpiNaplesMicronMode::DummyCycle,
        };
        // Explicitly set for Naples compatibility
        setup.efh.efs_generations = 0xffffffff.into();
        setup.set_spi_mode_zen_naples(Some(spi_mode));
        assert!(setup.spi_mode_bulldozer()?.is_none());
        assert!(
            setup.spi_mode_zen_naples()?.unwrap().read_mode
                == SpiReadMode::Dual112
        );
        assert!(setup.spi_mode_zen_rome()?.is_none());
        Ok(())
    }

    #[test]
    fn test_spi_mode_zen_rome() -> Result<(), Error> {
        let storage = Storage {};
        let mut setup = setup_efs_test(&storage);
        let spi_mode = EfhRomeSpiMode {
            read_mode: SpiReadMode::Dual112,
            fast_speed_new: SpiFastSpeedNew::_33_33MHz,
            micron_mode: SpiRomeMicronMode::SupportMicron,
        };
        setup.set_spi_mode_zen_rome(Some(spi_mode));
        assert!(setup.spi_mode_bulldozer()?.is_none());
        assert!(setup.spi_mode_zen_naples()?.is_none());
        assert!(
            setup.spi_mode_zen_rome()?.unwrap().read_mode
                == SpiReadMode::Dual112
        );
        Ok(())
    }
}
