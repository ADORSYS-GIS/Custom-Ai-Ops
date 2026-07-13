use anyhow::{anyhow, Result};
use clap::Parser;
use serde::Serialize;
use std::fmt;

const VRAM_UTILIZATION_FACTOR: f64 = 0.90;
const FIXED_OVERHEAD_GB: f64 = 1.0;
const PREFILL_UTILIZATION_FACTOR: f64 = 0.92;
const DECODE_UTILIZATION_FACTOR: f64 = 0.85;
const PREFILL_KV_CACHE_FACTOR: f64 = 0.3;
const DECODE_KV_CACHE_FACTOR: f64 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuantFormat {
    Fp32,
    Fp16,
    Bf16,
    Fp8,
    Int8,
    Int4,
    Q4KM,
}

impl fmt::Display for QuantFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QuantFormat::Fp32 => write!(f, "fp32"),
            QuantFormat::Fp16 => write!(f, "fp16"),
            QuantFormat::Bf16 => write!(f, "bf16"),
            QuantFormat::Fp8 => write!(f, "fp8"),
            QuantFormat::Int8 => write!(f, "int8"),
            QuantFormat::Int4 => write!(f, "int4"),
            QuantFormat::Q4KM => write!(f, "q4_km"),
        }
    }
}

impl QuantFormat {
    fn bytes_per_weight(&self) -> f64 {
        match self {
            QuantFormat::Fp32 => 4.0,
            QuantFormat::Fp16 => 2.0,
            QuantFormat::Bf16 => 2.0,
            QuantFormat::Fp8 => 1.0,
            QuantFormat::Int8 => 1.0,
            QuantFormat::Int4 => 0.5,
            QuantFormat::Q4KM => 0.55,
        }
    }

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "fp32" => Ok(QuantFormat::Fp32),
            "fp16" => Ok(QuantFormat::Fp16),
            "bf16" => Ok(QuantFormat::Bf16),
            "fp8" => Ok(QuantFormat::Fp8),
            "int8" => Ok(QuantFormat::Int8),
            "int4" => Ok(QuantFormat::Int4),
            "q4_km" | "q4km" | "q4-km" => Ok(QuantFormat::Q4KM),
            _ => Err(anyhow!("unsupported quantization format: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GpuArch {
    Ampere,
    Ada,
    Hopper,
    Blackwell,
    Other,
}

impl fmt::Display for GpuArch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GpuArch::Ampere => write!(f, "Ampere"),
            GpuArch::Ada => write!(f, "Ada Lovelace"),
            GpuArch::Hopper => write!(f, "Hopper"),
            GpuArch::Blackwell => write!(f, "Blackwell"),
            GpuArch::Other => write!(f, "Other"),
        }
    }
}

impl GpuArch {
    fn supports_fp8(&self) -> bool {
        matches!(self, GpuArch::Ada | GpuArch::Hopper | GpuArch::Blackwell)
    }

    fn from_gpu_name(name: &str) -> Self {
        let lower = name.to_lowercase();
        if lower.contains("a100")
            || lower.contains("a2000")
            || lower.contains("a4000")
            || lower.contains("a4500")
            || lower.contains("a5000")
            || lower.contains("a6000")
            || lower.contains("rtx 30")
            || lower.contains("rtx a")
            || lower.contains("a10")
            || lower.contains("a16")
            || lower.contains("a30")
            || lower.contains("a40")
        {
            GpuArch::Ampere
        } else if lower.contains("rtx 40") || lower.contains("l4") || lower.contains("l40") {
            GpuArch::Ada
        } else if lower.contains("h100") || lower.contains("h200") || lower.contains("hgx") {
            GpuArch::Hopper
        } else if lower.contains("b100") || lower.contains("b200") || lower.contains("b200") {
            GpuArch::Blackwell
        } else {
            GpuArch::Other
        }
    }
}

#[derive(Serialize)]
struct VramBudget {
    total_vram_gb: f64,
    usable_vram_gb: f64,
    model_size_gb: f64,
    fixed_overhead_gb: f64,
    kv_cache_budget_gb: f64,
    remaining_gb: f64,
    fits: bool,
    warnings: Vec<String>,
    quantization: String,
    gpu_arch: String,
}

