// This file contains the serializers for the ondisk formats.
// These are meant automatically make serde use a temporary serde-aware struct as a proxy when serializing/deserializing a non-serde-aware struct.
// Note that if too many fields are private, it means that those are not in the proxy struct in the first place. This might cause problems.
// Also, serialization can fail if the nice simple user-visible type cannot represent what we are doing.

#![cfg(feature = "serde")]

use crate::ondisk::*;
use quote::quote;

// Note: This is written such that it will fail if the underlying struct has fields added/removed/renamed--if those have a public setter.
macro_rules! make_serde{($StructName:ident, $SerdeStructName:ident, [$($field_name:ident),* $(,)?]
) => (
    paste::paste!{
        #[cfg(feature = "serde")]
        impl<'de> serde::de::Deserialize<'de> for $StructName {
            fn deserialize<D>(deserializer: D) -> core::result::Result<Self, D::Error>
            where D: serde::de::Deserializer<'de>, {
                let config = $SerdeStructName::deserialize(deserializer)?;
                Ok($StructName::builder()
                $(
                .[<serde_with_ $field_name>](config.$field_name.into())
                )*.build())
                }
        }
        #[cfg(feature = "serde")]
        impl serde::Serialize for $StructName {
            fn serialize<S>(&self, serializer: S) -> core::result::Result<S::Ok, S::Error>
            where S: serde::Serializer, {
                $SerdeStructName {
                    $(
                        $field_name: self.[<serde_ $field_name>]()
                            .map_err(|_| serde::ser::Error::custom(format!("textual representation of value {:?} for field {:?}.{:?} unknown",
                                self.[<$field_name>](), quote!($StructName), quote!($field_name))))?
                            .into(),
                    )*
                }.serialize(serializer)
            }
        }
        #[cfg(feature = "schemars")]
        impl schemars::JsonSchema for $StructName {
            fn schema_name() -> String {
                $SerdeStructName::schema_name()
            }
            fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
                $SerdeStructName::json_schema(gen)
            }
            fn is_referenceable() -> bool {
                $SerdeStructName::is_referenceable()
            }
        }
    }
)}

make_serde!(
    DirectoryAdditionalInfo,
    SerdeDirectoryAdditionalInfo,
    [max_size, spi_block_size, base_address, address_mode, _reserved_0,]
);
make_serde!(
    PspSoftFuseChain,
    SerdePspSoftFuseChain,
    [
        secure_debug_unlock,
        _reserved_0,
        early_secure_debug_unlock,
        unlock_token_in_nvram,
        force_security_policy_loading_even_if_insecure,
        load_diagnostic_bootloader,
        disable_psp_debug_prints,
        _reserved_1,
        spi_decoding,
        postcode_decoding,
        _reserved_2,
        _reserved_3,
        skip_mp2_firmware_loading,
        postcode_output_control_1byte,
        force_recovery_booting,
        _reserved_4,
    ]
);
