use crate::storage::{atomic_write, xdg_config_home};
use std::fs;
use std::io;
use std::path::PathBuf;

fn default_reopen_last_project_on_startup() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StartupView {
    #[default]
    Editor,
    References,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_reopen_last_project_on_startup")]
    pub reopen_last_project_on_startup: bool,
    #[serde(default)]
    pub startup_view: StartupView,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_project_uri: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_folder_uri: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            reopen_last_project_on_startup: true,
            startup_view: StartupView::Editor,
            last_project_uri: None,
            default_folder_uri: None,
            extra: serde_json::Map::new(),
        }
    }
}

impl AppSettings {
    pub fn path() -> PathBuf {
        xdg_config_home().join("ovenbird/settings.json")
    }

    pub fn load() -> io::Result<Self> {
        match fs::read(Self::path()) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
            }
        }
        let mut json = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;
        json.push(b'\n');
        atomic_write(&path, &json)
    }
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, StartupView};

    #[test]
    fn app_settings_reopen_last_project_defaults_to_enabled() {
        let fresh = serde_json::to_value(AppSettings::default()).unwrap();
        assert!(fresh["reopenLastProjectOnStartup"].as_bool().unwrap());

        let legacy: AppSettings = serde_json::from_str(r#"{"colorScheme":"dark"}"#).unwrap();
        let legacy = serde_json::to_value(legacy).unwrap();
        assert!(legacy["reopenLastProjectOnStartup"].as_bool().unwrap());
        assert_eq!(legacy["colorScheme"], "dark");
    }

    #[test]
    fn app_settings_startup_view_defaults_to_latex_editor() {
        let fresh = serde_json::to_value(AppSettings::default()).unwrap();
        assert_eq!(fresh["startupView"], "editor");

        let references: AppSettings =
            serde_json::from_str(r#"{"startupView":"references"}"#).unwrap();
        assert_eq!(references.startup_view, StartupView::References);
        assert_eq!(
            serde_json::to_value(references).unwrap()["startupView"],
            "references"
        );
    }

    #[test]
    fn app_settings_keep_the_portal_uri_and_reopen_preference() {
        let settings: AppSettings = serde_json::from_str(
            r#"{"reopenLastProjectOnStartup":false,"lastProjectUri":"file:///run/user/1000/doc/abcd/project","colorScheme":"dark"}"#,
        )
        .unwrap();

        assert!(!settings.reopen_last_project_on_startup);
        assert_eq!(
            settings.last_project_uri.as_deref(),
            Some("file:///run/user/1000/doc/abcd/project")
        );

        let serialized = serde_json::to_value(settings).unwrap();
        assert!(!serialized["reopenLastProjectOnStartup"].as_bool().unwrap());
        assert_eq!(
            serialized["lastProjectUri"],
            "file:///run/user/1000/doc/abcd/project"
        );
        assert_eq!(serialized["colorScheme"], "dark");
    }

    #[test]
    fn app_settings_default_file_dialog_folder_is_optional_and_persists() {
        let older_settings: AppSettings =
            serde_json::from_str(r#"{"startupView":"editor"}"#).unwrap();
        assert_eq!(older_settings.default_folder_uri, None);

        let settings = AppSettings {
            default_folder_uri: Some("file:///home/alice/Documents".to_owned()),
            ..AppSettings::default()
        };
        let serialized = serde_json::to_value(settings).unwrap();
        assert_eq!(
            serialized["defaultFolderUri"],
            "file:///home/alice/Documents"
        );

        let restored: AppSettings = serde_json::from_value(serialized).unwrap();
        assert_eq!(
            restored.default_folder_uri.as_deref(),
            Some("file:///home/alice/Documents")
        );
    }
}
