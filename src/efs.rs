use crate::amdfletcher32::AmdFletcher32;
use crate::ondisk::header_from_collection;
use crate::ondisk::header_from_collection_mut;
pub use crate::ondisk::ProcessorGeneration;
use crate::ondisk::EFH_POSITION;
use crate::ondisk::{
	mmio_decode, mmio_encode, AddressMode, BhdDirectoryEntry,
	BhdDirectoryEntryAttrs, BhdDirectoryEntryType, BhdDirectoryHeader,
	DirectoryAdditionalInfo, DirectoryEntry, DirectoryHeader, Efh,
	PspDirectoryEntry, PspDirectoryEntryAttrs, PspDirectoryEntryType,
	PspDirectoryHeader, EfhRomeSpiMode, EfhNaplesSpiMode, EfhBulldozerSpiMode,
	ComboDirectoryHeader, ComboDirectoryEntry, ValueOrLocation,
};
use crate::types::Error;
use crate::types::Result;
use amd_flash::{ErasableLocation, FlashRead, FlashWrite, Location};
use core::convert::TryFrom;
use core::convert::TryInto;
use core::marker::PhantomData;
use core::mem::size_of;
use zerocopy::AsBytes;
use zerocopy::FromBytes;

pub struct DirectoryIter<
	'a,
	Item,
	T: FlashRead<ERASABLE_BLOCK_SIZE>,
	const ERASABLE_BLOCK_SIZE: usize,
> {
	storage: &'a T,
	directory_address_mode: AddressMode,
	current: Location, // pointer to current item (directory entry)
	end: Location,
	total_entries: u32,
	index: u32,
	_item: PhantomData<Item>,
}

impl<
		'a,
		Item: DirectoryEntry + FromBytes + Copy,
		T: FlashRead<ERASABLE_BLOCK_SIZE>,
		const ERASABLE_BLOCK_SIZE: usize,
	> Iterator for DirectoryIter<'a, Item, T, ERASABLE_BLOCK_SIZE>
{
	type Item = Item;
	fn next(&mut self) -> Option<<Self as Iterator>::Item> {
		if self.index < self.total_entries {
			let mut buf: [u8; ERASABLE_BLOCK_SIZE] =
				[0xff; ERASABLE_BLOCK_SIZE];
			let buf = &mut buf[.. size_of::<Item>()];
			self.storage.read_exact(self.current, buf).ok()?;
			let result = header_from_collection::<Item>(buf)?; // TODO: Check for errors
			self.current = self.current.checked_add(size_of::<Item>() as u32)?;
			self.index += 1;
			let q = *result;
			Some(q)
		} else {
			None
		}
	}
}

// TODO: split into Directory and DirectoryContents (disjunct) if requested in additional_info.
pub struct Directory<
	'a,
	MainHeader,
	Item: FromBytes,
	T: FlashRead<ERASABLE_BLOCK_SIZE> + FlashWrite<ERASABLE_BLOCK_SIZE>,
	Attrs: Sized,
	const _SPI_BLOCK_SIZE: usize,
	const ERASABLE_BLOCK_SIZE: usize,
	const MainHeaderSize: usize,
> {
	storage: &'a T,
	location: Location, // ideally ErasableLocation<ERASABLE_BLOCK_SIZE>--but that's impossible with AMD-generated images.
	directory_address_mode: AddressMode,
	header: MainHeader,
	directory_headers_size: u32,
	// On AMD, this field specifies how much of the memory area under
	// address 2**32 (towards lower addresses) is used to memory-map
	// Flash. This is used in order to store pointers to other
	// areas on Flash (with ValueOrLocation::PhysicalAddress).
	amd_physical_mode_mmio_size: Option<u32>,
	_attrs: PhantomData<Attrs>,
	_item: PhantomData<Item>,
}

