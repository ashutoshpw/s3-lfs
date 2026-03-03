use std::collections::HashSet;
use std::io::{self, IsTerminal, Write};

use anyhow::{anyhow, Context, Result};
use clap::{error::ErrorKind, Args, Parser, Subcommand};

use crate::config::{RuntimeConfig, DEFAULT_COMPRESSION};
use crate::profiles::{
    delete, list, load, profile_from_runtime, runtime_from_profile, runtime_from_profile_obj, save,
    validate_slug,
};
use crate::repo_config::{apply_repo_overrides, resolve_repo_config};
use crate::service;
use crate::usage::print_usage;

const CONFIG_FLAGS: [&str; 9] = [
    "access_key_id",
    "secret_access_key",
    "bucket",
    "endpoint",
    "region",
    "root_path",
    "use_path_style",
    "delete_other_versions",
    "compression",
];

#[derive(Parser, Debug)]
#[command(name = "s3-lfs")]
#[command(about = "S3-backed Git LFS custom transfer agent")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[command(flatten)]
    transfer: TransferArgs,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(
        about = "Create or edit S3 profiles. Without --profile, opens interactive profile selection."
    )]
    Setup(SetupArgs),
    Profile(ProfileArgs),
}

#[derive(Args, Clone, Debug, Default)]
struct TransferArgs {
    #[arg(
        long = "profile",
        help = "Profile slug. In `setup`, omit to choose/edit profiles interactively."
    )]
    profile: Option<String>,
    #[arg(long = "access_key_id")]
    access_key_id: Option<String>,
    #[arg(long = "secret_access_key")]
    secret_access_key: Option<String>,
    #[arg(long = "bucket")]
    bucket: Option<String>,
    #[arg(long = "endpoint")]
    endpoint: Option<String>,
    #[arg(long = "region")]
    region: Option<String>,
    #[arg(long = "root_path")]
    root_path: Option<String>,
    #[arg(long = "use_path_style", num_args = 0..=1, default_missing_value = "true")]
    use_path_style: Option<bool>,
    #[arg(
        long = "delete_other_versions",
        num_args = 0..=1,
        default_missing_value = "true"
    )]
    delete_other_versions: Option<bool>,
    #[arg(long = "compression")]
    compression: Option<String>,
}

#[derive(Args, Debug)]
struct SetupArgs {
    #[command(flatten)]
    transfer: TransferArgs,
}

#[derive(Args, Debug)]
struct ProfileArgs {
    #[command(subcommand)]
    command: ProfileCommand,
}

#[derive(Subcommand, Debug)]
enum ProfileCommand {
    List,
    Show {
        #[arg(long = "profile")]
        profile: String,
    },
    Delete {
        #[arg(long = "profile")]
        profile: String,
    },
}

pub fn run(raw_args: &[String]) -> Result<()> {
    let normalized = normalize_single_dash_args(raw_args);

    if let Some(first) = normalized.first() {
        if first == "-h" || first == "--help" || first == "help" {
            print_usage(&mut io::stdout())?;
            return Ok(());
        }
    }

    let argv = std::iter::once("s3-lfs".to_string()).chain(normalized.clone());
    let cli = match Cli::try_parse_from(argv) {
        Ok(cli) => cli,
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                err.print().map_err(|e| anyhow!(e.to_string()))?;
                return Ok(());
            }
            _ => return Err(anyhow!(err.to_string())),
        },
    };

    match cli.command {
        Some(Commands::Setup(setup)) => run_setup(&setup.transfer, &normalized),
        Some(Commands::Profile(profile)) => run_profile(profile),
        None => run_transfer_agent(&cli.transfer, &normalized),
    }
}

fn run_transfer_agent(parsed: &TransferArgs, raw_args: &[String]) -> Result<()> {
    let mut resolved = RuntimeConfig::default();

    if let Some(slug) = &parsed.profile {
        resolved =
            runtime_from_profile(slug).with_context(|| format!("load profile \"{}\"", slug))?;
    }

    let explicit = explicit_flags(raw_args);

    let cwd = std::env::current_dir()?;
    match resolve_repo_config(&cwd) {
        Ok(repo_cfg) => apply_repo_overrides(&mut resolved, &repo_cfg, &explicit),
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => return Err(anyhow!("load repo .lfsconfig: {}", err)),
    }

    apply_env_overrides(&mut resolved);
    apply_cli_overrides(&mut resolved, parsed);

    if resolved.compression.is_empty() {
        resolved.compression = DEFAULT_COMPRESSION.to_string();
    }
    if resolved.profile.is_empty() {
        if let Some(profile) = &parsed.profile {
            resolved.profile = profile.clone();
        }
    }

    resolved.validate()?;

    let mut stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    service::serve(&mut stdin, &mut stdout, &mut stderr, &resolved)
}

