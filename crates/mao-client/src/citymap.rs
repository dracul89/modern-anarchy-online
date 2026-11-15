//! The city's map of the controller's window: what GUI.dll's `BuildingMapView_c` keeps and decides (the drawing and the
//! mouse are `cityterminal.rs`). The reading is `docs/CITY_MAP_ORIGINAL.md`.
//!
//! The map holds a list of nodes: the city's houses (from the playfield's list) and the houses about to be built
//! ("pending": a building item dropped on the map, which waits there till "Construct"). A node covers the tiles of
//! its template's footprint, turned by its rotation, from its own tile as the least corner. A house may be built
//! where every tile of the turned footprint's rectangle lies in one of the city's zones and every covered tile on a
//! zone's set tile (`CityAI_c::CanBuildLayout`, city.dll 0x10002882), and where it shares no tile with another node.

use mao_formats::house::{covers_turned, rotate_in};

/// A world unit's tiles: a tile is 4 units.
pub const TILE: f32 = 4.0;

/// What the map draws and tests of a house template.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Shape {
    /// The house's type: 1 headquarters, 2 market, 3 grid, 4 guard house, 5 uplink, 6 cloaking device, 8 tax office,
    /// 9 a unique house; 0 and 7 plain ones.
    pub kind: i32,
    pub tiles: (i32, i32),
    pub mask: Vec<u8>,
    /// The doors' and the guards' places in the unturned footprint.
    pub doors: Vec<[f32; 3]>,
    pub guards: Vec<[f32; 3]>,
}

impl Shape {
    /// The turned footprint's rectangle, in tiles.
    pub fn size(&self, rotation: i32) -> (i32, i32) {
        if matches!(rotation, 90 | 270) { (self.tiles.1, self.tiles.0) } else { self.tiles }
    }

    /// Whether the turned footprint covers its tile (x, z).
    pub fn covers(&self, rotation: i32, x: i32, z: i32) -> bool {
        covers_turned(&self.mask, self.tiles, rotation, x, z)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// The building item's place in the inventory for a house about to be built; none for one that stands.
    pub item: Option<u32>,
    /// The house template's number (its identity is `(0xf4268, it)`).
    pub template: u32,
    pub shape: Shape,
    /// The least corner's tile.
    pub tile: (i32, i32),
    pub rotation: i32,
    pub demolishing: bool,
    pub valid: bool,
}

impl Node {
    pub fn pending(&self) -> bool {
        self.item.is_some()
    }

    /// Whether the node covers a tile of the world.
    pub fn covers(&self, tile: (i32, i32)) -> bool {
        self.shape.covers(self.rotation, tile.0 - self.tile.0, tile.1 - self.tile.1)
    }

    /// The tiles of the world it covers.
    pub fn tiles(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        let (w, h) = self.shape.size(self.rotation);
        (0..h).flat_map(move |z| (0..w).map(move |x| (x, z))).filter(|&(x, z)| self.shape.covers(self.rotation, x, z)).map(|(x, z)| (self.tile.0 + x, self.tile.1 + z))
    }

    /// `HouseLayout_c::Intersects`: the two share a tile.
    pub fn intersects(&self, other: &Node) -> bool {
        other.tiles().any(|t| self.covers(t))
    }

    /// A place of the template (a door's, a guard's) in the world, on the ground plane: x and z.
    pub fn place(&self, local: [f32; 3]) -> [f32; 2] {
        let p = rotate_in(self.shape.tiles, local, self.rotation);
        [self.tile.0 as f32 * TILE + p[0], self.tile.1 as f32 * TILE + p[2]]
    }
}

/// The city's zones: each ten tiles by ten from its corner, with the tiles of it a house may stand on.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Zones(pub Vec<((i32, i32), [u8; 13])>);

impl Zones {
    fn bit(mask: &[u8; 13], x: i32, z: i32) -> bool {
        let bit = (z * 10 + x) as usize;
        mask[bit / 8] >> (bit % 8) & 1 == 1
    }

    /// The zones' set tiles.
    pub fn tiles(&self) -> impl Iterator<Item = (i32, i32)> + '_ {
        self.0.iter().flat_map(|(corner, mask)| (0..10).flat_map(move |z| (0..10).map(move |x| (x, z))).filter(|&(x, z)| Self::bit(mask, x, z)).map(|(x, z)| (corner.0 + x, corner.1 + z)))
    }

