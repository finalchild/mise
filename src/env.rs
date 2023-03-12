use std::collections::{HashMap, HashSet};
pub use std::env::*;
use std::path::PathBuf;

use itertools::Itertools;
use lazy_static::lazy_static;
use log::LevelFilter;

use crate::env_diff::{EnvDiff, EnvDiffOperation, EnvDiffPatches};

lazy_static! {
    pub static ref ARGS: Vec<String> = args().collect();

    // paths and directories
    pub static ref HOME: PathBuf = dirs_next::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    pub static ref PWD: PathBuf =current_dir().unwrap_or_else(|_| PathBuf::new());
    pub static ref XDG_CACHE_HOME: PathBuf = dirs_next::cache_dir().unwrap_or_else(|| HOME.join(".cache"));
    pub static ref XDG_DATA_HOME: PathBuf =var_path("XDG_DATA_HOME").unwrap_or_else(|| HOME.join(".local/share"));
    pub static ref XDG_CONFIG_HOME: PathBuf =var_path("XDG_CONFIG_HOME").unwrap_or_else(|| HOME.join(".config"));
    pub static ref RTX_CACHE_DIR: PathBuf =var_path("RTX_CACHE_DIR").unwrap_or_else(|| XDG_CACHE_HOME.join("rtx"));
    pub static ref RTX_CONFIG_DIR: PathBuf =var_path("RTX_CONFIG_DIR").unwrap_or_else(|| XDG_CONFIG_HOME.join("rtx"));
    pub static ref RTX_DATA_DIR: PathBuf = var_path("RTX_DATA_DIR").unwrap_or_else(|| XDG_DATA_HOME.join("rtx"));
    pub static ref RTX_DEFAULT_TOOL_VERSIONS_FILENAME: String =var("RTX_DEFAULT_TOOL_VERSIONS_FILENAME").unwrap_or_else(|_| ".tool-versions".into());
    pub static ref RTX_DEFAULT_CONFIG_FILENAME: String =var("RTX_DEFAULT_CONFIG_FILENAME").unwrap_or_else(|_| ".rtx.toml".into());
    pub static ref RTX_CONFIG_FILE: Option<PathBuf> = var_path("RTX_CONFIG_FILE");
    pub static ref RTX_USE_TOML: bool = var_is_true("RTX_USE_TOML");
    pub static ref RTX_TMP_DIR: PathBuf = temp_dir().join("rtx");
    pub static ref SHELL: String = var("SHELL").unwrap_or_else(|_| "sh".into());
    pub static ref RTX_EXE: PathBuf = current_exe().unwrap_or_else(|_| "rtx".into());

    // logging
    pub static ref RTX_LOG_LEVEL: log::LevelFilter = {
        ARGS.iter().enumerate().for_each(|(i, arg)| {
            if let Some(("--log-level", level)) = arg.split_once('=') {
                std::env::set_var("RTX_LOG_LEVEL", level);
            }
            if arg == "--log-level" {
                if let Some(level) = ARGS.get(i + 1) {
                    std::env::set_var("RTX_LOG_LEVEL", level);
                }
            }
        });
        let log_level = var("RTX_LOG_LEVEL").unwrap_or_default();
        match log_level.parse::<LevelFilter>() {
            Ok(level) => level,
            _ => {
                if *RTX_TRACE {
                    log::LevelFilter::Trace
                } else if *RTX_DEBUG {
                    log::LevelFilter::Debug
                } else if *RTX_QUIET {
                    log::LevelFilter::Warn
                } else {
                    log::LevelFilter::Info
                }
            }
        }
    };
    pub static ref RTX_LOG_FILE_LEVEL: log::LevelFilter = {
        let log_level = var("RTX_LOG_FILE_LEVEL").unwrap_or_default();
        match log_level.parse::<log::LevelFilter>() {
            Ok(level) => level,
            _ => *RTX_LOG_LEVEL,
        }
    };
    pub static ref RTX_MISSING_RUNTIME_BEHAVIOR: Option<String> =var("RTX_MISSING_RUNTIME_BEHAVIOR").ok();
    pub static ref __RTX_DIFF: EnvDiff = get_env_diff();
    pub static ref RTX_QUIET: bool = var_is_true("RTX_QUIET");
    pub static ref RTX_DEBUG: bool = var_is_true("RTX_DEBUG");
    pub static ref RTX_TRACE: bool = var_is_true("RTX_TRACE");
    pub static ref RTX_VERBOSE: bool = *RTX_DEBUG || *RTX_TRACE || var_is_true("RTX_VERBOSE");
    pub static ref DUMB_TERMINAL: bool = cfg!(test) || var("TERM").map_or(false, |term| term == "dumb");
    pub static ref RTX_JOBS: usize = var("RTX_JOBS").ok().and_then(|v| v.parse::<usize>().ok()).unwrap_or(4);
    pub static ref PREFER_STALE: bool = prefer_stale(&ARGS);
    /// essentially, this is whether we show spinners or build output on runtime install
    pub static ref PRISTINE_ENV: HashMap<String, String> =
        get_pristine_env(&__RTX_DIFF, vars().collect());
    pub static ref PATH: Vec<PathBuf> = match PRISTINE_ENV.get("PATH") {
        Some(path) => split_paths(path).collect(),
        None => vec![],
    };
    pub static ref DIRENV_DIR: Option<String> = var("DIRENV_DIR").ok();
    pub static ref DIRENV_DIFF: Option<String> = var("DIRENV_DIFF").ok();
    pub static ref RTX_EXPERIMENTAL: bool = var_is_true("RTX_EXPERIMENTAL");
    pub static ref RTX_HIDE_OUTDATED_BUILD: bool = var_is_true("RTX_HIDE_OUTDATED_BUILD");
    pub static ref RTX_ASDF_COMPAT: bool = var_is_true("RTX_ASDF_COMPAT");
    pub static ref RTX_SHORTHANDS_FILE: Option<PathBuf> = var_path("RTX_SHORTHANDS_FILE");
    pub static ref RTX_DISABLE_DEFAULT_SHORTHANDS: bool = var_is_true("RTX_DISABLE_DEFAULT_SHORTHANDS");
    pub static ref RTX_SHIMS_DIR: Option<PathBuf> = var_path("RTX_SHIMS_DIR");
    pub static ref RTX_RAW: bool = var_is_true("RTX_RAW");
    pub static ref GITHUB_API_TOKEN: Option<String> = var("GITHUB_API_TOKEN").ok();
    pub static ref PRELOAD_ENV: bool = is_cmd("hook-env") || is_cmd("env") || is_cmd("exec");
}

