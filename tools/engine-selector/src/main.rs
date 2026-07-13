use anyhow::Result;
use clap::Parser;
use engine_selector::{cache_strategy_for, detect_family, detect_format, parse_format_override, select_engine, EngineSelection};

#[derive(Parser)]
#[command(
    name = "engine-selector",
    version,
    about = "Selects the optimal serving engine for a given ML model format",
    long_about = None
)]
struct Cli {
    #[arg(
        short,
        long,
        help = "Path to the model file or directory",
        value_name = "PATH"
    )]
    model: String,

    #[arg(short, long, help = "Force a specific format", value_name = "FORMAT")]
    format: Option<String>,

    #[arg(
        long,
        help = "Output selection as JSON for pipeline integration",
        default_value_t = false
    )]
    json: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let fmt = match cli.format {
        Some(ref f) => parse_format_override(f)?,
        None => detect_format(&cli.model)?,
    };

    let (engine, confidence, chart, rationale) = select_engine(fmt);
    let family = detect_family(&cli.model)?;
    let cache_strategy = cache_strategy_for(family);

    let selection = EngineSelection {
        format: format!("{:?}", fmt).to_lowercase(),
        engine: match engine {
            engine_selector::Engine::Vllm => "vllm".to_string(),
        },
        chart,
        confidence,
        rationale,
        family: format!("{:?}", family).to_lowercase(),
        cache_strategy: cache_strategy.to_string(),
    };

    if cli.json {
        println!("{}", serde_json::to_string_pretty(&selection)?);
    } else {
        println!("Model format   : {}", selection.format.to_uppercase());
        println!("Model family   : {}", selection.family.to_uppercase());
        println!("Serving engine : {}", selection.engine);
        println!("Helm chart     : {}", selection.chart);
        println!("Confidence     : {:.0}%", selection.confidence * 100.0);
        println!("Cache strategy : {}", selection.cache_strategy);
        println!("Rationale      : {}", selection.rationale);
    }

    Ok(())
}