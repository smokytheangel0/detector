//! This file contains all std-dependent logic, compiled only
//! when the "std" feature is enabled.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};

// Import the no_std items from the parent library *with new names*
use super::{
    calculate_statistics_static,
    detect_static,
    tally_lengths_static,
    ten_to_two_static,
    DetectionStatic,
    DetectionType as NoStdDetectionType, // Renamed
    EntropyStatisticsStatic,
    Timings as NoStdTimings, // Renamed
    BIT_WIDTH,
};

// --- CONSTANTS ---
// These are the parameters for the std-based "source_and_calculate"
#[cfg(debug_assertions)]
pub const ITERATIONS: usize = 10_000;
#[cfg(not(debug_assertions))]
pub const ITERATIONS: usize = 1_000_000;
const TOTAL_BITS: usize = ITERATIONS * BIT_WIDTH;
/// Worst case for runs is alternating bits (e.g., 010), giving N/3 runs.
/// We'll use N/2 as a safe upper bound.
const MAX_RUNS_BUFFER: usize = TOTAL_BITS / 2;
/// The max length a run can be is TOTAL_BITS. We need one extra
/// slot so `[TOTAL_BITS]` is a valid index.
const MAX_TALLY_LEN: usize = TOTAL_BITS + 1;
// Set stack size to 128 MiB (1024 * 1024 * 1024)
#[cfg(not(debug_assertions))]
const THREAD_STACK_SIZE: usize = 1024 * 1024 * 1024;
#[cfg(debug_assertions)]
const THREAD_STACK_SIZE: usize = 8 * 1024 * 1024;
// --- Serde-based (std) Structs for JSON ---

