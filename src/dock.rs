//! Icon dock: pointing at the nav dots makes the notch spring a little taller, and each dot slides to its own spot
//! and grows into its slide's icon, plus a Settings gear at the end. No bar behind the icons: the dots themselves
//! turn into the dock. The morph follows the notch height, so it moves with the same spring as the notch.

use super::*;

/// Distance between icon centres.
const STEP: f32 = 26.0;
/// Room for the thin divider between the slide icons and the gear.
const SEP: f32 = 12.0;
const ICON: f32 = 13.0;
/// Where the dots sit, and where the icons end up once the notch has grown by `DOCK_GROW`.
const DOT_Y: f32 = EXP_H - 11.0;
pub(crate) const DOCK_Y: f32 = EXP_H + DOCK_GROW - 22.0;
/// The pointer zone starts this far up, over the dot row of the expanded notch.
const ZONE_TOP: f32 = EXP_H - 24.0;

/// What a click on the dock lands on.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum DockHit {
    /// Index into `App::order`.
    Slide(usize),
    Gear,
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl App {
    /// Slides that get an icon: the carousel minus the Settings slide (the gear stands for it).
    fn dock_slides(&self) -> usize {
        self.order.iter().filter(|&&s| s != SlideId::Settings).count()
    }

    /// Left edge of the icon row and its total width (icons + divider + gear).
    fn dock_span(&self) -> (f32, f32) {
        let total = self.dock_slides() as f32 * STEP + SEP + STEP;
        (CX - total / 2.0, total)
    }

    fn gear_x(&self) -> f32 {
        let (x0, total) = self.dock_span();
        x0 + total - STEP / 2.0
    }

    /// Icon centre for `order[i]`; the Settings slide shares the gear's spot.
    pub(crate) fn dock_x(&self, i: usize) -> f32 {
        if self.order.get(i) == Some(&SlideId::Settings) {
            return self.gear_x();
        }
        let k = self.order[..i].iter().filter(|&&s| s != SlideId::Settings).count();
        self.dock_span().0 + STEP * (k as f32 + 0.5)
    }

    /// How far the dots have turned into icons (0..1): how much of the extra height the notch has grown so far.
    fn dock_e(&self) -> f32 {
        smooth((self.h - EXP_H) / DOCK_GROW)
    }

    /// The pointer is over the dot row (or the open dock): the notch grows by `DOCK_GROW` (see `anim::target`).
    pub(crate) fn dock_wanted(&self) -> bool {
        // A press is not a reason to close, only a real swipe (drag_dx != 0) is. Closing on the press made clicks
        // land on a half-closed dock and get lost.
        if !self.expanded() || self.editing || self.drag_dx != 0.0 || self.lyric_screen() {
            return false;
        }
        let (mx, my) = self.mouse;
        let (_, total) = self.dock_span();
        my >= ZONE_TOP && my <= self.h.max(EXP_H) + 8.0 && (mx - CX).abs() <= total / 2.0 + 10.0
    }

    /// Store the pointer position (logical px). True when the notch needs a redraw (dock opens/closes, hover moves).
    pub(crate) fn set_mouse(&mut self, x: f32, y: f32) -> bool {
        let before = (self.dock_wanted(), self.dock_hit(self.mouse.0, self.mouse.1));
        self.mouse = (x, y);
        let after = (self.dock_wanted(), self.dock_hit(x, y));
        before != after
    }

    /// The dock item under (x, y), only while the dock is (mostly) open.
    pub(crate) fn dock_hit(&self, x: f32, y: f32) -> Option<DockHit> {
        let e = self.dock_e();
        let row_y = DOT_Y + (DOCK_Y - DOT_Y) * e;
        if e < 0.5 || (y - row_y).abs() > 16.0 {
            return None;
        }
        if (x - self.gear_x()).abs() <= STEP / 2.0 {
            return Some(DockHit::Gear);
        }
        (0..self.order.len())
            .filter(|&i| self.order[i] != SlideId::Settings)
            .find(|&i| (x - self.dock_x(i)).abs() <= STEP / 2.0)
            .map(DockHit::Slide)
    }

    /// One dot-or-icon: the dot shrinks away while the icon grows out of the same spot. `dot`: draw the dot at all
    /// (the gear has none unless the Settings slide is open).
    fn dock_item(&self, g: &mut Gfx, id: SlideId, x: f32, y: f32, near: f32, hot: bool, dot: bool, e: f32, p: f32) {
        let da = (1.0 - e * 2.2).clamp(0.0, 1.0);
        if dot && da > 0.0 {
            let r = (2.3 + 1.2 * near) * (0.6 + 0.4 * da);
            let a = (0.28 + 0.72 * near) * p * da;
            if near > 0.5 {
                g.circle(x, y, r, &g.sacc(a));
            } else {
                g.circle(x, y, r, &paint(255, 255, 255, a));
            }
        }
        let k = ((e - 0.25) / 0.75).clamp(0.0, 1.0);
        if k <= 0.0 {
            return;
        }
        let size = ICON * (0.4 + 0.6 * k) * if hot { 1.18 } else { 1.0 };
        let ia = k * p;
        let col = if near > 0.5 { g.sacc(ia) } else { paint(255, 255, 255, if hot { 1.0 } else { 0.6 } * ia) };
        g.slide_icon(id, x, y, size, &col);
    }

    /// Dots when the dock is closed, the icons when it is open, and the morph between them.
    pub(crate) fn draw_dots(&self, g: &mut Gfx, pos: f32, p: f32) {
        let n = self.n_slides();
        let e = self.dock_e();
        let hot = self.dock_hit(self.mouse.0, self.mouse.1);
        let y = DOT_Y + (DOCK_Y - DOT_Y) * e;
        for i in 0..n {
            let id = self.order[i];
            let near = (1.0 - cyc(i as f32 - pos, n).abs()).clamp(0.0, 1.0);
            let x = dot_x(i, n) + (self.dock_x(i) - dot_x(i, n)) * e;
            let is_hot = match hot {
                Some(DockHit::Slide(h)) => h == i,
                Some(DockHit::Gear) => id == SlideId::Settings,
                None => false,
            };
            self.dock_item(g, id, x, y, near, is_hot, true, e, p);
        }
        if e > 0.01 {
            let (x0, _) = self.dock_span();
            let sx = x0 + self.dock_slides() as f32 * STEP + SEP / 2.0;
            g.line(sx, y - 5.0 * e, sx, y + 5.0 * e, 1.0, &paint(255, 255, 255, 0.2 * e * p));
            if !self.order.contains(&SlideId::Settings) {
                self.dock_item(g, SlideId::Settings, self.gear_x(), y, 0.0, hot == Some(DockHit::Gear), false, e, p);
            }
        }
    }
}

