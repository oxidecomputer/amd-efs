// This file contains the AMD firmware Flash on-disk format.  Please only change it in coordination with the AMD firmware team.  Even then, you probably shouldn't.

use core::mem::size_of;
use byteorder::LittleEndian;
use num_derive::FromPrimitive;
use zerocopy::{AsBytes, FromBytes, LayoutVerified, Unaligned, U16, U32, U64};

type LU16 = U16<LittleEndian>;
type LU32 = U32<LittleEndian>;
type LU64 = U64<LittleEndian>;

#[derive(FromBytes, AsBytes, Unaligned)]
#[repr(C, packed)]
pub struct EfhSpiMode {
    read_mode: u8,
    fast_speed_new: u8,
    micron_dummy_cycle: u8, // for Micron
}

#[derive(FromBytes, AsBytes, Unaligned)]
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
    _padding: [LU32; 2],
    pub promontory_firmware_location: LU32,
    pub low_power_promontory_firmware_location: LU32,
    _padding2: [LU32; 2], // at offset 0x38
    _reserved: EfhSpiMode, // SPI for family 15h; Note: micron_dummy_cycle is reserved instead
    pub spi_mode_zen_naples: EfhSpiMode,
    pub spi_mode_zen_rome: EfhSpiMode,
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
            _padding: [0xffff_ffff.into(); 2],
            promontory_firmware_location: 0xffff_ffff.into(),
            low_power_promontory_firmware_location: 0xffff_ffff.into(),
            _padding2: [0xffff_ffff.into(); 2],
            _reserved: EfhSpiMode { read_mode: 0xff, fast_speed_new: 0xff, micron_dummy_cycle: 0xff },
            spi_mode_zen_naples: EfhSpiMode { read_mode: 0xff, fast_speed_new: 0xff, micron_dummy_cycle: 0xff },
            spi_mode_zen_rome: EfhSpiMode { read_mode: 0xff, fast_speed_new: 0xff, micron_dummy_cycle: 0xff },
        }
    }
}

#[derive(FromBytes, AsBytes, Unaligned)]
#[repr(C, packed)]
pub struct PspDirectoryHeader {
    cookie: LU32, // fourcc "$PSP" or "$PL2"
    checksum: LU32, // 32-bit CRC value of header below this field and including all entries
    total_entries: LU32,
    _reserved: LU32, // 0xffff_ffff
}

impl Default for PspDirectoryHeader {
    fn default() -> Self {
        Self {
            cookie: 0.into(),
            checksum: 0.into(),
            total_entries: 0.into(),
            _reserved: 0xffff_ffff.into(),
        }
    }
}

#[derive(FromBytes, AsBytes, Unaligned)]
#[repr(C, packed)]
pub struct PspDirectoryEntry {
    pub type_: u8,
    pub sub_program: u8,
    _reserved: LU16,
    size: LU32,
    value_or_location: LU64, // Note: value iff size == 0; otherwise location
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

    #[test]
    fn test_struct_sizes() {
        assert!(size_of::<EfhSpiMode>() == 3);
        assert!(size_of::<Efh>() < 0x100);
        assert!(size_of::<PspDirectoryHeader>() == 16);
        assert!(size_of::<PspDirectoryEntry>() == 16);
        assert!(size_of::<BiosDirectoryHeader>() == 16);
        assert!(size_of::<BiosDirectoryEntry>() == 24);
    }
}
