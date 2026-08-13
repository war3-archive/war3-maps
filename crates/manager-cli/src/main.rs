use std::ffi::OsStr;
use std::path::Path;
use std::sync::Mutex;

use clap::Parser;
use war3parser::modscan::{self, ModInfo};
use war3parser::prelude::{War3MapW3i, War3MapW3x};

mod catalog;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
enum Command {
    /// Scan maps and archives, deduplicate them, and build a static-site catalog
    BuildCatalog {
        /// Directory containing downloaded maps and archives
        input_dir: String,

        /// Generated content-addressed dataset and catalog directory
        #[arg(short, long)]
        out_dir: String,

        /// Hugging Face dataset id (for example `owner/war3-maps`)
        #[arg(long)]
        hf_repo: Option<String>,

        /// Do not inspect supported archives (.zip/.rar/.7z/.tar/...)
        #[arg(long, default_value_t = false)]
        no_archives: bool,
    },

    /// Re-read only the w3i version fields of an existing dataset, as JSONL.
    ///
    /// Rebuilding the whole catalog would discard collection assignments and
    /// source provenance, so this emits just enough to patch those fields in.
    ScanVersions {
        /// Dataset root containing `objects/`
        dataset_root: String,

        /// Write JSONL here instead of stdout
        #[arg(short, long)]
        out: Option<String>,

        /// Worker threads
        #[arg(short, long)]
        jobs: Option<usize>,
    },

    /// Re-scan an existing dataset for known third-party script modifications,
    /// as JSONL.
    ///
    /// Signatures change more often than the catalog does, so this runs
    /// independently of `build-catalog`; feed the output to
    /// `deploy/apply_mods.py` to patch the records in place.
    ScanMods {
        /// Dataset root containing `objects/`
        dataset_root: String,

        /// Write JSONL here instead of stdout
        #[arg(short, long)]
        out: Option<String>,

        /// Worker threads
        #[arg(short, long)]
        jobs: Option<usize>,

        /// Emit every object, including those with no modification found
        #[arg(long, default_value_t = false)]
        all: bool,
    },

    /// Re-read every object's metadata (name, author, players, w3i fields,
    /// modifications) and recover missing covers, as JSONL.
    ///
    /// Parser fixes keep making previously unreadable maps readable; this
    /// refreshes an existing dataset in place instead of rebuilding it, which
    /// would lose collection assignments and source provenance. Recovered
    /// covers are written as `covers/<xx>/<sha256>.png` for
    /// `deploy/export_covers.py` to encode. Feed the JSONL to
    /// `deploy/apply_rescan.py`.
    Rescan {
        /// Dataset root containing `objects/`
        dataset_root: String,

        /// Write JSONL here instead of stdout
        #[arg(short, long)]
        out: Option<String>,

        /// Worker threads
        #[arg(short, long)]
        jobs: Option<usize>,

        /// Only re-read objects whose metadata previously failed to parse
        #[arg(long, default_value_t = false)]
        only_failed: bool,

        /// Re-extract covers even when one already exists
        #[arg(long, default_value_t = false)]
        force_covers: bool,
    },
}

#[derive(serde::Serialize)]
struct VersionRecord {
    sha256: String,
    format_version: Option<u32>,
    editor_version: Option<u32>,
    build_version: Option<[u32; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn scan_one(path: &Path) -> VersionRecord {
    let sha256 = path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_string();
    let mut record = VersionRecord {
        sha256,
        format_version: None,
        editor_version: None,
        build_version: None,
        error: None,
    };
    // A corrupt or protected archive is data, not a failure of the scan.
    let parsed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> anyhow::Result<_> {
        let mut w3x = War3MapW3x::open(path)?;
        let raw = w3x.read_file("war3map.w3i")?;
        Ok(War3MapW3i::parse(&raw)?)
    }));
    match parsed {
        Ok(Ok(info)) => {
            record.format_version = Some(info.version.0);
            record.editor_version = info.editor_version;
            record.build_version = info.build_version;
        }
        Ok(Err(error)) => record.error = Some(error.to_string()),
        Err(_) => record.error = Some("parser panic".to_string()),
    }
    record
}

#[derive(serde::Serialize)]
struct ModRecord {
    sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    modification: Option<ModInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn scan_mods_one(path: &Path) -> ModRecord {
    let sha256 = path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_string();
    // Same contract as scan_one: a map that cannot be opened is recorded, not
    // fatal. "No modification" and "unreadable" must stay distinguishable.
    let scanned =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> anyhow::Result<_> {
            let mut w3x = War3MapW3x::open(path)?;
            Ok(modscan::detect(&mut w3x))
        }));
    match scanned {
        Ok(Ok(modification)) => ModRecord {
            sha256,
            modification,
            error: None,
        },
        Ok(Err(error)) => ModRecord {
            sha256,
            modification: None,
            error: Some(error.to_string()),
        },
        Err(_) => ModRecord {
            sha256,
            modification: None,
            error: Some("parser panic".to_string()),
        },
    }
}

#[derive(serde::Serialize)]
struct RescanRecord {
    sha256: String,
    #[serde(flatten)]
    derived: catalog::Derived,
    /// Which embedded image the recovered cover came from, if one was written.
    #[serde(skip_serializing_if = "Option::is_none")]
    cover_source: Option<&'static str>,
    /// Whether a cover file exists for this object after the rescan.
    cover_status: &'static str,
}

