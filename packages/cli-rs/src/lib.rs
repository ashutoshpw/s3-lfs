pub mod cli;
pub mod compression;
pub mod config;
pub mod profiles;
pub mod protocol;
pub mod repo_config;
pub mod s3_adapter;
pub mod service;
pub mod usage;

pub fn run(args: &[String]) -> anyhow::Result<()> {
    cli::run(args)
}
