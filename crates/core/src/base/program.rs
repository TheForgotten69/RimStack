//! Another program run to its end, or stopped at a limit: one that
//! waits on a network or a prompt that never answers would otherwise
//! hold its task for as long as RimStack runs.

use crate::{say, say_n};

use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// What a program on this computer answers at once.
pub const LOCAL: Duration = Duration::from_secs(30);
/// What asks a server and brings back little.
pub const NETWORK: Duration = Duration::from_secs(10 * 60);
/// What downloads as much as mods weigh.
pub const DOWNLOAD: Duration = Duration::from_secs(2 * 60 * 60);

fn collected(pipe: Option<impl Read + Send + 'static>) -> std::thread::JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut all = Vec::new();
        if let Some(mut pipe) = pipe {
            // What was read before a failure is still what it said.
            let _ = pipe.read_to_end(&mut all);
        }
        all
    })
}

/// Runs the program with nothing to read, and gives what it wrote. Past
/// `limit` it is killed and the error is `TimedOut`. Only the program
/// itself is killed: what it started in turn is left to end by itself.
pub fn run(command: &mut Command, limit: Duration) -> std::io::Result<Output> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let (stdout, stderr) = (
        collected(child.stdout.take()),
        collected(child.stderr.take()),
    );
    let started = Instant::now();
    // Asked often at first: most programs run here answer in milliseconds.
    let mut pause = Duration::from_millis(1);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= limit {
            child.kill()?;
            child.wait()?;
            // The pipes are not waited for: a program it started may hold them open.
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                say_n!(
                    limit.as_secs() as usize,
                    "no answer after {} second",
                    "no answer after {} seconds"
                ),
            ));
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(50));
    };
    let read = |pipe: std::thread::JoinHandle<Vec<u8>>| {
        let lost = |_| std::io::Error::other(say!("what the program wrote could not be read"));
        pipe.join().map_err(lost)
    };
    Ok(Output {
        status,
        stdout: read(stdout)?,
        stderr: read(stderr)?,
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn what_a_program_wrote_and_how_it_ended_come_back() {
        let ran = run(
            Command::new("sh").args(["-c", "echo out; echo err >&2; exit 3"]),
            LOCAL,
        )
        .unwrap();
        assert_eq!(ran.status.code(), Some(3));
        assert_eq!(
            (ran.stdout.as_slice(), ran.stderr.as_slice()),
            (&b"out\n"[..], &b"err\n"[..])
        );
    }

    #[test]
    fn more_than_a_pipe_holds_is_read_while_the_program_runs() {
        let ran = run(
            Command::new("sh").args(["-c", "head -c 300000 /dev/zero"]),
            LOCAL,
        )
        .unwrap();
        assert_eq!(ran.stdout.len(), 300_000);
    }

    #[test]
    fn a_program_that_does_not_end_is_stopped_at_the_limit() {
        let started = Instant::now();
        let ran = run(Command::new("sleep").arg("30"), Duration::from_millis(150));
        assert_eq!(ran.unwrap_err().kind(), std::io::ErrorKind::TimedOut);
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn a_program_that_is_not_there_is_an_error_at_once() {
        assert!(run(&mut Command::new("/nonexistent/program"), LOCAL).is_err());
    }
}