fn run_setup(parsed: &TransferArgs, raw_args: &[String]) -> Result<()> {
    let is_tty = io::stdin().is_terminal();
    let slug = match parsed.profile.as_deref() {
        Some(slug) => {
            validate_slug(slug)?;
            slug.to_string()
        }
        None => {
            if !is_tty {
                return Err(anyhow!(
                    "--profile is required when stdin is not a terminal"
                ));
            }
            select_profile_for_setup()?
        }
    };

    let mut current = match load(&slug) {
        Ok(profile) => runtime_from_profile_obj(&slug, &profile),
        Err(err) if err.kind() == io::ErrorKind::NotFound => RuntimeConfig::with_profile(&slug),
        Err(err) => return Err(err.into()),
    };

    apply_cli_overrides(&mut current, parsed);

    let explicit = explicit_flags(raw_args);
    let has_config_flag = CONFIG_FLAGS.iter().any(|name| explicit.contains(*name));

    if !has_config_flag {
        if !is_tty {
            return Err(anyhow!(
                "interactive setup requires a terminal; pass flags for non-interactive setup"
            ));
        }

        current.access_key_id =
            prompt("S3 access key ID (optional)", &current.access_key_id, false)?;
        current.secret_access_key = prompt(
            "S3 secret access key (optional)",
            &current.secret_access_key,
            false,
        )?;
        current.bucket = prompt("S3 bucket", &current.bucket, true)?;
        current.endpoint = prompt("S3 endpoint", &current.endpoint, true)?;
        current.region = prompt("S3 region", &current.region, true)?;
        current.root_path = prompt("Root path in bucket (optional)", &current.root_path, false)?;
        current.compression = prompt("Compression (zstd|gzip|none)", &current.compression, true)?;
        current.use_path_style = prompt_bool("Use path-style URLs", current.use_path_style)?;
        current.delete_other_versions = prompt_bool(
            "Delete files uploaded with other compression",
            current.delete_other_versions,
        )?;
    } else {
        if !is_tty
            && (current.bucket.is_empty()
                || current.endpoint.is_empty()
                || current.region.is_empty())
        {
            return Err(anyhow!(
                "bucket, endpoint, and region are required; pass them as flags or run interactive setup in a terminal"
            ));
        }

        if is_tty && current.bucket.is_empty() {
            current.bucket = prompt("S3 bucket", &current.bucket, true)?;
        }
        if is_tty && current.endpoint.is_empty() {
            current.endpoint = prompt("S3 endpoint", &current.endpoint, true)?;
        }
        if is_tty && current.region.is_empty() {
            current.region = prompt("S3 region", &current.region, true)?;
        }
    }

    current.validate()?;
    save(&slug, &profile_from_runtime(&current))?;
    println!("Saved profile \"{}\"", slug);

    Ok(())
}

fn select_profile_for_setup() -> Result<String> {
    let profiles = list()?;
    if profiles.is_empty() {
        println!("No existing profiles found. Creating a new profile.");
        return prompt_profile_slug();
    }

    loop {
        println!("Select a profile for setup:");
        println!("  1) Add new profile");
        for (index, slug) in profiles.iter().enumerate() {
            println!("  {}) Edit {}", index + 2, slug);
        }

        let choice = prompt("Choice", "1", true)?;
        let selection = match choice.parse::<usize>() {
            Ok(value) => value,
            Err(_) => {
                println!("Invalid choice \"{}\". Enter a number.", choice);
                continue;
            }
        };

        if selection == 1 {
            return prompt_profile_slug();
        }

        let profile_index = selection.saturating_sub(2);
        if profile_index < profiles.len() {
            return Ok(profiles[profile_index].clone());
        }

        println!(
            "Invalid choice {}. Enter a number between 1 and {}.",
            selection,
            profiles.len() + 1
        );
    }
}

fn prompt_profile_slug() -> Result<String> {
    loop {
        let slug = prompt("Profile slug", "", true)?;
        match validate_slug(&slug) {
            Ok(_) => return Ok(slug),
            Err(err) => println!("{}", err),
        }
    }
}

fn run_profile(parsed: ProfileArgs) -> Result<()> {
    match parsed.command {
        ProfileCommand::List => {
            for profile in list()? {
                println!("{}", profile);
            }
            Ok(())
        }
        ProfileCommand::Show { profile } => {
            let profile = load(&profile)?;
            println!("{}", serde_json::to_string_pretty(&profile)?);
            Ok(())
        }
        ProfileCommand::Delete { profile } => {
            delete(&profile)?;
            println!("Deleted profile \"{}\"", profile);
            Ok(())
        }
    }
}