    /// `CanBuildLayout`: every tile of the node's turned rectangle lies in some zone's rectangle, and on a set tile
    /// of it when the node covers the tile.
    pub fn can_build(&self, node: &Node) -> bool {
        let (w, h) = node.shape.size(node.rotation);
        (0..h).all(|z| {
            (0..w).all(|x| {
                let covered = node.shape.covers(node.rotation, x, z);
                self.0.iter().any(|(corner, mask)| {
                    let (gx, gz) = (node.tile.0 + x - corner.0, node.tile.1 + z - corner.1);
                    (0..10).contains(&gx) && (0..10).contains(&gz) && (!covered || Self::bit(mask, gx, gz))
                })
            })
        })
    }

    /// The zones' rectangles' union, in tiles: left, low z, right, high z (the far edges past the last tile).
    pub fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        let x = |f: fn(i32, i32) -> i32, start: i32, add: i32| self.0.iter().map(|(c, _)| c.0 + add).fold(start, f);
        let z = |f: fn(i32, i32) -> i32, start: i32, add: i32| self.0.iter().map(|(c, _)| c.1 + add).fold(start, f);
        (!self.0.is_empty()).then(|| (x(i32::min, i32::MAX, 0), z(i32::min, i32::MAX, 0), x(i32::max, i32::MIN, 10), z(i32::max, i32::MIN, 10)))
    }
}

/// A building item of the backpack: its place, the house template it builds and the side it is for (stat 33, when the
/// item has one).
#[derive(Clone, Debug, PartialEq)]
pub struct Buildable {
    pub slot: u32,
    pub template: u32,
    pub side: Option<i64>,
    pub shape: Shape,
}

/// What the network loop tells the map: the city's houses as nodes, the backpack's building items, and (with the
/// session's opening) the playfield's zones and size in tiles.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Data {
    pub houses: Vec<Node>,
    pub buildables: Vec<Buildable>,
    pub zones: Zones,
    pub tiles: (i32, i32),
    /// What the land costs and its upkeep (the city record's).
    pub costs: (u32, u32),
}

/// Why a drop on the map was turned away: the text's category and name, and what it is fed.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// Not a building item (10000 `OnlyDropHousesOnMap`).
    NotAHouse,
    /// A building of another side (10002 `CityTerminal_HouseCanOnlyBeBuiltBy`, fed the side's name).
    Side(i64),
    /// One of the limits (category 10000).
    Limit(&'static str),
}

/// A house to ask the server to build.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Build {
    pub template: u32,
    pub tile: (i32, i32),
    pub rotation: i32,
    pub slot: u32,
}

/// The mode of the view: `BuildingMapView_c` +0x470.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    Build,
    Demolish,
    #[default]
    View,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Map {
    pub mode: Mode,
    /// The houses that stand, then the ones about to be built in the order they were dropped.
    pub nodes: Vec<Node>,
    pub selected: Option<usize>,
    pub zones: Zones,
    /// The playfield's size in tiles.
    pub tiles: (i32, i32),
    pub buildables: Vec<Buildable>,
    /// No market may be built here (the playfield's model is of type 0xc79e).
    pub no_market: bool,
    /// "Construct" may be pressed.
    pub can_construct: bool,
}

impl Map {
    /// The network loop's word: the standing houses anew, the pending ones kept (one whose item left the backpack is
    /// dropped, as the original's item signals drop it).
    pub fn set(&mut self, data: Data) {
        let selected = self.selected.and_then(|i| self.nodes.get(i)).map(|n| (n.item, n.tile));
        let pending: Vec<Node> = self.nodes.drain(..).filter(|n| n.item.is_some_and(|slot| data.buildables.iter().any(|b| b.slot == slot && b.template == n.template))).collect();
        self.nodes = data.houses;
        self.nodes.extend(pending);
        self.selected = selected.and_then(|s| self.nodes.iter().position(|n| (n.item, n.tile) == s && !n.demolishing));
        self.buildables = data.buildables;
        if !data.zones.0.is_empty() {
            self.zones = data.zones;
            self.tiles = data.tiles;
        }
        self.recompute();
    }

    /// How many nodes there are of each house type (+0x4bc).
    pub fn count(&self, kind: i32) -> usize {
        self.nodes.iter().filter(|n| n.shape.kind == kind).count()
    }

