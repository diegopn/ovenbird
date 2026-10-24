#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferDirection {
    Import,
    Export,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceFormat {
    Bibtex,
}

impl ReferenceFormat {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bibtex => "BibTeX (.bib)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransferAction {
    ImportBibtex,
    ExportBibtex,
}

pub const AVAILABLE_FORMATS: &[ReferenceFormat] = &[ReferenceFormat::Bibtex];

pub const fn action_for(direction: TransferDirection, format: ReferenceFormat) -> TransferAction {
    match (direction, format) {
        (TransferDirection::Import, ReferenceFormat::Bibtex) => TransferAction::ImportBibtex,
        (TransferDirection::Export, ReferenceFormat::Bibtex) => TransferAction::ExportBibtex,
    }
}
