use war3parser::prelude::*;

fn load_map() -> &'static [u8] {
    include_bytes!("../../../test_data/Legion_TD_11.1c_TeamOZE.w3x")
}

#[test]
fn archive_lists_files_and_reads_members() {
    let mut w3x = War3MapW3x::from_buffer(load_map()).expect("failed to parse w3x");
    assert!(w3x.header.has_hm3w);
    // This map is protected and ships no `(listfile)`.
    assert!(w3x.files.is_none());

    assert!(w3x.has("war3map.w3i"));
    let data = w3x.read_file("war3map.w3i").expect("read w3i bytes");
    assert!(!data.is_empty());

    let map_info = w3x.read_map_info().expect("failed to read map info");
    assert!(!map_info.name.is_empty());

    assert!(w3x.get_script_file().is_some());

    let missing = w3x.read_file("no/such/file.txt");
    assert!(matches!(missing, Err(Error::FileNotFound(_))));
}
