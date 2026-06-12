use crate::commands;
use crate::error::Result;
use crate::ui::Ui;
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "resxguard")]
#[command(
    version,
    about = "Guard .resx files against mojibake and encoding damage"
)]
pub struct Cli {
    /// Disable ANSI colors. Also disabled automatically when NO_COLOR is set or stdout is not a terminal.
    #[arg(long, global = true)]
    no_color: bool,

    /// Disable emoji in console output.
    #[arg(long, global = true)]
    no_emoji: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Scan .resx files and report suspicious/correctable strings without modifying files.
    Check {
        path: PathBuf,
        #[arg(short = 'r', long = "recursive")]
        recursive: bool,
        #[arg(long, value_enum, default_value_t = Mode::Safe)]
        mode: Mode,
        #[arg(long, default_value_t = 2)]
        max_passes: usize,
        #[arg(long)]
        decode_html_entities: bool,
        /// Include mojibake detection/repair for resource keys in data name attributes.
        /// Enabled by default because Codex and merge tools can corrupt keys too.
        /// Use --no-attributes to disable this.
        #[arg(long = "no-attributes", action = clap::ArgAction::SetFalse, default_value_t = true)]
        include_attributes: bool,
        #[arg(long)]
        report: Option<PathBuf>,
        #[arg(long)]
        fail_on_suspicious: bool,
    },
    /// Repair .resx files in place, optionally with .bak backup.
    Repair {
        path: PathBuf,
        #[arg(short = 'r', long = "recursive")]
        recursive: bool,
        #[arg(long, value_enum, default_value_t = Mode::Safe)]
        mode: Mode,
        #[arg(long, default_value_t = 2)]
        max_passes: usize,
        #[arg(long)]
        decode_html_entities: bool,
        /// Include mojibake detection/repair for resource keys in data name attributes.
        /// Enabled by default because Codex and merge tools can corrupt keys too.
        /// Use --no-attributes to disable this.
        #[arg(long = "no-attributes", action = clap::ArgAction::SetFalse, default_value_t = true)]
        include_attributes: bool,
        #[arg(long)]
        backup: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Set or optionally create a single .resx key/value without rewriting the full XML document.
    Set {
        path: PathBuf,
        name: String,
        value: String,
        #[arg(long)]
        comment: Option<String>,
        /// Create the resource when it does not already exist.
        #[arg(long)]
        create: bool,
        /// Allow adding text that still looks like mojibake. Disabled by default.
        #[arg(long)]
        allow_suspicious: bool,
        #[arg(long)]
        backup: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Import multiple .resx key/value rows from CSV or JSON. Missing resources are created by default.
    Import {
        path: PathBuf,
        input: PathBuf,
        /// Do not create missing resources; update existing keys only.
        #[arg(long)]
        update_only: bool,
        /// With --update-only, skip missing keys instead of failing.
        #[arg(long)]
        skip_missing: bool,
        /// Continue importing valid rows and report invalid rows instead of stopping at first error.
        #[arg(long)]
        continue_on_error: bool,
        /// Allow importing text that still looks like mojibake. Disabled by default.
        #[arg(long)]
        allow_suspicious: bool,
        #[arg(long)]
        backup: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        report: Option<PathBuf>,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Safe,
    Broad,
    Aggressive,
}

pub fn run_from_env() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(2)
        }
    }
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let ui = Ui::new(cli.no_color, cli.no_emoji);
    match cli.command {
        Command::Check {
            path,
            recursive,
            mode,
            max_passes,
            decode_html_entities,
            include_attributes,
            report,
            fail_on_suspicious,
        } => commands::check::run(
            &path,
            recursive,
            mode,
            max_passes,
            decode_html_entities,
            include_attributes,
            report.as_deref(),
            fail_on_suspicious,
            &ui,
        ),
        Command::Repair {
            path,
            recursive,
            mode,
            max_passes,
            decode_html_entities,
            include_attributes,
            backup,
            dry_run,
            report,
        } => commands::repair::run(
            &path,
            recursive,
            mode,
            max_passes,
            decode_html_entities,
            include_attributes,
            backup,
            dry_run,
            report.as_deref(),
            &ui,
        ),
        Command::Set {
            path,
            name,
            value,
            comment,
            create,
            allow_suspicious,
            backup,
            dry_run,
            report,
        } => commands::set::run(
            &path,
            name,
            value,
            comment,
            create,
            allow_suspicious,
            backup,
            dry_run,
            report.as_deref(),
            &ui,
        ),
        Command::Import {
            path,
            input,
            update_only,
            skip_missing,
            continue_on_error,
            allow_suspicious,
            backup,
            dry_run,
            report,
        } => commands::import::run(
            &path,
            &input,
            update_only,
            skip_missing,
            continue_on_error,
            allow_suspicious,
            backup,
            dry_run,
            report.as_deref(),
            &ui,
        ),
    }
}