fn rescan_one(path: &Path, root: &Path, force_covers: bool) -> Option<RescanRecord> {
    let sha256 = path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_string();
    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or("w3x")
        .to_ascii_lowercase();
    let filename = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
    let content_type = if extension == "w3n" {
        "campaign"
    } else {
        "map"
    };
    let bytes = std::fs::read(path).ok()?;

    let derived = catalog::derive(&bytes, filename, content_type);

    // Covers live next to the objects; only the missing ones are re-extracted.
    let shard = &sha256[..2.min(sha256.len())];
    let webp = root
        .join("covers")
        .join(shard)
        .join(format!("{sha256}.webp"));
    let png = root
        .join("covers")
        .join(shard)
        .join(format!("{sha256}.png"));
    let mut cover_source = None;
    let mut cover_status = if webp.is_file() || png.is_file() {
        "ok"
    } else {
        "missing"
    };
    if content_type == "map" && (force_covers || cover_status == "missing") {
        if let Some((data, source)) = catalog::cover_png(&bytes) {
            if let Some(parent) = png.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            if std::fs::write(&png, data).is_ok() {
                cover_source = Some(source);
                cover_status = "ok";
            }
        }
    }

    Some(RescanRecord {
        sha256,
        derived,
        cover_source,
        cover_status,
    })
}

/// Run `scan` over every object in the dataset, in parallel.
fn scan_objects<T, F>(root: &Path, jobs: Option<usize>, scan: F) -> anyhow::Result<Vec<T>>
where
    T: Send,
    F: Fn(&Path) -> T + Sync,
{
    let paths = collect_objects(root)?;
    let workers =
        jobs.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |value| value.get()));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results: Mutex<Vec<T>> = Mutex::new(Vec::with_capacity(paths.len()));
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                let mut local = Vec::new();
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    let Some(path) = paths.get(index) else { break };
                    local.push(scan(path));
                }
                results.lock().unwrap().extend(local);
            });
        }
    });
    Ok(results.into_inner().unwrap())
}

fn write_jsonl<T: serde::Serialize>(records: &[T], out: Option<String>) -> anyhow::Result<()> {
    let body: String = records
        .iter()
        .map(|record| serde_json::to_string(record).unwrap() + "\n")
        .collect();
    match out {
        Some(path) => std::fs::write(&path, body)?,
        None => print!("{body}"),
    }
    Ok(())
}

fn collect_objects(root: &Path) -> anyhow::Result<Vec<std::path::PathBuf>> {
    let mut paths = Vec::new();
    let mut stack = vec![root.join("objects")];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(OsStr::to_str).is_some_and(|ext| {
                matches!(ext.to_ascii_lowercase().as_str(), "w3x" | "w3m" | "w3n")
            }) {
                paths.push(path);
            }
        }
    }
    paths.sort();
    Ok(paths)
}

fn main() -> anyhow::Result<()> {
    let args = Command::parse();
    match args {
        Command::BuildCatalog {
            input_dir,
            out_dir,
            hf_repo,
            no_archives,
        } => catalog::build(catalog::BuildOptions {
            input_dir: Path::new(&input_dir),
            out_dir: Path::new(&out_dir),
            hf_repo: hf_repo.as_deref(),
            inspect_archives: !no_archives,
        }),
        Command::ScanVersions {
            dataset_root,
            out,
            jobs,
        } => {
            let root = Path::new(&dataset_root);
            let mut records = scan_objects(root, jobs, scan_one)?;
            records.sort_by(|a, b| a.sha256.cmp(&b.sha256));
            write_jsonl(&records, out)?;
            let failed = records.iter().filter(|r| r.error.is_some()).count();
            eprintln!("scanned {} objects, {failed} unreadable", records.len());
            Ok(())
        }
        Command::ScanMods {
            dataset_root,
            out,
            jobs,
            all,
        } => {
            let root = Path::new(&dataset_root);
            let mut records = scan_objects(root, jobs, scan_mods_one)?;
            records.sort_by(|a, b| a.sha256.cmp(&b.sha256));
            let total = records.len();
            let found = records.iter().filter(|r| r.modification.is_some()).count();
            let failed = records.iter().filter(|r| r.error.is_some()).count();
            if !all {
                records.retain(|record| record.modification.is_some() || record.error.is_some());
            }
            write_jsonl(&records, out)?;
            eprintln!("scanned {total} objects, {found} modified, {failed} unreadable");
            Ok(())
        }
        Command::Rescan {
            dataset_root,
            out,
            jobs,
            only_failed,
            force_covers,
        } => {
            let root = Path::new(&dataset_root);
            let mut records: Vec<RescanRecord> =
                scan_objects(root, jobs, |path| rescan_one(path, root, force_covers))?
                    .into_iter()
                    .flatten()
                    .collect();
            records.sort_by(|a, b| a.sha256.cmp(&b.sha256));
            let total = records.len();
            let ok = records
                .iter()
                .filter(|r| r.derived.parse_status == "ok")
                .count();
            let covers = records.iter().filter(|r| r.cover_source.is_some()).count();
            let modified = records
                .iter()
                .filter(|r| r.derived.modification.is_some())
                .count();
            if only_failed {
                records.retain(|record| record.derived.parse_status != "ok");
            }
            write_jsonl(&records, out)?;
            eprintln!(
                "rescanned {total} objects, {ok} with metadata, {covers} covers recovered, {modified} modified"
            );
            Ok(())
        }
    }
}
