//! The original's `Point` and `Rect` (Utils.dll, float coordinates as in BeOS): a rect's `width()` is `right - left`, so a
//! rect covering the pixels 0 to 9 is `Rect(0, 0, 9, 9)` and is "9 wide"; a size of -1 is no pixels. The layout code keeps
//! these units throughout, as GUI.dll does (the `+ 1.0` and `- 1.0` around `SpaceOut` convert to and from pixels).

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Point {
        Point { x, y }
    }
}

impl std::ops::Add for Point {
    type Output = Point;
    fn add(self, o: Point) -> Point {
        Point::new(self.x + o.x, self.y + o.y)
    }
}

impl std::ops::Sub for Point {
    type Output = Point;
    fn sub(self, o: Point) -> Point {
        Point::new(self.x - o.x, self.y - o.y)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Rect {
    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Rect {
        Rect { left, top, right, bottom }
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    pub fn left_top(&self) -> Point {
        Point::new(self.left, self.top)
    }

    /// `Rect::operator+=(Point)`: moved by `p`.
    pub fn offset(self, p: Point) -> Rect {
        Rect::new(self.left + p.x, self.top + p.y, self.right + p.x, self.bottom + p.y)
    }

    /// `Rect::Resize(l, t, r, b)`: each side moved by its amount (FUN_1012fed0 passes a view's borders as `l, t, -r, -b`:
    /// the rect inset by them).
    pub fn resize(self, l: f32, t: f32, r: f32, b: f32) -> Rect {
        Rect::new(self.left + l, self.top + t, self.right + r, self.bottom + b)
    }

    /// Inset by a view's borders (FUN_1012fed0).
    pub fn inset(self, borders: &Rect) -> Rect {
        self.resize(borders.left, borders.top, -borders.right, -borders.bottom)
    }

    /// `Rect::Floor`: every side floored.
    pub fn floor(self) -> Rect {
        Rect::new(self.left.floor(), self.top.floor(), self.right.floor(), self.bottom.floor())
    }

    /// The bounds of a frame: the same size at (0, 0) (`View::GetBounds`).
    pub fn bounds(&self) -> Rect {
        Rect::new(0.0, 0.0, self.width(), self.height())
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.left && p.x <= self.right && p.y >= self.top && p.y <= self.bottom
    }
}
