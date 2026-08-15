use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io::Cursor;
use std::io::Write;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{bail, Context};
use base64::{engine::general_purpose, Engine as _};
use image::ImageOutputFormat;
use serde::Serialize;
use sha2::{Digest, Sha256};
use war3parser::carve;
use war3parser::formats::wts::trigstr_id;
use war3parser::prelude::{War3Image, War3MapHeader, War3MapW3i, War3MapW3x};

use war3parser::modscan::{self, ModInfo};

const MAP_EXTENSIONS: &[&str] = &["w3x", "w3m", "w3n"];
const ARCHIVE_EXTENSIONS: &[&str] = &[
    "zip", "rar", "7z", "tar", "tgz", "tbz", "tbz2", "txz", "tar.gz", "tar.bz2", "tar.xz",
];
/// Cover lookup order: `war3mapPreview.*` first, then `war3mapMap.*`.
const PREVIEW_COVERS: &[&str] = &["war3mapPreview.tga", "war3mapPreview.blp"];
const MAP_COVERS: &[&str] = &["war3mapMap.blp", "war3mapMap.tga"];

pub struct BuildOptions<'a> {
    pub input_dir: &'a Path,
    pub out_dir: &'a Path,
    pub hf_repo: Option<&'a str>,
    pub inspect_archives: bool,
}

#[derive(Debug, Serialize)]
struct Catalog {
    schema_version: u32,
    generated_at_unix: u64,
    map_count: usize,
    playable_map_count: usize,
    campaign_count: usize,
    source_count: usize,
    total_bytes: u64,
    maps: Vec<MapRecord>,
}

#[derive(Debug, Clone, Serialize)]
struct MapRecord {
    sha256: String,
    name: String,
    /// Which tier of the name fallback produced `name`. The site needs this to
    /// avoid presenting a filename or a bare hash as if it were a map title.
    name_source: &'static str,
    author: String,
    description: String,
    recommended_players: String,
    max_players: Option<u32>,
    player_count: Option<u32>,
    category: String,
    filename: String,
    extension: String,
    format: String,
    content_type: &'static str,
    size: u64,
    dataset_path: String,
    download_url: Option<String>,
    cover_data: Option<String>,
    cover_source: Option<&'static str>,
    cover_status: &'static str,
    source_paths: Vec<String>,
    format_version: Option<u32>,
    /// World Editor build that last saved the map (w3i v18+). Within a single
    /// `format_version` this is the only direct evidence of which patch a map
    /// came from.
    editor_version: Option<u32>,
    /// Game version `[major, minor, patch, build]`, exact but only present from
    /// w3i v27 onward.
    build_version: Option<[u32; 4]>,
    tileset: Option<u8>,
    playable_width: Option<i32>,
    playable_height: Option<i32>,
    parse_status: &'static str,
    parse_error: Option<String>,
    /// Third-party modification found in the map script, when one is
    /// recognised. Absent means "no known signature matched", not "clean":
    /// protected maps whose script cannot be read are simply unscannable.
    #[serde(skip_serializing_if = "Option::is_none")]
    modification: Option<ModInfo>,
}

#[derive(Debug, Serialize)]
struct ScanFailure {
    source_path: String,
    error: String,
}

type CoverResult = (Option<String>, Option<&'static str>, &'static str);

