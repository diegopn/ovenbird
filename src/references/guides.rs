pub struct ReferenceManagerGuide {
    pub id: &'static str,
    pub name: &'static str,
    pub uri: &'static str,
}

pub const REFERENCE_MANAGER_GUIDES: &[ReferenceManagerGuide] = &[
    ReferenceManagerGuide {
        id: "zotero",
        name: "Zotero",
        uri: "https://diegopn.github.io/ovenbird-site/manual/#intercambio-zotero",
    },
    ReferenceManagerGuide {
        id: "jabref",
        name: "JabRef",
        uri: "https://diegopn.github.io/ovenbird-site/manual/#intercambio-jabref",
    },
    ReferenceManagerGuide {
        id: "mendeley",
        name: "Mendeley",
        uri: "https://diegopn.github.io/ovenbird-site/manual/#intercambio-mendeley",
    },
];
