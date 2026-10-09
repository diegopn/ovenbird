use gtk::prelude::*;
use std::cell::Cell;
#[cfg(feature = "poppler-preview")]
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

#[cfg(feature = "poppler-preview")]
mod poppler {
    use gtk::gio::prelude::FileExt;
    use gtk::glib::{self, translate::*};
    use std::ffi::{c_char, CString};
    use std::path::Path;

    #[repr(C)]
    pub struct Document {
        _private: [u8; 0],
    }
    #[repr(C)]
    pub struct Page {
        _private: [u8; 0],
    }

    #[link(name = "poppler-glib")]
    unsafe extern "C" {
        fn poppler_document_get_type() -> glib::ffi::GType;
        fn poppler_page_get_type() -> glib::ffi::GType;
        fn poppler_document_new_from_file(
            uri: *const c_char,
            password: *const c_char,
            error: *mut *mut glib::ffi::GError,
        ) -> *mut Document;
        fn poppler_document_get_n_pages(document: *mut Document) -> i32;
        fn poppler_document_get_page(document: *mut Document, index: i32) -> *mut Page;
        fn poppler_page_get_size(page: *mut Page, width: *mut f64, height: *mut f64);
        fn poppler_page_render(page: *mut Page, cairo: *mut gtk::cairo::ffi::cairo_t);
    }

    glib::wrapper! {
        pub struct PdfDocument(Object<Document>);
        match fn {
            type_ => || poppler_document_get_type(),
        }
    }

    glib::wrapper! {
        pub struct PdfPage(Object<Page>);
        match fn {
            type_ => || poppler_page_get_type(),
        }
    }

    impl PdfDocument {
        pub fn open(path: &Path) -> Result<Self, String> {
            let uri = gtk::gio::File::for_path(path).uri();
            let uri = CString::new(uri.as_str()).map_err(|error| error.to_string())?;
            let mut error = std::ptr::null_mut();
            let document = unsafe {
                poppler_document_new_from_file(uri.as_ptr(), std::ptr::null(), &mut error)
            };
            if document.is_null() {
                if error.is_null() {
                    return Err("Poppler could not open the PDF.".to_owned());
                }
                let error: glib::Error = unsafe { from_glib_full(error) };
                return Err(error.to_string());
            }
            Ok(unsafe { from_glib_full(document) })
        }

        pub fn page_count(&self) -> i32 {
            unsafe { poppler_document_get_n_pages(self.to_glib_none().0) }
        }

        pub fn page(&self, index: i32) -> Option<PdfPage> {
            let page = unsafe { poppler_document_get_page(self.to_glib_none().0, index) };
            (!page.is_null()).then(|| unsafe { from_glib_none(page) })
        }
    }

    impl PdfPage {
        pub fn size(&self) -> (f64, f64) {
            let (mut width, mut height) = (0.0, 0.0);
            unsafe { poppler_page_get_size(self.to_glib_none().0, &mut width, &mut height) };
            (width, height)
        }

        pub fn render(&self, context: &gtk::cairo::Context) {
            unsafe { poppler_page_render(self.to_glib_none().0, context.to_raw_none()) };
        }
    }
}

pub struct PdfPreview {
    root: gtk::Box,
    message: gtk::Label,
    page_label: gtk::Label,
    stack: gtk::Stack,
    #[cfg(feature = "poppler-preview")]
    drawing: gtk::DrawingArea,
    #[cfg(feature = "poppler-preview")]
    scroll: gtk::ScrolledWindow,
    previous: gtk::Button,
    next: gtk::Button,
    #[cfg(feature = "poppler-preview")]
    document: Rc<RefCell<Option<poppler::PdfDocument>>>,
    #[cfg(feature = "poppler-preview")]
    page_sizes: Rc<RefCell<Vec<(f64, f64)>>>,
    #[cfg(feature = "poppler-preview")]
    page_layouts: Rc<RefCell<Vec<PageLayout>>>,
    page_index: Rc<Cell<i32>>,
    page_count: Rc<Cell<i32>>,
}