pub fn build(options: BuildOptions<'_>) -> anyhow::Result<()> {
    if !options.input_dir.is_dir() {
        bail!(
            "input directory does not exist: {}",
            options.input_dir.display()
        );
    }
    fs::create_dir_all(options.out_dir)?;
    if paths_overlap(options.input_dir, options.out_dir) {
        bail!("out_dir must not be inside input_dir (or vice versa)");
    }

    fs::create_dir_all(options.out_dir.join("catalog"))?;
    fs::create_dir_all(options.out_dir.join("objects"))?;

    let mut files = Vec::new();
    collect_files(options.input_dir, &mut files)?;
    files.sort();

    let mut records = BTreeMap::<String, MapRecord>::new();
    let mut failures = Vec::<ScanFailure>::new();
    let mut source_count = 0usize;

    for file in files {
        let relative = relative_display(options.input_dir, &file);
        if is_temporary_download(&file) {
            continue;
        }
        if is_map_path(&file) {
            source_count += 1;
            match fs::read(&file).with_context(|| format!("read {}", file.display())) {
                Ok(bytes) => ingest(
                    &bytes,
                    &relative,
                    file.file_name()
                        .and_then(OsStr::to_str)
                        .unwrap_or("map.w3x"),
                    category_for(options.input_dir, &file, None),
                    &options,
                    &mut records,
                )
                .unwrap_or_else(|error| failures.push(failure(&relative, error))),
                Err(error) => failures.push(failure(&relative, error)),
            }
        } else if options.inspect_archives && is_archive_path(&file) {
            match archive_map_entries(&file) {
                Ok(entries) => {
                    for entry in entries {
                        source_count += 1;
                        let source = format!("{relative}!/{entry}");
                        match read_archive_entry(&file, &entry) {
                            Ok(bytes) => ingest(
                                &bytes,
                                &source,
                                Path::new(&entry)
                                    .file_name()
                                    .and_then(OsStr::to_str)
                                    .unwrap_or("map.w3x"),
                                category_for(options.input_dir, &file, Some(&entry)),
                                &options,
                                &mut records,
                            )
                            .unwrap_or_else(|error| failures.push(failure(&source, error))),
                            Err(error) => failures.push(failure(&source, error)),
                        }
                    }
                }
                Err(error) => failures.push(failure(&relative, error)),
            }
        }
    }

    let maps: Vec<_> = records.into_values().collect();
    let total_bytes = maps.iter().map(|map| map.size).sum();
    let campaign_count = maps
        .iter()
        .filter(|map| map.content_type == "campaign")
        .count();
    let catalog = Catalog {
        schema_version: 2,
        generated_at_unix: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        map_count: maps.len(),
        playable_map_count: maps.len() - campaign_count,
        campaign_count,
        source_count,
        total_bytes,
        maps,
    };

    write_json_atomic(&options.out_dir.join("catalog/maps.json"), &catalog)?;
    write_jsonl_atomic(&options.out_dir.join("catalog/maps.jsonl"), &catalog.maps)?;
    write_json_atomic(&options.out_dir.join("catalog/failures.json"), &failures)?;

    println!(
        "✅ Indexed {} unique maps from {} sources ({} failures) into '{}'",
        catalog.map_count,
        catalog.source_count,
        failures.len(),
        options.out_dir.display()
    );
    Ok(())
}

fn ingest(
    bytes: &[u8],
    source: &str,
    filename: &str,
    category: String,
    options: &BuildOptions<'_>,
    records: &mut BTreeMap<String, MapRecord>,
) -> anyhow::Result<()> {
    let sha256 = format!("{:x}", Sha256::digest(bytes));
    if let Some(existing) = records.get_mut(&sha256) {
        if !existing.source_paths.iter().any(|path| path == source) {
            existing.source_paths.push(source.to_string());
            existing.source_paths.sort();
        }
        return Ok(());
    }

    let extension = normalized_extension(Path::new(filename)).unwrap_or("w3x");
    let dataset_path = format!("objects/{}/{}.{}", &sha256[..2], sha256, extension);
    let object_path = options.out_dir.join(&dataset_path);
    if !object_path.exists() {
        if let Some(parent) = object_path.parent() {
            fs::create_dir_all(parent)?;
        }
        write_bytes_atomic(&object_path, bytes)?;
    }

    let content_type = if extension == "w3n" {
        "campaign"
    } else {
        "map"
    };
    let (cover_data, cover_source, cover_status) = if content_type == "campaign" {
        (None, None, "missing")
    } else {
        extract_cover(bytes)
    };
    let derived = derive(bytes, filename, content_type);
    let record = MapRecord {
        sha256: sha256.clone(),
        name: derived.name,
        name_source: derived.name_source,
        author: derived.author,
        description: derived.description,
        recommended_players: derived.recommended_players,
        max_players: derived.max_players,
        player_count: derived.player_count,
        category,
        filename: filename.to_string(),
        extension: extension.to_string(),
        format: extension.to_string(),
        content_type,
        size: bytes.len() as u64,
        download_url: hf_resolve_url(options.hf_repo, &dataset_path),
        dataset_path,
        cover_data,
        cover_source,
        cover_status,
        source_paths: vec![source.to_string()],
        format_version: derived.format_version,
        editor_version: derived.editor_version,
        build_version: derived.build_version,
        tileset: derived.tileset,
        playable_width: derived.playable_width,
        playable_height: derived.playable_height,
        parse_status: derived.parse_status,
        parse_error: derived.parse_error,
        modification: derived.modification,
    };
    records.insert(sha256, record);
    Ok(())
}

