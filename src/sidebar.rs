// src/sidebar.rs
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Image};
use iced::widget::{
    button, column, container, row, scrollable, slider, text, text_editor, text_input,
};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme};
use uslu::image::ImageCropperState;
use uslu::models::{FocusNode, NodeStatus};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabType {
    General,
    Description,
    ProgressNotes,
}

impl TabType {
    pub fn title(&self) -> &'static str {
        match self {
            Self::General => "⚙ Genel",
            Self::Description => "📝 Açıklama",
            Self::ProgressNotes => "📌 İlerleme Notları",
        }
    }
}

#[derive(Debug, Clone)]
pub enum SidebarMessage {
    TitleChanged(String),
    ProgressChanged(f32),

    DescriptionAction(text_editor::Action),
    ProgressNotesAction(text_editor::Action),

    OpenImagePicker,
    CropZoomChanged(f32),
    CropPanMoved { delta_x: f32, delta_y: f32 },
    ApplyCropAndSave,
    CancelCrop,

    ToggleTab(TabType),
    AddNode,
    DeleteSelected,
    ResetView,
}

#[derive(Debug)]
pub struct NodeForm {
    pub title: String,
    pub image_id: Option<Uuid>,
    pub progress: f32,
    pub description_editor: text_editor::Content,
    pub progress_notes_editor: text_editor::Content,
}

impl Clone for NodeForm {
    fn clone(&self) -> Self {
        Self {
            title: self.title.clone(),
            image_id: self.image_id,
            progress: self.progress,
            description_editor: text_editor::Content::with_text(&self.description_editor.text()),
            progress_notes_editor: text_editor::Content::with_text(
                &self.progress_notes_editor.text(),
            ),
        }
    }
}

impl Default for NodeForm {
    fn default() -> Self {
        Self {
            title: String::new(),
            image_id: None,
            progress: 0.0,
            description_editor: text_editor::Content::new(),
            progress_notes_editor: text_editor::Content::new(),
        }
    }
}

