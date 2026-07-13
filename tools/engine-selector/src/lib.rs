use anyhow::{anyhow, Result};
use serde::Serialize;
use std::path::Path;

/// Supported model formats — all served by vLLM.
/// Safetensors is the native format; AWQ and GPTQ are quantisation
/// variants that vLLM loads natively without conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelFormat {
    Safetensors,
    Awq,
    Gptq,
}

/// The only supported inference engine in this platform.
/// vLLM provides PagedAttention, continuous batching, prefix caching,
/// and native AWQ/GPTQ quantisation support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Engine {
    Vllm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelFamily {
    Transformer,
    Moe,
    Ssm,
    Hybrid,
}

/// KV-cache routing strategy — determines how the gateway dispatches
/// requests to vLLM replicas for maximum cache reuse.
///
/// `ConsistentHash`: legacy heuristic — hash of first 512 bytes of body
/// deterministically maps to a replica.  No knowledge of actual cache
/// state; relies on probabilistic prefix reuse.
///
/// `EPP`: llm-d Endpoint Picker — inspects each request's prompt prefix,
/// queries the EPP scoring pipeline (Discover→Filter→Score→Select) to
/// pick the replica most likely to hold relevant KV-cache blocks.
/// Requires no RDMA; works with LMCache on TCP.
///
/// `EPPWithIndexer`: EPP + KV-Cache Indexer — the indexer maintains a
/// cluster-wide map of cache blocks consumed via vLLM KV events.  EPP
/// queries this map for exact, real-time routing decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RoutingMode {
    ConsistentHash,
    Epp,
    EppWithIndexer,
}

/// Serving topology — determines how vLLM pods are organised.
///
/// `Unified`: all replicas run both prefill and decode (standard vLLM).
///
/// `Disaggregated`: prefill and decode are split onto independently
/// scalable pod sets (P/D disaggregation).  KV-cache is transferred
/// from prefill to decode pods via NIXL (RDMA/NVLink/TCP fallback).
/// Requires `--kv-transfer-config` with `kv_producer`/`kv_consumer` roles.
///
/// `WideExpertParallel`: MoE-specific — experts are distributed across
/// pods via LeaderWorkerSet so each expert set fits in GPU memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServingMode {
    Unified,
    Disaggregated,
    WideExpertParallel,
}

#[derive(Debug, Serialize)]
pub struct EngineSelection {
    pub format: String,
    pub engine: String,
    pub chart: String,
    pub confidence: f64,
    pub rationale: String,
    pub family: String,
    pub cache_strategy: String,
    pub routing_mode: String,
    pub serving_mode: String,
}

pub fn detect_format(path: &str) -> Result<ModelFormat> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(anyhow!("model path does not exist: {}", path));
    }

    let is_dir = p.is_dir();
    let filename = p
        .file_name()
        .ok_or_else(|| anyhow!("cannot determine filename from path: {}", path))?
        .to_string_lossy()
        .to_lowercase();
    let extension = p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    if is_dir {
        let has_safetensors = walk_extensions(p, &["safetensors"]);
        let has_awq = walk_extensions(p, &["safetensors"]) && has_awq_config(p);
        let has_gptq = has_gptq_config(p);

        if has_awq {
            return Ok(ModelFormat::Awq);
        }
        if has_gptq {
            return Ok(ModelFormat::Gptq);
        }
        if has_safetensors {
            return Ok(ModelFormat::Safetensors);
        }
        return Err(anyhow!(
            "cannot detect model format in directory: {} — supported formats: Safetensors, AWQ, GPTQ",
            path
        ));
    }

    match extension.as_str() {
        "safetensors" => Ok(ModelFormat::Safetensors),
        "bin" => {
            if filename.contains("awq") {
                Ok(ModelFormat::Awq)
            } else if filename.contains("gptq") {
                Ok(ModelFormat::Gptq)
            } else {
                Err(anyhow!(
                    "ambiguous .bin file — provide --format explicitly (supported: safetensors, awq, gptq)"
                ))
            }
        }
        _ => Err(anyhow!(
            "unsupported model extension '{}' — supported formats: safetensors, awq, gptq. Provide --format explicitly",
            extension
        )),
    }
}

