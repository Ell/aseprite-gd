//! User-data association verified against real fixtures whose contents are
//! self-describing (asefile's user_data.aseprite embeds its own expectations
//! in the strings; aseprite-tests' file-tests-props carries typed properties).

use std::path::PathBuf;

use ase_core::AseFile;
use ase_core::model::PropertyValue;

fn load(rel: &str) -> AseFile {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel);
    AseFile::parse(&std::fs::read(&path).unwrap()).unwrap()
}

#[test]
fn sprite_layer_cel_and_tag_user_data_attach_correctly() {
    let f = load("asefile/user_data.aseprite");

    assert_eq!(f.user_data.text.as_deref(), Some("test_user_data_sprite"));
    assert_eq!(f.user_data.color, Some([0, 255, 0, 255]));

    assert_eq!(
        f.layers[0].user_data.text.as_deref(),
        Some("test_user_data_layer")
    );
    assert_eq!(f.layers[0].user_data.color, Some([255, 0, 0, 255]));

    // Every frame's cel carries the same text.
    for frame in &f.frames {
        assert_eq!(
            frame.cels[0].user_data.text.as_deref(),
            Some("test_user_data_cel")
        );
    }

    // Tag user data arrives as N chunks after the tags chunk, in tag order.
    assert_eq!(
        f.tags[0].user_data.text.as_deref(),
        Some("test_user_data_tag_0")
    );
    assert_eq!(f.tags[0].user_data.color, Some([0, 255, 0, 255]));
    assert_eq!(f.tags[1].user_data.text, None);
    assert_eq!(
        f.tags[2].user_data.text.as_deref(),
        Some("test_user_data_tag_2")
    );
    assert_eq!(f.tags[2].user_data.color, Some([255, 0, 0, 255]));
}

#[test]
fn typed_properties_parse_including_extension_maps() {
    let f = load("aseprite-tests/file-tests-props.aseprite");

    // Map key 0 = user properties.
    let user = f.user_data.maps.iter().find(|m| m.key == 0).unwrap();
    let get = |name: &str| &user.properties.iter().find(|(n, _)| n == name).unwrap().1;
    assert_eq!(get("a"), &PropertyValue::Bool(true));
    assert_eq!(get("b"), &PropertyValue::I8(1));
    assert_eq!(get("c"), &PropertyValue::Str("hi".into()));
    assert_eq!(get("d"), &PropertyValue::F64(2.3));

    // Nonzero map key = extension properties (external files entry).
    let ext = f.user_data.maps.iter().find(|m| m.key != 0).unwrap();
    assert_eq!(
        ext.properties[0].1,
        PropertyValue::Vector(vec![
            PropertyValue::Str("one".into()),
            PropertyValue::Str("two".into()),
            PropertyValue::Str("three".into()),
        ])
    );
    assert!(
        !f.external_files.is_empty(),
        "extension key should have an external files entry"
    );

    // Layer and cel properties from the same file.
    assert!(f.layers.iter().any(|l| !l.user_data.maps.is_empty()));
    assert!(
        f.frames
            .iter()
            .flat_map(|fr| &fr.cels)
            .any(|c| !c.user_data.is_empty())
    );
}

#[test]
fn tag_repeat_counts_survive() {
    let f = load("aseprite-tests/tags3x123reps.aseprite");
    let reps: Vec<u16> = f.tags.iter().map(|t| t.repeat).collect();
    assert_eq!(reps, vec![1, 1, 2]);
}
