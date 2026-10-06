import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import GObject from 'gi://GObject';
import Gtk from 'gi://Gtk?version=4.0';
import Adw from 'gi://Adw?version=1';
import Gdk from 'gi://Gdk?version=4.0';
import { parseBibtex, serializeBibtex, createCitationKey } from './core/bibtex.js';
import { LatexEditor } from './core/editor.js';
import { PdfPreview, availableLatexEngine, compileLatex } from './core/build.js';
import { ZoteroClient, loadZoteroApiKey, storeZoteroApiKey } from './core/zotero.js';
import { createListSnippet, createMathSnippet, createTableSnippet } from './core/latex-commands.js';
import { findDocumentMatch } from './core/document-search.js';
import { _, ngettext } from './core/i18n.js';

const CSS = `
.ovenbird-sidebar { background: @sidebar_bg_color; color: @sidebar_fg_color; }
.ovenbird-brand { font-size: 20px; font-weight: 700; }
.ovenbird-editor-toolbar { padding: 6px 12px; border-bottom: 1px solid alpha(@borders, 0.65); }
.ovenbird-mode-toolbar { padding: 6px 12px; border-bottom: 1px solid alpha(@borders, 0.65); }
.ovenbird-editor-page { background: @view_bg_color; color: @view_fg_color; }
.visual-document { background: @view_bg_color; color: @view_fg_color; font-size: 12pt; line-height: 1.5; }
.source-view { background-color: @view_bg_color; color: @view_fg_color; }
.source-view text { background-color: @view_bg_color; color: @view_fg_color; font-family: monospace; font-size: 12pt; }
.preview-placeholder { color: @dim_label; }
.sidebar-section { font-size: 0.85em; font-weight: 600; }
`;

function addCss() {
    const display = Gdk.Display.get_default();
    if (!display) return;
    const provider = new Gtk.CssProvider();
    provider.load_from_data(CSS, -1);
    Gtk.StyleContext.add_provider_for_display(display, provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION);
}

function box(orientation, spacing = 0) {
    return new Gtk.Box({ orientation, spacing });
}

