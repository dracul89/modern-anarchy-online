//! GUI.dll's layout nodes: `LayoutNode` (0x1012ff40, 0x10130ede), `HLayoutNode` (0x101301c8, 0x10130052), `VLayoutNode`
//! (0x10130722, 0x101305b2) and `View::SpaceOut` (0x1014a44a), which shares a row's or column's length out by weight
//! between its children's minimum and maximum sizes.

use super::geom::{Point, Rect};
use super::view::{Align, Id, Layout, Tree};

/// `View::SpaceOut(count, total, sum of minimums, sum of weights, min, max, weight, out)`: each child gets its minimum
/// plus its weight's share of what is left; a child that would pass its maximum is held there and the rest shared again
/// without it. What is still left then goes to the children below their maximum by how far below it they are (each by
/// what is left after the ones before it). Returns what could not be given.
pub fn space_out(total: f32, sum_min: f32, mut sum_weight: f32, min: &[f32], max: &[f32], weight: Option<&[f32]>, out: &mut [f32]) -> f32 {
    let n = min.len();
    let mut extra = total - sum_min;
    let mut fixed = vec![false; n];
    if weight.is_none() {
        sum_weight = n as f32;
    }
    let mut start = 0;
    loop {
        let mut i = start;
        while i < n {
            if !fixed[i] {
                let (w, share) = match weight {
                    None => (1.0, 1.0 / sum_weight),
                    // (a weight below 1e-5, the constant at 0x101c8204, takes no share)
                    Some(ws) => (ws[i], if ws[i] < 1e-5 { 0.0 } else { ws[i] / sum_weight }),
                };
                out[i] = share * extra + min[i];
                if out[i] >= max[i] {
                    extra -= max[i] - min[i];
                    sum_weight -= w;
                    out[i] = max[i];
                    fixed[i] = true;
                    if i == start {
                        start = i + 1;
                    }
                    break;
                }
            }
            i += 1;
        }
        if i == n {
            break;
        }
    }
    let mut left = total;
    for o in out.iter() {
        left -= o;
    }
    if left > 0.0 {
        let mut room = 0.0;
        for i in 0..n {
            if max[i] <= out[i] {
                fixed[i] = true;
            } else {
                fixed[i] = false;
                room += max[i] - out[i];
            }
        }
        if room > 0.0 {
            for i in 0..n {
                if !fixed[i] {
                    let more = (max[i] - out[i]) / room * left;
                    out[i] += more;
                    left -= more;
                }
            }
        }
    }
    left
}

/// The children a layout counts, with their minimum and maximum preferred sizes and borders.
fn children(tree: &Tree, id: Id) -> Vec<(Id, Point, Point, Rect)> {
    tree.get(id)
        .children
        .iter()
        .copied()
        .filter(|&c| tree.laid_out(c))
        .map(|c| (c, tree.preferred(c, false), tree.preferred(c, true), tree.get(c).borders))
        .collect()
}

/// The layout node's `CalculatePreferredSize(max)`.
pub fn preferred(tree: &Tree, id: Id, max: bool) -> Point {
    let kids = children(tree, id);
    match tree.get(id).layout {
        Layout::None | Layout::Stacked => Point::new(-1.0, -1.0),
        Layout::Base => {
            // (the largest minimum, the smallest maximum, the maximum at least the minimum)
            let (mut lo, mut hi) = (Point::new(-1.0, -1.0), Point::new(16000.0, 16000.0));
            for (_, a, b, r) in &kids {
                lo.x = lo.x.max(a.x + r.left + r.right);
                lo.y = lo.y.max(a.y + r.top + r.bottom);
                hi.x = hi.x.min(b.x + r.left + r.right);
                hi.y = hi.y.min(b.y + r.top + r.bottom);
            }
            hi.x = hi.x.max(lo.x);
            hi.y = hi.y.max(lo.y);
            if max { hi } else { lo }
        }
        Layout::Horizontal => {
            // (widths add up, a pixel each between; the height is the tallest minimum, the shortest maximum)
            let (mut lo, mut hi) = (Point::new(-1.0, 0.0), Point::new(-1.0, 16000.0));
            for (_, a, b, r) in &kids {
                lo.x += a.x + r.left + r.right + 1.0;
                hi.x += b.x + r.left + r.right + 1.0;
                lo.y = lo.y.max(a.y + r.top + r.bottom);
                hi.y = hi.y.min(b.y + r.top + r.bottom);
            }
            hi.y = hi.y.max(lo.y);
            if max { hi } else { lo }
        }
        Layout::Vertical => {
            let (mut lo, mut hi) = (Point::new(0.0, -1.0), Point::new(16000.0, -1.0));
            for (_, a, b, r) in &kids {
                lo.y += a.y + r.top + r.bottom + 1.0;
                hi.y += b.y + r.top + r.bottom + 1.0;
                lo.x = lo.x.max(a.x + r.left + r.right);
                hi.x = hi.x.min(b.x + r.left + r.right);
            }
            hi.x = hi.x.max(lo.x);
            if max { hi } else { lo }
        }
    }
}