impl NodeForm {
    pub fn from_node(node: &FocusNode) -> Self {
        Self {
            title: node.title.clone(),
            image_id: node.image_id,
            progress: node.status.progress,
            description_editor: text_editor::Content::with_text(&node.description),
            progress_notes_editor: text_editor::Content::with_text(&node.progress_notes),
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.title.trim().is_empty()
    }

    pub fn to_status(&self) -> NodeStatus {
        NodeStatus {
            progress: self.progress.clamp(0.0, 100.0),
        }
    }

    pub fn get_description_text(&self) -> String {
        self.description_editor.text()
    }

    pub fn get_progress_notes_text(&self) -> String {
        self.progress_notes_editor.text()
    }
}

pub fn sidebar_view<'a>(
    form: &'a NodeForm,
    selected_node: Option<&'a FocusNode>,
    node_count: usize,
    edge_count: usize,
    open_tabs: &'a [TabType],
    cropper_state: Option<&'a ImageCropperState>,
) -> Element<'a, SidebarMessage> {
    // Sadece Siyah Ekran Kırpma Arayüzü
    if let Some(cropper) = cropper_state {
        let cropper_canvas = canvas::Canvas::new(CropperProgram { state: cropper })
            .width(240)
            .height(240);

        // Canvas'ı taşırmayacak kırpma konteyneri
        let canvas_container = container(cropper_canvas)
            .width(240)
            .height(240)
            .clip(true) // Tuval dışına çıkan tüm piksel çizimlerini GPU seviyesinde keser
            .style(|_t| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(0x0a, 0x0a, 0x0c))),
                border: iced::Border {
                    color: Color::from_rgb8(0x3a, 0x3a, 0x44),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Default::default()
            });

        let cropper_view = column![
            text("Kadrajı Ayarlayın")
                .size(14)
                .color(Color::from_rgb8(0xd4, 0xaf, 0x37)),
            text("Fare ile sürükleyin, tekerlek ile zoom yapın.")
                .size(10)
                .color(Color::from_rgb8(0x88, 0x88, 0x90)),
            canvas_container,
            row![
                button(text("Kırp ve Kaydet").size(12))
                    .on_press(SidebarMessage::ApplyCropAndSave)
                    .padding(8),
                button(text("İptal").size(12))
                    .on_press(SidebarMessage::CancelCrop)
                    .padding(8),
            ]
            .spacing(8)
        ]
        .spacing(12)
        .padding(12);

        return container(cropper_view)
            .width(360)
            .height(Length::Fill)
            .style(|_t| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(0x1c, 0x1c, 0x22))),
                ..Default::default()
            })
            .into();
    }
    let mut top_strip = column![].spacing(8);

    for tab in &[
        TabType::General,
        TabType::Description,
        TabType::ProgressNotes,
    ] {
        let is_open = open_tabs.contains(tab);
        let label = match tab {
            TabType::General => "⚙",
            TabType::Description => "📝",
            TabType::ProgressNotes => "📌",
        };

        top_strip = top_strip.push(
            button(text(label).size(16))
                .on_press(SidebarMessage::ToggleTab(*tab))
                .padding(8)
                .style(move |_t, _s| button::Style {
                    background: Some(iced::Background::Color(if is_open {
                        Color::from_rgb8(0xd4, 0xaf, 0x37)
                    } else {
                        Color::from_rgb8(0x28, 0x28, 0x30)
                    })),
                    text_color: if is_open { Color::BLACK } else { Color::WHITE },
                    border: iced::border::rounded(6),
                    ..Default::default()
                }),
        );
    }

    let reset_btn = button(text("🔄").size(16))
        .on_press(SidebarMessage::ResetView)
        .padding(8)
        .style(|_t, _s| button::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(0x38, 0x38, 0x42))),
            text_color: Color::WHITE,
            border: iced::border::rounded(6),
            ..Default::default()
        });

    let tab_strip_container = container(
        column![
            top_strip,
            container(reset_btn)
                .height(Length::Fill)
                .align_y(iced::alignment::Vertical::Bottom)
        ]
        .padding(6)
        .height(Length::Fill),
    )
    .width(48)
    .height(Length::Fill)
    .style(|_t| container::Style {
        background: Some(iced::Background::Color(Color::from_rgb8(0x18, 0x18, 0x1c))),
        ..Default::default()
    });

    if open_tabs.is_empty() {
        return row![tab_strip_container].into();
    }

    let mut main_stack = column![].spacing(16).width(Length::Fill);

    for tab in open_tabs {
        let mut tab_card = column![].spacing(8);

        let header = row![
            text(tab.title())
                .size(13)
                .color(Color::from_rgb8(0xd4, 0xaf, 0x37)),
            text(format!("({} düğüm, {} bağ)", node_count, edge_count))
                .size(10)
                .color(Color::from_rgb8(0x77, 0x77, 0x80)),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center);

        tab_card = tab_card.push(header);

        match tab {
            // src/sidebar.rs -> sidebar_view fonksiyonu içi TabType::General bloğu
            TabType::General => {
                let mut gen_box = column![].spacing(10);

                gen_box = gen_box.push(
                    column![
                        text("Başlık")
                            .size(11)
                            .color(Color::from_rgb8(0x90, 0x90, 0x99)),
                        text_input("Başlık...", &form.title)
                            .on_input(SidebarMessage::TitleChanged)
                            .padding(6),
                    ]
                    .spacing(2),
                );

                gen_box = gen_box.push(
                    column![
                        text("Resim Seç")
                            .size(11)
                            .color(Color::from_rgb8(0x90, 0x90, 0x99)),
                        button(
                            text(if form.image_id.is_some() {
                                "Resmi Değiştir..."
                            } else {
                                "Görsel Dosyası Seç..."
                            })
                            .size(11)
                        )
                        .on_press(SidebarMessage::OpenImagePicker)
                        .padding(6),
                    ]
                    .spacing(2),
                );

                gen_box = gen_box.push(
                    column![
                        row![
                            text("İlerleme")
                                .size(11)
                                .color(Color::from_rgb8(0x90, 0x90, 0x99)),
                            text(format!("%{:.0}", form.progress))
                                .size(11)
                                .color(Color::from_rgb8(0xd4, 0xaf, 0x37)),
                        ]
                        .spacing(8),
                        slider(0.0..=100.0, form.progress, SidebarMessage::ProgressChanged)
                            .step(1.0),
                    ]
                    .spacing(2),
                );

                // Sadece Seçili Düğüm Olmadığında "Yeni Düğüm Ekle" Butonunu Göster
                if selected_node.is_none() {
                    gen_box = gen_box.push(
                        button(text("Yeni Düğüm Ekle").size(12))
                            .on_press_maybe(if form.is_valid() {
                                Some(SidebarMessage::AddNode)
                            } else {
                                None
                            })
                            .padding(8),
                    );
                }

                tab_card = tab_card.push(gen_box);
            }
            TabType::Description => {
                let editor_widget = text_editor(&form.description_editor)
                    .on_action(SidebarMessage::DescriptionAction)
                    .padding(1)
                    .height(Length::Fill);

                let editor_container = container(editor_widget)
                    .height(400)
                    .width(Length::Fill)
                    .style(|_t| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(
                            0x18, 0x18, 0x1e,
                        ))),
                        ..Default::default()
                    });

                tab_card = tab_card.push(editor_container);
            }

            TabType::ProgressNotes => {
                let editor_widget = text_editor(&form.progress_notes_editor)
                    .on_action(SidebarMessage::ProgressNotesAction)
                    .padding(1)
                    .height(Length::Fill);

                let editor_container = container(editor_widget)
                    .height(400)
                    .width(Length::Fill)
                    .style(|_t| container::Style {
                        background: Some(iced::Background::Color(Color::from_rgb8(
                            0x18, 0x18, 0x1e,
                        ))),
                        ..Default::default()
                    });

                tab_card = tab_card.push(editor_container);
            }
        }

        let card_container = container(tab_card)
            .width(Length::Fill)
            .padding(10)
            .style(|_t| container::Style {
                background: Some(iced::Background::Color(Color::from_rgb8(0x22, 0x22, 0x28))),
                ..Default::default()
            });

        main_stack = main_stack.push(card_container);
    }

    let scrollable_panel = container(scrollable(main_stack).height(Length::Fill))
        .width(360)
        .height(Length::Fill)
        .padding(10)
        .style(|_t| container::Style {
            background: Some(iced::Background::Color(Color::from_rgb8(0x1c, 0x1c, 0x22))),
            ..Default::default()
        });

    row![tab_strip_container, scrollable_panel].into()
}

