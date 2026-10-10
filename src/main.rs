use std::process::ExitCode;

use std::io::IsTerminal;

use statac::album::Progress;
use statac::cli::{self, Terminal};
use statac::report::colour_for;

#[cfg(target_env = "musl")]
#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() -> ExitCode {
    // The main thread's stack is the system's to size, as small as 1 MB on
    // Windows, so the work runs on a thread with the room pages need.
    let thread = std::thread::Builder::new()
        .name("statac".into())
        .stack_size(cli::STACK_SIZE)
        .spawn(run);
    let code = match thread {
        Ok(thread) => thread
            .join()
            .unwrap_or_else(|fault| std::panic::resume_unwind(fault)),
        Err(_) => run(),
    };
    ExitCode::from(code)
}

fn run() -> u8 {
    let (stdout, mut stderr) = (std::io::stdout(), std::io::stderr());
    let (colour_out, colour_err) = (colour_for(&stdout), colour_for(&stderr));
    let progress = match stderr.is_terminal() {
        true => Progress::Line,
        false => Progress::Lines,
    };
    let mut terminal = Terminal {
        out: &mut stdout.lock(),
        // Left unlocked: the threads making copies say how far they have got.
        err: &mut stderr,
        colour_out,
        colour_err,
        progress,
    };
    cli::run(std::env::args_os().skip(1), &mut terminal)
}
