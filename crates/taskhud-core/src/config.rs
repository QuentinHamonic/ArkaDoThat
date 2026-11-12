use std::env;
use std::ffi::OsString;
use std::path::PathBuf;

use crate::TodoError;

/// Environment variable that overrides the project registry path.
pub const CONFIG_ENV: &str = "TASKHUD_CONFIG";

/// Project registry used by the overlay and, by default, by the connectors:
/// `%APPDATA%\TaskHUD\config.json` on Windows, `$XDG_CONFIG_HOME/TaskHUD/config.json`
/// or `~/.config/TaskHUD/config.json` elsewhere.
pub fn default_config_path() -> PathBuf {
    platform_config_path(
        cfg!(windows),
        env::var_os("APPDATA"),
        env::var_os("XDG_CONFIG_HOME"),
        env::var_os("HOME"),
    )
}

/// Registry path for a process: `--config <path>` first, then `TASKHUD_CONFIG`,
/// then [`default_config_path`].
pub fn config_path_from_args(
    args: impl IntoIterator<Item = OsString>,
) -> Result<PathBuf, TodoError> {
    let mut args = args.into_iter();
    let mut cli_override = None;
    while let Some(argument) = args.next() {
        if argument == "--config" {
            cli_override = Some(args.next().map(PathBuf::from).ok_or_else(|| {
                TodoError::InvalidArguments("--config expects a path to a JSON file".to_owned())
            })?);
            break;
        }
    }
    Ok(resolve_config_path(cli_override, env::var_os(CONFIG_ENV)))
}

fn resolve_config_path(cli_override: Option<PathBuf>, env_override: Option<OsString>) -> PathBuf {
    cli_override
        .or_else(|| env_override.map(PathBuf::from))
        .unwrap_or_else(default_config_path)
}

fn platform_config_path(
    windows: bool,
    appdata: Option<OsString>,
    xdg_config_home: Option<OsString>,
    home: Option<OsString>,
) -> PathBuf {
    let base = if windows {
        appdata.map(PathBuf::from)
    } else {
        xdg_config_home
            .map(PathBuf::from)
            .or_else(|| home.map(|home| PathBuf::from(home).join(".config")))
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("TaskHUD")
        .join("config.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_paths_follow_each_os_convention() {
        let os = |value: &str| Some(OsString::from(value));
        assert_eq!(
            platform_config_path(true, os("C:/Users/me/AppData/Roaming"), None, None),
            PathBuf::from("C:/Users/me/AppData/Roaming/TaskHUD/config.json")
        );
        assert_eq!(
            platform_config_path(false, None, os("/xdg"), os("/home/me")),
            PathBuf::from("/xdg/TaskHUD/config.json")
        );
        assert_eq!(
            platform_config_path(false, None, None, os("/home/me")),
            PathBuf::from("/home/me/.config/TaskHUD/config.json")
        );
    }

    #[test]
    fn overrides_take_precedence_over_the_default() {
        assert_eq!(
            resolve_config_path(Some("cli.json".into()), Some("env.json".into())),
            PathBuf::from("cli.json")
        );
        assert_eq!(
            resolve_config_path(None, Some("env.json".into())),
            PathBuf::from("env.json")
        );
        assert_eq!(resolve_config_path(None, None), default_config_path());
    }

    #[test]
    fn a_config_flag_without_a_path_is_rejected_in_english() {
        let error = config_path_from_args(["--config".into()]).unwrap_err();
        assert_eq!(
            error.to_string(),
            "invalid arguments: --config expects a path to a JSON file"
        );
        assert_eq!(
            config_path_from_args(["--config".into(), "a.json".into()]).unwrap(),
            PathBuf::from("a.json")
        );
    }
}
