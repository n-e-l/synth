use std::f32::consts::{PI};
use std::ops::{Add, RangeInclusive, Sub};
use cen::egui::{Align2, Color32, FontId, Painter, Pos2, Response, Sense, Stroke, Ui, Vec2, Widget};
use cen::egui::epaint::PathShape;

pub struct Knob<'a> {
    value: &'a mut f32,
    pub range: RangeInclusive<f32>,
    text: Option<String>,
}

impl<'a> Knob<'a> {
    pub fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            value,
            range,
            text: None
        }
    }

    pub fn text(mut self, text: impl ToString) -> Self {
        self.text = Some(text.to_string());
        self
    }
}

fn filled_arc(painter: &Painter, center: Pos2, r1: f32, r2: f32, start_angle: f32, end_angle: f32, color: Color32) {
    let steps = 32;

    for i in 0..steps {
        let mut points = vec![];

        let t1 = i as f32 / steps as f32;
        let t2 = (i as f32 + 1f32) / steps as f32;
        let angle1 = start_angle + t1 * (end_angle - start_angle);
        let angle2 = start_angle + t2 * (end_angle - start_angle);

        points.push(Pos2::new(
            center.x + r1 * angle1.cos(),
            center.y + r1 * angle1.sin(),
        ));
        points.push(Pos2::new(
            center.x + r2 * angle1.cos(),
            center.y + r2 * angle1.sin(),
        ));
        points.push(Pos2::new(
            center.x + r2 * angle2.cos(),
            center.y + r2 * angle2.sin(),
        ));
        points.push(Pos2::new(
            center.x + r1 * angle2.cos(),
            center.y + r1 * angle2.sin(),
        ));

        painter.add(PathShape::convex_polygon(points, color, Stroke::NONE));
    }

}

impl Widget for Knob<'_> {
    fn ui(self, ui: &mut Ui) -> Response {

        let radius = 20.0;
        let padding = 2.0;
        let text_height = 10.0;

        let (response, painter) = ui.allocate_painter(Vec2::splat((radius + padding) * 2.0).add(Vec2::new(0f32, text_height)), Sense::drag());
        let pot_center = response.rect.center().sub(Vec2::new(0f32, text_height / 2f32));
        let visuals = ui.style().interact(&response);

        let inner_radius = radius - 7f32;
        painter.circle_filled(pot_center, inner_radius, visuals.bg_fill);
        // painter.circle_stroke(pot_center, inner_radius, visuals.bg_stroke);

        // Relative range
        let rel_value = ( *self.value - self.range.start() ) / self.range.end();
        let start_angle = 1.5 * PI / 2f32;
        let end_angle = start_angle + rel_value * 1.5 * PI;
        painter.line(vec![pot_center, pot_center.add(inner_radius * Vec2::new(f32::cos(end_angle), f32::sin(end_angle)))], visuals.fg_stroke);

        filled_arc(&painter, pot_center, radius - 3f32, radius, start_angle, end_angle, Color32::ORANGE);
        filled_arc(&painter, pot_center, radius - 3f32, radius, end_angle, start_angle + 1.5 * PI, Color32::from_rgba_unmultiplied(50, 50, 50, 255));

        let drag = response.drag_delta().y;
        *self.value -= (self.range.end() - self.range.start()) / 200.0 * drag;
        *self.value = self.value.clamp(*self.range.start(), *self.range.end());

        if let Some(text) = self.text {
            painter.text(response.rect.center_bottom(), Align2::CENTER_BOTTOM, text, FontId::monospace(10f32), visuals.text_color());
        }

        response
    }
}