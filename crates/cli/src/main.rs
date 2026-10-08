//! The `fmtkit` command line.
//!
//! Exit codes: 0 clean (or fixed), 1 findings, 2 usage or configuration
//! error, 3 internal failure.

use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::{Args, Parser, Subcommand, ValueEnum};

use fmtkit_config::Config;
use fmtkit_core::{Lane, Mode, RunResult, VERSION};
use fmtkit_discover::Scope;
use fmtkit_engine::{EngineError, Options, Progress};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const EXIT_FINDINGS: u8 = 1;
const EXIT_USAGE: u8 = 2;
const EXIT_INTERNAL: u8 = 3;

#[derive(Parser)]
#[command(name = "fmtkit", about = "Formats and checks Go, TypeScript, JavaScript, Vue, HTML, and Markdown.", disable_version_flag = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Rewrite files in place, then report what is left to fix by hand.
    Format(RunArgs),
    /// Report what `format` would change and every finding; write nothing.
    Check(RunArgs),
    /// Print the version.
    Version,
}

#[derive(Args)]
#[allow(clippy::struct_excessive_bools)]
struct RunArgs {
    /// Files or directories to cover, changed or not. Without paths: the files changed against HEAD, or every file with --all.
    paths: Vec<PathBuf>,

    /// Cover every file, not only the ones changed against HEAD.
    #[arg(long)]
    all: bool,

    /// Only the Go lane.
    #[arg(long, conflicts_with = "ts")]
    go: bool,

    /// Only the TypeScript lane (scripts, Vue, HTML, Markdown).
    #[arg(long)]
    ts: bool,

    /// Worker threads; 0 means one per CPU. Defaults to FMTKIT_JOBS, then `jobs` in fmtkit.toml, then one per CPU.
    #[arg(long, short)]
    jobs: Option<usize>,

    /// Ignore and do not update the outcome cache.
    #[arg(long)]
    no_cache: bool,

    /// Report format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,

    /// Print only findings and the summary line.
    #[arg(long, short)]
    quiet: bool,

    /// Colour in text output.
    #[arg(long, value_enum, default_value_t = Color::Auto)]
    color: Color,

    /// Read a source from stdin as if it were at this path. `format` prints
    /// the formatted text; `check` prints nothing and exits 1 when formatting
    /// would change it.
    #[arg(long, value_name = "PATH", conflicts_with_all = ["paths", "all"])]
    stdin_filepath: Option<PathBuf>,

    /// Let goimports add and remove imports (slow; loads packages).
    #[arg(long)]
    resolve_imports: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    Agent,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Color {
    Auto,
    Always,
    Never,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Version => {
            println!("fmtkit {VERSION}");

            ExitCode::SUCCESS
        }
        Command::Format(args) => run(Mode::Format, &args),
        Command::Check(args) => run(Mode::Check, &args),
    }
}

fn run(mode: Mode, args: &RunArgs) -> ExitCode {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => return fail(EXIT_INTERNAL, &format!("current directory: {e}")),
    };

    let root = fmtkit_discover::find_root(&cwd);
    let mut config = match Config::load(&root) {
        Ok(loaded) => loaded.config,
        Err(e) => return fail(EXIT_USAGE, &e.to_string()),
    };

    config.go.resolve_imports |= args.resolve_imports;

    if let Some(path) = &args.stdin_filepath {
        return format_stdin(mode, &config, &root, &cwd.join(path));
    }

    let jobs = match config.resolve_jobs(args.jobs) {
        Ok(jobs) => jobs,
        Err(e) => return fail(EXIT_USAGE, &e.to_string()),
    };

    let lanes = match (args.go, args.ts) {
        (true, _) => vec![Lane::Go],
        (_, true) => vec![Lane::Ts],
        _ => Vec::new(),
    };

    let options = Options { root, mode, scope: Scope { all: args.all, lanes, paths: args.paths.clone() }, jobs, cache: !args.no_cache, go_helper: None };
    let format = match args.format {
        OutputFormat::Text => fmtkit_report::Format::Text,
        OutputFormat::Json => fmtkit_report::Format::Json,
        OutputFormat::Agent => fmtkit_report::Format::Agent,
    };

    let interactive = io::stderr().is_terminal() && format == fmtkit_report::Format::Text && !args.quiet;
    let progress = if interactive { progress_bar() } else { Progress::default() };
    let report = match fmtkit_engine::run(&config, &options, &progress) {
        Ok(report) => report,
        Err(e) => return fail(if e.is_internal() { EXIT_INTERNAL } else { EXIT_USAGE }, &e.to_string()),
    };

    let color = match args.color {
        Color::Always => true,
        Color::Never => false,
        Color::Auto => io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
    };

    let mut out = io::stdout().lock();
    let rendered = fmtkit_report::render(&report, fmtkit_report::Options { format, color, quiet: args.quiet }, &mut out).and_then(|()| out.flush());

    if let Err(e) = rendered
        && e.kind() != io::ErrorKind::BrokenPipe
    {
        return fail(EXIT_INTERNAL, &format!("write report: {e}"));
    }

    match report.result {
        RunResult::Pass | RunResult::Fixed => ExitCode::SUCCESS,
        RunResult::Fail => ExitCode::from(EXIT_FINDINGS),
    }
}

fn format_stdin(mode: Mode, config: &Config, root: &std::path::Path, path: &std::path::Path) -> ExitCode {
    let mut source = String::new();

    if let Err(e) = io::stdin().read_to_string(&mut source) {
        return fail(EXIT_USAGE, &format!("read stdin: {e}"));
    }

    match fmtkit_engine::format_text(config, root, path, &source, None) {
        Ok(output) if mode == Mode::Check => {
            if output == source {
                ExitCode::SUCCESS
            } else {
                fail(EXIT_FINDINGS, &format!("{}: formatting would change it", path.display()))
            }
        }
        Ok(output) => {
            let mut out = io::stdout().lock();

            match out.write_all(output.as_bytes()).and_then(|()| out.flush()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
                Err(e) => fail(EXIT_INTERNAL, &format!("write stdout: {e}")),
            }
        }
        Err(e @ (EngineError::File { .. } | EngineError::Unsupported { .. })) => fail(EXIT_FINDINGS, &e.to_string()),
        Err(e) => fail(if e.is_internal() { EXIT_INTERNAL } else { EXIT_USAGE }, &e.to_string()),
    }
}

/// A single-line bar on stderr, redrawn at most once per percent.
fn progress_bar() -> Progress {
    const WIDTH: usize = 30;

    let last = AtomicUsize::new(usize::MAX);

    Progress::new(move |done, total| {
        if done == usize::MAX {
            let _ = write!(io::stderr().lock(), "\r\x1b[2K");

            return;
        }

        if total == 0 {
            return;
        }

        let percent = done * 100 / total;

        if last.swap(percent, Ordering::Relaxed) == percent {
            return;
        }

        let filled = done * WIDTH / total;

        let _ = write!(io::stderr().lock(), "\r[{}{}] {done}/{total}", "#".repeat(filled), " ".repeat(WIDTH - filled));
    })
}

fn fail(code: u8, message: &str) -> ExitCode {
    eprintln!("fmtkit: {message}");

    ExitCode::from(code)
}
