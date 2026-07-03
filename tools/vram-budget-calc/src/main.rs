use anyhow::{anyhow, Result};
use clap::Parser;
use serde::Serialize;
use std::fmt;

const VRAM_UTILIZATION_FACTOR: f64 = 0.90;
const FIXED_OVERHEAD_GB: f64 = 1.0;

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
        if lower.contains("a100") || lower.contains("a2000") || lower.contains("a4000")
            || lower.contains("a4500") || lower.contains("a5000") || lower.contains("a6000")
            || lower.contains("rtx 30") || lower.contains("rtx a")
            || lower.contains("a10") || lower.contains("a16") || lower.contains("a30")
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

#[derive(Parser)]
#[command(
    name = "vram-budget-calc",
    version,
    about = "Calculates whether a model fits within GPU VRAM constraints",
    long_about = None
)]
struct Cli {
    #[arg(
        long,
        help = "Total VRAM of the GPU in GB",
        value_name = "GB"
    )]
    total_vram: f64,

    #[arg(short, long, help = "Model size in GB", value_name = "GB")]
    model_size: f64,

    #[arg(short, long, help = "Quantization format (fp32, fp16, bf16, fp8, int8, int4, q4_km)")]
    quant: String,

    #[arg(short = 'g', long, help = "GPU name for architecture detection (e.g. 'RTX A2000')")]
    gpu: Option<String>,

    #[arg(long, help = "Batch size for KV cache calculation", default_value_t = 1)]
    batch: u32,

    #[arg(long, help = "Context length for KV cache calculation", default_value_t = 4096)]
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
}

fn calculate_kv_cache_gb(batch: u32, context: u32, layers: u32, heads: u32, bytes_per_elem: f64) -> f64 {
    let b = batch as f64;
    let s = context as f64;
    let l = layers as f64;
    let h = heads as f64;
    let byt = bytes_per_elem;
    2.0 * b * s * l * h * byt / (1024.0 * 1024.0 * 1024.0)
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
        .map(|name| GpuArch::from_gpu_name(name))
        .unwrap_or(GpuArch::Other);

    let mut warnings = Vec::new();

    if quant == QuantFormat::Fp8 && !gpu_arch.supports_fp8() {
        return Err(anyhow!(
            "FP8 quantization is not supported on {} GPUs. \
             RTX A2000 and other Ampere GPUs lack FP8 Tensor Core support. \
             Use fp16, bf16, int8, or int4 instead.",
            gpu_arch
        ));
    }

    let usable_vram = cli.total_vram * VRAM_UTILIZATION_FACTOR;
    let bytes_per_elem = quant.bytes_per_weight();
    let kv_cache = if cli.layers > 0 && cli.heads > 0 {
        calculate_kv_cache_gb(cli.batch, cli.context, cli.layers, cli.heads, bytes_per_elem)
    } else {
        0.0
    };

    let remaining = usable_vram - cli.model_size - FIXED_OVERHEAD_GB - kv_cache;

    if remaining < 0.0 {
        warnings.push(format!(
            "Insufficient VRAM: need {:.2} GB but only {:.2} GB usable ( shortfall {:.2} GB )",
            cli.model_size + FIXED_OVERHEAD_GB + kv_cache,
            usable_vram,
            -remaining
        ));
    }

    let budget = VramBudget {
        total_vram_gb: cli.total_vram,
        usable_vram_gb: usable_vram,
        model_size_gb: cli.model_size,
        fixed_overhead_gb: FIXED_OVERHEAD_GB,
        kv_cache_budget_gb: kv_cache,
        remaining_gb: remaining,
        fits: remaining >= 0.0,
        warnings,
        quantization: quant.to_string(),
        gpu_arch: gpu_arch.to_string(),
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&budget)?);
    } else {
        println!("VRAM Budget Analysis");
        println!("─────────────────────");
        println!("Total VRAM        : {:.2} GB", budget.total_vram_gb);
        println!("Usable VRAM (90%) : {:.2} GB", budget.usable_vram_gb);
        println!("Model size ({:>4}) : {:.2} GB", budget.quantization, budget.model_size_gb);
        println!("Fixed overhead    : {:.2} GB", budget.fixed_overhead_gb);
        if budget.kv_cache_budget_gb > 0.0 {
            println!("KV cache budget   : {:.2} GB", budget.kv_cache_budget_gb);
        }
        println!("─────────────────────");
        println!("Remaining         : {:.2} GB", budget.remaining_gb);
        println!("Fits on GPU       : {}", if budget.fits { "YES" } else { "NO" });
        for w in &budget.warnings {
            println!("WARNING: {}", w);
        }
    }

    if !budget.fits {
        std::process::exit(1);
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
        // 2 * 1 * 4096 * 32 * 32 * 2 / (1024^3) = 2 * 4096 * 32 * 32 * 2 / 1073741824
        // = 2 * 8388608 / 1073741824 = 16777216 / 1073741824 ≈ 0.015625
        assert!((kv - 0.015625).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_kv_cache_gb_zero_batch() {
        let kv = calculate_kv_cache_gb(0, 4096, 32, 32, 2.0);
        assert_eq!(kv, 0.0);
    }

    #[test]
    fn test_calculate_kv_cache_gb_large_context() {
        // batch=1, context=32768, layers=32, heads=32, bf16 (2 bytes)
        let kv = calculate_kv_cache_gb(1, 32768, 32, 32, 2.0);
        // 2 * 1 * 32768 * 32 * 32 * 2 / (1024^3)
        // = 2 * 32768 * 1024 * 2 / 1073741824
        // = 134217728 / 1073741824 = 0.125
        assert!((kv - 0.125).abs() < 0.0001);
    }

    #[test]
    fn test_vram_budget_fits() {
        // 8GB VRAM, 4.7GB model, q4_km, RTX A2000
        // usable = 8 * 0.90 = 7.2
        // remaining = 7.2 - 4.7 - 1.0 - kv_cache
        // kv_cache with layers=32, heads=32, context=8192, q4_km (0.55 bytes)
        // = 2 * 1 * 8192 * 32 * 32 * 0.55 / 1024^3 ≈ 0.00858
        // remaining ≈ 7.2 - 4.7 - 1.0 - 0.00858 ≈ 1.49
        let kv = calculate_kv_cache_gb(1, 8192, 32, 32, 0.55);
        let usable = 8.0 * VRAM_UTILIZATION_FACTOR;
        let remaining = usable - 4.7 - FIXED_OVERHEAD_GB - kv;
        assert!(remaining > 0.0, "Should fit on 8GB GPU");
    }

    #[test]
    fn test_vram_budget_oom() {
        // 4GB VRAM, 4.7GB model — should not fit
        let usable = 4.0 * VRAM_UTILIZATION_FACTOR;
        let remaining = usable - 4.7 - FIXED_OVERHEAD_GB;
        assert!(remaining < 0.0, "Should not fit on 4GB GPU");
    }

    #[test]
    fn test_fp8_blocked_on_ampere() {
        // FP8 on Ampere should not be supported
        assert!(!GpuArch::Ampere.supports_fp8());
    }

    #[test]
    fn test_fp8_supported_on_hopper() {
        assert!(GpuArch::Hopper.supports_fp8());
    }
}