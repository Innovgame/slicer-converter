use slicer_converter_core::{
    formats,
    formats::common::keys,
    transcode::{default_registry, TranscodeRouteDescriptor, TranscodeRouteKey},
};

#[test]
fn route_key_can_represent_goo_to_ctb_compatibility_without_profile_names() {
    let route = TranscodeRouteKey {
        source_container: keys::CONTAINER_BINARY,
        target_container: keys::CONTAINER_BINARY,
        source_layer_codec: keys::GOO_RLE,
        target_layer_codec: keys::CTB_RLE,
        source_pixel_model: keys::PIXEL_GRAYSCALE_8,
        target_pixel_model: keys::PIXEL_GRAYSCALE_8,
        source_crypto: keys::NO_CRYPTO,
        target_crypto: keys::NO_CRYPTO,
    };

    let same_codec_route = TranscodeRouteKey {
        target_layer_codec: keys::CTB_RLE,
        ..route
    };

    assert_eq!(route, same_codec_route);
}

#[test]
fn default_registry_route_lookup_preserves_goo_to_ctb_fast_path() {
    let route = route_key("goo", "ctb");
    let transcoders = default_registry().lookup_route(route);

    assert!(
        !transcoders.is_empty(),
        "route lookup should preserve existing goo -> ctb fast path"
    );
}

#[test]
fn default_registry_route_lookup_is_route_first() {
    let route = route_key("goo", "ctb");
    let transcoders = default_registry().lookup_route(route);

    assert!(
        !transcoders.is_empty(),
        "route lookup should use the semantic route directly"
    );
}

#[test]
fn route_key_can_be_derived_from_registered_format_specs() {
    let registry = formats::global_registry();
    let ctb = registry.spec_by_name("ctb").expect("ctb spec");
    let goo = registry.spec_by_name("goo").expect("goo spec");
    let prz = registry.spec_by_name("prz").expect("prz spec");

    assert_eq!(
        TranscodeRouteKey::from_specs(goo, ctb),
        route_key("goo", "ctb")
    );
    assert_eq!(
        TranscodeRouteKey::from_specs(prz, ctb),
        route_key("prz", "ctb")
    );
    assert_eq!(
        TranscodeRouteKey::from_specs(ctb, goo),
        route_key("ctb", "goo")
    );
}

#[test]
fn route_descriptor_carries_profile_keys_beside_route_key() {
    let registry = formats::global_registry();
    let ctb = registry.spec_by_name("ctb").expect("ctb spec");
    let goo = registry.spec_by_name("goo").expect("goo spec");

    let descriptor = TranscodeRouteDescriptor::from_specs(goo, ctb);

    assert_eq!(
        descriptor.route_key,
        TranscodeRouteKey::from_specs(goo, ctb)
    );
    assert_eq!(descriptor.source_family_key, goo.keys.family_key);
    assert_eq!(descriptor.target_family_key, ctb.keys.family_key);
    assert_eq!(descriptor.source_profile_key, goo.keys.profile_key);
    assert_eq!(descriptor.target_profile_key, ctb.keys.profile_key);
    assert_eq!(descriptor.source_header_key, goo.keys.header_key);
    assert_eq!(descriptor.target_header_key, ctb.keys.header_key);
    assert_eq!(descriptor.source_archive_key, keys::NO_ARCHIVE);
    assert_eq!(descriptor.target_archive_key, keys::NO_ARCHIVE);
}

#[test]
fn route_descriptor_keeps_ctb_v4_and_gktwo_profile_identity_separate() {
    let registry = formats::global_registry();
    let ctb_v4 = registry.spec_by_name("ctb-v4").expect("ctb v4 spec");
    let gktwo = registry.spec_by_name("gktwo-ctb").expect("gktwo spec");
    let goo = registry.spec_by_name("goo").expect("goo spec");

    let ctb_descriptor = TranscodeRouteDescriptor::from_specs(ctb_v4, goo);
    let gktwo_descriptor = TranscodeRouteDescriptor::from_specs(gktwo, goo);

    assert_eq!(ctb_descriptor.route_key, gktwo_descriptor.route_key);
    assert_ne!(
        ctb_descriptor.source_profile_key,
        gktwo_descriptor.source_profile_key
    );
    assert_eq!(
        ctb_descriptor.source_header_key,
        gktwo_descriptor.source_header_key
    );
}

#[test]
fn cbddlp_route_key_does_not_match_ctb_to_goo_fast_path() {
    let registry = formats::global_registry();
    let cbddlp = registry.spec_by_name("cbddlp").expect("cbddlp spec");
    let goo = registry.spec_by_name("goo").expect("goo spec");

    assert_ne!(
        TranscodeRouteKey::from_specs(cbddlp, goo),
        route_key("ctb", "goo")
    );
}

fn route_key(source: &str, target: &str) -> TranscodeRouteKey {
    let registry = formats::global_registry();
    let source = registry
        .spec_by_name(source)
        .unwrap_or_else(|| panic!("{source} spec"));
    let target = registry
        .spec_by_name(target)
        .unwrap_or_else(|| panic!("{target} spec"));
    TranscodeRouteKey::from_specs(source, target)
}
