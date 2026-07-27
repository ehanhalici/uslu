// src/canvas.rs
use iced::mouse;
use iced::widget::canvas::{self, Event, Frame, Geometry, Image, Path};
use iced::{Color, Element, Point, Rectangle, Renderer, Size, Theme, Vector};
use std::collections::HashMap;
use uslu::image::ImageManager;
use uslu::models::{FocusGraph, FocusNode};
use uuid::Uuid;

#[derive(Clone)]
pub struct CanvasData<'a> {
    pub graph: &'a FocusGraph,
    pub view: Viewport,
    pub selected: Option<Uuid>,
    pub is_shift_pressed: bool,
    pub loaded_images: &'a HashMap<String, String>, // Base64 Önbelleği
}

#[derive(Clone, Copy, Debug)]
pub struct Viewport {
    pub pan_x: f32,
    pub pan_y: f32,
    pub zoom: f32,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom: 1.0,
        }
    }
}

impl Viewport {
    pub fn world_to_screen(&self, p: Point) -> Point {
        Point::new(p.x * self.zoom + self.pan_x, p.y * self.zoom + self.pan_y)
    }

    pub fn screen_to_world(&self, p: Point) -> Point {
        Point::new(
            (p.x - self.pan_x) / self.zoom,
            (p.y - self.pan_y) / self.zoom,
        )
    }
}

#[derive(Default)]
pub enum Interaction {
    #[default]
    Idle,
    Panning {
        start: Point,
        original_pan: Vector,
    },
    DraggingNode {
        id: Uuid,
        grab_offset: Vector,
    },
}

#[derive(Debug, Clone)]
pub enum CanvasMessage {
    NodeClicked { id: Uuid, shift: bool },
    NodeMoved { id: Uuid, x: f32, y: f32 },
    DeleteNodeClicked(Uuid),
    DeleteEdgeClicked { parent_id: Uuid, child_id: Uuid },
    BackgroundClicked,
    ViewChanged(Viewport),
}

pub fn canvas_view<'a>(data: CanvasData<'a>) -> Element<'a, CanvasMessage> {
    canvas::Canvas::new(CanvasProgram { data })
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .into()
}

pub struct CanvasProgram<'a> {
    pub data: CanvasData<'a>,
}

