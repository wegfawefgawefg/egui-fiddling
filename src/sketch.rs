use eframe::egui::{
    self, Color32, FontId, Pos2, Rect, Shape, Slider, Stroke, StrokeKind, output::OutputCommand,
};
use std::collections::HashMap;

pub const FRAMES_PER_SECOND: u32 = 60;

/* ---------------- data types ---------------- */

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    Square,
    Circle,
    Triangle,
}

#[derive(Debug, Clone)]
pub struct SceneObject {
    pub id: u32,
    pub text: String,
    pub text_buffer: String,
    pub shape: ShapeKind,
    pub color: Color32,
    pub rotation_speed: f32,
    pub current_rotation: f32,
    pub children: Vec<SceneObject>,
}

impl SceneObject {
    fn new(id: u32, name: &str, shape: ShapeKind, color: Color32) -> Self {
        Self {
            id,
            text: name.into(),
            text_buffer: name.into(),
            shape,
            color,
            rotation_speed: 20.0,
            current_rotation: 0.0,
            children: vec![],
        }
    }
}

#[derive(Debug, Clone)]
enum EditorRequest {
    AddChild { parent_id: u32 },
    DeleteNode { node_id: u32 },
}

pub struct AppState {
    time_since_last_update: f32,
    scene_objects: Vec<SceneObject>,
    camera_target: egui::Vec2,
    zoom: f32,
    active_settings_id: Option<u32>,
    requests: Vec<EditorRequest>,
    next_id: u32,
    dragging: bool,
    last_pointer: Pos2,
}

impl AppState {
    pub fn new(_: &eframe::CreationContext<'_>) -> Self {
        let mut s = Self {
            time_since_last_update: 0.0,
            scene_objects: vec![],
            camera_target: egui::Vec2::new(400.0, 450.0),
            zoom: 1.0,
            active_settings_id: None,
            requests: vec![],
            next_id: 0,
            dragging: false,
            last_pointer: Pos2::ZERO,
        };

        /* sample tree */
        let mut root = SceneObject::new(s.new_id(), "Root", ShapeKind::Square, Color32::RED);
        let mut a = SceneObject::new(s.new_id(), "Data", ShapeKind::Circle, Color32::BLUE);
        let mut b = SceneObject::new(s.new_id(), "Render", ShapeKind::Triangle, Color32::GREEN);

        a.children.push(SceneObject::new(
            s.new_id(),
            "Mesh",
            ShapeKind::Square,
            Color32::YELLOW,
        ));
        a.children.push(SceneObject::new(
            s.new_id(),
            "Texture",
            ShapeKind::Triangle,
            Color32::from_rgb(255, 128, 0),
        ));
        b.children.push(SceneObject::new(
            s.new_id(),
            "Shader",
            ShapeKind::Circle,
            Color32::from_rgb(128, 0, 255),
        ));

        root.children.push(a);
        root.children.push(b);
        s.scene_objects.push(root);
        s
    }
    fn new_id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }
}

/* ---------------- eframe::App impl ---------------- */

impl eframe::App for AppState {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let dt = 1.0 / FRAMES_PER_SECOND as f32;
        self.time_since_last_update += dt;

        /* ----- pan & zoom ----- */
        let input = ctx.input(|i| i.clone());
        self.zoom =
            (self.zoom + input.raw_scroll_delta.y * 0.001 * ctx.pixels_per_point()).clamp(0.1, 2.0);

        if input.pointer.secondary_down() && !self.dragging {
            self.dragging = true;
            self.last_pointer = input.pointer.hover_pos().unwrap_or(self.last_pointer);
        }
        if self.dragging {
            if let Some(p) = input.pointer.hover_pos() {
                let delta = (p - self.last_pointer) / self.zoom;
                self.camera_target -= egui::Vec2::new(delta.x, delta.y);
                self.last_pointer = p;
            }
            if !input.pointer.secondary_down() {
                self.dragging = false;
            }
        }

        for o in &mut self.scene_objects {
            animate(o, dt);
        }

