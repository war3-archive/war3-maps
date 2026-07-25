//! HM3W container header (absent on pure-MPQ / some protected maps).

use crate::api_type;

api_type! {
    /// Fields from the optional `HM3W` prefix before the embedded MPQ.
    #[derive(Default)]
    pub struct War3MapHeader {
        pub has_hm3w: bool,
        pub name: Option<String>,
        pub flags: Option<u32>,
        pub max_players: Option<u32>,
        pub u1: Option<u32>,
    }
}
