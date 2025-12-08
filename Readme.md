# 1 Billion Row Challenge (1BRC) in Rust 🦀

This is my implementation of the [One Billion Row Challenge](https://github.com/gunnarmorling/1brc) using Rust. The goal is to aggregate 1 billion rows of meteorological data (approx. 13GB) as fast as possible.

Through a series of optimizations—ranging from memory management to SIMD instructions—I reduced the processing time from **1 minute 53 seconds** to approximately **1.5 seconds**.

## 🚀 Performance Results

| Stage | Implementation Details | Time (approx.) |
| :--- | :--- | :--- |
| **Baseline** | `BufReader`, `String` allocation per line, `BTreeMap` | **1m 53s** |
| **Opt 1** | `HashMap`, Buffer reuse (`read_line`), `f64` parsing | **1m 03s** |
| **Opt 2** | `FxHashMap`, `Vec<u8>` (Zero-Copy), manual float parsing | **32s** |
| **Opt 3** | Parallelism (`Rayon`) + Memory Mapping (`Mmap`) | **10s** |
| **Final** | **SIMD (`memchr`) + Branchless Parsing + Zero Allocation** | **~1.5s (Hot)** / 8.6s (Cold) |

> **Environment**:
> - **CPU**: AMD Ryzen AI 9 HX 370
> - **OS**: Arch Linux (WSL2)
> - **Disk**: NVMe SSD

## 🛠️ Optimization Journey

### 1. The Naive Approach (Baseline)
I started with idiomatic, high-level Rust code.
- Used `BufReader::lines()` which allocates a new `String` for every line.
- Used `BTreeMap` to keep keys sorted (as required by the output format).
- **Bottleneck**: Massive heap allocations and tree rebalancing.

### 2. Reducing Allocations
- Switched to `HashMap` for O(1) lookups.
- Reused a single `String` buffer for reading lines to reduce allocator pressure.
- **Result**: ~45% speedup.

### 3. Byte-Level Processing & FxHash
- **Hashing**: Switched to `FxHashMap` (from `rustc-hash`), which is much faster than the default DDoS-resistant SipHash.
- **No UTF-8 Checks**: Switched from `String` to `Vec<u8>` using `read_until`. This bypasses Rust's expensive UTF-8 validation.
- **Custom Parsing**: Wrote a manual parser for the temperature to avoid `f64::parse` overhead.
- **Key Optimization**: Implemented a "Lookup First" strategy. I only allocate memory for the station name (`to_vec()`) if the key is *not* found in the map.
- **Result**: ~50% speedup.

### 4. Parallelism & Mmap
- **Memory Mapping**: Replaced `BufReader` with `memmap2`. This maps the file directly into virtual memory, reducing system call overhead.
- **Rayon**: Used data parallelism to split the 13GB file into chunks (aligned to newlines) and processed them across all CPU cores.
- **Map Reduce**: Each thread computes a local HashMap, which are then merged into a global one.
- **Result**: Massive speedup, bringing time down to ~10s.

### 5. SIMD & Branchless Logic (The Final Polish)
- **SIMD**: Replaced byte-by-byte loops with `memchr`. This uses AVX/SSE instructions to scan for `;` and `\n` (32 bytes at a time).
- **Branchless Parsing**: The temperature format is fixed (`X.X`, `XX.X`, etc.). I implemented a branchless integer parser that converts bytes to integers using raw arithmetic, avoiding CPU branch mispredictions.
- **Fixed Allocation Bug**: I discovered I was accidentally allocating `Vec<u8>` for *every* lookup in the parallel version. Fixed this to strictly use references `&[u8]` for lookups.
- **Result**: Reached the hardware limit of **~1.5s** (warm cache).