impl<
		'a,
		MainHeader: Copy + DirectoryHeader + FromBytes + AsBytes + Default,
		Item: Copy + FromBytes + AsBytes + DirectoryEntry + core::fmt::Debug,
		T: 'a
			+ FlashRead<ERASABLE_BLOCK_SIZE>
			+ FlashWrite<ERASABLE_BLOCK_SIZE>,
		Attrs: Sized,
		const SPI_BLOCK_SIZE: usize,
		const ERASABLE_BLOCK_SIZE: usize,
		const MainHeaderSize: usize,
	>
	Directory<'a, MainHeader, Item, T, Attrs, SPI_BLOCK_SIZE, ERASABLE_BLOCK_SIZE, MainHeaderSize>
{
	const SPI_BLOCK_SIZE: usize = SPI_BLOCK_SIZE;
	const MAX_DIRECTORY_HEADERS_SIZE: u32 = SPI_BLOCK_SIZE as u32; // AMD says 0x400; but then good luck with modifying the first entry payload without clobbering the directory that comes right before it.
	const MAX_DIRECTORY_ENTRIES: usize = ((Self::MAX_DIRECTORY_HEADERS_SIZE
		as usize) - size_of::<MainHeader>(
	)) / size_of::<Item>();

	pub fn header(&self) -> MainHeader {
		self.header
	}
	pub fn directory_address_mode(&self) -> AddressMode {
		self.directory_address_mode
	}

	fn minimal_directory_headers_size(total_entries: u32) -> Result<u32> {
		Ok(size_of::<MainHeader>()
			.checked_add(
				size_of::<Item>()
					.checked_mul(total_entries as usize)
					.ok_or(Error::DirectoryRangeCheck)?,
			)
			.ok_or(Error::DirectoryRangeCheck)?
			.try_into()
			.map_err(|_| Error::DirectoryRangeCheck)?)
	}

	/// Note: Caller has to check whether it is the right cookie (possibly afterwards)!
	pub fn load(
		storage: &'a T,
		location: Location,
		amd_physical_mode_mmio_size: Option<u32>,
	) -> Result<Self> {
		let mut buf: [u8; MainHeaderSize] =
			[0xff; MainHeaderSize];
		assert!(MainHeaderSize == size_of::<MainHeader>()); // TODO: move to compile-time
		storage.read_exact(location, &mut buf)?;
		match header_from_collection::<MainHeader>(&buf[..]) {
			Some(header) => {
				let cookie = header.cookie();
				if cookie == *b"$PSP" ||
					cookie == *b"$PL2" || cookie == *b"$BHD" ||
					cookie == *b"$BL2"
				{
					let contents_base = DirectoryAdditionalInfo::try_from_unit(
						header.additional_info().base_address(),
					)
					.unwrap();
					let directory_address_mode = header.additional_info().address_mode();
					match directory_address_mode {
						AddressMode::PhysicalAddress | AddressMode::EfsRelativeOffset | AddressMode::DirectoryRelativeOffset => {
						}
						_ => {
							return Err(Error::DirectoryTypeMismatch)
						}
					}
					Ok(Self {
						storage,
						location,
						directory_address_mode,
						header: *header,
						directory_headers_size:
							if contents_base == 0 {
								// Note: This means the number of entries cannot be changed (without moving ALL the payload--which we don't want).
								Self::minimal_directory_headers_size(
									header.total_entries(),
								)?
							} else {
								// Note: This means the number of entries can be changed even when payload is already present.
								// TODO: This is maybe still bad since we are only guaranteed 0x400 B of space, which is less than the following:
								Self::MAX_DIRECTORY_HEADERS_SIZE
							},
						amd_physical_mode_mmio_size,
						_attrs: PhantomData,
						_item: PhantomData,
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
		storage: &'a T,
		beginning: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		end: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		cookie: [u8; 4],
		amd_physical_mode_mmio_size: Option<u32>,
	) -> Result<Self> {
		// FIXME: handle directory_address_mode
		let mut buf: [u8; ERASABLE_BLOCK_SIZE] =
			[0xFF; ERASABLE_BLOCK_SIZE];
		match header_from_collection_mut::<MainHeader>(&mut buf[..]) {
			Some(item) => {
				*item = MainHeader::default();
				item.set_cookie(cookie);
				// Note: It is valid that ERASABLE_BLOCK_SIZE <= SPI_BLOCK_SIZE.
				if Self::SPI_BLOCK_SIZE % ERASABLE_BLOCK_SIZE !=
					0
				{
					return Err(Error::DirectoryRangeCheck);
				}
				let additional_info = DirectoryAdditionalInfo::new()
					.with_max_size_checked(
						DirectoryAdditionalInfo::try_into_unit(
							ErasableLocation::<ERASABLE_BLOCK_SIZE>::extent(
								beginning, end,
							)
							.try_into()
							.map_err(|_| Error::DirectoryRangeCheck)?,
						)
						.ok_or_else(|| Error::DirectoryRangeCheck)?,
					)
					.map_err(|_| Error::DirectoryRangeCheck)?
					.with_spi_block_size_checked(
						DirectoryAdditionalInfo::try_into_unit(
							Self::SPI_BLOCK_SIZE,
						)
						.ok_or_else(|| Error::DirectoryRangeCheck)?
						.try_into()
						.map_err(|_| Error::DirectoryRangeCheck)?,
					)
					.map_err(|_| Error::DirectoryRangeCheck)?
					// We put the actual payload at some distance from the directory, but still close-by--in order to be able to grow the directory later (when there's already payload)
					.with_base_address(
						DirectoryAdditionalInfo::try_into_unit(
							(Location::from(beginning)
								.checked_add(Self::MAX_DIRECTORY_HEADERS_SIZE)
								.ok_or_else(|| Error::DirectoryRangeCheck)?)
							.try_into()
							.map_err(|_| Error::DirectoryRangeCheck)?,
						)
						.ok_or_else(|| Error::DirectoryRangeCheck)?
						.try_into()
						.map_err(|_| Error::DirectoryRangeCheck)?,
					)
					.with_address_mode(AddressMode::EfsRelativeOffset);
				item.set_additional_info(additional_info);
				storage.erase_and_write_blocks(beginning, &buf)?;
				Self::load(
					storage,
					Location::from(beginning),
					amd_physical_mode_mmio_size,
				)
			}
			None => Err(Error::Marshal),
		}
	}
	/// Updates the main header checksum.  Also updates total_entries (in the same header) to TOTAL_ENTRIES.
	/// Precondition: Since the checksum is over the entire directory, that means that all the directory entries needs to be correct already.
	fn update_main_header(&mut self, total_entries: u32) -> Result<()> {
		let old_total_entries = self.header.total_entries();
		let flash_input_block_size =
			Self::minimal_directory_headers_size(total_entries)?;
		let mut flash_input_block_address: Location =
			self.location.into();
		let mut buf = [0xFFu8; ERASABLE_BLOCK_SIZE];
		let mut flash_input_block_remainder = flash_input_block_size;
		let mut checksummer = AmdFletcher32::new();
		// Good luck with that: assert!(((flash_input_block_size as usize) % ERASABLE_BLOCK_SIZE) == 0);
		let mut skip: usize = 12; // Skip fields "signature", "checksum" and "total_entries"
			  // Note: total_entries on the flash has not been updated yet--so manually account for it.
		checksummer.update(&[
			(total_entries & 0xffff) as u16,
			(total_entries >> 16) as u16,
		]);
		while flash_input_block_remainder > 0 {
			self.storage.read_exact(
				flash_input_block_address,
				&mut buf,
			)?;
			let mut count = ERASABLE_BLOCK_SIZE as u32;
			if count > flash_input_block_remainder {
				count = flash_input_block_remainder;
			}
			assert!(count % 2 == 0);
			assert!(count as usize >= skip);
			let block = &buf[skip .. count as usize].chunks(2).map(
				|bytes| {
					u16::from_le_bytes(
						bytes.try_into().unwrap(),
					)
				},
			);
			skip = 0;
			// TODO: Optimize performance
			block.clone().for_each(|item: u16| {
				checksummer.update(&[item])
			});
			flash_input_block_remainder -= count;
			flash_input_block_address = flash_input_block_address
				.checked_add(count)
				.ok_or(Error::DirectoryRangeCheck)?;
		}

		let checksum = checksummer.value().value();
		self.header.set_checksum(checksum);
		let flash_input_block_address = ErasableLocation::<
			ERASABLE_BLOCK_SIZE,
		>::try_from(self.location)?;
		self.storage.read_erasable_block(
			flash_input_block_address,
			&mut buf,
		)?;
		// Write main header--and at least the directory entries that are "in the way"
		match header_from_collection_mut::<MainHeader>(
			&mut buf[.. size_of::<MainHeader>()],
		) {
			Some(item) => {
				self.header.set_total_entries(total_entries); // Note: reverted on error--see below
				*item = self.header;
			}
			None => {
				return Err(Error::DirectoryRangeCheck);
			}
		}
		match self
			.storage
			.erase_and_write_blocks(flash_input_block_address, &buf)
		{
			Ok(()) => Ok(()),
			Err(e) => {
				self.header
					.set_total_entries(old_total_entries);
				Err(Error::from(e))
			}
		}
	}
	fn directory_beginning(&self) -> Location {
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
		let contents_base = DirectoryAdditionalInfo::try_from_unit(
			additional_info.base_address(),
		)
		.unwrap();
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
		let size: u32 = DirectoryAdditionalInfo::try_from_unit(
			additional_info.max_size(),
		)
		.unwrap()
		.try_into()
		.unwrap();
		let location = Location::from(self.contents_beginning());
		// Assumption: SIZE includes the size of the main directory header.
		// FIXME: What happens in the case contents_base != 0 ?  I think then it doesn't include it.
		let end = location + size - self.directory_headers_size; // FIXME: range check
		ErasableLocation::<ERASABLE_BLOCK_SIZE>::try_from(end).unwrap()
	}
	pub fn entries(&self) -> DirectoryIter<Item, T, ERASABLE_BLOCK_SIZE> {
		DirectoryIter::<Item, T, ERASABLE_BLOCK_SIZE> {
			storage: self.storage,
			directory_address_mode: self.directory_address_mode,
			current: self.directory_beginning(),
			end: self.directory_end(), // actually, much earlier--this here is the allocation, not the actual size
			total_entries: self.header.total_entries(),
			index: 0u32,
			_item: PhantomData,
		}
	}

	pub fn location_of_source(&self, source: ValueOrLocation, entry_base_location: Location) -> Result<Location> {
		match source {
			ValueOrLocation::Value(_) => {
				Err(Error::DirectoryTypeMismatch)
			}
			ValueOrLocation::PhysicalAddress(y) => { // or unknown
				if let Some(amd_physical_mode_mmio_size) = self.amd_physical_mode_mmio_size {
					match mmio_decode(y, amd_physical_mode_mmio_size) {
						Ok(x) => Ok(x),
						Err(_) => {
							// Older Zen models also allowed a flash offset here.
							// So allow that as well.
							// TODO: Maybe thread through the processor
							// generation and only do on Naples.
							if y < amd_physical_mode_mmio_size {
								Ok(y)
							} else {
								return Err(Error::EntryTypeMismatch)
							}
						}
					}
				} else {
					return Err(Error::EntryTypeMismatch)
				}
			}
			ValueOrLocation::EfsRelativeOffset(x) => {
				Ok(x)
			}
			ValueOrLocation::DirectoryRelativeOffset(y) => {
				Ok(self.location.checked_add(y).ok_or(Error::DirectoryPayloadRangeCheck)?)
			}
			ValueOrLocation::EntryRelativeOffset(y) => {
				Ok(y.checked_add(entry_base_location).ok_or(Error::DirectoryPayloadRangeCheck)?)
			}
		}
	}

	pub(crate) fn find_payload_empty_slot(
		&self,
		size: u32,
	) -> Result<ErasableLocation<ERASABLE_BLOCK_SIZE>> {
		let entries = self.entries();
		let contents_beginning =
			Location::from(self.contents_beginning()) as u64;
		let contents_end = Location::from(self.contents_end()) as u64;
		let mut frontier: u64 = contents_beginning;
		// TODO: Also use gaps between entries
		let mut entry_offset = 0u32;
		for ref entry in entries {
			let size = match entry.size() {
				None => {
					entry_offset = entry_offset.checked_add(size_of::<Item>().try_into().map_err(|_| Error::DirectoryPayloadRangeCheck)?).ok_or(Error::DirectoryPayloadRangeCheck)?;
					continue
				}
				Some(x) => x as u64,
			};
			let x = self.location_of_source(entry.source(self.directory_address_mode)?, self.location.checked_add(entry_offset).ok_or(Error::DirectoryPayloadRangeCheck)? /* FIXME */)?;
			let x = u64::from(x);
			if x >= contents_beginning &&
				x + size <= contents_end
			{
				let new_frontier = x + size; // FIXME bounds check
				if new_frontier > frontier {
					frontier = new_frontier;
				}
			}

			entry_offset = entry_offset.checked_add(size_of::<Item>().try_into().map_err(|_| Error::DirectoryPayloadRangeCheck)?).ok_or(Error::DirectoryPayloadRangeCheck)?;
		}
		let frontier: Location = frontier
			.try_into()
			.map_err(|_| Error::DirectoryPayloadRangeCheck)?;
		let frontier_end = frontier
			.checked_add(size)
			.ok_or(Error::DirectoryPayloadRangeCheck)?;
		let (_, frontier) =
			self.storage.grow_to_erasable_block(frontier, frontier);
		Ok(frontier.try_into()?)
	}

	pub(crate) fn write_directory_entry(
		&mut self,
		directory_entry_position: Location,
		entry: &Item,
	) -> Result<()> {
		let mut buf: [u8; ERASABLE_BLOCK_SIZE] =
			[0xFF; ERASABLE_BLOCK_SIZE];
		let buf_index = (directory_entry_position as usize) %
			ERASABLE_BLOCK_SIZE;
		let beginning =
			directory_entry_position - (buf_index as Location); // align
		let beginning =
			beginning.try_into().map_err(|_| Error::Misaligned)?;
		self.storage.read_erasable_block(beginning, &mut buf)?;
		// FIXME: what if this straddles two different blocks?
		match header_from_collection_mut::<Item>(
			&mut buf[buf_index .. buf_index + size_of::<Item>()],
		) {
			Some(item) => {
				*item = *entry;
				self.storage.erase_and_write_blocks(
					beginning, &buf,
				)?;
			}
			None => {
				return Err(Error::DirectoryRangeCheck);
			}
		}
		Ok(())
	}
	/// PAYLOAD_POSITION: If you have a position on the Flash that you want this fn to use, specify it.  Otherwise, one will be calculated.
	/// ENTRY: The directory entry to put.  Note that we WILL set entry.source = (maybe calculated) payload_position in the copy we save on Flash.
	/// Result: Location where to put the payload.
	pub(crate) fn add_entry(
		&mut self,
		payload_position: Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>,
		entry: &Item,
	) -> Result<Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>> {
		let total_entries = self
			.header
			.total_entries()
			.checked_add(1)
			.ok_or(Error::DirectoryRangeCheck)?;
		if Self::minimal_directory_headers_size(total_entries)? <=
			self.directory_headers_size
		{
			// there's still space for the directory entry
			let result: Option<
				ErasableLocation<ERASABLE_BLOCK_SIZE>,
			> = match entry.size() {
				None => None,
				Some(size) => {
					if size == 0 {
						None
					} else {
						let beginning =
							match payload_position {
								Some(x) => x,
								None => self
									.find_payload_empty_slot(
									size,
								)?,
							};
						Some(beginning)
					}
				}
			};
			let mut entry = *entry;
			match result {
				None => {}
				Some(beginning) => {
					let beginning = Location::from(beginning);
					entry.set_source(self.directory_address_mode, match self.directory_address_mode {
						AddressMode::PhysicalAddress => {
							ValueOrLocation::PhysicalAddress(mmio_encode(Location::from(beginning).into(), self.amd_physical_mode_mmio_size)?)
						},
						AddressMode::EfsRelativeOffset => {
							ValueOrLocation::EfsRelativeOffset(beginning)
						},
						AddressMode::DirectoryRelativeOffset => {
							// Note: could be overridden by SOURCE--not sure that we want that here.
							ValueOrLocation::DirectoryRelativeOffset(beginning.checked_sub(self.location).ok_or(Error::DirectoryPayloadRangeCheck)?)
						}
						AddressMode::EntryRelativeOffset => { // not allowed
							return Err(Error::DirectoryTypeMismatch)
						}
					})?;
				}
			}
			let location: Location = self.location.into();
			self.write_directory_entry(
				location +
					Self::minimal_directory_headers_size(
						self.header.total_entries(),
					)?,
				&entry,
			)?; // FIXME check bounds
			self.update_main_header(total_entries)?;
			Ok(result)
		} else {
			Err(Error::DirectoryRangeCheck)
		}
	}

	/// Repeatedly calls GENERATE_CONTENTS, which fills it's passed buffer as much as possible, as long as the total <= SIZE.
	/// (GENERATE_CONTENTS can only return a number (of u8 that are filled in BUF) smaller than the possible size if the blob is ending)
	/// Then ADD_PAYLOAD stores all that starting at PAYLOAD_POSITION, or, if that is not present, the next available location in the directory.
	/// If what we stored so far is less than SIZE, we store 0xFF for the remainder.
	/// It is an error to add a payload that is bigger than SIZE.
	/// The reason we only allows full-size buffer results from each callback because we will be erasing a flash block and then writing the callback result to it.
	pub(crate) fn add_payload(
		&mut self,
		payload_position: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		size: usize,
		generate_contents: &mut dyn FnMut(&mut [u8]) -> Result<usize>,
	) -> Result<()> {
		let mut buf: [u8; ERASABLE_BLOCK_SIZE] =
			[0xFF; ERASABLE_BLOCK_SIZE];
		let mut remaining_size = size;
		let mut payload_position = payload_position;
		let contents_beginning =
			Location::from(self.contents_beginning());
		let contents_end = Location::from(self.contents_end());
		let payload_meant_inside_contents = Location::from(
			payload_position,
		) >= contents_beginning &&
			Location::from(payload_position) <= contents_end;
		let mut padding = false;
		while remaining_size > 0 {
			let count = if padding {
				0
			} else {
				let count = generate_contents(&mut buf)?;
				if count == 0 {
					// EOF
					padding = true;
				}
				if count > remaining_size {
					return Err(Error::DirectoryPayloadRangeCheck);
				}
				count
			};
			// pad with 0xFF
			if count < buf.len() {
				for i in count .. buf.len() {
					buf[i] = 0xFF;
				}
			}
			let count = buf.len();

			let end = (Location::from(payload_position) as usize)
				.checked_add(count)
				.ok_or(Error::DirectoryPayloadRangeCheck)?;
			if payload_meant_inside_contents &&
				end > contents_end as usize
			{
				return Err(Error::DirectoryPayloadRangeCheck);
			}
			remaining_size = remaining_size.saturating_sub(count);
			self.storage.erase_and_write_blocks(
				payload_position,
				&buf,
			)?;
			payload_position = payload_position.advance(count)?;
			if count < buf.len() {
				padding = true;
			}
		}
		Ok(())
	}
}

pub type PspDirectory<'a, T, const ERASABLE_BLOCK_SIZE: usize> = Directory<
	'a,
	PspDirectoryHeader,
	PspDirectoryEntry,
	T,
	PspDirectoryEntryAttrs,
	0x3000,
	ERASABLE_BLOCK_SIZE,
	{ size_of::<PspDirectoryHeader>() },
>;
pub type BhdDirectory<'a, T, const ERASABLE_BLOCK_SIZE: usize> = Directory<
	'a,
	BhdDirectoryHeader,
	BhdDirectoryEntry,
	T,
	BhdDirectoryEntryAttrs,
	0x1000,
	ERASABLE_BLOCK_SIZE,
	{ size_of::<BhdDirectoryHeader>() },
>;
pub type ComboDirectory<'a, T, const ERASABLE_BLOCK_SIZE: usize> = Directory<
	'a,
	ComboDirectoryHeader,
	ComboDirectoryEntry,
	T,
	(),
	0x1000,
	ERASABLE_BLOCK_SIZE,
	{ size_of::<ComboDirectoryHeader>() },
>;

impl<
		'a,
		T: 'a
			+ FlashRead<ERASABLE_BLOCK_SIZE>
			+ FlashWrite<ERASABLE_BLOCK_SIZE>,
		const SPI_BLOCK_SIZE: usize,
		const ERASABLE_BLOCK_SIZE: usize,
	>
	Directory<
		'a,
		PspDirectoryHeader,
		PspDirectoryEntry,
		T,
		PspDirectoryEntryAttrs,
		SPI_BLOCK_SIZE,
		ERASABLE_BLOCK_SIZE,
		{ size_of::<PspDirectoryHeader>() },
	>
{
	// Note: Function is crate-private because there's no overlap checking
	pub(crate) fn create_subdirectory(
		&mut self,
		beginning: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		end: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		amd_physical_mode_mmio_size: Option<u32>,
	) -> Result<Self> {
		// Find existing SecondLevelDirectory, error out if found.
		let entries = self.entries();
		for entry in entries {
			match entry.type_or_err() {
				Ok(PspDirectoryEntryType::SecondLevelDirectory) => {
					return Err(Error::Duplicate);
				}
				_ => { // maybe just unknown
				}
			}
		}
		self.add_entry(
			beginning.into(),
			&PspDirectoryEntry::new_payload(
				&PspDirectoryEntryAttrs::new()
					.with_type_(PspDirectoryEntryType::SecondLevelDirectory),
				ErasableLocation::<ERASABLE_BLOCK_SIZE>::extent(beginning, end),
				beginning.into(),
			)?,
		)?;
		Self::create(
			self.directory_address_mode,
			self.storage,
			beginning,
			end,
			*b"$PL2",
			amd_physical_mode_mmio_size,
		)
	}

	// FIXME: Type-check
	pub fn add_value_entry(
		&mut self,
		attrs: &PspDirectoryEntryAttrs,
		value: u64,
	) -> Result<()> {
		match self.add_entry(
			None,
			&PspDirectoryEntry::new_value(attrs, value),
		)? {
			None => Ok(()),
			_ => Err(Error::EntryTypeMismatch),
		}
	}

	pub fn add_blob_entry(
		&mut self,
		payload_position: Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>,
		attrs: &PspDirectoryEntryAttrs,
		size: u32,
		iterative_contents: &mut dyn FnMut(&mut [u8]) -> Result<usize>,
	) -> Result<ErasableLocation<ERASABLE_BLOCK_SIZE>> {
		let xpayload_position = self.add_entry(
			payload_position,
			&PspDirectoryEntry::new_payload(
				attrs,
				size,
				match payload_position {
					None => 0,
					Some(x) => x.into(),
				},
			)?,
		)?;
		match xpayload_position {
			None => Err(Error::EntryTypeMismatch),
			Some(pos) => {
				self.add_payload(
					pos,
					size as usize,
					iterative_contents,
				)?;
				Ok(pos)
			}
		}
	}
}

impl<
		'a,
		T: 'a
			+ FlashRead<ERASABLE_BLOCK_SIZE>
			+ FlashWrite<ERASABLE_BLOCK_SIZE>,
		const SPI_BLOCK_SIZE: usize,
		const ERASABLE_BLOCK_SIZE: usize,
	>
	Directory<
		'a,
		BhdDirectoryHeader,
		BhdDirectoryEntry,
		T,
		BhdDirectoryEntryAttrs,
		SPI_BLOCK_SIZE,
		ERASABLE_BLOCK_SIZE,
		{ size_of::<BhdDirectoryHeader>() },
	>
{
	// Note: Function is crate-private because there's no overlap checking
	pub(crate) fn create_subdirectory(
		&mut self,
		beginning: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		end: ErasableLocation<ERASABLE_BLOCK_SIZE>,
	) -> Result<Self> {
		// Find existing SecondLevelDirectory, error out if found.
		let entries = self.entries();
		for entry in entries {
			match entry.type_or_err() {
				Ok(BhdDirectoryEntryType::SecondLevelDirectory) => {
					return Err(Error::Duplicate);
				}
				_ => { // maybe just unknown type.
				}
			}
		}
		self.add_entry(
			beginning.into(),
			&BhdDirectoryEntry::new_payload(
				&BhdDirectoryEntryAttrs::new()
					.with_type_(BhdDirectoryEntryType::SecondLevelDirectory),
				ErasableLocation::<ERASABLE_BLOCK_SIZE>::extent(beginning, end),
				beginning.into(),
				None,
			)?,
		)?;
		Self::create(
			self.directory_address_mode,
			&mut self.storage,
			beginning,
			end,
			*b"$BL2",
			self.amd_physical_mode_mmio_size,
		)
	}
	pub(crate) fn add_entry_with_destination(
		&mut self,
		payload_position: Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>,
		attrs: &BhdDirectoryEntryAttrs,
		size: u32,
		destination_location: u64,
	) -> Result<Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>> {
		self.add_entry(
			payload_position,
			&BhdDirectoryEntry::new_payload(
				attrs,
				size,
				0,
				Some(destination_location),
			)?,
		)
	}

	pub fn add_apob_entry(
		&mut self,
		payload_position: Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>,
		type_: BhdDirectoryEntryType,
		ram_destination_address: u64,
	) -> Result<()> {
		let attrs = BhdDirectoryEntryAttrs::new().with_type_(type_);
		match self.add_entry_with_destination(
			payload_position,
			&attrs,
			0,
			ram_destination_address,
		)? {
			None => Ok(()),
			_ => Err(Error::EntryTypeMismatch),
		}
	}

	pub fn add_blob_entry(
		&mut self,
		payload_position: Option<ErasableLocation<ERASABLE_BLOCK_SIZE>>,
		attrs: &BhdDirectoryEntryAttrs,
		size: u32,
		destination_location: Option<u64>,
		iterative_contents: &mut dyn FnMut(&mut [u8]) -> Result<usize>,
	) -> Result<ErasableLocation<ERASABLE_BLOCK_SIZE>> {
		let xpayload_position = self.add_entry(
			payload_position,
			&BhdDirectoryEntry::new_payload(
				attrs,
				size,
				match payload_position {
					None => 0,
					Some(x) => x.into(),
				},
				destination_location,
			)?,
		)?;
		match xpayload_position {
			None => Err(Error::EntryTypeMismatch),
			Some(pos) => {
				self.add_payload(
					pos,
					size as usize,
					iterative_contents,
				)?;
				Ok(pos)
			}
		}
	}
}

pub struct EfhBhdsIterator<
	'a,
	T: FlashRead<ERASABLE_BLOCK_SIZE> + FlashWrite<ERASABLE_BLOCK_SIZE>,
	const ERASABLE_BLOCK_SIZE: usize,
> {
	storage: &'a T,
	physical_address_mode: bool,
	positions: [u32; 4], // 0xffff_ffff: invalid
	index_into_positions: usize,
	amd_physical_mode_mmio_size: Option<u32>,
}

impl<
		'a,
		T: FlashRead<ERASABLE_BLOCK_SIZE>
			+ FlashWrite<ERASABLE_BLOCK_SIZE>,
		const ERASABLE_BLOCK_SIZE: usize,
	> Iterator for EfhBhdsIterator<'a, T, ERASABLE_BLOCK_SIZE>
{
	type Item = BhdDirectory<'a, T, ERASABLE_BLOCK_SIZE>;
	fn next(&mut self) -> Option<<Self as Iterator>::Item> {
		while self.index_into_positions < self.positions.len() {
			let position =
				self.positions[self.index_into_positions];
			self.index_into_positions += 1;
			if position != 0xffff_ffff && position != 0
			/* sigh.  Some images have 0 as "invalid" mark */
			{
				match BhdDirectory::load(
					self.storage,
					position,
					self.amd_physical_mode_mmio_size,
				) {
					Ok(e) => {
						return Some(e);
					}
					Err(e) => {
						return None; // FIXME: error check
					}
				}
			}
		}
		None
	}
}

// TODO: Borrow storage.
pub struct Efs<
	T: FlashRead<ERASABLE_BLOCK_SIZE> + FlashWrite<ERASABLE_BLOCK_SIZE>,
	const ERASABLE_BLOCK_SIZE: usize,
> {
	storage: T,
	efh_beginning: ErasableLocation<ERASABLE_BLOCK_SIZE>,
	pub efh: Efh,
	amd_physical_mode_mmio_size: Option<u32>,
}

impl<
		T: FlashRead<ERASABLE_BLOCK_SIZE>
			+ FlashWrite<ERASABLE_BLOCK_SIZE>,
		const ERASABLE_BLOCK_SIZE: usize,
	> Efs<T, ERASABLE_BLOCK_SIZE>
{
	// TODO: If we wanted to, we could also try the whole thing on the top 16 MiB again
	// (I think it would be better to have the user just construct two
	// different Efs instances in that case)
	const EFH_SIZE: u32 = 0x200;
	pub(crate) fn efh_beginning(
		storage: &T,
		processor_generation: Option<ProcessorGeneration>,
	) -> Result<ErasableLocation<ERASABLE_BLOCK_SIZE>> {
		for position in EFH_POSITION.iter() {
			let mut xbuf: [u8; ERASABLE_BLOCK_SIZE] =
				[0; ERASABLE_BLOCK_SIZE];
			storage.read_exact(*position, &mut xbuf)?;
			match header_from_collection::<Efh>(&xbuf[..]) {
				Some(item) => {
					// Note: only one Efh with second_gen_efs() allowed in entire Flash!
					if item.signature().ok().or(Some(0)).unwrap() == 0x55AA55AA &&
						item.second_gen_efs() &&
						match processor_generation {
							Some(x) => item
								.compatible_with_processor_generation(
									x,
								),
							None => true,
						} {
						return Ok(ErasableLocation::<
							ERASABLE_BLOCK_SIZE,
						>::try_from(
							*position
						)?);
					}
				}
				None => {}
			}
		}
		// Old firmware header is better than no firmware header; TODO: Warn.
		for position in EFH_POSITION.iter() {
			let mut xbuf: [u8; ERASABLE_BLOCK_SIZE] =
				[0; ERASABLE_BLOCK_SIZE];
			storage.read_exact(*position, &mut xbuf)?;
			match header_from_collection::<Efh>(&xbuf[..]) {
				Some(item) => {
					if item.signature().ok().or(Some(0)).unwrap() == 0x55AA55AA &&
						!item.second_gen_efs() &&
						match processor_generation {
							//Some(x) => item.compatible_with_processor_generation(x),
							None => true,
							_ => false,
						} {
						return Ok(ErasableLocation::<
							ERASABLE_BLOCK_SIZE,
						>::try_from(
							*position
						)?);
					}
				}
				None => {}
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
		storage: T,
		processor_generation: Option<ProcessorGeneration>,
		amd_physical_mode_mmio_size: Option<u32>,
	) -> Result<Self> {
		let efh_beginning =
			Self::efh_beginning(&storage, processor_generation)?;
		let mut xbuf: [u8; ERASABLE_BLOCK_SIZE] =
			[0; ERASABLE_BLOCK_SIZE];
		storage.read_erasable_block(efh_beginning, &mut xbuf)?;
		let efh = header_from_collection::<Efh>(&xbuf[..])
			.ok_or_else(|| Error::EfsHeaderNotFound)?;
		if efh.signature().ok().or(Some(0)).unwrap() != 0x55aa_55aa {
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
		mut storage: T,
		processor_generation: ProcessorGeneration,
		efh_beginning: Location,
		amd_physical_mode_mmio_size: Option<u32>,
	) -> Result<Self> {
		if !EFH_POSITION.contains(&efh_beginning) {
			return Err(Error::EfsRangeCheck);
		}

		let mut buf: [u8; ERASABLE_BLOCK_SIZE] =
			[0xFF; ERASABLE_BLOCK_SIZE];
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
			ErasableLocation::<ERASABLE_BLOCK_SIZE>::try_from(
				efh_beginning,
			)?,
			&buf,
		)?;
		Self::load(storage, Some(processor_generation), amd_physical_mode_mmio_size)
	}

	/// Note: Either psp_directory or psp_combo_directory will succeed--but not both.
	pub fn psp_directory(
		&self,
	) -> Result<PspDirectory<T, ERASABLE_BLOCK_SIZE>> {
		let psp_directory_table_location =
			self.efh.psp_directory_table_location_zen().ok().or(Some(0xffff_ffff)).unwrap();
		if psp_directory_table_location == 0xffff_ffff {
			Err(Error::PspDirectoryHeaderNotFound)
		} else {
			let directory = match PspDirectory::load(
				&self.storage,
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
					.or(Some(0xffff_ffff))
					.unwrap();
				if addr == 0xffff_ffff {
					addr
				} else {
					addr & 0x00ff_ffff
				}
			};
			if psp_directory_table_location == 0xffff_ffff ||
				psp_directory_table_location == 0
			{
				Err(Error::PspDirectoryHeaderNotFound)
			} else {
				let directory = PspDirectory::load(
					&self.storage,
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
	pub fn psp_combo_directory(
		&self,
	) -> Result<ComboDirectory<T, ERASABLE_BLOCK_SIZE>> {
		let psp_directory_table_location =
			self.efh.psp_directory_table_location_zen().ok().or(Some(0xffff_ffff)).unwrap();
		if psp_directory_table_location == 0xffff_ffff {
			Err(Error::PspDirectoryHeaderNotFound)
		} else {
			let directory = match ComboDirectory::load(
				&self.storage,
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
					.or(Some(0xffff_ffff))
					.unwrap();
				if addr == 0xffff_ffff {
					addr
				} else {
					addr & 0x00ff_ffff
				}
			};
			if psp_directory_table_location == 0xffff_ffff ||
				psp_directory_table_location == 0
			{
				Err(Error::PspDirectoryHeaderNotFound)
			} else {
				let directory = ComboDirectory::load(
					&self.storage,
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

	pub fn second_level_psp_directory(
		&self,
	) -> Result<PspDirectory<T, ERASABLE_BLOCK_SIZE>> {
		let main_directory = self.psp_directory()?;
		for entry in main_directory.entries() {
			match entry.type_or_err() {
				Ok(PspDirectoryEntryType::SecondLevelDirectory) => {
					let psp_directory_table_location = main_directory.location_of_source(entry.source(main_directory.directory_address_mode)?, 0/*FIXME*/)?;
					let directory = PspDirectory::load(
						&self.storage,
						psp_directory_table_location
							.try_into()
							.map_err(|_| {
								Error::DirectoryRangeCheck
							})?,
						self.amd_physical_mode_mmio_size,
					)?;
					return Ok(directory);
				}
				Ok(_) => {
				}
				Err(_) => { // maybe just unknown entry type.
					// FIXME: check
				}
			}
		}
		Err(Error::PspDirectoryHeaderNotFound)
	}

	/// Returns an iterator over level 1 BHD directories
	pub fn bhd_directories(
		&self,
	) -> Result<EfhBhdsIterator<T, ERASABLE_BLOCK_SIZE>> {
		fn de_mmio(v: u32, amd_physical_mode_mmio_size: Option<u32>) -> u32 {
			if v == 0xffff_ffff || v == 0 {
				0xffff_ffff
			} else {
				if let Some(amd_physical_mode_mmio_size) = amd_physical_mode_mmio_size {
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
						Err(_) => {
							0xffff_ffff
						}
					}
				} else {
					0xffff_ffff
				}
			}
		}
		let efh = &self.efh;
		let amd_physical_mode_mmio_size = self.amd_physical_mode_mmio_size;
		let positions = [
			efh.bhd_directory_table_milan().ok().or(Some(0xffff_ffff)).unwrap(),
			de_mmio(efh.bhd_directory_tables[2].get(), amd_physical_mode_mmio_size),
			de_mmio(efh.bhd_directory_tables[1].get(), amd_physical_mode_mmio_size),
			de_mmio(efh.bhd_directory_tables[0].get(), amd_physical_mode_mmio_size),
		]; // the latter are physical addresses
		Ok(EfhBhdsIterator {
			storage: &self.storage,
			physical_address_mode: self.physical_address_mode(),
			positions: positions,
			index_into_positions: 0,
			amd_physical_mode_mmio_size: self.amd_physical_mode_mmio_size,
		})
	}

	// Make sure there's no overlap (even when rounded to entire erasure blocks)
	fn ensure_no_overlap(
		&self,
		beginning: Location,
		end: Location,
	) -> Result<()> {
		let (beginning, end) =
			self.storage.grow_to_erasable_block(beginning, end);
		// Check EFH no-overlap
		let reference_beginning = Location::from(self.efh_beginning);
		let reference_end = reference_beginning
			.checked_add(Self::EFH_SIZE)
			.ok_or(Error::Misaligned)?;
		let (reference_beginning, reference_end) =
			self.storage.grow_to_erasable_block(
				reference_beginning,
				reference_end,
			);
		let intersection_beginning = beginning.max(reference_beginning);
		let intersection_end = end.min(reference_end);
		if intersection_beginning < intersection_end {
			return Err(Error::Overlap);
		}

		match self.psp_directory() {
			Ok(psp_directory) => {
				let (reference_beginning, reference_end) =
					self.storage.grow_to_erasable_block(
						psp_directory
							.directory_beginning(),
						psp_directory.directory_end(),
					);
				let intersection_beginning =
					beginning.max(reference_beginning);
				let intersection_end = end.min(reference_end);
				if intersection_beginning < intersection_end {
					return Err(Error::Overlap);
				}
				let (reference_beginning, reference_end) = (
					Location::from(
						psp_directory
							.contents_beginning(),
					),
					Location::from(
						psp_directory.contents_end(),
					),
				);
				let intersection_beginning =
					beginning.max(reference_beginning);
				let intersection_end = end.min(reference_end);
				if intersection_beginning < intersection_end {
					return Err(Error::Overlap);
				}
			}
			Err(Error::PspDirectoryHeaderNotFound) => {}
			Err(e) => {
				return Err(e);
			}
		}
		let bhd_directories = self.bhd_directories()?;
		for bhd_directory in bhd_directories {
			let (reference_beginning, reference_end) =
				self.storage.grow_to_erasable_block(
					bhd_directory.directory_beginning(),
					bhd_directory.directory_end(),
				);
			let intersection_beginning =
				beginning.max(reference_beginning);
			let intersection_end = end.min(reference_end);
			if intersection_beginning < intersection_end {
				return Err(Error::Overlap);
			}
			let (reference_beginning, reference_end) = (
				Location::from(
					bhd_directory.contents_beginning(),
				),
				Location::from(bhd_directory.contents_end()),
			);
			let intersection_beginning =
				beginning.max(reference_beginning);
			let intersection_end = end.min(reference_end);
			if intersection_beginning < intersection_end {
				return Err(Error::Overlap);
			}
		}
		Ok(())
	}

	fn write_efh(&mut self) -> Result<()> {
		let mut buf: [u8; ERASABLE_BLOCK_SIZE] =
			[0xFF; ERASABLE_BLOCK_SIZE];
		match header_from_collection_mut(&mut buf[..]) {
			Some(item) => {
				*item = self.efh;
			}
			None => {}
		}

		self.storage
			.erase_and_write_blocks(self.efh_beginning, &buf)?;
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
		beginning: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		end: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		default_entry_address_mode: AddressMode,
	) -> Result<BhdDirectory<'_, T, ERASABLE_BLOCK_SIZE>> {
		match default_entry_address_mode {
			AddressMode::PhysicalAddress => {
				if !self.physical_address_mode() {
					return Err(Error::DirectoryTypeMismatch)
				}
			}
			AddressMode::EfsRelativeOffset => {
				if self.physical_address_mode() {
					return Err(Error::DirectoryTypeMismatch)
				}
			}
			_ => {
				return Err(Error::DirectoryTypeMismatch)
			}
		}
		match self.bhd_directories() {
			Ok(items) => {
				for directory in items {
					// TODO: Ensure that we don't have too many similar ones
				}
			}
			Err(e) => {
				return Err(e);
			}
		}
		self.ensure_no_overlap(
			Location::from(beginning),
			Location::from(end),
		)?;
		if self.efh.compatible_with_processor_generation(
			ProcessorGeneration::Milan,
		) {
			self.efh.set_bhd_directory_table_milan(beginning.into());
		// FIXME: ensure that the others are unset?
		} else {
			self.efh.bhd_directory_tables[2].set(beginning.into());
			// FIXME: ensure that the others are unset?
		}
		self.write_efh()?;
		let result = BhdDirectory::create(
			default_entry_address_mode,
			&mut self.storage,
			beginning,
			end,
			*b"$BHD",
			self.amd_physical_mode_mmio_size,
		)?;
		Ok(result)
	}

	// Note: BEGINNING, END are coordinates (in Byte).
	pub fn create_psp_directory(
		&mut self,
		beginning: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		end: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		default_entry_address_mode: AddressMode,
	) -> Result<PspDirectory<'_, T, ERASABLE_BLOCK_SIZE>> {
		match default_entry_address_mode {
			AddressMode::PhysicalAddress => {
				if !self.physical_address_mode() {
					return Err(Error::DirectoryTypeMismatch)
				}
			}
			AddressMode::EfsRelativeOffset => {
				if self.physical_address_mode() {
					return Err(Error::DirectoryTypeMismatch)
				}
			}
			_ => {
				return Err(Error::DirectoryTypeMismatch)
			}
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
		self.ensure_no_overlap(
			Location::from(beginning),
			Location::from(end),
		)?;
		// TODO: Boards older than Rome have 0xff at the top bits.  Depends on address_mode maybe.  Then, also psp_directory_table_location_naples should be set, instead.
		self.efh.set_psp_directory_table_location_zen(beginning.into());
		self.write_efh()?;
		let result = PspDirectory::create(
			default_entry_address_mode,
			&mut self.storage,
			beginning,
			end,
			*b"$PSP",
			self.amd_physical_mode_mmio_size,
		)?;
		Ok(result)
	}

	pub fn create_second_level_psp_directory(
		&mut self,
		beginning: ErasableLocation<ERASABLE_BLOCK_SIZE>,
		end: ErasableLocation<ERASABLE_BLOCK_SIZE>,
	) -> Result<PspDirectory<'_, T, ERASABLE_BLOCK_SIZE>> {
		self.ensure_no_overlap(
			Location::from(beginning),
			Location::from(end),
		)?;
		self.psp_directory()?.create_subdirectory(beginning, end, self.amd_physical_mode_mmio_size)
	}

	/*pub fn create_second_level_bhd_directory<'c>(&self, bhd_directory: &mut BhdDirectory<'c, T, ERASABLE_BLOCK_SIZE>, beginning: ErasableLocation<ERASABLE_BLOCK_SIZE>, end: ErasableLocation<ERASABLE_BLOCK_SIZE>) -> Result<BhdDirectory<'c, T, ERASABLE_BLOCK_SIZE>> {
		self.ensure_no_overlap(Location::from(beginning), Location::from(end))?;
		bhd_directory.create_subdirectory(beginning, end)
	}*/
}
