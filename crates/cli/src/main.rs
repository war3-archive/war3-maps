use std::path::Path;

use clap::Parser;
use war3parser::prelude::{War3Image, War3MapMetadata, War3MapW3x};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
enum Command {
    /// Dump metadata from a map file
    #[command(visible_alias = "d")]
    DumpMetadata {
        /// Path to the MPQ archive
        map_path: String,

        /// Path to the output directory
        #[arg(short, long, default_value = ".")]
        out_dir: String,
    },

    /// Extract a file from a MPQ archive and save it
    #[command(visible_alias = "x")]
    ExtractFile {
        /// Path to the MPQ archive
        map_path: String,

        /// File name to extract
        #[arg(short, long)]
        file_name: String,

        /// Path to the output directory
        #[arg(short, long, default_value = ".")]
        out_dir: String,
    },

    /// Extract images with *.tga and *.blp extensions
    #[command(visible_alias = "i")]
    ExtractImages {
        /// Path to the MPQ archive
        map_path: String,

        /// Path to the output directory
        #[arg(short, long, default_value = ".")]
        out_dir: String,

        /// Keep the original file format. Default is to convert to PNG
        #[arg(short, long, default_value = "false")]
        keep_ori: bool,
    },

    /// Convert a *tga/blp file to png
    #[command(visible_alias = "c")]
    ConvertImage {
        /// Path to the image file
        file_name: String,

        /// Path to the output directory
        #[arg(short, long)]
        out_dir: Option<String>,
    },

    /// List files in a MPQ archive
    #[command(visible_alias = "l")]
    ListFiles {
        /// Path to the MPQ archive
        map_path: String,
    },
}

fn extract_file_buffer(w3x: &mut War3MapW3x, file_name: &str) -> anyhow::Result<Vec<u8>> {
    let out_file_file = w3x.get(file_name)?;
    let mut out_file_content = vec![0u8; out_file_file.size() as usize];
    out_file_file.read(&mut w3x.archive, &mut out_file_content)?;
    Ok(out_file_content)
}

fn save_file(file_name: &str, out_dir: &Path, file_content: &[u8]) -> anyhow::Result<()> {
    let out_file_path = out_dir.join(file_name.replace('\\', "/"));
    if let Some(parent) = out_file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(out_file_path, file_content)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args = Command::parse();
    match args {
        Command::ExtractFile {
            map_path,
            file_name,
            out_dir,
        } => {
            let map_path = Path::new(&map_path);
            let out_dir = Path::new(&out_dir);

            let mut w3x = War3MapW3x::new(map_path.to_path_buf())?;
            let file_content = extract_file_buffer(&mut w3x, &file_name)?;
            save_file(&file_name, out_dir, &file_content)?;
            println!("✅ Extracted file: {file_name}");
            Ok(())
        }
        Command::ListFiles { map_path } => {
            let map_path = Path::new(&map_path);
            let w3x = War3MapW3x::new(map_path.to_path_buf())?;
            let files = w3x
                .files
                .ok_or_else(|| anyhow::anyhow!("Failed to get files from MPQ"))?;
            println!("{files:#?}");
            Ok(())
        }
        Command::ExtractImages {
            map_path,
            out_dir,
            keep_ori,
        } => {
            let map_path = Path::new(&map_path);
            let out_dir = Path::new(&out_dir);

            let mut w3x = War3MapW3x::new(map_path.to_path_buf())?;
            let files = w3x.files.clone().unwrap_or_default();

            fn handle_image(
                w3x: &mut War3MapW3x,
                file: &str,
                out_dir: &Path,
                keep_ori: bool,
            ) -> anyhow::Result<()> {
                let file_content = extract_file_buffer(w3x, file)?;
                if keep_ori {
                    save_file(file, out_dir, &file_content)?;
                } else {
                    let image = War3Image::from_buffer(&file_content, file)?;
                    let out_file_path = out_dir.join(
                        file.replace('\\', "/")
                            .replace(".blp", ".png")
                            .replace(".tga", ".png"),
                    );
                    if let Some(parent) = out_file_path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    image.data.save(out_file_path)?;
                }
                Ok(())
            }

            for file in files
                .iter()
                .filter(|file| file.ends_with(".tga") || file.ends_with(".blp"))
            {
                match handle_image(&mut w3x, file, out_dir, keep_ori) {
                    Ok(_) => println!("✅ Extracted image: {file}"),
                    Err(e) => println!("❌ Failed to extract '{file}': {e}"),
                }
            }
            Ok(())
        }
        Command::ConvertImage { file_name, out_dir } => {
            let image_path = Path::new(&file_name);
            let file_content = std::fs::read(image_path)?;

            let image = War3Image::from_buffer(&file_content, &file_name)?;
            let out_file_path = if let Some(out_dir) = out_dir {
                Path::new(&out_dir).join(
                    image_path
                        .with_extension("png")
                        .file_name()
                        .ok_or_else(|| anyhow::anyhow!("Failed to get file name"))?,
                )
            } else {
                image_path.with_extension("png")
            };
            image.data.save(&out_file_path)?;
            println!("✅ Converted image: {}", out_file_path.display());
            Ok(())
        }
        Command::DumpMetadata { map_path, out_dir } => {
            let map_path = Path::new(&map_path);
            let out_dir = Path::new(&out_dir);
            if !out_dir.exists() {
                std::fs::create_dir_all(out_dir)?;
            }
            let buffer = std::fs::read(map_path)?;
            let mut metadata = War3MapMetadata::from(&buffer).ok_or_else(|| {
                anyhow::anyhow!("Failed to parse metadata from '{}'", map_path.display())
            })?;
            metadata.update_string_table().ok();
            metadata.save(
                out_dir
                    .to_str()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get output directory"))?,
            )?;
            println!(
                "✅ Dumped '{}' metadata to '{}'",
                map_path.display(),
                out_dir.display()
            );
            Ok(())
        }
    }
}
