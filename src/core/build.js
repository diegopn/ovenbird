import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk?version=4.0';
import { _, ngettext } from './i18n.js';

const BUILD_TIMEOUT_SECONDS = 300;

export function availableLatexEngine() {
    for (const engine of ['latexmk', 'pdflatex', 'tectonic']) {
        const path = GLib.find_program_in_path(engine);
        if (path) return { name: engine, path };
    }
    return null;
}

export function buildOutputDirectory(sourcePath, cacheDirectory = GLib.get_user_cache_dir()) {
    const sourceId = GLib.compute_checksum_for_string(GLib.ChecksumType.SHA256, sourcePath, -1);
    return GLib.build_filenamev([cacheDirectory, 'ovenbird', 'build', sourceId]);
}

function runProcess(args, directory, environment = {}) {
    return new Promise((resolve, reject) => {
        try {
            const launcher = new Gio.SubprocessLauncher({
                flags: Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_PIPE,
            });
            launcher.set_cwd(directory);
            for (const [name, value] of Object.entries(environment))
                launcher.setenv(name, value, true);
            const process = launcher.spawnv(args);
            let settled = false;
            let timedOut = false;
            let timeoutSource = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT,
                BUILD_TIMEOUT_SECONDS, () => {
                    timeoutSource = 0;
                    if (!settled) {
                        timedOut = true;
                        process.force_exit();
                    }
                    return GLib.SOURCE_REMOVE;
                });
            process.communicate_utf8_async(null, null, (subprocess, result) => {
                settled = true;
                if (timeoutSource) {
                    GLib.source_remove(timeoutSource);
                    timeoutSource = 0;
                }
                try {
                    const [, stdout, stderr] = subprocess.communicate_utf8_finish(result);
                    if (timedOut) {
                        const minutes = BUILD_TIMEOUT_SECONDS / 60;
                        reject(new Error(ngettext('Compilation stopped after %d minute.',
                            'Compilation stopped after %d minutes.', minutes).replace('%d', minutes)));
                        return;
                    }
                    if (!subprocess.get_successful()) {
                        reject(new Error((stderr || stdout || _('The document failed to compile.')).trim()));
                        return;
                    }
                    resolve({ stdout, stderr });
                } catch (error) {
                    if (timedOut) {
                        const minutes = BUILD_TIMEOUT_SECONDS / 60;
                        reject(new Error(ngettext('Compilation stopped after %d minute.',
                            'Compilation stopped after %d minutes.', minutes).replace('%d', minutes)));
                    } else reject(error);
                }
            });
        } catch (error) {
            reject(error);
        }
    });
}

function bibSearchPath(directory) {
    const existing = GLib.getenv('BIBINPUTS') || '';
    return `${directory}${GLib.SEARCHPATH_SEPARATOR_S}${existing}`;
}

function readAuxiliary(path) {
    try {
        const [ok, bytes] = GLib.file_get_contents(path);
        return ok ? new TextDecoder().decode(bytes) : '';
    } catch {
        return '';
    }
}

