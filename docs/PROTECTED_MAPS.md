# Protected and damaged Warcraft III archives

Notes from working through the maps in the war3-maps dataset that no name-based
reader can open. Everything here was measured on the 10,365-map corpus; the
counts refer to the 301 archives that `war3parser` could not read by name
(223 recovered by sector carving, 78 outright failures) as of 2026-08-14.

That the 301 are a hard floor and not a gap in this reader was checked against
the other implementations: StormLib reads 2 of them and `mdx-m3-viewer` 5, with
all four MPQ libraries measured — StormLib, `mdx-m3-viewer`, `mpq-rs`, `wow-mpq`
— together reaching 7. Over the whole corpus StormLib and this reader agree
exactly (9,775 of 10,076 maps each, differing on 2 in either direction), so the
recoveries below are the only route left.

## The classic protection folklore, checked

Chinese map-protection writeups from the mid-2000s circulate a recipe built on
two claims about MPQ:

1. an archive does not reveal which files it contains, and
2. a file can only be reached if you know its name.

Both are about the hash table being a one-way index, and both are weaker than
they sound.

**Claim 1 is true but nearly irrelevant for `w3x`.** The name set is fixed —
`war3map.w3i`, `war3map.j`, `war3mapMap.blp`, and so on — and imported files are
listed by name inside `war3map.imp`. A reader that knows the format never needs
to enumerate.

**Claim 2 is false.** The block table alone gives offset, packed size and flags,
so member *bytes* can be read without any name at all; only the decryption key
of an encrypted member is name-derived, and even that key is recoverable from
known plaintext (see below). The sector data is also self-describing enough to
be carved directly — an MPQ sector is a one-byte compression mask followed by the
stream, so a zlib member starts with the literal bytes `02 78 9C`.

The recipe itself — inflate `dwBlockTableSize` past `dwHashTableSize`, resurrect
dead hash/block entries, enlarge `dwHashTableSize` so a brute-force listfile
crawls — only ever attacked *enumerators*. Warcraft III reads the files it wants
by name and never enumerates, which is why the protected maps still played. Two
details in that recipe are real and worth mirroring:

- **`dwHeaderSize` is only checked against `0x20`.** Storm accepts any header
  with `dwHeaderSize >= 0x20`; tools that demand exactly `0x20` are the ones the
  trick killed. 72 of the 301 carry a vandalised size — 27 of them the same
  `0x504F7856` (`"VxOP"` in file order), 8 of them `0xFFFFFFFF`.
  `Archive::load` mirrors Storm.
- **A truncated hash table must not be rounded down.** `HashString(name, 0) &
  (dwHashTableSize - 1)` is what Storm computes. This reader used to clamp a
  table that runs past EOF to the largest power of two that fits, so that the
  mask stayed valid — which threw away every entry above that power of two, and
  with it any file whose slot landed there. It now reads every slot the file
  holds and keeps the declared count for the mask.

How much of the recipe is actually present in the 301 hard maps:

| Trick | Count |
|---|---|
| `dwBlockTableSize > dwHashTableSize` | 10 |
| Declared block count past EOF | 9 |
| Declared hash count past EOF | 2 |
| Vandalised `dwHeaderSize` | 72 |
| A second, decoy `MPQ\x1A` header | 1 |

So the folklore recipe explains almost none of this corpus, and the parts it does
explain were already handled.

## What is actually wrong with these archives

For 284 of the 295 that have a usable header, the bytes at the declared hash
table position do not decrypt to a hash table, and 276 resolve not one known
Warcraft III filename.

That is worth stating precisely, because "the table is encrypted with another
key" is the obvious guess and it is wrong. Storm's table cipher derives word 0's
mask from the key alone:

```
seed0 = 0xEEEEEEEE + CryptTable[0x400 + (key & 0xFF)]
mask0 = key + seed0
```

So one guess at plaintext word 0 — an empty slot is `0xFFFFFFFF`, a block
entry's offset is usually `0x20` — pins `mask0`, and each of the 256 possible low
bytes of the key yields exactly one candidate key. 256 candidates to verify, not
2^32. On a healthy archive this recovers the standard `0xC3AF3770` immediately;
on these archives **no key explains the region**, and a full-file scan finds no
standard-key block table anywhere either. The tables are not relocated or
re-keyed. They are gone.

The member data is intact, though, and that is what recovery hangs on.

### Recovery 1: walk the data region

MPQ members are laid out contiguously from the start of the archive, and a
sector-based member begins with its own sector offset table: `n+1` little-endian
u32s whose first value is `4*(n+1)` and whose last value is the packed size.
That is enough to chain from one member to the next, from `0x20` to the start of
the hash table, without consulting either table.

Validated against 300 healthy maps, the walk reproduces the real block table
exactly — same offsets, same packed sizes, same order — in 113 cases and as a
correct prefix in most of the rest, 60,999 block entries in all. It breaks on
zero-length members, on single-unit members, and on the occasional archive whose
block order is not its data order, so it is a salvage tool and not a substitute
for a table.

