// This file contains the AMD firmware Flash on-disk format.  Please only change it in coordination with the AMD firmware team.  Even then, you probably shouldn't.

use crate::flash::Location;
use crate::struct_accessors::make_accessors;
use crate::struct_accessors::DummyErrorChecks;
use crate::struct_accessors::Getter;
use crate::struct_accessors::Setter;
use crate::types::Error;
use crate::types::Result;
use byteorder::LittleEndian;
use core::convert::TryFrom;
use core::convert::TryInto;
use modular_bitfield::prelude::*;
use num_derive::FromPrimitive;
use num_derive::ToPrimitive;
use num_traits::FromPrimitive;
use num_traits::ToPrimitive;
use strum_macros::EnumString;
use zerocopy::{AsBytes, FromBytes, LayoutVerified, Unaligned, U32, U64};

//use crate::configs;

/// Given *BUF (a collection of multiple items), retrieves the first of the items and returns it.
/// If the item cannot be parsed, returns None.
pub fn header_from_collection_mut<T: Sized + FromBytes + AsBytes>(
    buf: &mut [u8],
) -> Option<&mut T> {
    LayoutVerified::<_, T>::new_from_prefix(buf)
        .map(|(item, _xbuf)| item.into_mut())
}

/// Given *BUF (a collection of multiple items), retrieves the first of the items and returns it.
/// If the item cannot be parsed, returns None.
pub fn header_from_collection<T: Sized + FromBytes>(buf: &[u8]) -> Option<&T> {
    LayoutVerified::<_, T>::new_from_prefix(buf)
        .map(|(item, _xbuf)| item.into_ref())
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
    #[cfg_attr(feature = "serde", serde(rename = "Normal up to 33.33 MHz"))]
    Normal33_33MHz = 0b000, // up to 33.33 MHz
    /// First digit of name: number of lines (bits) for the command
    /// Second digit of name: number of lines (bits) for the address
    /// Third digit of name: number of lines (bits) for the data
    Dual112 = 0b010,
    /// First digit of name: number of lines (bits) for the command
    /// Second digit of name: number of lines (bits) for the address
    /// Third digit of name: number of lines (bits) for the data
    Quad114 = 0b011,
    /// First digit of name: number of lines (bits) for the command
    /// Second digit of name: number of lines (bits) for the address
    /// Third digit of name: number of lines (bits) for the data
    Dual122 = 0b100,
    /// First digit: number of lines (bits) for the command
    /// Second digit: number of lines (bits) for the address
    /// Third digit: number of lines (bits) for the data
    Quad144 = 0b101,
    #[cfg_attr(feature = "serde", serde(rename = "Normal up to 66.66 MHz"))]
    Normal66_66MHz = 0b110, // up to 66.66 MHz
    Fast = 0b111,
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, ToPrimitive, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum SpiFastSpeedNew {
    #[cfg_attr(feature = "serde", serde(rename = "66.66 MHz"))]
    _66_66MHz = 0,
    #[cfg_attr(feature = "serde", serde(rename = "33.33 MHz"))]
    _33_33MHz = 1,
    #[cfg_attr(feature = "serde", serde(rename = "22.22 MHz"))]
    _22_22MHz = 2,
    #[cfg_attr(feature = "serde", serde(rename = "16.66 MHz"))]
    _16_66MHz = 3,
    #[cfg_attr(feature = "serde", serde(rename = "100 MHz"))]
    _100MHz = 0b100,
    #[cfg_attr(feature = "serde", serde(rename = "800 kHz"))]
    _800kHz = 0b101,
}

impl Getter<Result<[u32; 2]>> for [LU32; 2] {
    fn get1(self) -> Result<[u32; 2]> {
        let a = self[0].get();
        let b = self[1].get();
        Ok([a, b])
    }
}

impl Setter<[u32; 2]> for [LU32; 2] {
    fn set1(&mut self, value: [u32; 2]) {
        self[0].set(value[0]);
        self[1].set(value[1]);
    }
}

impl Getter<Result<[u32; 3]>> for [LU32; 3] {
    fn get1(self) -> Result<[u32; 3]> {
        let a = self[0].get();
        let b = self[1].get();
        let c = self[2].get();
        Ok([a, b, c])
    }
}

impl Setter<[u32; 3]> for [LU32; 3] {
    fn set1(&mut self, value: [u32; 3]) {
        self[0].set(value[0]);
        self[1].set(value[1]);
        self[2].set(value[2]);
    }
}

impl Getter<Result<[u8; 3]>> for [u8; 3] {
    fn get1(self) -> Result<[u8; 3]> {
        Ok(self)
    }
}

impl Getter<Result<[u8; 2]>> for [u8; 2] {
    fn get1(self) -> Result<[u8; 2]> {
        Ok(self)
    }
}

impl Setter<[u8; 2]> for [u8; 2] {
    fn set1(&mut self, value: [u8; 2]) {
        *self = value
    }
}

impl Getter<Result<[u8; 4]>> for [u8; 4] {
    fn get1(self) -> Result<[u8; 4]> {
        Ok(self)
    }
}

impl Setter<[u8; 4]> for [u8; 4] {
    fn set1(&mut self, value: [u8; 4]) {
        *self = value
    }
}

impl Getter<Result<[u8; 16]>> for [u8; 16] {
    fn get1(self) -> Result<[u8; 16]> {
        Ok(self)
    }
}

impl Setter<[u8; 16]> for [u8; 16] {
    fn set1(&mut self, value: [u8; 16]) {
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
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, ToPrimitive, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[non_exhaustive]
pub enum SpiRomeMicronMode {
    SupportMicron = 0x55,
    ForceMicron = 0xaa,
}

make_accessors! {
    #[derive(FromBytes, AsBytes, Unaligned, Clone, Copy, Debug)]
    #[repr(C, packed)]
    pub(crate) struct Efh {
        signature || u32 : LU32 | pub get u32 : pub set u32,                           // 0x55aa_55aa
        imc_fw_location || u32 : LU32 | pub get u32 : pub set u32,                     // usually unused
        gbe_fw_location || u32 : LU32 | pub get u32 : pub set u32,                     // usually unused
        xhci_fw_location || u32 : LU32 | pub get u32 : pub set u32,                    // usually unused
        psp_directory_table_location_naples || u32 : LU32 | pub get u32 : pub set u32, // usually unused
        psp_directory_table_location_zen || u32 : LU32 | pub get u32 : pub set u32,
        /// High nibble of model number is either 0 (Naples), 1 (Raven Ridge), or 3 (Rome).  Then, corresponding indices into BHD_DIRECTORY_TABLES are 0, 1, 2, respectively.  Newer models always use BHD_DIRECTORY_TABLE_MILAN instead.
        pub bhd_directory_tables || [u32; 3] : [LU32; 3],
        pub(crate) efs_generations || u32 : LU32, // bit 0: All pointers are Flash MMIO pointers; should be clear for Rome; bit 1: Clear for Milan
        bhd_directory_table_milan || u32 : LU32 | pub get u32 : pub set u32, // or Combo
        _padding || #[serde(default)] u32 : LU32,
        promontory_firmware_location || u32 : LU32 | pub get u32 : pub set u32,
        pub low_power_promontory_firmware_location || u32 : LU32 | pub get u32 : pub set u32,
        _padding2 || #[serde(default)] [u32; 2] : [LU32; 2],                      // at offset 0x38
        // Excavator, Merlin Falcon
        pub(crate) spi_mode_bulldozer : [u8; 3],
        pub(crate) spi_mode_zen_naples : [u8; 3], // and Raven Ridge
        _reserved1 || #[serde(default)] u8 : u8,
        pub(crate) spi_mode_zen_rome : [u8; 3],
        _reserved2 || #[serde(default)] u8 : u8,
        _reserved3 || #[serde(default)] u8 : u8,
        vendor_id: [u8; 2],
        vendor_board_id: [u8; 2],
        pub(crate) espi0_configuration: u8, // bit 0 = 1: invalid
        pub(crate) espi1_configuration: u8, // bit 0 = 1: invalid
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
            _padding: 0.into(),
            promontory_firmware_location: 0.into(),
            low_power_promontory_firmware_location: 0.into(),
            _padding2: [0.into(); 2],
            spi_mode_bulldozer: [0xff, 0xff, 0xff],
            spi_mode_zen_naples: [0xff /* Observed: 0 */, 0xff, 0xff],
            _reserved1: 0xff,
            spi_mode_zen_rome: [0xff, 0xff, 0xff],
            _reserved2: 0xff,
            _reserved3: 0xff,
            vendor_board_id: [0xff, 0xff],
            vendor_id: [0xff, 0xff],
            espi0_configuration: 0xff,
            espi1_configuration: 0xff,
        }
    }
}

#[cfg(test)]
#[test]
fn test_spi_mode_offsets() {
    use memoffset::offset_of;
    assert!(offset_of!(Efh, psp_directory_table_location_naples) == 0x10);
    assert!(offset_of!(Efh, psp_directory_table_location_zen) == 0x14);
    assert!(offset_of!(Efh, spi_mode_bulldozer) == 0x40);
    assert!(offset_of!(Efh, spi_mode_zen_naples) == 0x43);
    assert!(offset_of!(Efh, spi_mode_zen_rome) == 0x47);
}

#[derive(Debug, PartialEq, Clone, Copy, EnumString, strum_macros::EnumIter)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum ProcessorGeneration {
    Naples,
    Rome,
    Milan,
    Genoa,
    Turin,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[derive(Clone)]
pub struct EfhBulldozerSpiMode {
    pub read_mode: SpiReadMode,
    pub fast_speed_new: SpiFastSpeedNew,
}
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[derive(Clone)]
pub struct EfhNaplesSpiMode {
    pub read_mode: SpiReadMode,
    pub fast_speed_new: SpiFastSpeedNew,
    pub micron_mode: SpiNaplesMicronMode,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[derive(Clone)]
pub struct EfhRomeSpiMode {
    pub read_mode: SpiReadMode,
    pub fast_speed_new: SpiFastSpeedNew,
    pub micron_mode: SpiRomeMicronMode,
}

impl Efh {
    /// As a safeguard, this finds out whether the EFH position V is likely a
    /// flash location from the beginning of the flash.
    /// If it's more than 4 GB then we don't accept it (in that case, it's
    /// more likely to be an MMIO address or garbage).
    pub(crate) fn is_likely_location(v: u32) -> bool {
        v & 0xff00_0000 == 0
    }
    /// Precondition: signature needs to be there--otherwise you might be reading garbage in the first place.
    /// Old (pre-Rome) boards had MMIO addresses instead of offsets in the slots.  Find out whether that's the case.
    pub fn second_gen_efs(&self) -> bool {
        self.efs_generations.get() & 1 == 0
    }

    /// Precondition: signature needs to be there--otherwise you might be reading garbage in the first place.
    /// Old (pre-Rome) boards had MMIO addresses instead of offsets in the slots.  Find out whether that's the case.
    pub fn physical_address_mode(&self) -> bool {
        // Family 1Ah Models 00h–0Fh and 10h–1Fh does not clear bit 0 but expects offsets.
        if self.efs_generations.get()
            == Self::efs_generations_for_processor_generation(
                ProcessorGeneration::Turin,
            )
        {
            return false;
        }
        !self.second_gen_efs()
    }

    /// Given V which is possibly a MMIO address (from inside an
    /// EFH entry), convert it to a regular offset
    /// relative to the beginning of the flash.
    /// The result is None on error.
    pub(crate) fn de_mmio(
        v: u32,
        amd_physical_mode_mmio_size: Option<u32>,
    ) -> Option<Location> {
        if Efh::is_invalid_directory_table_location(v) {
            None
        } else if let Some(mmio_size) = amd_physical_mode_mmio_size {
            match mmio_decode(v, mmio_size) {
                Ok(v) => Some(v),
                Err(Error::DirectoryTypeMismatch) => {
                    // Rome is a grey-area that supports both MMIO addresses and offsets
                    (v < mmio_size).then_some(v)
                }
                Err(_) => None,
            }
        } else if Self::is_likely_location(v) {
            Some(v)
        } else {
            None
        }
    }
    pub fn is_invalid_directory_table_location(beginning: u32) -> bool {
        // AMD sometimes does 0 as well--even though that seems like a really
        // bad idea.
        beginning == 0xffff_ffff || beginning == 0
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
                // Bit 0 should be cleared (i.e. this is a second-gen EFS).
                self.efs_generations.get() == 0xffff_fffe
            }
            ProcessorGeneration::Milan | ProcessorGeneration::Genoa => {
                (self.efs_generations.get() & (1 << 0b0000)) == 0
            }
            ProcessorGeneration::Turin => {
                // XXX: Is Turin Model 00h-0Fh or 10h-1Fh? If the former, should be 0b0010 instead.
                (self.efs_generations.get() & (1 << 0b0011)) == 0
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
            ProcessorGeneration::Milan => 0xffff_fffc,
            ProcessorGeneration::Genoa => 0xffff_fffe,
            ProcessorGeneration::Turin => 0xffff_ffe3, // 0b1...00011
        }
    }

    pub fn spi_mode_bulldozer(&self) -> Result<Option<EfhBulldozerSpiMode>> {
        if self.spi_mode_bulldozer == [0xff, 0xff, 0xff] {
            Ok(None)
        } else {
            Ok(Some(EfhBulldozerSpiMode {
                read_mode: SpiReadMode::from_u8(self.spi_mode_bulldozer[0])
                    .ok_or(Error::Marshal)?,
                fast_speed_new: SpiFastSpeedNew::from_u8(
                    self.spi_mode_bulldozer[1],
                )
                .ok_or(Error::Marshal)?,
            }))
        }
    }

    pub fn set_spi_mode_bulldozer(
        &mut self,
        value: Option<EfhBulldozerSpiMode>,
    ) {
        self.spi_mode_bulldozer = value.map_or([0xff, 0xff, 0xff], |x| {
            [
                x.read_mode.to_u8().unwrap(),
                x.fast_speed_new.to_u8().unwrap(),
                0xff,
            ]
        });
    }

    pub fn spi_mode_zen_naples(&self) -> Result<Option<EfhNaplesSpiMode>> {
        if !self
            .compatible_with_processor_generation(ProcessorGeneration::Naples)
            || self.spi_mode_zen_naples == [0xff, 0xff, 0xff]
        {
            Ok(None)
        } else {
            Ok(Some(EfhNaplesSpiMode {
                read_mode: SpiReadMode::from_u8(self.spi_mode_zen_naples[0])
                    .ok_or(Error::Marshal)?,
                fast_speed_new: SpiFastSpeedNew::from_u8(
                    self.spi_mode_zen_naples[1],
                )
                .ok_or(Error::Marshal)?,
                micron_mode: SpiNaplesMicronMode::from_u8(
                    self.spi_mode_zen_naples[2],
                )
                .ok_or(Error::Marshal)?,
            }))
        }
    }

    pub fn set_spi_mode_zen_naples(&mut self, value: Option<EfhNaplesSpiMode>) {
        self.spi_mode_zen_naples = value.map_or([0xff, 0xff, 0xff], |x| {
            [
                x.read_mode.to_u8().unwrap(),
                x.fast_speed_new.to_u8().unwrap(),
                x.micron_mode.to_u8().unwrap(),
            ]
        });
    }

    pub fn spi_mode_zen_rome(&self) -> Result<Option<EfhRomeSpiMode>> {
        if self.spi_mode_zen_rome == [0xff, 0xff, 0xff] {
            Ok(None)
        } else {
            Ok(Some(EfhRomeSpiMode {
                read_mode: SpiReadMode::from_u8(self.spi_mode_zen_rome[0])
                    .ok_or(Error::Marshal)?,
                fast_speed_new: SpiFastSpeedNew::from_u8(
                    self.spi_mode_zen_rome[1],
                )
                .ok_or(Error::Marshal)?,
                micron_mode: SpiRomeMicronMode::from_u8(
                    self.spi_mode_zen_rome[2],
                )
                .ok_or(Error::Marshal)?,
            }))
        }
    }

    pub fn set_spi_mode_zen_rome(&mut self, value: Option<EfhRomeSpiMode>) {
        self.spi_mode_zen_rome = value.map_or([0xff, 0xff, 0xff], |x| {
            [
                x.read_mode.to_u8().unwrap(),
                x.fast_speed_new.to_u8().unwrap(),
                x.micron_mode.to_u8().unwrap(),
            ]
        });
    }

    pub fn espi0_configuration(&self) -> Result<Option<EfhEspiConfiguration>> {
        if self.espi0_configuration & 1 == 1 {
            Ok(None)
        } else {
            Ok(Some(EfhEspiConfiguration::from_bytes([
                self.espi0_configuration
            ])))
        }
    }

    pub fn set_espi0_configuration(
        &mut self,
        value: Option<EfhEspiConfiguration>,
    ) {
        self.espi0_configuration = match value {
            None => 0xff,
            Some(x) => x.into_bytes()[0],
        }
    }

    pub fn espi1_configuration(&self) -> Result<Option<EfhEspiConfiguration>> {
        if self.espi1_configuration & 1 == 1 {
            Ok(None)
        } else {
            Ok(Some(EfhEspiConfiguration::from_bytes([
                self.espi1_configuration
            ])))
        }
    }

    pub fn set_espi1_configuration(
        &mut self,
        value: Option<EfhEspiConfiguration>,
    ) {
        self.espi1_configuration = match value {
            None => 0xff,
            Some(x) => x.into_bytes()[0],
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

impl Default for AddressMode {
    fn default() -> Self {
        Self::EfsRelativeOffset
    }
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
            Self::Value(x) => write!(fmt, "Value({x:x?})"),
            Self::PhysicalAddress(x) => {
                write!(fmt, "PhysicalAddress({x:#x?})")
            }
            Self::EfsRelativeOffset(x) => {
                write!(fmt, "EfsRelativeOffset({x:#x?})")
            }
            Self::DirectoryRelativeOffset(x) => {
                write!(fmt, "DirectoryRelativeOffset({x:#x?})")
            }
            Self::OtherDirectoryRelativeOffset(x) => {
                write!(fmt, "OtherDirectoryRelativeOffset({x:#x?})")
            }
        }
    }
}

#[allow(dead_code)]
pub(crate) fn mmio_encode(
    value: Location,
    amd_physical_mode_mmio_size: Option<u32>,
) -> Result<u32> {
    let mmio_address_lower = match amd_physical_mode_mmio_size {
        None | Some(0) => return Err(Error::DirectoryTypeMismatch),
        Some(x) => 1 + (0xFFFF_FFFFu32 - x),
    };
    value.checked_add(mmio_address_lower).ok_or(Error::DirectoryTypeMismatch)
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
                    let v = u64::from(*x)
                        | if directory_address_mode
                            == AddressMode::DirectoryRelativeOffset
                            || directory_address_mode
                                == AddressMode::OtherDirectoryRelativeOffset
                        {
                            0x4000_0000_0000_0000
                        } else {
                            0
                        };
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

/// A variant of the make_accessors macro for modular_bitfields.
macro_rules! make_bitfield_serde {(
    $(#[$struct_meta:meta])*
    $struct_vis:vis
    struct $StructName:ident {
        $(
            $(#[$field_meta:meta])*
            $field_vis:vis
            $field_name:ident
            $(|| $(#[$serde_field_orig_meta:meta])* $serde_ty:ty : $field_orig_ty:ty)?
            $(: $field_ty:ty)?
            $(| $getter_vis:vis get $field_user_ty:ty $(: $setter_vis:vis set $field_setter_user_ty:ty)?)?
        ),* $(,)?
    }
) => {
    $(#[$struct_meta])*
    $struct_vis
    struct $StructName {
        $(
            $(#[$field_meta])*
            $field_vis
            $($field_name : $field_ty,)?
            $($field_name : $field_orig_ty,)?
        )*
    }

    impl $StructName {
        pub fn builder() -> Self {
            Self::new() // NOT default
        }
        pub fn build(&self) -> Self {
            self.clone()
        }
    }

    #[cfg(feature = "serde")]
    impl $StructName {
        $(
            paste::paste!{
                $(
                    #[allow(non_snake_case)]
                    pub(crate) fn [<serde_ $field_name>] (self : &'_ Self)
                        -> Result<$field_ty> {
                        Ok(self.$field_name())
                    }
                )?
                $(
                    #[allow(non_snake_case)]
                    pub(crate) fn [<serde_ $field_name>] (self : &'_ Self)
                        -> Result<$serde_ty> {
                        Ok(self.$field_name().into())
                    }
                )?
                $(
                    #[allow(non_snake_case)]
                    pub(crate) fn [<serde_with_ $field_name>]<'a>(self : &mut Self, value: $field_ty) -> &mut Self {
                        self.[<set_ $field_name>](value.into());
                        self
                    }
                )?
                $(
                    #[allow(non_snake_case)]
                    pub(crate) fn [<serde_with_ $field_name>]<'a>(self : &mut Self, value: $serde_ty) -> &mut Self {
                        self.[<set_ $field_name>](value.into());
                        self
                    }
                )?
            }
        )*
    }

    #[cfg(feature = "serde")]
    paste::paste! {
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
        #[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
        #[cfg_attr(feature = "serde", serde(rename = "" $StructName))]
        pub(crate) struct [<Serde $StructName>] {
            $(
                $(pub $field_name : <$field_ty as Specifier>::InOut,)?
                $($(#[$serde_field_orig_meta])* pub $field_name : $serde_ty,)?
            )*
        }
    }
}}

make_bitfield_serde! {
    #[bitfield(bits = 8)]
    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    pub struct EfhEspiConfiguration {
        #[skip(getters, setters)]
        invalid || #[serde(default)] bool : bool,
        pub enable_port_0x80 || bool : bool | pub get bool : pub set bool,
        pub alert_pin || u8 : B1 | pub get u8 : pub set u8,
        pub data_bus || u8 : B1 | pub get u8 : pub set u8,
        pub clock || u8 : B1 | pub get u8 : pub set u8,
        pub respond_port_0x80 || bool : bool | pub get bool : pub set bool,
        #[allow(non_snake_case)]
        _reserved_1 || #[serde(default)] u8 : B1,
        #[allow(non_snake_case)]
        _reserved_2 || #[serde(default)] u8 : B1,
    }
}

impl EfhEspiConfiguration {
    fn set_invalid(&mut self, value: bool) {
        assert!(!value);
    }
    fn invalid(&self) -> bool {
        false
    }
}

make_bitfield_serde! {
    #[bitfield(bits = 32)]
    #[repr(u32)]
    #[derive(Copy, Clone, Debug)]
    pub struct DirectoryAdditionalInfo {
        pub max_size || u16 : B10 | pub get u16 : pub set u16, // directory size in 4 kiB; Note: doc error in AMD docs // TODO: Shrink setter.
        #[skip(getters, setters)]
        pub spi_block_size || u16 : B4, // spi block size in 4 kiB; Note: 0 = 64 kiB
        pub base_address || u16 : B15 | pub get u16 : pub set u16, // base address in 4 kiB; if the actual payload (the file contents) of the directory are somewhere else, this can specify where. // TODO: Shrink setter.
        #[bits = 2]
        pub address_mode : AddressMode | pub get AddressMode : pub set AddressMode, // FIXME: This should not be able to be changed (from/to 2 at least) as you are iterating over a directory--since the iterator has to interpret what it is reading relative to this setting // TODO: Shrink setter.
        #[allow(non_snake_case)]
        _reserved_0 || #[serde(default)] bool : bool,
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
        let mut mask = u32::from(*self) & !0b11_1100_0000_0000;
        if value > 0 && value <= 15 {
            mask |= (value as u32) << 10;
        } else if value == 16 { // 64 kiB
        } else {
            return Err(modular_bitfield::error::OutOfBounds);
        }
        *self = Self::from(mask);
        Ok(())
    }
    // This is for serde only--so if serde were disabled, we'd get a warning.
    #[allow(dead_code)]
    pub(crate) fn set_spi_block_size(&mut self, value: u16) {
        self.set_spi_block_size_checked(value).unwrap() // FIXME error checking
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
    const ALLOWED_COOKIES: [[u8; 4]; 2];
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

impl PspDirectoryHeader {
    pub const FIRST_LEVEL_COOKIE: [u8; 4] = *b"$PSP";
    pub const SECOND_LEVEL_COOKIE: [u8; 4] = *b"$PL2";
}

impl DirectoryHeader for PspDirectoryHeader {
    const ALLOWED_COOKIES: [[u8; 4]; 2] = [*b"$PSP", *b"$PL2"];
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
#[derive(
    Debug,
    PartialEq,
    FromPrimitive,
    Clone,
    Copy,
    BitfieldSpecifier,
    EnumString,
    strum_macros::Display,
)]
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
    TeeIpKeyManagerDriver = 0x15,
    TeeSevDriver = 0x1a,
    TeeBootDriver = 0x1b,
    TeeSocDriver = 0x1c,
    TeeDebugDriver = 0x1d,
    TeeInterfaceDriver = 0x1f,
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
    MpioOffchipFirmware = 0x5D,
    RasDriver = 0x64,
    RasTrustedApplication = 0x65,
    TeeFhpDriver = 0x67,
    TeeSpdmDriver = 0x68,
    PspStage2Bootloader = 0x73,
    RegisterInitializationBinary = 0x76,
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
    MpdmaTigerfishFirmware = 0x8C,
    Gmi3PhyFirmware = 0x91,
    MpdmaPageMigrationFirmware = 0x92,
    AspSramFirmwareExtension = 0x9D,
    RegisterAccessWhitelist = 0x9F,
    S3Image = 0xA0,
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
        pub secure_debug_unlock || #[serde(default)] bool : bool | pub get bool : pub set bool,
        #[allow(non_snake_case)]
        _reserved_0 || #[serde(default)] bool : bool,
        pub early_secure_debug_unlock || #[serde(default)] bool : bool | pub get bool : pub set bool,
        pub unlock_token_in_nvram || #[serde(default)] bool : bool | pub get bool : pub set bool, // if the unlock token has been stored (by us) into NVRAM
        pub force_security_policy_loading_even_if_insecure || #[serde(default)] bool : bool | pub get bool : pub set bool,
        pub load_diagnostic_bootloader || #[serde(default)] bool : bool | pub get bool : pub set bool,
        pub disable_psp_debug_prints || #[serde(default)] bool : bool | pub get bool : pub set bool,
        #[allow(non_snake_case)]
        _reserved_1 || #[serde(default)] u8 : B7,
        pub spi_decoding || PspSoftFuseChain32MiBSpiDecoding : PspSoftFuseChain32MiBSpiDecoding | pub get PspSoftFuseChain32MiBSpiDecoding : pub set PspSoftFuseChain32MiBSpiDecoding,
        pub postcode_decoding || PspSoftFuseChainPostCodeDecoding : PspSoftFuseChainPostCodeDecoding | pub get PspSoftFuseChainPostCodeDecoding : pub set PspSoftFuseChainPostCodeDecoding,
        #[allow(non_snake_case)]
        _reserved_2 || #[serde(default)] u16 : B12,
        #[allow(non_snake_case)]
        _reserved_3 || #[serde(default)] bool : bool,
        pub skip_mp2_firmware_loading || #[serde(default)] bool : bool | pub get bool : pub set bool,
        pub postcode_output_control_1byte || #[serde(default)] bool : bool | pub get bool : pub set bool, // ???
        pub force_recovery_booting || #[serde(default)] bool : bool | pub get bool : pub set bool,
        #[allow(non_snake_case)]
        _reserved_4 || #[serde(default)] u32 : B32,
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
        pub type_: PspDirectoryEntryType | pub get PspDirectoryEntryType : pub set PspDirectoryEntryType,
        pub sub_program || u8 : B8 | pub get u8 : pub set u8, // function of AMD Family and Model; only useful for types 8, 0x24, 0x25
        pub rom_id: PspDirectoryRomId | pub get PspDirectoryRomId : pub set PspDirectoryRomId,
        pub writable: bool | pub get bool : pub set bool,
        pub instance || u8 : B4 | pub get u8 : pub set u8,
        #[allow(non_snake_case)]
        _reserved_0 || #[serde(default)] u16 : B9,
    }
}

make_accessors! {
    #[derive(FromBytes, AsBytes, Unaligned, Clone, Copy)]
    #[repr(C, packed)]
    pub struct PspDirectoryEntry {
        pub(crate) attrs || u32 : LU32,
        pub(crate) internal_size || u32 : LU32,
        // Note: value iff size == 0; otherwise location
        // Note: (iff directory.address_mode == 2)
        //   entry address mode (top 2 bits), or 0
        pub(crate) internal_source || u64 : LU64,
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
        destination.copy_from_slice(self.as_bytes())
    }
}

macro_rules! make_attr_proxy_with_fallible_getter {(
    $our_name:ident,
    $attr_name:ident,
    $attr_type:ty
) => {
    paste::paste! {
        pub fn [<$our_name _or_err>](&self) -> Result<$attr_type> {
            let attrs = <Self as Attributed>::Attrs::from(self.attrs.get());
            attrs.[<$attr_name _or_err>]().map_err(|_| Error::EntryTypeMismatch)
        }
        pub fn [<set_ $our_name>](&mut self, value: $attr_type) {
            let mut attrs = <Self as Attributed>::Attrs::from(self.attrs.get());
            attrs.[<set_ $attr_name>](value);
            self.attrs.set(u32::from_le_bytes(attrs.into_bytes()));
        }
        pub fn [<with_ $our_name>](&mut self, value: $attr_type) -> &mut Self {
            self.[<set_ $our_name>](value);
            self
        }
    }
}}
macro_rules! make_attr_proxy {
    (
    $our_name:ident,
    $attr_name:ident,
    $attr_type:ty
) => {
        make_attr_proxy_with_fallible_getter!(
            $our_name, $attr_name, $attr_type
        );
        paste::paste! {
            pub fn [<$our_name>](&self) -> $attr_type {
                self.[<$our_name _or_err>]().unwrap()
            }
        }
    };
}

trait Attributed {
    type Attrs;
}

impl Attributed for PspDirectoryEntry {
    type Attrs = PspDirectoryEntryAttrs;
}

impl PspDirectoryEntry {
    const SIZE_VALUE_MARKER: u32 = 0xFFFF_FFFF;
    make_attr_proxy_with_fallible_getter!(typ, type_, PspDirectoryEntryType);
    make_attr_proxy!(sub_program, sub_program, u8);
    make_attr_proxy!(instance, instance, u8);
    make_attr_proxy!(writable, writable, bool);
    make_attr_proxy_with_fallible_getter!(rom_id, rom_id, PspDirectoryRomId);
    pub fn new() -> Self {
        Self::default()
    }
    /// Note: Caller can modify other attributes using the with_ accessors.
    pub fn new_value(type_: PspDirectoryEntryType, value: u64) -> Result<Self> {
        let mut result = Self::new().with_typ(type_).build();
        result.internal_size = Self::SIZE_VALUE_MARKER.into();
        result.internal_source = value.into();
        Ok(result)
    }
    pub fn value(&self) -> Result<u64> {
        if self.internal_size.get() == Self::SIZE_VALUE_MARKER {
            Ok(self.internal_source.get())
        } else {
            Err(Error::EntryTypeMismatch)
        }
    }
    /// Note: Caller can modify other attributes using the with_ accessors.
    pub fn new_payload(
        directory_address_mode: AddressMode,
        type_: PspDirectoryEntryType,
        size: Option<u32>,
        source: Option<ValueOrLocation>,
    ) -> Result<Self> {
        let mut result = Self::new().with_typ(type_).build();
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

impl BhdDirectoryHeader {
    pub const FIRST_LEVEL_COOKIE: [u8; 4] = *b"$BHD";
    pub const SECOND_LEVEL_COOKIE: [u8; 4] = *b"$BL2";
}

impl DirectoryHeader for BhdDirectoryHeader {
    const ALLOWED_COOKIES: [[u8; 4]; 2] = [*b"$BHD", *b"$BL2"];
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
#[derive(
    Debug,
    PartialEq,
    FromPrimitive,
    Clone,
    Copy,
    BitfieldSpecifier,
    EnumString,
    strum_macros::Display,
)]
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
        pub type_: BhdDirectoryEntryType | pub get BhdDirectoryEntryType : pub set BhdDirectoryEntryType,
        #[bits = 8]
        pub region_type: BhdDirectoryEntryRegionType | pub get BhdDirectoryEntryRegionType : pub set BhdDirectoryEntryRegionType,
        pub reset_image || #[serde(default)] bool : bool | pub get bool : pub set bool,
        pub copy_image: bool | pub get bool : pub set bool,
        /// This field is only useful for
        /// region_type != BhdDirectoryEntryRegionType::Normal.
        pub read_only || #[serde(default)] bool : bool | pub get bool : pub set bool,
        pub compressed || #[serde(default)] bool : bool | pub get bool : pub set bool,
        pub instance || u8 : B4 | pub get u8 : pub set u8, // TODO: Shrink setter.
        // TODO: Shrink setter once possible (currently the libraries we use
        // to implement bitfields can't do that).
        /// A function of AMD Family and Model; only useful for types PMU
        /// firmware and APCB binaries.
        pub sub_program || #[serde(default)] u8 : B3 | pub get u8 : pub set u8,
        pub rom_id || #[serde(default)] BhdDirectoryRomId : BhdDirectoryRomId | pub get BhdDirectoryRomId : pub set BhdDirectoryRomId,
        #[allow(non_snake_case)]
        _reserved_0 || #[serde(default)] u8 : B3,
    }
}
make_accessors! {
    #[derive(FromBytes, AsBytes, Unaligned, Clone, Copy)]
    #[repr(C, packed)]
    pub struct BhdDirectoryEntry {
        attrs || u32 : LU32,
        pub(crate) internal_size || u32 : LU32,   // 0xFFFF_FFFF for value entry
        pub(crate) internal_source || u64 : LU64, // value (or nothing) iff size == 0; otherwise source_location; TODO: (iff directory.address_mode == 2) entry address mode (top 2 bits), or 0
        pub(crate) internal_destination_location || u64 : LU64, // 0xffff_ffff_ffff_ffff: none
    }
}

impl DirectoryEntrySerde for BhdDirectoryEntry {
    fn from_slice(source: &[u8]) -> Option<Self> {
        let result = LayoutVerified::<_, Self>::new_unaligned(source)?;
        Some(*result.into_ref())
    }
    fn copy_into_slice(&self, destination: &mut [u8]) {
        destination.copy_from_slice(self.as_bytes())
    }
}

impl Attributed for BhdDirectoryEntry {
    type Attrs = BhdDirectoryEntryAttrs;
}

impl BhdDirectoryEntry {
    const SIZE_VALUE_MARKER: u32 = 0xFFFF_FFFF;
    const DESTINATION_NONE_MARKER: u64 = 0xffff_ffff_ffff_ffff;

    pub fn destination_location(&self) -> Option<u64> {
        let destination_location = self.internal_destination_location.get();
        if destination_location == Self::DESTINATION_NONE_MARKER {
            None
        } else {
            Some(destination_location)
        }
    }
    make_attr_proxy_with_fallible_getter!(typ, type_, BhdDirectoryEntryType);
    make_attr_proxy_with_fallible_getter!(
        region_type,
        region_type,
        BhdDirectoryEntryRegionType
    );
    make_attr_proxy!(reset_image, reset_image, bool);
    make_attr_proxy!(copy_image, copy_image, bool);
    make_attr_proxy!(read_only, read_only, bool);
    make_attr_proxy!(compressed, compressed, bool);
    // Actually u4--but serde freaks out.
    // See <https://github.com/kjetilkjeka/uX/issues/17> for when we'd be able
    // to use u4. Even then, modular-bitfield has ::InOut in order to "upgrade"
    // to the next getter-/setter-able type--and that still won't use u4.
    make_attr_proxy!(instance, instance, u8);
    // Actually u3--but serde freaks out.
    // See <https://github.com/kjetilkjeka/uX/issues/17> for when we'd be able
    // to use u3. Even then, modular-bitfield has ::InOut in order to "upgrade"
    // to the next getter-/setter-able type--and that still won't use u3.
    make_attr_proxy!(sub_program, sub_program, u8);
    make_attr_proxy_with_fallible_getter!(rom_id, rom_id, BhdDirectoryRomId);
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
            .with_typ(type_)
            .with_internal_destination_location(match destination_location {
                None => Self::DESTINATION_NONE_MARKER,
                Some(x) => {
                    if x == Self::DESTINATION_NONE_MARKER {
                        return Err(Error::EntryTypeMismatch);
                    }
                    x
                }
            })
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
        pub(crate) checksum || u32 : LU32, // 32-bit CRC value of header below this field and including all entries
        pub(crate) total_entries || u32 : LU32,
        pub(crate) lookup_mode || u32 : LU32 | pub get ComboDirectoryLookupMode : pub set ComboDirectoryLookupMode,
        pub(crate) _reserved || #[cfg_attr(feature = "serde", serde(default))] [u8; 16] : [u8; 16], // 0
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
    const ALLOWED_COOKIES: [[u8; 4]; 2] = [*b"2PSP", *b"2BHD"];
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
    pub const PSP_COOKIE: [u8; 4] = *b"2PSP";
    pub const BHD_COOKIE: [u8; 4] = *b"2BHD";
    pub fn new(cookie: [u8; 4]) -> Result<Self> {
        if cookie == Self::PSP_COOKIE || cookie == Self::BHD_COOKIE {
            let result = Self { cookie, ..Default::default() };
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
        destination.copy_from_slice(self.as_bytes())
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
    fn set_size(&mut self, _value: Option<u32>) {
        todo!()
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
                self.internal_value.set(value);
            }
            ComboDirectoryEntryFilter::ChipFamilyId(value) => {
                self.internal_key.set(0);
                self.internal_value.set(value);
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
