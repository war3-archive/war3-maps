# war3-mpq

Fork of [mpq](https://crates.io/crates/mpq) 0.8.1 by Michael Sierks, hardened for
reading **protected Warcraft III maps**. Upstream trusts values that come straight
out of the archive; map protectors falsify exactly those, so a reader that
believes them panics, over-allocates, or declares a perfectly good archive
invalid.

Fixes in this fork, each a separate commit:

| Fix | Upstream behaviour |
|---|---|
| Bounds-checked hash lookups | A hash entry's block index is used unchecked, so a crafted entry panics |
| Table counts clamped to the file | A 1.7 MB map declaring 33410 block entries fails on a short read |
| Validated sector offsets | Sector offsets may decrease or exceed the sector size, panicking the slice |
| Circular hash probing | Probing stops at the end of the table instead of wrapping, hiding files |
| Short-buffer decrypt guard | `data.len() - 3` underflows on buffers under four bytes |
| Fallback past bogus user data headers | A fake user data header points nowhere and aborts the open |
| Truncated hash tables read in full | A table running past EOF is rounded down to a power of two, dropping the entries above it |
| Guarded sector size shift | A shift of 65292 panics a debug build |

Over a 10365-map archive these took readable maps from 9218 to 9746.

Beyond the name-based path, two salvage entry points read archives whose tables
are gone entirely — see [docs/PROTECTED_MAPS.md](docs/PROTECTED_MAPS.md):

- `Archive::salvage_members` walks the data region member by member, using each
  member's own sector offset table instead of the block table. `war3-mpq -s FILE`
  prints what it finds.
- `carve::carve_sectors` scans raw bytes for the shape of a zlib MPQ sector, for
  archives whose member chain is broken too.

The API is unchanged from upstream, so `use mpq::…` keeps working through a
renamed dependency:

```toml
[dependencies]
mpq = { package = "war3-mpq", version = "0.9" }
```

Upstream is unmaintained as of this writing; the fixes are offered back if it
ever resumes. Licensed MIT/Apache-2.0, same as upstream.

---

## Upstream README

A library for reading MPQ archives.

## Reading an archive

```rust,no_run
extern crate mpq;

use std::str;
use mpq::Archive;

fn main() {
    let mut a = Archive::open("common.MPQ").unwrap();
    let file = a.open_file("(listfile)").unwrap();

    let mut buf: Vec<u8> = vec![0; file.size(&a) as usize];

    file.read(&mut a, &mut buf).unwrap();

    print!("{}", str::from_utf8(&buf).unwrap());
}
```

## CLI

### Build

```sh
git clone https://github.com/msierks/mpq-rust.git && cd mpq-rust && cargo build --release
```

### Run

print '(listfile)' contents:
```sh
target/release/mpq -l common.MPQ
```

extract file:
```
target/release/mpq -x "(listfile)" common.MPQ
```

More help:
```
target/release/mpq -h
```

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any 
additional terms or conditions.
