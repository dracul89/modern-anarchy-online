//! Gamecode's playfields on the planet (FUN_100bbd21's table, the playfields' world offsets as doubles: x, 0, z; read
//! through FUN_100bbafa) and the offset from Omni-1's headquarters (FUN_100b8e6f, for Notum Wars players: the eye's
//! place on the planet less playfield 700's point (605, 0, 419)), which the scripts' horizon scrolls by
//! (`GAME.OffsetFromHQ_X/Z`, the frame block's +0x11c and +0x120).

/// FUN_100bbd21's table: playfield, x, z.
const OFFSETS: [(u32, f64, f64); 47] = [
    (505, 35214.0, 44566.0),
    (540, 39621.0, 40670.0),
    (545, 39122.0, 40646.0),
    (550, 37085.0, 39823.0),
    (551, 37078.0, 41004.0),
    (560, 41975.0, 45030.0),
    (565, 41959.0, 42152.0),
    (566, 42862.0, 42389.0),
    (567, 41950.0, 42166.0),
    (570, 45486.0, 45364.0),
    (585, 39721.0, 39418.0),
    (586, 39743.0, 40342.0),
    (590, 42836.0, 36053.0),
    (595, 46219.0, 35172.0),
    (600, 41975.0, 39148.0),
    (605, 46248.0, 32165.0),
    (610, 42839.0, 33283.0),
    (615, 46680.0, 24729.0),
    (620, 46689.0, 27892.0),
    (625, 41891.0, 30614.0),
    (630, 43499.0, 27918.0),
    (635, 40889.0, 33041.0),
    (640, 47921.0, 39716.0),
    (646, 46178.0, 39071.0),
    (647, 46175.0, 39073.0),
    (650, 40866.0, 36044.0),
    (655, 37182.0, 31311.0),
    (665, 31002.0, 29384.0),
    (670, 37169.0, 27031.0),
    (685, 34495.0, 29814.0),
    (687, 34518.0, 29814.0),
    (695, 39962.0, 27028.0),
    (696, 41950.0, 27705.0),
    (700, 43204.0, 27102.0),
    (705, 43756.0, 26526.0),
    (710, 43308.0, 27595.0),
    (716, 44526.0, 24729.0),
    (717, 44526.0, 24729.0),
    (730, 34049.0, 30363.0),
    (735, 33917.0, 30370.0),
    (740, 33837.0, 30358.0),
    (760, 38486.0, 33943.0),
    (790, 38062.0, 36352.0),
    (791, 38081.0, 37660.0),
    (795, 33860.0, 39227.0),
    (800, 37107.0, 38360.0),
    (4526, 43204.0, 27102.0),
];

/// A playfield's offset on the planet (none for one not in the table: the place is taken as it is).
pub fn offset(playfield: u32) -> Option<[f64; 2]> {
    OFFSETS.iter().find(|o| o.0 == playfield).map(|o| [o.1, o.2])
}

/// A place (the server's frame, Y up) on the planet: FUN_100bbafa.
pub fn on_planet(playfield: u32, at: [f32; 3]) -> [f32; 3] {
    match offset(playfield) {
        Some([x, z]) => [at[0] + x as f32, at[1], at[2] + z as f32],
        None => at,
    }
}

/// FUN_100b8e6f: the eye's offset on the planet from the headquarters' point.
pub fn offset_from_hq(playfield: u32, eye: [f32; 3]) -> [f32; 2] {
    let me = on_planet(playfield, eye);
    let hq = on_planet(700, [605.0, 0.0, 419.0]);
    [me[0] - hq[0], me[2] - hq[2]]
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_hq_offset() {
        assert_eq!(super::offset_from_hq(700, [605.0, 0.0, 419.0]), [0.0, 0.0]);
        assert_eq!(super::offset(566), Some([42862.0, 42389.0]));
        assert_eq!(super::offset_from_hq(566, [0.0; 3]), [42862.0 - 43809.0, 42389.0 - 27521.0]);
    }
}
