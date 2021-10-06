// This file contains the AMD firmware Flash on-disk format.  Please only change it in coordination with the AMD firmware team.  Even then, you probably shouldn't.

use byteorder::LittleEndian;
use core::convert::TryInto;
use modular_bitfield::prelude::*;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use amd_flash::Location;
use crate::types::ValueOrLocation;
use crate::types::Error;
use crate::types::Result;
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
pub fn header_from_collection<'a, T: Sized + FromBytes>(buf: &'a [u8]) -> Option<&'a T> {
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
    _66_66MHz = 0,
    _33_33MHz = 1,
    _22_22MHz = 2,
    _16_66MHz = 3,
    _100MHz = 0b100,
    _800kHz = 0b101,
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
    pub psp_directory_table_location_naples: LU32, // usually unused
    pub psp_directory_table_location_zen: LU32,
    /// High nibble of model number is either 0 (Naples), 1 (Raven Ridge), or 3 (Rome).  Then, corresponding indices into BIOS_DIRECTORY_TABLES are 0, 1, 2, respectively.  Newer models always use BIOS_DIRECTORY_TABLE_MILAN instead.
    pub bios_directory_tables: [LU32; 3],
    pub(crate) second_gen_efs: LU32, // bit 0: All pointers are Flash MMIO pointers; should be clear for Rome
    pub bios_directory_table_milan: LU32, // or Combo
    _padding: LU32,
    pub promontory_firmware_location: LU32,
    pub low_power_promontory_firmware_location: LU32,
    _padding2: [LU32; 2], // at offset 0x38
    _reserved: [u8; 3], // SPI for family 15h; Note: micron_mode is reserved instead
    pub spi_mode_zen_naples: EfhNaplesSpiMode, // and Raven Ridge
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
            psp_directory_table_location_naples: 0.into(),
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

    pub fn second_gen_efs_for_processor_generation(generation: ProcessorGeneration) -> u32 {
        let generation: u8 = generation as u8;
        assert!(generation < 16);
        0xffff_fffe &! (1 << generation)
    }
}

