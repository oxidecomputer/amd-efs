// This file contains the AMD firmware Flash on-disk format.  Please only change it in coordination with the AMD firmware team.  Even then, you probably shouldn't.

use crate::struct_accessors::make_accessors;
use crate::struct_accessors::DummyErrorChecks;
use crate::struct_accessors::Getter;
use crate::struct_accessors::Setter;
use crate::types::Error;
use crate::types::Result;
use amd_flash::Location;
use byteorder::LittleEndian;
use core::convert::TryFrom;
use core::convert::TryInto;
use modular_bitfield::prelude::*;
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use num_traits::FromPrimitive;
use strum_macros::EnumString;
use zerocopy::{AsBytes, FromBytes, LayoutVerified, Unaligned, U32, U64};
//use crate::configs;

/// Given *BUF (a collection of multiple items), retrieves the first of the items and returns it.
/// If the item cannot be parsed, returns None.
pub fn header_from_collection_mut<'a, T: Sized + FromBytes + AsBytes>(
    buf: &'a mut [u8],
) -> Option<&'a mut T> {
    match LayoutVerified::<_, T>::new_from_prefix(buf) {
        Some((item, _xbuf)) => Some(item.into_mut()),
        None => None,
    }
}

/// Given *BUF (a collection of multiple items), retrieves the first of the items and returns it.
/// If the item cannot be parsed, returns None.
pub fn header_from_collection<'a, T: Sized + FromBytes>(
    buf: &'a [u8],
) -> Option<&'a T> {
    match LayoutVerified::<_, T>::new_from_prefix(buf) {
        Some((item, _xbuf)) => Some(item.into_ref()),
        None => None,
    }
}

type LU32 = U32<LittleEndian>;
type LU64 = U64<LittleEndian>;

