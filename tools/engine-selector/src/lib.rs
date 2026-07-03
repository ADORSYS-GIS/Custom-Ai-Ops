use anyhow::{anyhow, Result};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelFormat {
    Onnx,
    Safetensors,
    Awq,
    Gptq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Engine {
    Vllm,
    OnnxRuntimeGenai,
}

#[derive(Debug, Serialize)]
pub struct EngineSelection {
    pub format: String,
    pub engine: String,
    pub chart: String,
    pub confidence: f64,
    pub rationale: String,
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
        let has_onnx = walk_extensions(p, &["onnx", "onnx_data"]);
        let has_safetensors = walk_extensions(p, &["safetensors"]);
        let has_awq = walk_extensions(p, &["safetensors"]) && has_awq_config(p);
        let has_gptq = has_gptq_config(p);

        if has_awq {
            return Ok(ModelFormat::Awq);
        }
        if has_gptq {
            return Ok(ModelFormat::Gptq);
        }
        if has_onnx {
            return Ok(ModelFormat::Onnx);
        }
        if has_safetensors {
            return Ok(ModelFormat::Safetensors);
        }
        return Err(anyhow!(
            "cannot detect model format in directory: {} — supported formats: ONNX, Safetensors, AWQ, GPTQ",
            path
        ));
    }

    match extension.as_str() {
        "onnx" | "onnx_data" => Ok(ModelFormat::Onnx),
        "safetensors" => Ok(ModelFormat::Safetensors),
        "bin" => {
            if filename.contains("awq") {
                Ok(ModelFormat::Awq)
            } else if filename.contains("gptq") {
                Ok(ModelFormat::Gptq)
            } else {
                Err(anyhow!(
                    "ambiguous .bin file — provide --format explicitly (supported: onnx, safetensors, awq, gptq)"
                ))
            }
        }
        _ => Err(anyhow!(
            "unsupported model extension '{}' — supported formats: onnx, safetensors, awq, gptq. Provide --format explicitly",
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
        "onnx" => Ok(ModelFormat::Onnx),
        "safetensors" => Ok(ModelFormat::Safetensors),
        "awq" => Ok(ModelFormat::Awq),
        "gptq" => Ok(ModelFormat::Gptq),
        _ => Err(anyhow!(
            "unknown format override: {} — supported: onnx, safetensors, awq, gptq",
            s
        )),
    }
}

pub fn select_engine(fmt: ModelFormat) -> (Engine, f64, String, String) {
    match fmt {
        ModelFormat::Onnx => (
            Engine::OnnxRuntimeGenai,
            0.95,
            "model-serving-onnx-rust".to_string(),
            "ONNX Runtime GenAI provides native ONNX execution with Rust FFI integration".to_string(),
        ),
        ModelFormat::Safetensors => (
            Engine::Vllm,
            0.96,
            "model-serving-vllm".to_string(),
            "vLLM offers PagedAttention and continuous batching for maximum throughput on safetensors".to_string(),
        ),
        ModelFormat::Awq => (
            Engine::Vllm,
            0.94,
            "model-serving-vllm".to_string(),
            "vLLM has native AWQ support, avoiding re-conversion from quantised format".to_string(),
        ),
        ModelFormat::Gptq => (
            Engine::Vllm,
            0.93,
            "model-serving-vllm".to_string(),
            "vLLM supports GPTQ natively without format conversion".to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn test_parse_format_override_onnx() {
        assert_eq!(parse_format_override("onnx").unwrap(), ModelFormat::Onnx);
    }

    #[test]
    fn test_parse_format_override_safetensors() {
        assert_eq!(parse_format_override("safetensors").unwrap(), ModelFormat::Safetensors);
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
        assert!(parse_format_override("gguf").is_err());
        assert!(parse_format_override("tensorrt").is_err());
        assert!(parse_format_override("pytorch").is_err());
    }

    #[test]
    fn test_select_engine_onnx() {
        let (engine, confidence, chart, _) = select_engine(ModelFormat::Onnx);
        assert_eq!(engine, Engine::OnnxRuntimeGenai);
        assert_eq!(chart, "model-serving-onnx-rust");
        assert!(confidence > 0.9);
    }

    #[test]
    fn test_select_engine_safetensors() {
        let (engine, confidence, chart, _) = select_engine(ModelFormat::Safetensors);
        assert_eq!(engine, Engine::Vllm);
        assert_eq!(chart, "model-serving-vllm");
        assert!(confidence > 0.9);
    }

    #[test]
    fn test_select_engine_awq() {
        let (engine, confidence, chart, _) = select_engine(ModelFormat::Awq);
        assert_eq!(engine, Engine::Vllm);
        assert_eq!(chart, "model-serving-vllm");
        assert!(confidence > 0.9);
    }

    #[test]
    fn test_select_engine_gptq() {
        let (engine, confidence, chart, _) = select_engine(ModelFormat::Gptq);
        assert_eq!(engine, Engine::Vllm);
        assert_eq!(chart, "model-serving-vllm");
        assert!(confidence > 0.9);
    }

    #[test]
    fn test_detect_format_onnx_file() {
        let dir = create_temp_dir();
        let onnx_path = dir.path().join("model.onnx");
        fs::write(&onnx_path, "fake onnx data").unwrap();
        let result = detect_format(onnx_path.to_str().unwrap()).unwrap();
        assert_eq!(result, ModelFormat::Onnx);
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
    fn test_detect_format_onnx_dir() {
        let dir = create_temp_dir();
        let subdir = dir.path().join("model.onnx");
        fs::write(&subdir, "fake").unwrap();
        let model_dir = dir.path().to_path_buf();
        let result = detect_format(model_dir.to_str().unwrap()).unwrap();
        assert_eq!(result, ModelFormat::Onnx);
    }

    #[test]
    fn test_detect_format_awq_dir() {
        let dir = create_temp_dir();
        let safetensors_path = dir.path().join("model.safetensors");
        fs::write(&safetensors_path, "fake").unwrap();
        let config_path = dir.path().join("config.json");
        fs::write(&config_path, r#"{"quant_method": "awq", "model_type": "llama"}"#).unwrap();
        let result = detect_format(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(result, ModelFormat::Awq);
    }

    #[test]
    fn test_detect_format_gptq_dir() {
        let dir = create_temp_dir();
        let config_path = dir.path().join("config.json");
        fs::write(&config_path, r#"{"quant_method": "gptq", "model_type": "llama"}"#).unwrap();
        let result = detect_format(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(result, ModelFormat::Gptq);
    }

    #[test]
    fn test_detect_format_nonexistent_path() {
        assert!(detect_format("/nonexistent/path/model.onnx").is_err());
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
            (ModelFormat::Onnx, Engine::OnnxRuntimeGenai, "model-serving-onnx-rust"),
            (ModelFormat::Safetensors, Engine::Vllm, "model-serving-vllm"),
            (ModelFormat::Awq, Engine::Vllm, "model-serving-vllm"),
            (ModelFormat::Gptq, Engine::Vllm, "model-serving-vllm"),
        ];
        for (fmt, expected_engine, expected_chart) in formats {
            let (engine, _, chart, _) = select_engine(fmt);
            assert_eq!(engine, expected_engine, "Engine mismatch for {:?}", fmt);
            assert_eq!(chart, expected_chart, "Chart mismatch for {:?}", fmt);
        }
    }
}