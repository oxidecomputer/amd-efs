// This file contains the AMD firmware Flash on-disk format.  Please only change it in coordination with the AMD firmware team.  Even then, you probably shouldn't.

use byteorder::LittleEndian;
use num_derive::FromPrimitive;
use amd_flash::Location;
use zerocopy::{AsBytes, FromBytes, LayoutVerified, Unaligned, U16, U32, U64};

/// Given *BUF (a collection of multiple items), retrieves the first of the items and returns it.
/// If the item cannot be parsed, returns None.
pub fn header_from_collection_mut<'a, T: Sized + FromBytes + AsBytes>(buf: &'a mut [u8]) -> Option<&'a mut T> {
    match LayoutVerified::<_, T>::new_from_prefix(buf) {
        Some((item, _xbuf)) => {
            Some(item.into_mut())
        },
        None => None,
    }
}

type LU16 = U16<LittleEndian>;
type LU32 = U32<LittleEndian>;
type LU64 = U64<LittleEndian>;

// The first one is recommended by AMD; the last one is always used in practice.
pub const EMBEDDED_FIRMWARE_STRUCTURE_POSITION: [Location; 6] = [0xFA_0000, 0xF2_0000, 0xE2_0000, 0xC2_0000, 0x82_0000, 0x2_0000];

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy)]
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
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy)]
pub enum SpiFastSpeedNew {
    Speed66_66MHz = 0,
    Speed33_33MHz = 1,
    Speed22_22MHz = 2,
    Speed16_66MHz = 3,
    Speed100MHz = 0b100,
    Speed800kHz = 0b101,
    DoNothing = 0xff,
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy)]
pub enum SpiNaplesMicronMode {
    DummyCycle = 0x0a,
    DoNothing = 0xff,
}

#[derive(FromBytes, AsBytes, Unaligned, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct EfhNaplesSpiMode {
    read_mode: u8, // SpiReadMode or garbage
    fast_speed_new: u8, // SpiFastSpeedNew or garbage
    micron_mode: u8, // SpiNaplesMicronMode or garbage
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy)]
pub enum SpiRomeMicronMode {
    RomeSupportMicron = 0x55,
    RomeForceMicron = 0xaa,
    DoNothing = 0xff,
}

#[derive(FromBytes, AsBytes, Unaligned, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct EfhRomeSpiMode {
    read_mode: u8, // SpiReadMode or garbage
    fast_speed_new: u8, // SpiFastSpeedNew or garbage
    micron_mode: u8, // SpiRomeMicronMode or garbage
}

#[derive(FromBytes, AsBytes, Unaligned, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct Efh {
    pub signature: LU32, // 0x55aa_55aa
    pub imc_fw_location: LU32, // usually unused
    pub gbe_fw_location: LU32, // usually unused
    pub xhci_fw_location: LU32, // usually unused
    _psp_directory_table_location_early: LU32, // usually unused
    pub psp_directory_table_location_zen: LU32,
    pub bios_directory_tables: [LU32; 3], // Naples (usually unused), Newer (usually unused), Rome
    pub second_gen_efs: LU32, // bit 0: All pointers are Flash MMIO pointers; should be clear for Rome
    pub bios_directory_table_milan: LU32, // or Combo
    _padding: LU32,
    pub promontory_firmware_location: LU32,
    pub low_power_promontory_firmware_location: LU32,
    _padding2: [LU32; 2], // at offset 0x38
    _reserved: [u8; 3], // SPI for family 15h; Note: micron_mode is reserved instead
    pub spi_mode_zen_naples: EfhNaplesSpiMode,
    pub spi_mode_zen_rome: EfhRomeSpiMode,
    _reserved2: u8,
}

impl Default for Efh {
    fn default() -> Self {
        Self {
            signature: 0x55aa_55aa.into(),
            imc_fw_location: 0.into(),
            gbe_fw_location: 0.into(),
            xhci_fw_location: 0.into(),
            _psp_directory_table_location_early: 0.into(),
            psp_directory_table_location_zen: 0.into(), // probably invalid
            bios_directory_tables: [0.into(); 3], // probably invalid
            second_gen_efs: 0xffff_fffe.into(),
            bios_directory_table_milan: 0xffff_ffff.into(),
            _padding: 0xffff_ffff.into(),
            promontory_firmware_location: 0xffff_ffff.into(),
            low_power_promontory_firmware_location: 0xffff_ffff.into(),
            _padding2: [0xffff_ffff.into(); 2],
            _reserved: [0xff; 3],
            spi_mode_zen_naples: EfhNaplesSpiMode { read_mode: 0xff, fast_speed_new: 0xff, micron_mode: 0xff },
            spi_mode_zen_rome: EfhRomeSpiMode { read_mode: 0xff, fast_speed_new: 0xff, micron_mode: 0xff },
            _reserved2: 0,
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy)]
pub enum ProcessorGeneration {
    Milan = 1,
}

impl Efh {
    /// Precondition: signature needs to be there--otherwise you might be reading garbage in the first place
    pub fn second_gen_efs(&self) -> bool {
        self.second_gen_efs.get() & 1 == 0
    }

