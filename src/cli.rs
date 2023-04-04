use crate::installer::{self, ClientInstallation, LoaderVersion, MinecraftVersion};
use anyhow::Context;
use clap::{Parser, Subcommand};
use reqwest::Client;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(about, version)]
#[clap(propagate_version = true)]
pub struct Args {
    #[clap(subcommand)]
    pub subcommand: Option<Subcommands>,
    #[arg(short = 'm', long)]
    minecraft_version: Option<String>,
    #[arg(short = 'l', long)]
    loader_version: Option<String>,
}

#[derive(Subcommand)]
pub enum Subcommands {
    Client {
        #[arg(short, long, default_value_t = false)]
        profile: bool,
        #[arg(short = 'o', long)]
        install_dir: Option<PathBuf>,
    },
    Server {
        #[arg(short = 's', long)]
        create_scripts: bool,
        #[arg(short, long)]
        download_server: bool,
        #[arg(short = 'o', long)]
        install_dir: PathBuf,
    },
}

#[derive(Clone, PartialEq, Eq)]
pub enum MCVersionCLI {
    Stable,
    Snapshot,
    Custom(String),
}

#[derive(Clone, PartialEq, Eq)]
pub enum LoaderVersionCLI {
    Stable,
    Beta,
    Custom(String),
}

impl From<Option<String>> for MCVersionCLI {
    fn from(s: Option<String>) -> Self {
        if let Some(s) = s {
            match s.to_lowercase().as_ref() {
                "stable" => Self::Stable,
                "snapshot" => Self::Snapshot,
                _ => Self::Custom(s),
            }
        } else {
            Self::Stable
        }
    }
}

impl From<Option<String>> for LoaderVersionCLI {
    fn from(s: Option<String>) -> Self {
        if let Some(s) = s {
            match s.to_lowercase().as_ref() {
                "stable" => Self::Stable,
                "beta" => Self::Beta,
                _ => Self::Custom(s),
            }
        } else {
            Self::Stable
        }
    }
}

pub async fn cli(client: Client, args: Args) -> anyhow::Result<()> {
    let (minecraft_version, loader_version) = get_versions(
        client.clone(),
        args.minecraft_version.into(),
        args.loader_version.into(),
    )
    .await?;

    match args.subcommand.unwrap() {
        Subcommands::Client {
            profile,
            install_dir,
        } => {
            installer::install_client(
                client,
                ClientInstallation {
                    minecraft_version,
                    loader_version,
                    install_dir: install_dir
                        .unwrap_or_else(installer::get_default_client_directory),
                    generate_profile: profile,
                },
            )
            .await
        }
        Subcommands::Server { .. } => unimplemented!(),
    }
}

async fn get_versions(
    client: Client,
    minecraft_version: MCVersionCLI,
    loader_version: LoaderVersionCLI,
) -> anyhow::Result<(MinecraftVersion, LoaderVersion)> {
    let minecraft_versions = installer::fetch_minecraft_versions(client.clone()).await?;
    let loader_versions = installer::fetch_loader_versions(client).await?;

    Ok((
        match minecraft_version {
            MCVersionCLI::Stable => minecraft_versions.into_iter().find(|v| v.stable).unwrap(),
            MCVersionCLI::Snapshot => minecraft_versions.into_iter().find(|v| !v.stable).unwrap(),
            MCVersionCLI::Custom(input) => minecraft_versions
                .into_iter()
                .find(|v| v.version == input)
                .context(format!("Could not find Minecraft version {}", input))?,
        },
        match loader_version {
            LoaderVersionCLI::Stable => loader_versions
                .into_iter()
                .find(|v| v.version.pre.is_empty())
                .unwrap(),
            LoaderVersionCLI::Beta => loader_versions
                .into_iter()
                .find(|v| !v.version.pre.is_empty())
                .unwrap(),
            LoaderVersionCLI::Custom(input) => loader_versions
                .into_iter()
                .find(|v| v.version.to_string() == input)
                .context(format!("Could not find Quilt Loader version {}", input))?,
        },
    ))
}