impl<'a> canvas::Program<CanvasMessage> for CanvasProgram<'a> {
    type State = Interaction;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (iced::event::Status, Option<CanvasMessage>) {
        let cursor_pos = match cursor.position_in(bounds) {
            Some(p) => p,
            None => return (iced::event::Status::Ignored, None),
        };

        match event {
            // src/canvas.rs -> CanvasProgram::update fonksiyonu içi


Event::Mouse(mouse::Event::ButtonPressed(button)) if button == mouse::Button::Left => {
    let world = self.data.view.screen_to_world(cursor_pos);

    // 1. Shift basılıysa sadece Kırmızı '×' İkonlarına Tıklama Kontrolü Yapılır
    if self.data.is_shift_pressed {
        // A) Düğümün sağ üstündeki kırmızı '×' butonuna tıklandı mı?
        for node in &self.data.graph.nodes {
            let cross_world = Point::new(node.x + FocusNode::WIDTH - 6.0, node.y + 6.0);
            let screen_cross = self.data.view.world_to_screen(cross_world);

            // Tıklama imleci sadece '×' butonunun üzerindeyse (~16px yarıçap) silme yap
            if (cursor_pos.x - screen_cross.x).hypot(cursor_pos.y - screen_cross.y) < 16.0 {
                return (
                    iced::event::Status::Captured,
                    Some(CanvasMessage::DeleteNodeClicked(node.id)),
                );
            }
        }

        // B) Bağlantıların (Edge) ortasındaki kırmızı '×' butonuna tıklandı mı?
        for edge in &self.data.graph.edges {
            if let (Some(parent), Some(child)) = (
                self.data.graph.get_node(edge.parent_id),
                self.data.graph.get_node(edge.child_id),
            ) {
                let mid_world = Point::new(
                    (parent.x + child.x + FocusNode::WIDTH) / 2.0,
                    (parent.y + child.y + FocusNode::HEIGHT) / 2.0,
                );
                let screen_mid = self.data.view.world_to_screen(mid_world);
                if (cursor_pos.x - screen_mid.x).hypot(cursor_pos.y - screen_mid.y) < 14.0 {
                    return (
                        iced::event::Status::Captured,
                        Some(CanvasMessage::DeleteEdgeClicked {
                            parent_id: edge.parent_id,
                            child_id: edge.child_id,
                        }),
                    );
                }
            }
        }
    }

    // 2. Kırmızı '×' ikonlarına tıklanmadıysa (Gövdeye tıklandıysa):
    if let Some(node) = hit_test_node(self.data.graph, world) {
        *state = Interaction::DraggingNode {
            id: node.id,
            grab_offset: Vector::new(world.x - node.x, world.y - node.y),
        };
        return (
            iced::event::Status::Captured,
            Some(CanvasMessage::NodeClicked {
                id: node.id,
                shift: self.data.is_shift_pressed, // Shift ile 2 düğümü birbirine bağlar
            }),
        );
    }

    // 3. Boş alana tıklandıysa tuvali kaydır (Pan)
    *state = Interaction::Panning {
        start: cursor_pos,
        original_pan: Vector::new(self.data.view.pan_x, self.data.view.pan_y),
    };
    (iced::event::Status::Captured, Some(CanvasMessage::BackgroundClicked))
}
            Event::Mouse(mouse::Event::ButtonReleased(button)) if button == mouse::Button::Left => {
                *state = Interaction::Idle;
                (iced::event::Status::Captured, None)
            }

            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let world = self.data.view.screen_to_world(cursor_pos);
                match state {
                    Interaction::Idle => (iced::event::Status::Ignored, None),
                    Interaction::Panning {
                        start,
                        original_pan,
                    } => {
                        let new_view = Viewport {
                            pan_x: original_pan.x + (cursor_pos.x - start.x),
                            pan_y: original_pan.y + (cursor_pos.y - start.y),
                            zoom: self.data.view.zoom,
                        };
                        (
                            iced::event::Status::Captured,
                            Some(CanvasMessage::ViewChanged(new_view)),
                        )
                    }
                    Interaction::DraggingNode { id, grab_offset } => {
                        let new_x = world.x - grab_offset.x;
                        let new_y = world.y - grab_offset.y;
                        (
                            iced::event::Status::Captured,
                            Some(CanvasMessage::NodeMoved {
                                id: *id,
                                x: new_x,
                                y: new_y,
                            }),
                        )
                    }
                }
            }

            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let zoom_delta = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * 0.1,
                    mouse::ScrollDelta::Pixels { y, .. } => y * 0.001,
                };
                let new_zoom = (self.data.view.zoom * (1.0 + zoom_delta)).clamp(0.2, 3.0);
                let world_before = self.data.view.screen_to_world(cursor_pos);
                let new_view = Viewport {
                    zoom: new_zoom,
                    pan_x: cursor_pos.x - world_before.x * new_zoom,
                    pan_y: cursor_pos.y - world_before.y * new_zoom,
                };
                (
                    iced::event::Status::Captured,
                    Some(CanvasMessage::ViewChanged(new_view)),
                )
            }

            _ => (iced::event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgb8(0x1a, 0x1a, 0x1f),
        );
        draw_grid(&mut frame, &self.data.view, bounds);

        for edge in &self.data.graph.edges {
            if let (Some(parent), Some(child)) = (
                self.data.graph.get_node(edge.parent_id),
                self.data.graph.get_node(edge.child_id),
            ) {
                draw_edge(
                    &mut frame,
                    &self.data.view,
                    parent,
                    child,
                    self.data.is_shift_pressed,
                );
            }
        }

        for node in &self.data.graph.nodes {
            let is_selected = self.data.selected == Some(node.id);
            draw_node(
                &mut frame,
                &self.data.view,
                node,
                is_selected,
                self.data.is_shift_pressed,
                self.data.loaded_images,
            );
        }

        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &Self::State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match state {
            Interaction::Panning { .. } | Interaction::DraggingNode { .. } => {
                mouse::Interaction::Grabbing
            }
            Interaction::Idle => {
                if let Some(p) = cursor.position_in(bounds) {
                    let world = self.data.view.screen_to_world(p);
                    if hit_test_node(self.data.graph, world).is_some() {
                        return mouse::Interaction::Pointer;
                    }
                }
                mouse::Interaction::default()
            }
        }
    }
}

fn hit_test_node(graph: &FocusGraph, world: Point) -> Option<&FocusNode> {
    graph.nodes.iter().rev().find(|n| {
        world.x >= n.x
            && world.x <= n.x + FocusNode::WIDTH
            && world.y >= n.y
            && world.y <= n.y + FocusNode::HEIGHT
    })
}