/// Extra outline icons for the Settings window sidebar (`Gfx::icon_outline`).
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum OutlineIcon {
    Palette,
    Monitor,
    Layers,
    Speaker,
    Cloud,
    Info,
}

/// Outline icons in a unit box (-1..1 on both axes), drawn with round caps like the rest of the dock.
struct Pen<'a, 'p> {
    g: &'a mut Gfx,
    cx: f32,
    cy: f32,
    h: f32,
    p: &'a Paint<'p>,
}

impl Pen<'_, '_> {
    fn at(&self, x: f32, y: f32) -> (f32, f32) {
        (self.cx + x * self.h, self.cy + y * self.h)
    }

    fn width(&self) -> f32 {
        (self.h * 0.24).max(1.3)
    }

    fn stroke(&mut self, path: Path) {
        let st = Stroke { width: self.width(), line_cap: LineCap::Round, line_join: tiny_skia::LineJoin::Round, ..Default::default() };
        self.g.pm.stroke_path(&path, self.p, &st, self.g.ts, None);
    }

    fn polyline(&mut self, pts: &[(f32, f32)], close: bool) {
        let mut pb = PathBuilder::new();
        for (i, &(x, y)) in pts.iter().enumerate() {
            let (px, py) = self.at(x, y);
            if i == 0 {
                pb.move_to(px, py);
            } else {
                pb.line_to(px, py);
            }
        }
        if close {
            pb.close();
        }
        if let Some(path) = pb.finish() {
            self.stroke(path);
        }
    }

    fn rrect(&mut self, x: f32, y: f32, w: f32, h: f32, r: f32) {
        let (px, py) = self.at(x, y);
        self.stroke(rrect(px, py, w * self.h, h * self.h, r * self.h));
    }

