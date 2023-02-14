use std::{
    fs::{self, File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use memmap2::MmapOptions;
use memmem::{Searcher, TwoWaySearcher};

fn main() -> anyhow::Result<()> {
    let orig_wzsound_file = Path::new("WZSound.brsar");
    let modified_wzsound_file = Path::new("WZModified/WZSound.brsar");
    if !orig_wzsound_file.is_file() {
        bail!("original WZSound.brsar file doesn't exist, place it next to this binary");
    }
    if !modified_wzsound_file.exists() {
        fs::create_dir_all("WZModified").context("could not create WZModified dir!")?;
        fs::copy(orig_wzsound_file, modified_wzsound_file)
            .context("couldn't copy WZSound for backup")?;
    }
    // create readonly memmap of original (for searching)
    let orig_content = unsafe {
        MmapOptions::new().map(
            &File::open(orig_wzsound_file)
                .context("couldn't open original WZSound.brsar for reading")?,
        )
    }
    .context("could not create memmap of original WZSound.brsar")?;
    // open patched for writing
    let mut modified_file = OpenOptions::new()
        .write(true)
        .open(modified_wzsound_file)
        .context("couldn't open modified WZSound.brsar")?;

    // get patches
    for i in 1..=500 {
        let orig_audio_path = PathBuf::from(format!("Original Audio/exported/Audio[{i}]"));
        let replacement_audio_path =
            PathBuf::from(format!("Replacement Audio/exported/Audio[{i}]"));
        match (orig_audio_path.exists(), replacement_audio_path.exists()) {
            (false, false) => (),
            (true, false) | (false, true) => {
                println!("only one of original and replacement exists, ignoring Audio[{i}]");
            }
            (true, true) => {
                let orig_audio = fs::read(&orig_audio_path)
                    .with_context(|| format!("could not read {orig_audio_path:?}"))?;
                let mut replacement_audio = fs::read(&replacement_audio_path)
                    .with_context(|| format!("could not read {replacement_audio_path:?}"))?;
                if orig_audio.len() < replacement_audio.len() {
                    println!("replacement is bigger that original for Audio[{i}], ignoring");
                    continue;
                }
                while replacement_audio.len() < orig_audio.len() {
                    replacement_audio.push(0);
                }
                println!("starting replacement for Audio[{i}]");
                let mut current_pos = 0;
                let searcher = TwoWaySearcher::new(&orig_audio);
                while current_pos < orig_content.len() {
                    let Some(offset) = searcher.search_in(&orig_content[current_pos..]) else {
                        break;
                    };
                    current_pos += offset;
                    println!("found match for Audio[{i}] at {current_pos}");
                    modified_file
                        .seek(SeekFrom::Start(current_pos as u64))
                        .with_context(|| format!("seek for Audio[{i}] to {current_pos} failed!"))?;
                    modified_file
                        .write_all(&replacement_audio)
                        .with_context(|| {
                            format!("write for Audio[{i}] to {current_pos} failed!")
                        })?;
                    println!("match for Audio[{i}] at {current_pos} written");
                    current_pos += orig_audio.len();
                }
            }
        }
    }
    Ok(())
}
