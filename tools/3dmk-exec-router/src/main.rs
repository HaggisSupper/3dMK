use std::env;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Runner {
    Bash,
    PowerShellCore,
    WindowsPowerShell,
    Cargo,
    NvidiaSmi,
}

impl Runner {
    const fn display_name(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::PowerShellCore => "pwsh",
            Self::WindowsPowerShell => "powershell.exe",
            Self::Cargo => "cargo",
            Self::NvidiaSmi => "nvidia-smi",
        }
    }

    const fn windows_only(self) -> bool {
        matches!(self, Self::WindowsPowerShell)
    }
}

#[derive(Clone, Copy, Debug)]
enum Invocation {
    Script,
    CargoTest,
    NvidiaProbe,
}

#[derive(Clone, Copy, Debug)]
struct Profile {
    name: &'static str,
    runners: &'static [Runner],
    script: Option<&'static str>,
    windows_only: bool,
    invocation: Invocation,
}

const BASH_ONLY: &[Runner] = &[Runner::Bash];
const POWERSHELL_FIRST: &[Runner] = &[Runner::PowerShellCore, Runner::WindowsPowerShell];
const CARGO_ONLY: &[Runner] = &[Runner::Cargo];
const NVIDIA_ONLY: &[Runner] = &[Runner::NvidiaSmi];

impl Profile {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "toolkit-check" => Some(Self {
                name: "toolkit-check",
                runners: BASH_ONLY,
                script: Some("scripts/validate-3dmk-toolkit.sh"),
                windows_only: false,
                invocation: Invocation::Script,
            }),
            "windows-toolkit-check" => Some(Self {
                name: "windows-toolkit-check",
                runners: POWERSHELL_FIRST,
                script: Some("scripts/validate-3dmk-toolkit.ps1"),
                windows_only: true,
                invocation: Invocation::Script,
            }),
            "rust-check" => Some(Self {
                name: "rust-check",
                runners: CARGO_ONLY,
                script: None,
                windows_only: false,
                invocation: Invocation::CargoTest,
            }),
            "cuda-probe" => Some(Self {
                name: "cuda-probe",
                runners: NVIDIA_ONLY,
                script: None,
                windows_only: false,
                invocation: Invocation::NvidiaProbe,
            }),
            "mistralrs-agent" => Some(Self {
                name: "mistralrs-agent",
                runners: POWERSHELL_FIRST,
                script: Some("scripts/run-mistralrs-vwm-task.ps1"),
                windows_only: true,
                invocation: Invocation::Script,
            }),
            _ => None,
        }
    }

    fn preferred_runner(self) -> Runner {
        self.runners[0]
    }
}

#[derive(Debug)]
struct ProbeResult {
    profile: Profile,
    runner: Option<(Runner, PathBuf)>,
    reason: Option<String>,
}

impl ProbeResult {
    fn runnable(profile: Profile, runner: Runner, executable: PathBuf) -> Self {
        Self {
            profile,
            runner: Some((runner, executable)),
            reason: None,
        }
    }

    fn blocked(profile: Profile, reason: impl Into<String>) -> Self {
        Self {
            profile,
            runner: None,
            reason: Some(reason.into()),
        }
    }

    fn is_runnable(&self) -> bool {
        self.runner.is_some()
    }

    fn print_json(&self) {
        let state = if self.is_runnable() { "RUNNABLE" } else { "BLOCKED" };
        let runner = self
            .runner
            .as_ref()
            .map(|(runner, _)| runner.display_name())
            .unwrap_or("");
        let executable = self
            .runner
            .as_ref()
            .map(|(_, executable)| executable.to_string_lossy().into_owned())
            .unwrap_or_default();
        let reason = self.reason.as_deref().unwrap_or("");

        println!(
            "{{\"state\":\"{}\",\"profile\":\"{}\",\"runner\":\"{}\",\"executable\":\"{}\",\"reason\":\"{}\"}}",
            state,
            escape_json(self.profile.name),
            escape_json(runner),
            escape_json(&executable),
            escape_json(reason)
        );
    }
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            character if character.is_control() => format!("\\u{:04x}", character as u32).chars().collect(),
            character => vec![character],
        })
        .collect()
}

fn host_is_windows() -> bool {
    cfg!(windows)
}

fn executable_candidates(runner: Runner) -> Vec<OsString> {
    let mut candidates = vec![OsString::from(runner.display_name())];

    if host_is_windows() {
        match runner {
            Runner::Bash => {
                if let Some(program_files) = env::var_os("ProgramFiles") {
                    candidates.push(Path::new(&program_files).join("Git").join("bin").join("bash.exe").into_os_string());
                    candidates.push(Path::new(&program_files).join("Git").join("usr").join("bin").join("bash.exe").into_os_string());
                }
            }
            Runner::PowerShellCore => candidates.push(OsString::from("pwsh.exe")),
            Runner::Cargo => candidates.push(OsString::from("cargo.exe")),
            Runner::NvidiaSmi => candidates.push(OsString::from("nvidia-smi.exe")),
            Runner::WindowsPowerShell => {}
        }
    }

    candidates
}

