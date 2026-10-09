//! The `fmtkit` command line.
//!
//! Exit codes: 0 clean (or fixed), 1 findings, 2 usage or configuration
//! error, 3 internal failure.
//!
//! `format` and `check` hand the run to a `fmtkit serve` for the repository
//! when one is listening and agrees to it, and run it themselves otherwise.

#[cfg(unix)]
mod serve;

use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

use fmtkit_config::Config;
use fmtkit_core::{Lane, Mode, Report, RunResult, VERSION};
use fmtkit_discover::Scope;
use fmtkit_engine::{EngineError, Options, Progress, Session};

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
    /// Keep running for this repository, so `format` and `check` here start
    /// warm: they hand their runs to it while it runs.
    Serve(ServeArgs),
    /// Print the version.
    Version,
}

#[derive(Args)]
struct ServeArgs {
    /// Watch the repository between calls (kqueue on macOS, inotify on Linux), so a call looks only at what changed since the last.
    #[arg(long)]
    watch: bool,
}

#[derive(Args, Serialize, Deserialize)]
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

#[derive(Clone, Copy, ValueEnum, Serialize, Deserialize)]
enum OutputFormat {
    Text,
    Json,
    Agent,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
enum Color {
    Auto,
    Always,
    Never,
}

/// One `format` or `check`, with what it learned from its terminal.
#[derive(Serialize, Deserialize)]
struct Call {
    mode: Mode,
    args: RunArgs,
    cwd: PathBuf,
    /// The repository root that contains `cwd`.
    root: PathBuf,
    color: bool,
    /// Draw a progress bar on stderr.
    interactive: bool,
}

/// A run that ended before its report: the exit code and the message.
struct Failure(u8, String);

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Version => {
            println!("fmtkit {VERSION}");

            ExitCode::SUCCESS
        }
        Command::Format(args) => run(Mode::Format, args),
        Command::Check(args) => run(Mode::Check, args),
        Command::Serve(args) => serve(&args),
    }
}

#[cfg(unix)]
fn serve(args: &ServeArgs) -> ExitCode {
    match std::env::current_dir() {
        Ok(cwd) => serve::serve(&cwd, args.watch),
        Err(e) => fail(EXIT_INTERNAL, &format!("current directory: {e}")),
    }
}

#[cfg(not(unix))]
fn serve(_args: &ServeArgs) -> ExitCode {
    fail(EXIT_USAGE, "serve needs a Unix socket")
}

fn run(mode: Mode, args: RunArgs) -> ExitCode {
    let cwd = match std::env::current_dir() {
        Ok(cwd) => cwd,
        Err(e) => return fail(EXIT_INTERNAL, &format!("current directory: {e}")),
    };

    if let Some(path) = &args.stdin_filepath {
        return format_stdin(mode, &cwd, args.resolve_imports, path);
    }

    let color = match args.color {
        Color::Always => true,
        Color::Never => false,
        Color::Auto => io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none(),
    };

    let interactive = io::stderr().is_terminal() && matches!(args.format, OutputFormat::Text) && !args.quiet;
    let root = fmtkit_discover::find_root(&cwd);
    let call = Call { mode, args, cwd, root, color, interactive };

    // A run without the cache is asked for a cold start, so it stays local.
    #[cfg(unix)]
    if !call.args.no_cache
        && let Some(code) = serve::ask(&call)
    {
        return ExitCode::from(code);
    }

    let progress = if call.interactive { Progress::new(progress_bar()) } else { Progress::default() };
    let report = match prepare(&call).and_then(|(config, options)| fmtkit_engine::run(&config, &options, &progress).map_err(Failure::from)) {
        Ok(report) => report,
        Err(Failure(code, message)) => return fail(code, &message),
    };

    let code = finish(&report, &call, &mut io::stdout().lock()).unwrap_or_else(|Failure(code, message)| {
        eprintln!("fmtkit: {message}");

        code
    });

    // The process exits next; freeing every finding first only delays it.
    std::mem::forget(report);

    ExitCode::from(code)
}

/// Run `call` in `session`, writing the report to `out` and any failure to
/// `err`; the exit code.
fn execute(session: &mut Session, call: &Call, progress: &Progress, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    let ran = prepare(call).and_then(|(config, options)| session.run(&config, &options, progress).map_err(Failure::from));
    let finished = ran.and_then(|report| finish(&report, call, out));

    finished.unwrap_or_else(|Failure(code, message)| {
        let _ = writeln!(err, "fmtkit: {message}");

        code
    })
}

/// The configuration and options `call` runs with.
fn prepare(call: &Call) -> Result<(Config, Options), Failure> {
    let args = &call.args;
    let mut config = Config::load(&call.root).map_err(|e| Failure(EXIT_USAGE, e.to_string()))?.config;

    config.go.resolve_imports |= args.resolve_imports;

    let jobs = config.resolve_jobs(args.jobs).map_err(|e| Failure(EXIT_USAGE, e.to_string()))?;
    let lanes = match (args.go, args.ts) {
        (true, _) => vec![Lane::Go],
        (_, true) => vec![Lane::Ts],
        _ => Vec::new(),
    };

    let scope = Scope { all: args.all, lanes, paths: args.paths.clone(), cwd: Some(call.cwd.clone()) };
    let options = Options { root: call.root.clone(), mode: call.mode, scope, jobs, cache: !args.no_cache, go_helper: None };

    Ok((config, options))
}

/// Render `report` to `out`; the exit code it calls for.
fn finish(report: &Report, call: &Call, out: &mut dyn Write) -> Result<u8, Failure> {
    let format = match call.args.format {
        OutputFormat::Text => fmtkit_report::Format::Text,
        OutputFormat::Json => fmtkit_report::Format::Json,
        OutputFormat::Agent => fmtkit_report::Format::Agent,
    };

    let options = fmtkit_report::Options { format, color: call.color, quiet: call.args.quiet };
    let rendered = fmtkit_report::render(report, options, out).and_then(|()| out.flush());

    if let Err(e) = rendered
        && e.kind() != io::ErrorKind::BrokenPipe
    {
        return Err(Failure(EXIT_INTERNAL, format!("write report: {e}")));
    }

    Ok(match report.result {
        RunResult::Pass | RunResult::Fixed => 0,
        RunResult::Fail => EXIT_FINDINGS,
    })
}

impl From<EngineError> for Failure {
    fn from(e: EngineError) -> Self {
        Self(if e.is_internal() { EXIT_INTERNAL } else { EXIT_USAGE }, e.to_string())
    }
}

fn format_stdin(mode: Mode, cwd: &Path, resolve_imports: bool, path: &Path) -> ExitCode {
    let root = fmtkit_discover::find_root(cwd);
    let mut config = match Config::load(&root) {
        Ok(loaded) => loaded.config,
        Err(e) => return fail(EXIT_USAGE, &e.to_string()),
    };

    config.go.resolve_imports |= resolve_imports;

    let path = cwd.join(path);
    let mut source = String::new();

    if let Err(e) = io::stdin().read_to_string(&mut source) {
        return fail(EXIT_USAGE, &format!("read stdin: {e}"));
    }

    match fmtkit_engine::format_text(&config, &root, &path, &source, None) {
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
fn progress_bar() -> impl Fn(usize, usize) + Send + Sync + 'static {
    const WIDTH: usize = 30;

    let last = AtomicUsize::new(usize::MAX);

    move |done, total| {
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
    }
}

fn fail(code: u8, message: &str) -> ExitCode {
    eprintln!("fmtkit: {message}");

    ExitCode::from(code)
}