#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy, BitfieldSpecifier)]
pub enum AddressMode {
    PhysicalAddress = 0,
    EfsRelativeOffset = 1, // x
    DirectoryRelativeOffset = 2, // (x - Base)
    ImageBaseRelativeOffset = 3, // x; ImageBaseRelativeOffset == DirectoryRelativeOffset; not Base
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub struct DirectoryAdditionalInfo {
    pub max_size: B10, // directory size in 4 kiB; Note: doc error in AMD docs
    #[skip(getters, setters)]
    spi_block_size: B4, // spi block size in 4 kiB; TODO: 0 = 64 kiB
    pub base_address: B15, // base address in 4 kiB; if the actual payload (the file contents) of the directory are somewhere else, this can specify where.
    #[bits = 2]
    pub address_mode: AddressMode, // FIXME: This should not be able to be changed (from/to 2 at least) as you are iterating over a directory--since the iterator has to interpret what it is reading relative to this setting
    #[skip]
    __: bool,
}

impl DirectoryAdditionalInfo {
    pub const UNIT: usize = 4096; // Byte
    pub fn with_spi_block_size_checked(&mut self, value: u16) -> core::result::Result<Self, modular_bitfield::error::OutOfBounds> {
        let mut result = *self;
        result.set_spi_block_size_checked(value)?;
        Ok(result)
    }
    pub fn with_spi_block_size(&mut self, value: u16) -> &mut Self {
        self.with_spi_block_size_checked(value);
        self
    }
    pub fn spi_block_size_or_err(&self) -> core::result::Result<u16, modular_bitfield::error::InvalidBitPattern<u8>> {
        let spi_block_size = ((u32::from(*self) >> 10) & 0xf) as u16;
        match spi_block_size {
            0 => Ok(0x10), // 64 kiB
            n => Ok(n),
        }
    }
    pub fn spi_block_size(&self) -> u16 {
        self.spi_block_size_or_err().unwrap()
    }
    pub fn set_spi_block_size_checked(&mut self, value: u16) -> core::result::Result<(), modular_bitfield::error::OutOfBounds> {
        let mut mask = u32::from(*self) &! 0b1111_0000000000;
        if value > 0 && value <= 15 {
            mask |= (value as u32) << 10;
        } else if value == 16 { // 64 kiB
        } else {
            return Err(modular_bitfield::error::OutOfBounds);
        }
        *self = Self::from(mask);
        Ok(())
    }
    pub fn set_spi_block_size(&mut self, value: u16) {
        self.set_spi_block_size_checked(value);
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

impl core::fmt::Debug for PspDirectoryHeader {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let checksum = self.checksum.get();
        let total_entries = self.total_entries.get();
        let additional_info = DirectoryAdditionalInfo::from(self.additional_info.get());
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
#[bits = 8]
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

/// For 32 MiB SPI Flash, which half ot map to MMIO 0xff00_0000.
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy, BitfieldSpecifier)]
pub enum PspSoftFuseChain32MiBSpiDecoding {
    LowerHalf = 0,
    UpperHalf = 1,
}

#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy, BitfieldSpecifier)]
pub enum PspSoftFuseChainPostCodeDecoding {
    Lpc = 0,
    Espi = 1,
}

#[bitfield(bits = 64)]
#[repr(u64)]
#[derive(Copy, Clone, Debug)]
pub struct PspSoftFuseChain {
    pub secure_debug_unlock: bool,
    #[skip]
    __: bool,
    pub early_secure_debug_unlock: bool,
    pub unlock_token_in_nvram: bool, // if the unlock token has been stored (by us) into NVRAM
    pub force_security_policy_loading_even_if_insecure: bool,
    pub load_diagnostic_bootloader: bool,
    pub disable_psp_debug_prints: bool,
    #[skip]
    __: B7,
    pub spi_decoding: PspSoftFuseChain32MiBSpiDecoding,
    pub postcode_decoding: PspSoftFuseChainPostCodeDecoding,
    #[skip]
    __: B12,
    #[skip]
    __: bool,
    pub skip_mp2_firmware_loading: bool,
    pub postcode_output_control_1byte: bool, // ???
    pub force_recovery_booting: bool,
    #[skip]
    __: B32,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub struct PspDirectoryEntryAttrs {
    #[bits = 8]
    pub type_: PspDirectoryEntryType,
    pub sub_program: B8, // function of AMD Family and Model; only useful for types 8, 0x24, 0x25
    pub rom_id: B2, // romid
    #[skip] __: B14,
}

#[derive(FromBytes, AsBytes, Unaligned, Clone, Copy)]
#[repr(C, packed)]
pub struct PspDirectoryEntry {
    pub attrs: LU32,
    size: LU32,
    source: LU64, // Note: value iff size == 0; otherwise location; TODO: (iff directory.address_mode == 2) entry address mode (top 2 bits), or 0
}

impl Default for PspDirectoryEntry {
    fn default() -> Self {
        Self {
            attrs: 0.into(),
            size: 0.into(),
            source: 0.into(),
        }
    }
}

impl PspDirectoryEntry {
    const SIZE_VALUE_MARKER: u32 = 0xFFFF_FFFF;
    pub fn type_(&self) -> PspDirectoryEntryType {
        let attrs = PspDirectoryEntryAttrs::from(self.attrs.get());
        attrs.type_()
    }
    pub fn sub_program(&self) -> u8 {
        let attrs = PspDirectoryEntryAttrs::from(self.attrs.get());
        attrs.sub_program()
    }
    pub fn rom_id(&self) -> u8 {
        let attrs = PspDirectoryEntryAttrs::from(self.attrs.get());
        attrs.rom_id()
    }
    pub fn source(&self) -> ValueOrLocation {
        let size = self.size.get();
        let source = self.source.get();
        let source = if size == Self::SIZE_VALUE_MARKER {
            ValueOrLocation::Value(source)
        } else {
            ValueOrLocation::Location(source)
        };
        source
    }
    pub fn size(&self) -> Option<u32> {
        let size = self.size.get();
        if size == Self::SIZE_VALUE_MARKER {
            None
        } else {
            Some(size)
        }
    }
    pub fn new_value(attrs: &PspDirectoryEntryAttrs, value: u64) -> Self {
        Self {
            attrs: u32::from(*attrs).into(),
            size: Self::SIZE_VALUE_MARKER.into(),
            source: value.into(),
        }
    }
    pub fn new_payload(attrs: &PspDirectoryEntryAttrs, size: u32, source: Location) -> Result<Self> {
        if size == Self::SIZE_VALUE_MARKER {
            Err(Error::PspDirectoryEntryTypeMismatch)
        } else {
            Ok(Self {
                attrs: u32::from(*attrs).into(),
                size: size.into(),
                source: u64::from(source).into(),
            })
        }
    }
}

impl core::fmt::Debug for PspDirectoryEntry {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let attrs = PspDirectoryEntryAttrs::from(self.attrs.get());
        let source = self.source();
        let size = self.size();
        fmt.debug_struct("PspDirectoryEntry")
           .field("attrs", &attrs)
           .field("size", &size)
           .field("source", &source)
           .finish()
    }
}


#[derive(FromBytes, AsBytes, Unaligned, Clone, Copy)]
#[repr(C, packed)]
pub struct BiosDirectoryHeader {
    pub(crate) cookie: [u8; 4], // b"$BHD" or b"$BL2"
    pub(crate) checksum: LU32, // 32-bit CRC value of header below this field and including all entries
    pub(crate) total_entries: LU32,
    pub(crate) additional_info: LU32,
}

impl DirectoryHeader for BiosDirectoryHeader {
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

impl core::fmt::Debug for BiosDirectoryHeader {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let checksum = self.checksum.get();
        let total_entries = self.total_entries.get();
        let additional_info = DirectoryAdditionalInfo::from(self.additional_info.get());
        fmt.debug_struct("BiosDirectoryHeader")
           .field("cookie", &self.cookie)
           .field("checksum", &checksum)
           .field("total_entries", &total_entries)
           .field("additional_info", &additional_info)
           .finish()
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq, FromPrimitive, Clone, Copy, BitfieldSpecifier)]
#[bits = 8]
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

#[derive(Copy, Clone, Debug, FromPrimitive, BitfieldSpecifier)]
#[bits = 8]
pub enum BiosDirectoryEntryRegionType {
    Normal = 0,
    Ta1 = 1,
    Ta2 = 2,
}

#[bitfield(bits = 32)]
#[repr(u32)]
#[derive(Copy, Clone, Debug)]
pub struct BiosDirectoryEntryAttrs {
    #[bits = 8]
    pub type_: BiosDirectoryEntryType,
    #[bits = 8]
    pub region_type: BiosDirectoryEntryRegionType,
    pub reset_image: bool,
    pub copy_image: bool,
    pub read_only: bool, // only useful for region_type > 0
    pub compressed: bool,
    pub instance: B4,
    pub sub_program: B3, // function of AMD Family and Model; only useful for types PMU firmware and APCB binaries
    pub rom_id: B2,
    #[skip] __: B3,
}

#[derive(FromBytes, AsBytes, Unaligned, Clone, Copy)]
#[repr(C, packed)]
pub struct BiosDirectoryEntry {
    pub attrs: LU32,
    size: LU32, // 0xFFFF_FFFF for value entry
    source: LU64, // value (or nothing) iff size == 0; otherwise source_location; TODO: (iff directory.address_mode == 2) entry address mode (top 2 bits), or 0
    pub destination_location: LU64, // 0xffff_ffff_ffff_ffff: none
}

impl Default for BiosDirectoryEntry {
    fn default() -> Self {
        Self {
            attrs: 0.into(),
            size: 0.into(),
            source: 0.into(),
            destination_location: 0xffff_ffff_ffff_ffff.into(),
        }
    }
}

impl BiosDirectoryEntry {
    pub fn type_(&self) -> BiosDirectoryEntryType {
        let attrs = BiosDirectoryEntryAttrs::from(self.attrs.get());
        attrs.type_()
    }

