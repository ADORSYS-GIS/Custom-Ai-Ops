use anyhow::Result;
use clap::Parser;
use serde::Serialize;

/// ROI calculator for KV cache management.
///
/// Implements the formulas from docs/explain/vllm-kv-cache.md §9
/// and kv-cache.md §11.  The cache breaks the equation
/// cost = users × requests; with good cache hit rate, cost becomes
/// proportional to the *novelty* of requests, not their volume.
///
/// Also compares heuristic (consistent-hash) vs precise (llm-d EPP)
/// routing to quantify the ROI of upgrading to llm-d.
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

    /// Estimated cache hit rate with heuristic routing (0.0 to 1.0).
    #[arg(long, default_value_t = 0.80)]
    hit_rate: f64,

    /// Estimated cache hit rate with precise (EPP) routing (0.0 to 1.0).
    /// If higher than --hit-rate, shows the ROI of upgrading to llm-d.
    #[arg(long, default_value_t = 0.0)]
    precise_hit_rate: f64,

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
struct RoutingComparison {
    routing_mode: String,
    hit_rate: f64,
    gpu_cost_per_hour: f64,
    gpu_savings_per_month: f64,
    ttft_avg_s: f64,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    precise_comparison: Option<PreciseComparison>,
}

#[derive(Serialize)]
struct PreciseComparison {
    heuristic: RoutingComparison,
    precise: RoutingComparison,
    hit_rate_delta: f64,
    additional_gpu_savings_per_month: f64,
    additional_net_savings_per_month: f64,
    routing_upgrade_roi: String,
}