    /// Precondition: signature needs to be there--otherwise you might be reading garbage in the first place
    /// Note: generation 1 is Milan
    pub fn compatible_with_processor_generation(&self, generation: ProcessorGeneration) -> bool {
        let generation: u8 = generation as u8;
        assert!(generation < 16);
        self.second_gen_efs.get() & (1 << generation) == 0
    }
}

#[derive(FromBytes, AsBytes, Unaligned)]
#[repr(C, packed)]
pub struct PspDirectoryHeader {
    cookie: LU32, // fourcc "$PSP" or "$PL2"
    checksum: LU32, // 32-bit CRC value of header below this field and including all entries
    total_entries: LU32,
    additional_info: LU32, // 0xffff_ffff; or TODO: PSP Directory Table Additional Info Fields (9 bits: max size in blocks of 4 KiB; 4 bits: spi block size; 15 bits: [26:12] of Directory Image Base Address; 2 bits: address mode)
}

impl Default for PspDirectoryHeader {
    fn default() -> Self {
        Self {
            cookie: 0.into(),
            checksum: 0.into(),
            total_entries: 0.into(),
            additional_info: 0xffff_ffff.into(),
        }
    }
}

#[derive(FromBytes, AsBytes, Unaligned)]
#[repr(C, packed)]
pub struct PspDirectoryEntry {
    pub type_: u8,
    pub sub_program: u8,
    _reserved: LU16, // TODO: rom_id: u2; remainder: reserved
    size: LU32,
    value_or_location: LU64, // Note: value iff size == 0; otherwise location; TODO: (sometimes) entry address mode (2 bits)
}

impl Default for PspDirectoryEntry {
    fn default() -> Self {
        Self {
            type_: 0.into(),
            sub_program: 0.into(),
            _reserved: 0.into(),
            size: 0.into(),
            value_or_location: 0.into(),
        }
    }
}

#[derive(FromBytes, AsBytes, Unaligned)]
#[repr(C, packed)]
pub struct BiosDirectoryHeader {
    pub cookie: LU32, // fourcc "$BHD" or "$BL2"
    pub checksum: LU32, // 32-bit CRC value of header below this field and including all entries
    pub total_entry_count: LU32,
    _reserved: LU32,
}

impl Default for BiosDirectoryHeader {
    fn default() -> Self {
        Self {
            cookie: 0.into(),
            checksum: 0.into(),
            total_entry_count: 0.into(),
            _reserved: 0xffff_ffff.into(),
        }
    }
}

#[derive(FromBytes, AsBytes, Unaligned)]
#[repr(C, packed)]
pub struct BiosDirectoryEntry {
    pub type_: u8, // TODO: enum
    pub region_type: u8,
    pub flags: u8,
    pub sub_program: u8, // and reserved; default: 0
    size: LU32,
    value_or_source_location: LU64, // value (or nothing) iff size == 0; otherwise source_location
    pub destination_location: LU64, // 0xffff_ffff_ffff_ffff: none
}

impl Default for BiosDirectoryEntry {
    fn default() -> Self {
        Self {
            type_: 0xff,
            region_type: 0,
            flags: 0,
            sub_program: 0,
            size: 0.into(),
            value_or_source_location: 0.into(),
            destination_location: 0xffff_ffff_ffff_ffff.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn test_struct_sizes() {
        assert!(size_of::<EfhNaplesSpiMode>() == 3);
        assert!(size_of::<EfhRomeSpiMode>() == 3);
        assert!(size_of::<Efh>() < 0x100);
        assert!(size_of::<PspDirectoryHeader>() == 16);
        assert!(size_of::<PspDirectoryEntry>() == 16);
        assert!(size_of::<BiosDirectoryHeader>() == 16);
        assert!(size_of::<BiosDirectoryEntry>() == 24);
    }
}