#[cfg(feature = "poppler-preview")]
#[derive(Clone, Copy)]
struct PageLayout {
    index: i32,
    top: f64,
    width: f64,
    height: f64,
    scale: f64,
}

#[cfg(feature = "poppler-preview")]
const PAGE_MARGIN: f64 = 12.0;
#[cfg(feature = "poppler-preview")]
const PAGE_GAP: f64 = 24.0;
const PDF_CONTENT_WIDTH: i32 = 420;
#[cfg(feature = "poppler-preview")]
const MIN_ZOOM: f64 = 0.5;
#[cfg(feature = "poppler-preview")]
const MAX_ZOOM: f64 = 3.0;
#[cfg(feature = "poppler-preview")]
const ZOOM_STEP: f64 = 0.1;

#[cfg(feature = "poppler-preview")]
fn update_zoom_controls(
    zoom_factor: &Cell<f64>,
    value: f64,
    zoom_label: &gtk::Label,
    zoom_out: &gtk::Button,
    zoom_in: &gtk::Button,
    drawing: &gtk::DrawingArea,
) {
    let value = value.clamp(MIN_ZOOM, MAX_ZOOM);
    zoom_factor.set(value);
    zoom_label.set_label(&format!("{}%", (value * 100.0).round() as i32));
    zoom_out.set_sensitive(value > MIN_ZOOM);
    zoom_in.set_sensitive(value < MAX_ZOOM);
    drawing.set_content_width((PDF_CONTENT_WIDTH as f64 * value.max(1.0)).ceil() as i32);
    drawing.queue_resize();
    drawing.queue_draw();
}

#[cfg(feature = "poppler-preview")]
fn update_visible_page(
    adjustment: &gtk::Adjustment,
    layouts: &[PageLayout],
    page_index: &Cell<i32>,
    page_count: i32,
    page_label: &gtk::Label,
    previous: &gtk::Button,
    next: &gtk::Button,
) {
    if page_count <= 0 {
        return;
    }

    let probe_y = adjustment.value() + adjustment.page_size() / 2.0;
    let visible_page = layouts
        .iter()
        .find(|layout| {
            layout.height > 0.0 && probe_y <= layout.top + layout.height + PAGE_GAP / 2.0
        })
        .or_else(|| layouts.last());
    if let Some(layout) = visible_page {
        let index = layout.index;
        page_index.set(index);
        page_label.set_label(&format!("{} / {page_count}", index + 1));
        previous.set_sensitive(index > 0);
        next.set_sensitive(index + 1 < page_count);
    }
}

impl Default for PdfPreview {
    fn default() -> Self {
        Self::new()
    }
}

