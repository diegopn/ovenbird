# Sidebar and workspace layout

## Goal

Make Ovenbird's workspace easier to navigate by following the Builder's compact sidebar and toolbar patterns while keeping LaTeX editing, project files, references, and Zotero actions in the section where they belong.

## User requirements

- The left panel spans the full window height and has a uniform sidebar background, without a contrasting strip around or below it.
- The sidebar header has search on the left, “Ovenbird” centered, and the main menu on the right.
- Editor and References are tabs in the sidebar. The selected tab controls both the sidebar content and the main workspace.
- In Editor, project files appear above a code editor in the left panel. Both the outer sidebar width and the divider between the file tree and code editor can be adjusted.
- The right workspace keeps the PDF preview and LaTeX document controls.
- Files and folders can be dragged into folders or to the project root. A context menu offers relevant file operations, including rename and move to trash.
- References uses its own sidebar content and main workspace. It does not show the LaTeX file tree or code editor. Do not invent additional reference categories or tools before they are specified.
- Remove Zotero synchronization from the sidebar. Put synchronization in the References toolbar; keep document saving in the Editor toolbar.
- Keep a compact Builder-style center group in the main toolbar. It shows the active section, current status, and the section's primary action: compile in Editor or add a reference in References. Use smaller type and restrained spacing than the current header.
- Put Automatic, Light, and Dark theme choices in the main menu as the three circular selectors used by Builder.
- The search control follows the active section: document search in Editor and reference search in References.
- Long project names must not widen the sidebar or break the layout; constrain the file-name row and allow horizontal scrolling where needed.
- Keep source and generated build artifacts inside `/home/diegopn/Projects/DEVELOP/ovenbird`.

## Proposed layout

### Sidebar shell

Use one full-height sidebar surface with no outer inset that exposes the window background. Keep the existing left search, centered product name, and right menu arrangement. Under the header, add two equal navigation tabs: Editor and References. The active tab has the same clear selected treatment as Builder's sidebar.

The sidebar remains hideable and resizable against the workspace. Its background is applied to the full sidebar container and its expanding content, not only to an inset child box.

### Editor tab

Use a vertical adjustable split inside the sidebar. The project tree occupies the upper region, with the existing create-file/create-folder action in its header. The code editor occupies the lower region and keeps the existing Code/Visual mode and LaTeX editing actions close to the editor. The main workspace to the right displays the PDF preview and its existing build feedback.

Allow dragging files and folders. Folders and the project root accept drops; reject moves into the dragged folder's own descendants and preserve the active document path when its parent is moved. Keep long names within the sidebar width and provide horizontal scrolling without allowing the sidebar to grow from text width.

Provide item-specific context menus. Files can be opened in Ovenbird, opened externally when appropriate, copied or located in the file manager, renamed, and moved to the trash. Folders additionally allow creating a file or folder inside them and can be moved as a unit. Keep actions such as create, rename, and trash protected by the existing project-root validation and unsaved-document safeguards.

### References tab

Switch the main workspace to the existing local references library. Keep reference-specific search, add/import, and Zotero synchronization in this section. The sidebar's References surface must not reuse Editor-only file and code controls. Leave future reference organization tools out of this change.

### Main toolbar and menu

Keep the sidebar toggle at the left. In the center, use a compact Builder-like group with a modest section label, status, separator, and primary action. The primary action is compile for Editor and add reference for References. At the right, show Save in Editor and direct Zotero sync in References, together with the normal window controls.

The main menu starts with circular Automatic, Light, and Dark theme selectors and a visible selection state, matching Builder's treatment. Existing project and application actions remain in the menu.

## Behavior and boundaries

- The toolbar and search behavior update whenever the active section changes.
- Switching sections must not discard unsaved LaTeX edits or a pending reference edit.
- Moving the active document or its containing folder updates the active path and subsequent save/build targets.
- Moving items outside the open project is not allowed.
- Reference workflows continue using the local library and existing Zotero integration; this design does not add network requirements.
- Implement the interface in the Rust application. Do not restore the removed JavaScript application.

## Acceptance criteria

1. The left sidebar background covers its complete visible area at normal, resized, and maximized window sizes.
2. Editor and References tabs switch the correct sidebar content, workspace, toolbar actions, and search destination.
3. In Editor, the file tree is above the code editor with an adjustable divider, and the PDF remains in the right workspace.
4. Dragging files or folders into a folder and to the root updates the visible tree and keeps document paths valid; invalid descendant drops are rejected.
5. Context menus expose the appropriate operations for files and folders, including rename and move to trash.
6. Zotero sync is absent from the sidebar and available from the References toolbar.
7. The center toolbar uses smaller Builder-like typography and spacing, with the correct section title, status, and primary action.
8. The theme menu uses the three selected-state circles for Automatic, Light, and Dark.
9. Long filenames do not expand the sidebar or push the workspace out of view.
10. Project and build files remain under the repository directory.

## Deliberate exclusions

- No new reference collections, categories, filters, or secondary reference tools.
- No changes to LaTeX compilation engine behavior or Zotero credentials/sync protocol.
- No external project directories or build output locations.