pub fn walk_extensions(dir: &Path, exts: &[&str]) -> bool {
    walkdir_ext(dir, exts, 0, 3)
}

fn walkdir_ext(dir: &Path, exts: &[&str], depth: usize, max_depth: usize) -> bool {
    if depth > max_depth {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if walkdir_ext(&path, exts, depth + 1, max_depth) {
                return true;
            }
        } else {
            let e = path
                .extension()
                .map(|e| e.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if exts.iter().any(|target| *target == e) {
                return true;
            }
        }
    }
    false
}

fn has_awq_config(dir: &Path) -> bool {
    let config_path = dir.join("config.json");
    let Ok(content) = std::fs::read_to_string(&config_path) else {
        return false;
    };
    content.contains("\"quant_method\"") && content.contains("\"awq\"")
}

fn has_gptq_config(dir: &Path) -> bool {
    let config_path = dir.join("config.json");
    let Ok(content) = std::fs::read_to_string(&config_path) else {
        return false;
    };
    content.contains("\"quant_method\"") && content.contains("\"gptq\"")
}

pub fn parse_format_override(s: &str) -> Result<ModelFormat> {
    match s.to_lowercase().as_str() {
        "safetensors" => Ok(ModelFormat::Safetensors),
        "awq" => Ok(ModelFormat::Awq),
        "gptq" => Ok(ModelFormat::Gptq),
        _ => Err(anyhow!(
            "unknown format override: {} — supported: safetensors, awq, gptq",
            s
        )),
    }
}

pub fn select_engine(fmt: ModelFormat) -> (Engine, f64, String, String) {
    match fmt {
        ModelFormat::Safetensors => (
            Engine::Vllm,
            0.96,
            "model-serving-engine".to_string(),
            "vLLM offers PagedAttention and continuous batching for maximum throughput on safetensors".to_string(),
        ),
        ModelFormat::Awq => (
            Engine::Vllm,
            0.94,
            "model-serving-engine".to_string(),
            "vLLM has native AWQ support, avoiding re-conversion from quantised format".to_string(),
        ),
        ModelFormat::Gptq => (
            Engine::Vllm,
            0.93,
            "model-serving-engine".to_string(),
            "vLLM supports GPTQ natively without format conversion".to_string(),
        ),
    }
}

/// Detect the model architecture family from config.json.
/// See docs/explain/bible-kv-cache.md §5 and kv-cache.md §5 for the
/// distinction between Transformer (paginable KV cache), MoE
/// (dense KV + expert cache), SSM/Mamba (fixed-size recurrent state,
/// NOT paginable), and Hybrid (mixed memory profiles).
pub fn detect_family(path: &str) -> Result<ModelFamily> {
    let p = Path::new(path);
    let config_path = if p.is_dir() {
        p.join("config.json")
    } else if p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .as_deref()
        == Some("json")
    {
        p.to_path_buf()
    } else {
        let dir = p.parent().unwrap_or(Path::new("."));
        dir.join("config.json")
    };

    let Ok(content) = std::fs::read_to_string(&config_path) else {
        // Cannot read config — assume Transformer (most common)
        return Ok(ModelFamily::Transformer);
    };

    let lower = content.to_lowercase();

    // SSM / Mamba: model_type contains "mamba" or "ssm"
    if lower.contains("\"mamba\"")
        || lower.contains("\"model_type\": \"mamba\"")
        || lower.contains("_ssm")
    {
        return Ok(ModelFamily::Ssm);
    }

    // Hybrid: Jamba, Zamba — model_type contains "hybrid" or "jamba"
    if lower.contains("\"jamba\"") || lower.contains("\"zamba\"") || lower.contains("\"hybrid\"") {
        return Ok(ModelFamily::Hybrid);
    }

    // MoE: architectures contains "MoE" or config has num_experts
    if lower.contains("moe")
        || lower.contains("mixture_of_experts")
        || lower.contains("\"num_experts\"")
    {
        return Ok(ModelFamily::Moe);
    }

    Ok(ModelFamily::Transformer)
}

