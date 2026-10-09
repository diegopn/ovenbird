#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplicationMenuItem {
    pub icon: &'static str,
    pub label: &'static str,
    pub action: &'static str,
    pub shortcut: &'static str,
}

pub const ABOUT_DEDICATION: &str =
    "Dedicated to my wife, Karina, who always encourages me to go further.";

pub const ABOUT_LINKS: &[(&str, &str)] = &[
    ("Personal website", "https://diegopn.github.io/"),
    ("Project repository", "https://github.com/diegopn/ovenbird"),
    (
        "Project website",
        "https://diegopn.github.io/ovenbird-site/",
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreference {
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub fn from_setting(value: Option<&str>) -> Self {
        match value {
            Some("light") => Self::Light,
            Some("dark") => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn setting_value(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

pub const APPLICATION_MENU_GROUPS: &[&[ApplicationMenuItem]] = &[
    &[
        ApplicationMenuItem {
            icon: "document-new-symbolic",
            label: "New project",
            action: "win.new-project",
            shortcut: "Ctrl+Alt+N",
        },
        ApplicationMenuItem {
            icon: "folder-open-symbolic",
            label: "Open project",
            action: "win.open-project",
            shortcut: "Ctrl+Alt+O",
        },
    ],
    &[
        ApplicationMenuItem {
            icon: "document-open-symbolic",
            label: "Open document",
            action: "win.open-document",
            shortcut: "Ctrl+O",
        },
        ApplicationMenuItem {
            icon: "window-close-symbolic",
            label: "Close project",
            action: "win.close-project",
            shortcut: "Ctrl+W",
        },
    ],
    &[
        ApplicationMenuItem {
            icon: "preferences-system-symbolic",
            label: "Keyboard shortcuts",
            action: "win.shortcuts",
            shortcut: "Ctrl+?",
        },
        ApplicationMenuItem {
            icon: "application-exit-symbolic",
            label: "Quit",
            action: "win.quit",
            shortcut: "Ctrl+Q",
        },
    ],
    &[ApplicationMenuItem {
        icon: "help-about-symbolic",
        label: "About Ovenbird",
        action: "win.about",
        shortcut: "",
    }],
];

pub const THEME_MENU_CHOICES: &[ApplicationMenuItem] = &[
    ApplicationMenuItem {
        icon: "preferences-system-symbolic",
        label: "Automatic",
        action: "theme-auto",
        shortcut: "",
    },
    ApplicationMenuItem {
        icon: "weather-clear-symbolic",
        label: "Light",
        action: "theme-light",
        shortcut: "",
    },
    ApplicationMenuItem {
        icon: "weather-clear-night-symbolic",
        label: "Dark",
        action: "theme-dark",
        shortcut: "",
    },
];

#[cfg(test)]
mod tests {
    use super::{ThemePreference, APPLICATION_MENU_GROUPS, THEME_MENU_CHOICES};

    #[test]
    fn main_menu_has_project_lifecycle_and_shortcut_actions() {
        let items = APPLICATION_MENU_GROUPS
            .iter()
            .flat_map(|group| group.iter())
            .collect::<Vec<_>>();
        assert!(items.iter().any(|item| item.label == "New project"));
        assert!(items.iter().any(|item| item.label == "Open project"));
        assert!(items.iter().any(|item| item.label == "Close project"));
        assert!(items.iter().any(|item| item.label == "Keyboard shortcuts"));
        assert!(items
            .iter()
            .any(|item| { item.label == "About Ovenbird" && item.action == "win.about" }));
        assert!(!items.iter().any(|item| item.label == "Export PDF"));
        assert!(!items.iter().any(|item| item.label == "Save document"));
        assert_eq!(
            items
                .iter()
                .find(|item| item.label == "New project")
                .unwrap()
                .shortcut,
            "Ctrl+Alt+N"
        );
        assert_eq!(
            items
                .iter()
                .find(|item| item.label == "Open project")
                .unwrap()
                .shortcut,
            "Ctrl+Alt+O"
        );
        assert_eq!(
            items
                .iter()
                .find(|item| item.label == "Open document")
                .unwrap()
                .shortcut,
            "Ctrl+O"
        );
        assert_eq!(
            items
                .iter()
                .find(|item| item.label == "Close project")
                .unwrap()
                .shortcut,
            "Ctrl+W"
        );
        assert!(!items
            .iter()
            .any(|item| matches!(item.label, "Find in document" | "Local library")));
    }

    #[test]
    fn theme_menu_exposes_automatic_light_and_dark_modes() {
        let labels = THEME_MENU_CHOICES
            .iter()
            .map(|item| item.label)
            .collect::<Vec<_>>();
        assert_eq!(labels, ["Automatic", "Light", "Dark"]);
        assert_eq!(
            THEME_MENU_CHOICES
                .iter()
                .map(|item| item.action)
                .collect::<Vec<_>>(),
            ["theme-auto", "theme-light", "theme-dark"]
        );
    }

    #[test]
    fn theme_preference_defaults_to_system_and_round_trips() {
        assert_eq!(ThemePreference::from_setting(None), ThemePreference::System);
        for preference in [
            ThemePreference::System,
            ThemePreference::Light,
            ThemePreference::Dark,
        ] {
            assert_eq!(
                ThemePreference::from_setting(Some(preference.setting_value())),
                preference
            );
        }
        assert_eq!(
            ThemePreference::from_setting(Some("unknown")),
            ThemePreference::System
        );
    }
}
