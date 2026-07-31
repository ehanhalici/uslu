// src/main.rs
use iced::keyboard::key::Named as NamedKey;
use iced::{keyboard, window, Event, Point, Subscription, Task};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use uslu::canvas::{CanvasData, CanvasMessage, Viewport};
use uslu::file_selector::{FileSelectorMessage, FileSelectorState, FileSelectorView};
use uslu::image::{ImageCropperState, ImageManager};
use uslu::models::{FocusGraph, FocusNode};
use uslu::notebox::{NoteBoxMessage, OrgFormatter};
use uslu::orgmode::OrgmodeIO;
use uslu::sidebar::{self, NodeForm, SidebarMessage, TabType};
use uslu::sugiyama::SugiyamaEngine;

use uuid::Uuid;

const APP_TITLE: &str = "Uslu — Focus Tree";
const APP_WINDOW_WIDTH: f32 = 1280.0;
const APP_WINDOW_HEIGHT: f32 = 800.0;

const DEFAULT_TREE_FILE_PATH: &str = "tree.org";
const DEFAULT_IMAGES_FILE_PATH: &str = "images.org";

const AUTOSAVE_INTERVAL_SECS: u64 = 60;
const PERIODIC_TICK_SECS: u64 = 1;

const NEW_NODE_VIEWPORT_OFFSET_X: f32 = 200.0;
const NEW_NODE_VIEWPORT_OFFSET_Y: f32 = 200.0;

const SCREEN_CENTER_X: f32 = 460.0;
const SCREEN_CENTER_Y: f32 = 400.0;

const MATERIAL_FONT_BYTES: &[u8] = include_bytes!("../assets/material.ttf");

fn main() -> iced::Result {
    iced::application(APP_TITLE, Uslu::update, Uslu::view)
        .subscription(Uslu::subscription)
        .theme(|_| iced::Theme::Dark)
        .font(MATERIAL_FONT_BYTES)
        .window(window::Settings {
            size: iced::Size::new(APP_WINDOW_WIDTH, APP_WINDOW_HEIGHT),
            ..Default::default()
        })
        .run()
}

pub struct Uslu {
    graph: FocusGraph,
    view: Viewport,
    selected: Option<Uuid>,
    form: NodeForm,
    file_path: String,
    images_file_path: String,
    loaded_images: HashMap<String, String>,
    image_cache: HashMap<String, iced::widget::image::Handle>,
    frozen: HashSet<Uuid>,
    open_tabs: Vec<TabType>,

    cropper_state: Option<ImageCropperState>,

    is_shift_pressed: bool,
    is_ctrl_pressed: bool,

    is_editing_enabled: bool,
    max_visible_level: usize,

    is_dirty: bool,
    last_save_time: Instant,

    pub file_selector_state: Option<FileSelectorState>,
}

impl Default for Uslu {
    fn default() -> Self {
        let file_path = DEFAULT_TREE_FILE_PATH.to_string();
        let images_file_path = DEFAULT_IMAGES_FILE_PATH.to_string();

        let mut graph = FocusGraph::default();
        let mut frozen = HashSet::new();

        if std::path::Path::new(&file_path).exists() {
            if let Ok(imported_graph) = OrgmodeIO::import(&file_path) {
                graph = imported_graph;
                for node in &graph.nodes {
                    if node.is_frozen {
                        frozen.insert(node.id);
                    }
                }
                SugiyamaEngine::layout(&mut graph, &frozen);
            }
        }

        let loaded_images = ImageManager::load_all_images(&images_file_path).unwrap_or_default();
        let mut image_cache = HashMap::new();
        for (id, base64_str) in &loaded_images {
            if let Some(handle) = ImageManager::base64_to_handle(base64_str) {
                image_cache.insert(id.clone(), handle);
            }
        }

        let max_lvl = Self::determine_initial_max_level(&graph);

        let mut app = Self {
            graph,
            view: Viewport::default(),
            selected: None,
            form: NodeForm::default(),
            file_path,
            images_file_path,
            loaded_images,
            image_cache,
            frozen,
            open_tabs: Vec::new(),
            cropper_state: None,
            is_shift_pressed: false,
            is_ctrl_pressed: false,
            is_editing_enabled: true,
            max_visible_level: max_lvl,
            is_dirty: false,
            last_save_time: Instant::now(),
            file_selector_state: None,
        };

        app.reset_view_to_center();
        app
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Canvas(CanvasMessage),
    Sidebar(SidebarMessage),
    FileSelector(FileSelectorMessage),
    EventOccurred(Event),
    PeriodicSaveTick,
    ImagePicked(Option<(image::DynamicImage, Vec<u8>)>),
}

impl Uslu {
    fn subscription(&self) -> Subscription<Message> {
        let events = iced::event::listen().map(Message::EventOccurred);
        let timer = iced::time::every(Duration::from_secs(PERIODIC_TICK_SECS))
            .map(|_| Message::PeriodicSaveTick);

        Subscription::batch(vec![events, timer])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Canvas(msg) => self.handle_canvas(msg),
            Message::Sidebar(msg) => return self.handle_sidebar(msg),
            Message::FileSelector(msg) => return self.handle_file_selector(msg),
            Message::ImagePicked(data) => self.handle_image_picked(data),
            Message::PeriodicSaveTick => self.handle_periodic_autosave(),
            Message::EventOccurred(event) => self.handle_system_event(event),
        }
        Task::none()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        if let Some(ref selector_state) = self.file_selector_state {
            let selector_view = FileSelectorView::render(selector_state).map(Message::FileSelector);
            return iced::widget::container(selector_view)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .center_x(iced::Length::Fill)
                .center_y(iced::Length::Fill)
                .into();
        }