/// Returns the recommended KV cache strategy for a model family.
/// SSM/Mamba models do NOT use a pagable KV cache — their recurrent
/// state is fixed-size and cannot be paged or evicted (bible-kv-cache.md §5.3).
pub fn cache_strategy_for(family: ModelFamily) -> &'static str {
    match family {
        ModelFamily::Transformer => "PagedAttention + APC + LMCache (hierarchical L0/L1/L2/L3)",
        ModelFamily::Moe => "PagedAttention for KV + LRU/LFU for expert weights + PiKV sharding if multi-GPU",
        ModelFamily::Ssm => "Fixed recurrent state — NO PagedAttention. Allocate contiguous per-layer state. Prefix caching via state checkpointing (not token-bloc hash).",
        ModelFamily::Hybrid => "Asymmetric paging: PagedAttention for attention layers, contiguous allocation for SSM layers.",
    }
}

/// Recommend the KV-cache routing mode based on model family and
/// whether llm-d is enabled.
///
/// - SSM/Mamba models have fixed recurrent state — no paginable KV
///   cache, so consistent-hash is sufficient (no benefit from EPP).
/// - Transformer and Hybrid models benefit from EPP when llm-d is
///   enabled, because their paginable KV cache can be precisely
///   located on a specific replica.
/// - MoE models benefit from EPP + KV-Cache Indexer for expert-level
///   routing, especially in Wide Expert Parallel setups.
pub fn routing_mode_for(family: ModelFamily, llm_d_enabled: bool) -> RoutingMode {
    if !llm_d_enabled {
        return RoutingMode::ConsistentHash;
    }
    match family {
        ModelFamily::Transformer => RoutingMode::Epp,
        ModelFamily::Hybrid => RoutingMode::Epp,
        ModelFamily::Moe => RoutingMode::EppWithIndexer,
        ModelFamily::Ssm => RoutingMode::ConsistentHash,
    }
}

/// Detect the recommended serving topology from config.json and
/// model family.
///
/// - Unified (default): prefill + decode on same replicas.
/// - Disaggregated: split P/D when model is large (>40B params) and
///   config has `"disaggregated": true` or `"pd_split": true`.
///   Only Transformer and Hybrid families support P/D (SSM has no
///   paginable KV cache to transfer).
/// - WideExpertParallel: MoE models with many experts that exceed
///   single-GPU memory, when config has `"expert_parallel": "wide"`
///   or `"num_experts_per_gpu"` set.
pub fn detect_serving_mode(path: &str, family: ModelFamily) -> Result<ServingMode> {
    let p = Path::new(path);
    let config_path = if p.is_dir() {
        p.join("config.json")
    } else if p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .as_deref()
        == Some("json")
    {
        p.to_path_buf()
    } else {
        let dir = p.parent().unwrap_or(Path::new("."));
        dir.join("config.json")
    };

    let content = std::fs::read_to_string(&config_path).unwrap_or_default();
    let lower = content.to_lowercase();

    // Wide Expert Parallel for MoE
    if family == ModelFamily::Moe {
        if lower.contains("\"expert_parallel\"") && lower.contains("\"wide\"") {
            return Ok(ServingMode::WideExpertParallel);
        }
        if lower.contains("\"num_experts_per_gpu\"") {
            return Ok(ServingMode::WideExpertParallel);
        }
    }

    // P/D disaggregation — only for families with paginable KV cache
    if family == ModelFamily::Transformer || family == ModelFamily::Hybrid {
        if lower.contains("\"disaggregated\"") || lower.contains("\"pd_split\"") {
            return Ok(ServingMode::Disaggregated);
        }
        // Auto-detect from model size: if num_params or hidden_size suggests >40B
        if lower.contains("\"num_parameters\"") {
            if let Some(start) = lower.find("\"num_parameters\"") {
                let rest = &content[start..];
                if let Some(num_start) = rest.find(':') {
                    let num_part = &rest[num_start + 1..];
                    let num_str: String = num_part
                        .chars()
                        .skip_while(|c| c.is_whitespace() || *c == '"')
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if let Ok(n) = num_str.parse::<u64>() {
                        if n > 40_000_000_000 {
                            return Ok(ServingMode::Disaggregated);
                        }
                    }
                }
            }
        }
    }

    Ok(ServingMode::Unified)
}