    fn circle(&mut self, x: f32, y: f32, r: f32) {
        let (px, py) = self.at(x, y);
        if let Some(path) = PathBuilder::from_circle(px, py, r * self.h) {
            self.stroke(path);
        }
    }

    /// Teardrop, same curve as `Gfx::icon_drop` but as an outline.
    fn drop(&mut self) {
        let q = |x: f32, y: f32| self.at(x * 0.9, y * 0.95);
        let segs = [
            [q(0.55, -0.35), q(0.7, 0.05), q(0.7, 0.3)],
            [q(0.7, 0.7), q(0.38, 1.0), q(0.0, 1.0)],
            [q(-0.38, 1.0), q(-0.7, 0.7), q(-0.7, 0.3)],
            [q(-0.7, 0.05), q(-0.55, -0.35), q(0.0, -1.0)],
        ];
        let start = q(0.0, -1.0);
        let mut pb = PathBuilder::new();
        pb.move_to(start.0, start.1);
        for [c1, c2, e] in segs {
            pb.cubic_to(c1.0, c1.1, c2.0, c2.1, e.0, e.1);
        }
        pb.close();
        if let Some(path) = pb.finish() {
            self.stroke(path);
        }
    }

    fn dot(&mut self, x: f32, y: f32, r: f32) {
        let (px, py) = self.at(x, y);
        self.g.circle(px, py, r * self.h, self.p);
    }
}

impl Gfx {
    /// Gear: a ring with eight teeth around a small hole.
    pub(crate) fn icon_gear(&mut self, cx: f32, cy: f32, s: f32, p: &Paint) {
        let mut pen = Pen { g: self, cx, cy, h: s / 2.0, p };
        // Eight flat-topped teeth: each one is two points on the outer radius between two on the inner one.
        let mut pts = Vec::with_capacity(32);
        for k in 0..8 {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            for (da, r) in [(-0.39, 0.66), (-0.2, 0.96), (0.2, 0.96), (0.39, 0.66)] {
                pts.push(((a + da).cos() * r, (a + da).sin() * r));
            }
        }
        pen.polyline(&pts, true);
        pen.circle(0.0, 0.0, 0.26);
    }

    /// The Settings window's sidebar icons, in the same outline style as the dock.
    pub(crate) fn icon_outline(&mut self, kind: OutlineIcon, cx: f32, cy: f32, s: f32, p: &Paint) {
        let mut pen = Pen { g: self, cx, cy, h: s / 2.0, p };
        match kind {
            OutlineIcon::Palette => {
                pen.circle(0.0, 0.0, 0.9);
                pen.dot(-0.42, 0.05, 0.13);
                pen.dot(-0.18, -0.42, 0.13);
                pen.dot(0.32, -0.36, 0.13);
                pen.circle(0.3, 0.38, 0.2);
            }
            OutlineIcon::Monitor => {
                pen.rrect(-0.95, -0.8, 1.9, 1.25, 0.2);
                pen.polyline(&[(0.0, 0.45), (0.0, 0.85)], false);
                pen.polyline(&[(-0.42, 0.88), (0.42, 0.88)], false);
            }
            OutlineIcon::Layers => {
                pen.polyline(&[(0.0, -0.9), (0.95, -0.4), (0.0, 0.1), (-0.95, -0.4)], true);
                pen.polyline(&[(-0.95, 0.05), (0.0, 0.55), (0.95, 0.05)], false);
                pen.polyline(&[(-0.95, 0.48), (0.0, 0.98), (0.95, 0.48)], false);
            }
            OutlineIcon::Speaker => {
                pen.polyline(&[(-0.9, -0.3), (-0.5, -0.3), (0.0, -0.8), (0.0, 0.8), (-0.5, 0.3), (-0.9, 0.3)], true);
                let arc = |r: f32| -> Vec<(f32, f32)> {
                    (0..=8).map(|i| {
                        let a = -0.75 + 1.5 * i as f32 / 8.0;
                        (0.05 + a.cos() * r, a.sin() * r)
                    }).collect()
                };
                pen.polyline(&arc(0.42), false);
                pen.polyline(&arc(0.82), false);
            }
            OutlineIcon::Cloud => {
                let mut pts = Vec::new();
                for (ox, oy, r, a0, a1) in [(-0.45, 0.2, 0.42, 1.57, 3.9), (0.02, -0.18, 0.55, 3.6, 6.0), (0.5, 0.18, 0.44, 5.0, 7.85)] {
                    for i in 0..=8 {
                        let a = a0 + (a1 - a0) * i as f32 / 8.0;
                        pts.push((ox + a.cos() * r, oy + a.sin() * r));
                    }
                }
                pen.polyline(&pts, true);
            }
            OutlineIcon::Info => {
                pen.circle(0.0, 0.0, 0.9);
                pen.polyline(&[(0.0, -0.05), (0.0, 0.5)], false);
                pen.dot(0.0, -0.42, 0.12);
            }
        }
    }