        let canvas_data = CanvasData {
            graph: &self.graph,
            view: self.view,
            selected: self.selected,
            is_shift_pressed: self.is_shift_pressed,
            is_ctrl_pressed: self.is_ctrl_pressed,
            loaded_images: &self.loaded_images,
            image_cache: &self.image_cache,
        };

        let canvas_el = uslu::canvas::canvas_view(canvas_data).map(Message::Canvas);
        let selected_node = self.selected.and_then(|id| self.graph.get_node(id));
        let level_counts = Self::calculate_level_counts(&self.graph);

        let sidebar_el = sidebar::sidebar_view(
            &self.form,
            selected_node,
            self.graph.nodes.len(),
            self.graph.edges.len(),
            &self.open_tabs,
            self.cropper_state.as_ref(),
            self.is_editing_enabled,
            &level_counts,
            self.max_visible_level,
        )
        .map(Message::Sidebar);

        iced::widget::row![sidebar_el, canvas_el].spacing(0).into()
    }
}

impl Uslu {
    fn handle_system_event(&mut self, event: Event) {
        match event {
            Event::Window(window::Event::CloseRequested) => self.save_to_disk(),
            Event::Keyboard(kb_event) => {
                if self.file_selector_state.is_some() {
                if let keyboard::Event::KeyPressed { ref key, .. } = kb_event {
                    let _ = self.handle_file_selector(FileSelectorMessage::KeyPressed(key.clone()));
                    return;
                }
            }
                self.handle_keyboard_event(kb_event);
            }
            _ => {}
        }
        
    }

    fn handle_keyboard_event(&mut self, kb_event: keyboard::Event) {
        match kb_event {
            keyboard::Event::KeyPressed { key, modifiers, .. } => {
                if key == keyboard::Key::Named(NamedKey::Shift) {
                    self.is_shift_pressed = true;
                }
                if key == keyboard::Key::Named(NamedKey::Control) || modifiers.control() {
                    self.is_ctrl_pressed = true;
                }
                if key == keyboard::Key::Named(NamedKey::Delete) {
                    self.delete_selected_node();
                }

                #[cfg(target_os = "macos")]
                let is_save = modifiers.super() && key == keyboard::Key::Character("s".into());
                #[cfg(not(target_os = "macos"))]
                let is_save = modifiers.control() && key == keyboard::Key::Character("s".into());

                if is_save {
                    self.save_to_disk();
                }
            }
            keyboard::Event::KeyReleased { key, modifiers, .. } => {
                if key == keyboard::Key::Named(NamedKey::Shift) {
                    self.is_shift_pressed = false;
                }
                if key == keyboard::Key::Named(NamedKey::Control) || !modifiers.control() {
                    self.is_ctrl_pressed = false;
                }
            }
            _ => {}
        }
    }

    fn handle_periodic_autosave(&mut self) {
        if self.is_dirty && self.last_save_time.elapsed() >= Duration::from_secs(AUTOSAVE_INTERVAL_SECS) {
            self.save_to_disk();
        }
    }

