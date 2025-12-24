// sh! command execution

#![allow(dead_code)]

use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
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

#[derive(Copy, Clone, Default)]
pub enum StreamMode {
    #[default]
    Inherit,
    Pipe,
    Null,
}

impl From<StreamMode> for Stdio {
    fn from(m: StreamMode) -> Self {
        match m {
            StreamMode::Inherit => Stdio::inherit(),
            StreamMode::Pipe => Stdio::piped(),
            StreamMode::Null => Stdio::null(),
        }
    }
}

#[derive(Clone, Default, Builder)]
#[builder(default)]
pub struct ShOptions {
    pub stdout: StreamMode,
    pub stderr: StreamMode,
    pub cwd: Option<PathBuf>,
    pub quiet: bool,
}

impl ShConfig for ShOptions {
    fn apply(&self, cmd: &mut Command) {
        cmd.stdout(Stdio::from(self.stdout))
            .stderr(Stdio::from(self.stderr));
        if let Some(dir) = &self.cwd {
            cmd.current_dir(dir);
        }
    }
}

pub struct ShOutput(pub ExitStatus, pub String, pub String);

pub enum ShCmd {
    Script(String),
    Argv(OsString, Vec<OsString>),
}

#[macro_export]
macro_rules! sh {
    (options($opts:expr), $prog:expr, [$($arg:expr),* $(,)?] $(,)?) => {{
        $crate::sh::sh(
            $crate::sh::ShCmd::Argv(
                std::ffi::OsString::from($prog),
                vec![$(std::ffi::OsString::from($arg)),*],
            ),
            $opts,
        )
    }};

    ($prog:expr, [$($arg:expr),* $(,)?] $(,)?) => {{
        $crate::sh::sh(
            $crate::sh::ShCmd::Argv(
                std::ffi::OsString::from($prog),
                vec![$(std::ffi::OsString::from($arg)),*],
            ),
            $crate::sh::ShOptions::default(),
        )
    }};

    (options($opts:expr), $cmd:expr) => {{
        let cmd = $cmd.to_string();
        $crate::sh::sh($crate::sh::ShCmd::Script(cmd), $opts)
    }};

    ($cmd:expr) => {{
        let cmd = $cmd.to_string();
        $crate::sh::sh($crate::sh::ShCmd::Script(cmd), $crate::sh::ShOptions::default())
    }};
}

pub fn sh(cmd: ShCmd, opts: ShOptions) -> Result<ShOutput> {
    fn exec(
        mut c: Command,
        opts: &ShOptions,
        desc: &str,
        cleanup: Option<&Path>,
    ) -> Result<ShOutput> {
        opts.apply(&mut c);

        let output = c.output()?;
        if let Some(p) = cleanup {
            let _ = std::fs::remove_file(p);
        }

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if !output.status.success() {
            return Err(format!(
                "Command failed with exit code {:?}: {}\nStderr: {}",
                output.status.code(),
                desc,
                stderr
            )
            .into());
        }

        Ok(ShOutput(output.status, stdout, stderr))
    }

    fn build_argv(program: &OsString, args: &[OsString]) -> (Command, String) {
        let mut c = Command::new(program);
        c.args(args);
        let desc = format!("(argv) {:?} {:?}", program, args);
        (c, desc)
    }

    fn build_script(cmd: &str) -> Result<(Command, String, Option<PathBuf>)> {
        let cmd_trim = cmd.trim_start();
        if cmd_trim.starts_with("#!") {
            let path = std::env::temp_dir().join(format!(
                "zk-xtask-{}-{}.tmp",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ));
            std::fs::write(&path, cmd_trim)?;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))?;
            Ok((Command::new(&path), cmd_trim.to_string(), Some(path)))
        } else {
            let mut c = Command::new("sh");
            // Important: don't trim in the `sh -c` path; it can change shell script semantics.
            c.arg("-c").arg(cmd);
            Ok((c, cmd_trim.to_string(), None))
        }
    }

    match cmd {
        ShCmd::Argv(program, args) => {
            if !opts.quiet {
                log::debug!("[sh] (argv) {:?} {:?}", program, args);
            }
            let (c, desc) = build_argv(&program, &args);
            exec(c, &opts, &desc, None)
        }
        ShCmd::Script(cmd) => {
            let (c, desc, cleanup) = build_script(&cmd)?;
            if !opts.quiet {
                log::debug!("[sh] {}", desc);
            }
            exec(c, &opts, &desc, cleanup.as_deref())
        }
    }
}