        /* ----- drawing canvas ----- */
        egui::CentralPanel::default().show(ctx, |ui| {
            let resp = ui.allocate_rect(ui.max_rect(), egui::Sense::click_and_drag());
            let painter = ui.painter();
            let mut layout: HashMap<u32, egui::Vec2> = HashMap::new();
            let mut cy = 100.0;
            for o in &self.scene_objects {
                layout_recursive(o, 200.0, cy, &mut cy, &mut layout);
            }

            let to_screen = |p: egui::Vec2| {
                let offset = ui.max_rect().min.to_vec2() + ui.max_rect().size() / 2.0;
                let v = offset + (p - self.camera_target) * self.zoom;
                Pos2::new(v.x, v.y)
            };
            for (&id, &pos) in &layout {
                if let Some(obj) = find_object_by_id(&self.scene_objects, id) {
                    draw_world(painter, obj, pos, &layout, &to_screen);
                }
            }

            if resp.clicked() && input.pointer.primary_released() {
                if let Some(pos) = input.pointer.interact_pos() {
                    let world =
                        (pos.to_vec2() - ui.max_rect().min.to_vec2() - ui.max_rect().size() / 2.0)
                            / self.zoom
                            + self.camera_target;
                    self.active_settings_id = self
                        .scene_objects
                        .iter()
                        .filter_map(|o| find_clicked_object(o, world, &layout))
                        .next();
                }
            }
        });

        /* ----- inspector ----- */
        if let Some(id) = self.active_settings_id {
            if let Some(obj) = find_object_by_id_mut(&mut self.scene_objects, id) {
                egui::Window::new(format!("Settings: {}", obj.text))
                    .resizable(false)
                    .show(ctx, |ui| {
                        ui.label("Name:");
                        let resp = ui.text_edit_singleline(&mut obj.text_buffer);
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            obj.text.clone_from(&obj.text_buffer);
                        }
                        if ui.button("Apply Name").clicked() {
                            obj.text.clone_from(&obj.text_buffer);
                        }

                        ui.separator();
                        ui.label("Shape:");
                        ui.radio_value(&mut obj.shape, ShapeKind::Square, "Square");
                        ui.radio_value(&mut obj.shape, ShapeKind::Circle, "Circle");
                        ui.radio_value(&mut obj.shape, ShapeKind::Triangle, "Triangle");

                        ui.separator();
                        ui.label("Rotation Speed:");
                        ui.add(Slider::new(&mut obj.rotation_speed, -180.0..=180.0));

                        ui.separator();
                        ui.label("Color:");
                        let rgba = obj.color.to_array(); // Removed 'mut'
                        let mut col = [
                            rgba[0] as f32 / 255.0,
                            rgba[1] as f32 / 255.0,
                            rgba[2] as f32 / 255.0,
                            rgba[3] as f32 / 255.0,
                        ];
                        if ui.color_edit_button_rgba_unmultiplied(&mut col).changed() {
                            obj.color = Color32::from_rgba_unmultiplied(
                                (col[0] * 255.0) as u8,
                                (col[1] * 255.0) as u8,
                                (col[2] * 255.0) as u8,
                                (col[3] * 255.0) as u8,
                            );
                        }

                        ui.horizontal(|ui| {
                            if ui.button("Add Child").clicked() {
                                self.requests
                                    .push(EditorRequest::AddChild { parent_id: id });
                            }
                            if ui.button("Delete Node").clicked() {
                                self.requests
                                    .push(EditorRequest::DeleteNode { node_id: id });
                                self.active_settings_id = None;
                            }
                        });
                    });
            }
        }

        process_requests(
            &mut self.scene_objects,
            &mut self.requests,
            &mut self.next_id,
        );

        /* ----- clipboard ----- */
        for cmd in ctx.output(|o| o.commands.clone()) {
            if let OutputCommand::CopyText(_text) = cmd {} // Prefixed with _ to silence warning
        }

        ctx.request_repaint_after(std::time::Duration::from_secs_f32(
            1.0 / FRAMES_PER_SECOND as f32,
        ));
    }
}

/* ---------------- helpers ---------------- */

fn animate(o: &mut SceneObject, dt: f32) {
    o.current_rotation += o.rotation_speed * dt;
    for c in &mut o.children {
        animate(c, dt);
    }
}

fn layout_recursive(
    o: &SceneObject,
    x: f32,
    y: f32,
    cur: &mut f32,
    m: &mut HashMap<u32, egui::Vec2>,
) -> f32 {
    const XS: f32 = 250.0;
    const YS: f32 = 120.0;
    let mut th = 0.0;
    let mut cy = y;
    for c in &o.children {
        th += layout_recursive(c, x + XS, cy, &mut cy, m);
    }
    let p = if !o.children.is_empty() {
        egui::Vec2::new(x, y + th / 2.0 - YS / 2.0)
    } else {
        egui::Vec2::new(x, *cur)
    };
    m.insert(o.id, p);
    let h = th.max(YS);
    *cur = y + h;
    h
}

