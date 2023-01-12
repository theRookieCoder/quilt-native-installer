use std::borrow::Cow;
use std::fmt::Debug;
use std::path::PathBuf;

use anyhow::{anyhow, Error, Result};
use iced::widget::{Radio, Space};
use iced::window::{Icon, Settings as WindowSettings};
use iced::Theme;
use iced::{
    alignment::{Alignment, Horizontal},
    executor,
    widget::{Button, Checkbox, Column, PickList, ProgressBar, Row, Rule, Text, TextInput},
    Application, Command, Element, Length, Settings,
};
use native_dialog::FileDialog;
use png::Transformations;

use crate::installer::{
    fetch_loader_versions, fetch_minecraft_versions, install_client, install_server,
    ClientInstallation, Installation, LoaderVersion, MinecraftVersion, ServerInstallation,
};

pub fn run() -> Result<()> {
    State::run(Settings {
        window: WindowSettings {
            size: (600, 300),
            resizable: false,
            icon: Some(create_icon()?),
            ..Default::default()
        },
        ..Default::default()
    })?;

    Ok(())
}

fn create_icon() -> Result<Icon> {
    let mut decoder = png::Decoder::new(crate::ICON);
    decoder.set_transformations(Transformations::EXPAND);
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer)?;
    let bytes = &buffer[..info.buffer_size()];
    let icon = Icon::from_rgba(bytes.to_vec(), info.width, info.height)?;
    Ok(icon)
}

#[derive(Debug, Default)]
struct State {
    // Minecraft version picker
    minecraft_versions: Vec<MinecraftVersion>,
    selected_minecraft_version: Option<MinecraftVersion>,
    show_snapshots: bool,

    // Quilt Loader version picker
    loader_versions: Vec<LoaderVersion>,
    selected_loader_version: Option<LoaderVersion>,
    show_betas: bool,

    installation_type: Installation,

    // Client settings
    client_location: PathBuf,
    generate_profile: bool,

    // Server settings
    server_location: PathBuf,
    download_server_jar: bool,
    generate_launch_script: bool,

    // Progress information
    is_installing: bool,
    progress: f32,
}

#[derive(Debug)]
enum Message {
    Interaction(Interaction),
    Install,
    BrowseClientLocation,
    BrowseServerLocation,
    SetMcVersions(Result<Vec<MinecraftVersion>>),
    SetLoaderVersions(Result<Vec<LoaderVersion>>),
    DoneInstalling(Result<()>),
    Error(Error),
}

#[derive(Debug, Clone)]
enum Interaction {
    ChangeClientLocation(PathBuf),
    BrowseClientLocation,
    Install,
    SelectInstallation(Installation),
    SelectLoaderVersion(LoaderVersion),
    SelectMcVersion(MinecraftVersion),
    SetShowSnapshots(bool),
    SetShowBetas(bool),
    GenerateLaunchScript(bool),
    GenerateProfile(bool),
    ChangeServerLocation(PathBuf),
    BrowseServerLocation,
    DownloadServerJar(bool),
}

impl From<Message> for Command<Message> {
    fn from(m: Message) -> Self {
        Self::perform(async { m }, |t| t)
    }
}

#[cfg(target_os = "windows")]
fn get_default_client_directory() -> PathBuf {
    PathBuf::from(std::env::var("APPDATA").unwrap()).join(".minecraft")
}

#[cfg(target_os = "macos")]
fn get_default_client_directory() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap())
        .join("Library")
        .join("Application Support")
        .join("minecraft")
}

#[cfg(target_os = "linux")]
fn get_default_client_directory() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap()).join(".minecraft")
}

impl Application for State {
    type Message = Message;
    type Executor = executor::Default;
    type Flags = ();
    type Theme = Theme;

    fn theme(&self) -> Self::Theme {
        use dark_light::Mode;
        match dark_light::detect() {
            Mode::Light => Theme::Light,
            Mode::Dark | Mode::Default => Theme::Dark,
        }
    }

    fn new(_: ()) -> (Self, Command<Self::Message>) {
        (
            State {
                client_location: get_default_client_directory(),
                generate_profile: true,
                server_location: std::env::current_dir().unwrap_or_default(),
                download_server_jar: true,
                generate_launch_script: true,
                ..Default::default()
            },
            Command::batch([
                Command::perform(fetch_minecraft_versions(), Message::SetMcVersions),
                Command::perform(fetch_loader_versions(), Message::SetLoaderVersions),
            ]),
        )
    }

