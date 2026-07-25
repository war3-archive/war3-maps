use war3parser::parser::w3x::War3MapW3x;

fn load_map() -> &'static [u8] {
    include_bytes!("../../../test_data/Legion_TD_11.1c_TeamOZE.w3x")
}

#[test]
fn test_w3x_parse() {
    let map = load_map();
    let w3x = War3MapW3x::from_buffer(map).expect("failed to parse w3x");

    let mut w3x_box = Box::new(w3x);
    eprintln!("w3x files: {:?}", w3x_box.files);

    let map_info = w3x_box.read_map_info().expect("failed to read map info");
    eprintln!("map info: {:#?}", map_info);
}