// --- NEW std-specific, serializable enums/structs ---
// These are the new "bridge" types.

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum DetectionType {
    Zeros,
    Ones,
    Alternating,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Timings {
    pub collection_start: f64,
    pub collection_end: f64,
    pub processing_start: f64,
    pub processing_end: Option<f64>,
}

// --- NEW Converter functions ---
//should do this conversion in std_impl only, then we can alias it in there,
//and keep these names clean, and have the deserialization only in std_impl
fn to_no_std_detection_type(std_type: DetectionType) -> NoStdDetectionType {
    match std_type {
        DetectionType::Zeros => NoStdDetectionType::Zeros,
        DetectionType::Ones => NoStdDetectionType::Ones,
        DetectionType::Alternating => NoStdDetectionType::Alternating,
    }
}

fn from_no_std_timings(no_std: NoStdTimings) -> Timings {
    Timings {
        collection_start: no_std.collection_start,
        collection_end: no_std.collection_end,
        processing_start: no_std.processing_start,
        processing_end: no_std.processing_end,
    }
}

// --- End new sections ---

#[derive(Serialize, Deserialize, Debug)]
struct History {
    zero_counts: HashMap<u64, u64>,
    one_counts: HashMap<u64, u64>,
    alternating_counts: HashMap<u64, u64>,
    total: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DetectionStd {
    pub run_count: HashMap<u64, u64>,
    pub detection_type: DetectionType, // Uses NEW std type
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EntropyStatisticsStd {
    pub detection: DetectionStd,
    pub mode: SourceMode, // Re-adding this from your original code
    pub total_runs: u64,
    pub total_bits: u64,
    pub avg_map: HashMap<u64, f64>,
    pub long_ratio: f64,
    pub longest: u64,
    pub unique_lengths: u64,
    pub timings: Timings, // Uses NEW std type
}

#[derive(Serialize, Deserialize, Debug)]
pub enum SourceMode {
    SystemRandom,
    // Add other modes here if needed
}

// --- Global History (std) ---

static HISTORY: OnceLock<History> = OnceLock::new();

/// This should be called on app start from `main.rs`.
/// This is your original function, but with `expect` and `panic`
/// replaced with `eprintln` and error handling.
pub fn get_history() {
    let file = match File::options().read(true).open("sequence_history.json") {
        Ok(file) => Some(file),
        Err(_) => {
            // File doesn't exist, init with empty history
            HISTORY.get_or_init(|| History {
                zero_counts: HashMap::new(),
                one_counts: HashMap::new(),
                alternating_counts: HashMap::new(),
                total: 0,
            });
            None
        }
    };

    if let Some(mut file) = file {
        let mut file_contents = String::new();
        if file.read_to_string(&mut file_contents).is_err() {
            eprintln!("Failed to read from sequence_history.json");
            return;
        }

        let mut total = 0;
        let mut zero_counts: HashMap<u64, u64> = HashMap::new();
        let mut one_counts: HashMap<u64, u64> = HashMap::new();
        let mut alternating_counts: HashMap<u64, u64> = HashMap::new();

        for (line_number, line) in file_contents.split('\n').enumerate() {
            if line.is_empty() {
                continue;
            }
            let statistics: EntropyStatisticsStd = match serde_json::from_str(line) {
                Ok(stats) => stats,
                Err(e) => {
                    eprintln!(
                        "Failed to read line {} from sequence_history.json: {}",
                        line_number, e
                    );
                    continue; // Skip bad lines
                }
            };

            total += statistics.total_bits;
            for (length, &count) in &statistics.detection.run_count {
                let map_to_update = match statistics.detection.detection_type {
                    DetectionType::Zeros => &mut zero_counts, // Uses NEW std type
                    DetectionType::Ones => &mut one_counts,   // Uses NEW std type
                    DetectionType::Alternating => &mut alternating_counts, // Uses NEW std type
                };
                *map_to_update.entry(*length).or_insert(0) += count;
            }
        }

        HISTORY.get_or_init(|| History {
            zero_counts,
            one_counts,
            alternating_counts,
            total,
        });
    }
}

/// Saves a single timestep to the JSON history file.
fn save_timestep(statistics: &EntropyStatisticsStd) {
    let mut file = match File::options()
        .append(true)
        .create(true)
        .open("sequence_history.json")
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Failed to open sequence_history file: {}", e);
            return;
        }
    };

    let json_string = match serde_json::to_string(&statistics) {
        Ok(s) => s,
        Err(e) => {
            panic!("Failed to serialize statistics: {}", e);
        }
    };
    writeln!(&mut file, "{json_string}",).expect("failed to write new timestep to sequence file");
}
// --- Helper to convert static to std ---

/// Helper to convert the static, no_std stats struct into the
/// std, serde-serializable struct.
fn convert_static_to_std(
    static_stats: EntropyStatisticsStatic<MAX_TALLY_LEN>,
    detection_type: NoStdDetectionType, // Takes the NO_STD type
) -> EntropyStatisticsStd {
    let history = HISTORY
        .get()
        .expect("HISTORY must be initialized with get_history()");

    let mut run_count_map = HashMap::new();
    let mut avg_map = HashMap::new();

    // Convert no_std type to std type for matching and struct creation
    let std_detection_type = match detection_type {
        NoStdDetectionType::Zeros => DetectionType::Zeros,
        NoStdDetectionType::Ones => DetectionType::Ones,
        NoStdDetectionType::Alternating => DetectionType::Alternating,
    };

    let (history_counts, history_total) = match std_detection_type {
        // Match on NEW std type
        DetectionType::Zeros => (&history.zero_counts, history.total),
        DetectionType::Ones => (&history.one_counts, history.total),
        DetectionType::Alternating => (&history.alternating_counts, history.total),
    };

    for (length_idx, &count) in static_stats.detection.run_counts.iter().enumerate() {
        if count > 0 {
            let length = length_idx as u64;
            run_count_map.insert(length, count);
        }
    }

    if history_total > 0 {
        for (&length, &total_count) in history_counts {
            avg_map.insert(length, total_count as f64 / history_total as f64);
        }
    }

    EntropyStatisticsStd {
        detection: DetectionStd {
            run_count: run_count_map,
            detection_type: std_detection_type, // Use NEW std type
        },
        mode: SourceMode::SystemRandom,
        total_runs: static_stats.total_runs,
        total_bits: static_stats.total_bits,
        avg_map,
        long_ratio: static_stats.total_longs_in_runs as f64 / static_stats.total_runs as f64,
        longest: static_stats.longest,
        unique_lengths: static_stats.unique_lengths,
        timings: from_no_std_timings(static_stats.timings), // Use converter
    }
}

/// The main std-enabled entrypoint, called by the TUI.
/// This function allocates on the heap, spawns threads, and
/// calls the `no_std` static functions with slices.
pub fn source_and_calculate(long_threshold: u64) -> Vec<EntropyStatisticsStd> {
    let collection_start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before 1970")
        .as_secs_f64();

    // --- Data Collection (Heap-allocated) ---
    // Use `with_capacity` as indicated by your flamegraph analysis
    let mut source_data: Vec<u64> = Vec::with_capacity(ITERATIONS);
    for _ in 0..ITERATIONS {
        source_data.push(OsRng.next_u64());
    }

    let collection_end = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before 1970")
        .as_secs_f64();

    let processing_start = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time is before 1970")
        .as_secs_f64();

    // Use the NO_STD Timings struct for the no_std functions
    let timings = NoStdTimings {
        collection_start,
        collection_end,
        processing_start,
        processing_end: None,
    };

    // --- Bit Conversion (Heap-allocated) ---
    // Create the bit buffer on the heap
    let mut bit_buffer: Vec<bool> = vec![false; TOTAL_BITS];

    // Call the no_std function with a slice
    if let Err(e) = ten_to_two_static(&source_data, &mut bit_buffer) {
        eprintln!("Error in ten_to_two_static: {:?}", e);
        return vec![];
    }

    let bit_buffer = Arc::new(bit_buffer); // Share buffer between threads
    let mut threads = vec![];

    // --- Parallel Processing (std Threads) ---
    // Iterate over the NEW std enum
    for detection_type_std in [
        DetectionType::Alternating,
        DetectionType::Ones,
        DetectionType::Zeros,
    ] {
        let thread_bits = bit_buffer.clone();
        let thread_timings = timings; // This is the NoStdTimings

        // Convert to the no_std type to pass to the thread
        let detection_type_no_std = to_no_std_detection_type(detection_type_std);

        let detection_thread = thread::Builder::new()
            .name(format!("{:?}", detection_type_std)) // Use std type for name
            .stack_size(THREAD_STACK_SIZE)
            .spawn(move || {
                // --- Static Analysis (inside thread) ---
                // These buffers are large, so we Box them to ensure
                // they live on the heap, not the thread's stack.
                let mut runs_buffer: Box<Vec<u64>> = Box::new(vec![0; MAX_RUNS_BUFFER]);

                // Box this huge array to keep it off the stack
                let mut detection_store = Box::new(
                    DetectionStatic::<MAX_TALLY_LEN>::new(detection_type_no_std), // Use no_std type
                );

                // Call no_std `detect`
                let run_count = match detect_static(
                    &thread_bits,
                    detection_type_no_std, // Use no_std type
                    &mut runs_buffer,
                ) {
                    Ok(count) => count,
                    Err(e) => {
                        eprintln!("Error in detect_static: {:?}", e);
                        return None; // Return None on error
                    }
                };

                // Call no_std `tally`
                if let Err(e) =
                    tally_lengths_static(&runs_buffer[..run_count], &mut detection_store.run_counts)
                {
                    eprintln!("Error in tally_lengths_static: {:?}", e);
                    return None;
                }

                // Call no_std `calculate`
                let processing_end = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("system time is before 1970")
                    .as_secs_f64();

                let final_timings = NoStdTimings {
                    // This is the no_std struct
                    processing_end: Some(processing_end),
                    ..thread_timings
                };

                let static_stats = calculate_statistics_static(
                    TOTAL_BITS as u64,
                    long_threshold,
                    *detection_store, // Deref the Box
                    final_timings,    // Pass the no_std struct
                );

                // Convert to std struct for serialization and return
                let std_stats = convert_static_to_std(static_stats, detection_type_no_std);

                // Save to file *inside* the thread
                save_timestep(&std_stats);

                Some(std_stats)
            })
            .expect("failed to spawn thread");

        threads.push(detection_thread);
    }

    let mut all_statistics = vec![];
    for thread in threads {
        match thread.join() {
            Ok(Some(stats)) => all_statistics.push(stats),
            Ok(None) => eprintln!("A thread completed with an error."),
            Err(e) => eprintln!("A thread panicked: {:?}", e),
        }
    }
    all_statistics
}