fn draw_grid(frame: &mut Frame, view: &Viewport, bounds: Rectangle) {
    const GRID: f32 = 50.0;
    let z = view.zoom;
    if GRID * z < 8.0 {
        return;
    }
    let color = Color::from_rgba8(0x40, 0x40, 0x48, 0.33);

    let mut x = view.pan_x.rem_euclid(GRID * z) - GRID * z;
    while x < bounds.width {
        frame.fill_rectangle(Point::new(x, 0.0), Size::new(1.0, bounds.height), color);
        x += GRID * z;
    }

    let mut y = view.pan_y.rem_euclid(GRID * z) - GRID * z;
    while y < bounds.height {
        frame.fill_rectangle(Point::new(0.0, y), Size::new(bounds.width, 1.0), color);
        y += GRID * z;
    }
}

fn draw_node(
    frame: &mut Frame,
    view: &Viewport,
    node: &FocusNode,
    is_selected: bool,
    is_shift: bool,
    loaded_images: &HashMap<String, String>,
) {
    let screen = view.world_to_screen(Point::new(node.x, node.y));
    let w = FocusNode::WIDTH * view.zoom;

    let icon_size = 72.0 * view.zoom;
    let icon_pos = Point::new(screen.x + (w - icon_size) / 2.0, screen.y + 4.0 * view.zoom);

    // images.md İçinden Resim ID'si İle Base64 Yükleme
    if let Some(img_id) = &node.image_id {
        if let Some(base64_str) = loaded_images.get(&img_id.to_string()) {
            if let Some(handle) = ImageManager::base64_to_handle(base64_str) {
                frame.draw_image(
                    Rectangle::new(icon_pos, Size::new(icon_size, icon_size)),
                    Image::new(handle),
                );
            }
        }
    } else {
        let center = Point::new(icon_pos.x + icon_size / 2.0, icon_pos.y + icon_size / 2.0);
        frame.stroke_rectangle(
            Point::new(center.x - 12.0 * view.zoom, center.y - 12.0 * view.zoom),
            Size::new(24.0 * view.zoom, 24.0 * view.zoom),
            canvas::Stroke {
                style: canvas::Style::Solid(Color::from_rgba8(0xd4, 0xaf, 0x37, 0.4)),
                width: 1.5 * view.zoom,
                ..Default::default()
            },
        );
    }

    let banner_w = w * 0.96;
    let banner_h = 32.0 * view.zoom;
    let banner_pos = Point::new(
        screen.x + (w - banner_w) / 2.0,
        screen.y + icon_size + 6.0 * view.zoom,
    );

    frame.fill_rectangle(
        banner_pos,
        Size::new(banner_w, banner_h),
        Color::from_rgb8(0x1e, 0x20, 0x26),
    );

    let banner_border_color = if is_selected {
        Color::from_rgb8(0xff, 0xd7, 0x00)
    } else {
        Color::from_rgb8(0x99, 0x7a, 0x2d)
    };

    frame.stroke_rectangle(
        banner_pos,
        Size::new(banner_w, banner_h),
        canvas::Stroke {
            style: canvas::Style::Solid(banner_border_color),
            width: if is_selected {
                2.0 * view.zoom
            } else {
                1.2 * view.zoom
            },
            ..Default::default()
        },
    );

    let inset = 2.0 * view.zoom;
    frame.stroke_rectangle(
        Point::new(banner_pos.x + inset, banner_pos.y + inset),
        Size::new(banner_w - (inset * 2.0), banner_h - (inset * 2.0)),
        canvas::Stroke {
            style: canvas::Style::Solid(Color::from_rgba8(0xd4, 0xaf, 0x37, 0.3)),
            width: 0.8 * view.zoom,
            ..Default::default()
        },
    );

    if view.zoom > 0.35 {
        let title_size = (10.5 * view.zoom).max(7.0);
        frame.fill_text(canvas::Text {
            content: truncate_text(&node.title, banner_w - 8.0),
            position: Point::new(
                banner_pos.x + banner_w / 2.0,
                banner_pos.y + (banner_h - title_size) / 2.0 - 1.0 * view.zoom,
            ),
            color: Color::from_rgb8(0xf0, 0xe6, 0xd2),
            size: iced::Pixels(title_size),
            horizontal_alignment: iced::alignment::Horizontal::Center,
            vertical_alignment: iced::alignment::Vertical::Top,
            ..Default::default()
        });
    }

    let bar_h = 5.0 * view.zoom;
    let bar_y = banner_pos.y + banner_h + 4.0 * view.zoom;

    frame.fill_rectangle(
        Point::new(banner_pos.x, bar_y),
        Size::new(banner_w, bar_h),
        Color::from_rgb8(0x12, 0x12, 0x14),
    );

    let progress_ratio = (node.status.progress / 100.0).clamp(0.0, 1.0);
    let fill_w = banner_w * progress_ratio;
    let bar_color = interpolate_color(
        Color::from_rgb(0.85, 0.15, 0.15),
        Color::from_rgb(0.15, 0.85, 0.20),
        progress_ratio,
    );

    frame.fill_rectangle(
        Point::new(banner_pos.x, bar_y),
        Size::new(fill_w, bar_h),
        bar_color,
    );

    frame.stroke_rectangle(
        Point::new(banner_pos.x, bar_y),
        Size::new(banner_w, bar_h),
        canvas::Stroke {
            style: canvas::Style::Solid(Color::from_rgb8(0x40, 0x40, 0x48)),
            width: 0.8 * view.zoom,
            ..Default::default()
        },
    );

    if is_shift {
        let cross_center = Point::new(screen.x + w - 6.0, screen.y + 6.0);
        frame.fill_rectangle(
            Point::new(cross_center.x - 8.0, cross_center.y - 8.0),
            Size::new(16.0, 16.0),
            Color::from_rgb8(0xd9, 0x38, 0x38),
        );
        frame.fill_text(canvas::Text {
            content: "×".to_string(),
            position: cross_center,
            color: Color::WHITE,
            size: iced::Pixels(12.0),
            horizontal_alignment: iced::alignment::Horizontal::Center,
            vertical_alignment: iced::alignment::Vertical::Center,
            ..Default::default()
        });
    }
}

