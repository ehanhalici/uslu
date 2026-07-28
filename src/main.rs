// src/main.rs
mod canvas;
mod sidebar;

use crate::canvas::{CanvasData, CanvasMessage, Viewport};
use crate::sidebar::{NodeForm, SidebarMessage, TabType};
use iced::{Event, Point, Subscription, Task, keyboard, window};
use std::collections::{HashMap, HashSet};
use uslu::image::{ImageCropperState, ImageManager};
use uslu::markdown;
use uslu::models::{FocusGraph, FocusNode};
use uslu::sugiyama::SugiyamaEngine;
use uuid::Uuid;

fn main() -> iced::Result {
    iced::application("Uslu — Focus Tree", Uslu::update, Uslu::view)
        .subscription(Uslu::subscription)
        .theme(|_| iced::Theme::Dark)
        .window(iced::window::Settings {
            size: iced::Size::new(1280.0, 800.0),
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
    frozen: HashSet<Uuid>,
    open_tabs: Vec<TabType>,

    cropper_state: Option<ImageCropperState>,

    // Tuş durumları
    is_shift_pressed: bool,
    is_ctrl_pressed: bool,

    // Düzenleme Kilidi & Görünürlük Yönetimi
    is_editing_enabled: bool,
    max_visible_level: usize,

    is_dirty: bool,
    last_save_time: std::time::Instant,
}

impl Default for Uslu {
    fn default() -> Self {
        let file_path = "tree.md".to_string();
        let images_file_path = "images.md".to_string();

        let mut graph = FocusGraph::default();
        let frozen = HashSet::new();

        if std::path::Path::new(&file_path).exists() {
            if let Ok(imported_graph) = markdown::MarkdownIO::import(&file_path) {
                graph = imported_graph;
                SugiyamaEngine::layout(&mut graph, &frozen);
            }
        }

        let loaded_images = ImageManager::load_all_images(&images_file_path).unwrap_or_default();

        // En yüksek seviyeyi tespit edip slider varsayılanı yapalım
        let level_counts = Self::calculate_level_counts(&graph);
        let max_lvl = level_counts.keys().max().map(|m| m + 1).unwrap_or(1);

        let mut app = Self {
            graph,
            view: Viewport::default(),
            selected: None,
            form: NodeForm::default(),
            file_path,
            images_file_path,
            loaded_images,
            frozen,
            open_tabs: vec![],
            cropper_state: None,
            is_shift_pressed: false,
            is_ctrl_pressed: false,
            is_editing_enabled: false,
            max_visible_level: max_lvl,
            is_dirty: false,
            last_save_time: std::time::Instant::now(),
        };

        app.reset_view_to_center();
        app
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Canvas(CanvasMessage),
    Sidebar(SidebarMessage),
    EventOccurred(Event),
    PeriodicSaveTick,
    ImagePicked(Option<(image::DynamicImage, Vec<u8>)>),
}

impl Uslu {
    fn subscription(&self) -> Subscription<Message> {
        let events = iced::event::listen().map(Message::EventOccurred);
        let timer =
            iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::PeriodicSaveTick);
        Subscription::batch(vec![events, timer])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Canvas(msg) => self.handle_canvas(msg),
            Message::Sidebar(msg) => return self.handle_sidebar(msg),

            Message::ImagePicked(Some((img, bytes))) => {
                self.cropper_state = Some(ImageCropperState::new(img, bytes));
            }
            Message::ImagePicked(None) => {}

            Message::PeriodicSaveTick => {
                if self.is_dirty
                    && self.last_save_time.elapsed() >= std::time::Duration::from_secs(60)
                {
                    self.save_to_disk();
                }
            }

            Message::EventOccurred(event) => match event {
                Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                    if key == keyboard::Key::Named(keyboard::key::Named::Shift) {
                        self.is_shift_pressed = true;
                    }
                    // DÜZELTME: Ctrl tuşunu hem Named::Control hem de modifiers üzerinden garantiye alıyoruz
                    if key == keyboard::Key::Named(keyboard::key::Named::Control)
                        || modifiers.control()
                    {
                        self.is_ctrl_pressed = true;
                    }
                    if modifiers.control() && key == keyboard::Key::Character("s".into()) {
                        self.save_to_disk();
                    }
                }
                Event::Keyboard(keyboard::Event::KeyReleased { key, .. }) => {
                    if key == keyboard::Key::Named(keyboard::key::Named::Shift) {
                        self.is_shift_pressed = false;
                    }
                    // DÜZELTME: Ctrl tuşundan el çekildiğinde pasife çek
                    if key == keyboard::Key::Named(keyboard::key::Named::Control) {
                        self.is_ctrl_pressed = false;
                    }
                }
                Event::Window(window::Event::CloseRequested) => {
                    self.save_to_disk();
                }
                _ => {}
            },
        }
        Task::none()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        let canvas_data = CanvasData {
            graph: &self.graph,
            view: self.view,
            selected: self.selected,
            is_shift_pressed: self.is_shift_pressed,
            is_ctrl_pressed: self.is_ctrl_pressed,
            loaded_images: &self.loaded_images,
        };

        let canvas_el = canvas::canvas_view(canvas_data).map(Message::Canvas);
        let selected_node = self.selected.and_then(|id| self.graph.get_node(id));

        // HATA DÜZELTME: level_counts'u referans olarak değil, doğrudan sidebar_view'a veriyoruz
        // ya da sidebar_view imzasına uygun şekilde geçiyoruz.
        let level_counts = Self::calculate_level_counts(&self.graph);

        let sidebar_el = sidebar::sidebar_view(
            &self.form,
            selected_node,
            self.graph.nodes.len(),
            self.graph.edges.len(),
            &self.open_tabs,
            self.cropper_state.as_ref(),
            self.is_editing_enabled,
            &level_counts, // <--- Eğer sidebar_view tarafında imzanın yaşam süresi &HashMap ise sorun yaratır!
            self.max_visible_level,
        )
        .map(Message::Sidebar);

        iced::widget::row![sidebar_el, canvas_el].spacing(0).into()
    }
    fn handle_canvas(&mut self, msg: CanvasMessage) {
        match msg {
            CanvasMessage::NodeClicked { id, shift, ctrl } => {
                // 2. İSTER: Ctrl + Click ile Alt Ağacı Daralt / Aç
                if ctrl {
                    if let Some(node) = self.graph.get_node_mut(id) {
                        node.is_collapsed = !node.is_collapsed;
                    }
                    self.mark_dirty();
                    return;
                }

                // Shift + Click ile Bağlantı Kur
                if shift {
                    if let Some(parent) = self.selected {
                        if parent != id {
                            self.graph.add_edge(parent, id);
                            self.mark_dirty();
                        }
                    }
                }

                self.selected = Some(id);
                self.is_editing_enabled = false;

                if let Some(node) = self.graph.get_node(id) {
                    self.form = NodeForm::from_node(node);
                }
                if !self.open_tabs.contains(&TabType::General) {
                    self.open_tabs.push(TabType::General);
                }
            }
            CanvasMessage::DeleteNodeClicked(id) => {
                self.graph.remove_node(id);
                self.frozen.remove(&id);
                if self.selected == Some(id) {
                    self.selected = None;
                }
                self.form = NodeForm::default();
                self.mark_dirty();
            }
            CanvasMessage::DeleteEdgeClicked {
                parent_id,
                child_id,
            } => {
                self.graph.remove_edge(parent_id, child_id);
                self.mark_dirty();
            }
            CanvasMessage::NodeMoved { id, x, y } => {
                if let Some(node) = self.graph.get_node_mut(id) {
                    node.x = x;
                    node.y = y;
                    self.frozen.insert(id);
                    self.mark_dirty();
                }
            }
            CanvasMessage::BackgroundClicked => {
                self.selected = None;
                self.form = NodeForm::default();
                self.is_editing_enabled = false;
            }
            CanvasMessage::ViewChanged(new_view) => {
                self.view = new_view;
            }
        }
    }

    fn handle_sidebar(&mut self, msg: SidebarMessage) -> Task<Message> {
        match msg {
            // İSTER 1: Mutlak Editle Butonu Tetiklemesi
            SidebarMessage::ToggleEditing => {
                self.is_editing_enabled = !self.is_editing_enabled;
            }

            SidebarMessage::MaxLevelSliderChanged(new_max_lvl) => {
                self.max_visible_level = new_max_lvl;
                let levels = self.graph.get_node_levels();

                // Sadece event anında 1 defaya mahsus çalışır:
                // Seviyesi seçilen slider değerinden büyük olan ebeveynlerin çocuklarını kapatır,
                // küçük veya eşit olanları açar.
                for node in &mut self.graph.nodes {
                    if let Some(&lvl) = levels.get(&node.id) {
                        node.is_collapsed = (lvl + 1) >= new_max_lvl;
                    }
                }

                self.mark_dirty();
            }
            SidebarMessage::TitleChanged(s) => {
                if self.is_editing_enabled {
                    self.form.title = s;
                    self.auto_sync_selected();
                }
            }
            SidebarMessage::ProgressChanged(p) => {
                // İlerleme her zaman değişebilir!
                self.form.progress = p;
                self.auto_sync_selected();
            }
            SidebarMessage::DescriptionAction(action) => {
                if self.is_editing_enabled {
                    self.form.description_editor.perform(action);
                    self.auto_sync_selected();
                }
            }
            SidebarMessage::ProgressNotesAction(action) => {
                if self.is_editing_enabled {
                    self.form.progress_notes_editor.perform(action);
                    self.auto_sync_selected();
                }
            }
            SidebarMessage::OpenImagePicker => {
                if self.is_editing_enabled || self.selected.is_none() {
                    return Task::perform(ImageManager::pick_image_file(), Message::ImagePicked);
                }
            }
            SidebarMessage::CropZoomChanged(z) => {
                if let Some(ref mut cropper) = self.cropper_state {
                    cropper.zoom = z;
                }
            }
            SidebarMessage::CropPanMoved { delta_x, delta_y } => {
                if let Some(ref mut cropper) = self.cropper_state {
                    cropper.offset_x += delta_x;
                    cropper.offset_y += delta_y;
                }
            }
            SidebarMessage::ApplyCropAndSave => {
                if let Some(cropper) = self.cropper_state.take() {
                    if let Ok(base64_str) = cropper.crop_to_base64() {
                        let new_img_id = Uuid::new_v4();
                        if ImageManager::save_image_to_md(
                            &self.images_file_path,
                            new_img_id,
                            &base64_str,
                        )
                        .is_ok()
                        {
                            self.loaded_images
                                .insert(new_img_id.to_string(), base64_str);
                            self.form.image_id = Some(new_img_id);
                            self.auto_sync_selected();
                        }
                    }
                }
            }

            SidebarMessage::CancelCrop => {
                self.cropper_state = None;
            }

            SidebarMessage::ToggleTab(tab) => {
                if let Some(idx) = self.open_tabs.iter().position(|&t| t == tab) {
                    self.open_tabs.remove(idx);
                } else {
                    self.open_tabs.push(tab);
                }
            }

            SidebarMessage::AddNode => {
                if self.form.is_valid() {
                    let mut node =
                        FocusNode::new(self.form.title.clone(), self.form.get_description_text());
                    node.progress_notes = self.form.get_progress_notes_text();
                    node.status = self.form.to_status();
                    node.image_id = self.form.image_id;

                    node.x = -self.view.pan_x / self.view.zoom + 200.0;
                    node.y = -self.view.pan_y / self.view.zoom + 200.0;

                    self.graph.add_node(node);
                    self.form = NodeForm::default();

                    self.mark_dirty();
                }
            }

            SidebarMessage::DeleteSelected => {
                if let Some(id) = self.selected.take() {
                    self.graph.remove_node(id);
                    self.frozen.remove(&id);
                    self.form = NodeForm::default();
                    self.relayout();
                    self.mark_dirty();
                }
            }

            SidebarMessage::ResetView => {
                self.frozen.clear();
                self.relayout();
                self.reset_view_to_center();
            }
        }
        Task::none()
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
        if let Err(e) = markdown::MarkdownIO::export(&self.graph, &self.file_path) {
            eprintln!("Otomatik Kaydetme Hatası: {}", e);
        } else {
            self.is_dirty = false;
            self.last_save_time = std::time::Instant::now();
        }
    }

    fn reset_view_to_center(&mut self) {
        if self.graph.nodes.is_empty() {
            self.view.pan_x = 0.0;
            self.view.pan_y = 0.0;
            return;
        }

        let screen_center = Point::new(460.0, 400.0);
        let world_center = self.view.screen_to_world(screen_center);

        if let Some(closest_node) = self.graph.nodes.iter().min_by(|a, b| {
            let dist_a = (a.x - world_center.x).hypot(a.y - world_center.y);
            let dist_b = (b.x - world_center.x).hypot(b.y - world_center.y);
            dist_a
                .partial_cmp(&dist_b)
                .unwrap_or(std::cmp::Ordering::Equal)
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
