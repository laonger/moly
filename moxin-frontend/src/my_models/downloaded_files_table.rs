use std::collections::HashMap;

use makepad_widgets::*;
use moxin_protocol::data::DownloadedFile;

use crate::{data::store::Store, shared::utils::format_model_size};

use super::{
    downloaded_files_row::DownloadedFilesRowWidgetRefExt, my_models_screen::MyModelsSearchAction,
};

live_design! {
    import makepad_widgets::base::*;
    import makepad_widgets::theme_desktop_dark::*;

    import crate::shared::styles::*;
    import crate::shared::widgets::*;

    import crate::my_models::downloaded_files_row::DownloadedFilesRow

    RowHeaderLabel = <View> {
        width: 100
        height: Fit
        align: {x: 0.0, y: 0.5}
        label = <Label> {
            width: Fit
            draw_text: {
                text_style: <BOLD_FONT>{font_size: 9}
                color: #667085
            }
        }
    }

    HeaderRow = <View> {
        align: {x: 0.0, y: 0.5}
        width: Fill
        height: Fit
        padding: {top: 10, bottom: 10, left: 20, right: 20}
        // Heads-up: the spacing and row header widths need to match the row values
        spacing: 30,
        show_bg: true
        draw_bg: {
            color: #F2F4F7;
        }

        <RowHeaderLabel> { width: 600, label = {text: "Model File"} }
        <RowHeaderLabel> { width: 100, label = {text: "File Size"} }
        <RowHeaderLabel> { width: 100, label = {text: "Added Date"} }
        <RowHeaderLabel> { width: 250, label = {text: ""} }
    }

    DownloadedFilesTable = {{DownloadedFilesTable}} <RoundedView> {
        width: Fill,
        height: Fill,
        align: {x: 0.5, y: 0.5}

        list = <PortalList>{
            drag_scrolling: false
            HeaderRow = <HeaderRow> {}
            ItemRow = <DownloadedFilesRow> {}
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct DownloadedFilesTable {
    #[deref]
    view: View,
    #[rust]
    file_item_map: HashMap<u64, String>,
    #[rust]
    current_results: Vec<DownloadedFile>,
    #[rust]
    latest_store_fetch_len: usize,
    #[rust]
    search_status: SearchStatus,
}

impl Widget for DownloadedFilesTable {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.file_item_map.clear();
        let filter = match &self.search_status {
            SearchStatus::Filtered(keywords) => Some(keywords.clone()),
            _ => None,
        };

        // If we're already filtering, re-apply filter over the new store data
        // only re-filtering if there are new downloads in store
        match filter {
            Some(keywords) => {
                if self.latest_store_fetch_len
                    != scope.data.get::<Store>().unwrap().downloaded_files.len()
                {
                    self.filter_by_keywords(cx, scope, &keywords)
                }
            }
            None => self.fetch_results(scope),
        };

        self.current_results
            .sort_by(|a, b| b.downloaded_at.cmp(&a.downloaded_at));

        let entries_count = self.current_results.len();
        let last_item_id = if entries_count > 0 { entries_count } else { 0 };

        let mut current_chat_file_id = None;
        if let Some(current_chat) = &scope.data.get::<Store>().unwrap().get_current_chat() {
            current_chat_file_id = Some(current_chat.borrow().file_id.clone());
        }

        while let Some(item) = self.view.draw_walk(cx, scope, walk).step() {
            if let Some(mut list) = item.as_portal_list().borrow_mut() {
                list.set_item_range(cx, 0, last_item_id);
                while let Some(item_id) = list.next_visible_item(cx) {
                    if item_id <= last_item_id {
                        let template;
                        if item_id == 0 {
                            // Draw header row
                            template = live_id!(HeaderRow);
                            let item = list.item(cx, item_id, template).unwrap();
                            item.draw_all(cx, scope);
                            continue;
                        }

                        template = live_id!(ItemRow);
                        let item = list.item(cx, item_id, template).unwrap();

                        let file_data = &self.current_results[item_id - 1];

                        self.file_item_map
                            .insert(item.widget_uid().0, file_data.file.id.clone());

                        let mut downloaded_files_row_widget = item.as_downloaded_files_row();
                        downloaded_files_row_widget.set_file_id(file_data.file.id.clone());

                        let mut is_model_file_loaded = false;
                        if let Some(file_id) = &current_chat_file_id {
                            is_model_file_loaded = *file_id == file_data.file.id;
                        }
                        downloaded_files_row_widget.set_show_resume_button(is_model_file_loaded);

                        // Name tag
                        let name = human_readable_name(&file_data.file.name);
                        item.label(id!(h_wrapper.model_file.h_wrapper.name_tag.name))
                            .set_text(&name);

                        // Base model tag
                        let base_model = dash_if_empty(&file_data.model.architecture);
                        item.label(id!(h_wrapper
                            .model_file
                            .base_model_tag
                            .base_model
                            .attr_name))
                            .set_text(&base_model);

                        // Parameters tag
                        let parameters = dash_if_empty(&file_data.model.size);
                        item.label(id!(h_wrapper
                            .model_file
                            .parameters_tag
                            .parameters
                            .attr_name))
                            .set_text(&parameters);

                        // Version tag
                        let filename = format!("{}/{}", file_data.model.name, file_data.file.name);
                        item.label(id!(h_wrapper.model_file.model_version_tag.version))
                            .set_text(&filename);

                        // File size tag
                        let file_size =
                            format_model_size(&file_data.file.size).unwrap_or("-".to_string());
                        item.label(id!(h_wrapper.file_size_tag.file_size))
                            .set_text(&file_size);

                        // Added date tag
                        let formatted_date = file_data.downloaded_at.format("%d/%m/%Y").to_string();
                        item.label(id!(h_wrapper.date_added_tag.date_added))
                            .set_text(&formatted_date);

                        // Don't draw separator line on first row
                        if item_id == 1 {
                            item.view(id!(separator_line)).set_visible(false);
                        }

                        item.draw_all(cx, scope);
                    }
                }
            }
        }
        DrawStep::done()
    }
}

impl WidgetMatchEvent for DownloadedFilesTable {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, scope: &mut Scope) {
        for action in actions.iter() {
            match action.cast() {
                MyModelsSearchAction::Search(keywords) => {
                    self.filter_by_keywords(cx, scope, &keywords);
                }
                MyModelsSearchAction::Reset => {
                    self.reset_results(cx, scope);
                }
                _ => {}
            }
        }
    }
}

impl DownloadedFilesTable {
    fn fetch_results(&mut self, scope: &mut Scope) {
        self.current_results = scope.data.get::<Store>().unwrap().downloaded_files.clone();
        self.latest_store_fetch_len = self.current_results.len();
    }

    fn filter_by_keywords(&mut self, cx: &mut Cx, scope: &mut Scope, keywords: &str) {
        let keywords = keywords.to_lowercase();
        self.current_results = scope.data.get::<Store>().unwrap().downloaded_files.clone();
        self.latest_store_fetch_len = self.current_results.len();

        self.current_results.retain(|f| {
            f.file.name.to_lowercase().contains(&keywords)
                || f.model.name.to_lowercase().contains(&keywords)
        });

        self.search_status = SearchStatus::Filtered(keywords);
        self.redraw(cx);
    }

    fn reset_results(&mut self, cx: &mut Cx, scope: &mut Scope) {
        self.current_results = scope.data.get::<Store>().unwrap().downloaded_files.clone();

        self.search_status = SearchStatus::Idle;
        self.redraw(cx);
    }
}

#[derive(Clone, Default, Debug)]
enum SearchStatus {
    #[default]
    Idle,
    Filtered(String),
}

/// Removes dashes, file extension, and capitalizes the first letter of each word.
fn human_readable_name(name: &str) -> String {
    let name = name
        .to_lowercase()
        .replace("-", " ")
        .replace(".gguf", "")
        .replace("chat", "");

    let name = name
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first_char) => first_char.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<String>>()
        .join(" ");

    name
}

fn dash_if_empty(input: &str) -> &str {
    if input.is_empty() {
        "-"
    } else {
        input
    }
}