fn draw_edge(
    frame: &mut Frame,
    view: &Viewport,
    parent: &FocusNode,
    child: &FocusNode,
    is_shift: bool,
) {
    let start_world = Point::new(
        parent.x + FocusNode::WIDTH / 2.0,
        parent.y + FocusNode::HEIGHT,
    );
    let end_world = Point::new(child.x + FocusNode::WIDTH / 2.0, child.y);

    let start = view.world_to_screen(start_world);
    let end = view.world_to_screen(end_world);

    let gold = Color::from_rgb8(0xd4, 0xaf, 0x37);
    let stroke = canvas::Stroke {
        style: canvas::Style::Solid(gold),
        width: 2.0,
        ..Default::default()
    };

    let mid_screen = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);

    if (start.x - end.x).abs() < 1.0 {
        frame.stroke(&Path::line(start, end), stroke);
    } else {
        let mid_y = (start.y + end.y) / 2.0;
        let mid1 = Point::new(start.x, mid_y);
        let mid2 = Point::new(end.x, mid_y);
        frame.stroke(
            &Path::new(|b| {
                b.move_to(start);
                b.line_to(mid1);
                b.line_to(mid2);
                b.line_to(end);
            }),
            stroke,
        );
    }

    draw_arrowhead(frame, end);

    if is_shift {
        frame.fill_rectangle(
            Point::new(mid_screen.x - 7.0, mid_screen.y - 7.0),
            Size::new(14.0, 14.0),
            Color::from_rgb8(0xd9, 0x38, 0x38),
        );
        frame.fill_text(canvas::Text {
            content: "×".to_string(),
            position: mid_screen,
            color: Color::WHITE,
            size: iced::Pixels(10.0),
            horizontal_alignment: iced::alignment::Horizontal::Center,
            vertical_alignment: iced::alignment::Vertical::Center,
            ..Default::default()
        });
    }
}

fn draw_arrowhead(frame: &mut Frame, tip: Point) {
    let size = 6.0;
    let p1 = Point::new(tip.x - size, tip.y - size * 1.5);
    let p2 = Point::new(tip.x + size, tip.y - size * 1.5);
    frame.fill(
        &Path::new(|b| {
            b.move_to(tip);
            b.line_to(p1);
            b.line_to(p2);
            b.close();
        }),
        Color::from_rgb8(0xd4, 0xaf, 0x37),
    );
}

fn interpolate_color(c1: Color, c2: Color, t: f32) -> Color {
    Color::from_rgb(
        c1.r + (c2.r - c1.r) * t,
        c1.g + (c2.g - c1.g) * t,
        c1.b + (c2.b - c1.b) * t,
    )
}

fn truncate_text(s: &str, max_w: f32) -> String {
    let max_chars = ((max_w / 6.5) as usize).max(3);
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max_chars - 1).collect();
        t.push('…');
        t
    }
}
