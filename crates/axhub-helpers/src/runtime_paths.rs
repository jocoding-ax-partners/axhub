use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePaths {
    pub token_file: PathBuf,
    pub last_deploy_file: PathBuf,
    pub state_dir: PathBuf,
}

impl RuntimePaths {
    pub fn current() -> Option<Self> {
        Some(Self {
            token_file: token_file()?,
            last_deploy_file: last_deploy_file()?,
            state_dir: state_dir()?,
        })
    }
}

pub fn token_file() -> Option<PathBuf> {
    config_base_dir().map(|base| base.join("axhub-plugin").join("token"))
}

pub fn last_deploy_file() -> Option<PathBuf> {
    cache_base_dir().map(|base| base.join("axhub-plugin").join("last-deploy.json"))
}

pub fn state_dir() -> Option<PathBuf> {
    state_base_dir().map(|base| base.join("axhub-plugin"))
}

fn config_base_dir() -> Option<PathBuf> {
    config_base_dir_from(env_path("XDG_CONFIG_HOME"), home_dir())
}

fn cache_base_dir() -> Option<PathBuf> {
    cache_base_dir_from(env_path("XDG_CACHE_HOME"), home_dir())
}

fn state_base_dir() -> Option<PathBuf> {
    state_base_dir_from(env_path("XDG_STATE_HOME"), home_dir())
}

fn config_base_dir_from(
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Option<PathBuf> {
    xdg_config_home.or_else(|| home.map(|home| home.join(".config")))
}

fn cache_base_dir_from(xdg_cache_home: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    xdg_cache_home.or_else(|| home.map(|home| home.join(".cache")))
}

fn state_base_dir_from(xdg_state_home: Option<PathBuf>, home: Option<PathBuf>) -> Option<PathBuf> {
    xdg_state_home.or_else(|| home.map(|home| home.join(".local").join("state")))
}

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn home_dir() -> Option<PathBuf> {
    env_path("HOME")
        .or_else(|| env_path("USERPROFILE"))
        .or_else(|| {
            let drive = env::var_os("HOMEDRIVE")?;
            let path = env::var_os("HOMEPATH")?;
            let mut home = PathBuf::from(drive);
            home.push(path);
            Some(home)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_paths_override_home_contracts() {
        assert_eq!(
            config_base_dir_from(
                Some(PathBuf::from("/xdg-config")),
                Some(PathBuf::from("/home/user"))
            ),
            Some(PathBuf::from("/xdg-config"))
        );
        assert_eq!(
            cache_base_dir_from(
                Some(PathBuf::from("/xdg-cache")),
                Some(PathBuf::from("/home/user"))
            ),
            Some(PathBuf::from("/xdg-cache"))
        );
        assert_eq!(
            state_base_dir_from(
                Some(PathBuf::from("/xdg-state")),
                Some(PathBuf::from("/home/user"))
            ),
            Some(PathBuf::from("/xdg-state"))
        );
    }

    #[test]
    fn home_fallback_matches_plugin_paths_on_unix_and_windows() {
        assert_eq!(
            config_base_dir_from(None, Some(PathBuf::from("/home/user"))),
            Some(PathBuf::from("/home/user/.config"))
        );
        assert_eq!(
            cache_base_dir_from(None, Some(PathBuf::from("/home/user"))),
            Some(PathBuf::from("/home/user/.cache"))
        );
        assert_eq!(
            state_base_dir_from(None, Some(PathBuf::from("/home/user"))),
            Some(PathBuf::from("/home/user/.local/state"))
        );
    }
}
