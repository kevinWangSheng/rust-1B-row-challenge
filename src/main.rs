use std::{
    fs::File,
    io::{self}
};

use memchr::memchr;
use memmap2::Mmap;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rustc_hash::FxHashMap;

fn main() -> io::Result<()> {
    let f = File::open("../1brc/measurements.txt").unwrap();
    let mmap = unsafe { Mmap::map(&f)? };

    let num_threads = std::thread::available_parallelism()?.get();

    let chunk_size = mmap.len() / num_threads;

    let mut chunks = Vec::new();
    let mut start = 0;
    for _ in 0..num_threads - 1 {
        // chunk the file
        let mut end = start + chunk_size;
        if let Some(pos) = memchr(b'\n', &mmap[end..]) {
            end += pos + 1;
        } else {
            end = mmap.len();
        }
        if end > mmap.len() {
            end = mmap.len();
        }
        chunks.push(&mmap[start..end]);
        start = end;
    }
    chunks.push(&mmap[start..]);

    let global_stats: FxHashMap<Vec<u8>, (i32, i32, i32, i32)> = chunks
        .par_iter()
        .map(|chunk| {
            let mut local_stats = FxHashMap::<Vec<u8>, (i32, i32, i32, i32)>::default();

            let mut offset = 0;
            while offset < chunk.len() {
                let remaining = &chunk[offset..];

                let semi_rel = memchr(b';', remaining).unwrap();
                let semi_pos = offset + semi_rel;

                let temp_start = semi_pos + 1;
                let nl_rel =
                    memchr(b'\n', &chunk[temp_start..]).unwrap_or(remaining.len() - semi_rel - 1);
                let next_offset = temp_start + nl_rel + 1;
                let station = &chunk[offset..semi_pos];
                let temp_bytes = &chunk[temp_start..temp_start + nl_rel];
                let temperature = parse_temp_fast(temp_bytes);
                // find the char ';'
                if let Some(entry) = local_stats.get_mut(station){
                    entry.0 = entry.0.min(temperature);
                    entry.1 += temperature;
                    entry.2 += 1;
                    entry.3 = entry.3.max(temperature);
                }else {
                    local_stats.insert(station.to_vec(), (temperature,temperature,1,temperature));
                }

                offset = next_offset;
            }
            local_stats
        })
        .reduce(
            || FxHashMap::default(),
            |mut a, b| {
                for (k, v) in b {
                    let entry = a.entry(k).or_insert((i32::MAX, 0, 0, i32::MIN));
                    entry.0 = entry.0.min(v.0);
                    entry.1 += v.1;
                    entry.2 += 1;
                    entry.3 = entry.3.max(v.3);
                }
                a
            },
        );
    print!("{{");

    let mut sort_by_keys: Vec<_> = global_stats.into_iter().collect();
    sort_by_keys.sort_by_key(|k| k.0.clone());
    for (station_bytes, (min, sum, count, max)) in sort_by_keys {
        let station = unsafe { std::str::from_utf8_unchecked(&station_bytes) };
        print!(
            "{}={:.1}/{:.1}/{:.1},",
            station,
            ((min as f64 / 10.0) as f64),
            (sum as f64 / count as f64 / 10.0),
            (max as f64 / 10.0)
        )
    }

    print!("}}");

    Ok(())
}

// just deal with the xx.x -xx.x x.x -x.x string format
#[inline(always)]
fn parse_temp_fast(bytes: &[u8]) -> i32 {
    let len = bytes.len();

    let last_digit = (bytes[len - 1] - b'0') as i32;

    // the format is xx.x
    let dot_pos = len - 2;
    let digit1 = (bytes[dot_pos - 1] - b'0') as i32;

    let mut value = digit1 * 10 + last_digit;

    if len == 3 {
        return value;
    }

    if len == 4 {
        if bytes[0] == b'-' {
            return -value;
        } else {
            // xx.x format
            let digit2 = (bytes[0] - b'0') as i32;
            value += digit2 * 100;
            return value;
        }
    }

    if len == 5 {
        let digit2 = (bytes[1] - b'0') as i32;
        value += digit2 * 100;
        return -value;
    }
    value
}
