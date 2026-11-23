//! Replays captured prove-proxy requests through the local circuit manager, natively:
//! `cargo run --release --example lab_replay --features lab-bench -- <fixture dir> [threads] [limit per method]`.
//! Prints one JSON line per request (metadata and timings only, never proof or witness data).
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let dir = args.next().ok_or_else(|| anyhow::anyhow!("fixture directory required"))?;
    let threads: usize = args.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let limit: usize = args.next().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    if threads > 0 {
        rayon::ThreadPoolBuilder::new().num_threads(threads).build_global()?;
    }
    use psy_rust_sdk::lab_bench::*;
    println!("{}", serde_json::json!({"event": "init", "threads": rayon_threads(), "ms": replay_init()?}));

    println!("{{\"event\":\"fingerprints\",\"value\":{}}}", replay_fingerprints()?);
    let mut files: Vec<_> = std::fs::read_dir(&dir)?.filter_map(|e| e.ok().map(|e| e.path())).collect();
    files.sort();
    let stem = |p: &std::path::Path| p.file_name().unwrap().to_string_lossy().to_string();
    for path in files.iter().filter(|p| stem(p).starts_with("psy_resolve_contract_function")) {
        let contract_id = replay_add_contract_code_fixture(&std::fs::read_to_string(path)?)?;
        println!("{}", serde_json::json!({"event": "contract_code", "contract_id": contract_id}));
    }

    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    for path in files.iter().filter(|p| stem(p).starts_with("psy_prove_")) {
        let name = stem(path);
        let method = name.rsplit_once('-').map(|(m, _)| m.to_string()).unwrap_or_default();
        let count = seen.entry(method).or_default();
        if *count >= limit {
            continue;
        }
        *count += 1;
        let fixture = std::fs::read_to_string(path)?;
        let t = now_ms();
        match replay(&fixture) {
            Ok(line) => println!("{{\"event\":\"replay\",\"file\":\"{name}\",\"total_ms\":{:.1},\"result\":{line}}}", now_ms() - t),
            Err(error) => println!("{}", serde_json::json!({"event": "error", "file": name, "error": format!("{error:#}")})),
        }
    }
    Ok(())
}
