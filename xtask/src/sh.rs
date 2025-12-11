// sh! command execution

use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};

use derive_builder::Builder;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Trait for types that can configure a `Command` before execution.
pub trait ShConfig {
    /// Apply configuration to the given `Command`.
    fn apply(&self, cmd: &mut Command);
}

// Allow using `&T` where `T: ShConfig`.
impl<T: ShConfig + ?Sized> ShConfig for &T {
    fn apply(&self, cmd: &mut Command) {
        (*self).apply(cmd)
    }
}

#[derive(Clone)]
pub enum StreamMode {
    Inherit,
    Pipe,
    Null,
}

#[derive(Clone, Builder)]
#[builder(default)]
pub struct ShOptions {
    pub stdout: StreamMode,
    pub stderr: StreamMode,
    pub cwd: Option<PathBuf>,
    pub quiet: bool,
}

impl Default for ShOptions {
    fn default() -> Self {
        Self {
            stdout: StreamMode::Inherit,
            stderr: StreamMode::Inherit,
            cwd: None,
            quiet: false,
        }
    }
}

impl ShConfig for ShOptions {
    fn apply(&self, cmd: &mut Command) {
        match self.stdout {
            StreamMode::Inherit => cmd.stdout(Stdio::inherit()),
            StreamMode::Pipe => cmd.stdout(Stdio::piped()),
            StreamMode::Null => cmd.stdout(Stdio::null()),
        };

        match self.stderr {
            StreamMode::Inherit => cmd.stderr(Stdio::inherit()),
            StreamMode::Pipe => cmd.stderr(Stdio::piped()),
            StreamMode::Null => cmd.stderr(Stdio::null()),
        };

        if let Some(ref dir) = self.cwd {
            cmd.current_dir(dir);
        }
    }
}

pub struct ShOutput {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

#[macro_export]
macro_rules! sh {
    // Single command with explicit options
    (options($opts:expr), $cmd:expr $(,)?) => {{ $crate::sh::sh($cmd, $opts) }};

    // Single command with default options
    ($cmd:expr $(,)?) => {{ $crate::sh::sh($cmd, $crate::sh::ShOptions::default()) }};
}

pub fn sh<S, O>(cmd: S, opts: O) -> Result<ShOutput>
where
    S: AsRef<str>,
    O: ShConfig,
{
    let cmd = cmd.as_ref();
    log::debug!("[sh] {}", cmd);

    let mut command = Command::new("sh");
    command.arg("-c").arg(cmd);
    opts.apply(&mut command);

    let output = command.output()?;

    if !output.status.success() {
        return Err(format!(
            "Command failed: {}\nExit code: {:?}\n",
            cmd,
            output.status.code().unwrap_or(-1),
        )
        .into());
    }

    Ok(ShOutput {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}