fn compute_gpu_cost_per_hour(
    hit_rate: f64,
    ttft_no_cache: f64,
    ttft_cache: f64,
    gpu_cost_per_hour: f64,
    requests_per_hour: usize,
) -> f64 {
    let cost_per_request =
        (hit_rate * ttft_cache + (1.0 - hit_rate) * ttft_no_cache) * gpu_cost_per_hour / 3600.0;
    cost_per_request * requests_per_hour as f64
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.hit_rate < 0.0 || cli.hit_rate > 1.0 {
        return Err(anyhow::anyhow!("hit_rate must be between 0.0 and 1.0"));
    }
    if cli.precise_hit_rate < 0.0 || cli.precise_hit_rate > 1.0 {
        return Err(anyhow::anyhow!(
            "precise_hit_rate must be between 0.0 and 1.0"
        ));
    }

    // TTFT reduction
    let ttft_reduction_pct =
        (cli.ttft_without_cache_s - cli.ttft_with_cache_s) / cli.ttft_without_cache_s * 100.0;

    // GPU compute cost
    let cost_per_request_no_cache = cli.ttft_without_cache_s * cli.gpu_cost_per_hour / 3600.0;
    let gpu_cost_without = cost_per_request_no_cache * cli.requests_per_hour as f64;
    let gpu_cost_with = compute_gpu_cost_per_hour(
        cli.hit_rate,
        cli.ttft_without_cache_s,
        cli.ttft_with_cache_s,
        cli.gpu_cost_per_hour,
        cli.requests_per_hour,
    );
    let gpu_savings_per_hour = gpu_cost_without - gpu_cost_with;
    let gpu_savings_per_month = gpu_savings_per_hour * 24.0 * 30.0;

    let storage_cost_per_month = cli.storage_cost_per_gib_month * cli.storage_gib as f64;
    let net_savings_per_month = gpu_savings_per_month - storage_cost_per_month;
    let roi_ratio = if storage_cost_per_month > 0.0 {
        net_savings_per_month / storage_cost_per_month
    } else {
        f64::INFINITY
    };

    // Break-even hit rate
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

    // Precise vs heuristic comparison (only if precise_hit_rate is specified)
    let precise_comparison = if cli.precise_hit_rate > 0.0 && cli.precise_hit_rate != cli.hit_rate {
        let heuristic_gpu_cost = gpu_cost_with;
        let heuristic_ttft_avg =
            cli.hit_rate * cli.ttft_with_cache_s + (1.0 - cli.hit_rate) * cli.ttft_without_cache_s;

        let precise_gpu_cost = compute_gpu_cost_per_hour(
            cli.precise_hit_rate,
            cli.ttft_without_cache_s,
            cli.ttft_with_cache_s,
            cli.gpu_cost_per_hour,
            cli.requests_per_hour,
        );
        let precise_ttft_avg = cli.precise_hit_rate * cli.ttft_with_cache_s
            + (1.0 - cli.precise_hit_rate) * cli.ttft_without_cache_s;

        let precise_gpu_savings_month = (gpu_cost_without - precise_gpu_cost) * 24.0 * 30.0;
        let heuristic_gpu_savings_month = gpu_savings_per_month;

        let additional_savings = precise_gpu_savings_month - heuristic_gpu_savings_month;
        let additional_net = additional_savings; // storage cost is same for both

        let hit_rate_delta = cli.precise_hit_rate - cli.hit_rate;

        let routing_upgrade_roi = if additional_net > 0.0 {
            format!(
                "EPP upgrade POSITIVE: +{:.0}% hit rate → +${:.2}/month additional savings",
                hit_rate_delta * 100.0,
                additional_net
            )
        } else {
            format!(
                "EPP upgrade NOT beneficial: hit rate delta {:.0}% yields no additional savings",
                hit_rate_delta * 100.0
            )
        };

        Some(PreciseComparison {
            heuristic: RoutingComparison {
                routing_mode: "consistent-hash".to_string(),
                hit_rate: cli.hit_rate,
                gpu_cost_per_hour: heuristic_gpu_cost,
                gpu_savings_per_month: heuristic_gpu_savings_month,
                ttft_avg_s: heuristic_ttft_avg,
            },
            precise: RoutingComparison {
                routing_mode: "epp-precise".to_string(),
                hit_rate: cli.precise_hit_rate,
                gpu_cost_per_hour: precise_gpu_cost,
                gpu_savings_per_month: precise_gpu_savings_month,
                ttft_avg_s: precise_ttft_avg,
            },
            hit_rate_delta,
            additional_gpu_savings_per_month: additional_savings,
            additional_net_savings_per_month: additional_net,
            routing_upgrade_roi,
        })
    } else {
        None
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
        precise_comparison,
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("=== KV Cache ROI Analysis ===");
        println!("Hit rate              : {:.1}%", result.hit_rate * 100.0);
        println!("Requests/hour         : {}", result.requests_per_hour);
        println!("Context length        : {} tokens", result.context_length);
        println!();
        println!(
            "TTFT without cache    : {:.1}s",
            result.ttft_without_cache_s
        );
        println!("TTFT with cache hit   : {:.1}s", result.ttft_with_cache_s);
        println!("TTFT reduction        : {:.1}%", result.ttft_reduction_pct);
        println!();
        println!(
            "GPU cost without cache: ${:.2}/h",
            result.gpu_cost_without_cache_per_hour
        );
        println!(
            "GPU cost with cache   : ${:.2}/h",
            result.gpu_cost_with_cache_per_hour
        );
        println!(
            "GPU savings           : ${:.2}/h → ${:.2}/month",
            result.gpu_savings_per_hour, result.gpu_savings_per_month
        );
        println!();
        println!(
            "Storage cost (L2/L3)  : ${:.2}/month ({} GiB @ ${}/GiB)",
            result.storage_cost_per_month, cli.storage_gib, cli.storage_cost_per_gib_month
        );
        println!(
            "Net savings           : ${:.2}/month",
            result.net_savings_per_month
        );
        println!("ROI ratio             : {:.1}x", result.roi_ratio);
        println!(
            "Break-even hit rate   : {:.1}%",
            result.break_even_hit_rate * 100.0
        );
        println!();
        println!("→ {}", result.recommendation);

        if let Some(ref cmp) = result.precise_comparison {
            println!();
            println!("=== Routing Mode Comparison ===");
            println!("                    Heuristic (consistent-hash)  Precise (EPP)");
            println!(
                "Hit rate           {:>20}  {:>20}",
                format!("{:.1}%", cmp.heuristic.hit_rate * 100.0),
                format!("{:.1}%", cmp.precise.hit_rate * 100.0)
            );
            println!(
                "Avg TTFT           {:>19.1}s  {:>19.1}s",
                cmp.heuristic.ttft_avg_s, cmp.precise.ttft_avg_s
            );
            println!(
                "GPU cost/h         {:>19.2}$  {:>19.2}$",
                cmp.heuristic.gpu_cost_per_hour, cmp.precise.gpu_cost_per_hour
            );
            println!(
                "GPU savings/month  {:>19.2}$  {:>19.2}$",
                cmp.heuristic.gpu_savings_per_month, cmp.precise.gpu_savings_per_month
            );
            println!();
            println!("Hit rate delta      : {:.1}%", cmp.hit_rate_delta * 100.0);
            println!(
                "Additional savings : ${:.2}/month",
                cmp.additional_net_savings_per_month
            );
            println!("→ {}", cmp.routing_upgrade_roi);
        }

        println!();
        println!(
            "Reference: vllm-kv-cache.md §9 (ROI = (GPU_saved - storage_cost) / storage_cost)"
        );
    }

    Ok(())
}