impl PdfPreview {
    pub fn new() -> Self {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let toolbar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        toolbar.set_margin_start(8);
        toolbar.set_margin_end(8);
        toolbar.set_margin_top(6);
        toolbar.set_margin_bottom(6);
        let previous = gtk::Button::from_icon_name("go-up-symbolic");
        previous.set_tooltip_text(Some(&tr("Previous page")));
        let page_label = gtk::Label::new(Some("PDF"));
        page_label.set_hexpand(true);
        let next = gtk::Button::from_icon_name("go-down-symbolic");
        next.set_tooltip_text(Some(&tr("Next page")));
        toolbar.append(&previous);
        toolbar.append(&page_label);
        toolbar.append(&next);
        #[cfg(feature = "poppler-preview")]
        let (zoom_factor, zoom_out, zoom_label, zoom_in) = {
            let zoom_out = gtk::Button::from_icon_name("zoom-out-symbolic");
            zoom_out.set_tooltip_text(Some(&tr("Zoom out")));
            let zoom_label = gtk::Label::new(Some("100%"));
            zoom_label.set_width_chars(5);
            zoom_label.set_xalign(0.5);
            let zoom_in = gtk::Button::from_icon_name("zoom-in-symbolic");
            zoom_in.set_tooltip_text(Some(&tr("Zoom in")));
            let separator = gtk::Separator::new(gtk::Orientation::Vertical);
            toolbar.append(&separator);
            toolbar.append(&zoom_out);
            toolbar.append(&zoom_label);
            toolbar.append(&zoom_in);
            (Rc::new(Cell::new(1.0)), zoom_out, zoom_label, zoom_in)
        };
        root.append(&toolbar);

        let stack = gtk::Stack::builder().hexpand(true).vexpand(true).build();
        let message = gtk::Label::new(Some(&tr("The compiled PDF will appear here")));
        message.set_wrap(true);
        message.add_css_class("dim-label");
        stack.add_named(&message, Some("empty"));
        let drawing = gtk::DrawingArea::builder()
            .content_width(PDF_CONTENT_WIDTH)
            .content_height(600)
            .hexpand(true)
            .halign(gtk::Align::Center)
            .build();
        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Automatic)
            .vscrollbar_policy(gtk::PolicyType::Automatic)
            .vexpand(true)
            .child(&drawing)
            .build();
        stack.add_named(&scroll, Some("pdf"));
        stack.set_visible_child_name("empty");
        root.append(&stack);

        let page_index = Rc::new(Cell::new(0));
        let page_count = Rc::new(Cell::new(0));
        #[cfg(feature = "poppler-preview")]
        let document: Rc<RefCell<Option<poppler::PdfDocument>>> = Rc::new(RefCell::new(None));
        #[cfg(feature = "poppler-preview")]
        let page_sizes = Rc::new(RefCell::new(Vec::new()));
        #[cfg(feature = "poppler-preview")]
        let page_layouts = Rc::new(RefCell::new(Vec::new()));

        #[cfg(feature = "poppler-preview")]
        {
            let document = document.clone();
            let page_sizes = page_sizes.clone();
            let page_layouts = page_layouts.clone();
            let zoom_factor = zoom_factor.clone();
            drawing.set_draw_func(move |area, context, width, height| {
                context.set_source_rgb(0.92, 0.93, 0.95);
                let _ = context.paint();
                let document = document.borrow();
                let Some(document) = document.as_ref() else {
                    return;
                };
                if width <= 0 {
                    return;
                }

                let zoom = zoom_factor.get();
                let available_width = (PDF_CONTENT_WIDTH as f64 - 32.0).max(42.0);
                let mut top = PAGE_MARGIN;
                let layouts: Vec<_> = page_sizes
                    .borrow()
                    .iter()
                    .enumerate()
                    .map(|(index, (page_width, page_height))| {
                        let (display_width, display_height, scale) =
                            if *page_width > 0.0 && *page_height > 0.0 {
                                let scale = (available_width / page_width).clamp(0.1, 1.0) * zoom;
                                (page_width * scale, page_height * scale, scale)
                            } else {
                                (0.0, 0.0, 0.0)
                            };
                        let layout = PageLayout {
                            index: index as i32,
                            top,
                            width: display_width,
                            height: display_height,
                            scale,
                        };
                        top += display_height + PAGE_GAP;
                        layout
                    })
                    .collect();
                let content_height = (top - PAGE_GAP + PAGE_MARGIN).ceil().max(600.0) as i32;
                *page_layouts.borrow_mut() = layouts;
                area.set_content_height(content_height);

                let (_, clip_top, _, clip_bottom) =
                    context
                        .clip_extents()
                        .unwrap_or((0.0, 0.0, width as f64, height as f64));
                for layout in page_layouts.borrow().iter().copied() {
                    if layout.height <= 0.0
                        || layout.top + layout.height < clip_top
                        || layout.top > clip_bottom
                    {
                        continue;
                    }
                    let Some(page) = document.page(layout.index) else {
                        continue;
                    };
                    let left = (width as f64 - layout.width) / 2.0;
                    context.set_source_rgb(1.0, 1.0, 1.0);
                    context.rectangle(left, layout.top, layout.width, layout.height);
                    let _ = context.fill();
                    let _ = context.save();
                    context.translate(left, layout.top);
                    context.scale(layout.scale, layout.scale);
                    page.render(context);
                    let _ = context.restore();
                }
            });
        }