/// Recommend whether P/D disaggregation is beneficial for a model.
/// Disaggregation shines when:
/// - The model is large (prefill is compute-bound, decode is memory-bound)
/// - The workload has high request concurrency (amortises KV transfer)
/// - The KV cache is paginable (Transformer/Hybrid, NOT SSM)
pub fn should_disaggregate(family: ModelFamily, model_size_gb: f64, concurrency: u32) -> bool {
    if family == ModelFamily::Ssm {
        return false;
    }
    model_size_gb > 14.0 && concurrency > 8
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_parse_format_override_safetensors() {
        assert_eq!(
            parse_format_override("safetensors").unwrap(),
            ModelFormat::Safetensors
        );
    }

    #[test]
    fn test_parse_format_override_awq() {
        assert_eq!(parse_format_override("awq").unwrap(), ModelFormat::Awq);
    }

    #[test]
    fn test_parse_format_override_gptq() {
        assert_eq!(parse_format_override("gptq").unwrap(), ModelFormat::Gptq);
    }

    #[test]
    fn test_parse_format_override_unknown() {
        assert!(parse_format_override("unknown").is_err());
        assert!(parse_format_override("onnx").is_err());
        assert!(parse_format_override("gguf").is_err());
        assert!(parse_format_override("tensorrt").is_err());
        assert!(parse_format_override("pytorch").is_err());
    }

    #[test]
    fn test_select_engine_safetensors() {
        let (engine, confidence, chart, _) = select_engine(ModelFormat::Safetensors);
        assert_eq!(engine, Engine::Vllm);
        assert_eq!(chart, "model-serving-engine");
        assert!(confidence > 0.9);
    }

    #[test]
    fn test_select_engine_awq() {
        let (engine, confidence, chart, _) = select_engine(ModelFormat::Awq);
        assert_eq!(engine, Engine::Vllm);
        assert_eq!(chart, "model-serving-engine");
        assert!(confidence > 0.9);
    }

    #[test]
    fn test_select_engine_gptq() {
        let (engine, confidence, chart, _) = select_engine(ModelFormat::Gptq);
        assert_eq!(engine, Engine::Vllm);
        assert_eq!(chart, "model-serving-engine");
        assert!(confidence > 0.9);
    }

    #[test]
    fn test_detect_family_ssm() {
        let dir = create_temp_dir();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"model_type": "mamba", "architectures": ["MambaForCausalLM"]}"#,
        )
        .unwrap();
        let family = detect_family(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(family, ModelFamily::Ssm);
    }

    #[test]
    fn test_detect_family_hybrid() {
        let dir = create_temp_dir();
        let config_path = dir.path().join("config.json");
        fs::write(&config_path, r#"{"model_type": "jamba"}"#).unwrap();
        let family = detect_family(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(family, ModelFamily::Hybrid);
    }

    #[test]
    fn test_detect_family_moe() {
        let dir = create_temp_dir();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"model_type": "llama", "num_experts": 8, "architectures": ["MixtralForCausalLM"]}"#,
        )
        .unwrap();
        let family = detect_family(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(family, ModelFamily::Moe);
    }

    #[test]
    fn test_detect_family_transformer() {
        let dir = create_temp_dir();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"model_type": "llama", "architectures": ["LlamaForCausalLM"]}"#,
        )
        .unwrap();
        let family = detect_family(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(family, ModelFamily::Transformer);
    }

    #[test]
    fn test_cache_strategy_ssm_no_paging() {
        let strategy = cache_strategy_for(ModelFamily::Ssm);
        assert!(
            strategy.contains("NO PagedAttention"),
            "SSM should explicitly say no PagedAttention"
        );
    }

    #[test]
    fn test_cache_strategy_transformer_has_paged() {
        let strategy = cache_strategy_for(ModelFamily::Transformer);
        assert!(strategy.contains("PagedAttention"));
    }

    #[test]
    fn test_cache_strategy_moe_has_expert() {
        let strategy = cache_strategy_for(ModelFamily::Moe);
        assert!(strategy.contains("expert"));
    }

    #[test]
    fn test_cache_strategy_hybrid_asymmetric() {
        let strategy = cache_strategy_for(ModelFamily::Hybrid);
        assert!(strategy.contains("Asymmetric"));
    }

    #[test]
    fn test_detect_format_safetensors_file() {
        let dir = create_temp_dir();
        let safetensors_path = dir.path().join("model.safetensors");
        fs::write(&safetensors_path, "fake safetensors data").unwrap();
        let result = detect_format(safetensors_path.to_str().unwrap()).unwrap();
        assert_eq!(result, ModelFormat::Safetensors);
    }

    #[test]
    fn test_detect_format_safetensors_dir() {
        let dir = create_temp_dir();
        let safetensors_path = dir.path().join("model.safetensors");
        fs::write(&safetensors_path, "fake").unwrap();
        let model_dir = dir.path().to_path_buf();
        let result = detect_format(model_dir.to_str().unwrap()).unwrap();
        assert_eq!(result, ModelFormat::Safetensors);
    }

    #[test]
    fn test_detect_format_awq_dir() {
        let dir = create_temp_dir();
        let safetensors_path = dir.path().join("model.safetensors");
        fs::write(&safetensors_path, "fake").unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"quant_method": "awq", "model_type": "llama"}"#,
        )
        .unwrap();
        let result = detect_format(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(result, ModelFormat::Awq);
    }

    #[test]
    fn test_detect_format_gptq_dir() {
        let dir = create_temp_dir();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"quant_method": "gptq", "model_type": "llama"}"#,
        )
        .unwrap();
        let result = detect_format(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(result, ModelFormat::Gptq);
    }

    #[test]
    fn test_detect_format_nonexistent_path() {
        assert!(detect_format("/nonexistent/path/model.safetensors").is_err());
    }

    #[test]
    fn test_detect_format_ambiguous_bin_file() {
        let dir = create_temp_dir();
        let bin_path = dir.path().join("model.bin");
        fs::write(&bin_path, "fake").unwrap();
        let result = detect_format(bin_path.to_str().unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_format_awq_bin_file() {
        let dir = create_temp_dir();
        let bin_path = dir.path().join("model-awq.bin");
        fs::write(&bin_path, "fake").unwrap();
        let result = detect_format(bin_path.to_str().unwrap()).unwrap();
        assert_eq!(result, ModelFormat::Awq);
    }

    #[test]
    fn test_detect_format_gptq_bin_file() {
        let dir = create_temp_dir();
        let bin_path = dir.path().join("model-gptq.bin");
        fs::write(&bin_path, "fake").unwrap();
        let result = detect_format(bin_path.to_str().unwrap()).unwrap();
        assert_eq!(result, ModelFormat::Gptq);
    }

    #[test]
    fn test_all_format_to_chart_mappings() {
        let formats = vec![
            (
                ModelFormat::Safetensors,
                Engine::Vllm,
                "model-serving-engine",
            ),
            (ModelFormat::Awq, Engine::Vllm, "model-serving-engine"),
            (ModelFormat::Gptq, Engine::Vllm, "model-serving-engine"),
        ];
        for (fmt, expected_engine, expected_chart) in formats {
            let (engine, _, chart, _) = select_engine(fmt);
            assert_eq!(engine, expected_engine, "Engine mismatch for {:?}", fmt);
            assert_eq!(chart, expected_chart, "Chart mismatch for {:?}", fmt);
        }
    }

    // --- Routing mode tests ---

    #[test]
    fn test_routing_mode_consistent_hash_without_llm_d() {
        assert_eq!(
            routing_mode_for(ModelFamily::Transformer, false),
            RoutingMode::ConsistentHash
        );
        assert_eq!(
            routing_mode_for(ModelFamily::Moe, false),
            RoutingMode::ConsistentHash
        );
        assert_eq!(
            routing_mode_for(ModelFamily::Hybrid, false),
            RoutingMode::ConsistentHash
        );
    }

    #[test]
    fn test_routing_mode_epp_for_transformer() {
        assert_eq!(
            routing_mode_for(ModelFamily::Transformer, true),
            RoutingMode::Epp
        );
    }

    #[test]
    fn test_routing_mode_epp_for_hybrid() {
        assert_eq!(
            routing_mode_for(ModelFamily::Hybrid, true),
            RoutingMode::Epp
        );
    }

    #[test]
    fn test_routing_mode_epp_with_indexer_for_moe() {
        assert_eq!(
            routing_mode_for(ModelFamily::Moe, true),
            RoutingMode::EppWithIndexer
        );
    }

    #[test]
    fn test_routing_mode_consistent_hash_for_ssm_even_with_llm_d() {
        assert_eq!(
            routing_mode_for(ModelFamily::Ssm, true),
            RoutingMode::ConsistentHash
        );
    }

    // --- Serving mode detection tests ---

    #[test]
    fn test_detect_serving_mode_unified_for_transformer() {
        let dir = create_temp_dir();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"model_type": "llama", "architectures": ["LlamaForCausalLM"]}"#,
        )
        .unwrap();
        let mode =
            detect_serving_mode(dir.path().to_str().unwrap(), ModelFamily::Transformer).unwrap();
        assert_eq!(mode, ServingMode::Unified);
    }

    #[test]
    fn test_detect_serving_mode_disaggregated_by_flag() {
        let dir = create_temp_dir();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"model_type": "llama", "disaggregated": true}"#,
        )
        .unwrap();
        let mode =
            detect_serving_mode(dir.path().to_str().unwrap(), ModelFamily::Transformer).unwrap();
        assert_eq!(mode, ServingMode::Disaggregated);
    }

    #[test]
    fn test_detect_serving_mode_disaggregated_by_pd_split() {
        let dir = create_temp_dir();
        let config_path = dir.path().join("config.json");
        fs::write(&config_path, r#"{"model_type": "llama", "pd_split": true}"#).unwrap();
        let mode =
            detect_serving_mode(dir.path().to_str().unwrap(), ModelFamily::Transformer).unwrap();
        assert_eq!(mode, ServingMode::Disaggregated);
    }

    #[test]
    fn test_detect_serving_mode_disaggregated_by_large_model() {
        let dir = create_temp_dir();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"model_type": "llama", "num_parameters": 70000000000}"#,
        )
        .unwrap();
        let mode =
            detect_serving_mode(dir.path().to_str().unwrap(), ModelFamily::Transformer).unwrap();
        assert_eq!(mode, ServingMode::Disaggregated);
    }

    #[test]
    fn test_detect_serving_mode_wide_expert_parallel() {
        let dir = create_temp_dir();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"model_type": "llama", "num_experts": 64, "expert_parallel": "wide"}"#,
        )
        .unwrap();
        let mode = detect_serving_mode(dir.path().to_str().unwrap(), ModelFamily::Moe).unwrap();
        assert_eq!(mode, ServingMode::WideExpertParallel);
    }

    #[test]
    fn test_detect_serving_mode_ssm_never_disaggregated() {
        let dir = create_temp_dir();
        let config_path = dir.path().join("config.json");
        fs::write(
            &config_path,
            r#"{"model_type": "mamba", "disaggregated": true}"#,
        )
        .unwrap();
        let mode = detect_serving_mode(dir.path().to_str().unwrap(), ModelFamily::Ssm).unwrap();
        assert_eq!(mode, ServingMode::Unified);
    }

    // --- Disaggregation recommendation tests ---

    #[test]
    fn test_should_disaggregate_ssm_returns_false() {
        assert!(!should_disaggregate(ModelFamily::Ssm, 100.0, 100));
    }

    #[test]
    fn test_should_disaggregate_small_model_returns_false() {
        assert!(!should_disaggregate(ModelFamily::Transformer, 7.0, 100));
    }

    #[test]
    fn test_should_disaggregate_low_concurrency_returns_false() {
        assert!(!should_disaggregate(ModelFamily::Transformer, 70.0, 4));
    }

    #[test]
    fn test_should_disaggregate_large_model_high_concurrency() {
        assert!(should_disaggregate(ModelFamily::Transformer, 70.0, 16));
    }

    #[test]
    fn test_should_disaggregate_hybrid_model() {
        assert!(should_disaggregate(ModelFamily::Hybrid, 40.0, 32));
    }
}
