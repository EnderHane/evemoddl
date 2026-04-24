use std::{
    fs,
    io::{
        Read,
        Write,
    },
    path::Path,
};

use anyhow::{
    Context,
    Result,
};
use indicatif::{
    ProgressBar,
    ProgressStyle,
};
use xxhash_rust::xxh64::Xxh64;

pub async fn download_file(url: &str, dest: &Path) -> Result<()> {
    let mut response = reqwest::get(url)
        .await
        .with_context(|| format!("Failed to send request to {}", url))?;

    let total_size = response.content_length();

    let pb = if let Some(size) = total_size {
        let bar = ProgressBar::new(size);
        bar.set_style(
            ProgressStyle::default_bar()
                .template(
                    "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
                )
                .unwrap()
                .progress_chars("#>-")
        );
        bar
    } else {
        ProgressBar::new_spinner()
    };

    let mut file = if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).context("Failed to create directory for downloaded file")?;
        fs::File::create(dest).context("Failed to create file")?
    } else {
        fs::File::create(dest).context("Failed to create file")?
    };

    while let Some(chunk) = response
        .chunk()
        .await
        .with_context(|| format!("Failed to read chunk from {}", url))?
    {
        file.write_all(&chunk)
            .context("Failed to write chunk to file")?;
        pb.inc(chunk.len() as u64);
    }

    pb.finish_and_clear();
    Ok(())
}

pub fn verify_xxhash(path: &Path, expected_hashes: &[String]) -> Result<String> {
    let actual = file_xxhash(path)?;
    let matches = expected_hashes
        .iter()
        .any(|expected| expected.trim().eq_ignore_ascii_case(&actual));

    if !matches {
        anyhow::bail!(
            "xxHash mismatch for {:?}: expected one of [{}], got {}",
            path,
            expected_hashes.join(", "),
            actual
        );
    }

    Ok(actual)
}

fn file_xxhash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).context("Failed to open downloaded file for hashing")?;
    let mut hasher = Xxh64::new(0);
    let mut buffer = [0; 64 * 1024];

    loop {
        let bytes_read = file
            .read(&mut buffer)
            .context("Failed to read downloaded file for hashing")?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(format!("{:016x}", hasher.digest()))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{
            SystemTime,
            UNIX_EPOCH,
        },
    };

    use super::verify_xxhash;

    #[test]
    fn verifies_matching_xxhash() {
        let path = std::env::temp_dir().join(format!(
            "evemoddl-xxhash-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, b"hello").unwrap();

        let result = verify_xxhash(&path, &["26c7827d889f6da3".to_string()]);

        fs::remove_file(&path).unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn rejects_mismatched_xxhash() {
        let path = std::env::temp_dir().join(format!(
            "evemoddl-xxhash-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, b"hello").unwrap();

        let result = verify_xxhash(&path, &["0000000000000000".to_string()]);

        fs::remove_file(&path).unwrap();
        assert!(result.is_err());
    }
}
