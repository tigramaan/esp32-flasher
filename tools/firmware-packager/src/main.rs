mod builder;
mod idf;

use builder::{build_package, validate_directory, BuildKind, BuildOptions};
use clap::{Args, Parser, Subcommand, ValueEnum};
use programmer_core::{ErrorCode, OperationError};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "programmer-pack",
    version,
    about = "Создание и проверка пакетов Programmer"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Factory(PackageArgs),
    Update(UpdateArgs),
    Validate {
        package_path: PathBuf,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Args)]
struct PackageArgs {
    #[arg(long)]
    build_dir: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long)]
    package_id: String,
    #[arg(long)]
    version: String,
    #[arg(long)]
    success_marker: String,
    #[arg(long)]
    display_name: Option<String>,
    #[arg(long, default_value_t = 115_200)]
    monitor_baud: u32,
    #[arg(long, default_value_t = 15_000)]
    success_timeout_ms: u64,
    #[arg(long)]
    dry_run: bool,
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Clone, Args)]
struct UpdateArgs {
    #[command(flatten)]
    package: PackageArgs,
    #[arg(long, value_enum)]
    rollback: Rollback,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Rollback {
    Enabled,
    Disabled,
}

fn main() {
    let cli = Cli::parse();
    if let Err(error) = run(cli) {
        eprintln!("{}: {}", error.code, error.message);
        if let Some(detail) = error.detail {
            eprintln!("{detail}");
        }
        std::process::exit(exit_code(error.code));
    }
}

fn run(cli: Cli) -> programmer_core::Result<()> {
    match cli.command {
        Command::Factory(args) => {
            print_build(build_package(&options(BuildKind::Factory, args, false))?)
        }
        Command::Update(args) => print_build(build_package(&options(
            BuildKind::Update,
            args.package,
            matches!(args.rollback, Rollback::Enabled),
        ))?),
        Command::Validate { package_path, json } => {
            let package = validate_directory(&package_path)?;
            let summary = programmer_core::PackageSummary::from(&package);
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).map_err(json_error)?
                );
            } else {
                println!(
                    "OK: {} {} ({:?}, {} BIN, {} bytes)",
                    summary.display_name,
                    summary.firmware_version,
                    summary.kind,
                    summary.segment_count,
                    summary.total_bytes
                );
            }
        }
    }
    Ok(())
}

fn options(kind: BuildKind, args: PackageArgs, rollback_enabled: bool) -> BuildOptions {
    BuildOptions {
        kind,
        build_dir: args.build_dir,
        output_dir: args.out,
        display_name: args.display_name.unwrap_or_else(|| args.package_id.clone()),
        package_id: args.package_id,
        firmware_version: args.version,
        monitor_baud: args.monitor_baud,
        success_marker: args.success_marker,
        success_timeout_ms: args.success_timeout_ms,
        rollback_enabled,
        dry_run: args.dry_run,
        force: args.force,
    }
}

fn print_build(result: builder::BuildResult) {
    let action = if result.dry_run { "DRY-RUN" } else { "CREATED" };
    println!(
        "{action}: {} {} -> {} ({} BIN, {} bytes)",
        result.manifest.display_name,
        result.manifest.firmware_version,
        result.output_dir.display(),
        result.summary.segment_count,
        result.summary.total_bytes
    );
}

fn exit_code(code: ErrorCode) -> i32 {
    match code {
        ErrorCode::PackageInvalid
        | ErrorCode::PackageUnsupported
        | ErrorCode::PackagePathInvalid
        | ErrorCode::PackageFileMissing
        | ErrorCode::HashMismatch => 4,
        ErrorCode::IoError | ErrorCode::DataDirectoryUnwritable => 6,
        _ => 10,
    }
}

fn json_error(error: serde_json::Error) -> OperationError {
    OperationError::new(ErrorCode::InternalError, "Ошибка JSON").with_detail(error.to_string())
}
