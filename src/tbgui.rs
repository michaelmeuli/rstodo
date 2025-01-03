use crate::types::LoadError;
use crate::types::{Message, Screen, State};
use crate::utils::*;
use crate::views::*;
use crate::config::TbguiConfig;
use iced::futures::TryFutureExt;
use iced::keyboard;
use iced::widget;
use iced::window;
use iced::{Element, Subscription, Task};

#[derive(Debug)]
pub enum Tbgui {
    Loading,
    Loaded(State),
}

impl Tbgui {
    pub fn new() -> (Self, Task<Message>) {
        let cfg = async { confy::load("tbgui", None) };
        (
            Self::Loading,
            Task::perform(
                cfg.map_err(|e| LoadError {
                    error: e.to_string(),
                }),
                Message::Loaded,
            ),
        )
    }

    pub fn title(&self) -> String {
        "TbGUI - IMM".to_string()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match self {
            Tbgui::Loading => {
                match message {
                    Message::Loaded(Ok(state)) => {
                        *self = Tbgui::Loaded(State {
                            config: state,
                            ..State::default()
                        });
                    }
                    Message::Loaded(Err(_)) => {
                        *self = Tbgui::Loaded(State::default());
                    }
                    _ => {}
                }
                Task::done(Message::LoadRemoteState)
            }
            Tbgui::Loaded(state) => {
                let command = match message {
                    Message::LoadRemoteState => {
                        let config = state.config.clone();
                        Task::perform(
                            async move { load(&config).await },
                            Message::LoadedRemoteState,
                        )
                    }
                    Message::LoadedRemoteState(result) => match result {
                        Ok(remote_state) => {
                            state.items = remote_state.items;
                            state.client = Some(remote_state.client);
                            Task::none()
                        }
                        Err(e) => {
                            state.error_message = Some(e.error);
                            Task::none()
                        }
                    },
                    Message::FilterChanged(filter) => {
                        state.filter = filter;

                        Task::none()
                    }
                    Message::Item(i, item_message) => {
                        if let Some(item) = state.items.get_mut(i) {
                            item.update(item_message);
                            Task::none()
                        } else {
                            Task::none()
                        }
                    }
                    Message::TabPressed { shift } => {
                        if shift {
                            widget::focus_previous()
                        } else {
                            widget::focus_next()
                        }
                    }
                    Message::ToggleFullscreen(mode) => window::get_latest()
                        .and_then(move |window| window::change_mode(window, mode)),
                    Message::Loaded(_) => Task::none(),
                    Message::RunTbProfiler => {
                        let items_checked =
                            state.items.iter().filter(|item| item.is_checked).count();
                        let samples = state
                            .items
                            .iter()
                            .filter(|item| item.is_checked)
                            .map(|item| item.sample.clone())
                            .collect::<Vec<String>>()
                            .join(",");
                        println!("Running TB-Profiler for samples: {}", samples);
                        let client = state.client.clone();
                        let config = state.config.clone();
                        Task::perform(
                            async move {
                                if let Some(client) = client {
                                    if let Err(e) =
                                        run_tbprofiler(&client, items_checked, samples, &config)
                                            .await
                                    {
                                        println!("Error running tbprofiler: {:?}", e);
                                    }
                                }
                            },
                            |_| Message::ProfilerRunCompleted,
                        )
                    }
                    Message::DownloadResults => {
                        let client = state.client.clone();
                        let config = state.config.clone();
                        Task::perform(
                            async move {
                                if let Some(client) = client {
                                    if let Err(e) = download_results(&client, &config).await {
                                        println!("Error downloading results: {:?}", e);
                                    }
                                }
                            },
                            |_| Message::DownloadedResults,
                        )
                    }
                    Message::DeleteResults => {
                        let client = state.client.clone();
                        let config = state.config.clone();
                        Task::perform(
                            async move {
                                if let Some(client) = client {
                                    if let Err(e) = delete_results(&client, &config).await {
                                        println!("Error deleting results: {:?}", e);
                                    }
                                }
                            },
                            |_| Message::DeletedResults,
                        )
                    }
                    Message::SettingsPressed => {
                        state.screen = Screen::Settings;
                        Task::none()
                    }
                    Message::HomePressed => {
                        state.screen = Screen::Home;
                        Task::none()
                    }
                    Message::DownloadDefaultTemplate => {
                        let client = state.client.clone();
                        let config = state.config.clone();
                        Task::perform(
                            async move {
                                if let Some(client) = client {
                                    if let Err(e) =
                                        download_default_template(&client, &config).await
                                    {
                                        println!("Error downloading default template: {:?}", e);
                                    }
                                }
                            },
                            |_| Message::DownloadedDefaultTemplate,
                        )
                    }
                    Message::UploadUserTemplate => {
                        let client = state.client.clone();
                        let config = state.config.clone();
                        Task::perform(
                            async move {
                                if let Some(client) = client {
                                    if let Err(e) = upload_user_template(&client, &config).await {
                                        println!("Error uploading user template: {:?}", e);
                                    }
                                }
                            },
                            |_| Message::UploadedUserTemplate,
                        )
                    }
                    Message::ProfilerRunCompleted => Task::none(),
                    Message::DownloadedResults => Task::none(),
                    Message::DeletedResults => Task::none(),
                    Message::DownloadedDefaultTemplate => Task::none(),
                    Message::UploadedUserTemplate => Task::none(),
                    Message::ConfigPressed => {
                        state.screen = Screen::Config;
                        Task::none()
                    }

                    Message::ConfigNameChanged(username) => {
                        state.config.username = username;
                        Task::none()
                    }
                    Message::ConfigNameSubmitted => {
                        let config = TbguiConfig {
                            username: state.config.username.clone(),
                            ..state.config.clone()
                        };
                        confy::store("tbgui", None, &config).unwrap();
                        Task::none()
                    }

                    Message::ConfigRawDirChanged(remote_raw_dir) => {
                        state.config.remote_raw_dir = remote_raw_dir;
                        Task::none()
                    }
                    Message::ConfigRawDirSubmitted => {
                        let config = TbguiConfig {
                            remote_raw_dir: state.config.remote_raw_dir.clone(),
                            ..state.config.clone()
                        };
                        confy::store("tbgui", None, &config).unwrap();
                        Task::none()
                    }

                    Message::ConfigScriptPathChanged(tb_profiler_script) => {
                        state.config.tb_profiler_script = tb_profiler_script;
                        Task::none()
                    }
                    Message::ConfigScriptPathSubmitted => {
                        let config = TbguiConfig {
                            tb_profiler_script: state.config.tb_profiler_script.clone(),
                            ..state.config.clone()
                        };
                        confy::store("tbgui", None, &config).unwrap();
                        Task::none()
                    }

                    Message::ConfigResultsPathChanged(remote_results_dir) => {
                        state.config.remote_results_dir = remote_results_dir;
                        Task::none()
                    }
                    Message::ConfigResultsPathSubmitted => {
                        let config = TbguiConfig {
                            remote_results_dir: state.config.remote_results_dir.clone(),
                            ..state.config.clone()
                        };
                        confy::store("tbgui", None, &config).unwrap();
                        Task::none()
                    }

                    Message::ConfigDefaultTemplateChanged(default_template_remote) => {
                        state.config.default_template_remote = default_template_remote;
                        Task::none()
                    }
                    Message::ConfigDefaultTemplateSubmitted => {
                        let config = TbguiConfig {
                            default_template_remote: state.config.default_template_remote.clone(),
                            ..state.config.clone()
                        };
                        confy::store("tbgui", None, &config).unwrap();
                        Task::none()
                    }

                    Message::ConfigUserTemplateChanged(user_template_remote) => {
                        state.config.user_template_remote = user_template_remote;
                        Task::none()
                    }
                    Message::ConfigUserTemplateSubmitted => {
                        let config = TbguiConfig {
                            user_template_remote: state.config.user_template_remote.clone(),
                            ..state.config.clone()
                        };
                        confy::store("tbgui", None, &config).unwrap();
                        Task::none()
                    }
                    Message::ResetConfig => {
                        let config = TbguiConfig::default();
                        confy::store("tbgui", None, &config).unwrap();
                        state.config = config;
                        Task::none()
                    }
                };
                command
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        match self {
            Tbgui::Loading => loading_message(),
            Tbgui::Loaded(State {
                screen,
                filter,
                items,
                error_message,
                config,
                ..
            }) => match screen {
                Screen::Home => view_home(filter, items, error_message),
                Screen::Settings => view_settings(),
                Screen::Config => view_config(config),
            },
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        use keyboard::key;

        //let keyboard_subscription = keyboard::on_key_press(|key, modifiers| {
        keyboard::on_key_press(|key, modifiers| {
            let keyboard::Key::Named(key) = key else {
                return None;
            };

            match (key, modifiers) {
                (key::Named::Tab, _) => Some(Message::TabPressed {
                    shift: modifiers.shift(),
                }),
                (key::Named::ArrowUp, keyboard::Modifiers::SHIFT) => {
                    Some(Message::ToggleFullscreen(window::Mode::Fullscreen))
                }
                (key::Named::ArrowDown, keyboard::Modifiers::SHIFT) => {
                    Some(Message::ToggleFullscreen(window::Mode::Windowed))
                }
                _ => None,
            }
        })
        //let periodic_subscription = time::every(Duration::from_secs(9 * 60)).map(|_| Message::DownloadResults);
        //Subscription::batch(vec![keyboard_subscription, periodic_subscription])
    }
}
