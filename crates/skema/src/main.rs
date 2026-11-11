use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand, ValueEnum};
use skema::cli::{format_cli_failure, run_open_editor, RenderOutputs, WarningFormat, WarningOptions};

#[derive(Parser)]
#[command(name = "skema", version, about = "Pallas Skema — validate, render, and open the terminal Editor")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate a Manifest file without writing SVG. Label warnings go to stderr.
    Validate {
        #[arg(long)]
        manifest: PathBuf,
        /// Fail (exit non-zero) on any Label warning.
        #[arg(long)]
        strict: bool,
        /// Label warning format on stderr: text (one line each) or json (one array).
        #[arg(long, value_enum, default_value = "text")]
        warnings: WarningsArg,
    },
    /// Render SVG from a Manifest file (Validation runs first). Label warnings
    /// (shortened or overlapping text, an off-center Diagram) go to stderr and
    /// do not block output unless --strict is given.
    Render {
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        svg: PathBuf,
        /// Also write a PNG of the same Diagram (2 pixels per SVG unit).
        #[arg(long)]
        png: Option<PathBuf>,
        /// Color theme: phosphor, nord, tokyo, mocha, contrast, latte, tokyo-day, or terminal. Default: the theme remembered by the Editor, else terminal (Phosphor until its colors are cached).
        #[arg(long)]
        theme: Option<String>,
        /// Also write an animated GIF: the same size as the PNG, 16 frames of 100 ms
        /// showing Edge pulses (one frame when nothing pulses).
        #[arg(long)]
        gif: Option<PathBuf>,
        /// Fail (exit non-zero) on any Label warning and write no output; existing files stay unchanged.
        #[arg(long)]
        strict: bool,
        /// Label warning format on stderr: text (one line each) or json (one array of
        /// {code, kind, id, text, drawn, fits}).
        #[arg(long, value_enum, default_value = "text")]
        warnings: WarningsArg,
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

#[derive(Clone, Copy, ValueEnum)]
enum WarningsArg {
    Text,
    Json,
}

fn options(strict: bool, warnings: WarningsArg) -> WarningOptions {
    let format = match warnings {
        WarningsArg::Text => WarningFormat::Text,
        WarningsArg::Json => WarningFormat::Json,
    };
    WarningOptions { format, strict }
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Validate { manifest, strict, warnings } => {
            skema::cli::run_validate_with(&manifest, options(strict, warnings)).map(|_| ())
        }
        Commands::Render { manifest, svg, png, theme, gif, strict, warnings } => {
            skema::cli::resolve_theme(theme.as_deref()).and_then(|style| {
                let outputs = RenderOutputs { svg: &svg, png: png.as_deref(), gif: gif.as_deref() };
                skema::cli::run_render_outputs(&manifest, outputs, &style, options(strict, warnings)).map(|_| ())
            })
        }
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
