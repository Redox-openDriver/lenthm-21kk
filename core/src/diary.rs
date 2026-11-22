use std::io::{self, Read, Write};
use std::path::Path;

use calor::Sample;

const KEPT_DAYS: u32 = 120;
const LIMIT: u64 = 1024 * 1024;

pub fn samples(path: &Path, today: u32) -> io::Result<Vec<Sample>> {
    let mut text = String::new();
    match std::fs::File::open(path) {
        Ok(file) => {
            file.take(LIMIT).read_to_string(&mut text)?;
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    }
    Ok(text
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let (day, bucket, rise) = (words.next()?.parse::<u32>().ok()?, words.next()?.parse::<u8>().ok()?, words.next()?.parse::<f32>().ok()?);
            (words.next().is_none() && rise.is_finite()).then_some(Sample { day, bucket, rise })
        })
        .filter(|sample| sample.day <= today && sample.day + KEPT_DAYS >= today)
        .collect())
}

pub fn keep(path: &Path, sample: Sample, today: u32) -> io::Result<()> {
    let mut all = samples(path, today)?;
    all.push(sample);
    let text: String = all.iter().map(|sample| format!("{} {} {:.2}\n", sample.day, sample.bucket, sample.rise)).collect();
    let fresh = path.with_extension("new");
    let mut file = std::fs::File::create(&fresh)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    std::fs::rename(&fresh, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cooling_diary_keeps_four_months_and_skips_what_it_cannot_read() {
        let dir = std::env::temp_dir().join(format!("lenthm-diary-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("cooling");
        assert!(samples(&path, 20_000).unwrap().is_empty());
        keep(&path, Sample { day: 19_800, bucket: 2, rise: 21.5 }, 19_800).unwrap();
        keep(&path, Sample { day: 20_000, bucket: 3, rise: 25.25 }, 20_000).unwrap();
        assert_eq!(samples(&path, 20_000).unwrap(), [Sample { day: 20_000, bucket: 3, rise: 25.25 }]);
        std::fs::write(&path, "20000 2 21.5\nnonsense\n20000 2 NaN\n20000 2 1 extra\n30000 1 9\n").unwrap();
        assert_eq!(samples(&path, 20_000).unwrap(), [Sample { day: 20_000, bucket: 2, rise: 21.5 }]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