/// The layout node's `Layout(view)`: the children's frames.
pub fn layout(tree: &mut Tree, id: Id) {
    let node = tree.get(id);
    let bounds = node.bounds();
    let all = node.children.len();
    match node.layout {
        Layout::None => {}
        Layout::Stacked => {
            for c in node.children.clone() {
                tree.get_mut(c).frame = bounds;
            }
        }
        Layout::Base => {
            for (c, _, _, r) in children(tree, id) {
                tree.get_mut(c).frame = bounds.inset(&r);
            }
        }
        Layout::Horizontal | Layout::Vertical => {
            let horizontal = node.layout == Layout::Horizontal;
            let (h_align, v_align) = (node.h_align, node.v_align);
            let kids = children(tree, id);
            if all == 0 {
                return;
            }
            // (along: the size in pixels, borders in; across: the largest the child may be, borders in)
            let along = |p: Point, r: &Rect| if horizontal { p.x + r.left + r.right + 1.0 } else { p.y + r.top + r.bottom + 1.0 };
            let across = |p: Point, r: &Rect| if horizontal { p.y + r.top + r.bottom } else { p.x + r.left + r.right };
            let min: Vec<f32> = kids.iter().map(|(_, a, _, r)| along(*a, r)).collect();
            let max: Vec<f32> = kids.iter().map(|(_, _, b, r)| along(*b, r)).collect();
            let cross: Vec<f32> = kids.iter().map(|(_, _, b, r)| across(*b, r)).collect();
            let weight: Vec<f32> = kids.iter().map(|(c, ..)| tree.get(*c).weight).collect();
            let (sum_min, sum_weight) = (min.iter().sum(), weight.iter().sum());
            let length = if horizontal { bounds.width() } else { bounds.height() } + 1.0;
            let mut out = vec![0.0; kids.len()];
            let left = space_out(length, sum_min, sum_weight, &min, &max, Some(&weight), &mut out);
            // (what is left is spread round the children: the original divides it by all of them, laid out or not)
            let gap = left / all as f32;
            let mut at = gap * 0.5 + if horizontal { bounds.left } else { bounds.top };
            for (k, (c, _, _, r)) in kids.iter().enumerate() {
                let size = out[k] - 1.0;
                let mut rect = if horizontal { Rect::new(0.0, 0.0, size, bounds.bottom) } else { Rect::new(0.0, 0.0, bounds.right, size) };
                let place = if horizontal {
                    if cross[k] < rect.height() {
                        rect.bottom = cross[k];
                    }
                    let y = match v_align {
                        Align::Top => bounds.top,
                        Align::Bottom => bounds.bottom - rect.height(),
                        _ => (bounds.height() - rect.height()) * 0.5 + bounds.top,
                    };
                    Point::new(at, y)
                } else {
                    if cross[k] < rect.width() {
                        rect.right = cross[k];
                    }
                    // (VLayoutNode knows left, right and centre; for top or bottom the original reads a stale value
                    // and prints an error: left here)
                    let x = match h_align {
                        Align::Right => bounds.right - rect.width(),
                        Align::Center => (bounds.width() - rect.width()) * 0.5 + bounds.left,
                        _ => bounds.left,
                    };
                    Point::new(x, at)
                };
                rect = rect.offset(place);
                at += gap + out[k];
                tree.get_mut(*c).frame = rect.inset(r).floor();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::view::{Node, Plain, Spacer};
    use super::*;

    fn fixed(tree: &mut Tree, w: f32, h: f32) -> Id {
        let mut n = Node::new("", Box::new(Spacer { min: Point::new(w, h), max: Point::new(w, h) }));
        n.flags = 0;
        tree.add(n)
    }

    #[test]
    fn space_out_shares_by_weight_up_to_maximums() {
        let mut out = [0.0; 3];
        // 100 pixels: a fixed 10, two growing ones of weight 1 and 3
        let left = space_out(100.0, 30.0, 5.0, &[10.0, 10.0, 10.0], &[10.0, 1000.0, 1000.0], Some(&[1.0, 1.0, 3.0]), &mut out);
        assert_eq!(out, [10.0, 27.5, 62.5]);
        assert_eq!(left, 0.0);
        // all at their maximum: the rest is left over
        let left = space_out(100.0, 20.0, 2.0, &[10.0, 10.0], &[20.0, 30.0], Some(&[1.0, 1.0]), &mut out[..2]);
        assert_eq!(&out[..2], &[20.0, 30.0]);
        assert_eq!(left, 50.0);
    }

    #[test]
    fn a_row_with_a_spacer_pushes_its_buttons_right() {
        let mut t = Tree::default();
        let mut row = Node::new("row", Box::new(Plain));
        row.layout = Layout::Horizontal;
        let row = t.add(row);
        let spacer = t.add(Node::new("", Box::new(Spacer { min: Point::new(-1.0, -1.0), max: Point::new(15999.0, 15999.0) })));
        let a = fixed(&mut t, 39.0, 19.0);
        let b = fixed(&mut t, 49.0, 19.0);
        for c in [spacer, a, b] {
            t.add_child(row, c);
        }
        // (the row's minimum: the two buttons, 40 + 50 pixels, less one)
        assert_eq!(t.preferred(row, false), Point::new(89.0, 19.0));
        t.set_frame(row, Rect::new(0.0, 0.0, 199.0, 29.0));
        assert_eq!(t.get(a).frame, Rect::new(110.0, 5.0, 149.0, 24.0));
        assert_eq!(t.get(b).frame, Rect::new(150.0, 5.0, 199.0, 24.0));
    }

    #[test]
    fn a_column_without_room_to_grow_spreads_its_children() {
        let mut t = Tree::default();
        let mut col = Node::new("col", Box::new(Plain));
        col.layout = Layout::Vertical;
        col.h_align = Align::Left;
        let col = t.add(col);
        let a = fixed(&mut t, 9.0, 9.0);
        let b = fixed(&mut t, 9.0, 9.0);
        t.add_child(col, a);
        t.add_child(col, b);
        t.set_frame(col, Rect::new(0.0, 0.0, 49.0, 39.0));
        // 40 pixels, 20 used: 10 round each, 5 before the first
        assert_eq!(t.get(a).frame, Rect::new(0.0, 5.0, 9.0, 14.0));
        assert_eq!(t.get(b).frame, Rect::new(0.0, 25.0, 9.0, 34.0));
    }
}
