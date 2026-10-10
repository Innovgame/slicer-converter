macro_rules! define_chitubox_profiles {
    (@pipeline) => {
        &$crate::formats::common::DefaultBinaryPipeline
    };
    (@pipeline $pipeline:expr) => {
        $pipeline
    };
    (@default_extension $default_extension:literal, $($extension:literal),+) => {
        $default_extension
    };
    (@default_extension, $first_extension:literal $(, $extension:literal)*) => {
        $first_extension
    };
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
        runtime = binary {
            capabilities = $capabilities:expr,
            header_key = $header_key:expr,
            layer_codec_key = $layer_codec_key:expr,
            pixel_model_key = $pixel_model_key:expr,
            crypto_key = $crypto_key:expr,
            header = $header:expr,
            layer_codec = $layer_codec:expr,
            crypto = $crypto:expr
            $(, pipeline = $pipeline:expr)?
            $(,)?
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
                default_extension: define_chitubox_profiles!(
                    @default_extension $($default_extension)?, $($extension),+
                ),
                target_names: &[$($target_name),+],
                default_export: $default_export,
            };
            let runtime = $crate::formats::common::RuntimeProfileWiring {
                container: $crate::formats::common::ContainerKind::Binary,
                capabilities: $capabilities,
                keys: $crate::formats::common::FormatKeys {
                    family_key: $crate::formats::common::keys::FAMILY_CHITUBOX,
                    profile_key: $crate::formats::common::FormatKey(concat!(
                        "profile/",
                        $profile_id
                    )),
                    container_key: $crate::formats::common::keys::CONTAINER_BINARY,
                    header_key: $header_key,
                    layer_codec_key: $layer_codec_key,
                    pixel_model_key: $pixel_model_key,
                    crypto_key: $crypto_key,
                    footer_key: $crate::formats::common::keys::NO_FOOTER,
                    archive_key: $crate::formats::common::keys::NO_ARCHIVE,
                },
                header: $header,
                layer_codec: $layer_codec,
                crypto: $crypto,
                footer: &$crate::formats::common::NoFooter,
                archive: &$crate::formats::common::NoArchiveLayout,
                pipeline: define_chitubox_profiles!(@pipeline $($pipeline)?),
            };

            $crate::formats::common::format_spec_from_profile_parts(
                identity,
                $crate::formats::common::FormatFamily::Chitubox,
                catalog,
                runtime,
            )
        };
        pub static $profile: $crate::formats::common::FamilyProfile<
            $crate::formats::chitubox::specifics::ChituboxSpec,
        > = $crate::formats::common::FamilyProfile {
            spec: &$spec,
            facts: $facts,
        };
        pub static $catalog_profile: $crate::formats::catalog::CatalogProfile =
            $crate::formats::catalog::CatalogProfile::Chitubox(&$profile);
    };
}

macro_rules! define_chitubox_read_only_profiles {
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
        },
        facts = $facts:expr,
        runtime = binary {
            capabilities = $capabilities:expr,
            header_key = $header_key:expr,
            layer_codec_key = $layer_codec_key:expr,
            pixel_model_key = $pixel_model_key:expr,
            crypto_key = $crypto_key:expr,
            header = $header:expr,
            layer_codec = $layer_codec:expr,
            crypto = $crypto:expr
            $(, pipeline = $pipeline:expr)?
            $(,)?
        } $(,)?
    ) => {
        define_chitubox_profiles!(
            spec = $spec,
            profile = $profile,
            catalog_profile = $catalog_profile,
            identity = {
                name = $name,
                aliases = [$($alias),+],
                extensions = [$($extension),+],
            },
            catalog = {
                profile_id = $profile_id,
                display_name = $display_name,
                $(default_extension = $default_extension,)?
                target_names = [$($target_name),+],
                default_export = false,
            },
            facts = $facts,
            runtime = binary {
                capabilities = $capabilities,
                header_key = $header_key,
                layer_codec_key = $layer_codec_key,
                pixel_model_key = $pixel_model_key,
                crypto_key = $crypto_key,
                header = $header,
                layer_codec = $layer_codec,
                crypto = $crypto,
                $(pipeline = $pipeline,)?
            },
        );
    };
}

macro_rules! define_chitubox_ctb_profile {
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
        runtime = ctb {
            capabilities = $capabilities:expr,
            header_key = $header_key:expr,
            header = $header:expr $(,)?
        } $(,)?
    ) => {
        define_chitubox_profiles!(
            spec = $spec,
            profile = $profile,
            catalog_profile = $catalog_profile,
            identity = {
                name = $name,
                aliases = [$($alias),+],
                extensions = [$($extension),+],
            },
            catalog = {
                profile_id = $profile_id,
                display_name = $display_name,
                $(default_extension = $default_extension,)?
                target_names = [$($target_name),+],
                default_export = $default_export,
            },
            facts = $facts,
            runtime = binary {
                capabilities = $capabilities,
                header_key = $header_key,
                layer_codec_key = $crate::formats::common::keys::CTB_RLE,
                pixel_model_key = $crate::formats::common::keys::PIXEL_GRAYSCALE_8,
                crypto_key = $crate::formats::common::keys::CHITU_XOR,
                header = $header,
                layer_codec = &CTB_RLE_CODEC,
                crypto = &CHITUBOX_XOR_CRYPTO,
            },
        );
    };
}