        #[cfg(feature = "poppler-preview")]
        {
            let zoom_factor_out = zoom_factor.clone();
            let zoom_label_out = zoom_label.clone();
            let zoom_out_out = zoom_out.clone();
            let zoom_in_out = zoom_in.clone();
            let drawing_out = drawing.clone();
            zoom_out.connect_clicked(move |_| {
                update_zoom_controls(
                    &zoom_factor_out,
                    zoom_factor_out.get() - ZOOM_STEP,
                    &zoom_label_out,
                    &zoom_out_out,
                    &zoom_in_out,
                    &drawing_out,
                );
            });

            let zoom_factor_in = zoom_factor.clone();
            let zoom_label_in = zoom_label.clone();
            let zoom_out_in = zoom_out.clone();
            let zoom_in_in = zoom_in.clone();
            let drawing_in = drawing.clone();
            zoom_in.connect_clicked(move |_| {
                update_zoom_controls(
                    &zoom_factor_in,
                    zoom_factor_in.get() + ZOOM_STEP,
                    &zoom_label_in,
                    &zoom_out_in,
                    &zoom_in_in,
                    &drawing_in,
                );
            });
        }

        #[cfg(feature = "poppler-preview")]
        {
            let value_adjustment = scroll.vadjustment();
            let page_index_value = page_index.clone();
            let page_count_value = page_count.clone();
            let page_layouts_value = page_layouts.clone();
            let page_label_value = page_label.clone();
            let previous_value = previous.clone();
            let next_value = next.clone();
            value_adjustment.connect_value_changed(move |adjustment| {
                update_visible_page(
                    adjustment,
                    &page_layouts_value.borrow(),
                    &page_index_value,
                    page_count_value.get(),
                    &page_label_value,
                    &previous_value,
                    &next_value,
                );
            });

            let page_index_changed = page_index.clone();
            let page_count_changed = page_count.clone();
            let page_layouts_changed = page_layouts.clone();
            let page_label_changed = page_label.clone();
            let previous_changed = previous.clone();
            let next_changed = next.clone();
            value_adjustment.connect_changed(move |adjustment| {
                update_visible_page(
                    adjustment,
                    &page_layouts_changed.borrow(),
                    &page_index_changed,
                    page_count_changed.get(),
                    &page_label_changed,
                    &previous_changed,
                    &next_changed,
                );
            });

            let page_layouts_prev = page_layouts.clone();
            let page_index_prev = page_index.clone();
            let previous_adjustment = scroll.vadjustment().downgrade();
            previous.connect_clicked(move |_| {
                let index = (page_index_prev.get() - 1).max(0) as usize;
                if let Some(layout) = page_layouts_prev.borrow().get(index) {
                    if let Some(adjustment) = previous_adjustment.upgrade() {
                        adjustment.set_value((layout.top - PAGE_MARGIN).max(0.0));
                    }
                }
            });

            let page_layouts_next = page_layouts.clone();
            let page_index_next = page_index.clone();
            let page_count_next = page_count.clone();
            let next_adjustment = scroll.vadjustment().downgrade();
            next.connect_clicked(move |_| {
                let index = (page_index_next.get() + 1).min(page_count_next.get() - 1) as usize;
                if let Some(layout) = page_layouts_next.borrow().get(index) {
                    if let Some(adjustment) = next_adjustment.upgrade() {
                        adjustment.set_value((layout.top - PAGE_MARGIN).max(0.0));
                    }
                }
            });
        }

