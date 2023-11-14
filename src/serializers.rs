// This file contains the serializers for the ondisk formats.
// These are meant automatically make serde use a temporary serde-aware struct as a proxy when serializing/deserializing a non-serde-aware struct.
// Note that if too many fields are private, it means that those are not in the proxy struct in the first place. This might cause problems.
// Also, serialization can fail if the nice simple user-visible type cannot represent what we are doing.

#![cfg(feature = "serde")]

use crate::ondisk::*;
//use quote::quote;

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
                // TODO skip_serializing_if="has_error" on a regular struct with the Results in there. But how to then flatten the Ok case?
                // Also, how to have that depend on a flag like human_readable or some other flag?
                use serde::ser::SerializeStruct;
                let count = [$(stringify!($field_name),)*].len();
                let human_readable = serializer.is_human_readable();
                let mut state = serializer.serialize_struct(stringify!($StructName), count)?; // FIXME use serde renamed name, or schema_name if schemars
                $(
                    match self.[<serde_ $field_name>]() {
                        Ok(x) => {
                            state.serialize_field(stringify!($field_name), &x)?; // FIXME use serde renamed field name; but that's on the $Serde struct that we aren't using
                        },
                        Err(e) => {
                            let e = format!("textual representation of value for field '{}.{}' unknown: {}",
                                stringify!($StructName), stringify!($field_name), e);
                            // XXX: Proxy for "wants to have errors ignored"
                            if human_readable {
                                // Better to leave the field off entirely for clarity.
                                //state.serialize_field(stringify!($field_name), &e)?;
                                eprintln!("error: {}", e);
                                //std::os::set_exit_status(exitcode::DATAERR);
                            } else {
                                return Err(serde::ser::Error::custom(e))
                            }
                        }
                    }
                )*
                state.end()
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