    pub fn region_type(&self) -> BiosDirectoryEntryRegionType {
        let attrs = BiosDirectoryEntryAttrs::from(self.attrs.get());
        attrs.region_type()
    }

    pub fn reset_image(&self) -> bool {
        let attrs = BiosDirectoryEntryAttrs::from(self.attrs.get());
        attrs.reset_image()
    }

    pub fn copy_image(&self) -> bool {
        let attrs = BiosDirectoryEntryAttrs::from(self.attrs.get());
        attrs.copy_image()
    }

    pub fn read_only(&self) -> bool {
        let attrs = BiosDirectoryEntryAttrs::from(self.attrs.get());
        attrs.read_only()
    }

    pub fn compressed(&self) -> bool {
        let attrs = BiosDirectoryEntryAttrs::from(self.attrs.get());
        attrs.compressed()
    }

    pub fn instance(&self) -> u8 {
        let attrs = BiosDirectoryEntryAttrs::from(self.attrs.get());
        attrs.instance()
    }

    pub fn sub_program(&self) -> u8 {
        let attrs = BiosDirectoryEntryAttrs::from(self.attrs.get());
        attrs.sub_program()
    }

    pub fn rom_id(&self) -> u8 {
        let attrs = BiosDirectoryEntryAttrs::from(self.attrs.get());
        attrs.rom_id()
    }

