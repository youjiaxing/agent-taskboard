use super::super::*;

#[derive(Debug, Clone)]
pub(crate) struct CachedAgentConfig {
    pub(crate) at_ms: u64,
    pub(crate) discovery: AgentConfigDiscovery,
    pub(crate) error: Option<AgentConfigFailure>,
}

#[derive(Debug, Clone)]
pub(crate) enum AgentConfigFailure {
    LaunchEnvironment(String),
    Missing {
        command: String,
        searched_path: String,
        known_locations: Vec<PathBuf>,
    },
    Cli(String),
}

impl AgentConfigFailure {
    pub(crate) fn message(&self, language: Language) -> String {
        let detail = match self {
            Self::LaunchEnvironment(error) | Self::Cli(error) => error.clone(),
            Self::Missing {
                command,
                searched_path,
                known_locations,
            } => launch::missing_agent_cli(command, searched_path, known_locations, language),
        };
        launch::option_discovery_failure(&detail, language)
    }
}

const AGENT_CONFIG_CACHE_MS: u64 = 5 * 60 * 1000;

impl HostKernel {
    pub(crate) fn agent_config_for(
        &mut self,
        cwd: &Path,
        agent: &dyn AgentPort,
        language: Language,
    ) -> (AgentConfigDiscovery, Option<String>) {
        let key = (cwd.to_path_buf(), agent.id().to_string());
        if let Some(cached) = self
            .agent_config_cache
            .get(&key)
            .filter(|cached| self.now_ms.saturating_sub(cached.at_ms) < AGENT_CONFIG_CACHE_MS)
        {
            return (
                cached.discovery.clone(),
                cached.error.as_ref().map(|error| error.message(language)),
            );
        }
        let fallback = || AgentConfigDiscovery {
            fields: agent.config_fields(),
            seed: agent.seed_config(),
        };
        let result = self
            .launch_env
            .capture(cwd)
            .map_err(AgentConfigFailure::LaunchEnvironment)
            .and_then(|env| {
                let prepared =
                    agent::prepare_launch_env(env, &[], &agent.known_install_locations());
                Ok((prepared.clone(), agent.probe(&prepared)))
            })
            .and_then(|(env, probe)| match probe {
                ProbeResult::Found { executable } => agent
                    .discover_config(&executable, &env)
                    .map_err(AgentConfigFailure::Cli),
                ProbeResult::Missing {
                    command,
                    searched_path,
                    known_locations,
                } => Err(AgentConfigFailure::Missing {
                    command,
                    searched_path,
                    known_locations,
                }),
            });
        let (discovery, error) = match result {
            Ok(discovery) => (discovery, None),
            Err(error) => (fallback(), Some(error)),
        };
        self.agent_config_cache.insert(
            key,
            CachedAgentConfig {
                at_ms: self.now_ms,
                discovery: discovery.clone(),
                error: error.clone(),
            },
        );
        (
            discovery,
            error.as_ref().map(|error| error.message(language)),
        )
    }
}
