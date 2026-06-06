use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use anyhow::Context;
use clap::{Parser, Subcommand};

#[cfg(windows)]
mod proxy;

#[derive(Parser)]
#[command(name = "dae-box", about = "A sing-box wrapper using dae configuration")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run sing-box service
    Run {
        #[command(flatten)]
        args: ConfigArgs,
    },
    /// Check configuration
    Check {
        #[command(flatten)]
        args: ConfigArgs,
    },
    /// Format configuration
    Format {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<std::ffi::OsString>,
    },
    /// Utility tools
    Tools {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<std::ffi::OsString>,
    },
    /// Show version
    Version,
}

#[derive(clap::Args)]
struct ConfigArgs {
    #[arg(short = 'c', long, default_value = "config.dae")]
    config: PathBuf,
    #[arg(short = 'D', long)]
    directory: Option<PathBuf>,
    #[arg(long)]
    disable_color: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let code = match cli.command {
        Commands::Run { args } => run(&args)?,
        Commands::Check { args } => check(&args)?,
        Commands::Format { args } => exec_passthrough("format", &args)?,
        Commands::Tools { args } => exec_passthrough("tools", &args)?,
        Commands::Version => exec_passthrough("version", std::iter::empty::<&OsStr>())?,
    };
    std::process::exit(code);
}

fn run(args: &ConfigArgs) -> anyhow::Result<i32> {
    let (config_arg, _temp_dir, proxy_addr) = prepare_config(&args.config)?;

    #[cfg(windows)]
    proxy::set_ctrlc_handler()?;

    #[cfg(windows)]
    let _proxy_guard: Option<proxy::ProxyGuard> = if let Some(addr) = &proxy_addr {
        Some(proxy::ProxyGuard::set(addr)?)
    } else {
        eprintln!("[dae-box] Warning: no proxy-capable inbound found");
        None
    };

    #[cfg(not(windows))]
    let _ = proxy_addr;

    let status = exec_singbox("run", &build_singbox_args(&config_arg, args))?;
    Ok(status.code().unwrap_or(1))
}

fn check(args: &ConfigArgs) -> anyhow::Result<i32> {
    let (config_arg, _temp_dir, _) = prepare_config(&args.config)?;
    let status = exec_singbox("check", &build_singbox_args(&config_arg, args))?;
    Ok(status.code().unwrap_or(1))
}

fn exec_passthrough<I, S>(subcommand: &str, args: I) -> anyhow::Result<i32>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let status = Command::new("sing-box")
        .arg(subcommand)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("failed to execute sing-box")?;
    Ok(status.code().unwrap_or(1))
}

fn prepare_config(
    config_path: &Path,
) -> anyhow::Result<(String, Option<tempfile::TempDir>, Option<String>)> {
    let ext = config_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    if ext == "dae" {
        eprintln!("[dae-box] Converting dae config: {}", config_path.display());
        convert_dae_config(config_path)
    } else {
        let content = std::fs::read_to_string(config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;
        let proxy_addr = serde_json::from_str::<serde_json::Value>(&content)
            .ok()
            .as_ref()
            .and_then(extract_proxy_addr);
        Ok((config_path.to_string_lossy().to_string(), None, proxy_addr))
    }
}

fn convert_dae_config(
    config_path: &Path,
) -> anyhow::Result<(String, Option<tempfile::TempDir>, Option<String>)> {
    let dae_text = std::fs::read_to_string(config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let value = sing_dae::dae_to_singbox_value(&dae_text)?;

    let proxy_addr = extract_proxy_addr(&value);
    let json = serde_json::to_string_pretty(&value)?;

    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path().join("config.json");
    std::fs::write(&temp_path, &json)?;

    Ok((
        temp_path.to_string_lossy().to_string(),
        Some(temp_dir),
        proxy_addr,
    ))
}

fn build_singbox_args(config_path: &str, args: &ConfigArgs) -> Vec<String> {
    let mut result = vec!["-c".to_string(), config_path.to_string()];
    if let Some(ref dir) = args.directory {
        result.push("-D".to_string());
        result.push(dir.to_string_lossy().to_string());
    }
    if args.disable_color {
        result.push("--disable-color".to_string());
    }
    result
}

fn exec_singbox(subcommand: &str, args: &[String]) -> anyhow::Result<std::process::ExitStatus> {
    Command::new("sing-box")
        .arg(subcommand)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("failed to execute sing-box")
}

fn extract_proxy_addr(config: &serde_json::Value) -> Option<String> {
    let inbounds = config.get("inbounds")?.as_array()?;
    for inbound in inbounds {
        let inbound_type = inbound.get("type")?.as_str()?;
        if inbound_type == "mixed" || inbound_type == "http" {
            let listen = inbound
                .get("listen")
                .and_then(|v| v.as_str())
                .unwrap_or("127.0.0.1");
            let port = inbound.get("listen_port")?.as_u64()?;
            return Some(format!("{listen}:{port}"));
        }
    }
    None
}
