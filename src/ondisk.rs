// This file contains the AMD firmware Flash on-disk format.  Please only change it in coordination with the AMD firmware team.  Even then, you probably shouldn't.

use byteorder::LittleEndian;
use modular_bitfield::prelude::*;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
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

/// Given *BUF (a collection of multiple items), retrieves the first of the items and returns it.
/// If the item cannot be parsed, returns None.
pub fn header_from_collection<'a, T: Sized + FromBytes + AsBytes>(buf: &'a [u8]) -> Option<&'a T> {
    match LayoutVerified::<_, T>::new_from_prefix(buf) {
        Some((item, _xbuf)) => {
            Some(item.into_ref())
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
    second_gen_efs: LU32, // bit 0: All pointers are Flash MMIO pointers; should be clear for Rome
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

#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Copy, Clone)]
pub struct DirectoryAdditionalInfo {
    pub max_size: B10, // directory size in 4 KiB; Note: doc error in AMD docs
    pub spi_block_size: B4, // spi block size in 4 KiB
    pub base_address: B15, // base address in 4 KiB
    pub address_mode: B2, // 0: physical memory address; 1: address relative to the entire BIOS image; 2: address relative to base_address above FIXME is that true?; 3: TODO
    #[skip]
    __: bool,
}


#[derive(FromBytes, AsBytes, Unaligned, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct PspDirectoryHeader {
    pub(crate) cookie: [u8; 4], // b"$PSP" or b"$PL2"
    pub(crate) checksum: LU32, // 32-bit CRC value of header below this field and including all entries
    pub(crate) total_entries: LU32,
    pub(crate) additional_info: LU32, // 0xffff_ffff; or DirectoryAdditionalInfo
}

impl Default for PspDirectoryHeader {
    fn default() -> Self {
        Self {
            cookie: *b"    ", // invalid
            checksum: 0.into(), // invalid
            total_entries: 0.into(),
            additional_info: 0xffff_ffff.into(), // invalid
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy)]
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
    L2aPspDirectory = 0x48,
    L2BiosDirectory = 0x49,
    L2bPspDirectory = 0x4A,
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
    TosSplTable = 0x56, // used by off-chip bootloader
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

#[derive(FromBytes, AsBytes, Unaligned, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct PspDirectoryEntry {
    pub type_: u8,
    pub sub_program: u8, // function of AMD Family and Model; only useful for types 8, 0x24, 0x25
    _reserved: LU16, // TODO: rom_id: u2; remainder: reserved
    size: LU32,
    value_or_location: LU64, // Note: value iff size == 0; otherwise location; TODO: (sometimes) entry address mode (2 bits) or 0
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

#[derive(FromBytes, AsBytes, Unaligned, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct BiosDirectoryHeader {
    pub cookie: [u8; 4], // b"$BHD" or b"$BL2"
    pub checksum: LU32, // 32-bit CRC value of header below this field and including all entries
    pub total_entries: LU32,
    additional_info: LU32,
}

impl Default for BiosDirectoryHeader {
    fn default() -> Self {
        Self {
            cookie: *b"    ", // invalid
            checksum: 0.into(), // invalid
            total_entries: 0.into(),
            additional_info: 0xffff_ffff.into(), // invalid
        }
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy)]
pub enum BiosDirectoryEntryType {
    OemPublicKey = 0x05,
    CryptographicSignature = 0x07,
    Apcb = 0x60,
    Apob = 0x61,
    Bios = 0x62,
    ApobNvCopy = 0x63, // used during S3 resume
    PmuFirmwareInstructions = 0x64,
    PmuFirmwareData = 0x65,
    MicrocodePatch = 0x66,
    MceData = 0x67,
    ApcbBackup = 0x68,
    VgaInterpreter = 0x69,
    Mp2FirmwareConfiguration = 0x6A,
    CorebootVbootWorkbuffer = 0x6B, // main memory shared between PSP and x86
    MpmConfiguration = 0x6C,
    SecondLevelDirectory = 0x70, // also a BiosDirectory
}

#[derive(FromBytes, AsBytes, Unaligned, Clone, Copy)]
#[repr(C, packed)]
pub struct BiosDirectoryEntry {
    pub type_: u8, // TODO: enum
    pub region_type: u8,
    pub flags: u8,
    pub sub_program: u8, // and reserved; function of AMD Family and Model; only useful for types PMU firmware and APCB binaries
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

impl core::fmt::Debug for BiosDirectoryEntry {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let type_ = BiosDirectoryEntryType::from_u8(self.type_);
        let size = self.size.get();
        let value_or_source_location = self.value_or_source_location.get();
        let destination_location = self.destination_location.get();
        fmt.debug_struct("BiosDirectoryEntry")
           .field("type_", &type_)
           .field("region_type", &self.region_type) // FIXME
           .field("flags", &self.flags) // FIXME
           .field("sub_program", &self.sub_program)
           .field("size", &size)
           .field("value_or_source_location", &value_or_source_location)
           .field("destination_location", &destination_location)
           .finish()
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