On the 301 hard maps the walk finds a `war3map.w3i` in 281, a `war3mapMap.blp` in
266, and a `war3map.wts` in 190 — the covers and string tables in particular are
new, since carving only ever looked for `w3i` and `wts`. This is implemented as
[`Archive::salvage_members`](../src/salvage.rs), and `war3-mpq -s FILE` prints
what it finds.

Encrypted members used to end the walk; recovery 3 below is what lifted the
`w3i` count from 192 to 281 and the exact block-table reproduction from 17 maps
to 113.

Covers are salvaged by header rather than by name: a Warcraft III minimap or
preview is a 256-pixel square JPEG `BLP1` with no alpha channel, and the earliest
member of that shape is taken. Across 215 healthy maps whose real cover the walk
also reaches, every one has that header; the rule picks it in 189, picks imported
art of the same shape in 10, and declines in 16. That is good enough for maps
that would otherwise show nothing and not good enough to pass off as a read, so
the catalog records it as `cover_source: "salvage"`.

Verified end to end on one map whose tables are noise: the walk yields 15
members, 14 of which decompress, and their leading bytes line up one for one
with the names recovered from the stuffed tail below — member 0 is `W3E!`,
member 1 `BLP1`, member 7 a `w3i` declaring version 25, member 8 a BOM-prefixed
`wts`. The unpacked sizes match the surviving block entries exactly.

### Recovery 1b: scan for sectors

When the member chain itself breaks, the sector data is still recognisable: an
MPQ sector is a compression mask byte followed by the stream, so a zlib sector
reads as `02 78 9C`. [`carve`](../src/carve.rs) scans for that shape and inflates
runs of adjacent sectors, which is what a member larger than one sector looks
like. It knows nothing about what it finds — the caller judges each candidate by
content — and it cannot tell a live member from an orphaned copy left behind by a
re-save. `war3parser` has carried this logic since 0.5.3; it lives here now so
the salvage path and the scan path sit next to each other.

### Recovery 2: the stuffed tail

Some of these files have a `0x00` inserted after every eighth byte near the end.
On one 66 KB map the pattern runs from `0x101F8` to EOF; removing those bytes
makes the encrypted hash table appear intact, and it decrypts to 16 slots and 15
real filenames:

```
war3map.w3e  war3mapMap.blp  war3map.w3u  war3mapMisc.txt  war3map.w3a
war3mapSkin.txt  war3map.shd  war3map.w3i  war3map.wts  war3map.wpm
war3map.w3t  war3map.mmp  war3map.doo  war3map.w3q  Scripts\war3map.j
```

The block table survives only in part — the insertions push the tail off the end
of the file, so 7 of 15 entries remain — but those 7 match the walked members
index for index, offset for offset. Block index order is data order here, so the
walk completes what the tail lost.

Note what this rules out: the header, the member chain and the file length are
all mutually consistent with an intact archive, so this is damage inflicted after
the fact, not a protection scheme. Storm cannot read it either. Not implemented;
the walk recovers the same members without needing the names.

### Recovery 3: encrypted members

The walk used to stop dead at any `FILE_ENCRYPTED` member, and on 76 of the 301
that member was the first one, so the walk could not start at all. On most of
those the first dword of the data region is a value that repeats across unrelated
maps — the sector offset table of a member encrypted with the same
protector-chosen name in each. Encrypted members are keyed on the *basename*,
which we do not have.

The table is known plaintext, though. Its first entry is its own byte length,
`4*(n+1)`, and the cipher derives word 0's mask from the seed alone, so guessing
that one word costs 256 trials rather than 2^32 — the same argument that recovers
a table key above. Iterating plausible sector counts gives a few thousand
candidate plaintexts; word 1 must land within one sector of word 0, and the full
table must then decrypt to monotonic offsets ending at a packed size that stays
inside the data region. That is StormLib's `DetectFileSeed`, and it is now
[`crypt::detect_seed`](../src/crypt.rs), driven by `Archive::encrypted_member_at`.

Detection reads the *effective* key straight off the data, so `FILE_FIX_KEY` —
which mixes the member's offset and unpacked size into the key — needs no
separate handling.

Measured on all 300 healthy maps in the validation sample, this flags 8,623
encrypted blocks and gets every one right, with no plain member wrongly treated
as encrypted; 65 walks that could not start now do, and none that used to start
stopped.

## Where the remaining failures sit

Partitioning all 301 by what the walk achieves:

| Group | Count | State |
|---|---|---|
| Walk reaches a `war3map.w3i` | 281 | recoverable now |
| Walk runs but finds no `w3i` | 14 | member chain breaks part-way |
| No usable header at all | 6 | nothing to anchor on |

Behind `war3parser`'s `carve_deep`, which falls back to the sector scan when the
walk finds nothing, 293 of the 301 yield a `w3i`.

Two cross-cutting notes:

- 6 archives have no `MPQ\x1A` whose table positions land inside the file, under
  the stricter rule `Archive::load` applies.
- 10 archives *do* resolve `war3map.w3i` by name — their hash table is fine — but
  the block entry it points at claims data past EOF (offsets like 2637571592 and
  4294801836). Their block table is real but its offsets are not, so these need
  a different fix from the rest: the name is known, so the member can be located
  by walking instead and matched up by index.