// The first one is recommended by AMD; the last one is always used in practice.
pub const EFH_POSITION: [Location; 6] =
    [0xFA_0000, 0xF2_0000, 0xE2_0000, 0xC2_0000, 0x82_0000, 0x2_0000];

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, ToPrimitive, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum SpiReadMode {
    Normal33_33Mhz = 0b000, // up to 33.33 MHz
    Dual112 = 0b010,
    Quad114 = 0b011,
    Dual122 = 0b100,
    Quad144 = 0b101,
    Normal66_66Mhz = 0b110, // up to 66.66 MHz
    Fast = 0b111,
    DoNothing = 0xff,
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, ToPrimitive, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum SpiFastSpeedNew {
    _66_66MHz = 0,
    _33_33MHz = 1,
    _22_22MHz = 2,
    _16_66MHz = 3,
    _100MHz = 0b100,
    _800kHz = 0b101,
    DoNothing = 0xff,
}

make_accessors! {
    #[derive(FromBytes, AsBytes, Unaligned, Clone, Copy, Debug)]
    #[repr(C, packed)]
    pub struct EfhBulldozerSpiMode {
        read_mode: u8 : pub get SpiReadMode : pub set SpiReadMode,
        fast_speed_new: u8 : pub get SpiFastSpeedNew : pub set SpiFastSpeedNew,
        _reserved: u8,
    }
}

impl Default for EfhBulldozerSpiMode {
    fn default() -> Self {
        Self {
            read_mode: 0xff,
            fast_speed_new: 0xff,
            _reserved: 0xff, // FIXME: check
        }
    }
}

impl Getter<Result<EfhBulldozerSpiMode>> for EfhBulldozerSpiMode {
    fn get1(self) -> Result<Self> {
        Ok(self)
    }
}

impl Setter<EfhBulldozerSpiMode> for EfhBulldozerSpiMode {
    fn set1(&mut self, value: Self) {
        *self = value
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, ToPrimitive, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum SpiNaplesMicronMode {
    DummyCycle = 0x0a,
    DoNothing = 0xff,
}

make_accessors! {
    #[derive(FromBytes, AsBytes, Unaligned, Clone, Copy, Debug)]
    #[repr(C, packed)]
    pub struct EfhNaplesSpiMode {
        read_mode: u8 : pub get SpiReadMode : pub set SpiReadMode,
        fast_speed_new: u8 : pub get SpiFastSpeedNew : pub set SpiFastSpeedNew,
        micron_mode: u8 : pub get SpiNaplesMicronMode : pub set SpiNaplesMicronMode,
    }
}

impl Default for EfhNaplesSpiMode {
    fn default() -> Self {
        Self { read_mode: 0xff, fast_speed_new: 0xff, micron_mode: 0xff }
    }
}

impl Getter<Result<EfhNaplesSpiMode>> for EfhNaplesSpiMode {
    fn get1(self) -> Result<Self> {
        Ok(self)
    }
}

impl Setter<EfhNaplesSpiMode> for EfhNaplesSpiMode {
    fn set1(&mut self, value: EfhNaplesSpiMode) {
        *self = value
    }
}

impl Getter<Result<EfhRomeSpiMode>> for EfhRomeSpiMode {
    fn get1(self) -> Result<Self> {
        Ok(self)
    }
}

impl Setter<EfhRomeSpiMode> for EfhRomeSpiMode {
    fn set1(&mut self, value: EfhRomeSpiMode) {
        *self = value
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, ToPrimitive, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum SpiRomeMicronMode {
    SupportMicron = 0x55,
    ForceMicron = 0xaa,
    DoNothing = 0xff,
}

make_accessors! {
    #[derive(FromBytes, AsBytes, Unaligned, Clone, Copy, Debug)]
    #[repr(C, packed)]
    pub struct EfhRomeSpiMode {
        read_mode: u8 : pub get SpiReadMode : pub set SpiReadMode,
        fast_speed_new: u8 : pub get SpiFastSpeedNew : pub set SpiFastSpeedNew,
        micron_mode: u8 : pub get SpiRomeMicronMode : pub set SpiRomeMicronMode,
    }
}

impl Default for EfhRomeSpiMode {
    fn default() -> Self {
        Self { read_mode: 0xff, fast_speed_new: 0xff, micron_mode: 0x55 }
    }
}

make_accessors! {
    #[derive(FromBytes, AsBytes, Unaligned, Clone, Copy, Debug)]
    #[repr(C, packed)]
    pub struct Efh {
        signature: LU32 : pub get u32 : pub set u32,                           // 0x55aa_55aa
        imc_fw_location: LU32 : pub get u32 : pub set u32,                     // usually unused
        gbe_fw_location: LU32 : pub get u32 : pub set u32,                     // usually unused
        xhci_fw_location: LU32 : pub get u32 : pub set u32,                    // usually unused
        psp_directory_table_location_naples: LU32 : pub get u32 : pub set u32, // usually unused
        psp_directory_table_location_zen: LU32 : pub get u32 : pub set u32,
        /// High nibble of model number is either 0 (Naples), 1 (Raven Ridge), or 3 (Rome).  Then, corresponding indices into BHD_DIRECTORY_TABLES are 0, 1, 2, respectively.  Newer models always use BHD_DIRECTORY_TABLE_MILAN instead.
        pub bhd_directory_tables: [LU32; 3],
        pub(crate) efs_generations: LU32, // bit 0: All pointers are Flash MMIO pointers; should be clear for Rome; bit 1: Clear for Milan
        bhd_directory_table_milan: LU32 : pub get u32 : pub set u32, // or Combo
        _padding: LU32,
        promontory_firmware_location: LU32 : pub get u32 : pub set u32,
        pub low_power_promontory_firmware_location: LU32 : pub get u32 : pub set u32,
        _padding2: [LU32; 2],                      // at offset 0x38
        spi_mode_bulldozer: EfhBulldozerSpiMode : pub get EfhBulldozerSpiMode : pub set EfhBulldozerSpiMode,
        spi_mode_zen_naples: EfhNaplesSpiMode : pub get EfhNaplesSpiMode : pub set EfhNaplesSpiMode, // and Raven Ridge
        spi_mode_zen_rome: EfhRomeSpiMode : pub get EfhRomeSpiMode : pub set EfhRomeSpiMode,
        _reserved2: u8,
    }
}

impl Default for Efh {
    fn default() -> Self {
        Self {
            signature: 0x55aa_55aa.into(),
            imc_fw_location: 0.into(),
            gbe_fw_location: 0.into(),
            xhci_fw_location: 0.into(),
            psp_directory_table_location_naples: 0.into(),
            psp_directory_table_location_zen: 0.into(), // probably invalid
            bhd_directory_tables: [0.into(); 3],        // probably invalid
            efs_generations: 0xffff_fffe.into(),
            bhd_directory_table_milan: 0xffff_ffff.into(),
            _padding: 0xffff_ffff.into(),
            promontory_firmware_location: 0xffff_ffff.into(),
            low_power_promontory_firmware_location: 0xffff_ffff.into(),
            _padding2: [0xffff_ffff.into(); 2],
            spi_mode_bulldozer: EfhBulldozerSpiMode::default(),
            spi_mode_zen_naples: EfhNaplesSpiMode::default(),
            spi_mode_zen_rome: EfhRomeSpiMode::default(),
            _reserved2: 0,
        }
    }
}

#[repr(i8)]
#[derive(
    Debug,
    PartialEq,
    FromPrimitive,
    Clone,
    Copy,
    EnumString,
    strum_macros::EnumIter,
)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum ProcessorGeneration {
    Naples = -1,
    Rome = 0,
    Milan = 1,
}

impl Efh {
    /// Precondition: signature needs to be there--otherwise you might be reading garbage in the first place.
    /// Old (pre-Rome) boards had MMIO addresses instead of offsets in the slots.  Find out whether that's the case.
    pub fn second_gen_efs(&self) -> bool {
        self.efs_generations.get() & 1 == 0
    }

    /// Precondition: signature needs to be there--otherwise you might be reading garbage in the first place.
    /// Old (pre-Rome) boards had MMIO addresses instead of offsets in the slots.  Find out whether that's the case.
    pub fn physical_address_mode(&self) -> bool {
        !self.second_gen_efs()
    }
    /// Precondition: signature needs to be there--otherwise you might be reading garbage in the first place.
    /// Note: generation 1 is Milan
    pub fn compatible_with_processor_generation(
        &self,
        generation: ProcessorGeneration,
    ) -> bool {
        match generation {
            ProcessorGeneration::Naples => {
                // Naples didn't have generation flags yet, so make sure none of them are cleared.
                // Naples didn't have normal (non-MMIO) offsets yet--so those also should be unavailable.
                self.efs_generations.get() == 0xffff_ffff
            }
            ProcessorGeneration::Rome => {
                // Rome didn't have generation flags yet, so make sure none of them are cleared.
                self.efs_generations.get() == 0xffff_fffe
            }
            generation => {
                let generation: u8 = generation as u8;
                assert!(generation < 16);
                self.efs_generations.get() & (1 << generation) == 0
            }
        }
    }

    pub fn efs_generations_for_processor_generation(
        generation: ProcessorGeneration,
    ) -> u32 {
        match generation {
            // Naples didn't have normal (non-MMIO) offsets yet--so mark them unavailable.
            ProcessorGeneration::Naples => 0xffff_ffff,
            // Rome didn't have generation flags yet, so make sure to clear none of them.
            ProcessorGeneration::Rome => 0xffff_fffe,
            generation => {
                let generation: u8 = generation as u8;
                assert!(generation < 16);
                0xffff_fffe & !(1 << generation)
            }
        }
    }
}

#[derive(
    Debug, PartialEq, Eq, FromPrimitive, Clone, Copy, BitfieldSpecifier,
)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum AddressMode {
    /// Only supported for images <= 16 MiB.
    /// Right-justified in 4 GiB address space.
    /// Only really used in families older than Rome.
    PhysicalAddress = 0,
    EfsRelativeOffset = 1,            // x
    DirectoryRelativeOffset = 2,      // (x - Base)
    OtherDirectoryRelativeOffset = 3, // (x - other.Base);
}

pub(crate) const WEAK_ADDRESS_MODE: AddressMode =
    AddressMode::DirectoryRelativeOffset;

impl DummyErrorChecks for AddressMode {}

pub enum ValueOrLocation {
    Value(u64),
    PhysicalAddress(u32),
    EfsRelativeOffset(u32),
    DirectoryRelativeOffset(u32),
    OtherDirectoryRelativeOffset(u32),
}

impl core::fmt::Debug for ValueOrLocation {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::Value(x) => write!(fmt, "Value({:x?})", x),
            Self::PhysicalAddress(x) => {
                write!(fmt, "PhysicalAddress({:#x?})", x)
            }
            Self::EfsRelativeOffset(x) => {
                write!(fmt, "EfsRelativeOffset({:#x?})", x)
            }
            Self::DirectoryRelativeOffset(x) => {
                write!(fmt, "DirectoryRelativeOffset({:#x?})", x)
            }
            Self::OtherDirectoryRelativeOffset(x) => {
                write!(fmt, "OtherDirectoryRelativeOffset({:#x?})", x)
            }
        }
    }
}

pub(crate) fn mmio_encode(
    value: Location,
    amd_physical_mode_mmio_size: Option<u32>,
) -> Result<u32> {
    let mmio_address_lower = match amd_physical_mode_mmio_size {
        None | Some(0) => return Err(Error::DirectoryTypeMismatch),
        Some(x) => 1 + (0xFFFF_FFFFu32 - x),
    };
    Ok(value
        .checked_add(mmio_address_lower)
        .ok_or(Error::DirectoryTypeMismatch)?)
}

pub(crate) fn mmio_decode(
    value: u32,
    amd_physical_mode_mmio_size: u32,
) -> Result<u32> {
    let mmio_address_lower = match amd_physical_mode_mmio_size {
        0 => return Err(Error::DirectoryTypeMismatch),
        x => 1 + (0xFFFF_FFFFu32 - x),
    };
    if value >= mmio_address_lower {
        Ok(value - mmio_address_lower)
    } else {
        Err(Error::DirectoryTypeMismatch)
    }
}

impl ValueOrLocation {
    fn effective_address_mode(
        directory_address_mode: AddressMode,
        entry_address_mode: AddressMode,
    ) -> AddressMode {
        if directory_address_mode == WEAK_ADDRESS_MODE {
            entry_address_mode
        } else {
            directory_address_mode
        }
    }
    fn is_entry_address_mode_effective(
        directory_address_mode: AddressMode,
        entry_address_mode: AddressMode,
    ) -> bool {
        Self::effective_address_mode(directory_address_mode, entry_address_mode)
            == entry_address_mode
    }

    pub(crate) fn new_from_raw_location(
        directory_address_mode: AddressMode,
        source: u64,
    ) -> Result<Self> {
        let entry_address_mode = (source & 0xC000_0000_0000_0000) >> 62;
        let entry_address_mode =
            AddressMode::from_u64(entry_address_mode).unwrap();
        let value = u32::try_from(source & !0xC000_0000_0000_0000)
            .map_err(|_| Error::DirectoryPayloadRangeCheck)?;
        let address_mode = Self::effective_address_mode(
            directory_address_mode,
            entry_address_mode,
        );
        Ok(match address_mode {
            AddressMode::PhysicalAddress => Self::PhysicalAddress(value),
            AddressMode::EfsRelativeOffset => Self::EfsRelativeOffset(value),
            AddressMode::DirectoryRelativeOffset => {
                Self::DirectoryRelativeOffset(value)
            }
            AddressMode::OtherDirectoryRelativeOffset => {
                Self::OtherDirectoryRelativeOffset(value)
            }
        })
    }

    pub(crate) fn try_into_raw_location(
        &self,
        directory_address_mode: AddressMode,
    ) -> Result<u64> {
        match self {
            ValueOrLocation::Value(_) => Err(Error::EntryTypeMismatch),
            ValueOrLocation::PhysicalAddress(x) => {
                if Self::is_entry_address_mode_effective(
                    directory_address_mode,
                    AddressMode::PhysicalAddress,
                ) {
                    // AMD retrofitted (introduced) two
                    // flag bits at the top bits in Milan.
                    //
                    // In Rome, you actually COULD use
                    // all the bits.
                    //
                    // Newer platform do not regularily use
                    // AddressMode::PhysicalAddress anyway.
                    //
                    // But if someone uses
                    // AddressMode::PhysicalAddress,
                    // they might do it on Rome and use
                    // those two top bits as part of the
                    // address.
                    let v = u64::from(*x);
                    Ok(v)
                } else {
                    Err(Error::EntryTypeMismatch)
                }
            }
            ValueOrLocation::EfsRelativeOffset(x) => {
                if Self::is_entry_address_mode_effective(
                    directory_address_mode,
                    AddressMode::EfsRelativeOffset,
                ) {
                    let v = u64::from(*x) | 0x4000_0000_0000_0000;
                    Ok(v)
                } else {
                    Err(Error::EntryTypeMismatch)
                }
            }
            ValueOrLocation::DirectoryRelativeOffset(x) => {
                if Self::is_entry_address_mode_effective(
                    directory_address_mode,
                    AddressMode::DirectoryRelativeOffset,
                ) {
                    let v = u64::from(*x) | 0x8000_0000_0000_0000;
                    Ok(v)
                } else {
                    Err(Error::EntryTypeMismatch)
                }
            }
            ValueOrLocation::OtherDirectoryRelativeOffset(x) => {
                if Self::is_entry_address_mode_effective(
                    directory_address_mode,
                    AddressMode::OtherDirectoryRelativeOffset,
                ) {
                    let v = u64::from(*x) | 0xC000_0000_0000_0000;
                    Ok(v)
                } else {
                    Err(Error::EntryTypeMismatch)
                }
            }
        }
    }
}

/// XXX: If I move this to struct_accessors, it doesn't work anymore.

/// Since modular_bitfield has a lot of the things already, provide a macro
/// similar to make_accessors, but which doesn't generate any of the setters
/// or getters.  Instead, it just defines the user-friendly "Serde"* struct.
macro_rules! make_bitfield_serde {(
    $(#[$struct_meta:meta])*
    $struct_vis:vis
    struct $StructName:ident {
        $(
            $(#[$field_meta:meta])*
            $field_vis:vis
            $field_name:ident : $field_ty:ty $(: $getter_vis:vis get $field_user_ty:ty $(: $setter_vis:vis set $field_setter_user_ty:ty)?)?
        ),* $(,)?
    }
) => {
    $(#[$struct_meta])*
    $struct_vis
    struct $StructName {
        $(
            $(#[$field_meta])*
            $field_vis
            $field_name : $field_ty,
        )*
    }

    impl $StructName {
        pub fn builder() -> Self {
                Self::new()
        }
        pub fn build(&self) -> Self {
                self.clone()
        }
    }

    paste::paste! {
        #[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
        #[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
        //#[serde(remote = "" $StructName)]
        pub(crate) struct [<Serde $StructName>] {
            $(
                $(
                    $getter_vis
                    //pub(crate)
                    $field_name : <$field_ty as Specifier>::InOut, // $field_user_ty
                )?
            )*
        }
    }
}}

make_bitfield_serde! {
    #[bitfield(bits = 32)]
    #[repr(u32)]
    #[derive(Copy, Clone, Debug)]
    pub struct DirectoryAdditionalInfo {
        pub max_size: B10 : pub get u16 : pub set u16, // directory size in 4 kiB; Note: doc error in AMD docs // TODO: Shrink setter.
        #[skip(getters, setters)]
        spi_block_size: B4, // spi block size in 4 kiB; Note: 0 = 64 kiB
        pub base_address: B15 : pub get u16 : pub set u16, // base address in 4 kiB; if the actual payload (the file contents) of the directory are somewhere else, this can specify where. // TODO: Shrink setter.
        #[bits = 2]
        pub address_mode: AddressMode : pub get AddressMode : pub set AddressMode, // FIXME: This should not be able to be changed (from/to 2 at least) as you are iterating over a directory--since the iterator has to interpret what it is reading relative to this setting // TODO: Shrink setter.
        #[skip]
        __: bool,
    }
}

impl Default for DirectoryAdditionalInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectoryAdditionalInfo {
    pub const UNIT: usize = 4096; // Byte
    pub fn with_spi_block_size_checked(
        &mut self,
        value: u16,
    ) -> core::result::Result<Self, modular_bitfield::error::OutOfBounds> {
        let mut result = *self;
        result.set_spi_block_size_checked(value)?;
        Ok(result)
    }
    pub fn spi_block_size_or_err(
        &self,
    ) -> core::result::Result<u16, modular_bitfield::error::InvalidBitPattern<u8>>
    {
        let spi_block_size = ((u32::from(*self) >> 10) & 0xf) as u16;
        match spi_block_size {
            0 => Ok(0x10), // 64 kiB
            n => Ok(n),
        }
    }
    pub fn spi_block_size(&self) -> u16 {
        self.spi_block_size_or_err().unwrap()
    }
    pub fn set_spi_block_size_checked(
        &mut self,
        value: u16,
    ) -> core::result::Result<(), modular_bitfield::error::OutOfBounds> {
        let mut mask = u32::from(*self) & !0b1111_0000000000;
        if value > 0 && value <= 15 {
            mask |= (value as u32) << 10;
        } else if value == 16 { // 64 kiB
        } else {
            return Err(modular_bitfield::error::OutOfBounds);
        }
        *self = Self::from(mask);
        Ok(())
    }
    /// Given a value, tries to convert it into UNIT without loss.  If that's not possible, returns None
    pub fn try_into_unit(value: usize) -> Option<u16> {
        if value % Self::UNIT == 0 {
            let value = value / Self::UNIT;
            Some(value.try_into().ok()?)
        } else {
            None
        }
    }
    pub fn try_from_unit(value: u16) -> Option<usize> {
        let result: usize = value.try_into().ok()?;
        result.checked_mul(Self::UNIT)
    }
}

pub trait DirectoryHeader {
    fn cookie(&self) -> [u8; 4];
    fn set_cookie(&mut self, value: [u8; 4]);
    fn additional_info(&self) -> DirectoryAdditionalInfo;
    fn set_additional_info(&mut self, value: DirectoryAdditionalInfo);
    fn total_entries(&self) -> u32;
    fn set_total_entries(&mut self, value: u32);
    fn checksum(&self) -> u32;
    fn set_checksum(&mut self, value: u32);
}

#[derive(FromBytes, AsBytes, Unaligned, Clone, Copy)]
#[repr(C, packed)]
pub struct PspDirectoryHeader {
    pub(crate) cookie: [u8; 4], // b"$PSP" or b"$PL2"
    pub(crate) checksum: LU32, // 32-bit CRC value of header below this field and including all entries
    pub(crate) total_entries: LU32,
    pub(crate) additional_info: LU32, // 0xffff_ffff; or DirectoryAdditionalInfo
}

impl DirectoryHeader for PspDirectoryHeader {
    fn cookie(&self) -> [u8; 4] {
        self.cookie
    }
    fn set_cookie(&mut self, value: [u8; 4]) {
        self.cookie = value;
    }
    fn additional_info(&self) -> DirectoryAdditionalInfo {
        DirectoryAdditionalInfo::from(self.additional_info.get())
    }
    fn set_additional_info(&mut self, value: DirectoryAdditionalInfo) {
        self.additional_info.set(value.into())
    }
    fn total_entries(&self) -> u32 {
        self.total_entries.get()
    }
    fn set_total_entries(&mut self, value: u32) {
        self.total_entries.set(value)
    }
    fn checksum(&self) -> u32 {
        self.checksum.get()
    }
    fn set_checksum(&mut self, value: u32) {
        self.checksum.set(value)
    }
}

impl Default for PspDirectoryHeader {
    fn default() -> Self {
        Self {
            cookie: *b"    ",   // invalid
            checksum: 0.into(), // invalid
            total_entries: 0.into(),
            additional_info: 0xffff_ffff.into(), // invalid
        }
    }
}

impl core::fmt::Debug for PspDirectoryHeader {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let checksum = self.checksum.get();
        let total_entries = self.total_entries.get();
        let additional_info =
            DirectoryAdditionalInfo::from(self.additional_info.get());
        fmt.debug_struct("PspDirectoryHeader")
            .field("cookie", &self.cookie)
            .field("checksum", &checksum)
            .field("total_entries", &total_entries)
            .field("additional_info", &additional_info)
            .finish()
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy, BitfieldSpecifier)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[bits = 8]
#[non_exhaustive]
pub enum PspDirectoryEntryType {
    AmdPublicKey = 0x00,
    PspBootloader = 0x01,
    PspOs = 0x02,
    PspRecoveryBootloader = 0x03,
    PspNvdata = 0x04,
    SmuOffChipFirmware8 = 0x08,
    AmdSecureDebugKey = 0x09,
    AblPublicKey = 0x0A,
    PspSoftFuseChain = 0x0B,
    PspTrustlets = 0x0C,
    PspTrustletPublicKey = 0x0D,
    SmuOffChipFirmware12 = 0x12,
    PspEarlySecureUnlockDebugImage = 0x13,
    DiscoveryBinary = 0x20,
    WrappedIkek = 0x21,
    PspTokenUnlockData = 0x22,
    SecurityPolicyBinary = 0x24,
    Mp2Firmware = 0x25,
    Mp2Firmware2 = 0x26,
    UserModeUnitTests = 0x27,
    PspSystemDriverEntryPoints = 0x28,
    KvmImage = 0x29,
    Mp5Firmware = 0x2A,
    EfsPhysAddr = 0x2B,
    TeeWriteOnceNvram = 0x2C,
    ExternalChipsetPspBootLoader2d = 0x2D,
    ExternalChipsetMp0Dxio = 0x2E,
    ExternalChipsetMp1Firmware = 0x2F,
    Abl0 = 0x30,
    Abl1 = 0x31,
    Abl2 = 0x32,
    Abl3 = 0x33,
    Abl4 = 0x34,
    Abl5 = 0x35,
    Abl6 = 0x36,
    Abl7 = 0x37,
    SevData = 0x38,
    SevCode = 0x39,
    PpinWhiteListBinary = 0x3A,
    SerdesPhyMicrocode = 0x3B,
    VbiosPreload = 0x3C,
    WlanUmac = 0x3D,
    WlanImac = 0x3E,
    WlanBluetooth = 0x3F,
    SecondLevelDirectory = 0x40,
    ExternalChipsetMp0Bootloader = 0x41,
    DxioPhySramFirmware = 0x42,
    DxioPhySramPublicKey = 0x43,
    UsbUnifiedPhyFirmware = 0x44,
    TosSecurityPolicyBinary = 0x45,
    ExternalChipsetPspBootloader46 = 0x46,
    DrtmTa = 0x47,
    // Put into root PSP directory, if at all.
    SecondLevelAPspDirectory = 0x48, // multiple of those entries are possible
    // Put into SecondLevelAPspDirectory or SecondLevelBPspDirectory, payload
    // being a SecondLevelABhdDirectory or SecondLevelBBhdDirectory,
    // respectively.
    SecondLevelBhdDirectory = 0x49,
    // Put into root PSP directory, if at all.
    SecondLevelBPspDirectory = 0x4A,
    ExternalChipsetSecurityPolicyBinary = 0x4C,
    ExternalChipsetSecureDebugUnlockBinary = 0x4D,
    PmuPublicKey = 0x4E,
    UmcFirmware = 0x4F,
    PspBootloaderPublicKeysTable = 0x50,
    PspTosPublicKeysTable = 0x51,
    PspBootloaderUserApplication = 0x52,
    PspBootloaderUserApplicationPublicKey = 0x53,
    PspRpmcNvram = 0x54,
    BootloaderSplTable = 0x55, // used by boot ROM
    TosSplTable = 0x56,        // used by off-chip bootloader
    PspBootloaderCvipConfigurationTable = 0x57,
    DmcuEram = 0x58,
    DmcuIsr = 0x59,
    Msmu0 = 0x5A,
    Msmu1 = 0x5B,
    OemSysTa = 0x80,
    OemSysTaPublicKey = 0x81,
    OemIkek = 0x82,
    OemSplTable = 0x83, // used by customer-signed binary
    OemTkek = 0x84,
    AmfFirmwarePart1 = 0x85,
    AmfFirmwarePart2 = 0x86,
    MpmFactoryProvisioningData = 0x87,
    MpmWlanFirmware = 0x88,
    MpmSecurityDriver = 0x89,
}

impl DummyErrorChecks for PspDirectoryEntryType {}

/// For 32 MiB SPI Flash, which half to map to MMIO 0xff00_0000.
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy, BitfieldSpecifier)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[bits = 1]
pub enum PspSoftFuseChain32MiBSpiDecoding {
    LowerHalf = 0,
    UpperHalf = 1,
}

impl DummyErrorChecks for PspSoftFuseChain32MiBSpiDecoding {}

#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy, BitfieldSpecifier)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[bits = 1]
pub enum PspSoftFuseChainPostCodeDecoding {
    Lpc = 0,
    Espi = 1,
}

impl DummyErrorChecks for PspSoftFuseChainPostCodeDecoding {}

make_bitfield_serde! {
    #[bitfield(bits = 64)]
    #[repr(u64)]
    #[derive(Copy, Clone, Debug)]
    pub struct PspSoftFuseChain {
        pub secure_debug_unlock: bool : pub get bool : pub set bool,
        #[skip]
        __: bool,
        pub early_secure_debug_unlock: bool : pub get bool : pub set bool,
        pub unlock_token_in_nvram: bool : pub get bool : pub set bool, // if the unlock token has been stored (by us) into NVRAM
        pub force_security_policy_loading_even_if_insecure: bool : pub get bool : pub set bool,
        pub load_diagnostic_bootloader: bool : pub get bool : pub set bool,
        pub disable_psp_debug_prints: bool : pub get bool : pub set bool,
        #[skip]
        __: B7,
        pub spi_decoding: PspSoftFuseChain32MiBSpiDecoding : pub get PspSoftFuseChain32MiBSpiDecoding : pub set PspSoftFuseChain32MiBSpiDecoding,
        pub postcode_decoding: PspSoftFuseChainPostCodeDecoding : pub get PspSoftFuseChainPostCodeDecoding : pub set PspSoftFuseChainPostCodeDecoding,
        #[skip]
        __: B12,
        #[skip]
        __: bool,
        pub skip_mp2_firmware_loading: bool : pub get bool : pub set bool,
        pub postcode_output_control_1byte: bool : pub get bool : pub set bool, // ???
        pub force_recovery_booting: bool : pub get bool : pub set bool,
        #[skip]
        __: B32,
    }
}

impl Default for PspSoftFuseChain {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy, BitfieldSpecifier)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[bits = 2]
#[non_exhaustive]
pub enum PspDirectoryRomId {
    SpiCs1 = 0,
    SpiCs2 = 1,
}

impl DummyErrorChecks for PspDirectoryRomId {}

impl Default for PspDirectoryRomId {
    fn default() -> Self {
        Self::SpiCs1
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy, BitfieldSpecifier)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[bits = 2]
#[non_exhaustive]
pub enum BhdDirectoryRomId {
    SpiCs1 = 0,
    SpiCs2 = 1,
}

impl DummyErrorChecks for BhdDirectoryRomId {}

impl Default for BhdDirectoryRomId {
    fn default() -> Self {
        Self::SpiCs1
    }
}

make_bitfield_serde! {
    #[derive(Clone, Copy)]
    #[bitfield(bits = 32)]
    #[repr(u32)]
    pub struct PspDirectoryEntryAttrs {
        #[bits = 8]
        #[allow(non_snake_case)]
        pub type_: PspDirectoryEntryType : pub get PspDirectoryEntryType : pub set PspDirectoryEntryType,
        pub sub_program: B8 : pub get u8 : pub set u8, // function of AMD Family and Model; only useful for types 8, 0x24, 0x25
        pub rom_id: PspDirectoryRomId : pub get PspDirectoryRomId : pub set PspDirectoryRomId,
        #[skip]
        __: B14,
    }
}

make_accessors! {
    #[derive(FromBytes, AsBytes, Unaligned, Clone, Copy)]
    #[repr(C, packed)]
    pub struct PspDirectoryEntry {
        pub(crate) attrs: LU32,
        pub(crate) internal_size: LU32,
        pub(crate) internal_source: LU64, // Note: value iff size == 0; otherwise location; TODO: (iff directory.address_mode == 2) entry address mode (top 2 bits), or 0
    }
}

// TODO: Remove.
impl Default for PspDirectoryEntry {
    fn default() -> Self {
        Self {
            attrs: 0.into(),
            internal_size: 0.into(),
            internal_source: 0.into(),
        }
    }
}

pub trait DirectoryEntry {
    fn source(
        &self,
        directory_address_mode: AddressMode,
    ) -> Result<ValueOrLocation>;
    fn size(&self) -> Option<u32>;
    /// Note: This can also modify size as a side effect.
    fn set_source(
        &mut self,
        directory_address_mode: AddressMode,
        value: ValueOrLocation,
    ) -> Result<()>;
    fn set_size(&mut self, value: Option<u32>);
}
pub trait DirectoryEntrySerde: Sized {
    fn from_slice(source: &[u8]) -> Option<Self>;
    fn copy_into_slice(&self, destination: &mut [u8]);
}
impl DirectoryEntrySerde for PspDirectoryEntry {
    fn from_slice(source: &[u8]) -> Option<Self> {
        let result = LayoutVerified::<_, Self>::new_unaligned(source)?;
        Some(*result.into_ref())
    }
    fn copy_into_slice(&self, destination: &mut [u8]) {
        destination.copy_from_slice(&self.as_bytes())
    }
}

impl PspDirectoryEntry {
    const SIZE_VALUE_MARKER: u32 = 0xFFFF_FFFF;
    pub fn type_or_err(&self) -> Result<PspDirectoryEntryType> {
        let attrs = PspDirectoryEntryAttrs::from(self.attrs.get());
        attrs.type__or_err().map_err(|_| Error::EntryTypeMismatch)
    }
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_type_(&mut self, value: PspDirectoryEntryType) -> &mut Self {
        let mut attrs = PspDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_type_(value);
        self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        self
    }
    pub fn type_(&self) -> PspDirectoryEntryType {
        let attrs = PspDirectoryEntryAttrs::from(self.attrs.get());
        attrs.type_()
    }
    pub fn with_sub_program(&mut self, value: u8) -> &mut Self {
        let mut attrs = PspDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_sub_program(value);
        self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        self
    }
    pub fn sub_program(&self) -> u8 {
        let attrs = PspDirectoryEntryAttrs::from(self.attrs.get());
        attrs.sub_program()
    }
    pub fn with_rom_id(&mut self, value: PspDirectoryRomId) -> &mut Self {
        let mut attrs = PspDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_rom_id(value);
        self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        self
    }
    pub fn rom_id(&self) -> PspDirectoryRomId {
        let attrs = PspDirectoryEntryAttrs::from(self.attrs.get());
        attrs.rom_id()
    }
    /// Note: Caller can modify other attributes using the with_ accessors.
    pub fn new_value(type_: PspDirectoryEntryType, value: u64) -> Result<Self> {
        let mut result = Self::new().with_type_(type_.into()).build();
        result.internal_size = Self::SIZE_VALUE_MARKER.into();
        result.internal_source = value.into();
        Ok(result)
    }
    /// Note: Caller can modify other attributes using the with_ accessors.
    pub fn new_payload(
        directory_address_mode: AddressMode,
        type_: PspDirectoryEntryType,
        size: Option<u32>,
        source: Option<ValueOrLocation>,
    ) -> Result<Self> {
        let mut result = Self::new().with_type_(type_.into()).build();
        result.set_size(size);
        if let Some(x) = source {
            result.set_source(directory_address_mode, x)?;
        }
        Ok(result)
    }
}

impl DirectoryEntry for PspDirectoryEntry {
    fn source(
        &self,
        directory_address_mode: AddressMode,
    ) -> Result<ValueOrLocation> {
        let source = self.internal_source.get();
        let size = self.internal_size.get();
        if size == Self::SIZE_VALUE_MARKER {
            Ok(ValueOrLocation::Value(source))
        } else {
            ValueOrLocation::new_from_raw_location(
                directory_address_mode,
                source,
            )
        }
    }
    fn set_source(
        &mut self,
        directory_address_mode: AddressMode,
        value: ValueOrLocation,
    ) -> Result<()> {
        match value {
            ValueOrLocation::Value(v) => {
                self.internal_size.set(Self::SIZE_VALUE_MARKER);
                self.internal_source.set(v);
                Ok(())
            }
            x => {
                let v = x.try_into_raw_location(directory_address_mode)?;
                self.internal_source.set(v);
                Ok(())
            }
        }
    }
    fn size(&self) -> Option<u32> {
        let size = self.internal_size.get();
        if size == Self::SIZE_VALUE_MARKER {
            None
        } else {
            Some(size)
        }
    }
    fn set_size(&mut self, value: Option<u32>) {
        self.internal_size.set(match value {
            None => Self::SIZE_VALUE_MARKER,
            Some(x) => {
                assert!(x != Self::SIZE_VALUE_MARKER);
                x
            }
        })
    }
}

impl core::fmt::Debug for PspDirectoryEntry {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // DirectoryRelativeOffset (WEAK_ADDRESS_MODE) is the only one that's always overridable.
        let source = self.source(WEAK_ADDRESS_MODE);
        let size = self.size();
        let attrs = PspDirectoryEntryAttrs::from(self.attrs.get());
        fmt.debug_struct("PspDirectoryEntry")
            .field("type_", &attrs.type__or_err())
            .field("sub_program", &attrs.sub_program_or_err())
            .field("rom_id", &attrs.rom_id_or_err())
            .field("size", &size)
            .field("source", &source)
            .finish()
    }
}

#[derive(FromBytes, AsBytes, Unaligned, Clone, Copy)]
#[repr(C, packed)]
pub struct BhdDirectoryHeader {
    pub(crate) cookie: [u8; 4], // b"$BHD" or b"$BL2"
    pub(crate) checksum: LU32, // 32-bit CRC value of header below this field and including all entries
    pub(crate) total_entries: LU32,
    pub(crate) additional_info: LU32,
}

impl DirectoryHeader for BhdDirectoryHeader {
    fn cookie(&self) -> [u8; 4] {
        self.cookie
    }
    fn set_cookie(&mut self, value: [u8; 4]) {
        self.cookie = value;
    }
    fn additional_info(&self) -> DirectoryAdditionalInfo {
        DirectoryAdditionalInfo::from(self.additional_info.get())
    }
    fn set_additional_info(&mut self, value: DirectoryAdditionalInfo) {
        self.additional_info.set(value.into())
    }
    fn total_entries(&self) -> u32 {
        self.total_entries.get()
    }
    fn set_total_entries(&mut self, value: u32) {
        self.total_entries.set(value)
    }
    fn checksum(&self) -> u32 {
        self.checksum.get()
    }
    fn set_checksum(&mut self, value: u32) {
        self.checksum.set(value)
    }
}

impl Default for BhdDirectoryHeader {
    fn default() -> Self {
        Self {
            cookie: *b"    ",   // invalid
            checksum: 0.into(), // invalid
            total_entries: 0.into(),
            additional_info: 0xffff_ffff.into(), // invalid
        }
    }
}

impl core::fmt::Debug for BhdDirectoryHeader {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let checksum = self.checksum.get();
        let total_entries = self.total_entries.get();
        let additional_info =
            DirectoryAdditionalInfo::from(self.additional_info.get());
        fmt.debug_struct("BhdDirectoryHeader")
            .field("cookie", &self.cookie)
            .field("checksum", &checksum)
            .field("total_entries", &total_entries)
            .field("additional_info", &additional_info)
            .finish()
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy, BitfieldSpecifier)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[bits = 8]
#[non_exhaustive]
pub enum BhdDirectoryEntryType {
    OemPublicKey = 0x05,
    CryptographicSignature = 0x07,
    Apcb = 0x60, // usually instances 0 (updatable) and 1 (eventlog)
    Apob = 0x61,
    Bios = 0x62,
    ApobNvCopy = 0x63, // used during S3 resume
    PmuFirmwareInstructions = 0x64,
    PmuFirmwareData = 0x65,
    MicrocodePatch = 0x66,
    MceData = 0x67,
    ApcbBackup = 0x68, // usually instances 0 (backup), 8 (updatable) and 9 (eventlog)
    VgaInterpreter = 0x69,
    Mp2FirmwareConfiguration = 0x6A,
    CorebootVbootWorkbuffer = 0x6B, // main memory shared between PSP and x86
    MpmConfiguration = 0x6C,
    SecondLevelDirectory = 0x70, // also a BhdDirectory
}

impl DummyErrorChecks for BhdDirectoryEntryType {}

#[derive(Copy, Clone, Debug, FromPrimitive, BitfieldSpecifier)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[bits = 8]
#[non_exhaustive]
pub enum BhdDirectoryEntryRegionType {
    Normal = 0, // for X86: always
    Ta1 = 1,
    Ta2 = 2,
}

impl Default for BhdDirectoryEntryRegionType {
    fn default() -> Self {
        Self::Normal
    }
}

impl DummyErrorChecks for BhdDirectoryEntryRegionType {}

make_bitfield_serde! {
    #[bitfield(bits = 32)]
    #[derive(Clone, Copy)]
    #[repr(u32)]
    pub struct BhdDirectoryEntryAttrs {
        #[bits = 8]
        #[allow(non_snake_case)]
        pub type_: BhdDirectoryEntryType : pub get BhdDirectoryEntryType : pub set BhdDirectoryEntryType,
        #[bits = 8]
        pub region_type: BhdDirectoryEntryRegionType : pub get BhdDirectoryEntryRegionType : pub set BhdDirectoryEntryRegionType,
        pub reset_image: bool : pub get bool : pub set bool,
        pub copy_image: bool : pub get bool : pub set bool,
        pub read_only: bool : pub get bool : pub set bool, // only useful for region_type > 0
        pub compressed: bool : pub get bool : pub set bool,
        pub instance: B4 : pub get u8 : pub set u8, // TODO: Shrink setter.
        pub sub_program: B3 : pub get u8 : pub set u8, // function of AMD Family and Model; only useful for types PMU firmware and APCB binaries // TODO: Shrink setter.
        pub rom_id: BhdDirectoryRomId : pub get BhdDirectoryRomId : pub set BhdDirectoryRomId,
        #[skip]
        __: B3,
    }
}
make_accessors! {
    #[derive(FromBytes, AsBytes, Unaligned, Clone, Copy)]
    #[repr(C, packed)]
    pub struct BhdDirectoryEntry {
        attrs: LU32,
        pub(crate) internal_size: LU32,   // 0xFFFF_FFFF for value entry
        pub(crate) internal_source: LU64, // value (or nothing) iff size == 0; otherwise source_location; TODO: (iff directory.address_mode == 2) entry address mode (top 2 bits), or 0
        pub(crate) internal_destination_location: LU64, // 0xffff_ffff_ffff_ffff: none
    }
}

impl DirectoryEntrySerde for BhdDirectoryEntry {
    fn from_slice(source: &[u8]) -> Option<Self> {
        let result = LayoutVerified::<_, Self>::new_unaligned(source)?;
        Some(*result.into_ref())
    }
    fn copy_into_slice(&self, destination: &mut [u8]) {
        destination.copy_from_slice(&self.as_bytes())
    }
}

impl BhdDirectoryEntry {
    const SIZE_VALUE_MARKER: u32 = 0xFFFF_FFFF;
    const DESTINATION_NONE_MARKER: u64 = 0xffff_ffff_ffff_ffff;
    pub fn type_or_err(&self) -> Result<BhdDirectoryEntryType> {
        let attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.type__or_err().map_err(|_| Error::EntryTypeMismatch)
    }

    pub fn destination_location(&self) -> Option<u64> {
        let destination_location = self.internal_destination_location.get();
        if destination_location == Self::DESTINATION_NONE_MARKER {
            None
        } else {
            Some(destination_location)
        }
    }
    pub fn with_type_(&mut self, value: BhdDirectoryEntryType) -> &mut Self {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_type_(value);
        self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        self
    }
    pub fn with_region_type(
        &mut self,
        value: BhdDirectoryEntryRegionType,
    ) -> &mut Self {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_region_type(value);
        self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        self
    }
    pub fn with_reset_image(&mut self, value: bool) -> &mut Self {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_reset_image(value);
        self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        self
    }
    pub fn set_reset_image(&mut self, value: bool) {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_reset_image(value);
    }
    pub fn with_copy_image(&mut self, value: bool) -> &mut Self {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_copy_image(value);
        self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        self
    }
    pub fn set_copy_image(&mut self, value: bool) {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_copy_image(value);
    }
    pub fn with_read_only(&mut self, value: bool) -> &mut Self {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_read_only(value);
        self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        self
    }
    pub fn set_read_only(&mut self, value: bool) {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_read_only(value);
    }
    pub fn with_compressed(&mut self, value: bool) -> &mut Self {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_compressed(value);
        self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        self
    }
    pub fn set_compressed(&mut self, value: bool) {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_compressed(value);
    }
    pub fn with_instance(&mut self, value: u8) -> &mut Self {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_instance(value);
        self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        self
    }
    pub fn instance(&self) -> u8 {
        let attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.instance()
    }
    pub fn with_sub_program(&mut self, value: u8) -> &mut Self {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_sub_program(value);
        self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        self
    }
    pub fn sub_program(&self) -> u8 {
        let attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.sub_program()
    }
    pub fn with_rom_id(&mut self, value: BhdDirectoryRomId) -> &mut Self {
        let mut attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.set_rom_id(value);
        self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        self
    }
    pub fn rom_id(&self) -> BhdDirectoryRomId {
        let attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        attrs.rom_id()
    }
    pub(crate) fn with_internal_destination_location(
        &mut self,
        value: u64,
    ) -> &mut Self {
        self.internal_destination_location.set(value);
        self
    }
    pub fn new() -> Self {
        Self::default()
    }
    /// Note: Caller can modify other attributes afterwards (especially source--which he should modify).
    pub fn new_payload(
        directory_address_mode: AddressMode,
        type_: BhdDirectoryEntryType,
        size: Option<u32>,
        source: Option<ValueOrLocation>,
        destination_location: Option<u64>,
    ) -> Result<Self> {
        let mut result = Self::new()
            .with_type_(type_.into())
            .with_internal_destination_location(
                match destination_location {
                    None => Self::DESTINATION_NONE_MARKER,
                    Some(x) => {
                        if x == Self::DESTINATION_NONE_MARKER {
                            return Err(Error::EntryTypeMismatch);
                        }
                        x
                    }
                }
                .into(),
            )
            .build();
        result.set_size(size);
        if let Some(x) = source {
            result.set_source(directory_address_mode, x)?;
        } else {
            result.set_size(None);
        }
        Ok(result)
    }
}

// TODO: Remove.
impl Default for BhdDirectoryEntry {
    fn default() -> Self {
        Self {
            attrs: 0.into(),
            internal_size: 0.into(),
            internal_source: 0.into(),
            internal_destination_location: Self::DESTINATION_NONE_MARKER.into(),
        }
    }
}

impl DirectoryEntry for BhdDirectoryEntry {
    fn source(
        &self,
        directory_address_mode: AddressMode,
    ) -> Result<ValueOrLocation> {
        let size = self.internal_size.get();
        let source = self.internal_source.get();
        if size == Self::SIZE_VALUE_MARKER {
            Ok(ValueOrLocation::Value(source))
        } else {
            let source = self.internal_source.get();
            ValueOrLocation::new_from_raw_location(
                directory_address_mode,
                source,
            )
        }
    }
    fn set_source(
        &mut self,
        directory_address_mode: AddressMode,
        value: ValueOrLocation,
    ) -> Result<()> {
        match value {
            ValueOrLocation::Value(v) => {
                if self.internal_size.get() == Self::SIZE_VALUE_MARKER {
                    self.internal_source.set(v);
                    Ok(())
                } else {
                    Err(Error::EntryTypeMismatch)
                }
            }
            x => {
                let v = x.try_into_raw_location(directory_address_mode)?;
                self.internal_source.set(v);
                Ok(())
            }
        }
    }
    fn size(&self) -> Option<u32> {
        let size = self.internal_size.get();
        if size == Self::SIZE_VALUE_MARKER {
            None
        } else {
            Some(size)
        }
    }
    fn set_size(&mut self, value: Option<u32>) {
        self.internal_size.set(match value {
            None => Self::SIZE_VALUE_MARKER,
            Some(x) => {
                assert!(x != Self::SIZE_VALUE_MARKER);
                x
            }
        })
    }
}

impl core::fmt::Debug for BhdDirectoryEntry {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // DirectoryRelativeOffset (WEAK_ADDRESS_MODE) is the only one that's always overridable.
        let source = self.source(WEAK_ADDRESS_MODE);
        let destination_location = self.destination_location();
        let size = self.size();
        let attrs = BhdDirectoryEntryAttrs::from(self.attrs.get());
        fmt.debug_struct("BhdDirectoryEntry")
            .field("type_", &attrs.type__or_err())
            .field("region_type", &attrs.region_type_or_err())
            .field("reset_image", &attrs.reset_image_or_err())
            .field("copy_image", &attrs.copy_image_or_err())
            .field("read_only", &attrs.read_only_or_err())
            .field("compressed", &attrs.compressed_or_err())
            .field("instance", &attrs.instance_or_err())
            .field("size", &size)
            .field("source", &source)
            .field("destination_location", &destination_location)
            .finish()
    }
}

#[repr(u32)]
#[derive(Debug, PartialEq, FromPrimitive, ToPrimitive, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ComboDirectoryLookupMode {
    BruteForce = 0,
    MatchId = 1,
}

make_accessors! {
    #[derive(FromBytes, AsBytes, Unaligned, Clone, Copy, Debug)]
    #[repr(C, packed)]
    pub struct ComboDirectoryHeader {
        pub(crate) cookie: [u8; 4], // b"2PSP" or b"2BHD"
        pub(crate) checksum: LU32, // 32-bit CRC value of header below this field and including all entries
        pub(crate) total_entries: LU32,
        pub(crate) lookup_mode: LU32 : pub get ComboDirectoryLookupMode : pub set ComboDirectoryLookupMode,
        _reserved: [u8; 16], // 0
    }
}

impl Default for ComboDirectoryHeader {
    fn default() -> Self {
        Self {
            cookie: *b"FIXM",
            checksum: 0.into(),
            lookup_mode: 0.into(),
            total_entries: 0.into(),
            _reserved: [0; 16],
        }
    }
}

impl DirectoryHeader for ComboDirectoryHeader {
    fn cookie(&self) -> [u8; 4] {
        self.cookie
    }
    fn set_cookie(&mut self, value: [u8; 4]) {
        self.cookie = value;
    }
    fn additional_info(&self) -> DirectoryAdditionalInfo {
        DirectoryAdditionalInfo::from(0)
    }
    fn set_additional_info(&mut self, _value: DirectoryAdditionalInfo) {}
    fn total_entries(&self) -> u32 {
        self.total_entries.get()
    }
    fn set_total_entries(&mut self, value: u32) {
        self.total_entries.set(value)
    }
    fn checksum(&self) -> u32 {
        self.checksum.get()
    }
    fn set_checksum(&mut self, value: u32) {
        self.checksum.set(value)
    }
}

impl ComboDirectoryHeader {
    pub fn new(cookie: [u8; 4]) -> Result<Self> {
        if cookie == *b"2PSP" || cookie == *b"2BHD" {
            let mut result = Self::default();
            result.cookie = cookie;
            Ok(result)
        } else {
            Err(Error::DirectoryTypeMismatch)
        }
    }
}

//#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum ComboDirectoryEntryFilter {
    PspId(u32),        // = 0,
    ChipFamilyId(u32), // = 1,
}

#[derive(Clone, Copy, FromBytes, AsBytes, Unaligned)]
#[repr(C, packed)]
pub struct ComboDirectoryEntry {
    pub(crate) internal_key: LU32, // 0-PSP ID; 1-chip family ID
    pub(crate) internal_value: LU32,
    pub(crate) internal_source: LU64, // that's the (Psp|Bhd) directory entry location. Note: If 32 bit high nibble is set, then that's a physical address
}

impl DirectoryEntrySerde for ComboDirectoryEntry {
    fn from_slice(source: &[u8]) -> Option<Self> {
        let result = LayoutVerified::<_, Self>::new_unaligned(source)?;
        Some(*result.into_ref())
    }
    fn copy_into_slice(&self, destination: &mut [u8]) {
        destination.copy_from_slice(&self.as_bytes())
    }
}

impl core::fmt::Debug for ComboDirectoryEntry {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // DirectoryRelativeOffset (WEAK_ADDRESS_MODE) is the only one that's always overridable.
        let internal_key = self.internal_key;
        let internal_value = self.internal_value;
        let source = self.source(WEAK_ADDRESS_MODE);
        fmt.debug_struct("ComboDirectoryEntry")
            .field("key", &internal_key)
            .field("value", &internal_value)
            .field("source", &source)
            .finish()
    }
}