    fn handle_image_picked(&mut self, data: Option<(image::DynamicImage, Vec<u8>)>) {
        if let Some((img, bytes)) = data {
            self.cropper_state = Some(ImageCropperState::new(img, bytes));
        }
    }

    fn handle_file_selector(&mut self, msg: FileSelectorMessage) -> Task<Message> {
        if matches!(msg, FileSelectorMessage::CancelSelection) {
            self.file_selector_state = None;
            return Task::none();
        }

        if let Some(ref mut selector_state) = self.file_selector_state {
            if let Some(selected_path) = selector_state.update(msg) {
                self.file_selector_state = None;
                return Task::perform(
                    ImageManager::load_selected_image_file(selected_path),
                    Message::ImagePicked,
                );
            }
        }
        Task::none()
    }

    fn handle_canvas(&mut self, msg: CanvasMessage) {
        match msg {
            CanvasMessage::NodeClicked { id, shift, ctrl } => {
                self.process_node_click(id, shift, ctrl)
            }
            CanvasMessage::DeleteNodeClicked(id) => self.delete_node_by_id(id),
            CanvasMessage::DeleteEdgeClicked { parent_id, child_id } => {
                self.graph.remove_edge(parent_id, child_id);
                self.mark_dirty();
            }
            CanvasMessage::NodeMoved { id, x, y } => self.move_node(id, x, y),
            CanvasMessage::BackgroundClicked => self.deselect_all(),
            CanvasMessage::ViewChanged(new_view) => self.view = new_view,
        }
    }

    fn process_node_click(&mut self, id: Uuid, shift: bool, ctrl: bool) {
        if ctrl {
            self.toggle_node_collapse(id);
            return;
        }

        if shift {
            self.connect_selected_to_target(id);
        }

        self.select_node(id);
    }

    fn toggle_node_collapse(&mut self, id: Uuid) {
        if let Some(node) = self.graph.get_node_mut(id) {
            node.is_collapsed = !node.is_collapsed;
            self.mark_dirty();
        }
    }

    fn connect_selected_to_target(&mut self, target_id: Uuid) {
        if let Some(parent_id) = self.selected {
            if parent_id != target_id {
                self.graph.add_edge(parent_id, target_id);
                self.mark_dirty();
            }
        }
    }

    fn select_node(&mut self, id: Uuid) {
        self.auto_sync_selected();

        self.selected = Some(id);
        self.is_editing_enabled = false;

        if let Some(node) = self.graph.get_node(id) {
            self.form = NodeForm::from_node(node);
        }

        if !self.open_tabs.contains(&TabType::General) {
            self.open_tabs.push(TabType::General);
        }
    }

    fn delete_node_by_id(&mut self, id: Uuid) {
        self.graph.remove_node(id);
        self.frozen.remove(&id);
        if self.selected == Some(id) {
            self.selected = None;
        }
        self.form = NodeForm::default();
        self.mark_dirty();
    }

    fn move_node(&mut self, id: Uuid, x: f32, y: f32) {
        if let Some(node) = self.graph.get_node_mut(id) {
            node.x = x;
            node.y = y;
            node.is_frozen = true;
            self.frozen.insert(id);
            self.mark_dirty();
        }
    }

    fn deselect_all(&mut self) {
        self.auto_sync_selected();
        self.selected = None;
        self.form = NodeForm::default();
    }

