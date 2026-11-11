use gtk::glib::{
    object::ObjectType,
    translate::{from_glib_full, ToGlibPtr},
};
use gtk::prelude::*;
use std::ffi::c_void;

#[link(name = "panel-1")]
unsafe extern "C" {
    fn panel_init();
    fn panel_omni_bar_new() -> *mut gtk4_sys::GtkWidget;
    fn panel_omni_bar_add_prefix(
        self_: *mut c_void,
        priority: i32,
        widget: *mut gtk4_sys::GtkWidget,
    );
    fn panel_omni_bar_start_pulsing(self_: *mut c_void);
    fn panel_omni_bar_stop_pulsing(self_: *mut c_void);
}

pub fn initialize() {
    unsafe { panel_init() }
}

#[derive(Clone)]
pub struct OmniBar {
    widget: gtk::Widget,
}

impl OmniBar {
    pub fn new() -> Self {
        let widget: gtk::Widget = unsafe { from_glib_full(panel_omni_bar_new()) };
        Self { widget }
    }

    pub fn widget(&self) -> &gtk::Widget {
        &self.widget
    }

    pub fn add_prefix<W: IsA<gtk::Widget>>(&self, priority: i32, widget: &W) {
        let widget: &gtk::Widget = widget.as_ref();
        unsafe {
            panel_omni_bar_add_prefix(
                self.widget.as_ptr().cast::<c_void>(),
                priority,
                widget.as_ptr(),
            );
        }
    }

    pub fn set_action_name(&self, name: &str) {
        unsafe {
            gtk4_sys::gtk_actionable_set_action_name(
                self.widget.as_ptr().cast::<gtk4_sys::GtkActionable>(),
                name.to_glib_none().0,
            );
        }
    }

    pub fn set_action_tooltip(&self, tooltip: &str) {
        self.widget.set_property("action-tooltip", tooltip);
    }

    pub fn set_icon_name(&self, name: &str) {
        self.widget.set_property("icon-name", name);
    }

    pub fn set_popover(&self, popover: Option<&gtk::Popover>) {
        self.widget.set_property("popover", popover);
    }

    pub fn set_menu_model(&self, model: Option<&gtk::gio::MenuModel>) {
        self.widget.set_property("menu-model", model);
    }

    pub fn start_pulsing(&self) {
        unsafe { panel_omni_bar_start_pulsing(self.widget.as_ptr().cast::<c_void>()) }
    }

    pub fn stop_pulsing(&self) {
        unsafe { panel_omni_bar_stop_pulsing(self.widget.as_ptr().cast::<c_void>()) }
    }
}