    /// The slots of the items that wait on the map: the inventory holds them locked (`InventoryItemLocker_c`).
    pub fn locked(&self) -> Vec<u32> {
        self.nodes.iter().filter_map(|n| n.item).collect()
    }

    /// The node that covers a tile: the first in the list (FUN_100e3afe).
    pub fn hit(&self, tile: (i32, i32)) -> Option<usize> {
        self.nodes.iter().position(|n| n.covers(tile))
    }

    /// FUN_100e39d6: a node whose demolishing has begun is not selected (what was selected stays).
    pub fn select(&mut self, node: Option<usize>) {
        match node {
            Some(i) if self.nodes.get(i).is_some_and(|n| n.demolishing) => {}
            other => self.selected = other,
        }
    }

    /// An inventory item dropped on a tile (FUN_100e56a8, FUN_100e4d71): a building item of the character's side, not
    /// past the limits of its type, is a new pending node with that tile as its least corner, unturned.
    pub fn drop_item(&mut self, slot: u32, tile: (i32, i32), own_side: Option<i64>) -> Result<(), Refusal> {
        // (an item that waits on the map already is locked in the original's inventory and cannot be taken up again:
        // here it is not taken a second time)
        if self.nodes.iter().any(|n| n.item == Some(slot)) {
            return Ok(());
        }
        let Some(b) = self.buildables.iter().find(|b| b.slot == slot).cloned() else { return Err(Refusal::NotAHouse) };
        // (an item with no side is `N3Msg_GetSkill`'s 1234567890: any side's)
        if let Some(side) = b.side.filter(|&s| s != 1_234_567_890)
            && Some(side) != own_side
        {
            return Err(Refusal::Side(side));
        }
        let one = |kind: i32, text: &'static str| if self.count(kind) >= 1 { Err(Refusal::Limit(text)) } else { Ok(()) };
        match b.shape.kind {
            1 => one(1, "AlreadyHaveHQhere")?,
            3 => one(3, "AlreadyHaveGridHere")?,
            5 => one(5, "AlreadyHaveUplinkHere")?,
            6 => one(6, "AlreadyHaveCloakDev")?,
            2 => {
                one(2, "AlreadyHaveMarket")?;
                if self.no_market {
                    return Err(Refusal::Limit("NoMarketPFType"));
                }
            }
            8 => one(8, "AlreadyHaveTaxOfficeHere")?,
            4 => one(4, "AlreadyHaveUniqueHouse")?,
            9 if self.nodes.iter().any(|n| n.template == b.template) => return Err(Refusal::Limit("AlreadyHaveUniqueHouse")),
            _ => {}
        }
        self.nodes.push(Node { item: Some(slot), template: b.template, shape: b.shape, tile, rotation: 0, demolishing: false, valid: true });
        self.recompute();
        Ok(())
    }

    /// FUN_100e44a2: which pending nodes are valid and whether "Construct" may be pressed: some node is pending, no two
    /// nodes share a tile, there is a headquarters (standing or pending) and every pending node lies in the zones.
    pub fn recompute(&mut self) {
        let mut flag = false;
        for n in self.nodes.iter_mut().filter(|n| n.pending()) {
            n.valid = true;
            flag = true;
        }
        for a in 0..self.nodes.len() {
            for b in a + 1..self.nodes.len() {
                if self.nodes[a].intersects(&self.nodes[b]) {
                    self.nodes[a].valid = false;
                    self.nodes[b].valid = false;
                    flag = false;
                }
            }
        }
        if self.count(1) == 0 {
            flag = false;
        }
        let zones = &self.zones;
        for n in self.nodes.iter_mut().filter(|n| n.pending() && n.valid) {
            if !zones.can_build(n) {
                n.valid = false;
                flag = false;
            }
        }
        self.can_construct = flag;
    }

    fn selected_node(&self) -> Option<&Node> {
        self.selected.and_then(|i| self.nodes.get(i))
    }

    /// Rotate and Delete want a pending node selected; Demolish a standing one whose demolishing has not begun
    /// (FUN_100e7ec9).
    pub fn pending_selected(&self) -> bool {
        self.selected_node().is_some_and(Node::pending)
    }

    pub fn standing_selected(&self) -> bool {
        self.selected_node().is_some_and(|n| !n.pending() && !n.demolishing)
    }