#[derive(Default)]
pub enum CropperInteraction {
    #[default]
    Idle,
    Dragging {
        last_pos: Point,
    },
}

pub struct CropperProgram<'a> {
    pub state: &'a ImageCropperState,
}

impl<'a> canvas::Program<SidebarMessage> for CropperProgram<'a> {
    type State = CropperInteraction;

    fn update(
        &self,
        interaction: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (iced::event::Status, Option<SidebarMessage>) {
        let cursor_pos = match cursor.position_in(bounds) {
            Some(p) => p,
            None => return (iced::event::Status::Ignored, None),
        };

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                *interaction = CropperInteraction::Dragging {
                    last_pos: cursor_pos,
                };
                (iced::event::Status::Captured, None)
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                *interaction = CropperInteraction::Idle;
                (iced::event::Status::Captured, None)
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if let CropperInteraction::Dragging { last_pos } = interaction {
                    let delta_x = cursor_pos.x - last_pos.x;
                    let delta_y = cursor_pos.y - last_pos.y;
                    *last_pos = cursor_pos;
                    return (
                        iced::event::Status::Captured,
                        Some(SidebarMessage::CropPanMoved { delta_x, delta_y }),
                    );
                }
                (iced::event::Status::Ignored, None)
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let zoom_delta = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * 0.1,
                    mouse::ScrollDelta::Pixels { y, .. } => y * 0.002,
                };
                // Minimum zoom, resmin küçük kenarının (base_size=200) her zaman
                // 240px'lik kırpma kutusunu tam kaplamasını garanti eder (240/200 = 1.2).
                // Bu olmadan zoom 1.0'da bile resim kutuyu doldurmuyor ve boşluk kalıyordu.
                const MIN_ZOOM: f32 = 240.0 / 200.0;
                let new_zoom = (self.state.zoom + zoom_delta).clamp(MIN_ZOOM, 5.0);
                (
                    iced::event::Status::Captured,
                    Some(SidebarMessage::CropZoomChanged(new_zoom)),
                )
            }
            _ => (iced::event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        _interaction: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);

        // 1. Resmin Orijinal Boyutları ve Oranı (Bükülmeyi Önler)
        let (orig_w, orig_h) = (
            self.state.original_image.width() as f32,
            self.state.original_image.height() as f32,
        );
        let aspect_ratio = orig_w / orig_h;

        let base_size = 200.0;
        let (base_img_w, base_img_h) = if aspect_ratio >= 1.0 {
            (base_size * aspect_ratio, base_size)
        } else {
            (base_size, base_size / aspect_ratio)
        };

        let img_w = base_img_w * self.state.zoom;
        let img_h = base_img_h * self.state.zoom;

        let img_pos = Point::new(
            center.x - img_w / 2.0 + self.state.offset_x,
            center.y - img_h / 2.0 + self.state.offset_y,
        );

        // 2. Resim Tam Orijinal Şekliyle Çizilir (Ezilme / Bükülme YOK)
        // ÖNEMLİ: draw_image çağrısını frame.with_clip içine alıyoruz.
        // Frame::new zaten kendi bounds'u dışına taşan çizimleri kesmesi gerekiyor,
        // ancak resim (Image) primitive'leri bazı iced sürümlerinde/backend'lerde
        // bu clip'e tam uymayıp taşabiliyor; with_clip bunu kesin olarak garanti eder.
        // Sonuç: resim ne kadar büyütülür/kaydırılırsa kaydırılsın, 240x240'lık
        // kutunun dışına asla tek bir piksel bile taşmaz (alttaki butonların üstüne çıkmaz).
        frame.with_clip(Rectangle::with_size(bounds.size()), |frame| {
            frame.draw_image(
                Rectangle::new(img_pos, Size::new(img_w, img_h)),
                Image::new(self.state.image_handle.clone()),
            );
        });

        // 4. Siyah Tuvalin Sınır Çizgisi (İnce İçi Gösteren Çerçeve)
        frame.stroke_rectangle(
            Point::ORIGIN,
            bounds.size(),
            canvas::Stroke {
                style: canvas::Style::Solid(Color::from_rgb8(0x3a, 0x3a, 0x44)),
                width: 1.0,
                ..Default::default()
            },
        );

        vec![frame.into_geometry()]
    }
}