// TODO: Remove.
impl Default for ComboDirectoryEntry {
    fn default() -> Self {
        Self {
            internal_key: 0.into(),
            internal_value: 0.into(),
            internal_source: 0.into(),
        }
    }
}

impl DirectoryEntry for ComboDirectoryEntry {
    // XXX
    fn source(
        &self,
        directory_address_mode: AddressMode,
    ) -> Result<ValueOrLocation> {
        let source = self.internal_source.get();
        ValueOrLocation::new_from_raw_location(directory_address_mode, source)
    }
    fn set_source(
        &mut self,
        directory_address_mode: AddressMode,
        value: ValueOrLocation,
    ) -> Result<()> {
        match value {
            ValueOrLocation::Value(_) => Err(Error::EntryTypeMismatch),
            x => {
                let v = x.try_into_raw_location(directory_address_mode)?;
                self.internal_source = v.into();
                Ok(())
            }
        }
    }
    fn size(&self) -> Option<u32> {
        None
    }
    fn set_size(&mut self, value: Option<u32>) {
        assert!(false);
    }
}

impl ComboDirectoryEntry {
    pub fn filter(&self) -> Result<ComboDirectoryEntryFilter> {
        let key = self.internal_key.get();
        let value = self.internal_value.get();
        match key {
            0 => Ok(ComboDirectoryEntryFilter::PspId(value)),
            1 => Ok(ComboDirectoryEntryFilter::ChipFamilyId(value)),
            _ => Err(Error::DirectoryTypeMismatch),
        }
    }
    pub fn set_filter(&mut self, value: ComboDirectoryEntryFilter) {
        match value {
            ComboDirectoryEntryFilter::PspId(value) => {
                self.internal_key.set(0);
                self.internal_value.set(value.into());
            }
            ComboDirectoryEntryFilter::ChipFamilyId(value) => {
                self.internal_key.set(0);
                self.internal_value.set(value.into());
            }
        }
    }
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn test_struct_sizes() {
        assert!(size_of::<EfhBulldozerSpiMode>() == 3);
        assert!(size_of::<EfhNaplesSpiMode>() == 3);
        assert!(size_of::<EfhRomeSpiMode>() == 3);
        assert!(size_of::<Efh>() < 0x100);
        assert!(size_of::<PspDirectoryHeader>() == 16);
        assert!(size_of::<PspDirectoryEntry>() == 16);
        assert!(size_of::<BhdDirectoryHeader>() == 16);
        assert!(size_of::<BhdDirectoryEntry>() == 24);
        assert!(size_of::<ComboDirectoryHeader>() == 32);
        assert!(size_of::<ComboDirectoryEntry>() == 16);
    }