    /// FUN_100e4843: the selected pending node a quarter turn on, about its least corner's tile.
    pub fn rotate(&mut self) {
        if let Some(n) = self.selected.and_then(|i| self.nodes.get_mut(i)).filter(|n| n.pending()) {
            n.rotation = (n.rotation + 90) % 360;
            self.recompute();
        }
    }

    /// FUN_100e4882: the selected pending node off the map.
    pub fn delete(&mut self) {
        if self.pending_selected() {
            self.nodes.remove(self.selected.take().unwrap());
            self.recompute();
        }
    }

    /// FUN_100e498a: the houses to ask for: the first pending headquarters, then the others in their order. They leave
    /// the map at once; "Construct" stays as it was till the next recompute (the original's stale flag), by which
    /// time nothing is pending.
    pub fn construct(&mut self) -> Vec<Build> {
        if !self.can_construct {
            return Vec::new();
        }
        let build = |n: &Node| Build { template: n.template, tile: n.tile, rotation: n.rotation, slot: n.item.unwrap_or(0) };
        let mut pending: Vec<&Node> = self.nodes.iter().filter(|n| n.pending()).collect();
        let mut out = Vec::new();
        if let Some(hq) = pending.iter().position(|n| n.shape.kind == 1) {
            out.push(build(pending.remove(hq)));
        }
        out.extend(pending.into_iter().map(build));
        if self.selected_node().is_some_and(Node::pending) {
            self.selected = None;
        } else if let Some(i) = self.selected {
            // (the standing nodes come first: their places do not move)
            self.selected = Some(i).filter(|&i| !self.nodes[i].pending());
        }
        self.nodes.retain(|n| !n.pending());
        out
    }

    /// FUN_100e48bb: the selected standing house's tile, to ask the server to demolish it; nothing is selected after.
    /// The house stays on the map till the server says its demolishing began, and that it is gone.
    pub fn demolish(&mut self) -> Option<(i32, i32)> {
        let tile = self.selected_node().filter(|n| !n.pending()).map(|n| n.tile)?;
        self.selected = None;
        Some(tile)
    }

    /// A pending node moved so that its least corner is on `tile` (the mouse's drag, FUN_100e4abf): anywhere at all;
    /// it is not valid where it may not stand.
    pub fn move_selected(&mut self, tile: (i32, i32)) {
        if let Some(n) = self.selected.and_then(|i| self.nodes.get_mut(i)).filter(|n| n.pending()) {
            if n.tile != tile {
                n.tile = tile;
                self.recompute();
            }
        }
    }
}

/// The view's geometry (`PFMapGroundRenderer_c` with the margins of FUN_100e375d): the zones' rectangles and 50 units
/// round them, 2 pixels to a world unit, +x to the right and +z up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    /// The world's x at the left edge and z at the top edge.
    pub left: f32,
    pub top: f32,
    /// The content's size in pixels.
    pub size: [f32; 2],
}

pub const SCALE: f32 = 2.0;

impl Geometry {
    pub fn new(zones: &Zones, tiles: (i32, i32)) -> Option<Geometry> {
        let (l, t, r, b) = zones.bounds()?;
        let (w4, h4) = (tiles.0 as f32 * TILE, tiles.1 as f32 * TILE);
        let left = (l as f32 * TILE - 50.0).max(0.0);
        let low = (t as f32 * TILE - 50.0).max(0.0);
        let right = (w4 - 1.0 - r as f32 * TILE - 50.0).max(0.0);
        let high = (h4 - 1.0 - b as f32 * TILE - 50.0).max(0.0);
        Some(Geometry { left, top: h4 - high, size: [(w4 - left - right) * SCALE, (h4 - low - high) * SCALE] })
    }

    /// A point of the content (pixels from its top left) in the world: x and z.
    pub fn world(&self, p: [f32; 2]) -> [f32; 2] {
        [p[0] / SCALE + self.left, self.top - p[1] / SCALE]
    }

    /// The tile under a point of the content (the mouse's snapped place, FUN_1004657d).
    pub fn tile_at(&self, p: [f32; 2]) -> (i32, i32) {
        let w = self.world(p);
        ((w[0] / TILE).floor() as i32, (w[1] / TILE).floor() as i32)
    }

    /// A tile's top left corner in the content; it is 8 pixels a side.
    pub fn tile_pixel(&self, tile: (i32, i32)) -> [f32; 2] {
        [(tile.0 as f32 * TILE - self.left) * SCALE, (self.top - (tile.1 + 1) as f32 * TILE) * SCALE]
    }