export function compileLatex(file, callback, onProgress = () => {}) {
    (async () => {
        const engine = availableLatexEngine();
        if (!engine)
            throw new Error(_('No LaTeX compiler was found. Install latexmk, Tectonic, or TeX Live.'));
        onProgress(engine.name === 'tectonic' ?
            _('Compiling with Tectonic · the first build may download support files.') :
            _('Compiling with %s…').replace('%s', engine.name));

        const sourcePath = file.get_path();
        if (!sourcePath) throw new Error(_('The document must be available as a local file.'));
        const directory = GLib.path_get_dirname(sourcePath);
        const basename = GLib.path_get_basename(sourcePath);
        const jobName = basename.replace(/\.tex$/i, '');
        const buildDirectory = buildOutputDirectory(sourcePath);
        try {
            Gio.File.new_for_path(buildDirectory).make_directory_with_parents(null);
        } catch (error) {
            if (!Gio.File.new_for_path(buildDirectory).query_exists(null))
                throw new Error(_('Could not prepare the build folder: %s').replace('%s', error.message));
        }
        const expectedPdf = GLib.build_filenamev([buildDirectory, `${jobName}.pdf`]);
        const bcfFile = Gio.File.new_for_path(
            GLib.build_filenamev([buildDirectory, `${jobName}.bcf`]));
        const environment = { BIBINPUTS: bibSearchPath(directory) };

        let latexArgs;
        if (engine.name === 'latexmk') {
            latexArgs = [engine.path, '-pdf', '-interaction=nonstopmode', '-file-line-error',
                `-outdir=${buildDirectory}`, basename];
        } else if (engine.name === 'tectonic') {
            latexArgs = [engine.path, '--outdir', buildDirectory, '--keep-logs', basename];
        } else {
            latexArgs = [engine.path, '-interaction=nonstopmode', '-file-line-error',
                `-output-directory=${buildDirectory}`, basename];
        }

        if (engine.name === 'pdflatex' && bcfFile.query_exists(null))
            bcfFile.delete(null);
        await runProcess(latexArgs, directory, environment);

        if (engine.name === 'pdflatex') {
            const auxiliaryPath = GLib.build_filenamev([buildDirectory, `${jobName}.aux`]);
            if (bcfFile.query_exists(null)) {
                const biber = GLib.find_program_in_path('biber');
                if (!biber)
                    throw new Error(_('This document uses biblatex and needs Biber. Install Biber, latexmk, or Tectonic.'));
                await runProcess([biber, '--input-directory', buildDirectory,
                    '--output-directory', buildDirectory, jobName], buildDirectory, environment);
                await runProcess(latexArgs, directory);
                await runProcess(latexArgs, directory);
            } else if (/\\bibdata\s*\{/.test(readAuxiliary(auxiliaryPath))) {
                const bibtex = GLib.find_program_in_path('bibtex');
                if (!bibtex)
                    throw new Error(_('This document needs BibTeX. Install BibTeX, latexmk, or Tectonic.'));
                await runProcess([bibtex, jobName], buildDirectory, environment);
                await runProcess(latexArgs, directory);
                await runProcess(latexArgs, directory);
            }
        }

        if (!Gio.File.new_for_path(expectedPdf).query_exists(null))
            throw new Error(_('%s finished without creating the expected PDF. Check that the document has a complete LaTeX structure.')
                .replace('%s', engine.name));

        return { expectedPdf, message: _('%s: compilation complete.').replace('%s', engine.name) };
    })().then(({ expectedPdf, message }) => callback(null, expectedPdf, message), callback);
}

export class PdfPreview {
    constructor() {
        this.document = null;
        this.popplerModule = import('gi://Poppler?version=0.18').catch(() => null);
        this.pageIndex = 0;
        this.pageCount = 0;
        this.path = null;

        this.root = new Gtk.Box({ orientation: Gtk.Orientation.VERTICAL, spacing: 0 });
        this.toolbar = new Gtk.Box({ orientation: Gtk.Orientation.HORIZONTAL, spacing: 6,
            margin_start: 8, margin_end: 8, margin_top: 6, margin_bottom: 6 });
        this.previousButton = new Gtk.Button({ icon_name: 'go-up-symbolic', tooltip_text: _('Previous page') });
        this.nextButton = new Gtk.Button({ icon_name: 'go-down-symbolic', tooltip_text: _('Next page') });
        this.pageLabel = new Gtk.Label({ label: 'PDF', hexpand: true, xalign: 0.5 });
        this.toolbar.append(this.previousButton);
        this.toolbar.append(this.pageLabel);
        this.toolbar.append(this.nextButton);
        this.root.append(this.toolbar);

        this.drawingArea = new Gtk.DrawingArea({ content_width: 420, content_height: 600, hexpand: true,
            halign: Gtk.Align.CENTER });
        this.drawingArea.set_draw_func((_area, context, width, _height) => this._draw(context, width));
        this.scroll = new Gtk.ScrolledWindow({
            hscrollbar_policy: Gtk.PolicyType.AUTOMATIC,
            vscrollbar_policy: Gtk.PolicyType.AUTOMATIC,
            child: this.drawingArea,
            vexpand: true,
        });
        this.root.append(this.scroll);

        this.previousButton.connect('clicked', () => this.setPage(this.pageIndex - 1));
        this.nextButton.connect('clicked', () => this.setPage(this.pageIndex + 1));
        this._updateControls();
    }

    setMessage(message) {
        this.document = null;
        this.path = null;
        this.pageLabel.set_label(message);
        this.drawingArea.queue_draw();
        this._updateControls();
    }

    async open(path) {
        try {
            const poppler = await this.popplerModule;
            if (!poppler) {
                const uri = Gio.File.new_for_path(path).get_uri();
                Gio.AppInfo.launch_default_for_uri(uri, null);
                this.setMessage(_('PDF opened in the default viewer'));
                return;
            }
            const file = Gio.File.new_for_path(path);
            this.document = poppler.default.Document.new_from_file(file.get_uri(), null);
            this.pageCount = this.document.get_n_pages();
            this.pageIndex = 0;
            this.path = path;
            this._updateControls();
            this.drawingArea.queue_draw();
        } catch (error) {
            this.setMessage(_('Could not open the PDF'));
            throw error;
        }
    }

    setPage(index) {
        if (!this.document) return;
        this.pageIndex = Math.max(0, Math.min(index, this.pageCount - 1));
        this._updateControls();
        this.drawingArea.queue_draw();
    }

    _updateControls() {
        const hasPdf = Boolean(this.document);
        this.pageLabel.set_label(hasPdf ? `${this.pageIndex + 1} / ${this.pageCount}` : this.pageLabel.get_label());
        this.previousButton.set_sensitive(hasPdf && this.pageIndex > 0);
        this.nextButton.set_sensitive(hasPdf && this.pageIndex + 1 < this.pageCount);
    }

    _draw(context, width) {
        context.set_source_rgb(0.92, 0.93, 0.95);
        context.paint();
        if (!this.document) return;

        const page = this.document.get_page(this.pageIndex);
        if (!page) return;
        const [pageWidth, pageHeight] = page.get_size();
        const scale = Math.min(1, (width - 32) / pageWidth);
        const displayWidth = pageWidth * scale;
        const displayHeight = pageHeight * scale;
        this.drawingArea.set_content_height(Math.ceil(displayHeight + 24));
        context.set_source_rgb(1, 1, 1);
        context.rectangle((width - displayWidth) / 2, 12, displayWidth, displayHeight);
        context.fill();
        context.save();
        context.translate((width - displayWidth) / 2, 12);
        context.scale(scale, scale);
        page.render(context);
        context.restore();
    }
}