    fn title(&self) -> String {
        "Quilt Installer".into()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Interaction(interaction) => match interaction {
                Interaction::ChangeClientLocation(location) => self.client_location = location,
                Interaction::BrowseClientLocation => return Message::BrowseClientLocation.into(),
                Interaction::Install => return Message::Install.into(),
                Interaction::SelectInstallation(i) => self.installation_type = i,
                Interaction::SelectLoaderVersion(v) => self.selected_loader_version = Some(v),
                Interaction::SelectMcVersion(v) => self.selected_minecraft_version = Some(v),
                Interaction::SetShowSnapshots(enable) => {
                    self.show_snapshots = enable;
                    self.selected_minecraft_version = self
                        .minecraft_versions
                        .iter()
                        .find(|v| enable || v.stable)
                        .cloned();
                }
                Interaction::SetShowBetas(enable) => {
                    self.show_betas = enable;
                    self.selected_loader_version = self
                        .loader_versions
                        .iter()
                        .find(|v| enable || v.version.pre.is_empty())
                        .cloned();
                }
                Interaction::GenerateLaunchScript(value) => self.generate_launch_script = value,
                Interaction::GenerateProfile(value) => self.generate_profile = value,
                Interaction::ChangeServerLocation(location) => self.server_location = location,
                Interaction::BrowseServerLocation => return Message::BrowseServerLocation.into(),
                Interaction::DownloadServerJar(value) => self.download_server_jar = value,
            },
            Message::SetMcVersions(result) => {
                match result {
                    Ok(versions) => self.minecraft_versions = versions,
                    Err(error) => return Message::Error(error).into(),
                }
                if self.selected_minecraft_version.is_none() {
                    self.selected_minecraft_version =
                        self.minecraft_versions.iter().find(|v| v.stable).cloned();
                }
            }
            Message::SetLoaderVersions(result) => {
                match result {
                    Ok(versions) => self.loader_versions = versions,
                    Err(error) => return Message::Error(error).into(),
                }
                if self.selected_loader_version.is_none() {
                    self.selected_loader_version = self
                        .loader_versions
                        .iter()
                        .find(|v| v.version.pre.is_empty())
                        .cloned();
                }
            }
            Message::BrowseClientLocation => {
                let mut dialog = FileDialog::new();
                let working_dir = std::env::current_dir();
                if self.client_location.is_dir() {
                    dialog = dialog.set_location(&self.client_location);
                } else if working_dir.is_ok() {
                    dialog = dialog.set_location(working_dir.as_deref().unwrap())
                }
                let result = dialog.show_open_single_dir();
                match result {
                    Ok(Some(path)) => self.client_location = path,
                    Ok(None) => (),
                    Err(error) => return Message::Error(error.into()).into(),
                }
            }
            Message::BrowseServerLocation => {
                let mut dialog = FileDialog::new();
                let working_dir = std::env::current_dir();
                if self.client_location.is_dir() {
                    dialog = dialog.set_location(&self.server_location);
                } else if working_dir.is_ok() {
                    dialog = dialog.set_location(working_dir.as_deref().unwrap())
                }
                let result = dialog.show_open_single_dir();
                match result {
                    Ok(Some(path)) => self.server_location = path,
                    Ok(None) => (),
                    Err(error) => return Message::Error(error.into()).into(),
                }
            }
            Message::Install => {
                self.is_installing = true;
                self.progress = 0.0;

                return match self.installation_type {
                    Installation::Client => Command::perform(
                        install_client(ClientInstallation {
                            minecraft_version: match &self.selected_minecraft_version {
                                Some(s) => s.clone(),
                                None => {
                                    return Message::Error(anyhow!(
                                        "No Minecraft version selected!"
                                    ))
                                    .into()
                                }
                            },
                            loader_version: match &self.selected_loader_version {
                                Some(s) => s.clone(),
                                None => {
                                    return Message::Error(anyhow!("No Loader version selected!"))
                                        .into()
                                }
                            },
                            install_location: self.client_location.clone(),
                            generate_profile: self.generate_profile,
                        }),
                        Message::DoneInstalling,
                    ),
                    Installation::Server => Command::perform(
                        install_server(ServerInstallation {
                            minecraft_version: match &self.selected_minecraft_version {
                                Some(s) => s.clone(),
                                None => {
                                    return Message::Error(anyhow!(
                                        "No Minecraft version selected!"
                                    ))
                                    .into()
                                }
                            },
                            loader_version: match &self.selected_loader_version {
                                Some(s) => s.clone(),
                                None => {
                                    return Message::Error(anyhow!("No Loader version selected!"))
                                        .into()
                                }
                            },
                            install_location: self.server_location.clone(),
                            download_jar: self.download_server_jar,
                            generate_script: self.generate_launch_script,
                        }),
                        Message::DoneInstalling,
                    ),
                };
            }
            Message::DoneInstalling(res) => {
                self.is_installing = false;
                self.progress = 1.0;

                if let Err(e) = res {
                    return Message::Error(e).into();
                }
            }
            Message::Error(error) => {
                eprintln!("{:?}", error);
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        Element::<Interaction>::from(
            Column::new()
                .padding(5)
                .spacing(5)
                .push(
                    Row::new()
                        .push(Text::new("Installation:").width(140.into()))
                        .push(Radio::new(
                            Installation::Client,
                            "Client",
                            Some(self.installation_type),
                            Interaction::SelectInstallation,
                        ))
                        .push(Radio::new(
                            Installation::Server,
                            "Server",
                            Some(self.installation_type),
                            Interaction::SelectInstallation,
                        ))
                        .width(Length::Fill)
                        .align_items(Alignment::Fill)
                        .spacing(50)
                        .padding(5),
                )
                .push(
                    Row::new()
                        .push(Text::new("Minecraft version:").width(140.into()))
                        .push(
                            PickList::new(
                                Cow::from_iter(
                                    self.minecraft_versions
                                        .iter()
                                        .filter(|v| self.show_snapshots || v.stable)
                                        .cloned(),
                                ),
                                self.selected_minecraft_version.clone(),
                                Interaction::SelectMcVersion,
                            )
                            .width(200.into()),
                        )
                        .push(Space::new(20.into(), 0.into()))
                        .push(Checkbox::new(
                            self.show_snapshots,
                            "Show snapshots",
                            Interaction::SetShowSnapshots,
                        ))
                        .width(Length::Fill)
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .padding(5),
                )
                .push(
                    Row::new()
                        .push(Text::new("Loader version:").width(140.into()))
                        .push(
                            PickList::new(
                                Cow::from_iter(
                                    self.loader_versions
                                        .iter()
                                        .filter(|v| self.show_betas || v.version.pre.is_empty())
                                        .cloned(),
                                ),
                                self.selected_loader_version.clone(),
                                Interaction::SelectLoaderVersion,
                            )
                            .width(200.into()),
                        )
                        .push(Space::new(20.into(), 0.into()))
                        .push(Checkbox::new(
                            self.show_betas,
                            "Show betas",
                            Interaction::SetShowBetas,
                        ))
                        .width(Length::Fill)
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .padding(5),
                )
                .push(Rule::horizontal(5))
                .push(match self.installation_type {
                    Installation::Client => Row::new()
                        .push(Text::new("Directory:").width(140.into()))
                        .push(
                            TextInput::new(
                                "Install location",
                                self.client_location.to_str().unwrap(),
                                |s| Interaction::ChangeClientLocation(PathBuf::from(s)),
                            )
                            .padding(5),
                        )
                        .push(
                            Button::new(Text::new("Browse"))
                                .on_press(Interaction::BrowseClientLocation),
                        )
                        .width(Length::Fill)
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .padding(5),
                    Installation::Server => Row::new()
                        .push(Text::new("Directory:").width(140.into()))
                        .push(
                            TextInput::new(
                                "Install location",
                                self.server_location.to_str().unwrap(),
                                |s| Interaction::ChangeServerLocation(PathBuf::from(s)),
                            )
                            .padding(5),
                        )
                        .push(
                            Button::new(Text::new("Browse"))
                                .on_press(Interaction::BrowseServerLocation),
                        )
                        .width(Length::Fill)
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .padding(5),
                })
                .push(match self.installation_type {
                    Installation::Client => Row::new()
                        .push(Text::new("Options:").width(140.into()))
                        .push(Checkbox::new(
                            self.generate_profile,
                            "Generate profile",
                            Interaction::GenerateProfile,
                        ))
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .padding(5),
                    Installation::Server => Row::new()
                        .push(Text::new("Options:").width(140.into()))
                        .push(Checkbox::new(
                            self.download_server_jar,
                            "Download server jar",
                            Interaction::DownloadServerJar,
                        ))
                        .push(Space::new(35.into(), 0.into()))
                        .push(Checkbox::new(
                            self.generate_launch_script,
                            "Generate launch script",
                            Interaction::GenerateLaunchScript,
                        ))
                        .align_items(Alignment::Center)
                        .spacing(5)
                        .padding(5),
                })
                .push({
                    let mut button = Button::new(
                        Text::new("Install")
                            .horizontal_alignment(Horizontal::Center)
                            .width(Length::Fill),
                    )
                    .width(Length::Fill);
                    if !self.is_installing {
                        button = button.on_press(Interaction::Install);
                    }
                    button
                })
                .push(ProgressBar::new(0.0..=1.0, self.progress)),
        )
        .map(Message::Interaction)
    }
}