    /// A marker's middle in the content (FUN_100467e8: a pixel's worth of world higher than the tiles).
    pub fn marker_pixel(&self, world: [f32; 2]) -> [f32; 2] {
        [(world[0] - self.left) * SCALE, (self.top - 1.0 - world[1]) * SCALE]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape(kind: i32, tiles: (i32, i32)) -> Shape {
        Shape { kind, tiles, mask: vec![0xff; ((tiles.0 * tiles.1 + 7) / 8) as usize], doors: vec![[2.0, 0.0, 6.0]], guards: Vec::new() }
    }

    fn standing(kind: i32, tiles: (i32, i32), tile: (i32, i32)) -> Node {
        Node { item: None, template: 100 + kind as u32, shape: shape(kind, tiles), tile, rotation: 0, demolishing: false, valid: true }
    }

    /// Two zones side by side, all their tiles set but the first's column x 9.
    fn zones() -> Zones {
        let mut first = [0xffu8; 13];
        for z in 0..10 {
            let bit = z * 10 + 9;
            first[bit / 8] &= !(1 << (bit % 8));
        }
        Zones(vec![((210, 280), first), ((220, 280), [0xff; 13])])
    }

    fn map() -> Map {
        let mut m = Map { mode: Mode::Build, zones: zones(), tiles: (600, 600), ..Default::default() };
        m.buildables = vec![
            Buildable { slot: 70, template: 1, side: Some(1), shape: shape(1, (3, 2)) },
            Buildable { slot: 71, template: 9, side: None, shape: shape(9, (2, 2)) },
            Buildable { slot: 72, template: 9, side: Some(2), shape: shape(9, (2, 2)) },
            Buildable { slot: 73, template: 50, side: Some(1_234_567_890), shape: shape(0, (1, 4)) },
            Buildable { slot: 74, template: 2, side: Some(1), shape: shape(1, (3, 2)) },
            Buildable { slot: 75, template: 9, side: None, shape: shape(9, (2, 2)) },
        ];
        m
    }

    #[test]
    fn a_house_is_built_in_the_zones_only() {
        let z = zones();
        assert_eq!(z.tiles().count(), 190);
        assert_eq!(z.bounds(), Some((210, 280, 230, 290)));
        let mut n = Node { item: Some(1), ..standing(0, (2, 3), (211, 281)) };
        assert!(z.can_build(&n));
        // (over the clear column; out of every zone's rectangle; turned, a tile wider)
        n.tile = (208, 281);
        assert!(!z.can_build(&n));
        n.tile = (218, 281);
        assert!(!z.can_build(&n));
        n.tile = (229, 281);
        assert!(!z.can_build(&n));
        n.tile = (228, 287);
        assert!(z.can_build(&n));
        n.rotation = 90;
        assert_eq!(n.shape.size(90), (3, 2));
        assert!(!z.can_build(&n));
        // (a tile of the rectangle the house does not cover may lie on a clear zone tile, but not outside the zones)
        let mut l = Node { item: Some(1), ..standing(0, (2, 1), (218, 281)) };
        l.shape.mask = vec![0b01];
        assert!(z.can_build(&l));
        l.tile = (229, 281);
        assert!(!z.can_build(&l));
    }

    #[test]
    fn drops_are_held_to_the_limits_and_the_side() {
        let mut m = map();
        assert_eq!(m.drop_item(5, (211, 281), Some(1)), Err(Refusal::NotAHouse));
        assert_eq!(m.drop_item(72, (211, 281), Some(1)), Err(Refusal::Side(2)));
        // (no headquarters yet: a house alone cannot be constructed)
        assert_eq!(m.drop_item(73, (225, 281), Some(1)), Ok(()));
        assert!(m.nodes[0].valid && !m.can_construct);
        assert_eq!(m.drop_item(70, (211, 281), Some(1)), Ok(()));
        assert!(m.can_construct);
        assert_eq!(m.drop_item(74, (215, 285), Some(1)), Err(Refusal::Limit("AlreadyHaveHQhere")));
        // (an item that waits on the map is not taken a second time)
        assert_eq!((m.drop_item(70, (215, 285), Some(1)), m.nodes.len()), (Ok(()), 2));
        // (a unique house once a template; an item with no side is anyone's)
        assert_eq!(m.drop_item(71, (221, 286), None), Ok(()));
        assert_eq!(m.drop_item(75, (221, 283), None), Err(Refusal::Limit("AlreadyHaveUniqueHouse")));
        assert_eq!(m.locked(), [73, 70, 71]);
        // (two that share a tile are both not valid, and nothing can be constructed)
        m.select(Some(2));
        m.move_selected((225, 283));
        assert!(!m.nodes[0].valid && !m.nodes[2].valid && m.nodes[1].valid && !m.can_construct);
        m.move_selected((221, 286));
        assert!(m.can_construct);
        // (turned, the headquarters is 2 by 3 from the same tile; four turns bring it round)
        m.select(Some(1));
        m.rotate();
        assert_eq!((m.nodes[1].rotation, m.nodes[1].shape.size(90)), (90, (2, 3)));
        for _ in 0..3 {
            m.rotate();
        }
        assert_eq!(m.nodes[1].rotation, 0);
        // (construct: the headquarters first, then the others as dropped; they leave the map)
        let builds = m.construct();
        assert_eq!(builds.iter().map(|b| (b.slot, b.tile, b.rotation)).collect::<Vec<_>>(), [(70, (211, 281), 0), (73, (225, 281), 0), (71, (221, 286), 0)]);
        assert!(m.nodes.is_empty() && m.selected.is_none());
        assert!(m.construct().is_empty() || !m.nodes.iter().any(Node::pending));
    }

    #[test]
    fn standing_houses_are_selected_demolished_and_kept_through_the_servers_word() {
        let mut m = map();
        m.set(Data { houses: vec![standing(1, (3, 2), (211, 281)), Node { demolishing: true, ..standing(0, (2, 2), (221, 281)) }], buildables: m.buildables.clone(), ..Default::default() });
        assert_eq!((m.hit((213, 282)), m.hit((214, 282)), m.hit((222, 282))), (Some(0), None, Some(1)));
        // (one whose demolishing began is not selected)
        m.select(Some(0));
        m.select(Some(1));
        assert_eq!(m.selected, Some(0));
        assert!(m.standing_selected() && !m.pending_selected());
        // (a pending one over a standing one: neither valid)
        m.drop_item(73, (212, 280), Some(1)).unwrap();
        assert!(!m.nodes[2].valid && !m.can_construct);
        // (the server's list again: the pending node and the selection stay; an item gone from the backpack takes its
        // node with it)
        let houses = vec![standing(1, (3, 2), (211, 281))];
        m.set(Data { houses: houses.clone(), buildables: m.buildables.clone(), ..Default::default() });
        assert_eq!((m.nodes.len(), m.selected, m.nodes[1].item), (2, Some(0), Some(73)));
        assert_eq!(m.demolish(), Some((211, 281)));
        assert_eq!((m.selected, m.demolish()), (None, None));
        m.set(Data { houses, buildables: Vec::new(), ..Default::default() });
        assert_eq!(m.nodes.len(), 1);
        // (the zones stay from the opening's data)
        assert_eq!(m.zones.0.len(), 2);
    }

    #[test]
    fn the_views_geometry_is_the_originals_for_6010() {
        // (the zones' union in 6010: tiles x 210..290, z 280..390; the playfield 600 tiles a side)
        let z = Zones(vec![((210, 280), [0; 13]), ((280, 380), [0; 13])]);
        let g = Geometry::new(&z, (600, 600)).unwrap();
        assert_eq!((g.left, g.top, g.size), (790.0, 1611.0, [842.0, 1082.0]));
        // (tile (X, Z) is at x (4X - 790) * 2, y (4 * (599 - Z) - 789) * 2)
        assert_eq!(g.tile_pixel((217, 306)), [156.0, 766.0]);
        assert_eq!(g.tile_at([156.0, 767.0]), (217, 306));
        assert_eq!(g.tile_at([163.9, 773.9]), (217, 306));
        assert_eq!(g.tile_at([164.0, 774.5]), (218, 305));
        // (the top pixel row of a tile is the tile above's: z comes out at 4Z + 4 exactly)
        assert_eq!(g.tile_at([156.0, 766.0]), (217, 307));
        assert_eq!(g.marker_pixel([868.0, 1224.0]), [156.0, 772.0]);
        let n = standing(0, (10, 10), (217, 306));
        assert_eq!(n.place([2.0, 0.0, 6.0]), [870.0, 1230.0]);
        assert_eq!(Geometry::new(&Zones::default(), (600, 600)), None);
    }
}
