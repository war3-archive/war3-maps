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
use war3parser::formats::wts::trigstr_id;
use war3parser::prelude::{War3Image, War3MapW3x};

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

/// Read a map's metadata. Never panics: a malformed archive becomes a record
/// with `parse_status: "metadata_error"` and the filename as its name.
pub fn derive(bytes: &[u8], filename: &str, content_type: &str) -> Derived {
    let (header, info, parse_error, modification) = if content_type == "campaign" {
        (
            None,
            None,
            Some("campaign metadata parsing is not supported yet; indexed by filename".to_string()),
            None,
        )
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
                        (Some(header), Some(info), None, modification)
                    }
                    Err(error) => (Some(header), None, Some(error.to_string()), modification),
                }
            }
            Err(error) => (None, None, Some(error.to_string()), None),
        })) {
            Ok(parsed) => parsed,
            Err(payload) => (
                None,
                None,
                Some(format!("parser panic: {}", panic_payload_message(payload))),
                None,
            ),
        }
    };

    let fallback_name = Path::new(filename)
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or(filename);
    Derived {
        name: info
            .as_ref()
            .map(|value| strip_warcraft_codes(&value.name))
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| strip_warcraft_codes(fallback_name)),
        author: info
            .as_ref()
            .map(|value| strip_warcraft_codes(&value.author))
            .unwrap_or_default(),
        description: info
            .as_ref()
            .map(|value| strip_warcraft_codes(&value.description))
            .unwrap_or_default(),
        recommended_players: info
            .as_ref()
            .map(|value| strip_warcraft_codes(&value.recommended_players))
            .unwrap_or_default(),
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
        parse_status: if content_type == "campaign" {
            "metadata_unavailable"
        } else if parse_error.is_none() {
            "ok"
        } else {
            "metadata_error"
        },
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
        let mut archive = War3MapW3x::from_buffer(bytes).ok()?;
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
                let mut png = Cursor::new(Vec::new());
                if image
                    .data
                    .write_to(&mut png, ImageOutputFormat::Png)
                    .is_err()
                {
                    continue;
                }
                return Some((png.into_inner(), source));
            }
        }
        None
    }))
    .ok()
    .flatten()
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