    #[test]
    fn test_directory_additional_info() {
        let info = DirectoryAdditionalInfo::new()
            .with_spi_block_size_checked(
                DirectoryAdditionalInfo::try_into_unit(0x1_0000).unwrap(),
            )
            .unwrap();
        assert_eq!(u32::from(info), 0);

        let info = DirectoryAdditionalInfo::new()
            .with_spi_block_size_checked(
                DirectoryAdditionalInfo::try_into_unit(0x1000).unwrap(),
            )
            .unwrap();
        assert_eq!(u32::from(info), 1 << 10);

        let info = DirectoryAdditionalInfo::new()
            .with_spi_block_size_checked(
                DirectoryAdditionalInfo::try_into_unit(0x2000).unwrap(),
            )
            .unwrap();
        assert_eq!(u32::from(info), 2 << 10);

        let info = DirectoryAdditionalInfo::new()
            .with_spi_block_size_checked(
                DirectoryAdditionalInfo::try_into_unit(0xf000).unwrap(),
            )
            .unwrap();
        assert_eq!(u32::from(info), 0xf << 10);
    }

    #[test]
    #[should_panic]
    fn test_directory_additional_info_invalid() {
        let _info = DirectoryAdditionalInfo::new()
            .with_spi_block_size_checked(0)
            .unwrap();
    }
}
