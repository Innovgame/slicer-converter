//! Macro kit for declaring Anycubic profiles.
//!
//! The Anycubic family has many machine-specific binary profiles that share the
//! same shape: identity, catalog metadata, machine facts, runtime layout, and
//! layer codec. These macros keep that wiring in one declaration per profile
//! while still producing concrete `FormatSpec`, typed `FamilyProfile`, and
//! `CatalogProfile` statics.
//!
//! Anycubic family 有很多 machine-specific binary profiles，结构基本一致：identity、
//! catalog metadata、machine facts、runtime layout 与 layer codec。这组宏让每个 profile
//! 用一个声明完成 wiring，同时生成具体的 `FormatSpec`、typed `FamilyProfile` 与
//! `CatalogProfile` statics。

/// Declare one Anycubic binary machine profile.
///
/// The macro emits:
/// - a private [`AnycubicSpec`](crate::formats::anycubic::specifics::AnycubicSpec)
///   with physical machine facts and runtime implementations,
/// - header/layer codec adapters for the generic format pipeline,
/// - a public [`FormatSpec`](crate::formats::common::FormatSpec),
/// - a typed family profile and catalog profile.
///
/// 声明一个 Anycubic binary machine profile。该宏会生成 machine facts/runtime
/// implementations、generic pipeline adapters、公开 `FormatSpec`、typed family profile
/// 与 catalog profile。
macro_rules! define_anycubic_profiles {
    (@default_extension $default_extension:literal, $($extension:literal),+) => {
        $default_extension
    };
    (@default_extension, $first_extension:literal $(, $extension:literal)*) => {
        $first_extension
    };
    (
        facts_static = $specifics:ident,
        header_adapter = $header_adapter:ident,
        codec_adapter = $codec_adapter:ident,
        spec = $spec:ident,
        profile = $profile:ident,
        catalog_profile = $catalog_profile:ident,
        identity = {
            name = $name:literal,
            aliases = [$($identity_alias:literal),+ $(,)?],
            extensions = [$($extension:literal),+ $(,)?],
        },
        catalog = {
            profile_id = $profile_id:literal,
            display_name = $catalog_display_name:literal,
            $(default_extension = $default_extension:literal,)?
            target_names = [$($target_name:literal),+ $(,)?],
            default_export = $default_export:expr $(,)?
        },
        facts = {
            machine_id = $machine_id:path,
            display_name = $machine_display_name:literal,
            aliases = [$($machine_alias:literal),* $(,)?],
            resolution = ($resolution_x:literal, $resolution_y:literal),
            display_mm = ($display_width_mm:literal, $display_height_mm:literal),
            machine_z_mm = $machine_z_mm:literal,
            supported_versions = [$($version:path),+ $(,)?],
            default_version = $default_version:path,
            table_layout = $table_layout:path $(,)?
        },
        runtime = binary {
            layout = $layout:path,
            codec = $codec_impl:path,
            header_key = $header_key:expr,
            layer_codec_key = $codec_key:expr,
            capabilities = $capabilities:expr $(,)?
        } $(,)?
    ) => {
        static $specifics: $crate::formats::anycubic::specifics::AnycubicSpec =
            $crate::formats::anycubic::specifics::AnycubicSpec {
                machine: $crate::formats::anycubic::specifics::AnycubicMachineFacts {
                    machine_id: $machine_id,
                    display_name: $machine_display_name,
                    machine_name_aliases: &[$($machine_alias),*],
                    resolution_x: $resolution_x,
                    resolution_y: $resolution_y,
                    display_width_mm: $display_width_mm,
                    display_height_mm: $display_height_mm,
                    machine_z_mm: $machine_z_mm,
                },
                extension: $name,
                supported_versions: &[$($version),+],
                default_version: $default_version,
                table_layout: $table_layout,
                layout: &$layout,
                codec: &$codec_impl,
            };
        static $header_adapter: $crate::formats::anycubic::adapters::AnycubicHeaderLayoutAdapter =
            $crate::formats::anycubic::adapters::AnycubicHeaderLayoutAdapter {
                specifics: &$specifics,
            };
        static $codec_adapter: $crate::formats::anycubic::adapters::AnycubicLayerCodecAdapter =
            $crate::formats::anycubic::adapters::AnycubicLayerCodecAdapter {
                specifics: &$specifics,
            };

        pub static $spec: $crate::formats::common::FormatSpec = {
            let identity = $crate::formats::common::ProfileIdentityDescriptor {
                name: $name,
                aliases: &[$($identity_alias),+],
                extensions: &[$($extension),+],
            };
            let catalog = $crate::formats::common::ProfileCatalogDescriptor {
                profile_id: $profile_id,
                display_name: $catalog_display_name,
                default_extension: define_anycubic_profiles!(
                    @default_extension $($default_extension)?, $($extension),+
                ),
                target_names: &[$($target_name),+],
                default_export: $default_export,
            };
            let runtime = $crate::formats::common::RuntimeProfileWiring {
                container: $crate::formats::common::ContainerKind::Binary,
                capabilities: $capabilities,
                keys: $crate::formats::common::FormatKeys {
                    family_key: $crate::formats::common::keys::FAMILY_ANYCUBIC,
                    profile_key: $crate::formats::common::FormatKey(concat!(
                        "profile/",
                        $profile_id
                    )),
                    container_key: $crate::formats::common::keys::CONTAINER_BINARY,
                    header_key: $header_key,
                    layer_codec_key: $codec_key,
                    pixel_model_key: $crate::formats::common::keys::PIXEL_GRAYSCALE_8,
                    crypto_key: $crate::formats::common::keys::NO_CRYPTO,
                    footer_key: $crate::formats::common::keys::NO_FOOTER,
                    archive_key: $crate::formats::common::keys::NO_ARCHIVE,
                },
                header: &$header_adapter,
                layer_codec: &$codec_adapter,
                crypto: &$crate::formats::common::NoCrypto,
                footer: &$crate::formats::common::NoFooter,
                archive: &$crate::formats::common::NoArchiveLayout,
                pipeline: &$crate::formats::common::DefaultBinaryPipeline,
            };

            $crate::formats::common::format_spec_from_profile_parts(
                identity,
                $crate::formats::common::FormatFamily::Anycubic,
                catalog,
                runtime,
            )
        };
        pub static $profile: $crate::formats::common::FamilyProfile<
            $crate::formats::anycubic::specifics::AnycubicSpec,
        > = $crate::formats::common::FamilyProfile {
            spec: &$spec,
            facts: &$specifics,
        };
        pub static $catalog_profile: $crate::formats::catalog::CatalogProfile =
            $crate::formats::catalog::CatalogProfile::Anycubic(&$profile);
    };
}

