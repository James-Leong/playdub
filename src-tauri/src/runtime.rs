use serde::Serialize;
use std::{env, path::PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeLocation {
    pub path: String,
    pub source: RuntimeSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RuntimeSource {
    System,
}

pub(crate) fn system_executable(name: &str) -> Option<RuntimeLocation> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .flat_map(|directory| {
            executable_names(name)
                .into_iter()
                .map(move |name| directory.join(name))
        })
        .find_map(|candidate| canonical_regular_file(&candidate))
        .map(|path| RuntimeLocation {
            path: path.to_string_lossy().into_owned(),
            source: RuntimeSource::System,
        })
}

pub(crate) fn whisper_model() -> Option<String> {
    let path = PathBuf::from(env::var_os("PLAYDUB_WHISPER_MODEL")?);
    canonical_regular_file(&path).map(|path| path.to_string_lossy().into_owned())
}

fn executable_names(name: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        vec![format!("{name}.exe"), name.to_owned()]
    }
    #[cfg(not(windows))]
    {
        vec![name.to_owned()]
    }
}

fn canonical_regular_file(path: &std::path::Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    canonical.metadata().ok()?.is_file().then_some(canonical)
}