/// Everything a catalog record derives from the map bytes alone.
///
/// Split out from the record so a rescan can refresh these fields on an
/// existing dataset without touching provenance (collection, source paths).
#[derive(Debug, Serialize)]
pub struct Derived {
    pub name: String,
    /// `"w3i"`, `"hm3w"`, or `"filename"` — see [`derive`].
    pub name_source: &'static str,
    pub author: String,
    pub description: String,
    pub recommended_players: String,
    pub max_players: Option<u32>,
    pub player_count: Option<u32>,
    pub format_version: Option<u32>,
    pub editor_version: Option<u32>,
    pub build_version: Option<[u32; 4]>,
    pub tileset: Option<u8>,
    pub playable_width: Option<i32>,
    pub playable_height: Option<i32>,
    pub parse_status: &'static str,
    pub parse_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modification: Option<ModInfo>,
}

/// Raw result of the parsing pass, before it is flattened into a [`Derived`]
/// record.
struct Parsed {
    header: Option<War3MapHeader>,
    info: Option<War3MapW3i>,
    /// `info` was carved out of sector data rather than read by name, so it is
    /// a salvage guess rather than an authoritative read.
    carved: bool,
    parse_error: Option<String>,
    modification: Option<ModInfo>,
}

impl Parsed {
    /// Nothing was readable; `message` explains why.
    fn failed(message: &str) -> Self {
        Self {
            header: None,
            info: None,
            carved: false,
            parse_error: Some(message.to_string()),
            modification: None,
        }
    }
}

/// Read a map's metadata. Never panics: a malformed archive becomes a record
/// with `parse_status: "metadata_error"` (or `"carved"` if a `w3i` could still
/// be recovered from sector data) rather than aborting the rest of the catalog.
pub fn derive(bytes: &[u8], filename: &str, content_type: &str) -> Derived {
    let Parsed {
        header,
        info,
        carved,
        parse_error,
        modification,
    } = if content_type == "campaign" {
        Parsed::failed("campaign metadata parsing is not supported yet; indexed by filename")
    } else {
        match catch_unwind(AssertUnwindSafe(|| match War3MapW3x::from_buffer(bytes) {
            Ok(mut archive) => {
                let header = archive.header.clone();
                // Script scan first: a map whose w3i fails to parse can still
                // carry a readable script, and the modification is worth
                // recording either way.
                let modification = modscan::detect(&mut archive);
                match archive.read_map_info() {
                    Ok(mut info) => {
                        if let Ok(wts) = archive.read_string_table() {
                            info.visit_strings(|text| {
                                if let Some(id) = trigstr_id(text) {
                                    if let Some(value) = wts.get(id) {
                                        *text = value.to_string();
                                    }
                                }
                            });
                        }
                        Parsed {
                            header: Some(header),
                            info: Some(info),
                            carved: false,
                            parse_error: None,
                            modification,
                        }
                    }
                    // The archive opened, but the tables do not resolve
                    // `war3map.w3i`. Protected maps often leave the sector
                    // payload intact, so carve it out of the raw file.
                    Err(error) => Parsed {
                        header: Some(header),
                        info: salvage_w3i(bytes),
                        carved: true,
                        parse_error: Some(error.to_string()),
                        modification,
                    },
                }
            }
            // The archive is unreadable, so nothing can be found by name. Two
            // things survive that anyway: the `HM3W` prefix sits in plaintext
            // ahead of the MPQ, and the sector data is usually intact even when
            // the tables are noise, which lets the `w3i` be carved out whole.
            Err(error) => Parsed {
                header: War3MapHeader::from_buffer(bytes).ok(),
                info: salvage_w3i(bytes),
                carved: true,
                parse_error: Some(error.to_string()),
                modification: None,
            },
        })) {
            Ok(parsed) => parsed,
            Err(payload) => {
                Parsed::failed(&format!("parser panic: {}", panic_payload_message(payload)))
            }
        }
    };

    let fallback_name = Path::new(filename)
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or(filename);
    let info_name = info.as_ref().map(|value| value.name.as_str());
    let (name, name_source) = pick_name(
        info_name.filter(|_| !carved),
        header.as_ref().and_then(|value| value.name.as_deref()),
        info_name.filter(|_| carved),
        fallback_name,
    );
    Derived {
        name,
        name_source,
        author: display_text(info.as_ref().map(|value| value.author.as_str())),
        description: display_text(info.as_ref().map(|value| value.description.as_str())),
        recommended_players: display_text(
            info.as_ref()
                .map(|value| value.recommended_players.as_str()),
        ),
        max_players: header
            .as_ref()
            .and_then(|value| value.max_players)
            .or_else(|| {
                info.as_ref()
                    .map(|value| value.players.len().try_into().unwrap_or(u32::MAX))
            }),
        player_count: info.as_ref().map(|value| value.players.len() as u32),
        format_version: info.as_ref().map(|value| value.version.0),
        editor_version: info.as_ref().and_then(|value| value.editor_version),
        build_version: info.as_ref().and_then(|value| value.build_version),
        tileset: info.as_ref().map(|value| value.tileset),
        playable_width: info.as_ref().map(|value| value.playable_size[0]),
        playable_height: info.as_ref().map(|value| value.playable_size[1]),
        parse_status: metadata_status(content_type, parse_error.is_none(), info.is_some()),
        parse_error,
        modification,
    }
}