macro_rules! define_chitubox_ctb_profile_parts {
    (
        header = $header:ident,
        facts = $facts:ident,
        kind = $kind:expr,
        ctb_version = $ctb_version:expr,
        machine_profile = $machine_profile:expr,
        allowed_magics = $allowed_magics:expr,
        allowed_versions = [$($allowed_version:literal),+ $(,)?],
        build_magic = $build_magic:expr,
        build_version = $build_version:expr,
        layer_table_entry_size = $layer_table_entry_size:literal $(,)?
    ) => {
        static $header: ChituboxHeaderLayout = ChituboxHeaderLayout {
            allowed_magic: $allowed_magics,
            allowed_versions: &[$($allowed_version),+],
            build_magic: Some($build_magic),
            build_version: Some($build_version),
            layer_table_entry_size: $layer_table_entry_size,
        };

        static $facts: ChituboxSpec = ChituboxSpec {
            kind: $kind,
            ctb_version: Some($ctb_version),
            machine_profile: $machine_profile,
            allowed_magics: $allowed_magics,
            allowed_versions: &[$($allowed_version),+],
            build_version: Some($build_version),
            layer_table_entry_size: Some($layer_table_entry_size),
        };
    };
}

macro_rules! define_chitubox_legacy_bitplane_profile_parts {
    (
        header = $header:ident,
        facts = $facts:ident,
        kind = $kind:expr,
        ctb_version = $ctb_version:expr,
        machine_profile = $machine_profile:expr,
        allowed_magics = $allowed_magics:expr,
        allowed_versions = [$($allowed_version:literal),+ $(,)?],
        build_version = $build_version:expr $(,)?
    ) => {
        static $header: ChituboxHeaderLayout = ChituboxHeaderLayout {
            allowed_magic: $allowed_magics,
            allowed_versions: &[$($allowed_version),+],
            build_magic: Some(ctb::MAGIC_CBDDLP),
            build_version: Some($build_version),
            layer_table_entry_size: 36,
        };

        static $facts: ChituboxSpec = ChituboxSpec {
            kind: $kind,
            ctb_version: Some($ctb_version),
            machine_profile: $machine_profile,
            allowed_magics: $allowed_magics,
            allowed_versions: &[$($allowed_version),+],
            build_version: Some($build_version),
            layer_table_entry_size: Some(36),
        };
    };
}

macro_rules! define_chitubox_legacy_bitplane_profile {
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
        runtime = legacy_bitplane {
            header = $header:expr $(,)?
        } $(,)?
    ) => {
        define_chitubox_profiles!(
            spec = $spec,
            profile = $profile,
            catalog_profile = $catalog_profile,
            identity = {
                name = $name,
                aliases = [$($alias),+],
                extensions = [$($extension),+],
            },
            catalog = {
                profile_id = $profile_id,
                display_name = $display_name,
                $(default_extension = $default_extension,)?
                target_names = [$($target_name),+],
                default_export = $default_export,
            },
            facts = $facts,
            runtime = binary {
                capabilities = $crate::formats::common::FormatCapabilities::binary_import_export()
                    .with_preview(true)
                    .with_anti_alias(true),
                header_key = $crate::formats::common::keys::HEADER_CHITU_LEGACY,
                layer_codec_key = $crate::formats::common::keys::CBDDLP_BITPLANE,
                pixel_model_key = $crate::formats::common::keys::PIXEL_BITPLANE_AA,
                crypto_key = $crate::formats::common::keys::NO_CRYPTO,
                header = $header,
                layer_codec = &CBDDLP_BITPLANE_CODEC,
                crypto = &$crate::formats::common::NoCrypto,
            },
        );
    };
}

macro_rules! define_chitubox_archive_profile {
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
                default_extension: define_chitubox_profiles!(
                    @default_extension $($default_extension)?, $($extension),+
                ),
                target_names: &[$($target_name),+],
                default_export: $default_export,
            };
            let runtime = $crate::formats::common::RuntimeProfileWiring {
                container: $crate::formats::common::ContainerKind::Archive,
                capabilities: $capabilities,
                keys: $crate::formats::common::FormatKeys {
                    family_key: $crate::formats::common::keys::FAMILY_CHITUBOX,
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
                $crate::formats::common::FormatFamily::Chitubox,
                catalog,
                runtime,
            )
        };
        pub static $profile: $crate::formats::common::FamilyProfile<
            $crate::formats::chitubox::specifics::ChituboxSpec,
        > = $crate::formats::common::FamilyProfile {
            spec: &$spec,
            facts: $facts,
        };
        pub static $catalog_profile: $crate::formats::catalog::CatalogProfile =
            $crate::formats::catalog::CatalogProfile::Chitubox(&$profile);
    };
}

pub(crate) use define_chitubox_archive_profile;
pub(crate) use define_chitubox_ctb_profile;
pub(crate) use define_chitubox_ctb_profile_parts;
pub(crate) use define_chitubox_legacy_bitplane_profile;
pub(crate) use define_chitubox_legacy_bitplane_profile_parts;
pub(crate) use define_chitubox_profiles;
pub(crate) use define_chitubox_read_only_profiles;
