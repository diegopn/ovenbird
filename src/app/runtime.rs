const STABLE_APPLICATION_ID: &str = "io.github.diegopn.ovenbird";

pub fn application_id_for_runtime(flatpak_id: Option<&str>) -> &str {
    flatpak_id
        .filter(|application_id| !application_id.trim().is_empty())
        .unwrap_or(STABLE_APPLICATION_ID)
}

#[cfg(test)]
mod tests {
    const DEVELOPMENT_MANIFEST: &str =
        include_str!("../../flatpak/io.github.diegopn.ovenbird.json");

    #[test]
    fn flatpak_builder_manifest_uses_development_application_id() {
        assert!(DEVELOPMENT_MANIFEST.contains("\"app-id\": \"io.github.diegopn.ovenbird.Devel\""));
    }

    #[test]
    fn application_uses_the_flatpak_runtime_id() {
        assert_eq!(
            super::application_id_for_runtime(Some("io.github.diegopn.ovenbird.Devel")),
            "io.github.diegopn.ovenbird.Devel"
        );
    }

    #[test]
    fn application_falls_back_to_stable_id_outside_flatpak() {
        assert_eq!(
            super::application_id_for_runtime(None),
            "io.github.diegopn.ovenbird"
        );
    }

    #[test]
    fn application_ignores_an_empty_flatpak_runtime_id() {
        assert_eq!(
            super::application_id_for_runtime(Some("  ")),
            "io.github.diegopn.ovenbird"
        );
    }
}