/// Extract a cover as PNG bytes for the dataset's `covers/` tree, together with
/// which embedded image it came from.
///
/// `extract_cover` produces the inline thumbnail `build-catalog` writes into the
/// catalog; this one keeps the original resolution and leaves downscaling to
/// `deploy/export_covers.py`, which owns the published WebP.
pub fn cover_png(bytes: &[u8]) -> Option<(Vec<u8>, &'static str)> {
    catch_unwind(AssertUnwindSafe(|| {
        if let Ok(mut archive) = War3MapW3x::from_buffer(bytes) {
            for (candidates, source) in [(PREVIEW_COVERS, "preview"), (MAP_COVERS, "map")] {
                for candidate in candidates {
                    if !archive.has(candidate) {
                        continue;
                    }
                    let Ok(data) = archive.read_file(candidate) else {
                        continue;
                    };
                    if let Some(png) = encode_cover(&data, candidate) {
                        return Some((png, source));
                    }
                }
            }
        }

        // The archive is unreadable by name, or holds no cover under one. The
        // member data usually survives both, so fall back to the salvage walk —
        // it cannot say *which* image it found, hence its own source tier.
        let data = carve::carve_cover(bytes)?;
        encode_cover(&data, "war3mapMap.blp").map(|png| (png, "salvage"))
    }))
    .ok()
    .flatten()
}

/// Decode a cover member and re-encode it as PNG.
fn encode_cover(data: &[u8], filename: &str) -> Option<Vec<u8>> {
    let image = War3Image::from_buffer(data, filename).ok()?;
    let mut png = Cursor::new(Vec::new());
    image
        .data
        .write_to(&mut png, ImageOutputFormat::Png)
        .ok()
        .map(|()| png.into_inner())
}

fn hf_resolve_url(repo: Option<&str>, dataset_path: &str) -> Option<String> {
    repo.map(|repo| {
        format!("https://huggingface.co/datasets/{repo}/resolve/main/{dataset_path}?download=true")
    })
}

fn extract_cover(bytes: &[u8]) -> CoverResult {
    match catch_unwind(AssertUnwindSafe(|| -> anyhow::Result<CoverResult> {
        match War3MapW3x::from_buffer(bytes) {
            Ok(mut archive) => {
                for (candidates, source) in [(PREVIEW_COVERS, "preview"), (MAP_COVERS, "map")] {
                    for candidate in candidates {
                        if !archive.has(candidate) {
                            continue;
                        }
                        let Ok(data) = archive.read_file(candidate) else {
                            continue;
                        };
                        let Ok(image) = War3Image::from_buffer(&data, candidate) else {
                            continue;
                        };
                        let thumbnail = image.data.thumbnail(128, 128);
                        let mut jpeg = Cursor::new(Vec::new());
                        thumbnail.write_to(&mut jpeg, ImageOutputFormat::Jpeg(65))?;
                        let data_url = format!(
                            "data:image/jpeg;base64,{}",
                            general_purpose::STANDARD.encode(jpeg.into_inner())
                        );
                        return Ok((Some(data_url), Some(source), "ok"));
                    }
                }
                Ok((None, None, "missing"))
            }
            Err(_) => Ok((None, None, "missing")),
        }
    })) {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => (None, None, "error"),
        Err(_) => (None, None, "error"),
    }
}

fn panic_payload_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else {
        "unknown panic payload".to_string()
    }
}

fn collect_files(directory: &Path, output: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("read directory {}", directory.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            collect_files(&entry.path(), output)?;
        } else if file_type.is_file() {
            output.push(entry.path());
        }
    }
    Ok(())
}