fn get_env_diff() -> EnvDiff {
    let env = vars().collect::<HashMap<_, _>>();
    match env.get("__RTX_DIFF") {
        Some(raw) => EnvDiff::deserialize(raw).unwrap_or_else(|err| {
            warn!("Failed to deserialize __RTX_DIFF: {:#}", err);
            EnvDiff::default()
        }),
        None => EnvDiff::default(),
    }
}

fn var_is_true(key: &str) -> bool {
    match var(key) {
        Ok(v) => {
            let v = v.to_lowercase();
            !v.is_empty()
                && v != "n"
                && v != "no"
                && v != "false"
                && v != "0"
                && v != "off"
                && v != " "
        }
        Err(_) => false,
    }
}

fn var_path(key: &str) -> Option<PathBuf> {
    var_os(key).map(PathBuf::from)
}

/// this returns the environment as if __RTX_DIFF was reversed.
/// putting the shell back into a state before hook-env was run
fn get_pristine_env(
    rtx_diff: &EnvDiff,
    orig_env: HashMap<String, String>,
) -> HashMap<String, String> {
    let patches = rtx_diff.reverse().to_patches();
    let mut env = apply_patches(&orig_env, &patches);

    // get the current path as a vector
    let path = match env.get("PATH") {
        Some(path) => split_paths(path).collect(),
        None => vec![],
    };
    // get the paths that were removed by rtx as a hashset
    let mut to_remove = rtx_diff.path.iter().collect::<HashSet<_>>();

    // remove those paths that were added by rtx, but only once (the first time)
    let path = path
        .into_iter()
        .filter(|p| !to_remove.remove(p))
        .collect_vec();

    // put the pristine PATH back into the environment
    env.insert(
        "PATH".into(),
        join_paths(path).unwrap().to_string_lossy().to_string(),
    );
    env
}

fn apply_patches(
    env: &HashMap<String, String>,
    patches: &EnvDiffPatches,
) -> HashMap<String, String> {
    let mut new_env = env.clone();
    for patch in patches {
        match patch {
            EnvDiffOperation::Add(k, v) | EnvDiffOperation::Change(k, v) => {
                new_env.insert(k.into(), v.into());
            }
            EnvDiffOperation::Remove(k) => {
                new_env.remove(k);
            }
        }
    }

    new_env
}

/// returns true if new runtime versions should not be fetched
fn prefer_stale(args: &[String]) -> bool {
    if let Some(c) = args.get(1) {
        return vec![
            "env", "hook-env", "x", "exec", "direnv", "activate", "current", "ls", "where",
        ]
        .contains(&c.as_str());
    }
    false
}

// returns true if the subcommand to rtx is this value
fn is_cmd(cmd: &str) -> bool {
    if let Some(c) = ARGS.get(1) {
        return c == cmd;
    }
    false
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::env::apply_patches;
    use crate::env_diff::EnvDiffOperation;

    use super::*;

    #[test]
    fn test_apply_patches() {
        let mut env = HashMap::new();
        env.insert("foo".into(), "bar".into());
        env.insert("baz".into(), "qux".into());
        let patches = vec![
            EnvDiffOperation::Add("foo".into(), "bar".into()),
            EnvDiffOperation::Change("baz".into(), "qux".into()),
            EnvDiffOperation::Remove("quux".into()),
        ];
        let new_env = apply_patches(&env, &patches);
        assert_eq!(new_env.len(), 2);
        assert_eq!(new_env.get("foo").unwrap(), "bar");
        assert_eq!(new_env.get("baz").unwrap(), "qux");
    }

    #[test]
    fn test_var_path() {
        set_var("RTX_TEST_PATH", "/foo/bar");
        assert_eq!(
            var_path("RTX_TEST_PATH").unwrap(),
            PathBuf::from("/foo/bar")
        );
        remove_var("RTX_TEST_PATH");
    }
}