    fn handle_sidebar(&mut self, msg: SidebarMessage) -> Task<Message> {
        match msg {
            SidebarMessage::ToggleEditing => self.is_editing_enabled = !self.is_editing_enabled,
            SidebarMessage::MaxLevelSliderChanged(lvl) => self.update_max_visible_level(lvl),
            SidebarMessage::TitleChanged(title) => self.update_form_title(title),
            SidebarMessage::ProgressChanged(progress) => self.update_form_progress(progress),

            SidebarMessage::DescriptionAction(action) => {
                if self.is_editing_enabled || self.selected.is_none() {
                    self.form.description_editor.perform(action);
                    self.auto_sync_selected();
                }
            }
            SidebarMessage::ProgressNotesAction(action) => {
                if self.is_editing_enabled || self.selected.is_none() {
                    self.form.progress_notes_editor.perform(action);
                    self.auto_sync_selected();
                }
            }
            SidebarMessage::OpenImagePicker => {
                if self.is_editing_enabled || self.selected.is_none() {
                    self.file_selector_state = Some(FileSelectorState::new(None));
                }
            }
            SidebarMessage::CropZoomChanged(z) => {
                if let Some(ref mut cropper) = self.cropper_state {
                    cropper.zoom = z;
                    cropper.clamp_pan();
                }
            }
            SidebarMessage::CropPanMoved { delta_x, delta_y } => {
                if let Some(ref mut cropper) = self.cropper_state {
                    cropper.offset_x += delta_x;
                    cropper.offset_y += delta_y;
                    cropper.clamp_pan();
                }
            }
            SidebarMessage::ApplyCropAndSave => self.apply_image_crop(),
            SidebarMessage::CancelCrop => self.cropper_state = None,

            SidebarMessage::ToggleTab(tab) => self.toggle_tab_visibility(tab),
            SidebarMessage::AddNode => self.spawn_new_node_at_viewport(),
            SidebarMessage::DeleteSelected => self.delete_selected_node(),
            SidebarMessage::ResetView => self.reset_layout_and_view(),
            SidebarMessage::DescriptionNoteBox(note_msg) => {
                match note_msg {
                    NoteBoxMessage::EditorActionPerformed(action) => {
                        self.form.description_editor.perform(action);
                        self.auto_sync_selected();
                    }
                    NoteBoxMessage::HeaderFoldToggled(line_idx) => {
                        if !self.form.description_notebox_state.collapsed_line_indices.remove(&line_idx) {
                            self.form.description_notebox_state.collapsed_line_indices.insert(line_idx);
                        }
                    }
                    NoteBoxMessage::TaskStateToggled(line_idx) => {
                        let current_text = self.form.get_description_text();
                        let updated_text = OrgFormatter::toggle_task_state_at_line(&current_text, line_idx);
                        self.form.description_editor = iced::widget::text_editor::Content::with_text(&updated_text);
                        self.auto_sync_selected();
                    }
                }
            }

            SidebarMessage::ProgressNotesNoteBox(note_msg) => {
                match note_msg {
                    NoteBoxMessage::EditorActionPerformed(action) => {
                        self.form.progress_notes_editor.perform(action);
                        self.auto_sync_selected();
                    }
                    NoteBoxMessage::HeaderFoldToggled(line_idx) => {
                        if !self.form.progress_notes_notebox_state.collapsed_line_indices.remove(&line_idx) {
                            self.form.progress_notes_notebox_state.collapsed_line_indices.insert(line_idx);
                        }
                    }
                    NoteBoxMessage::TaskStateToggled(line_idx) => {
                        let current_text = self.form.get_progress_notes_text();
                        let updated_text = OrgFormatter::toggle_task_state_at_line(&current_text, line_idx);
                        self.form.progress_notes_editor = iced::widget::text_editor::Content::with_text(&updated_text);
                        self.auto_sync_selected();
                    }
                }
            }
        }
        Task::none()
    }

    fn update_max_visible_level(&mut self, new_max_lvl: usize) {
        self.max_visible_level = new_max_lvl;
        let levels = self.graph.get_node_levels();

        for node in &mut self.graph.nodes {
            if let Some(&lvl) = levels.get(&node.id) {
                node.is_collapsed = (lvl + 1) >= new_max_lvl;
            }
        }
        self.mark_dirty();
    }

    fn update_form_title(&mut self, title: String) {
        if self.is_editing_enabled || self.selected.is_none() {
            self.form.title = title;
            self.auto_sync_selected();
        }
    }

    fn update_form_progress(&mut self, progress: f32) {
        if self.is_editing_enabled || self.selected.is_none() {
            self.form.progress = progress;
            self.auto_sync_selected();
        }
    }

    fn apply_image_crop(&mut self) {
        if let Some(cropper) = self.cropper_state.take() {
            if let Ok(base64_str) = cropper.crop_to_base64() {
                let new_img_id = Uuid::new_v4();
                if ImageManager::save_image_to_md(&self.images_file_path, new_img_id, &base64_str).is_ok() {
                    self.loaded_images.insert(new_img_id.to_string(), base64_str.clone());
                    if let Some(handle) = ImageManager::base64_to_handle(&base64_str) {
                        self.image_cache.insert(new_img_id.to_string(), handle);
                    }
                    self.form.image_id = Some(new_img_id);
                    self.auto_sync_selected();
                }
            }
        }
    }

