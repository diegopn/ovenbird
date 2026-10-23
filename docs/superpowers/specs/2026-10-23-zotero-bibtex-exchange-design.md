# Zotero BibTeX exchange design

## Intent

Let people move bibliographic references between Zotero and Ovenbird using `.bib` files, without connecting either account to Ovenbird. This is a manual exchange flow, not automatic account synchronization.

## Goals

- In the References page, provide a clear shortcut to import a BibTeX file exported from Zotero.
- Keep Ovenbird's existing BibTeX export available for importing into Zotero.
- Merge imported entries into the local library, preserving existing data and skipping duplicate citation keys as the current importer does.
- Keep the local library and exported files in standard, usable BibTeX.

## Non-goals and limitations

- No Zotero login, API token, network access, account linking, or automatic sync.
- No transfer of Zotero attachments, notes, collections, or other data outside what BibTeX represents.
- No promise that repeated exports/imports update matching records in place; an exported BibTeX file is not a complete Zotero backup.
- Automatic two-way synchronization remains a later project and would need stable remote item identifiers and conflict handling.

## Approaches

1. **Manual BibTeX exchange — selected.** Use Zotero's built-in BibTeX export and import, and Ovenbird's existing BibTeX importer and exporter. This avoids account credentials and a service operated by the project, while keeping the data portable. It requires the user to move the file and has the metadata limitations above.
2. **Zotero Web API.** Browser authorization can support account-based pulls and future writes, but requires application registration, sensitive per-user credentials, and explicit handling of remote identifiers, permissions, and conflicts. It is more than the first manual-transfer step needs.
3. **GitHub or Google Drive as storage.** These can host or synchronize a file, but add provider authentication and file-conflict behavior without improving BibTeX's bibliographic data model. They are not a better first exchange path.

## Design

Add a Zotero exchange shortcut to the main header, visible only while References is selected. Place it beside the window's minimize control as requested. Activating it exposes the existing import and export BibTeX actions with Zotero-specific wording. The import continues through the portal-backed file chooser, parses the selected `.bib`, merges it into the local library, and reports the number of newly added references. Export writes the local library as a `.bib` file for the user to import in Zotero.

This keeps the current local `.bib` library as Ovenbird's editable data and makes the two manual directions available. It does not claim continuous synchronization. The Zotero documentation confirms BibTeX import/export support and warns that export/reimport is not a complete library transfer or backup: attachments and collections are not carried by BibTeX, and reimport can alter metadata. See [BibTeX import](https://www.zotero.org/support/kb/importing_standardized_formats), [exporting from Zotero](https://www.zotero.org/support/kb/exporting), and [field mappings](https://www.zotero.org/support/kb/field_mappings).

## Constraints and implementation calls

- Reuse existing GTK file chooser, import/export actions, local library merge, and BibTeX parser rather than adding a dependency.
- Keep Flatpak permissions unchanged and use the existing user-selected-file portal flow.
- Add or update English gettext source strings and keep the Portuguese and Spanish catalogs in sync.
- The manual exchange itself needs no new persistent metadata or credentials.