fn draw_world<F>(
    painter: &egui::Painter,
    o: &SceneObject,
    p: egui::Vec2,
    m: &HashMap<u32, egui::Vec2>,
    to_screen: &F,
) where
    F: Fn(egui::Vec2) -> Pos2,
{
    for c in &o.children {
        if let Some(&cp) = m.get(&c.id) {
            painter.line_segment(
                [to_screen(p), to_screen(cp)],
                Stroke::new(1.0, Color32::GRAY),
            );
        }
    }

    let center = to_screen(p);
    let sz = 40.0;
    match o.shape {
        ShapeKind::Square => {
            let rect = Rect::from_center_size(center, egui::Vec2::splat(sz));
            painter.rect(rect, 0.0, o.color, Stroke::NONE, StrokeKind::Middle);
        }
        ShapeKind::Circle => {
            painter.circle(center, sz * 0.5, o.color, Stroke::NONE);
        }
        ShapeKind::Triangle => {
            let a = o.current_rotation.to_radians();
            let rot = |v: egui::Vec2| {
                egui::Vec2::new(v.x * a.cos() - v.y * a.sin(), v.x * a.sin() + v.y * a.cos())
                    + center.to_vec2()
            };
            let v = |v: egui::Vec2| Pos2::new(v.x, v.y);
            painter.add(Shape::convex_polygon(
                vec![
                    v(rot(egui::Vec2::new(0.0, -sz / 2.0))),
                    v(rot(egui::Vec2::new(-sz / 2.0, sz / 2.0))),
                    v(rot(egui::Vec2::new(sz / 2.0, sz / 2.0))),
                ],
                o.color,
                Stroke::NONE,
            ));
        }
    }
    painter.text(
        Pos2::new(center.x, center.y + sz * 0.65),
        egui::Align2::CENTER_CENTER,
        &o.text,
        FontId::proportional(16.0),
        Color32::WHITE,
    );
}

// MODIFIED: Takes a mutable reference to next_id to generate new IDs.
fn process_requests(v: &mut Vec<SceneObject>, reqs: &mut Vec<EditorRequest>, next_id: &mut u32) {
    for r in reqs.drain(..) {
        match r {
            EditorRequest::AddChild { parent_id } => {
                if let Some(p) = find_object_by_id_mut(v, parent_id) {
                    // This now uses the AppState's counter, avoiding the borrow error
                    // and fixing the latent ID bug.
                    *next_id += 1;
                    let id = *next_id;
                    p.children.push(SceneObject::new(
                        id,
                        "New Node",
                        ShapeKind::Square,
                        Color32::WHITE,
                    ));
                }
            }
            EditorRequest::DeleteNode { node_id } => {
                find_and_delete_node(v, node_id);
            }
        }
    }
}

// REMOVED: This function is no longer needed.
// fn next_id_recursive(v: &[SceneObject]) -> u32 { ... }

fn find_and_delete_node(v: &mut Vec<SceneObject>, id: u32) -> bool {
    if let Some(i) = v.iter().position(|o| o.id == id) {
        v.remove(i);
        return true;
    }
    v.iter_mut()
        .any(|o| find_and_delete_node(&mut o.children, id))
}

fn find_object_by_id(v: &[SceneObject], id: u32) -> Option<&SceneObject> {
    v.iter().find_map(|o| {
        if o.id == id {
            Some(o)
        } else {
            find_object_by_id(&o.children, id)
        }
    })
}
fn find_object_by_id_mut(v: &mut [SceneObject], id: u32) -> Option<&mut SceneObject> {
    v.iter_mut().find_map(|o| {
        if o.id == id {
            Some(o)
        } else {
            find_object_by_id_mut(&mut o.children, id)
        }
    })
}

fn find_clicked_object(
    o: &SceneObject,
    w: egui::Vec2,
    m: &HashMap<u32, egui::Vec2>,
) -> Option<u32> {
    if let Some(&p) = m.get(&o.id) {
        if (w - p).length() < 20.0 {
            return Some(o.id);
        }
    }
    o.children.iter().find_map(|c| find_clicked_object(c, w, m))
}