        #[cfg(not(feature = "poppler-preview"))]
        {
            let page_label_prev = page_label.clone();
            let page_index_prev = page_index.clone();
            let page_count_prev = page_count.clone();
            let drawing_prev = drawing.clone();
            let next_prev = next.clone();
            previous.connect_clicked(move |button| {
                let index = (page_index_prev.get() - 1).max(0);
                page_index_prev.set(index);
                page_label_prev.set_label(&format!("{} / {}", index + 1, page_count_prev.get()));
                button.set_sensitive(index > 0);
                next_prev.set_sensitive(index + 1 < page_count_prev.get());
                drawing_prev.queue_draw();
            });
            let page_label_next = page_label.clone();
            let page_index_next = page_index.clone();
            let page_count_next = page_count.clone();
            let drawing_next = drawing.clone();
            let previous_next = previous.clone();
            next.connect_clicked(move |button| {
                let index = (page_index_next.get() + 1).min(page_count_next.get() - 1);
                page_index_next.set(index);
                page_label_next.set_label(&format!("{} / {}", index + 1, page_count_next.get()));
                previous_next.set_sensitive(index > 0);
                button.set_sensitive(index + 1 < page_count_next.get());
                drawing_next.queue_draw();
            });
        }
        previous.set_sensitive(false);
        next.set_sensitive(false);

        Self {
            root,
            message,
            page_label,
            stack,
            #[cfg(feature = "poppler-preview")]
            drawing,
            #[cfg(feature = "poppler-preview")]
            scroll,
            previous,
            next,
            #[cfg(feature = "poppler-preview")]
            document,
            #[cfg(feature = "poppler-preview")]
            page_sizes,
            #[cfg(feature = "poppler-preview")]
            page_layouts,
            page_index,
            page_count,
        }
    }

    pub fn widget(&self) -> &gtk::Box {
        &self.root
    }

    pub fn set_message(&self, message: &str) {
        #[cfg(feature = "poppler-preview")]
        self.document.borrow_mut().take();
        #[cfg(feature = "poppler-preview")]
        self.page_sizes.borrow_mut().clear();
        #[cfg(feature = "poppler-preview")]
        self.page_layouts.borrow_mut().clear();
        self.page_index.set(0);
        self.page_count.set(0);
        self.message.set_label(message);
        self.page_label.set_label("PDF");
        self.previous.set_sensitive(false);
        self.next.set_sensitive(false);
        self.stack.set_visible_child_name("empty");
    }

    pub fn open(&self, path: &Path) -> Result<(), String> {
        #[cfg(feature = "poppler-preview")]
        {
            let document = poppler::PdfDocument::open(path)?;
            let count = document.page_count();
            if count <= 0 {
                return Err(tr("The PDF has no pages."));
            }
            let page_sizes: Vec<_> = (0..count)
                .map(|index| {
                    document
                        .page(index)
                        .map(|page| page.size())
                        .unwrap_or((0.0, 0.0))
                })
                .collect();
            *self.document.borrow_mut() = Some(document);
            *self.page_sizes.borrow_mut() = page_sizes;
            self.page_layouts.borrow_mut().clear();
            self.page_index.set(0);
            self.page_count.set(count);
            self.page_label.set_label(&format!("1 / {count}"));
            self.previous.set_sensitive(false);
            self.next.set_sensitive(count > 1);
            self.stack.set_visible_child_name("pdf");
            self.scroll.vadjustment().set_value(0.0);
            self.drawing.queue_resize();
            self.drawing.queue_draw();
            Ok(())
        }

        #[cfg(not(feature = "poppler-preview"))]
        {
            let uri = gtk::gio::File::for_path(path).uri();
            gtk::gio::AppInfo::launch_default_for_uri(&uri, None::<&gtk::gio::AppLaunchContext>)
                .map_err(|error| error.to_string())?;
            self.set_message(&tr("PDF opened in the default viewer"));
            Ok(())
        }
    }
}

fn tr(message: &str) -> String {
    crate::i18n::gettext(message)
}
