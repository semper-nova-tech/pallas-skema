use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};
use skema::cli::{format_cli_failure, run_open_editor, run_render, run_validate, CliInvocation};

#[derive(Parser)]
#[command(name = "skema", about = "Pallas Skema v1 — validate, render, and Editor handoff")]
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
    /// Write Editor JSON handoff payload (does not open a Herdr pane).
    OpenEditor {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        svg: PathBuf,
        #[arg(long)]
        handoff_out: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    let invocation = match &cli.command {
        Commands::Validate { manifest } => CliInvocation::Validate {
            manifest_path: manifest.display().to_string(),
        },
        Commands::Render { manifest, svg } => CliInvocation::Render {
            manifest_path: manifest.display().to_string(),
            svg_path: svg.display().to_string(),
        },
        Commands::OpenEditor {
            manifest,
            svg,
            handoff_out,
        } => CliInvocation::OpenEditor {
            manifest_path: manifest.display().to_string(),
            svg_path: svg.display().to_string(),
            handoff_out: handoff_out.display().to_string(),
        },
    };
    let _ = invocation;

    let result = match cli.command {
        Commands::Validate { manifest } => run_validate(&manifest),
        Commands::Render { manifest, svg } => run_render(&manifest, &svg),
        Commands::OpenEditor {
            manifest,
            svg,
            handoff_out,
        } => run_open_editor(&manifest, &svg, &handoff_out),
    };

    if let Err(failure) = result {
        eprintln!("{}", format_cli_failure(&failure));
        process::exit(1);
    }
}