#[derive(Serialize)]
struct DisaggregatedVramBudget {
    prefill: VramBudget,
    decode: VramBudget,
    total_gpu_count: u32,
    recommendation: String,
}

#[derive(Parser)]
#[command(
    name = "vram-budget-calc",
    version,
    about = "Calculates whether a model fits within GPU VRAM constraints",
    long_about = None
)]
struct Cli {
    #[arg(long, help = "Total VRAM of the GPU in GB", value_name = "GB")]
    total_vram: f64,

    #[arg(short, long, help = "Model size in GB", value_name = "GB")]
    model_size: f64,

    #[arg(
        short,
        long,
        help = "Quantization format (fp32, fp16, bf16, fp8, int8, int4, q4_km)"
    )]
    quant: String,

    #[arg(
        short = 'g',
        long,
        help = "GPU name for architecture detection (e.g. 'RTX A2000')"
    )]
    gpu: Option<String>,

    #[arg(
        long,
        help = "Batch size for KV cache calculation",
        default_value_t = 1
    )]
    batch: u32,

    #[arg(
        long,
        help = "Context length for KV cache calculation",
        default_value_t = 4096
    )]
    context: u32,

    #[arg(long, help = "Number of transformer layers", default_value_t = 0)]
    layers: u32,

    #[arg(long, help = "Number of attention heads", default_value_t = 0)]
    heads: u32,

    #[arg(
        long,
        help = "Output budget as JSON for pipeline integration",
        default_value_t = false
    )]
    json: bool,

    #[arg(
        long,
        help = "Enable disaggregated P/D mode (prefill/decode split)",
        default_value_t = false
    )]
    disaggregated: bool,

    #[arg(
        long,
        help = "GPU memory utilization for prefill pods (0.0-1.0)",
        default_value_t = PREFILL_UTILIZATION_FACTOR
    )]
    prefill_gpu_util: f64,

    #[arg(
        long,
        help = "GPU memory utilization for decode pods (0.0-1.0)",
        default_value_t = DECODE_UTILIZATION_FACTOR
    )]
    decode_gpu_util: f64,

    #[arg(long, help = "Number of GPUs for prefill pool", default_value_t = 1)]
    prefill_gpus: u32,

    #[arg(long, help = "Number of GPUs for decode pool", default_value_t = 1)]
    decode_gpus: u32,
}

fn calculate_kv_cache_gb(
    batch: u32,
    context: u32,
    layers: u32,
    heads: u32,
    bytes_per_elem: f64,
) -> f64 {
    let b = batch as f64;
    let s = context as f64;
    let l = layers as f64;
    let h = heads as f64;
    let byt = bytes_per_elem;
    2.0 * b * s * l * h * byt / (1024.0 * 1024.0 * 1024.0)
}

