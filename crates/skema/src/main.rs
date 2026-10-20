use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use skema::cli::{format_cli_failure, run_open_editor, run_render, run_validate};

#[derive(Parser)]
#[command(name = "skema", about = "Pallas Skema v1 — validate, render, and open the terminal Editor")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a Manifest file without writing SVG.
    Validate {
        #[arg(long)]
        manifest: PathBuf,
    },
    /// Render SVG from a Manifest file (Validation runs first).
    Render {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        svg: PathBuf,
    },
    /// Open the terminal Editor for one Manifest and SVG in this terminal.
    OpenEditor {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        svg: PathBuf,
        /// Write the Editor JSON handoff to this file and exit instead of opening the Editor.
        #[arg(long)]
        handoff_out: Option<PathBuf>,
        /// Only write the Manifest and SVG on `s`, instead of after every accepted edit.
        #[arg(long)]
        manual_save: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Validate { manifest } => run_validate(&manifest),
        Commands::Render { manifest, svg } => run_render(&manifest, &svg),
        Commands::OpenEditor {
            manifest,
            svg,
            handoff_out: Some(out),
            ..
        } => run_open_editor(&manifest, &svg, &out),
        Commands::OpenEditor {
            manifest,
            svg,
            handoff_out: None,
            manual_save,
        } => skema_tui::run(&manifest, &svg, !manual_save),
    };

    if let Err(failure) = result {
        eprintln!("{}", format_cli_failure(&failure));
        process::exit(1);
    }
}