    pub fn source(&self) -> ValueOrLocation {
        let size = self.size.get();
        let source = self.source.get();
        let source = if size == 0xFFFF_FFFF { ValueOrLocation::Value(source) } else { ValueOrLocation::Location(source) };
        source
    }
    pub fn size(&self) -> Option<u32> {
        let size = self.size.get();
        if size == 0xFFFF_FFFF {
            None
        } else {
            Some(size)
        }
    }
    pub fn destination_location(&self) -> Option<u64> {
        let destination_location = self.destination_location.get();
        if destination_location == 0xffff_ffff_ffff_ffff {
            None
        } else {
            Some(destination_location)
        }
    }
}

impl core::fmt::Debug for BiosDirectoryEntry {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let source = self.source();
        let destination_location = self.destination_location();
        let attrs = BiosDirectoryEntryAttrs::from(self.attrs.get());
        let size = self.size();
        fmt.debug_struct("BiosDirectoryEntry")
           .field("attrs", &attrs)
           .field("size", &size)
           .field("source", &source)
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

    #[test]
    fn test_directory_additional_info() {
        let info = DirectoryAdditionalInfo::new().with_spi_block_size_checked(DirectoryAdditionalInfo::try_into_unit(0x1_0000).unwrap()).unwrap();
        assert_eq!(u32::from(info), 0);

        let info = DirectoryAdditionalInfo::new().with_spi_block_size_checked(DirectoryAdditionalInfo::try_into_unit(0x1000).unwrap()).unwrap();
        assert_eq!(u32::from(info), 1 << 10);

        let info = DirectoryAdditionalInfo::new().with_spi_block_size_checked(DirectoryAdditionalInfo::try_into_unit(0x2000).unwrap()).unwrap();
        assert_eq!(u32::from(info), 2 << 10);

        let info = DirectoryAdditionalInfo::new().with_spi_block_size_checked(DirectoryAdditionalInfo::try_into_unit(0xf000).unwrap()).unwrap();
        assert_eq!(u32::from(info), 0xf << 10);
    }

    #[test]
    #[should_panic]
    fn test_directory_additional_info_invalid() {
        let _info = DirectoryAdditionalInfo::new().with_spi_block_size_checked(0).unwrap();
    }
}