function escapeLatexArgument(value) {
    return value.replace(/[#$%&_{}]/g, char => `\\${char}`);
}

export const OvenbirdWindow = GObject.registerClass(
class OvenbirdWindow extends Adw.ApplicationWindow {
    constructor(params) {
        super({ ...params, title: 'Ovenbird', default_width: 1440, default_height: 900,
            width_request: 920, height_request: 600 });
        addCss();

        this.library = this.application.library;
        this._zoteroSettings = this._loadSettings();
        this._libraryQuery = '';
        this._sidebarCollapsed = false;
        this._isCompiling = false;
        this._buildState = 'Ready';
        this._lastBuildAt = _('Not built yet');
        this._lastBuildMessage = _('Compile the document to create the PDF.');

        this.toastOverlay = new Adw.ToastOverlay();
        this.splitView = new Adw.NavigationSplitView({
            min_sidebar_width: 220,
            max_sidebar_width: 300,
            sidebar_width_fraction: 0.22,
        });
        this.sidebarStack = new Gtk.Stack();
        this.sidebarStack.add_named(this._buildSidebar(), 'full');
        this.sidebarStack.add_named(this._buildSidebarRail(), 'compact');
        this.sidebarPage = new Adw.NavigationPage({ title: _('Navigation'), child: this.sidebarStack });
        this.mainPage = new Adw.NavigationPage({ title: _('Writing'), child: this._buildMainPage() });
        this.splitView.set_sidebar(this.sidebarPage);
        this.splitView.set_content(this.mainPage);
        this.toastOverlay.set_child(this.splitView);
        this.set_content(this.toastOverlay);

        this._buildActions();
        this._setSidebarCollapsed(false);
        this._showPage('editor');
        this._renderLibrary();
    }

    _buildSidebar() {
        const sidebar = box(Gtk.Orientation.VERTICAL, 10);
        sidebar.add_css_class('ovenbird-sidebar');
        sidebar.set_margin_top(18);
        sidebar.set_margin_bottom(12);
        sidebar.set_margin_start(12);
        sidebar.set_margin_end(12);

        const brand = box(Gtk.Orientation.HORIZONTAL, 10);
        brand.set_margin_bottom(8);
        const brandIcon = new Gtk.Image({ icon_name: 'accessories-text-editor-symbolic', pixel_size: 30 });
        const brandName = new Gtk.Label({ label: 'Ovenbird', xalign: 0 });
        brandName.add_css_class('ovenbird-brand');
        brandName.set_hexpand(true);
        brand.append(brandIcon);
        brand.append(brandName);
        this.fullSearchButton = this._searchButton(_('Find in document'));
        this.fullMenuButton = this._buildAppMenuButton();
        brand.append(this.fullSearchButton);
        brand.append(this.fullMenuButton);
        sidebar.append(brand);

        const newButton = new Gtk.Button({ label: _('New document'), icon_name: 'document-new-symbolic',
            halign: Gtk.Align.FILL });
        newButton.add_css_class('suggested-action');
        newButton.connect('clicked', () => this._newDocument());
        sidebar.append(newButton);

        const section = new Gtk.Label({ label: _('WORKSPACE'), xalign: 0 });
        section.add_css_class('sidebar-section');
        section.add_css_class('dim-label');
        section.set_margin_top(16);
        section.set_margin_start(8);
        sidebar.append(section);

        this.navigationList = new Gtk.ListBox({ selection_mode: Gtk.SelectionMode.SINGLE });
        this.navigationList.add_css_class('navigation-sidebar');
        this.editorNavRow = new Adw.ActionRow({ title: _('Writing'), activatable: true });
        this.editorNavRow.add_prefix(new Gtk.Image({ icon_name: 'document-edit-symbolic' }));
        this.libraryNavRow = new Adw.ActionRow({ title: _('Local library'), activatable: true });
        this.libraryNavRow.add_prefix(new Gtk.Image({ icon_name: 'view-list-symbolic' }));
        this.navigationList.append(this.editorNavRow);
        this.navigationList.append(this.libraryNavRow);
        this.editorNavRow.connect('activated', () => this._showPage('editor'));
        this.libraryNavRow.connect('activated', () => this._showPage('library'));
        sidebar.append(this.navigationList);

        const spacer = new Gtk.Box({ vexpand: true });
        sidebar.append(spacer);

        const zoteroButton = new Gtk.Button({ label: _('Connect to Zotero'), icon_name: 'network-server-symbolic' });
        zoteroButton.add_css_class('flat');
        zoteroButton.connect('clicked', () => this._showZoteroSettings());
        sidebar.append(zoteroButton);

        const version = new Gtk.Label({ label: _('LaTeX · local references'), xalign: 0 });
        version.add_css_class('dim-label');
        version.set_margin_start(8);
        sidebar.append(version);
        return sidebar;
    }

    _buildSidebarRail() {
        const rail = box(Gtk.Orientation.VERTICAL, 8);
        rail.add_css_class('ovenbird-sidebar');
        rail.set_margin_top(12);
        rail.set_margin_bottom(12);
        rail.set_margin_start(4);
        rail.set_margin_end(4);

        const brandIcon = new Gtk.Image({ icon_name: 'accessories-text-editor-symbolic', pixel_size: 30,
            halign: Gtk.Align.CENTER });
        brandIcon.set_margin_bottom(8);
        rail.append(brandIcon);

        const newDocument = this._iconButton('document-new-symbolic', _('New document'), () => this._newDocument());
        rail.append(newDocument);
        rail.append(new Gtk.Separator({ orientation: Gtk.Orientation.HORIZONTAL }));

        this.compactEditorButton = this._iconButton('document-edit-symbolic', _('Writing'), () => this._showPage('editor'));
        this.compactLibraryButton = this._iconButton('view-list-symbolic', _('Local library'), () => this._showPage('library'));
        rail.append(this.compactEditorButton);
        rail.append(this.compactLibraryButton);

        const spacer = new Gtk.Box({ vexpand: true });
        rail.append(spacer);
        const zotero = this._iconButton('network-server-symbolic', _('Set up Zotero'), () => this._showZoteroSettings());
        rail.append(zotero);
        return rail;
    }

    _searchButton(tooltip) {
        return this._iconButton('system-search-symbolic', tooltip, () => this._showDocumentSearch());
    }

    _buildAppMenuButton() {
        const button = new Gtk.MenuButton({ icon_name: 'open-menu-symbolic', tooltip_text: _('Application menu') });
        const popover = new Gtk.Popover();
        popover.set_has_arrow(false);
        const content = box(Gtk.Orientation.VERTICAL, 2);
        content.set_size_request(320, -1);
        content.set_margin_top(6);
        content.set_margin_bottom(6);
        content.set_margin_start(6);
        content.set_margin_end(6);

        const items = [
            [_('New document'), 'document-new-symbolic', '', () => this._newDocument()],
            [_('Open document'), 'document-open-symbolic', 'Ctrl+O', () => this._openDocument()],
            [_('Open project folder'), 'folder-open-symbolic', '', () => this._openProjectFolder()],
            [_('Save document'), 'document-save-symbolic', 'Ctrl+S', () => this._saveDocument()],
            [_('Find in document'), 'system-search-symbolic', 'Ctrl+F', () => this._showDocumentSearch()],
            [_('Compile and create PDF'), 'media-playback-start-symbolic', 'Ctrl+Shift+R', () => this._compile()],
            [_('Insert citation'), 'document-edit-symbolic', 'Ctrl+Shift+C', () => this._showCitationPicker()],
            [_('Local library'), 'view-list-symbolic', '', () => this._showPage('library')],
            ['Zotero', 'network-server-symbolic', '', () => this._showZoteroSettings()],
            [_('Keyboard shortcuts'), 'preferences-system-symbolic', '', () => this._showShortcuts()],
            [_('Collapse/expand sidebar'), 'sidebar-show-symbolic', '', () => this._setSidebarCollapsed(!this._sidebarCollapsed)],
        ];
        for (const [label, icon, shortcut, callback] of items) {
            const action = new Gtk.Button({ has_frame: false, halign: Gtk.Align.FILL });
            const row = box(Gtk.Orientation.HORIZONTAL, 10);
            row.append(new Gtk.Image({ icon_name: icon, pixel_size: 16 }));
            const title = new Gtk.Label({ label, xalign: 0, hexpand: true });
            row.append(title);
            if (shortcut) {
                const accelerator = new Gtk.Label({ label: shortcut });
                accelerator.add_css_class('dim-label');
                row.append(accelerator);
            }
            action.set_child(row);
            action.connect('clicked', () => {
                popover.popdown();
                callback();
            });
            content.append(action);
        }
        popover.set_child(content);
        button.set_popover(popover);
        return button;
    }

    _buildBuildStatusButton() {
        const button = new Gtk.MenuButton({ tooltip_text: _('Document and build details'),
            has_frame: false });
        button.set_size_request(190, -1);
        const child = box(Gtk.Orientation.HORIZONTAL, 7);
        this.buildStateIcon = new Gtk.Image({ icon_name: 'checkbox-checked-symbolic' });
        this.buildStateLabel = new Gtk.Label({ label: this._buildState });
        child.append(this.buildStateIcon);
        child.append(this.buildStateLabel);
        button.set_child(child);

        const popover = new Gtk.Popover();
        popover.set_has_arrow(true);
        const content = box(Gtk.Orientation.VERTICAL, 10);
        content.set_size_request(340, -1);
        content.set_margin_top(14);
        content.set_margin_bottom(14);
        content.set_margin_start(16);
        content.set_margin_end(16);

        const title = new Gtk.Label({ label: _('Current document'), xalign: 0 });
        title.add_css_class('title-4');
        content.append(title);
        this.buildDetailsFile = new Gtk.Label({ xalign: 0, wrap: true, selectable: true });
        this.buildDetailsPath = new Gtk.Label({ xalign: 0, wrap: true, selectable: true });
        this.buildDetailsPath.add_css_class('dim-label');
        content.append(this.buildDetailsFile);
        content.append(this.buildDetailsPath);

        const details = new Gtk.Grid({ column_spacing: 14, row_spacing: 6 });
        const rows = [
            [_('Compiler'), 'buildDetailsEngine'],
            [_('Status'), 'buildDetailsState'],
            [_('Last build'), 'buildDetailsLast'],
        ];
        for (let row = 0; row < rows.length; row++) {
            const [caption, property] = rows[row];
            const label = new Gtk.Label({ label: caption, xalign: 1 });
            label.add_css_class('dim-label');
            const value = new Gtk.Label({ xalign: 0, hexpand: true, wrap: true });
            details.attach(label, 0, row, 1, 1);
            details.attach(value, 1, row, 1, 1);
            this[property] = value;
        }
        content.append(details);
        this.buildDetailsOutput = new Gtk.Label({ xalign: 0, wrap: true, selectable: true,
            max_width_chars: 48 });
        this.buildDetailsOutput.add_css_class('dim-label');
        content.append(this.buildDetailsOutput);

        popover.set_child(content);
        button.set_popover(popover);
        return button;
    }

    _buildDocumentSearchBar() {
        const searchBar = new Gtk.SearchBar();
        const row = box(Gtk.Orientation.HORIZONTAL, 6);
        row.set_margin_top(6);
        row.set_margin_bottom(6);
        row.set_margin_start(12);
        row.set_margin_end(12);

        this.documentSearchEntry = new Gtk.SearchEntry({ placeholder_text: _('Find in document…'),
            hexpand: true, width_chars: 32 });
        this.documentSearchResult = new Gtk.Label({ label: '', xalign: 0.5 });
        this.documentSearchResult.add_css_class('dim-label');
        const previous = this._iconButton('go-up-symbolic', _('Previous result'),
            () => this._searchDocument('backward'));
        const next = this._iconButton('go-down-symbolic', _('Next result'),
            () => this._searchDocument('forward'));
        const close = this._iconButton('window-close-symbolic', _('Close search'), () => {
            searchBar.set_search_mode(false);
            this.editor.focus();
        });

        row.append(this.documentSearchEntry);
        row.append(previous);
        row.append(next);
        row.append(this.documentSearchResult);
        row.append(close);
        searchBar.set_child(row);
        searchBar.set_key_capture_widget(this);
        this.documentSearchEntry.connect('search-changed', () => this._searchDocument('forward', true));
        this.documentSearchEntry.connect('activate', () => this._searchDocument('forward'));
        return searchBar;
    }

    _updateBuildDetails() {
        if (!this.buildStateLabel) return;
        const file = this.editor?.file;
        const pendingChanges = Boolean(this.editor?.dirty);
        this.buildStateLabel.set_label(this._buildState === 'Error' ? _('Error') :
            pendingChanges ? _('Modified') : _(this._buildState));
        const icon = this._buildState === 'Error' ? 'dialog-error-symbolic' :
            pendingChanges ? 'document-edit-symbolic' : this._buildState === 'Building…' ?
                'view-refresh-symbolic' : 'checkbox-checked-symbolic';
        this.buildStateIcon.set_from_icon_name(icon);
        this.buildDetailsFile.set_label(file?.get_basename() || _('New document not saved yet'));
        const folder = this.projectFolder?.get_path() || file?.get_parent()?.get_path() || _('No project folder selected');
        const filePath = file?.get_path() || _('Save the document to set the project file.');
        this.buildDetailsPath.set_label(_('File: %s\nProject: %s').replace('%s', filePath).replace('%s', folder));
        this.buildDetailsEngine.set_label(availableLatexEngine()?.name || _('Not found'));
        this.buildDetailsState.set_label(pendingChanges ?
            _('%s · local changes pending').replace('%s', _(this._buildState)) : _(this._buildState));
        this.buildDetailsLast.set_label(this._lastBuildAt);
        this.buildDetailsOutput.set_label(this._lastBuildMessage);
    }

    _setBuildStatus(state, message = null) {
        this._buildState = state;
        if (message !== null) this._lastBuildMessage = message.slice(-600);
        if (state === 'Built' || state === 'Error')
            this._lastBuildAt = GLib.DateTime.new_now_local().format('%H:%M:%S');
        this._updateBuildDetails();
    }

    _showDocumentSearch() {
        if (this.pageStack.get_visible_child_name() !== 'editor') this._showPage('editor');
        this.searchBar.set_search_mode(true);
        this.documentSearchEntry.grab_focus();
        this.documentSearchEntry.select_region(0, -1);
    }

    _searchDocument(direction, fromStart = false) {
        const query = this.documentSearchEntry.get_text();
        const mode = this.editor.editorStack.get_visible_child_name();
        const visual = mode === 'visual';
        const buffer = visual ? this.editor.visualBuffer : this.editor.sourceBuffer;
        const view = visual ? this.editor.visualView : this.editor.sourceView;
        const [hasSelection, start, end] = buffer.get_selection_bounds();
        const insertOffset = buffer.get_iter_at_mark(buffer.get_insert()).get_offset();
        const startOffset = fromStart ? 0 : direction === 'backward' ?
            (hasSelection ? start.get_offset() : insertOffset) :
            (hasSelection ? end.get_offset() : insertOffset);
        const text = buffer.get_text(buffer.get_start_iter(), buffer.get_end_iter(), true);
        const match = findDocumentMatch(text, query, startOffset, direction);

        if (!match) {
            this.documentSearchResult.set_label(query.trim() ? _('No results') : '');
            if (hasSelection) buffer.place_cursor(buffer.get_iter_at_mark(buffer.get_insert()));
            this.documentSearchEntry.grab_focus();
            return;
        }
        const matchStart = buffer.get_iter_at_offset(match.start);
        const matchEnd = buffer.get_iter_at_offset(match.end);
        buffer.select_range(matchEnd, matchStart);
        view.scroll_to_iter(matchStart, 0.15, false, 0, 0.5);
        this.documentSearchResult.set_label(_('Match found'));
        this.documentSearchEntry.grab_focus();
    }

    _runPrimaryAction() {
        if (this.pageStack.get_visible_child_name() === 'library') this._editReference(null);
        else this._compile();
    }

    _setSidebarCollapsed(collapsed) {
        this._sidebarCollapsed = Boolean(collapsed);
        this.sidebarStack.set_visible_child_name(this._sidebarCollapsed ? 'compact' : 'full');
        if (this._sidebarCollapsed) {
            this.splitView.set_min_sidebar_width(72);
            this.splitView.set_max_sidebar_width(84);
        } else {
            this.splitView.set_max_sidebar_width(300);
            this.splitView.set_min_sidebar_width(220);
        }
        this.splitView.set_sidebar_width_fraction(this._sidebarCollapsed ? 0.07 : 0.22);
        this.sidebarToggleButton.set_icon_name('sidebar-show-symbolic');
        this.sidebarToggleButton.set_tooltip_text(this._sidebarCollapsed ? _('Expand sidebar') : _('Collapse sidebar'));
        this.collapsedSearchButton.set_visible(this._sidebarCollapsed);
        this.collapsedMenuButton.set_visible(this._sidebarCollapsed);
    }

    _buildMainPage() {
        const toolbarView = new Adw.ToolbarView();
        this.headerBar = new Adw.HeaderBar();
        this.headerTitle = new Gtk.Label({ label: _('Writing'), xalign: 0 });
        this.headerTitle.add_css_class('title');
        this.buildStatusButton = this._buildBuildStatusButton();
        this.headerBar.set_title_widget(this.buildStatusButton);

        this.sidebarToggleButton = this._iconButton('sidebar-show-symbolic', _('Collapse sidebar'),
            () => this._setSidebarCollapsed(!this._sidebarCollapsed));
        this.collapsedSearchButton = this._searchButton(_('Find in document'));
        this.collapsedMenuButton = this._buildAppMenuButton();
        this.headerBar.pack_start(this.sidebarToggleButton);
        this.headerBar.pack_start(this.collapsedSearchButton);
        this.headerBar.pack_start(this.collapsedMenuButton);

        this.openButton = this._iconButton('document-open-symbolic', _('Open LaTeX document'), () => this._openDocument());
        this.openProjectButton = this._iconButton('folder-open-symbolic', _('Open project folder'), () => this._openProjectFolder());
        this.saveButton = this._iconButton('document-save-symbolic', _('Save document'), () => this._saveDocument());
        this.primaryActionButton = new Gtk.Button({ tooltip_text: _('Compile and create PDF'), has_frame: false });
        const primaryActionContent = box(Gtk.Orientation.HORIZONTAL, 6);
        this.primaryActionIcon = new Gtk.Image({ icon_name: 'media-playback-start-symbolic' });
        this.primaryActionLabel = new Gtk.Label({ label: _('Add reference') });
        primaryActionContent.append(this.primaryActionIcon);
        primaryActionContent.append(this.primaryActionLabel);
        this.primaryActionButton.set_child(primaryActionContent);
        this.primaryActionButton.connect('clicked', () => this._runPrimaryAction());
        this.syncButton = this._iconButton('view-refresh-symbolic', _('Sync Zotero'), () => this._syncZotero());
        this.headerBar.pack_start(this.openButton);
        this.headerBar.pack_start(this.openProjectButton);
        this.headerBar.pack_start(this.saveButton);
        this.headerBar.pack_end(this.syncButton);
        this.headerBar.pack_end(this.primaryActionButton);

        this.modeBox = box(Gtk.Orientation.HORIZONTAL, 0);
        this.codeModeButton = new Gtk.ToggleButton({ label: _('Code'), active: true });
        this.visualModeButton = new Gtk.ToggleButton({ label: _('Visual') });
        this.visualModeButton.set_group(this.codeModeButton);
        this.codeModeButton.connect('toggled', button => {
            if (button.get_active()) {
                this.editor?.setMode('code');
            }
        });
        this.visualModeButton.connect('toggled', button => {
            if (button.get_active()) {
                this.editor?.setMode('visual');
            }
        });
        this.modeBox.append(this.codeModeButton);
        this.modeBox.append(this.visualModeButton);
        this.modeBox.add_css_class('linked');
        toolbarView.add_top_bar(this.headerBar);

        this.pageStack = new Gtk.Stack({ transition_type: Gtk.StackTransitionType.CROSSFADE });
        this.editor = new LatexEditor();
        this.editor.onModeChanged = mode => {
            if (mode === 'code' && this.visualModeButton.get_active()) this.codeModeButton.set_active(true);
        };
        this.editor.onDirtyChanged = dirty => this._updateTitle(dirty);
        this.editor.onHistoryChanged = (canUndo, canRedo) => {
            this.undoButton?.set_sensitive(canUndo);
            this.redoButton?.set_sensitive(canRedo);
        };
        this.pdfPreview = new PdfPreview();
        this.pdfPreview.setMessage(_('The compiled PDF will appear here'));
        this._updateBuildDetails();

        const editorPage = box(Gtk.Orientation.VERTICAL);
        editorPage.add_css_class('ovenbird-editor-page');
        editorPage.set_hexpand(true);
        editorPage.set_vexpand(true);
        this.searchBar = this._buildDocumentSearchBar();
        editorPage.append(this.searchBar);
        this.modeBar = box(Gtk.Orientation.HORIZONTAL, 8);
        this.modeBar.add_css_class('ovenbird-mode-toolbar');
        this.modeBox.set_valign(Gtk.Align.CENTER);
        this.modeBar.append(this.modeBox);
        const modeHint = new Gtk.Label({ label: _('Switch modes at any time without creating another version of the document'),
            hexpand: true, xalign: 1 });
        modeHint.add_css_class('dim-label');
        this.modeBar.append(modeHint);
        editorPage.append(this.modeBar);

        this.formatBar = box(Gtk.Orientation.HORIZONTAL, 4);
        this.formatBar.add_css_class('ovenbird-editor-toolbar');
        this.undoButton = this._iconButton('edit-undo-symbolic', _('Undo'), () => this.editor.undo());
        this.redoButton = this._iconButton('edit-redo-symbolic', _('Redo'), () => this.editor.redo());
        this.undoButton.set_sensitive(this.editor.history.canUndo);
        this.redoButton.set_sensitive(this.editor.history.canRedo);
        this.formatBar.append(this.undoButton);
        this.formatBar.append(this.redoButton);
        this._appendToolbarSeparator(this.formatBar);

        this.styleButton = this._toolbarMenuButton(_('Normal text'), null, [
            [_('Normal text'), () => this._applyHeadingStyle(_('Normal text'), 'normal')],
            [_('Section'), () => this._applyHeadingStyle(_('Section'), 'section')],
            [_('Subsection'), () => this._applyHeadingStyle(_('Subsection'), 'subsection')],
            [_('Subsubsection'), () => this._applyHeadingStyle(_('Subsubsection'), 'subsubsection')],
            [_('Paragraph'), () => this._applyHeadingStyle(_('Paragraph'), 'paragraph')],
            [_('Subparagraph'), () => this._applyHeadingStyle(_('Subparagraph'), 'subparagraph')],
        ]);
        this.formatBar.append(this.styleButton);
        this._appendToolbarSeparator(this.formatBar);

        for (const [label, icon, format] of [
            [_('Bold'), 'format-text-bold-symbolic', 'bold'],
            [_('Italic'), 'format-text-italic-symbolic', 'italic'],
            [_('Underline'), 'format-text-underline-symbolic', 'underline'],
            [_('Monospace'), 'text-x-generic-symbolic', 'monospace'],
        ]) this.formatBar.append(this._iconButton(icon, label, () => this.editor.applyFormat(format)));
        this._appendToolbarSeparator(this.formatBar);

        this.formatBar.append(this._toolbarMenuButton(_('Math'), null, [
            [_('Inline equation'), () => this.editor.insertCommand(createMathSnippet(false, this.editor.selectedText()), 'insert:math-inline')],
            [_('Display equation'), () => this.editor.insertCommand(createMathSnippet(true, this.editor.selectedText()), 'insert:math-display')],
        ]));
        const insertMathSymbol = command => this.editor.insertCommand(createMathSnippet(false, command), 'insert:symbol');
        this.formatBar.append(this._toolbarMenuButton('Ω', null, [
            [_('α  Alpha'), () => insertMathSymbol(String.raw`\alpha`)],
            [_('β  Beta'), () => insertMathSymbol(String.raw`\beta`)],
            [_('γ  Gamma'), () => insertMathSymbol(String.raw`\gamma`)],
            [_('π  Pi'), () => insertMathSymbol(String.raw`\pi`)],
            [_('≤  Less than or equal to'), () => insertMathSymbol(String.raw`\leq`)],
            [_('≥  Greater than or equal to'), () => insertMathSymbol(String.raw`\geq`)],
            [_('≠  Not equal to'), () => insertMathSymbol(String.raw`\neq`)],
            [_('×  Multiplication'), () => insertMathSymbol(String.raw`\times`)],
            [_('±  Plus or minus'), () => insertMathSymbol(String.raw`\pm`)],
            [_('∞  Infinity'), () => insertMathSymbol(String.raw`\infty`)],
            [_('∑  Summation'), () => insertMathSymbol(String.raw`\sum`)],
            [_('∫  Integral'), () => insertMathSymbol(String.raw`\int`)],
            [_('√  Square root'), () => this.editor.insertLatexSnippet(String.raw`\(\sqrt{}\)`, 8, 'insert:symbol')],
        ]));
        this.formatBar.append(this._iconButton('insert-link-symbolic', _('Insert link'), () => this._showLinkDialog()));
        this.citationButton = new Gtk.Button({ label: _('Cite'), icon_name: 'system-search-symbolic',
            tooltip_text: _('Find and insert a bibliography reference') });
        this.citationButton.add_css_class('flat');
        this.citationButton.connect('clicked', () => this._showCitationPicker());
        this.formatBar.append(this.citationButton);
        this._appendToolbarSeparator(this.formatBar);
        this.formatBar.append(this._toolbarMenuButton(_('List'), 'view-list-bullet-symbolic', [
            [_('Bulleted list'), () => this.editor.insertCommand(createListSnippet('itemize', this.editor.selectedText()), 'insert:list')],
            [_('Numbered list'), () => this.editor.insertCommand(createListSnippet('enumerate', this.editor.selectedText()), 'insert:list')],
            [_('Quotation block'), () => this.editor.insertCommand(createListSnippet('quote', this.editor.selectedText()), 'insert:quote')],
            [_('Increase indent'), () => this.editor.adjustIndent(true)],
            [_('Decrease indent'), () => this.editor.adjustIndent(false)],
        ]));
        this.formatBar.append(this._toolbarMenuButton(_('More'), 'view-more-symbolic', [
            [_('Insert image'), () => this._chooseImageForDocument()],
            [_('Insert table'), () => this._showTableDialog()],
            [_('LaTeX comment'), () => this.editor.insertComment()],
            [_('Footnote'), () => this.editor.insertFootnote()],
            [_('Review note'), () => this.editor.insertLatexAtCursor(String.raw`\marginpar{}`, 11, 'insert:review-note')],
            [_('Add label'), () => this.editor.insertLatexAtCursor(String.raw`\label{}`, 7, 'insert:label')],
            [_('Cross-reference'), () => this.editor.insertLatexAtCursor(String.raw`\ref{}`, 5, 'insert:reference')],
        ]));
        this.formatToolbarScroll = new Gtk.ScrolledWindow({
            hscrollbar_policy: Gtk.PolicyType.AUTOMATIC,
            vscrollbar_policy: Gtk.PolicyType.NEVER,
            child: this.formatBar,
            hexpand: true,
        });
        this.formatToolbarScroll.set_min_content_height(48);
        editorPage.append(this.formatToolbarScroll);

        this.editorPdfPane = new Gtk.Paned({ orientation: Gtk.Orientation.HORIZONTAL, position: 680,
            wide_handle: true, hexpand: true, vexpand: true });
        this.editorPdfPane.set_start_child(this.editor.root);
        this.editorPdfPane.set_end_child(this.pdfPreview.root);
        this.editorPdfPane.set_resize_start_child(true);
        this.editorPdfPane.set_resize_end_child(true);
        editorPage.append(this.editorPdfPane);

        this.editorStatus = new Gtk.Label({ label: _('New document · unsaved changes'), xalign: 0 });
        this.editorStatus.add_css_class('dim-label');
        this.editorStatus.set_margin_start(14);
        this.editorStatus.set_margin_end(14);
        this.editorStatus.set_margin_top(5);
        this.editorStatus.set_margin_bottom(7);
        editorPage.append(this.editorStatus);
        this.pageStack.add_named(editorPage, 'editor');

        this.libraryPage = this._buildLibraryPage();
        this.pageStack.add_named(this.libraryPage, 'library');
        toolbarView.set_content(this.pageStack);
        return toolbarView;
    }

    _buildLibraryPage() {
        const page = box(Gtk.Orientation.VERTICAL, 12);
        page.set_margin_top(16);
        page.set_margin_start(22);
        page.set_margin_end(22);
        page.set_margin_bottom(14);

        const intro = box(Gtk.Orientation.VERTICAL, 4);
        const heading = new Gtk.Label({ label: _('Local library'), xalign: 0 });
        heading.add_css_class('title-1');
        const subtitle = new Gtk.Label({ label: _('Your references stay on this computer and remain available offline.'), xalign: 0 });
        subtitle.add_css_class('dim-label');
        intro.append(heading);
        intro.append(subtitle);
        page.append(intro);

        const controls = box(Gtk.Orientation.HORIZONTAL, 8);
        this.librarySearch = new Gtk.SearchEntry({ placeholder_text: _('Search references…'), hexpand: true });
        this.librarySearch.connect('search-changed', entry => {
            this._libraryQuery = entry.get_text().trim().toLocaleLowerCase();
            this._renderLibrary();
        });
        const importButton = new Gtk.Button({ label: _('Import .bib'), icon_name: 'document-open-symbolic' });
        importButton.connect('clicked', () => this._importBibtex());
        const exportButton = new Gtk.Button({ label: _('Export .bib'), icon_name: 'document-save-symbolic' });
        exportButton.connect('clicked', () => this._exportBibtex());
        controls.append(this.librarySearch);
        controls.append(importButton);
        controls.append(exportButton);
        page.append(controls);

        this.referenceList = new Gtk.ListBox({ selection_mode: Gtk.SelectionMode.NONE });
        this.referenceList.add_css_class('boxed-list');
        this.referenceList.set_valign(Gtk.Align.START);
        this.referenceScroll = new Gtk.ScrolledWindow({
            vexpand: true,
            hscrollbar_policy: Gtk.PolicyType.NEVER,
            child: this.referenceList,
        });
        page.append(this.referenceScroll);

        this.libraryStatus = new Gtk.Label({ label: '', xalign: 0 });
        this.libraryStatus.add_css_class('dim-label');
        page.append(this.libraryStatus);
        return page;
    }

    _iconButton(icon, tooltip, callback) {
        const button = new Gtk.Button({ icon_name: icon, tooltip_text: tooltip, has_frame: false });
        button.connect('clicked', callback);
        return button;
    }

    _appendToolbarSeparator(toolbar) {
        const separator = new Gtk.Separator({ orientation: Gtk.Orientation.VERTICAL });
        separator.set_margin_start(4);
        separator.set_margin_end(4);
        toolbar.append(separator);
    }

    _toolbarMenuButton(label, icon, items) {
        const button = new Gtk.MenuButton({ label, ...(icon ? { icon_name: icon } : {}), tooltip_text: label });
        const popover = new Gtk.Popover();
        popover.set_has_arrow(false);
        const content = box(Gtk.Orientation.VERTICAL, 2);
        content.set_margin_top(6);
        content.set_margin_bottom(6);
        content.set_margin_start(6);
        content.set_margin_end(6);
        for (const [itemLabel, callback] of items) {
            const item = new Gtk.Button({ label: itemLabel, has_frame: false, halign: Gtk.Align.FILL });
            item.connect('clicked', () => {
                popover.popdown();
                callback();
            });
            content.append(item);
        }
        popover.set_child(content);
        button.set_popover(popover);
        return button;
    }

    _applyHeadingStyle(label, style) {
        this.styleButton.set_label(label);
        this.editor.applyHeading(style);
    }

    _buildActions() {
        const actions = [
            ['new-document', () => this._newDocument()],
            ['open', () => this._openDocument()],
            ['open-project', () => this._openProjectFolder()],
            ['save', () => this._saveDocument()],
            ['compile', () => this._compile()],
            ['find', () => this._showDocumentSearch()],
            ['toggle-sidebar', () => this._setSidebarCollapsed(!this._sidebarCollapsed)],
            ['undo', () => this.editor.undo()],
            ['redo', () => this.editor.redo()],
            ['cite', () => this._showCitationPicker()],
        ];
        for (const [name, callback] of actions) {
            const action = new Gio.SimpleAction({ name });
            action.connect('activate', callback);
            this.add_action(action);
        }
    }

    _showPage(name) {
        this.pageStack.set_visible_child_name(name);
        this.headerTitle.set_label(name === 'editor' ? _('Writing') : _('Local library'));
        this.headerBar.set_title_widget(name === 'editor' ? this.buildStatusButton : this.headerTitle);
        this.navigationList.select_row(name === 'editor' ? this.editorNavRow : this.libraryNavRow);
        this.modeBar.set_visible(name === 'editor');
        this.openButton.set_visible(name === 'editor');
        this.openProjectButton.set_visible(name === 'editor');
        this.saveButton.set_visible(name === 'editor');
        this.primaryActionIcon.set_from_icon_name(name === 'editor' ?
            'media-playback-start-symbolic' : 'list-add-symbolic');
        this.primaryActionLabel.set_visible(name === 'library');
        if (name === 'library') this.primaryActionButton.add_css_class('suggested-action');
        else this.primaryActionButton.remove_css_class('suggested-action');
        this.primaryActionButton.set_tooltip_text(name === 'editor' ?
            _('Compile and create PDF') : _('Add a reference to the local library'));
        this.primaryActionButton.set_sensitive(name === 'library' || !this._isCompiling);
        this.formatToolbarScroll.set_visible(name === 'editor');
        this.searchBar.set_search_mode(false);
        this.syncButton.set_visible(name === 'library');
        this.compactEditorButton?.remove_css_class('suggested-action');
        this.compactLibraryButton?.remove_css_class('suggested-action');
        (name === 'editor' ? this.compactEditorButton : this.compactLibraryButton)
            ?.add_css_class('suggested-action');
        this._updateBuildDetails();
        if (name === 'editor') this.editor.focus();
    }

    _updateTitle(dirty) {
        const fileName = this.editor.file?.get_basename() || _('New document');
        this.editorStatus.set_label(dirty ? _('%s · unsaved changes').replace('%s', fileName) : _('%s · saved').replace('%s', fileName));
        this.saveButton.set_sensitive(dirty || !this.editor.file);
        this._updateBuildDetails();
    }

    _showShortcuts() {
        const dialog = new Gtk.MessageDialog({ transient_for: this, modal: true,
            message_type: Gtk.MessageType.INFO, buttons: Gtk.ButtonsType.CLOSE,
            text: _('Keyboard shortcuts'),
            secondary_text: _('Ctrl+O  Open document\nCtrl+S  Save\nCtrl+F  Find in document\nCtrl+Shift+R  Compile and create PDF\nCtrl+Shift+C  Insert citation\nCtrl+Z / Ctrl+Shift+Z  Undo / Redo') });
        dialog.connect('response', () => dialog.destroy());
        dialog.present();
    }

    _chooseFile(action, title, name, callback) {
        const chooser = new Gtk.FileChooserNative({
            title,
            transient_for: this,
            action,
            accept_label: action === Gtk.FileChooserAction.OPEN ? _('Open') : _('Save'),
            cancel_label: _('Cancel'),
        });
        if (name) chooser.set_current_name(name);
        const filter = new Gtk.FileFilter();
        filter.set_name(_('LaTeX (*.tex)'));
        filter.add_pattern('*.tex');
        chooser.add_filter(filter);
        chooser.connect('response', (_dialog, response) => {
            if (response === Gtk.ResponseType.ACCEPT) callback(chooser.get_file());
            chooser.destroy();
        });
        chooser.show();
    }

    _openDocument() {
        this._chooseFile(Gtk.FileChooserAction.OPEN, _('Open LaTeX document'), null, file => {
            if (file) this.openDocument(file);
        });
    }

    _chooseProjectFolder(title, callback) {
        const chooser = new Gtk.FileChooserNative({ title, transient_for: this,
            action: Gtk.FileChooserAction.SELECT_FOLDER, accept_label: _('Select folder'),
            cancel_label: _('Cancel') });
        chooser.connect('response', (_dialog, response) => {
            const folder = response === Gtk.ResponseType.ACCEPT ? chooser.get_file() : null;
            chooser.destroy();
            if (folder) callback(folder);
        });
        chooser.show();
    }

    _openProjectFolder() {
        this._chooseProjectFolder(_('Open LaTeX project folder'), folder => {
            try {
                const enumerator = folder.enumerate_children('standard::name,standard::type',
                    Gio.FileQueryInfoFlags.NONE, null);
                const documents = [];
                let info;
                while ((info = enumerator.next_file(null))) {
                    if (info.get_file_type() === Gio.FileType.REGULAR && /\.tex$/i.test(info.get_name()))
                        documents.push(folder.get_child(info.get_name()));
                }
                enumerator.close(null);
                documents.sort((left, right) => left.get_basename().localeCompare(right.get_basename()));
                if (!documents.length) {
                    this._toast(_('No .tex files were found in this folder'));
                    return;
                }
                this.projectFolder = folder;
                if (documents.length === 1) this.openDocument(documents[0]);
                else this._chooseProjectDocument(documents);
            } catch (error) {
                this._toast(_("Could not list the folder: %s").replace('%s', error.message));
            }
        });
    }

    _chooseProjectDocument(documents) {
        const dialog = new Gtk.Dialog({ title: _('Choose main document'), transient_for: this, modal: true });
        dialog.set_default_size(520, 500);
        dialog.add_button(_('Cancel'), Gtk.ResponseType.CANCEL);
        dialog.add_button(_('Open'), Gtk.ResponseType.ACCEPT);
        dialog.set_default_response(Gtk.ResponseType.ACCEPT);
        const content = dialog.get_content_area();
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        const list = new Gtk.ListBox({ selection_mode: Gtk.SelectionMode.SINGLE });
        list.add_css_class('boxed-list');
        const scroll = new Gtk.ScrolledWindow({ vexpand: true, child: list });
        content.append(scroll);
        for (const file of documents) {
            const row = new Adw.ActionRow({ title: file.get_basename(), activatable: true });
            row.connect('activated', () => {
                list.select_row(row);
                dialog.response(Gtk.ResponseType.ACCEPT);
            });
            list.append(row);
        }
        list.select_row(list.get_row_at_index(0));
        dialog.connect('response', (_dialog, response) => {
            const row = response === Gtk.ResponseType.ACCEPT ? list.get_selected_row() : null;
            dialog.destroy();
            if (row) this.openDocument(documents[row.get_index()]);
        });
        dialog.present();
    }

    openDocument(file) {
        const load = () => {
            try {
                const path = file.get_path();
                if (!path) throw new Error(_('The document must be available as a local file.'));
                const [ok, bytes] = GLib.file_get_contents(path);
                if (!ok) throw new Error(_('Could not read the file.'));
                this.editor.setFile(file);
                this.projectFolder = file.get_parent();
                this.editor.loadText(new TextDecoder().decode(bytes));
                this._lastBuildAt = _('Not built yet');
                this._setBuildStatus('Ready', _('This document has not been compiled yet.'));
                this.pdfPreview.setMessage(_('Compile the document to see the PDF'));
                this._showPage('editor');
            } catch (error) {
                this._toast(_("Could not open: %s").replace('%s', error.message));
            }
        };
        this._confirmDiscard(load);
    }

    _confirmDiscard(action) {
        if (!this.editor.dirty) {
            action();
            return;
        }
        const dialog = new Gtk.MessageDialog({
            transient_for: this,
            modal: true,
            message_type: Gtk.MessageType.WARNING,
            buttons: Gtk.ButtonsType.NONE,
            text: _('Discard unsaved changes?'),
            secondary_text: _('The current document has changes that have not been saved.'),
        });
        dialog.add_button(_('Keep editing'), Gtk.ResponseType.CANCEL);
        dialog.add_button(_('Discard'), Gtk.ResponseType.ACCEPT);
        dialog.connect('response', (_dialog, response) => {
            dialog.destroy();
            if (response === Gtk.ResponseType.ACCEPT) action();
        });
        dialog.present();
    }

    _newDocument() {
        this._confirmDiscard(() => this._chooseProjectFolder(_('Choose a folder for the new project'), folder => {
            this._promptNewDocumentName(folder, LatexEditor.newDocument(), true);
        }));
    }

    _promptNewDocumentName(folder, source, isNewDocument) {
        const dialog = new Gtk.Dialog({ title: isNewDocument ? _('New LaTeX document') : _('Save LaTeX document'),
            transient_for: this, modal: true });
        dialog.add_button(_('Cancel'), Gtk.ResponseType.CANCEL);
        dialog.add_button(isNewDocument ? _('Create') : _('Save'), Gtk.ResponseType.ACCEPT);
        dialog.set_default_response(Gtk.ResponseType.ACCEPT);
        const content = dialog.get_content_area();
        content.set_spacing(10);
        content.set_margin_top(18);
        content.set_margin_bottom(18);
        content.set_margin_start(20);
        content.set_margin_end(20);
        content.append(new Gtk.Label({ label: _('File name in the selected folder'), xalign: 0 }));
        const nameEntry = new Gtk.Entry({ text: 'document.tex', hexpand: true, activates_default: true });
        content.append(nameEntry);
        dialog.connect('response', (_dialog, response) => {
            if (response !== Gtk.ResponseType.ACCEPT) {
                dialog.destroy();
                return;
            }
            let name = nameEntry.get_text().trim();
            if (!name || name === '.' || name === '..' || /[\/\\\u0000-\u001f]/.test(name)) {
                this._toast(_('Enter a valid file name'));
                return;
            }
            if (!/\.tex$/i.test(name)) name += '.tex';
            const file = folder.get_child(name);
            if (file.query_exists(null)) {
                this._toast(_('A file with that name already exists in this folder'));
                return;
            }
            try {
                const path = file.get_path();
                if (!path) throw new Error(_('The folder is not available as a local directory.'));
                GLib.file_set_contents(path, source);
                this.projectFolder = folder;
                this.editor.setFile(file);
                if (isNewDocument) this.editor.loadText(source);
                else this.editor.save();
                this._lastBuildAt = _('Not built yet');
                this._setBuildStatus('Ready', _('This document has not been compiled yet.'));
                this.pdfPreview.setMessage(_('Compile the document to see the PDF'));
                this._showPage('editor');
                dialog.destroy();
            } catch (error) {
                this._toast(_("Could not create the file: %s").replace('%s', error.message));
            }
        });
        dialog.present();
    }

    _saveDocument() {
        if (this.editor.save()) {
            this._toast(_('Document saved'));
            return;
        }
        this._chooseProjectFolder(_('Choose a folder for the document'), folder => {
            this._promptNewDocumentName(folder, this.editor.text(), false);
        });
    }

    _prepareProjectBibliography() {
        const source = this.editor.text();
        const resources = [
            ...[...source.matchAll(/\\addbibresource(?:\s*\[[^\]]*\])?\s*\{([^}]+)\}/g)]
                .map(match => match[1].trim()),
            ...[...source.matchAll(/\\bibliography\s*\{([^}]+)\}/g)]
                .flatMap(match => match[1].split(',').map(name => name.trim()))
                .filter(Boolean)
                .map(name => /\.bib$/i.test(name) ? name : `${name}.bib`),
        ].filter(Boolean);
        if (!resources.length) return { added: 0, conflicts: 0 };

        const folder = this.projectFolder || this.editor.file?.get_parent();
        if (!folder) throw new Error(_('Choose a project folder before updating the BibTeX file.'));
        const seenResources = new Set();
        let added = 0;
        let conflicts = 0;

        for (const resource of resources) {
            if (GLib.path_is_absolute(resource) || resource.startsWith('~') ||
                resource.split(/[\\/]/).includes('..'))
                continue;
            const file = folder.resolve_relative_path(resource);
            const path = file.get_path();
            if (!path || seenResources.has(path)) continue;
            seenResources.add(path);

            const exists = file.query_exists(null);
            let resourceAdded = 0;
            let directivesChanged = false;
            if (!exists && GLib.mkdir_with_parents(GLib.path_get_dirname(path), 0o755) !== 0)
                throw new Error(_('Could not create a folder for %s.').replace('%s', resource));
            let projectEntries = [];
            let projectDirectives = [];
            if (exists) {
                const [ok, bytes] = GLib.file_get_contents(path);
                if (!ok) throw new Error(_('Could not read %s.').replace('%s', resource));
                projectEntries = parseBibtex(new TextDecoder().decode(bytes));
                projectDirectives = projectEntries.directives || [];
            }

            const entryByKey = new Map(projectEntries.map(entry => [entry.key.toLowerCase(), entry]));
            for (const local of this.library.entries) {
                const fields = { ...local.fields };
                delete fields.zotero_key;
                delete fields.zotero_version;
                const rawFields = { ...local.rawFields };
                delete rawFields.zotero_key;
                delete rawFields.zotero_version;
                const clean = { ...local, fields, rawFields };
                const current = entryByKey.get(local.key.toLowerCase());
                if (current) {
                    const existingFields = { ...current.fields };
                    delete existingFields.zotero_key;
                    delete existingFields.zotero_version;
                    const normalizedFields = value => JSON.stringify(Object.fromEntries(
                        Object.entries(value).sort(([left], [right]) => left.localeCompare(right))));
                    if (current.type !== clean.type || normalizedFields(existingFields) !== normalizedFields(fields))
                        conflicts++;
                    continue;
                }
                projectEntries.push(clean);
                entryByKey.set(local.key.toLowerCase(), clean);
                added++;
                resourceAdded++;
            }

            const stringKeys = new Set(projectDirectives.map(directive =>
                directive.match(/^\s*@string\s*[({]\s*([^=\s,]+)/i)?.[1]?.toLowerCase()).filter(Boolean));
            for (const directive of this.library.directives) {
                const key = directive.match(/^\s*@string\s*[({]\s*([^=\s,]+)/i)?.[1]?.toLowerCase();
                if (key && stringKeys.has(key)) continue;
                if (!projectDirectives.includes(directive)) {
                    projectDirectives.push(directive);
                    directivesChanged = true;
                }
                if (key) stringKeys.add(key);
            }

            if (resourceAdded > 0 || directivesChanged || !exists)
                GLib.file_set_contents(path, serializeBibtex(projectEntries, projectDirectives));
        }
        return { added, conflicts };
    }

    _compile() {
        if (this._isCompiling) return;
        if (!this.editor.file) {
            this._toast(_('Save the document before compiling'));
            this._setBuildStatus('Ready', _('Save the document before compiling.'));
            return;
        }
        if (this.editor.dirty) this.editor.save();
        if (!availableLatexEngine()) {
            this._setBuildStatus('Error', _('No LaTeX compiler was found in the Ovenbird environment.'));
            this._showBuildError(_('No LaTeX compiler was found. Install latexmk, Tectonic, or TeX Live in the Ovenbird environment.'));
            return;
        }
        try {
            const bib = this._prepareProjectBibliography();
            if (bib.added) this._toast(ngettext('%d local reference added to the project BibTeX',
                '%d local references added to the project BibTeX', bib.added).replace('%d', bib.added));
            if (bib.conflicts) this._toast(ngettext('%d duplicate key; the project file was left unchanged',
                '%d duplicate keys; the project file was left unchanged', bib.conflicts).replace('%d', bib.conflicts));
        } catch (error) {
            this._setBuildStatus('Error', error.message);
            this._toast(_('Could not update the project bibliography: %s').replace('%s', error.message));
            return;
        }
        this._isCompiling = true;
        this.primaryActionButton.set_sensitive(false);
        this._setBuildStatus('Building…', _('Compilation is in progress.'));
        this.editorStatus.set_label(_('Compiling LaTeX…'));
        compileLatex(this.editor.file, (error, pdfPath, message) => {
            this._isCompiling = false;
            this.primaryActionButton.set_sensitive(true);
            if (error) {
                this.editorStatus.set_label(_('Compilation found errors'));
                this._setBuildStatus('Error', error.message);
                this._showBuildError(error.message);
                return;
            }
            this.pdfPreview.open(pdfPath).then(() => {
                this.editorStatus.set_label(message);
                this._setBuildStatus('Built', message);
                this._toast(_('PDF updated'));
            }).catch(previewError => {
                this.editorStatus.set_label(_('PDF compiled, but the preview could not be opened'));
                this._setBuildStatus('Error', _('PDF created, but the preview could not be opened: %s').replace('%s', previewError.message));
                this._showBuildError(previewError.message, _('Could not open the PDF preview'));
            });
        }, status => {
            this.editorStatus.set_label(status);
            this._setBuildStatus('Building…', status);
        });
    }

    _showBuildError(message, title = _('Could not compile the document')) {
        this.pdfPreview.setMessage(_('Check the build output'));
        const dialog = new Gtk.MessageDialog({
            transient_for: this,
            modal: true,
            message_type: Gtk.MessageType.ERROR,
            buttons: Gtk.ButtonsType.CLOSE,
            text: title,
            secondary_text: message.slice(-5000),
        });
        dialog.connect('response', () => dialog.destroy());
        dialog.present();
    }

    _renderLibrary() {
        if (!this.referenceList) return;
        while (this.referenceList.get_first_child())
            this.referenceList.remove(this.referenceList.get_first_child());

        const entries = this.library.entries.filter(entry => {
            if (!this._libraryQuery) return true;
            const searchable = `${entry.key} ${Object.values(entry.fields).join(' ')}`.toLocaleLowerCase();
            return searchable.includes(this._libraryQuery);
        });

        if (entries.length === 0) {
            const empty = new Adw.StatusPage({
                icon_name: this.library.entries.length ? 'system-search-symbolic' : 'view-list-symbolic',
                title: this.library.entries.length ? _('No references found') : _('Your library starts here'),
                description: this.library.entries.length ? _('Try another search term.') : _('Add a reference, import a BibTeX file, or connect your Zotero account.'),
            });
            this.referenceList.append(empty);
        } else {
            for (const entry of entries) {
                const author = entry.fields.author || entry.fields.editor || _('Unknown author');
                const detail = [author, entry.fields.year, entry.fields.journal || entry.fields.publisher]
                    .filter(Boolean).join(' · ');
                const row = new Adw.ActionRow({ title: entry.fields.title || entry.key, subtitle: detail, activatable: true });
                const key = new Gtk.Label({ label: entry.key });
                key.add_css_class('dim-label');
                row.add_suffix(key);
                const edit = new Gtk.Button({ icon_name: 'document-edit-symbolic', tooltip_text: _('Edit reference'), has_frame: false });
                row.add_suffix(edit);
                edit.connect('clicked', () => this._editReference(entry));
                row.connect('activated', () => this._editReference(entry));
                this.referenceList.append(row);
            }
        }
        const referenceCount = ngettext('%d local reference', '%d local references', this.library.entries.length)
            .replace('%d', this.library.entries.length);
        this.libraryStatus.set_label(`${referenceCount} · ${this.library.path}`);
    }

    _editReference(entry) {
        const dialog = new Gtk.Dialog({ title: entry ? _('Edit reference') : _('New reference'), transient_for: this, modal: true });
        dialog.set_default_size(720, 680);
        dialog.add_button(_('Cancel'), Gtk.ResponseType.CANCEL);
        if (entry) dialog.add_button(_('Delete'), Gtk.ResponseType.REJECT);
        dialog.add_button(entry ? _('Save') : _('Add'), Gtk.ResponseType.ACCEPT);
        dialog.set_default_response(Gtk.ResponseType.ACCEPT);

        const content = dialog.get_content_area();
        content.set_spacing(12);
        content.set_margin_top(18);
        content.set_margin_bottom(18);
        content.set_margin_start(20);
        content.set_margin_end(20);
        const grid = new Gtk.Grid({ column_spacing: 12, row_spacing: 10,
            margin_top: 12, margin_bottom: 12, margin_start: 12, margin_end: 12 });
        const formScroll = new Gtk.ScrolledWindow({ vexpand: true, child: grid,
            hscrollbar_policy: Gtk.PolicyType.NEVER });
        content.append(formScroll);

        const bibTypes = ['article', 'book', 'incollection', 'inproceedings', 'phdthesis', 'mastersthesis', 'techreport', 'online', 'misc'];
        const typePicker = Gtk.DropDown.new_from_strings(bibTypes);
        typePicker.set_selected(Math.max(0, bibTypes.indexOf(entry?.type || 'article')));
        const typeCaption = new Gtk.Label({ label: _('Reference type'), xalign: 1 });
        typeCaption.add_css_class('dim-label');
        grid.attach(typeCaption, 0, 0, 1, 1);
        grid.attach(typePicker, 1, 0, 1, 1);

        const fields = [
            [_('Citation key'), 'key', entry?.key || ''],
            [_('Title'), 'title', entry?.fields.title || ''],
            [_('Author(s)'), 'author', entry?.fields.author || ''],
            [_('Editor(s)'), 'editor', entry?.fields.editor || ''],
            [_('Year / date'), 'date', entry?.fields.date || entry?.fields.year || ''],
            [_('Journal / event / book'), 'venue', entry?.fields.journal || entry?.fields.booktitle || ''],
            [_('Publisher'), 'publisher', entry?.fields.publisher || ''],
            [_('Volume'), 'volume', entry?.fields.volume || ''],
            [_('Number / issue'), 'number', entry?.fields.number || ''],
            [_('Edition'), 'edition', entry?.fields.edition || ''],
            [_('Pages'), 'pages', entry?.fields.pages || ''],
            [_('DOI'), 'doi', entry?.fields.doi || ''],
            [_('URL'), 'url', entry?.fields.url || ''],
            [_('Keywords'), 'keywords', entry?.fields.keywords || ''],
        ];
        const inputs = {};
        for (let row = 0; row < fields.length; row++) {
            const [label, name, value] = fields[row];
            const caption = new Gtk.Label({ label, xalign: 1 });
            caption.add_css_class('dim-label');
            const input = new Gtk.Entry({ text: value, hexpand: true });
            if (name === 'title') input.set_placeholder_text(_('Full title of the work'));
            grid.attach(caption, 0, row + 1, 1, 1);
            grid.attach(input, 1, row + 1, 1, 1);
            inputs[name] = input;
        }

        dialog.connect('response', (_dialog, response) => {
            if (response === Gtk.ResponseType.REJECT && entry) {
                this.library.remove(entry.key);
                this._renderLibrary();
                this._toast(_('Reference removed from the local library'));
            } else if (response === Gtk.ResponseType.ACCEPT) {
                const values = Object.fromEntries(Object.entries(inputs).map(([name, input]) => [name, input.get_text().trim()]));
                if (!values.title) {
                    this._toast(_('Enter the reference title'));
                    return;
                }
                const fieldsValue = { ...entry?.fields, ...values };
                delete fieldsValue.key;
                fieldsValue.year = fieldsValue.date?.match(/\d{4}/)?.[0] || fieldsValue.date || '';
                const type = bibTypes[typePicker.get_selected()];
                const venue = fieldsValue.venue;
                delete fieldsValue.venue;
                if (['incollection', 'inproceedings'].includes(type)) {
                    fieldsValue.booktitle = venue;
                    delete fieldsValue.journal;
                } else {
                    fieldsValue.journal = venue;
                    delete fieldsValue.booktitle;
                }
                fieldsValue.journal = fieldsValue.journal || undefined;
                const used = new Set(this.library.entries.filter(item => item.key !== entry?.key).map(item => item.key.toLowerCase()));
                const key = values.key || createCitationKey(fieldsValue, used);
                const updated = { type, key, fields: fieldsValue, rawFields: {} };
                try {
                    if (entry) this.library.update(entry.key, updated);
                    else this.library.add(updated);
                    this._renderLibrary();
                    this._toast(entry ? _('Reference updated') : _('Reference added'));
                } catch (error) {
                    this._toast(error.message);
                    return;
                }
            }
            dialog.destroy();
        });
        dialog.present();
    }

    _showLinkDialog() {
        const dialog = new Gtk.Dialog({ title: _('Insert link'), transient_for: this, modal: true });
        dialog.add_button(_('Cancel'), Gtk.ResponseType.CANCEL);
        dialog.add_button(_('Insert link'), Gtk.ResponseType.ACCEPT);
        dialog.set_default_response(Gtk.ResponseType.ACCEPT);
        const content = dialog.get_content_area();
        content.set_spacing(10);
        content.set_margin_top(16);
        content.set_margin_bottom(16);
        content.set_margin_start(18);
        content.set_margin_end(18);
        const selected = this.editor.selectedText();
        const url = new Gtk.Entry({ placeholder_text: 'https://example.org', hexpand: true });
        const label = new Gtk.Entry({ text: selected, placeholder_text: _('Text shown in the document'), hexpand: true });
        content.append(new Gtk.Label({ label: _('Link address'), xalign: 0 }));
        content.append(url);
        content.append(new Gtk.Label({ label: _('Link text'), xalign: 0 }));
        content.append(label);
        dialog.connect('response', (_dialog, response) => {
            if (response !== Gtk.ResponseType.ACCEPT) {
                dialog.destroy();
                return;
            }
            const address = url.get_text().trim();
            if (!address) {
                this._toast(_('Enter a link address'));
                return;
            }
            const visibleText = label.get_text().trim() || address;
            const snippet = String.raw`\href{${escapeLatexArgument(address)}}{${escapeLatexArgument(visibleText)}}`;
            this.editor.insertLatexSnippet(snippet, snippet.length, 'insert:link');
            dialog.destroy();
        });
        dialog.present();
        url.grab_focus();
    }

    _showTableDialog() {
        const dialog = new Gtk.Dialog({ title: _('Insert table'), transient_for: this, modal: true });
        dialog.add_button(_('Cancel'), Gtk.ResponseType.CANCEL);
        dialog.add_button(_('Insert table'), Gtk.ResponseType.ACCEPT);
        dialog.set_default_response(Gtk.ResponseType.ACCEPT);
        const content = dialog.get_content_area();
        content.set_spacing(10);
        content.set_margin_top(18);
        content.set_margin_bottom(18);
        content.set_margin_start(20);
        content.set_margin_end(20);
        const grid = new Gtk.Grid({ column_spacing: 12, row_spacing: 10 });
        const rows = Gtk.SpinButton.new_with_range(1, 10, 1);
        const columns = Gtk.SpinButton.new_with_range(1, 8, 1);
        rows.set_value(3);
        columns.set_value(3);
        grid.attach(new Gtk.Label({ label: _('Rows'), xalign: 1 }), 0, 0, 1, 1);
        grid.attach(rows, 1, 0, 1, 1);
        grid.attach(new Gtk.Label({ label: _('Columns'), xalign: 1 }), 0, 1, 1, 1);
        grid.attach(columns, 1, 1, 1, 1);
        content.append(grid);
        dialog.connect('response', (_dialog, response) => {
            if (response === Gtk.ResponseType.ACCEPT) {
                const table = createTableSnippet(rows.get_value_as_int(), columns.get_value_as_int());
                this.editor.insertLatexAtCursor(table.text, table.cursorOffset, 'insert:table');
            }
            dialog.destroy();
        });
        dialog.present();
    }

    _chooseImageForDocument() {
        const chooser = new Gtk.FileChooserNative({ title: _('Insert image'), transient_for: this,
            action: Gtk.FileChooserAction.OPEN, accept_label: _('Insert'), cancel_label: _('Cancel') });
        const filter = new Gtk.FileFilter();
        filter.set_name(_('Images and PDF'));
        for (const pattern of ['*.png', '*.jpg', '*.jpeg', '*.pdf', '*.eps']) filter.add_pattern(pattern);
        chooser.add_filter(filter);
        chooser.connect('response', (_dialog, response) => {
            if (response === Gtk.ResponseType.ACCEPT) {
                const file = chooser.get_file();
                const relative = this.projectFolder?.get_relative_path(file);
                const path = relative || file.get_path();
                const command = String.raw`\includegraphics[width=\linewidth]{${path}}`;
                this.editor.insertLatexAtCursor(command, command.length, 'insert:image');
            }
            chooser.destroy();
        });
        chooser.show();
    }

    _showCitationPicker() {
        if (this.pageStack.get_visible_child_name() !== 'editor') this._showPage('editor');
        if (this.citationPopover) {
            this.citationPopover.popup();
            return;
        }

        const popover = new Gtk.Popover({ position: Gtk.PositionType.BOTTOM });
        popover.set_has_arrow(false);
        popover.set_parent(this.citationButton);
        this.citationPopover = popover;
        const content = box(Gtk.Orientation.VERTICAL, 8);
        content.set_size_request(360, -1);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);

        const search = new Gtk.SearchEntry({ placeholder_text: _('Search by title, author, or year…') });
        const emptyLibrary = this.library.entries.length === 0;
        if (emptyLibrary) {
            const empty = new Gtk.Label({
                label: _('Your local library is empty. Add a reference or import a BibTeX file to cite it.'),
                wrap: true, xalign: 0,
            });
            empty.add_css_class('dim-label');
            content.append(empty);
            const addButton = new Gtk.Button({ label: _('Add reference'), icon_name: 'list-add-symbolic' });
            const importButton = new Gtk.Button({ label: _('Import BibTeX'), icon_name: 'document-open-symbolic' });
            addButton.connect('clicked', () => { popover.popdown(); this._editReference(null); });
            importButton.connect('clicked', () => { popover.popdown(); this._importBibtex(); });
            content.append(addButton);
            content.append(importButton);
            popover.set_child(content);
            popover.connect('closed', () => {
                popover.unparent();
                if (this.citationPopover === popover) this.citationPopover = null;
            });
            popover.popup();
            addButton.grab_focus();
            return;
        }

        content.append(search);
        const list = new Gtk.ListBox({ selection_mode: Gtk.SelectionMode.SINGLE });
        list.add_css_class('boxed-list');
        const scroll = new Gtk.ScrolledWindow({ min_content_height: 100, max_content_height: 360,
            hscrollbar_policy: Gtk.PolicyType.NEVER, child: list });
        const noResults = new Gtk.Label({ label: _('No references found'), xalign: 0 });
        noResults.add_css_class('dim-label');
        content.append(scroll);
        content.append(noResults);
        let matching = [];
        const render = () => {
            while (list.get_first_child()) list.remove(list.get_first_child());
            const query = search.get_text().trim().toLocaleLowerCase();
            matching = this.library.entries.filter(entry => {
                const fields = entry.fields;
                return `${fields.title || ''} ${fields.author || ''} ${fields.editor || ''} ${fields.year || ''} ${fields.date || ''} ${entry.key}`
                    .toLocaleLowerCase().includes(query);
            });
            noResults.set_visible(matching.length === 0);
            scroll.set_visible(matching.length > 0);
            for (const entry of matching) {
                const author = entry.fields.author || entry.fields.editor || _('Unknown author');
                const year = entry.fields.year || entry.fields.date?.match(/\d{4}/)?.[0] || _('Year unknown');
                const row = new Adw.ActionRow({
                    title: entry.fields.title || _('Untitled reference'),
                    subtitle: `${author} · ${year}`,
                    activatable: true,
                });
                list.append(row);
            }
        };
        const insertEntry = entry => {
            if (!entry) return;
            popover.popdown();
            this.editor.insertLatexSnippet(String.raw`\cite{${entry.key}}`, null, 'insert:citation');
        };
        list.connect('row-activated', (_list, row) => insertEntry(matching[row.get_index()]));
        search.connect('search-changed', render);
        search.connect('activate', () => insertEntry(matching[0]));
        popover.set_child(content);
        popover.connect('closed', () => {
            popover.unparent();
            if (this.citationPopover === popover) this.citationPopover = null;
        });
        render();
        popover.popup();
        search.grab_focus();
    }

    _importBibtex() {
        const chooser = new Gtk.FileChooserNative({ title: _('Import BibTeX'), transient_for: this,
            action: Gtk.FileChooserAction.OPEN, accept_label: _('Import'), cancel_label: _('Cancel') });
        const filter = new Gtk.FileFilter();
        filter.set_name(_('BibTeX (*.bib)'));
        filter.add_pattern('*.bib');
        chooser.add_filter(filter);
        chooser.connect('response', (_dialog, response) => {
            if (response === Gtk.ResponseType.ACCEPT) {
                try {
                    const [ok, bytes] = GLib.file_get_contents(chooser.get_file().get_path());
                    if (!ok) throw new Error(_('Could not read the file.'));
                    const imported = parseBibtex(new TextDecoder().decode(bytes));
                    for (const directive of imported.directives || []) {
                        if (!this.library.directives.includes(directive)) this.library.directives.push(directive);
                    }
                    const known = new Set(this.library.entries.map(entry => entry.key.toLowerCase()));
                    let count = 0;
                    for (const entry of imported) {
                        if (known.has(entry.key.toLowerCase())) continue;
                        this.library.entries.push(entry);
                        known.add(entry.key.toLowerCase());
                        count++;
                    }
                    this.library.save();
                    this._renderLibrary();
                    this._toast(ngettext('%d reference imported', '%d references imported', count).replace('%d', count));
                } catch (error) {
                    this._toast(_('Import failed: %s').replace('%s', error.message));
                }
            }
            chooser.destroy();
        });
        chooser.show();
    }

    _exportBibtex() {
        const chooser = new Gtk.FileChooserNative({ title: _('Export BibTeX library'), transient_for: this,
            action: Gtk.FileChooserAction.SAVE, accept_label: _('Export'), cancel_label: _('Cancel') });
        chooser.set_current_name('ovenbird-library.bib');
        chooser.connect('response', (_dialog, response) => {
            if (response === Gtk.ResponseType.ACCEPT) {
                try {
                    GLib.file_set_contents(chooser.get_file().get_path(),
                        serializeBibtex(this.library.entries, this.library.directives));
                    this._toast(_('Library exported'));
                } catch (error) {
                    this._toast(_('Export failed: %s').replace('%s', error.message));
                }
            }
            chooser.destroy();
        });
        chooser.show();
    }

    _loadSettings() {
        const directory = GLib.build_filenamev([GLib.get_user_config_dir(), 'ovenbird']);
        GLib.mkdir_with_parents(directory, 0o700);
        this.settingsPath = GLib.build_filenamev([directory, 'settings.json']);
        try {
            const [ok, bytes] = GLib.file_get_contents(this.settingsPath);
            if (ok) return JSON.parse(new TextDecoder().decode(bytes));
        } catch (error) {
            const isMissingFile = error?.matches?.(GLib.FileError, GLib.FileError.NOENT) ?? false;
            if (!isMissingFile)
                logError(error, 'Could not load app settings');
        }
        return {};
    }

    _saveSettings() {
        GLib.file_set_contents(this.settingsPath, JSON.stringify(this._zoteroSettings, null, 2));
    }

    _showZoteroSettings() {
        const dialog = new Gtk.Dialog({ title: _('Sync with Zotero'), transient_for: this, modal: true });
        dialog.set_default_size(600, 320);
        dialog.add_button(_('Cancel'), Gtk.ResponseType.CANCEL);
        dialog.add_button(_('Save and sync'), Gtk.ResponseType.ACCEPT);
        const content = dialog.get_content_area();
        content.set_spacing(12);
        content.set_margin_top(18);
        content.set_margin_bottom(18);
        content.set_margin_start(20);
        content.set_margin_end(20);

        const description = new Gtk.Label({ label: _('The local library works without an account. To sync, enter your library ID and a Zotero key with read and write access.'), wrap: true, xalign: 0 });
        content.append(description);
        const grid = new Gtk.Grid({ column_spacing: 12, row_spacing: 10 });
        content.append(grid);
        const idLabel = new Gtk.Label({ label: _('User ID'), xalign: 1 });
        const idEntry = new Gtk.Entry({ text: this._zoteroSettings.userId || '', placeholder_text: _('Found at zotero.org/settings/keys'), hexpand: true });
        const keyLabel = new Gtk.Label({ label: _('API key'), xalign: 1 });
        const keyEntry = new Gtk.PasswordEntry({ placeholder_text: _('Zotero key with write access'), hexpand: true, show_peek_icon: true });
        grid.attach(idLabel, 0, 0, 1, 1);
        grid.attach(idEntry, 1, 0, 1, 1);
        grid.attach(keyLabel, 0, 1, 1, 1);
        grid.attach(keyEntry, 1, 1, 1, 1);

        dialog.connect('response', async (_dialog, response) => {
            if (response === Gtk.ResponseType.ACCEPT) {
                const userId = idEntry.get_text().trim();
                const key = keyEntry.get_text().trim();
                if (!userId || !key) {
                    this._toast(_('Enter the ID and API key'));
                    return;
                }
                try {
                    storeZoteroApiKey(key);
                    this._zoteroSettings.userId = userId;
                    this._saveSettings();
                    dialog.destroy();
                    await this._syncZotero();
                } catch (error) {
                    this._toast(_('Could not save the key: %s').replace('%s', error.message));
                    return;
                }
            } else dialog.destroy();
        });
        dialog.present();
    }

    async _syncZotero() {
        let apiKey;
        try { apiKey = loadZoteroApiKey(); }
        catch (error) { this._toast(_('The password service is unavailable: %s').replace('%s', error.message)); return; }
        if (!this._zoteroSettings.userId || !apiKey) {
            this._showZoteroSettings();
            return;
        }
        this.syncButton.set_sensitive(false);
        this._toast(_('Syncing references…'));
        try {
            const client = new ZoteroClient(this._zoteroSettings.userId, apiKey);
            const result = await client.sync(this.library);
            this._renderLibrary();
            const conflicts = result.conflicts.length;
            const message = [
                ngettext('%d reference downloaded', '%d references downloaded', result.imported).replace('%d', result.imported),
                ngettext('%d reference uploaded', '%d references uploaded', result.created).replace('%d', result.created),
                ngettext('%d reference updated locally', '%d references updated locally', result.updatedLocally).replace('%d', result.updatedLocally),
                ...(conflicts ? [ngettext('%d conflict kept locally', '%d conflicts kept locally', conflicts).replace('%d', conflicts)] : []),
                ...(result.failed ? [ngettext('%d upload failed', '%d uploads failed', result.failed).replace('%d', result.failed)] : []),
            ].join(' · ');
            this._toast(message);
        } catch (error) {
            this._toast(_('Sync failed: %s').replace('%s', error.message));
        } finally {
            this.syncButton.set_sensitive(true);
        }
    }

    _toast(message) {
        this.toastOverlay.add_toast(new Adw.Toast({ title: message, timeout: 5 }));
    }
});
