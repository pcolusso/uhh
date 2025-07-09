use clap::Parser;
use infer::InferenceEngine;

use crate::app::App;

pub mod app;
pub mod event;
pub mod infer;
pub mod ui;

#[derive(Parser)]
#[command(name = "uhh")]
#[command(about = "A CLI tool to build up complex CLI commands with LLMs using a TUI interface.")]
#[command(version)]
struct Args {
    #[arg(short, long)]
    input: Option<String>,
    #[arg(short, long)]
    output: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| color_eyre::eyre::eyre!("OPENROUTER_API_KEY environment variable not set"))?;
    let base_url = "https://openrouter.ai/api/v1".to_string();
    let model_name = "google/gemini-2.5-flash".into();

    let args = Args::parse();

    let infer = InferenceEngine::new(api_key, base_url, model_name, args.input, args.output)?;

    let terminal = ratatui::init();
    let result = App::new(infer).run(terminal).await;
    ratatui::restore();
    result
}
