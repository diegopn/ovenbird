use super::*;

pub(in crate::app::window) fn build_header(state: &Rc<State>) {
    let header = adw::HeaderBar::new();
    header.add_css_class("ovenbird-main-toolbar");
    header.set_size_request(-1, MAIN_HEADER_HEIGHT);

    let omni_bar = crate::app::libpanel::OmniBar::new();
    let center = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    center.set_valign(gtk::Align::Center);
    state.editor_ui.title.set_label(&tr("LaTeX editor"));
    center.append(&state.editor_ui.title);

    let build_status = build_status_widgets(state.editor_ui.status.clone());
    center.append(&build_status.status_row);

    let reference_summary_group = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    reference_summary_group.set_valign(gtk::Align::Center);
    reference_summary_group.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let reference_summary = gtk::Label::new(None);
    reference_summary.add_css_class("dim-label");
    reference_summary_group.append(&reference_summary);
    reference_summary_group.set_visible(false);
    center.append(&reference_summary_group);

    omni_bar.add_prefix(0, &center);
    omni_bar.set_action_name("win.compile");
    omni_bar.set_action_tooltip(&tr("Compile and create PDF"));
    omni_bar.set_icon_name("media-playback-start-symbolic");
    header.set_title_widget(Some(omni_bar.widget()));

    state
        .build_ui
        .reference_summary
        .replace(Some(reference_summary));
    state
        .reference_summary_group
        .replace(Some(reference_summary_group));
    state
        .build_ui
        .build_status_widgets
        .replace(Some(build_status));
    state.build_ui.omni_bar.replace(Some(omni_bar));

    let toggle = button("sidebar-show-symbolic", &tr("Show or hide the sidebar"));
    toggle.set_action_name(Some("win.toggle-sidebar"));
    header.pack_start(&toggle);

    let weak_state = Rc::downgrade(state);
    state
        .window_ui
        .layout
        .connect_position_notify(move |layout| {
            if layout.start_child().is_some() {
                let width = layout.position();
                if width > 0 {
                    if let Some(state) = weak_state.upgrade() {
                        state.window_ui.sidebar_width.set(width);
                    }
                }
            }
        });

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let save = button("document-save-symbolic", &tr("Save document"));
    save.set_action_name(Some("win.save-document"));
    save.add_css_class("flat");
    actions.append(&save);
    state.build_ui.save_button.replace(Some(save));
    header.pack_end(&actions);

    install_main_header(&state.window_ui.toolbar_view, &header);
    update_primary_action(state);
    refresh_build_status(state);
}

pub(in crate::app::window) fn install_main_header(
    toolbar_view: &adw::ToolbarView,
    header: &adw::HeaderBar,
) {
    header.set_decoration_layout(Some("minimize,maximize,close"));
    header.set_show_start_title_buttons(false);
    header.set_show_end_title_buttons(true);
    toolbar_view.add_top_bar(header);
}