fn find_executable(candidate: &OsString) -> Option<PathBuf> {
    let candidate_path = Path::new(candidate);
    if candidate_path.components().count() > 1 {
        return candidate_path.is_file().then(|| candidate_path.to_path_buf());
    }

    let path_variable = env::var_os("PATH")?;
    for directory in env::split_paths(&path_variable) {
        let path = directory.join(candidate);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn probe_profile(profile: Profile, repository_root: &Path) -> ProbeResult {
    if profile.windows_only && !host_is_windows() {
        return ProbeResult::blocked(profile, "This profile requires a Windows host.");
    }

    if let Some(script) = profile.script {
        let script_path = repository_root.join(script);
        if !script_path.is_file() {
            return ProbeResult::blocked(
                profile,
                format!("Required script is missing: {}", script_path.display()),
            );
        }
    }

    for runner in profile.runners {
        if runner.windows_only() && !host_is_windows() {
            continue;
        }
        for candidate in executable_candidates(*runner) {
            if let Some(executable) = find_executable(&candidate) {
                return ProbeResult::runnable(profile, *runner, executable);
            }
        }
    }

    let candidates = profile
        .runners
        .iter()
        .map(|runner| runner.display_name())
        .collect::<Vec<_>>()
        .join(", ");
    ProbeResult::blocked(profile, format!("No declared runner is available: {candidates}."))
}

fn command_for(
    probe: &ProbeResult,
    repository_root: &Path,
    forwarded_arguments: &[String],
) -> io::Result<Command> {
    let (runner, executable) = probe
        .runner
        .as_ref()
        .ok_or_else(|| io::Error::other("Cannot execute a blocked profile."))?;

    let mut command = Command::new(executable);
    match probe.profile.invocation {
        Invocation::Script => {
            let script = repository_root.join(probe.profile.script.expect("script profile"));
            match runner {
                Runner::Bash => {
                    command.arg(script);
                }
                Runner::PowerShellCore | Runner::WindowsPowerShell => {
                    command.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"]);
                    command.arg(script);
                }
                Runner::Cargo | Runner::NvidiaSmi => {
                    return Err(io::Error::other("Script profile selected a non-script runner."));
                }
            }
        }
        Invocation::CargoTest => {
            command.args(["test", "--all-targets"]);
            command.current_dir(repository_root.join("tools/3dmk-exec-router"));
        }
        Invocation::NvidiaProbe => {
            command.args([
                "--query-gpu=name,driver_version,memory.total",
                "--format=csv,noheader",
            ]);
        }
    }
    command.args(forwarded_arguments);
    Ok(command)
}

fn print_usage() {
    eprintln!(
        "Usage: 3dmk-exec-router <probe|run> <profile> [-- arguments...]\nProfiles: toolkit-check, windows-toolkit-check, rust-check, cuda-probe, mistralrs-agent"
    );
}

fn split_forwarded_arguments(arguments: &[String]) -> &[String] {
    arguments
        .iter()
        .position(|argument| argument == "--")
        .map(|index| &arguments[index + 1..])
        .unwrap_or(&[])
}

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() < 2 {
        print_usage();
        return ExitCode::from(2);
    }

    let action = &arguments[0];
    let Some(profile) = Profile::from_name(&arguments[1]) else {
        eprintln!("Unknown profile: {}", arguments[1]);
        return ExitCode::from(2);
    };
    let repository_root = match env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("Unable to resolve repository root: {error}");
            return ExitCode::from(3);
        }
    };

    let probe = probe_profile(profile, &repository_root);
    if action == "probe" {
        probe.print_json();
        return if probe.is_runnable() {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(3)
        };
    }
    if action != "run" {
        print_usage();
        return ExitCode::from(2);
    }

    if !probe.is_runnable() {
        probe.print_json();
        return ExitCode::from(3);
    }

    let forwarded_arguments = split_forwarded_arguments(&arguments[2..]);
    let mut command = match command_for(&probe, &repository_root, forwarded_arguments) {
        Ok(command) => command,
        Err(error) => {
            eprintln!("Unable to build command: {error}");
            return ExitCode::from(3);
        }
    };

    match command.status() {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("Failed to start selected runner: {error}");
            ExitCode::from(3)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{escape_json, Profile, Runner};

    #[test]
    fn toolkit_check_prefers_bash() {
        let profile = Profile::from_name("toolkit-check").expect("known profile");
        assert_eq!(profile.preferred_runner(), Runner::Bash);
    }

    #[test]
    fn windows_toolkit_check_prefers_pwsh() {
        let profile = Profile::from_name("windows-toolkit-check").expect("known profile");
        assert_eq!(profile.preferred_runner(), Runner::PowerShellCore);
    }

    #[test]
    fn unknown_profile_is_rejected() {
        assert!(Profile::from_name("not-a-profile").is_none());
    }

    #[test]
    fn json_escaping_is_safe_for_control_characters() {
        assert_eq!(escape_json("\"\\\n"), "\\\"\\\\\\n");
    }
}