/// Declare one Anycubic ZIP/archive profile.
///
/// Archive profiles use shared header/layer/archive implementations instead of a
/// machine-specific binary table layout, but still publish both runtime and
/// catalog profiles.
///
/// 声明一个 Anycubic ZIP/archive profile。Archive profiles 使用共享的
/// header/layer/archive implementations，而不是 machine-specific binary table layout；
/// 但仍会同时发布 runtime 与 catalog profiles。
macro_rules! define_anycubic_archive_profile {
    (
        spec = $spec:ident,
        profile = $profile:ident,
        catalog_profile = $catalog_profile:ident,
        identity = {
            name = $name:literal,
            aliases = [$($alias:literal),+ $(,)?],
            extensions = [$($extension:literal),+ $(,)?],
        },
        catalog = {
            profile_id = $profile_id:literal,
            display_name = $display_name:literal,
            $(default_extension = $default_extension:literal,)?
            target_names = [$($target_name:literal),+ $(,)?],
            default_export = $default_export:expr $(,)?
        },
        facts = $facts:expr,
        runtime = archive {
            capabilities = $capabilities:expr,
            header_key = $header_key:expr,
            layer_codec_key = $layer_codec_key:expr,
            pixel_model_key = $pixel_model_key:expr,
            archive_key = $archive_key:expr,
            header = $header:expr,
            layer_codec = $layer_codec:expr,
            archive = $archive:expr $(,)?
        } $(,)?
    ) => {
        pub static $spec: $crate::formats::common::FormatSpec = {
            let identity = $crate::formats::common::ProfileIdentityDescriptor {
                name: $name,
                aliases: &[$($alias),+],
                extensions: &[$($extension),+],
            };
            let catalog = $crate::formats::common::ProfileCatalogDescriptor {
                profile_id: $profile_id,
                display_name: $display_name,
                default_extension: define_anycubic_profiles!(
                    @default_extension $($default_extension)?, $($extension),+
                ),
                target_names: &[$($target_name),+],
                default_export: $default_export,
            };
            let runtime = $crate::formats::common::RuntimeProfileWiring {
                container: $crate::formats::common::ContainerKind::Archive,
                capabilities: $capabilities,
                keys: $crate::formats::common::FormatKeys {
                    family_key: $crate::formats::common::keys::FAMILY_ANYCUBIC,
                    profile_key: $crate::formats::common::FormatKey(concat!(
                        "profile/",
                        $profile_id
                    )),
                    container_key: $crate::formats::common::keys::CONTAINER_ARCHIVE,
                    header_key: $header_key,
                    layer_codec_key: $layer_codec_key,
                    pixel_model_key: $pixel_model_key,
                    crypto_key: $crate::formats::common::keys::NO_CRYPTO,
                    footer_key: $crate::formats::common::keys::NO_FOOTER,
                    archive_key: $archive_key,
                },
                header: $header,
                layer_codec: $layer_codec,
                crypto: &$crate::formats::common::NoCrypto,
                footer: &$crate::formats::common::NoFooter,
                archive: $archive,
                pipeline: &$crate::formats::common::DefaultArchivePipeline,
            };

            $crate::formats::common::format_spec_from_profile_parts(
                identity,
                $crate::formats::common::FormatFamily::Anycubic,
                catalog,
                runtime,
            )
        };
        pub static $profile: $crate::formats::common::FamilyProfile<
            $crate::formats::anycubic::specifics::AnycubicArchiveSpec,
        > = $crate::formats::common::FamilyProfile {
            spec: &$spec,
            facts: $facts,
        };
        pub static $catalog_profile: $crate::formats::catalog::CatalogProfile =
            $crate::formats::catalog::CatalogProfile::AnycubicArchive(&$profile);
    };
}

pub(crate) use define_anycubic_archive_profile;
pub(crate) use define_anycubic_profiles;