fn normalize_single_dash_args(raw_args: &[String]) -> Vec<String> {
    let known: HashSet<&str> = HashSet::from([
        "profile",
        "access_key_id",
        "secret_access_key",
        "bucket",
        "endpoint",
        "region",
        "root_path",
        "use_path_style",
        "delete_other_versions",
        "compression",
    ]);

    raw_args
        .iter()
        .map(|arg| {
            if arg.starts_with("--") || arg == "-h" || arg == "-v" {
                return arg.clone();
            }
            if let Some(stripped) = arg.strip_prefix('-') {
                if stripped.is_empty() {
                    return arg.clone();
                }

                let (name, suffix) = if let Some((name, value)) = stripped.split_once('=') {
                    (name, format!("={}", value))
                } else {
                    (stripped, String::new())
                };

                if known.contains(name) {
                    return format!("--{}{}", name, suffix);
                }
            }
            arg.clone()
        })
        .collect()
}

fn explicit_flags(raw_args: &[String]) -> HashSet<String> {
    let mut set = HashSet::new();

    for arg in raw_args {
        if let Some(rest) = arg.strip_prefix("--") {
            let name = rest.split('=').next().unwrap_or(rest);
            if !name.is_empty() {
                set.insert(name.to_string());
            }
        }
    }

    set
}

fn apply_env_overrides(cfg: &mut RuntimeConfig) {
    if let Ok(value) = std::env::var("AWS_ACCESS_KEY_ID") {
        if !value.is_empty() {
            cfg.access_key_id = value;
        }
    }
    if let Ok(value) = std::env::var("AWS_SECRET_ACCESS_KEY") {
        if !value.is_empty() {
            cfg.secret_access_key = value;
        }
    }
    if let Ok(value) = std::env::var("S3_BUCKET") {
        if !value.is_empty() {
            cfg.bucket = value;
        }
    }
    if let Ok(value) = std::env::var("AWS_REGION") {
        if !value.is_empty() {
            cfg.region = value;
        }
    }
    if let Ok(value) = std::env::var("AWS_S3_ENDPOINT") {
        if !value.is_empty() {
            cfg.endpoint = value;
        }
    }
}

fn apply_cli_overrides(cfg: &mut RuntimeConfig, args: &TransferArgs) {
    if let Some(value) = &args.profile {
        cfg.profile = value.clone();
    }
    if let Some(value) = &args.access_key_id {
        cfg.access_key_id = value.clone();
    }
    if let Some(value) = &args.secret_access_key {
        cfg.secret_access_key = value.clone();
    }
    if let Some(value) = &args.bucket {
        cfg.bucket = value.clone();
    }
    if let Some(value) = &args.endpoint {
        cfg.endpoint = value.clone();
    }
    if let Some(value) = &args.region {
        cfg.region = value.clone();
    }
    if let Some(value) = &args.root_path {
        cfg.root_path = value.clone();
    }
    if let Some(value) = args.use_path_style {
        cfg.use_path_style = value;
    }
    if let Some(value) = args.delete_other_versions {
        cfg.delete_other_versions = value;
    }
    if let Some(value) = &args.compression {
        cfg.compression = value.clone();
    }
}

fn prompt(label: &str, current: &str, required: bool) -> Result<String> {
    let mut input = String::new();

    loop {
        if current.is_empty() {
            print!("{}: ", label);
        } else {
            print!("{} [{}]: ", label, current);
        }
        io::stdout().flush()?;

        input.clear();
        io::stdin().read_line(&mut input)?;
        let value = input.trim();

        if value.is_empty() {
            if !current.is_empty() {
                return Ok(current.to_string());
            }
            if required {
                println!("Value is required.");
                continue;
            }
            return Ok(String::new());
        }

        return Ok(value.to_string());
    }
}

fn prompt_bool(label: &str, current: bool) -> Result<bool> {
    let mut input = String::new();

    loop {
        let default_value = if current { "y" } else { "n" };
        print!("{} [y/n, default={}]: ", label, default_value);
        io::stdout().flush()?;

        input.clear();
        io::stdin().read_line(&mut input)?;
        let value = input.trim().to_ascii_lowercase();

        if value.is_empty() {
            return Ok(current);
        }

        match value.as_str() {
            "y" | "yes" | "true" | "1" => return Ok(true),
            "n" | "no" | "false" | "0" => return Ok(false),
            _ => println!("Please enter y or n."),
        }
    }
}