#[allow(clippy::too_many_arguments)]
fn compute_unified_budget(
    total_vram: f64,
    model_size: f64,
    quant: QuantFormat,
    gpu_arch: GpuArch,
    batch: u32,
    context: u32,
    layers: u32,
    heads: u32,
) -> VramBudget {
    let mut warnings = Vec::new();

    if quant == QuantFormat::Fp8 && !gpu_arch.supports_fp8() {
        warnings.push(format!(
            "FP8 quantization is not supported on {} GPUs.",
            gpu_arch
        ));
    }

    let usable_vram = total_vram * VRAM_UTILIZATION_FACTOR;
    let bytes_per_elem = quant.bytes_per_weight();
    let kv_cache = if layers > 0 && heads > 0 {
        calculate_kv_cache_gb(batch, context, layers, heads, bytes_per_elem)
    } else {
        0.0
    };

    let remaining = usable_vram - model_size - FIXED_OVERHEAD_GB - kv_cache;

    if remaining < 0.0 {
        warnings.push(format!(
            "Insufficient VRAM: need {:.2} GB but only {:.2} GB usable (shortfall {:.2} GB)",
            model_size + FIXED_OVERHEAD_GB + kv_cache,
            usable_vram,
            -remaining
        ));
    }

    VramBudget {
        total_vram_gb: total_vram,
        usable_vram_gb: usable_vram,
        model_size_gb: model_size,
        fixed_overhead_gb: FIXED_OVERHEAD_GB,
        kv_cache_budget_gb: kv_cache,
        remaining_gb: remaining,
        fits: remaining >= 0.0,
        warnings,
        quantization: quant.to_string(),
        gpu_arch: gpu_arch.to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn compute_disaggregated_budget(
    total_vram: f64,
    model_size: f64,
    quant: QuantFormat,
    gpu_arch: GpuArch,
    batch: u32,
    context: u32,
    layers: u32,
    heads: u32,
    prefill_gpu_util: f64,
    decode_gpu_util: f64,
    prefill_gpus: u32,
    decode_gpus: u32,
) -> DisaggregatedVramBudget {
    let bytes_per_elem = quant.bytes_per_weight();
    let base_kv = if layers > 0 && heads > 0 {
        calculate_kv_cache_gb(batch, context, layers, heads, bytes_per_elem)
    } else {
        0.0
    };

    // Prefill: high compute, low KV cache (only active prefill batches)
    let prefill_usable = total_vram * prefill_gpu_util;
    let prefill_kv = base_kv * PREFILL_KV_CACHE_FACTOR;
    let prefill_remaining = prefill_usable - model_size - FIXED_OVERHEAD_GB - prefill_kv;
    let mut prefill_warnings = Vec::new();
    if prefill_remaining < 0.0 {
        prefill_warnings.push(format!(
            "Prefill pool insufficient: need {:.2} GB, have {:.2} GB (shortfall {:.2} GB)",
            model_size + FIXED_OVERHEAD_GB + prefill_kv,
            prefill_usable,
            -prefill_remaining
        ));
    }

    let prefill = VramBudget {
        total_vram_gb: total_vram,
        usable_vram_gb: prefill_usable,
        model_size_gb: model_size,
        fixed_overhead_gb: FIXED_OVERHEAD_GB,
        kv_cache_budget_gb: prefill_kv,
        remaining_gb: prefill_remaining,
        fits: prefill_remaining >= 0.0,
        warnings: prefill_warnings,
        quantization: quant.to_string(),
        gpu_arch: gpu_arch.to_string(),
    };

    // Decode: lower compute, high KV cache (holding many concurrent sequences)
    let decode_usable = total_vram * decode_gpu_util;
    let decode_kv = base_kv * DECODE_KV_CACHE_FACTOR;
    let decode_remaining = decode_usable - model_size - FIXED_OVERHEAD_GB - decode_kv;
    let mut decode_warnings = Vec::new();
    if decode_remaining < 0.0 {
        decode_warnings.push(format!(
            "Decode pool insufficient: need {:.2} GB, have {:.2} GB (shortfall {:.2} GB)",
            model_size + FIXED_OVERHEAD_GB + decode_kv,
            decode_usable,
            -decode_remaining
        ));
    }

    let decode = VramBudget {
        total_vram_gb: total_vram,
        usable_vram_gb: decode_usable,
        model_size_gb: model_size,
        fixed_overhead_gb: FIXED_OVERHEAD_GB,
        kv_cache_budget_gb: decode_kv,
        remaining_gb: decode_remaining,
        fits: decode_remaining >= 0.0,
        warnings: decode_warnings,
        quantization: quant.to_string(),
        gpu_arch: gpu_arch.to_string(),
    };

    let total_gpu_count = prefill_gpus + decode_gpus;
    let recommendation = if prefill.fits && decode.fits {
        format!(
            "P/D disaggregation viable: prefill {}×{}GB + decode {}×{}GB = {} GPUs total",
            prefill_gpus, total_vram, decode_gpus, total_vram, total_gpu_count
        )
    } else {
        let mut issues = Vec::new();
        if !prefill.fits {
            issues.push("prefill pool OOM");
        }
        if !decode.fits {
            issues.push("decode pool OOM");
        }
        format!(
            "P/D disaggregation NOT viable: {} — increase GPU count or reduce model size",
            issues.join(", ")
        )
    };

    DisaggregatedVramBudget {
        prefill,
        decode,
        total_gpu_count,
        recommendation,
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.total_vram <= 0.0 {
        return Err(anyhow!("total VRAM must be positive"));
    }

    if cli.model_size < 0.0 {
        return Err(anyhow!("model size cannot be negative"));
    }

    let quant = QuantFormat::from_str(&cli.quant)?;
    let gpu_arch = cli
        .gpu
        .as_deref()
        .map(GpuArch::from_gpu_name)
        .unwrap_or(GpuArch::Other);

    if quant == QuantFormat::Fp8 && !gpu_arch.supports_fp8() {
        return Err(anyhow!(
            "FP8 quantization is not supported on {} GPUs. \
             RTX A2000 and other Ampere GPUs lack FP8 Tensor Core support. \
             Use fp16, bf16, int8, or int4 instead.",
            gpu_arch
        ));
    }

    if cli.disaggregated {
        if cli.prefill_gpu_util <= 0.0 || cli.prefill_gpu_util > 1.0 {
            return Err(anyhow!("prefill_gpu_util must be between 0.0 and 1.0"));
        }
        if cli.decode_gpu_util <= 0.0 || cli.decode_gpu_util > 1.0 {
            return Err(anyhow!("decode_gpu_util must be between 0.0 and 1.0"));
        }
        if cli.prefill_gpus == 0 {
            return Err(anyhow!("prefill_gpus must be at least 1"));
        }
        if cli.decode_gpus == 0 {
            return Err(anyhow!("decode_gpus must be at least 1"));
        }

        let budget = compute_disaggregated_budget(
            cli.total_vram,
            cli.model_size,
            quant,
            gpu_arch,
            cli.batch,
            cli.context,
            cli.layers,
            cli.heads,
            cli.prefill_gpu_util,
            cli.decode_gpu_util,
            cli.prefill_gpus,
            cli.decode_gpus,
        );

        if cli.json {
            println!("{}", serde_json::to_string_pretty(&budget)?);
        } else {
            println!("VRAM Budget Analysis (Disaggregated P/D)");
            println!("════════════════════════════════════════");
            println!();
            println!(
                "Prefill Pool ({} GPU(s), {:.0}% utilization):",
                cli.prefill_gpus,
                cli.prefill_gpu_util * 100.0
            );
            println!(
                "  Usable VRAM     : {:.2} GB",
                budget.prefill.usable_vram_gb
            );
            println!("  Model size      : {:.2} GB", budget.prefill.model_size_gb);
            println!(
                "  Fixed overhead  : {:.2} GB",
                budget.prefill.fixed_overhead_gb
            );
            println!(
                "  KV cache (30%)  : {:.2} GB",
                budget.prefill.kv_cache_budget_gb
            );
            println!("  Remaining       : {:.2} GB", budget.prefill.remaining_gb);
            println!(
                "  Fits            : {}",
                if budget.prefill.fits { "YES" } else { "NO" }
            );
            for w in &budget.prefill.warnings {
                println!("  WARNING: {}", w);
            }
            println!();
            println!(
                "Decode Pool ({} GPU(s), {:.0}% utilization):",
                cli.decode_gpus,
                cli.decode_gpu_util * 100.0
            );
            println!("  Usable VRAM     : {:.2} GB", budget.decode.usable_vram_gb);
            println!("  Model size      : {:.2} GB", budget.decode.model_size_gb);
            println!(
                "  Fixed overhead  : {:.2} GB",
                budget.decode.fixed_overhead_gb
            );
            println!(
                "  KV cache (150%) : {:.2} GB",
                budget.decode.kv_cache_budget_gb
            );
            println!("  Remaining       : {:.2} GB", budget.decode.remaining_gb);
            println!(
                "  Fits            : {}",
                if budget.decode.fits { "YES" } else { "NO" }
            );
            for w in &budget.decode.warnings {
                println!("  WARNING: {}", w);
            }
            println!();
            println!("Total GPUs        : {}", budget.total_gpu_count);
            println!("Recommendation    : {}", budget.recommendation);
        }

        if !budget.prefill.fits || !budget.decode.fits {
            std::process::exit(1);
        }
    } else {
        let budget = compute_unified_budget(
            cli.total_vram,
            cli.model_size,
            quant,
            gpu_arch,
            cli.batch,
            cli.context,
            cli.layers,
            cli.heads,
        );

        if cli.json {
            println!("{}", serde_json::to_string_pretty(&budget)?);
        } else {
            println!("VRAM Budget Analysis");
            println!("─────────────────────");
            println!("Total VRAM        : {:.2} GB", budget.total_vram_gb);
            println!("Usable VRAM (90%) : {:.2} GB", budget.usable_vram_gb);
            println!(
                "Model size ({:>4}) : {:.2} GB",
                budget.quantization, budget.model_size_gb
            );
            println!("Fixed overhead    : {:.2} GB", budget.fixed_overhead_gb);
            if budget.kv_cache_budget_gb > 0.0 {
                println!("KV cache budget   : {:.2} GB", budget.kv_cache_budget_gb);
            }
            println!("─────────────────────");
            println!("Remaining         : {:.2} GB", budget.remaining_gb);
            println!(
                "Fits on GPU       : {}",
                if budget.fits { "YES" } else { "NO" }
            );
            for w in &budget.warnings {
                println!("WARNING: {}", w);
            }
        }

        if !budget.fits {
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quant_format_bytes_per_weight() {
        assert_eq!(QuantFormat::Fp32.bytes_per_weight(), 4.0);
        assert_eq!(QuantFormat::Fp16.bytes_per_weight(), 2.0);
        assert_eq!(QuantFormat::Bf16.bytes_per_weight(), 2.0);
        assert_eq!(QuantFormat::Fp8.bytes_per_weight(), 1.0);
        assert_eq!(QuantFormat::Int8.bytes_per_weight(), 1.0);
        assert_eq!(QuantFormat::Int4.bytes_per_weight(), 0.5);
        assert_eq!(QuantFormat::Q4KM.bytes_per_weight(), 0.55);
    }

    #[test]
    fn test_quant_format_from_str() {
        assert_eq!(QuantFormat::from_str("fp32").unwrap(), QuantFormat::Fp32);
        assert_eq!(QuantFormat::from_str("FP16").unwrap(), QuantFormat::Fp16);
        assert_eq!(QuantFormat::from_str("bf16").unwrap(), QuantFormat::Bf16);
        assert_eq!(QuantFormat::from_str("fp8").unwrap(), QuantFormat::Fp8);
        assert_eq!(QuantFormat::from_str("int8").unwrap(), QuantFormat::Int8);
        assert_eq!(QuantFormat::from_str("int4").unwrap(), QuantFormat::Int4);
        assert_eq!(QuantFormat::from_str("q4_km").unwrap(), QuantFormat::Q4KM);
        assert_eq!(QuantFormat::from_str("q4km").unwrap(), QuantFormat::Q4KM);
        assert_eq!(QuantFormat::from_str("q4-km").unwrap(), QuantFormat::Q4KM);
        assert!(QuantFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_quant_format_display() {
        assert_eq!(QuantFormat::Fp32.to_string(), "fp32");
        assert_eq!(QuantFormat::Fp16.to_string(), "fp16");
        assert_eq!(QuantFormat::Bf16.to_string(), "bf16");
        assert_eq!(QuantFormat::Fp8.to_string(), "fp8");
        assert_eq!(QuantFormat::Int8.to_string(), "int8");
        assert_eq!(QuantFormat::Int4.to_string(), "int4");
        assert_eq!(QuantFormat::Q4KM.to_string(), "q4_km");
    }

    #[test]
    fn test_gpu_arch_supports_fp8() {
        assert!(!GpuArch::Ampere.supports_fp8());
        assert!(GpuArch::Ada.supports_fp8());
        assert!(GpuArch::Hopper.supports_fp8());
        assert!(GpuArch::Blackwell.supports_fp8());
        assert!(!GpuArch::Other.supports_fp8());
    }

    #[test]
    fn test_gpu_arch_from_gpu_name_ampere() {
        assert_eq!(GpuArch::from_gpu_name("RTX A2000"), GpuArch::Ampere);
        assert_eq!(GpuArch::from_gpu_name("A100"), GpuArch::Ampere);
        assert_eq!(GpuArch::from_gpu_name("RTX 3090"), GpuArch::Ampere);
        assert_eq!(GpuArch::from_gpu_name("A10"), GpuArch::Ampere);
        assert_eq!(GpuArch::from_gpu_name("A30"), GpuArch::Ampere);
    }

    #[test]
    fn test_gpu_arch_from_gpu_name_ada() {
        assert_eq!(GpuArch::from_gpu_name("RTX 4090"), GpuArch::Ada);
        assert_eq!(GpuArch::from_gpu_name("L4"), GpuArch::Ada);
        assert_eq!(GpuArch::from_gpu_name("L40"), GpuArch::Ada);
    }

    #[test]
    fn test_gpu_arch_from_gpu_name_hopper() {
        assert_eq!(GpuArch::from_gpu_name("H100"), GpuArch::Hopper);
        assert_eq!(GpuArch::from_gpu_name("H200"), GpuArch::Hopper);
        assert_eq!(GpuArch::from_gpu_name("HGX"), GpuArch::Hopper);
    }

    #[test]
    fn test_gpu_arch_from_gpu_name_blackwell() {
        assert_eq!(GpuArch::from_gpu_name("B200"), GpuArch::Blackwell);
        assert_eq!(GpuArch::from_gpu_name("B100"), GpuArch::Blackwell);
    }

    #[test]
    fn test_gpu_arch_from_gpu_name_other() {
        assert_eq!(GpuArch::from_gpu_name("Unknown GPU"), GpuArch::Other);
        assert_eq!(GpuArch::from_gpu_name("Radeon RX 7900"), GpuArch::Other);
    }

    #[test]
    fn test_calculate_kv_cache_gb_basic() {
        // batch=1, context=4096, layers=32, heads=32, fp16 (2 bytes)
        let kv = calculate_kv_cache_gb(1, 4096, 32, 32, 2.0);
        assert!((kv - 0.015625).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_kv_cache_gb_zero_batch() {
        let kv = calculate_kv_cache_gb(0, 4096, 32, 32, 2.0);
        assert_eq!(kv, 0.0);
    }

    #[test]
    fn test_calculate_kv_cache_gb_large_context() {
        let kv = calculate_kv_cache_gb(1, 32768, 32, 32, 2.0);
        assert!((kv - 0.125).abs() < 0.0001);
    }

    #[test]
    fn test_vram_budget_fits() {
        let kv = calculate_kv_cache_gb(1, 8192, 32, 32, 0.55);
        let usable = 8.0 * VRAM_UTILIZATION_FACTOR;
        let remaining = usable - 4.7 - FIXED_OVERHEAD_GB - kv;
        assert!(remaining > 0.0, "Should fit on 8GB GPU");
    }

    #[test]
    fn test_vram_budget_oom() {
        let usable = 4.0 * VRAM_UTILIZATION_FACTOR;
        let remaining = usable - 4.7 - FIXED_OVERHEAD_GB;
        assert!(remaining < 0.0, "Should not fit on 4GB GPU");
    }

    #[test]
    fn test_fp8_blocked_on_ampere() {
        assert!(!GpuArch::Ampere.supports_fp8());
    }

    #[test]
    fn test_fp8_supported_on_hopper() {
        assert!(GpuArch::Hopper.supports_fp8());
    }

    // --- Disaggregated mode tests ---

    #[test]
    fn test_disaggregated_prefill_fits() {
        let budget = compute_disaggregated_budget(
            80.0, // H100 80GB
            14.0, // 7B model in fp16
            QuantFormat::Fp16,
            GpuArch::Hopper,
            1,
            8192,
            32,
            32,
            PREFILL_UTILIZATION_FACTOR,
            DECODE_UTILIZATION_FACTOR,
            1,
            1,
        );
        assert!(budget.prefill.fits, "Prefill pool should fit on 80GB H100");
    }

    #[test]
    fn test_disaggregated_decode_fits() {
        let budget = compute_disaggregated_budget(
            80.0,
            14.0,
            QuantFormat::Fp16,
            GpuArch::Hopper,
            1,
            8192,
            32,
            32,
            PREFILL_UTILIZATION_FACTOR,
            DECODE_UTILIZATION_FACTOR,
            1,
            1,
        );
        assert!(budget.decode.fits, "Decode pool should fit on 80GB H100");
    }

    #[test]
    fn test_disaggregated_prefill_low_kv() {
        let budget = compute_disaggregated_budget(
            80.0,
            14.0,
            QuantFormat::Fp16,
            GpuArch::Hopper,
            1,
            8192,
            32,
            32,
            PREFILL_UTILIZATION_FACTOR,
            DECODE_UTILIZATION_FACTOR,
            1,
            1,
        );
        assert!(
            budget.prefill.kv_cache_budget_gb < budget.decode.kv_cache_budget_gb,
            "Prefill KV cache should be smaller than decode"
        );
    }

    #[test]
    fn test_disaggregated_decode_high_kv() {
        let budget = compute_disaggregated_budget(
            80.0,
            14.0,
            QuantFormat::Fp16,
            GpuArch::Hopper,
            1,
            8192,
            32,
            32,
            PREFILL_UTILIZATION_FACTOR,
            DECODE_UTILIZATION_FACTOR,
            1,
            1,
        );
        assert!(
            budget.decode.kv_cache_budget_gb > budget.prefill.kv_cache_budget_gb,
            "Decode KV cache should be larger than prefill"
        );
    }

    #[test]
    fn test_disaggregated_total_gpu_count() {
        let budget = compute_disaggregated_budget(
            80.0,
            14.0,
            QuantFormat::Fp16,
            GpuArch::Hopper,
            1,
            8192,
            32,
            32,
            0.92,
            0.85,
            2,
            3,
        );
        assert_eq!(budget.total_gpu_count, 5);
    }

    #[test]
    fn test_disaggregated_oom() {
        let budget = compute_disaggregated_budget(
            4.0,  // tiny GPU
            14.0, // 7B model
            QuantFormat::Fp16,
            GpuArch::Other,
            1,
            4096,
            32,
            32,
            0.90,
            0.80,
            1,
            1,
        );
        assert!(!budget.prefill.fits, "Prefill should not fit on 4GB");
        assert!(!budget.decode.fits, "Decode should not fit on 4GB");
    }

    #[test]
    fn test_disaggregated_recommendation_viable() {
        let budget = compute_disaggregated_budget(
            80.0,
            14.0,
            QuantFormat::Fp16,
            GpuArch::Hopper,
            1,
            8192,
            32,
            32,
            0.92,
            0.85,
            1,
            1,
        );
        assert!(
            budget.recommendation.contains("viable"),
            "Should be viable on H100"
        );
    }

    #[test]
    fn test_disaggregated_recommendation_not_viable() {
        let budget = compute_disaggregated_budget(
            4.0,
            14.0,
            QuantFormat::Fp16,
            GpuArch::Other,
            1,
            4096,
            32,
            32,
            0.90,
            0.80,
            1,
            1,
        );
        assert!(
            budget.recommendation.contains("NOT viable"),
            "Should not be viable on 4GB"
        );
    }

    #[test]
    fn test_unified_budget_computes_correctly() {
        let budget = compute_unified_budget(
            80.0,
            14.0,
            QuantFormat::Fp16,
            GpuArch::Hopper,
            1,
            8192,
            32,
            32,
        );
        assert!(budget.fits, "7B on H100 80GB should fit");
        assert!(
            budget.remaining_gb > 50.0,
            "Should have plenty of remaining VRAM"
        );
    }
}
