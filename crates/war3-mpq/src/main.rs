use std::env;
use std::io::{self, Write};
use std::process;
use std::str;
use war3_mpq::Archive;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_usage(program: &str, opts: &getopts::Options) {
    let brief = format!("Usage: {} [options] MPQ_FILE", program);
    print!("{}", opts.usage(&brief));
}

fn list(archive_file_name: &str) {
    let mut archive = match Archive::open(archive_file_name) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };

    let file = match archive.open_file("(listfile)") {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };

    let mut buf: Vec<u8> = vec![0; file.size() as usize];

    match file.read(&mut archive, &mut buf) {
        Ok(_) => {}
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    }

    io::stdout().write_all(&buf).unwrap();
}

/// Walk the data region and report every member found, for archives whose
/// tables are unusable. Prints one line per member: index, offset, packed size,
/// sectors, and how much it decompresses to (or why it does not).
fn salvage(archive_file_name: &str) {
    let mut archive = match Archive::open(archive_file_name) {
        Ok(v) => v,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };

    let members = archive.salvage_members();
    for (i, member) in members.iter().enumerate() {
        // An encrypted member's key was recovered from its own sector table, so
        // it is worth showing: it is the difference between a member the walk
        // read and one it only stepped over.
        let key = match member.key {
            Some(key) => format!(" key={:#010x}", key),
            None => String::new(),
        };
        // Four inflated bytes name the member; inflating it whole to read them
        // would decompress every script and texture in the archive.
        match archive.peek_salvaged(member, 4) {
            // Names live only in the hash table, so the leading bytes are all a
            // caller has to identify a salvaged member by.
            Ok(magic) => println!(
                "{:4} offset={:#x} packed={} sectors={}{} magic={}",
                i,
                member.offset,
                member.packed_size,
                member.sector_count(),
                key,
                magic
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>()
            ),
            Err(e) => println!(
                "{:4} offset={:#x} packed={} sectors={}{} error={}",
                i,
                member.offset,
                member.packed_size,
                member.sector_count(),
                key,
                e
            ),
        }
    }
    let encrypted = members.iter().filter(|m| m.key.is_some()).count();
    println!("{} member(s), {} encrypted", members.len(), encrypted);
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let program = args[0].clone();
    let mut opts = getopts::Options::new();

    opts.optopt("x", "extract", "extract file from archive", "FILE");
    opts.optflag("o", "to-stdout", "extract file to standard output");
    opts.optflag("l", "list", "print (listfile) contents");
    opts.optflag(
        "s",
        "salvage",
        "walk the data region and list members without using the tables",
    );
    opts.optflag("v", "version", "print version info");
    opts.optflag("h", "help", "print this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!("{}", f.to_string()),
    };

    if matches.opt_present("version") {
        println!("{} {}", program, VERSION);
        return;
    }

    if matches.opt_present("help") {
        print_usage(&program, &opts);
        return;
    }

    let archive_file_name = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        print_usage(&program, &opts);
        return;
    };

    if matches.opt_present("list") {
        list(&archive_file_name);
        return;
    }

    if matches.opt_present("salvage") {
        salvage(&archive_file_name);
        return;
    }

    if let Some(filename) = matches.opt_str("extract") {
        let mut archive = match Archive::open(archive_file_name) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", e);
                process::exit(1);
            }
        };

        let file = archive.open_file(&filename).unwrap();

        if matches.opt_present("to-stdout") {
            let mut buf: Vec<u8> = vec![0; file.size() as usize];

            match file.read(&mut archive, &mut buf) {
                Ok(_) => {}
                Err(e) => {
                    println!("{}", e);
                    process::exit(1);
                }
            }

            io::stdout().write_all(&buf).unwrap();
        } else {
            match file.extract(&mut archive, &filename) {
                Ok(_) => {}
                Err(e) => {
                    println!("{}", e);
                    process::exit(1);
                }
            }
        }
    }
}