fn archive_map_entries(archive: &Path) -> anyhow::Result<Vec<String>> {
    let output = Command::new("bsdtar")
        .args(["-tf"])
        .arg(archive)
        .output()
        .with_context(|| "run bsdtar (install libarchive/bsdtar to scan compressed maps)")?;
    if !output.status.success() {
        bail!(
            "bsdtar list failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(map_entries_from_listing(&output.stdout))
}

fn map_entries_from_listing(listing: &[u8]) -> Vec<String> {
    let mut entries: Vec<_> = String::from_utf8_lossy(listing)
        .lines()
        .filter(|entry| is_map_path(Path::new(entry)))
        .map(str::to_string)
        .collect();
    entries.sort();
    entries.dedup();
    entries
}

fn read_archive_entry(archive: &Path, entry: &str) -> anyhow::Result<Vec<u8>> {
    let child = Command::new("bsdtar")
        .args(["-xOf"])
        .arg(archive)
        .arg(entry)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("extract {entry} from {}", archive.display()))?;
    let output = child.wait_with_output()?;
    if output.status.success() {
        return Ok(output.stdout);
    }
    if archive
        .extension()
        .and_then(OsStr::to_str)
        .map(|value| value.eq_ignore_ascii_case("zip"))
        .unwrap_or(false)
    {
        if let Some(filename) = Path::new(entry).file_name().and_then(OsStr::to_str) {
            if let Ok(bytes) = read_legacy_zip_entry_by_filename(archive, filename) {
                return Ok(bytes);
            }
        }
    }
    // Some legacy ZIPs contain GBK path bytes without the UTF-8 flag. libarchive can
    // list and extract them, but the lossy display name cannot be passed back to -xOf.
    // Extracting into an isolated temporary directory preserves those raw path bytes.
    read_archive_entry_from_temporary_directory(archive, entry).with_context(|| {
        format!(
            "bsdtar direct extract failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )
    })
}

fn read_legacy_zip_entry_by_filename(archive: &Path, filename: &str) -> anyhow::Result<Vec<u8>> {
    let output = Command::new("unzip")
        .arg("-p")
        .arg(archive)
        .arg(format!("*/{filename}"))
        .output()
        .with_context(|| "run unzip for legacy ZIP path encoding fallback")?;
    if !output.status.success() || output.stdout.is_empty() {
        bail!(
            "unzip fallback failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

fn read_archive_entry_from_temporary_directory(
    archive: &Path,
    entry: &str,
) -> anyhow::Result<Vec<u8>> {
    let temporary = TemporaryDirectory::new("war3parser-archive")?;
    let output = Command::new("bsdtar")
        .args(["-xf"])
        .arg(archive)
        .arg("-C")
        .arg(&temporary.path)
        .output()
        .with_context(|| format!("extract archive {}", archive.display()))?;
    if !output.status.success() {
        bail!(
            "bsdtar fallback extract failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let expected_name = Path::new(entry).file_name().and_then(OsStr::to_str);
    let mut files = Vec::new();
    collect_files(&temporary.path, &mut files)?;
    let mut candidates = files.into_iter().filter(|path| {
        is_map_path(path)
            && expected_name
                .map(|name| path.file_name().map(|value| value == name).unwrap_or(false))
                .unwrap_or(false)
    });
    let candidate = candidates
        .next()
        .with_context(|| format!("fallback could not locate archive member {entry}"))?;
    if candidates.next().is_some() {
        bail!("fallback archive member name is ambiguous: {entry}");
    }
    fs::read(&candidate).with_context(|| format!("read extracted member {}", candidate.display()))
}

struct TemporaryDirectory {
    path: PathBuf,
}

impl TemporaryDirectory {
    fn new(prefix: &str) -> anyhow::Result<Self> {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id()));
        fs::create_dir(&path)
            .with_context(|| format!("create temporary directory {}", path.display()))?;
        Ok(Self { path })
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn is_map_path(path: &Path) -> bool {
    normalized_extension(path)
        .map(|extension| MAP_EXTENSIONS.contains(&extension))
        .unwrap_or(false)
}

fn is_temporary_download(path: &Path) -> bool {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(|name| name.to_ascii_lowercase().ends_with(".qkdownloading"))
        .unwrap_or(false)
}

fn is_archive_path(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    ARCHIVE_EXTENSIONS
        .iter()
        .any(|extension| name.ends_with(&format!(".{extension}")))
}

fn normalized_extension(path: &Path) -> Option<&str> {
    path.extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .and_then(|extension| {
            MAP_EXTENSIONS
                .iter()
                .copied()
                .find(|item| *item == extension)
        })
}

fn category_for(input: &Path, path: &Path, archive_entry: Option<&str>) -> String {
    let relative = path.strip_prefix(input).unwrap_or(path);
    let components: Vec<_> = relative.components().collect();
    if components.len() > 1 {
        return components[0].as_os_str().to_string_lossy().into_owned();
    }
    if archive_entry.is_some() {
        let name = path.file_stem().and_then(OsStr::to_str).unwrap_or("未分类");
        return name.trim_end_matches(".tar").to_string();
    }
    "未分类".to_string()
}

fn relative_display(input: &Path, path: &Path) -> String {
    path.strip_prefix(input)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn paths_overlap(input: &Path, output: &Path) -> bool {
    let input = input.canonicalize().unwrap_or_else(|_| input.to_path_buf());
    let output = output
        .canonicalize()
        .unwrap_or_else(|_| output.to_path_buf());
    input.starts_with(&output) || output.starts_with(&input)
}

/// Recover a `w3i` from raw sector data, for archives that will not open by
/// name. Carving cannot tell a live sector from an orphaned one, so the result
/// is a salvage guess — [`pick_name`] ranks it below the plaintext `HM3W` title.
/// Salvage metadata from an archive no name-based read could open.
///
/// This asks for the exhaustive variant deliberately. `carve` alone walks the
/// member chain and is what a library caller should get; here the whole point
/// of the pass is to recover what nothing else can, and measured over the 593
/// unresolved objects in the corpus the byte scan adds under two seconds while
/// recovering 29 maps the walk cannot reach.
fn salvage_w3i(bytes: &[u8]) -> Option<War3MapW3i> {
    carve::carve_deep(bytes).map(|mut carved| {
        carved.resolve_trigger_strings();
        carved.info
    })
}

/// Strip color codes and drop unresolved `TRIGSTR_*` placeholders.
fn display_text(value: Option<&str>) -> String {
    value
        .map(strip_warcraft_codes)
        .filter(|text| !text.trim().is_empty() && trigstr_id(text).is_none())
        .unwrap_or_default()
}

fn metadata_status(content_type: &str, archive_ok: bool, has_info: bool) -> &'static str {
    if content_type == "campaign" {
        "metadata_unavailable"
    } else if archive_ok {
        "ok"
    } else if has_info {
        "carved"
    } else {
        "metadata_error"
    }
}

/// Resolve a map's display name, best source first, and report which tier won.
///
/// A `w3i` read out of the archive by name is authoritative. The `HM3W` prefix
/// is plaintext and survives an unreadable archive, which is what keeps
/// protected maps from falling through to a filename — and, on the
/// content-addressed dataset, to a bare sha256 that reads as a title but is not
/// one.
///
/// A *carved* `w3i` ranks below `HM3W` on purpose. Carving reads sector data
/// with no table to say which sectors are live, so a map that was re-saved can
/// hand back the title of an older build still lying in the file: 26 of the
/// dataset's 224 carved maps disagree with their header that way, and the
/// header is the title the map was published under.
fn pick_name(
    info_name: Option<&str>,
    header_name: Option<&str>,
    carved_name: Option<&str>,
    fallback: &str,
) -> (String, &'static str) {
    for (candidate, source) in [
        (info_name, "w3i"),
        (header_name, "hm3w"),
        (carved_name, "w3i_carved"),
    ] {
        if let Some(value) = candidate.map(strip_warcraft_codes) {
            if !value.trim().is_empty() && trigstr_id(&value).is_none() {
                return (value, source);
            }
        }
    }
    (strip_warcraft_codes(fallback), "filename")
}

fn strip_warcraft_codes(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut cleaned = String::with_capacity(input.len());
    let mut index = 0;
    while index < chars.len() {
        if chars[index] == '|' && index + 1 < chars.len() {
            match chars[index + 1].to_ascii_lowercase() {
                'c' if index + 10 <= chars.len()
                    && chars[index + 2..index + 10]
                        .iter()
                        .all(|value| value.is_ascii_hexdigit()) =>
                {
                    index += 10;
                    continue;
                }
                'r' | 'n' => {
                    if chars[index + 1].eq_ignore_ascii_case(&'n') {
                        cleaned.push(' ');
                    }
                    index += 2;
                    continue;
                }
                '|' => {
                    cleaned.push('|');
                    index += 2;
                    continue;
                }
                _ => {}
            }
        }
        cleaned.push(chars[index]);
        index += 1;
    }
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn failure(source: &str, error: impl std::fmt::Display) -> ScanFailure {
    ScanFailure {
        source_path: source.to_string(),
        error: error.to_string(),
    }
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> anyhow::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    write_bytes_atomic(path, &bytes)
}

fn write_jsonl_atomic(path: &Path, values: &[MapRecord]) -> anyhow::Result<()> {
    let temporary = path.with_extension("jsonl.tmp");
    let mut file = fs::File::create(&temporary)?;
    for value in values {
        serde_json::to_writer(&mut file, value)?;
        file.write_all(b"\n")?;
    }
    file.sync_all()?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(OsStr::to_str).unwrap_or("file")
    ));
    let mut file = fs::File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_warcraft_color_and_line_codes() {
        assert_eq!(
            strip_warcraft_codes("|cffff0000Red|r|nLine || Pipe"),
            "Red Line | Pipe"
        );
    }

    #[test]
    fn prefers_w3i_name_then_hm3w_then_filename() {
        assert_eq!(
            pick_name(Some("W3i 名"), Some("HM3W 名"), None, "文件名"),
            ("W3i 名".to_string(), "w3i")
        );
        // The tier that matters: no w3i, because the archive would not open.
        assert_eq!(
            pick_name(None, Some("|cffff0000攻守兼备TD|r"), None, "文件名"),
            ("攻守兼备TD".to_string(), "hm3w")
        );
        assert_eq!(
            pick_name(None, None, None, "文件名"),
            ("文件名".to_string(), "filename")
        );
        // A blank name is not a name; fall through rather than title a map "".
        assert_eq!(
            pick_name(Some("   "), Some("守卫剑阁"), None, "文件名"),
            ("守卫剑阁".to_string(), "hm3w")
        );
        // An unresolved string-table ref is not a title.
        assert_eq!(
            pick_name(
                Some("TRIGSTR_001"),
                Some("宝可梦大冒险0.8a"),
                None,
                "文件名"
            ),
            ("宝可梦大冒险0.8a".to_string(), "hm3w")
        );
    }

    /// A carved `w3i` can be an orphaned older copy, so the plaintext header
    /// wins when the two disagree — but it still beats a sha256 filename.
    #[test]
    fn ranks_a_carved_w3i_below_the_header() {
        assert_eq!(
            pick_name(
                None,
                Some("攻守兼备TD V4.6正式版"),
                Some("攻守兼备TD V4.4正式版"),
                "文件名"
            ),
            ("攻守兼备TD V4.6正式版".to_string(), "hm3w")
        );
        assert_eq!(
            pick_name(None, None, Some("守卫剑阁-纵横天下V1.22"), "文件名"),
            ("守卫剑阁-纵横天下V1.22".to_string(), "w3i_carved")
        );
    }

    /// A map whose MPQ archive cannot be opened at all still gets its real
    /// title, instead of falling through to the object's sha256 filename.
    #[test]
    fn derives_hm3w_name_when_the_archive_is_unreadable() {
        let mut bytes = Vec::from(*war3parser::prelude::HM3W_MAGIC);
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice("守卫剑阁-降龙伏虎".as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&8u32.to_le_bytes());
        bytes.resize(512, 0);
        bytes.extend_from_slice(&[0xAB; 1024]);

        let derived = derive(&bytes, "0d1cc85dd54d538173977c37590473e7.w3x", "map");
        assert_eq!(derived.name, "守卫剑阁-降龙伏虎");
        assert_eq!(derived.name_source, "hm3w");
        assert_eq!(derived.max_players, Some(8));
        assert_eq!(derived.parse_status, "metadata_error");
        assert!(derived.parse_error.is_some());
    }

    /// An archive whose tables are noise but whose members can still be walked,
    /// which is what a protected map looks like and what `carve` recovers from.
    fn unreadable_archive_with_carvable_w3i(header_name: &str, map_name: &str) -> Vec<u8> {
        use flate2::write::ZlibEncoder;
        use flate2::Compression;
        use std::io::Write;

        let mut bytes = Vec::from(*war3parser::prelude::HM3W_MAGIC);
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(header_name.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&8u32.to_le_bytes());
        bytes.resize(512, 0);

        let mut w3i = Vec::new();
        w3i.extend(25u32.to_le_bytes());
        w3i.extend(0u32.to_le_bytes());
        w3i.extend(0u32.to_le_bytes());
        for field in [map_name, "某作者", "", ""] {
            w3i.extend(field.as_bytes());
            w3i.push(0);
        }
        w3i.resize(w3i.len() + 256, 0);

        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&w3i).unwrap();
        let mut body = vec![0x02u8];
        body.extend(encoder.finish().unwrap());

        // One member: its own sector offset table, then the single sector.
        let mut member = Vec::new();
        member.extend_from_slice(&8u32.to_le_bytes());
        member.extend_from_slice(&(8 + body.len() as u32).to_le_bytes());
        member.extend_from_slice(&body);

        let hash_pos = 0x20 + member.len() as u32;
        let mut mpq = Vec::new();
        mpq.extend_from_slice(b"MPQ\x1a");
        mpq.extend_from_slice(&0x20u32.to_le_bytes());
        mpq.extend_from_slice(&(hash_pos + 32).to_le_bytes());
        mpq.extend_from_slice(&0u16.to_le_bytes());
        mpq.extend_from_slice(&3u16.to_le_bytes());
        mpq.extend_from_slice(&hash_pos.to_le_bytes());
        mpq.extend_from_slice(&(hash_pos + 16).to_le_bytes());
        mpq.extend_from_slice(&1u32.to_le_bytes());
        mpq.extend_from_slice(&1u32.to_le_bytes());
        mpq.extend_from_slice(&member);
        // Where the tables were, a protector leaves noise behind.
        mpq.extend((0..32u8).map(|i| i.wrapping_mul(31).wrapping_add(7)));

        bytes.extend_from_slice(&mpq);
        bytes
    }

    #[test]
    fn derives_carved_w3i_when_the_archive_is_unreadable() {
        // No header title, so the carved one is all there is.
        let bytes = unreadable_archive_with_carvable_w3i("", "守卫剑阁");
        let derived = derive(&bytes, "deadbeef.w3x", "map");
        assert_eq!(derived.name, "守卫剑阁");
        assert_eq!(derived.name_source, "w3i_carved");
        assert_eq!(derived.author, "某作者");
        assert_eq!(derived.parse_status, "carved");
        assert!(derived.parse_error.is_some());
    }

    /// The carved title may be an orphaned older copy, so a header title wins —
    /// the rest of the carved metadata is still kept.
    #[test]
    fn a_header_title_outranks_the_carved_one() {
        let bytes =
            unreadable_archive_with_carvable_w3i("攻守兼备TD V4.6正式版", "攻守兼备TD V4.4正式版");
        let derived = derive(&bytes, "deadbeef.w3x", "map");
        assert_eq!(derived.name, "攻守兼备TD V4.6正式版");
        assert_eq!(derived.name_source, "hm3w");
        assert_eq!(derived.author, "某作者");
        assert_eq!(derived.parse_status, "carved");
    }

    #[test]
    fn recognizes_supported_files_case_insensitively() {
        assert!(is_map_path(Path::new("Demo.W3X")));
        assert!(is_map_path(Path::new("Campaign.W3N")));
        assert!(is_archive_path(Path::new("maps.TAR.GZ")));
        assert!(!is_map_path(Path::new("notes.txt")));
        assert!(is_temporary_download(Path::new(
            "Campaign.w3n.qkdownloading"
        )));
        assert!(!is_map_path(Path::new("Campaign.w3n.qkdownloading")));
    }

    #[test]
    fn selects_campaigns_from_archive_listing_and_skips_partial_members() {
        let entries = map_entries_from_listing(
            "说明.txt\n战役/完整.W3N\n战役/下载中.w3n.qkdownloading\n地图/demo.w3x\n".as_bytes(),
        );
        assert_eq!(entries, vec!["地图/demo.w3x", "战役/完整.W3N"]);
    }

    #[test]
    fn indexes_campaign_with_filename_fallback_and_ignores_quark_temp() {
        let root = std::env::temp_dir().join(format!(
            "war3parser-campaign-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let input = root.join("input/战役包");
        let output = root.join("output");
        fs::create_dir_all(&input).expect("create test input");
        fs::write(input.join("|cffff0000赤焰战役|r.w3n"), b"campaign").expect("write campaign");
        fs::write(
            input.join("未完成.w3n.qkdownloading"),
            b"temporary campaign",
        )
        .expect("write temporary download");

        build(BuildOptions {
            input_dir: &root.join("input"),
            out_dir: &output,
            hf_repo: Some("example/campaigns"),
            inspect_archives: true,
        })
        .expect("build catalog");

        let catalog: serde_json::Value = serde_json::from_slice(
            &fs::read(output.join("catalog/maps.json")).expect("read catalog"),
        )
        .expect("parse catalog");
        assert_eq!(catalog["schema_version"], 2);
        assert_eq!(catalog["map_count"], 1);
        assert_eq!(catalog["campaign_count"], 1);
        assert_eq!(catalog["source_count"], 1);
        let campaign = &catalog["maps"][0];
        assert_eq!(campaign["name"], "赤焰战役");
        assert_eq!(campaign["extension"], "w3n");
        assert_eq!(campaign["format"], "w3n");
        assert_eq!(campaign["content_type"], "campaign");
        assert_eq!(campaign["parse_status"], "metadata_unavailable");
        assert!(campaign["dataset_path"]
            .as_str()
            .expect("dataset path")
            .ends_with(".w3n"));
        assert!(campaign["download_url"]
            .as_str()
            .expect("download URL")
            .contains("example/campaigns"));

        fs::remove_dir_all(root).expect("clean test directory");
    }
}