    fn toggle_tab_visibility(&mut self, tab: TabType) {
        if let Some(idx) = self.open_tabs.iter().position(|&t| t == tab) {
            self.open_tabs.remove(idx);
        } else {
            self.open_tabs.push(tab);
        }
    }

    fn spawn_new_node_at_viewport(&mut self) {
        if !self.form.is_valid() {
            return;
        }

        let mut node = FocusNode::new(self.form.title.clone(), self.form.get_description_text());
        node.progress_notes = self.form.get_progress_notes_text();
        node.status = self.form.to_status();
        node.image_id = self.form.image_id;

        node.x = -self.view.pan_x / self.view.zoom + NEW_NODE_VIEWPORT_OFFSET_X;
        node.y = -self.view.pan_y / self.view.zoom + NEW_NODE_VIEWPORT_OFFSET_Y;

        let new_id = self.graph.add_node(node);
        self.form = NodeForm::default();
        self.select_node(new_id);
        self.mark_dirty();
    }

    fn delete_selected_node(&mut self) {
        if let Some(id) = self.selected.take() {
            self.graph.remove_node(id);
            self.frozen.remove(&id);
            self.form = NodeForm::default();
            self.mark_dirty();
        }
    }

    fn reset_layout_and_view(&mut self) {
        self.frozen.clear();
        self.relayout();
        self.reset_view_to_center();
        self.mark_dirty();
    }

    fn auto_sync_selected(&mut self) {
        if let Some(id) = self.selected {
            if let Some(node) = self.graph.get_node_mut(id) {
                node.title = self.form.title.clone();
                node.description = self.form.get_description_text();
                node.progress_notes = self.form.get_progress_notes_text();
                node.image_id = self.form.image_id;
                node.status = self.form.to_status();
                self.mark_dirty();
            }
        }
    }

    fn determine_initial_max_level(graph: &FocusGraph) -> usize {
        let level_counts = Self::calculate_level_counts(graph);
        level_counts.keys().max().map(|m| m + 1).unwrap_or(1)
    }

    fn calculate_level_counts(graph: &FocusGraph) -> HashMap<usize, usize> {
        let levels = graph.get_node_levels();
        let mut counts = HashMap::new();
        for &lvl in levels.values() {
            *counts.entry(lvl).or_insert(0) += 1;
        }
        counts
    }

    fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    fn save_to_disk(&mut self) {
        self.cleanup_orphan_images();

        if let Err(err) = OrgmodeIO::export(&self.graph, &self.file_path) {
            eprintln!("Otomatik Kaydetme Hatası: {}", err);
        } else {
            self.is_dirty = false;
            self.last_save_time = Instant::now();
        }
    }

    fn cleanup_orphan_images(&mut self) {
        let active_image_ids: HashSet<String> = self
            .graph
            .nodes
            .iter()
            .filter_map(|n| n.image_id.map(|id| id.to_string()))
            .collect();

        self.loaded_images
            .retain(|id, _| active_image_ids.contains(id));
        self.image_cache
            .retain(|id, _| active_image_ids.contains(id));

        let _ = uslu::image::write_images_map_to_file(&self.images_file_path, &self.loaded_images);
    }

    fn reset_view_to_center(&mut self) {
        if self.graph.nodes.is_empty() {
            self.view.pan_x = 0.0;
            self.view.pan_y = 0.0;
            return;
        }

        let screen_center = Point::new(SCREEN_CENTER_X, SCREEN_CENTER_Y);
        let world_center = self.view.screen_to_world(screen_center);

        if let Some(closest_node) = self.graph.nodes.iter().min_by(|a, b| {
            let dist_a = (a.x - world_center.x).hypot(a.y - world_center.y);
            let dist_b = (b.x - world_center.x).hypot(b.y - world_center.y);
            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            let node_center_x = closest_node.x + FocusNode::WIDTH / 2.0;
            let node_center_y = closest_node.y + FocusNode::HEIGHT / 2.0;

            self.view.pan_x = screen_center.x - node_center_x * self.view.zoom;
            self.view.pan_y = screen_center.y - node_center_y * self.view.zoom;
        }
    }

    fn relayout(&mut self) {
        SugiyamaEngine::layout(&mut self.graph, &self.frozen);
    }
}
