use anyhow::Result;
use clap::Parser;
use serde::Serialize;

/// ROI calculator for KV cache management.
///
/// Implements the formulas from docs/explain/bible-kv-cache.md §9
/// and kv-cache.md §11.  The cache breaks the equation
/// cost = users × requests; with good cache hit rate, cost becomes
/// proportional to the *novelty* of requests, not their volume.
#[derive(Parser)]
#[command(
    name = "cache-roi-calc",
    version,
    about = "Calculates the ROI of KV cache management"
)]
struct Cli {
    /// Average context length (tokens) per request.
    #[arg(long, default_value_t = 128000)]
    context_length: usize,

    /// Number of requests per hour.
    #[arg(long, default_value_t = 1000)]
    requests_per_hour: usize,

    /// Estimated cache hit rate (0.0 to 1.0).
    #[arg(long, default_value_t = 0.80)]
    hit_rate: f64,

    /// GPU cluster cost per hour in USD (e.g. 4×H100 SXM5 spot = $22.88/h).
    #[arg(long, default_value_t = 22.88)]
    gpu_cost_per_hour: f64,

    /// Storage cost per GiB per month in USD (Redis/S3).
    #[arg(long, default_value_t = 0.023)]
    storage_cost_per_gib_month: f64,

    /// Cache storage size in GiB (L2/L3 tiers combined).
    #[arg(long, default_value_t = 500)]
    storage_gib: usize,

    /// TTFT without cache in seconds.
    #[arg(long, default_value_t = 11.0)]
    ttft_without_cache_s: f64,

    /// TTFT with cache hit in seconds.
    #[arg(long, default_value_t = 1.5)]
    ttft_with_cache_s: f64,

    /// Output as JSON.
    #[arg(long, default_value_t = false)]
    json: bool,
}

#[derive(Serialize)]
struct RoiResult {
    hit_rate: f64,
    requests_per_hour: usize,
    context_length: usize,
    ttft_without_cache_s: f64,
    ttft_with_cache_s: f64,
    ttft_reduction_pct: f64,
    gpu_cost_without_cache_per_hour: f64,
    gpu_cost_with_cache_per_hour: f64,
    gpu_savings_per_hour: f64,
    gpu_savings_per_month: f64,
    storage_cost_per_month: f64,
    net_savings_per_month: f64,
    roi_ratio: f64,
    break_even_hit_rate: f64,
    recommendation: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.hit_rate < 0.0 || cli.hit_rate > 1.0 {
        return Err(anyhow::anyhow!("hit_rate must be between 0.0 and 1.0"));
    }

    // TTFT reduction
    let ttft_reduction_pct =
        (cli.ttft_without_cache_s - cli.ttft_with_cache_s) / cli.ttft_without_cache_s * 100.0;

    // GPU compute cost:
    // Without cache: every request does full prefill → full cost
    // With cache: only (1 - hit_rate) requests do full prefill,
    //             hit_rate requests do cache-hit (much cheaper)
    // The compute cost scales linearly with prefill time.
    let cost_per_request_no_cache = cli.ttft_without_cache_s * cli.gpu_cost_per_hour / 3600.0;
    let cost_per_request_with_cache =
        (cli.hit_rate * cli.ttft_with_cache_s + (1.0 - cli.hit_rate) * cli.ttft_without_cache_s)
            * cli.gpu_cost_per_hour
            / 3600.0;

    let gpu_cost_without = cost_per_request_no_cache * cli.requests_per_hour as f64;
    let gpu_cost_with = cost_per_request_with_cache * cli.requests_per_hour as f64;
    let gpu_savings_per_hour = gpu_cost_without - gpu_cost_with;
    let gpu_savings_per_month = gpu_savings_per_hour * 24.0 * 30.0;

    let storage_cost_per_month = cli.storage_cost_per_gib_month * cli.storage_gib as f64;
    let net_savings_per_month = gpu_savings_per_month - storage_cost_per_month;
    let roi_ratio = if storage_cost_per_month > 0.0 {
        net_savings_per_month / storage_cost_per_month
    } else {
        f64::INFINITY
    };

    // Break-even hit rate: solve gpu_savings_per_hour == storage_cost_per_hour.
    // gpu_savings(h) = h * (ttft_no - ttft_cache) * gpu_cost/3600 * rph
    // h_breakeven    = storage_per_hour / ((ttft_no - ttft_cache) * gpu_cost/3600 * rph)
    let storage_cost_per_hour = storage_cost_per_month / (24.0 * 30.0);
    let denom = cli.ttft_without_cache_s - cli.ttft_with_cache_s;
    let break_even = if denom > 0.0 {
        let raw = storage_cost_per_hour
            / (denom * (cli.gpu_cost_per_hour / 3600.0) * cli.requests_per_hour as f64);
        raw.clamp(0.0, 1.0)
    } else {
        0.0
    };

    let recommendation = if cli.hit_rate >= break_even {
        format!(
            "POSITIVE ROI: hit rate {:.0}% exceeds break-even {:.0}% — net savings ${:.2}/month",
            cli.hit_rate * 100.0,
            break_even * 100.0,
            net_savings_per_month
        )
    } else {
        format!(
            "NEGATIVE ROI: hit rate {:.0}% below break-even {:.0}% — increase hit rate or reduce storage cost",
            cli.hit_rate * 100.0,
            break_even * 100.0
        )
    };

    let result = RoiResult {
        hit_rate: cli.hit_rate,
        requests_per_hour: cli.requests_per_hour,
        context_length: cli.context_length,
        ttft_without_cache_s: cli.ttft_without_cache_s,
        ttft_with_cache_s: cli.ttft_with_cache_s,
        ttft_reduction_pct,
        gpu_cost_without_cache_per_hour: gpu_cost_without,
        gpu_cost_with_cache_per_hour: gpu_cost_with,
        gpu_savings_per_hour,
        gpu_savings_per_month,
        storage_cost_per_month,
        net_savings_per_month,
        roi_ratio,
        break_even_hit_rate: break_even,
        recommendation,
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("=== KV Cache ROI Analysis ===");
        println!("Hit rate              : {:.1}%", result.hit_rate * 100.0);
        println!("Requests/hour         : {}", result.requests_per_hour);
        println!("Context length        : {} tokens", result.context_length);
        println!();
        println!("TTFT without cache    : {:.1}s", result.ttft_without_cache_s);
        println!("TTFT with cache hit   : {:.1}s", result.ttft_with_cache_s);
        println!("TTFT reduction        : {:.1}%", result.ttft_reduction_pct);
        println!();
        println!("GPU cost without cache: ${:.2}/h", result.gpu_cost_without_cache_per_hour);
        println!("GPU cost with cache   : ${:.2}/h", result.gpu_cost_with_cache_per_hour);
        println!("GPU savings           : ${:.2}/h → ${:.2}/month", result.gpu_savings_per_hour, result.gpu_savings_per_month);
        println!();
        println!("Storage cost (L2/L3)  : ${:.2}/month ({} GiB @ ${}/GiB)", result.storage_cost_per_month, cli.storage_gib, cli.storage_cost_per_gib_month);
        println!("Net savings           : ${:.2}/month", result.net_savings_per_month);
        println!("ROI ratio             : {:.1}x", result.roi_ratio);
        println!("Break-even hit rate   : {:.1}%", result.break_even_hit_rate * 100.0);
        println!();
        println!("→ {}", result.recommendation);
        println!();
        println!("Reference: bible-kv-cache.md §9 (ROI = (GPU_saved - storage_cost) / storage_cost)");
    }

    Ok(())
}