    /// The dock icon for a slide (outline style, `s` = box size in px).
    pub(crate) fn slide_icon(&mut self, id: SlideId, cx: f32, cy: f32, s: f32, p: &Paint) {
        let mut pen = Pen { g: self, cx, cy, h: s / 2.0, p };
        match id {
            SlideId::Music => {
                pen.polyline(&[(-0.35, 0.6), (-0.35, -0.75), (0.85, -0.95), (0.85, 0.42)], false);
                pen.dot(-0.62, 0.62, 0.32);
                pen.dot(0.58, 0.44, 0.32);
            }
            SlideId::Water => pen.drop(),
            SlideId::Todo => {
                pen.polyline(&[(-0.95, -0.5), (-0.72, -0.27), (-0.3, -0.72)], false);
                pen.polyline(&[(-0.95, 0.45), (-0.72, 0.68), (-0.3, 0.23)], false);
                pen.polyline(&[(0.1, -0.5), (0.95, -0.5)], false);
                pen.polyline(&[(0.1, 0.45), (0.95, 0.45)], false);
            }
            SlideId::Ledge => {
                pen.polyline(&[(-0.95, 0.1), (-0.95, 0.85), (0.95, 0.85), (0.95, 0.1), (0.55, -0.8), (-0.55, -0.8)], true);
                pen.polyline(&[(-0.95, 0.1), (-0.38, 0.1), (-0.22, 0.38), (0.22, 0.38), (0.38, 0.1), (0.95, 0.1)], false);
            }
            SlideId::Scrap => {
                pen.rrect(-0.72, -0.92, 1.44, 1.84, 0.28);
                for y in [-0.38, 0.02, 0.42] {
                    pen.polyline(&[(-0.32, y), (0.32, y)], false);
                }
            }
            SlideId::Notifications => {
                pen.polyline(
                    &[(-0.85, 0.48), (-0.62, 0.2), (-0.62, -0.18), (-0.38, -0.66), (0.0, -0.82), (0.38, -0.66), (0.62, -0.18), (0.62, 0.2), (0.85, 0.48)],
                    true,
                );
                pen.polyline(&[(-0.25, 0.8), (0.25, 0.8)], false);
            }
            SlideId::System => {
                pen.polyline(&[(-1.0, 0.05), (-0.5, 0.05), (-0.25, -0.75), (0.2, 0.8), (0.45, 0.05), (1.0, 0.05)], false);
            }
            SlideId::Timer => {
                pen.circle(0.0, 0.15, 0.78);
                pen.polyline(&[(0.0, 0.15), (0.0, -0.25)], false);
                pen.polyline(&[(-0.28, -0.98), (0.28, -0.98)], false);
            }
            SlideId::Clipboard => {
                pen.rrect(-0.75, -0.72, 1.5, 1.7, 0.26);
                pen.rrect(-0.34, -0.98, 0.68, 0.4, 0.12);
                pen.polyline(&[(-0.32, 0.12), (0.32, 0.12)], false);
                pen.polyline(&[(-0.32, 0.5), (0.18, 0.5)], false);
            }
            SlideId::Agenda => {
                pen.rrect(-0.88, -0.66, 1.76, 1.6, 0.28);
                pen.polyline(&[(-0.88, -0.14), (0.88, -0.14)], false);
                pen.polyline(&[(-0.42, -0.95), (-0.42, -0.5)], false);
                pen.polyline(&[(0.42, -0.95), (0.42, -0.5)], false);
            }
            SlideId::Settings => {
                pen.g.icon_gear(cx, cy, s, p);
            }
        }
    }
}
