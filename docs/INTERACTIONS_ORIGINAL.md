# Every click and interface interaction: the original client against this one

Written 2026-11-25, after a play session in which Shift + click on an inventory item took the item up where it
should show its info. The cause was that the keyboard's Shift, Ctrl and Alt never reached the interface at all
(`Ui::held_keys`, fixed 2026-11-24); the review that followed is this document. The user's word: "I'm not looking
to deviate from the original" on clicks and interface interactions.

## How to read it

One table per window, one row per interaction: what the original does and where that was read (an address in
GUI.dll unless marked `GC` Gamecode.dll, `IF` Interfaces.dll, `exe` anarchyonline.exe, or `n3.c` for N3.dll), what
this client does and where, and the verdict:

- `MATCH`: the same.
- `DIFFER`: both do something, not the same thing.
- `MISSING`: the original has it, the client does not.
- `ADDITION`: the client has it, the original does not.
- `ASKED`: a departure the user asked for or decided (the row names the note). Listed and left. Since the two
  answers of 2026-11-25 these are the five extras the user keeps.
- `UNSURE`: one side could not be read to the end. The row says what is not known.
- `FIXED 2026-11-25`: was a difference, changed in the client on that day (the row says what it was). The rows
  changed after the user's answer to the questions below, the same day, are marked the same way.

Marks on the original's statements: **[V addr]** read in the code at that address; **[NF]** looked for and not
found; **[NR]** not read.

**How it was made, and how far to trust it.** Ten readers (subagents) each compared one group of windows, reading
the decompiles in `AnarchyReimagined/exports_gui` and `exports_gamecode` and this client's source as it stood on
2026-11-25 (with the uncommitted work of 2026-11-24: title-row tabs, the chat's font, the item use check). Their
tables are below as they wrote them. Checked again by hand in the disassembly before anything was changed: the two
mouse schemes of the item lists (below), `WindowController_c::HandleMouseUp`, `N3Msg_UseItem`'s own-item test,
the info view's URL slot, the NCU's right click and `N3Msg_RemoveBuff`, `ButtonBase_c`'s press, the shortcut
bar's strip, the wear page's two drop handlers and the order they run in. Every `FIXED` row rests on one of those
readings. A `MATCH` row was read by its reader on both sides but not read a second time, and a few rest on
`docs/INVENTORY_ORIGINAL.md` rather than on a fresh reading (the row then says "doc"). A second round the same day, after the user's answer to the questions
at the end, sent four more readers after what was not read to the end (the buttons, how a window's menu is
opened and how windows are docked, the hover tips, the chat's keys and the split of a stack); what they found is in
the rules below and in "Settled by the user's answer". One of them overturned a rule of the first round (the
buttons), which was then checked by hand in `ButtonBase_c`'s four functions. **Nothing here was tried in
the live game**, and the client's side of the Shift and Ctrl rows could never have been tried with a real keyboard
before 2026-11-24: the scripted live tests hand their held keys to the interface themselves (`sclick@`, `cclick@`,
`MAO_CLICK_KEYS`), so every earlier "confirmed live" on a Shift or Ctrl click proves nothing about a real
keyboard. What has to be tried by hand is in `docs/LIVE_TEST_LIST.md`, "Clicks and held keys (2026-11-25)".

## The counts

| Windows | Rows | Match | Differ | Missing | Addition | Asked | Unsure | Fixed |
|---|---|---|---|---|---|---|---|---|
| Inventory, wear window, Inspect window | 76 | 37 | 6 | 4 | 0 | 0 | 13 | 16 |
| Backpacks, chests and corpses, the bank, reclaim | 93 | 49 | 9 | 19 | 0 | 1 | 8 | 7 |
| Shop, player trade, NPC chat and trade | 55 | 33 | 8 | 3 | 0 | 0 | 9 | 2 |
| Shortcut bars and the Actions window | 48 | 25 | 0 | 5 | 0 | 1 | 8 | 9 |
| Nano programs, NCU, perks | 56 | 28 | 0 | 3 | 0 | 0 | 7 | 18 |
| Chat windows, tabs, tell windows | 65 | 24 | 14 | 10 | 1 | 1 | 10 | 5 |
| Info window, missions, mission booth, reference board | 68 | 43 | 4 | 6 | 0 | 1 | 8 | 6 |
| Planet map, playfield map, team, friends, target bars | 66 | 35 | 6 | 8 | 0 | 1 | 10 | 6 |
| The window frame, docking, rollup, Escape, menus | 66 | 42 | 11 | 0 | 0 | 0 | 12 | 1 |
| Clicks in the world | 41 | 27 | 6 | 1 | 1 | 0 | 5 | 1 |
| **All** | **634** | **343** | **64** | **59** | **2** | **5** | **90** | **71** |

(A row with two verdicts is counted under the first. `Fixed` rows were `Differ`, `Missing` or `Unsure` before.)

## Rules that hold in every window

- **The held keys.** `WindowController_c::GetQualifiers` (0x10156df6): bits 1|2 Shift, 4|8 Ctrl, 0x10|0x20 Alt.
  In all the handlers read, only Shift and Ctrl are ever tested by a list or a window; Alt is tested in the chat's
  input line (Alt+Enter) and, by an earlier note that was not read again, in the target NCU windows (Alt + double
  click), nowhere else that was read.
- **The item lists** (`ItemListViewBase_c`, constructor FUN_100423fc) have two mouse schemes.
  The *old* one (inventory, wear pages, bags, bank, chests, corpses, the NCU, the trade lists, the mission
  booth's offers): the left button coming up on the cell it went down on raises the pick-up signal, with Shift the
  qualifier signal (the info) instead [V 0x100414e4, 0x10041578]; a list with the selection flags `0xc0` turns the
  item's selection round instead, and one with flag 2 raises the click signal [V 0x1004154c, 0x1004156a]. The right
  button coming up raises the click signal, whatever key is held, unless the list has flag `0x400` (a corpse)
  [V 0x10041529]. The right button going down starts `MouseButtonHoldDelay` (0.3 s), at whose end the menu signal
  is raised and the click is over [V 0x10041c24, FUN_100420fd]; a list nobody listens to for clicks raises the menu
  at once [V 0x10041c04]. Ctrl + left button down on a stack of more than one starts a split, whatever Shift
  [V FUN_10041926].
  The *new* one (lists that called FUN_10041248 while `UseNewMouseFunc` is on, as shipped: nano programs, actions,
  shortcut bars, the mission journal, the reference board): the left button going down starts the same delay, at
  whose end the item is taken up, moved or not [V 0x10041b90, FUN_1004213e]; coming up before that it is the click,
  with Shift the qualifier signal [V 0x100414dd]; the right button coming up is the menu signal [V 0x100414cf].
- **Something in hand.** A thing taken up is a `DragObject_c` kept by the window controller until a button comes
  up. A list handed the drag object on its mouse-up raises only its drop signal, with the button: no click, no
  info, whatever key is held [V FUN_1004141b, last branch]. A button coming up over no window raises the drag's own
  drop signal and then cancels it [V 0x10157f13]. Escape cancels it: `BeginDrag` connects every drag object's
  `Cancel` to the Escape signal [V 0x10157842].
- **Buttons: a plain one acts on the release over it, a toggle on the press.** (Corrected 2026-11-25: the first
  round read `ButtonBase_c::MouseDown`'s first branch as the rule.) `ButtonBase_c::Initialize` (0x10129bc7) sets the
  button's `ModifyWhenClicked` flag [V 0x10129bd0]. With it, `MouseDown` (0x101298db) only gives a plain button its
  pressed look; moving off it takes the look away and moving back gives it again (`MouseMove`, 0x10129a37); and
  `MouseUp` (0x101294c9), while it looks pressed, raises the click (`SetValue(false, true)`, 0x101295ab), however
  long the button was held. A **toggle** (a check box's box, a radio button, the frame's menu icon and pin, a rollup
  page's three header buttons, a toolbar button) flips in `MouseDown` and its release does nothing; a button told
  `SetModifyWhenClicked(false)` (a perk line's, a radio button's) raises its click in `MouseDown` too. Left button
  only; no key presses a button (`KeyDown` is the plain view's). Also on the press: a tab (any button), a list's row
  and a folder, a text link, a scroll bar's arrows (which repeat after 0.5 s, then every 80 ms) and track, a drop-down
  opening. On the release: a pop-up menu's entry (and only when the press was in the menu too), a list header's sort.
- **Asking for the info page already shown puts it away.** `InfoViewModule_c::ShowURL` (0x100efa86) hands the
  view's slot a flag; with it FUN_100ee973 closes the window when the URL is the one last asked for
  [V 0x100ee9b6..0x100ee9d4]. Every Shift + click, "Look at" entry and mission click goes through `ShowURL`.
- **A window's own menu** (List Mode, Auto Arrange, Labels On Top and the like) is the view's menu builder (its
  virtual at +0xf8), and there are two ways to it: the frame's menu button of a free window (`DockWindow_c`'s slot
  0x1003c2b8 hands the opening on to the shown view; the frame then adds "Autofade Window" and "Close Window",
  FUN_1015b34f), and the menu button of a rollup page's header (FUN_10048c5c: the view's entries and "CloseView").
  A right press beside the items of a list does nothing (FUN_10041aa4 returns when no item is under the pointer).
  Only the chat's text opens its window's menu by the right button (`ChatView_c::MouseDown`, 0x1008fa92).
- **Docking is by the mouse alone**: a view's tab carried from its window (FUN_1003c428), or a rollup page taken up
  by the right button (FUN_100493f2), put down on another window's tabs, on the column (FUN_10049121) or nowhere (a
  window of its own, FUN_1003b263). No button and no menu entry docks. Any `DockableView_c` can be a page.
- **A tip** comes when the pointer has rested 500 ms on a thing (`UpdateToolTip` 0x10157adc: a constant), is placed
  once (`MoveToMouse` 0x101550de: 16 right and 32 under the pointer, flipped past the screen's middle) and goes when
  a button goes down or comes up. The item lists' tip (FUN_10042193) is the name in its rarity's colour over the
  item's page built for a tip; the NCU list has its own (FUN_10045b79).

## Fixed on 2026-11-25

Each has a unit test where one could be written, a line in `CHANGELOG.md` under Unreleased, and a note in
`docs/CLIENT_UI.md` ("Clicks brought in line with the original (2026-11-25)"). None was tried live.

1. **Item windows** (`invwindows.rs`, `loot.rs`): Ctrl + click on a single item takes it up (it did nothing); with
   something in hand a click is the letting go alone, Shift or not (Shift also opened the info page); Ctrl + Shift
   on a stack is the split; the right button beside the items with something in hand puts it back (it did nothing,
   or opened the window's menu); a wear place that refuses the item leaves the hand empty (the item stayed in
   hand, a rule taken from half of the original's two handlers); the Inspect window shows an item's info by
   Shift + click; a bag's item in hand right-clicked on another item of that bag is used on it (the click acted on
   the item clicked and the other stayed in hand).
2. **Escape puts back what is in hand** (an item, a split of a stack, a carried tab or rollup page, a carried chat
   tab). The client had this, but tested the key through the interface library, which a real keyboard's Escape
   never reaches: `Ui::close_with_escape`.
3. **The info window** closes when it is asked from outside for the page it was last asked for
   (`gui/infoview.rs` `asked`): a second Shift + click on an item, a second click on a mission.
4. **The NCU window** (`ncu.rs`): a plain click on a running nano does nothing and Shift + click shows its info
   (any click did); a right click takes the nano off, with the original's line, and a hostile one stays with the
   other line (the right click opened a menu, and its "Cancel this nano" sent the cancel for any nano without a
   word); the right button held 0.3 s opens the nano's menu, "Look at". The target NCU windows show the info by
   Shift + click only.
5. **Nano programs, actions, shortcut bars**: Shift + click shows the info and does not cast, perform or use
   (`hotbar::info_url`); a tile is taken up by the left button held 0.3 s, moved or not, and a quicker press is a
   click however the pointer moved (`multilist::hold_to_pick`, `shortcutbar::cell_mouse`; a press that moved used
   to drag at once).
6. **Shortcut bars** (`ui.rs` `shortcut_layer`): the right button on a bar's control strip turns it upright or
   flat (a double click did, which the original does not have); a locked bar can still be moved by its strip; the
   shortcut a drop pushes out of a cell goes into hand on a locked bar too (it was thrown away).
7. **The mission booth** (`mission.rs` `offer_click`): a click on the selected offer deselects it; Shift + click
   shows the offer's page.

## Settled by the user's answer (2026-11-25)

The ten questions at the end were put to the user, who answered: "Yes, as the original" (and earlier the same
evening: "I'm not looking to deviate from the original on that"). What that changed, each with a unit test where one
could be written and a line in `CHANGELOG.md`; none tried live:

1. **Splitting a stack** (`gear.rs` `count_begin`, `count_step`, `count_frame`): Ctrl + the left button down on a
   stack starts the count at one; the mouse's way sideways changes it, a pixel a unit, between one and one fewer
   than the stack; the pointer is put back after every move and the count stands at it in yellow; the release has
   that many in hand. The number box is gone (INV-008).
2. **"Cancel this nano"** is gone from the NCU's held-button menu (NAN-060): the right click cancels.
3. **A right click beside a window's items does nothing** (inventory, wear, Inspect, bags and the bank, the nano
   programs, the NCU, the shop, and found on the way: Stats, the target's NCU windows, the playfield map). Each of
   those menus opens from the frame's menu button (or the rollup page's); the NCU's, the Stats window's, the target
   NCU windows' and the playfield map's did not before and do now.
4. **The dock button** in the title is gone (WIN-180): Faction and Stats are dock views like the others
   (`docks::VIEWS`: `faction_window`, `stat_window`), carried by their tab or their page; the pet window is no
   dockable view in the original and no page here any more.
5. **The chat menu's "Move to a window" entries** are gone (CHT-001): a tab is carried by the mouse.
6. **The tips** are the original's (NAN-059): a nano program's is its page built for a tip (`iteminfo::nano_tip`),
   a running nano's the NCU list's own (`ncu::tip`), a bag's item's the inventory's, a shortcut's the short one
   (`hotbar::tip`); a tip's modifier list has no cast lines and no criteria (`spells::convert_in_tip`); all come by
   the window controller's rule (`gui/tooltip.rs` `rest`, `place`): after half a second, placed once, gone at a
   button.
7. **Alt+Enter** starts typing (not typing) and takes the typing to the next chat window, with Shift the one before
   (typing; `chatwindow::next_typing`). The window mode keeps F11, the client's options window and the launcher.
8. **Tab on an empty chat line, or Ctrl+Tab**, opens the list of the groups the window can talk to
   (`open_channel_list`); Tab no longer leaves the line.
9. **The middle button** does nothing in the world (CAM-001's reset keeps its key).
10. **The planet map** (`planetwindow.rs`): the wheel scrolls up and down and does not zoom; a double left click
    steps a level closer and a double right click a level further, the point clicked brought to the middle; a grab
    moves the map 1.5 times the mouse's way; the zoom buttons bring the character to the middle and are dead at the
    ends; the pointer's coordinates are not written on the map.
11. **Enter in a player trade's credits field** sends nothing (it ends the typing); the credits go a second after
    the last change, or by Accept.
12. **Buttons** (`chrome::on_press`, `chrome::on_release_inside`): the toggles act on the press (check boxes and
    radio buttons of both kinds of windows, the frame's menu icon and pin, a rollup page's three header buttons, a
    perk line's button), and so does a text link; a plain button is clicked by the release over it however long it
    was held and wherever the pointer went meanwhile (the interface library's own click wanted the release within
    0.8 s and 6 points of the press), and looks pressed only while the pointer is on it.

Not carried out, and then **kept on the user's word**. Each of these five is a feature the user asked for or chose
by name, was not among the five named in the question the user answered, and is off or out of the way until asked
for, so none was removed and the user was asked about them by name the same day: "Keep the five extras you asked for
earlier, which the original does not have?" The answer: **"Keep them"**. Their rows are `ASKED`:

- **The smart shortcut bars** (`smart_hotbar`) and **the round corner map** (`own_map`): both settings, both off by
  default, so the bars and the map are the original's unless the player turns them on.
- **Auto-loot** (`auto_loot`, off by default) and **the mission booth's helper** (one of the user's own tools of
  2026-11-12; it asks nothing until its lists are filled).
- **The chat's command list** while a command is typed (asked for 2026-11-21). The original completes by Tab into
  a pop-up of the candidates (FUN_100a44e8, FUN_1008f3d8); here the list stands over the line as one types and Tab
  takes the highlighted one.

## Left, with the reason (biggest for a player first)

1. **One hand for every container.** An item taken up in a bag or the bank is known to that window only
   (`LootWindow::dragging`), the inventory's to `gear::held`. So a bag or bank item in hand cannot be put into
   another bag, the bank or a wear place, nor used on an item of another window (containers A28, A29, C8; A9 across
   windows), and Ctrl + right click adds to a trade from the inventory only (containers A23, B12, C18; trade 15).
   Reason: medium-large (every window that takes a drop asks one of the two), and the messages for an item named
   by a bag's identity going to another bag or a wear place were never captured.
2. **Wearing from a bag by a right click** (containers A8). The original's `N3Msg_UseItem` counts an item of one's
   own backpack as one's own (`GC` FUN_10048ca7) and wears it from there; here it is taken into the inventory.
   Reason: the move message from a bag to a wear place was never captured.
3. **Closing a bag tells the server** (containers A33, A34, B15, D7; INV-038, INV-062): the window's close and a
   second use send `N3Msg_UseItem(container)`. Reason: the bytes are not known; needs a capture.
4. **What is left of the buttons.** The interface library's own widgets in this client's own windows (the
   launcher's page, the client's options window, the damage meter, the item finder, the help assistant: check
   boxes, choice labels and buttons drawn without the skin) keep the library's rules: a toggle there flips on the
   release, and a button's click wants the release within 0.8 s and 6 points. The scroll bars' arrows do not repeat
   while held (the original: after 0.5 s, then every 80 ms). The right button's link menu in a text comes on the
   release (the original: on the press). A window's own menu, opened from the frame's button, ends with "Close
   Window" alone (the original puts "Autofade Window" before it). Reason: small each, not reached.
5. **Selecting and copying text** in the chat and the info pages, with the right button's Select All / Copy menu
   (chat X14 to X16, M9; info I3). Reason: large (the chat's lines are drawn one by one without a text cursor).
6. **Chat keys**: Page Up / Page Down scroll the chat (X21), Shift+R starts a reply (I4), Shift+C opens the chat
   configuration (I5), Escape in the input line clears what was typed (I9), Up/Down past the newest line brings
   back what was being typed (I10); Enter and Alt+Enter start typing in the chat window itself, where the original
   picks the default window, else the one last typed in (FUN_1009401d); Tab with text completes into a pop-up of
   the candidates in the original (see "Settled", not carried out); Space on a half-typed command completes it.
   Reason: not reached; small to medium each.
7. **The chat's menus**: the title's menu button gives only Autofade and Close where the original's gives the whole
   chat menu (M10); Delete Window asks first and refuses the default window (M7); New Window asks for a name
   (M8); "Private Groups" as a submenu of its own (M3); a tell window's name menu has only Ignore, and a name click
   there does nothing (X6, X7). Reason: not reached; small each.
8. **Putting a carried tab down** (chat T4, T5; frame 30, 32, 36, 37): the original lets go on any button's press
   over a tab, a tab strip or the rollup column, and on any button's release over no window (a new window, its
   corner at the pointer); over a window's body the tab stays carried. Here only a left click lets go, anywhere.
   Reason: not reached; small to medium, wants looking at with the mouse.
9. **The frame** (frame 1, 3, 5, 7, 26, 28, 42): a press on a title-row tab also moves the window; any button
   moves and sizes; the grab areas are 9x9 corners and 4-pixel edges inside the frame; a window can be dragged
   partly off the screen (here the interface library holds every window inside); minimum sizes; no fading while a
   dialog is open; a rollup page's own smallest and largest height. Reason: these sit in the interface library's
   own window handling; medium each, and each wants a picture and a hand on the mouse.
10. **The reference board** (info K1 to K3, K8, K10): a press takes a reference up, Shift + click shows its page, a
    plain click does nothing. Here any click shows the page and nothing can be taken up. Reason: medium.
11. **The world** (world 4, 12, 13, 15, 20, 28, 31g, 33): the default action on a character has more steps than
    read here (needs a capture of right clicks on an NPC, a monster, a player and a corpse); a double click on an
    item that can be picked up picks it up (`N3Msg_GetItem`, a message never captured from a click) where a use is
    sent; a button let go over a window after a press in the world clicks the world here; an item let go over a
    floor indoors does nothing (the drop point is looked for on terrain only); a thing whose `Can` has 0x100000 is
    selected by hovering; the double-click time is Windows' own (500 ms and 2 pixels are fixed here); the mouse
    look does not end when the window loses the focus. Reason: each needs the game loop's side read too, and two
    need captures.
12. **Maps** (maps P1, P9 to P11, F8): the pointer is not held still while a map is grabbed; the planet map's own
    menu ("Center on player", "Center on mission", "Select Map", "Show buttons") is not on its frame's button, and
    its maps are chosen by a box under the buttons where the original opens an info page; the playfield map's X and
    Z fields with Add and Remove Waypoint. How far one notch of the wheel scrolls the planet map is taken as 10
    (the step is 10 a unit; what a notch is in units was not read). Reason: not reached.
13. **Team, friends, target bars** (maps T13, T14, R3, R6, R11, V3, V4, V7, V8): the keys that select team member
    1 to 5; a member not in the playfield in grey; a friend folder opened by any button; one chat window shown or
    hidden per row; the target-of-target button and the bar's two other buttons; the raid bars. Reason: not
    reached (the raid bars are not ported at all).
14. **Trade** (trade 6, 17, 22, 28, P1, P3, P8, P9, N5, N12): the shop's own cash field shows what is sold less what
    is bought, red when below zero; Accept goes grey once pressed; the refusals before an item goes in; the
    partner's items in a player trade (needs a capture, INV-055); Ctrl + click on a stack's row does nothing;
    `ShowNPCQuestions`. Reason: not reached, one needs a capture.
15. **Small things in the item windows**: the pointer's shapes over an item (use, info with Shift: items I33, W18,
    N14); the tip's name in the rarity's colour in container windows (A37); Lock in the Inspect window's item menu
    (N5); the confirm for an item whose `Can` has 0x10 (I12); joining and splitting stacks in the bank (C4, C5); a
    split let go on a row in list mode (A7, I38); the count's `%uM` (I39); the lock of the item a wear drop lands
    on (W23); the reclaim window (D1 to D7); the menu of the item the right button went down on when the pointer
    has left it (I16). Reason: not reached, or (reclaim) no captured message.
16. **Nanos, actions, bars**: an inventory item let go on a nano program casts the program on it (nanos N16); the
    Actions window's tiles placed by hand and kept (bars A7, A8; nanos P18); a newly learned nano put on the
    primary bar (bars 30); the keys of each bar's slots and the bars' window key (bars 28, 29); a nano taken off
    is removed from the NCU list at once where the original waits for the server (nanos C5). Reason: not reached;
    the Actions grid is large.
17. **Info pages**: `charref://` pages (info I14, chat X10), the tip on a chat-command link (I18), the "please
    wait" page for a character (I15), the booth's advanced-options toggle (B14). Reason: not reached.

## The questions, and the user's answer

Put to the user on 2026-11-25 as "Should the review's ten open questions all be settled as the original does it?"
(with five of them named: 3, 4, 6, 9 and 10). The answer: **"Yes, as the original"**. A second question the same
day, about the five extras that answer did not name (the smart shortcut bars, the round map, auto-loot, the mission
booth's helper, the chat's command list while typing): **"Keep them"**. The user was also told that a plain button
of the original acts on the release, so that buttons were not moved to the press; no objection. What was done is in
"Settled by the user's answer" above; in short, by question:

1. **The split box** (INV-008): ANSWERED, done. The count is by the mouse, as FUN_10041926 / FUN_10041ca4 /
   FUN_100419d5 have it.
2. **The shortcut bars' own mode** (WIN-001) and **the round corner map** (WIN-151): KEPT ("Keep them"). Both are
   settings that are off by default: the client does the original's unless the player asks otherwise.
3. **"Cancel this nano"** (NAN-060): ANSWERED, done. Gone.
4. **A window's menu by a right click beside its items**: ANSWERED, done. Gone in every window that had it; the
   frame's menu button (or the rollup page's) opens each.
5. **The dock button** (WIN-180): done, gone. **The chat menu's "Move to a window" entries** (CHT-001): done, gone.
   **The tips' own layout** (NAN-059): done, the original's. **The chat's command list on Tab** (asked 2026-11-21):
   KEPT ("Keep them"); Tab on an empty line and Ctrl+Tab are the original's now.
6. **Alt+Enter**: ANSWERED, done. It starts typing, and while typing goes to the next chat window.
7. **The middle button**: ANSWERED, done. Nothing.
8. **Additions**: the planet map's coordinates under the pointer and Enter in a player trade's credits field: done,
   gone. **Auto-loot** (INV-079, a setting, off by default) and **the mission booth's helper** (NPC-039, WIN-029,
   one of the user's own tools): KEPT ("Keep them").
9. **The planet map's wheel**: ANSWERED, done. It scrolls; the double clicks and the two buttons zoom.
10. **Buttons acting on the press**: ANSWERED, and the question was wrong. A plain button of the original acts on
    the release over it; only toggles and a few others act on the press (see the rules). Done as the original has
    it: toggles to the press, plain buttons to the original's release.

## Not read, not tried

- Nothing was run against the live game, and no window was clicked by hand: the changes are from the code, their
  unit tests and offscreen pictures (the inventory, the NCU window and the shortcut bars were rendered after the
  change; Ctrl + click and a plain click-click were run through the inventory's picture by `MAO_CLICK`).
- The offscreen picture does not show the info window, so the info window's closing on a second asking rests on
  its unit test.
- Of the second round nothing was tried either. The split's pointer being put back rests on the window system
  moving the pointer when asked (Windows does; not tried). The tips were not looked at in a picture (they come only
  to a resting pointer). The Stats and Faction windows as dock views (a tab in the title row, carried to the column)
  and the planet map's double clicks were not tried by hand.
- Not read in the second round: who listens to a button's hold signal (+0x158, a second's hold begins a drag
  somewhere); the scroll bar timers' unit; `PopupMenu_c`'s Shift-activate flag; the tips of the Actions window's
  tiles (the list's own, but what an action's page holds needs a capture: left as it was), of the perks and of
  single item views (FUN_10009529, FUN_10106bca); what one notch of the wheel is in the scroll view's units; the
  order of a view's and the frame's entries where the view appends (Stats, the maps, the pet window).
- Not read at all, by any reader: the skills window, the options window, mail, the organization and city windows,
  the pet window, the tradeskill window, the vehicles and tower windows, the help windows, the login and
  character windows, the perk training tree beyond its buttons, the own health bar's clicks. They are not in the
  tables.
- Each window's section ends with its reader's own "Not read / not found" list.
- The readers' reports as written are kept beside this session's notes (`r_*.md`).


## Inventory, wear window, Inspect window


Read 2026-11-25, read-only. Original: GUI.dll decompile (`gf.py`, `calls.py`, `gdis.py`) and Gamecode (`gcf.py`).
Client: `crates/mao-client/src/{invwindows,gear,places,ui,net}.rs`, `gui/multilist.rs`, `pointer.rs`, `settings.rs`.
Marks: **[V addr]** read at that address today, **[NF]** looked, not found, **[NR]** not read.
Client line numbers are of the files as on disk today. "Doc" = `docs/INVENTORY_ORIGINAL.md`.

### 0. What I re-read in the original today (the spot check you asked for)

All ten named functions were read; the doc's original-side claims stand, with the corrections below.

| Function | Result |
|---|---|
| FUN_1004141b mouse up | [V] Same as doc. Old path (flag +0x319 clear): left up with Shift (`GetQualifiers & 3`) -> `+0x2e4` qualifier click; else flags 0xc0 -> select toggle; else flags & 2 -> `+0x2e0` click; else `+0x2d8` pick-up. Right up -> `+0x2e0` click unless flags & 0x400 (corpse). Asm 100414b0..1004157d. Only Shift is tested on a left up; Ctrl is not. |
| FUN_10041aa4 mouse down | [V] Left down does nothing. Right down: no click listener (`+0x2e0` empty, the Inspect lists) -> menu signal at once; else hold timer `+0x320`. |
| FUN_10041926 Ctrl split | [V] Ctrl or Alt bits `& 0xc` (Ctrl only: 4\|8), left button, hit item is an inventory item, its count (`FUN_1003d484(2)`) > 1 -> split mode on the PRESS. Shift is not tested here. Otherwise the normal MouseDown. |
| FUN_100cc6e8 pick-up | [V] Locked: nothing. View type 0 and the item is an open container: `CloseContainer`. View not own (`+0x14c == 0`): `MoveItemToInventory`. Else drag object + `BeginDrag`. |
| FUN_100cae1b click | [V] Locked: nothing. Trade view open and Ctrl (`& 0xc`): `N3Msg_TradeAddItem(partner, item)`. Else view not own or bank 0xdead: `MoveItemToInventory`. Else `N3Msg_UseItem(item, false)`. |
| FUN_100cc855 drop | [V] Doc correct. Extra detail: for a view with `+0x13c != 0` (wear pages are type 3-6) a left drop from another container calls `ContainerAddItem` and then **`Accept(2)`** (100cccf6/100cccfb). This matters for W3 below. |
| FUN_100e220f wear drop | [V] Doc correct. Right button: split > 0 or no item on the cell -> `Cancel`; else `UseItemOnItem`, Accept(1). Left: slot by FUN_100ce01d, else stat 0x58; `IsItemPossibleToWear(item, slot, page, true, inv)`; false -> return with neither Accept nor Cancel. |
| FUN_100caf07 item menu | [V] Entries LookAtItem, CreateReference; DeleteItem when not read-only and the view type is not 3/4/5/6; for types 3-6 **whatever read-only** the Lock/Unlock entry when `GetSkill(item, 8) == 0x499602d2 or <= 0` (asm 100cb006..100cb0be); Looter for 0xc76a in a team. |
| FUN_100cbace window menu | [V] ListMode; AutoArrange submenu (arg: own or 0xdeae, `FUN_10041637`), greyed in list mode; Rename only for container type 1 and `IsItemMine`. |
| FUN_100e20d6 wear menu | [V] LabelsOnTop (checked); UnlockAllItems only if `+0x1b4 == 0 && +0x1b8 == 0` (the viewed character is the own one; asm 100e2184). |

Also read: 100425fe/1004258f/1004266a are plain signal emitters; 100419d5 (mouse up in split mode raises pick-up with `+0x308` count); 100420fd (hold timer -> menu signal for the pressed item `FUN_10042a09`, not the item under the pointer then); FUN_10041ca4 (pointer shapes, split count); FUN_100ccde7 (wear pages ARE `InventoryViewBase_c`, and connect 100cc6e8/100cc855/100cae1b only when `+0x15c == 0`, 100caf07 and 100ca730 always); FUN_100ca730, FUN_100ca5cb (URL `itemid://%d/%d/%d/%d` = item identity + the view's container identity); FUN_10157f13 HandleMouseUp (the drag object is cancelled at the end only when no window is under the pointer, `+400 == 0`); `DragObject_c::Accept`/`Cancel` (both end by deleting the drag object, 1012b8f2/1012b989); FUN_100e1ded (the "no" pointer); FUN_100406a4 (count text); FUN_10042193 (tooltip: name in rarity colour + description when `ItemDescriptionTooltips`); Gamecode 10028883 (`Can & 0x10` raises GlobalSignals `+0xa0`, line 297).

### 1. Inventory window

`| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |`

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| I1 | Left click on an own item, nothing in hand | Pick-up on the RELEASE on the cell it went down on: item on the pointer; locked: nothing; an open bag in the inventory is closed first | [V 1004141b 10041568, 100cc6e8] | Same: `Click::TakeUp` -> `gear::pick_up`; locked skipped; a bag taken up has its window closed | invwindows.rs `events_of` 1009-1043 (1026, 1035); ui.rs 1677-1682 | MATCH |
| I2 | Left press alone | nothing | [V 10041aa4] | nothing (the grid is `click_only`, signals only on release) | invwindows.rs 1012; multilist.rs 486-507 | MATCH |
| I3 | Press, move, release on another cell | nothing (MultiListView only signals when up cell = down cell) | [V 1004141b; doc 0x10133d04 not re-read] | `click_only` list: no drag signal, no click for another cell | multilist.rs 493-497, 510 | MATCH |
| I4 | Item in hand, left click on an EMPTY cell (icon mode, not arranged) | `MoveItem` to the cell, Accept; nothing sent | [V 100cc855 `MultiListView_c::MoveItem` path] | `places::put` (client's own cells), nothing sent | invwindows.rs 679-684 | MATCH |
| I5 | Item in hand, left click on ANOTHER item of the pack | `JoinItems(target, held)`; true: accept; else swap: held goes to the cell, the other is picked up (its bag window closes) | [V 100cc855] | `Placing::Join` -> `events.joins`; else `Placing::Swap`: `places::exchange`, `gear::pick_up(other)`; other-level line `Feedback_CanOnlyStackItemsOfSameLevel` | invwindows.rs 686-705; places.rs 278-287 | MATCH (the join rule itself, Gamecode 1001bb2a, I did not re-read) |
| I6 | Left click on the item in hand itself | same identity -> Cancel (back in place) | [V 100cc855 `goto LAB_100ccc5f`] | `Placing::Cancel`, then `hand_opening` puts down | invwindows.rs 719; ui.rs 1785 | MATCH |
| I7 | Left drop on an item that is locked | `DroppedItemWasLocked`, Cancel, before anything else | [V 100cc855 100cc9a9..] | feedback + nothing, but only for a source in the pack; a worn item let go on the pack is moved without this test | invwindows.rs 663-674 | MATCH (tiny gap: wear -> pack path skips the lock test) |
| I8 | Worn item let go on an inventory cell | other container: `MoveItemToInventory(item)`, the cell remembered | [V 100cc855 100ccc86..100cccbd] | `events.moves (from, first free)`, cell awaited | invwindows.rs 663-671 | MATCH |
| I9 | Item in hand, left release over a window that takes no item (chat, a dock...) | HandleMouseUp cancels the drag ONLY when no window is under the pointer; with a window under it the drag object stays unless that view accepts or cancels. Whether the generic view cancels is not read | [V 10157f13: the Cancel is under `+400 == 0`] [NR generic Window::_HandleMouseUp] | any left release puts the item down | ui.rs 1785 -> `gear::put_down` | UNSURE |
| I10 | Right click (release before 0.3 s) on an own item, nothing in hand | `N3Msg_UseItem(item,false)`; locked: nothing | [V 100cae1b, 1004141b 10041526..] | `events.uses` -> `net.rs` fire: Gamecode's wear / take-off rule for items without Can bit 8, else use | invwindows.rs 915-925; ui.rs 1683-1704; net.rs 11657-11731 | MATCH |
| I11 | Right click on a bag in the inventory | Sends `UseItem` again every time; the server's answer opens or closes the window | [V 100cae1b; GC per doc 10087f63, 1007e6e2] | Second use toggles the window locally, nothing sent | ui.rs 1683-1691 | DIFFER (DEVIATIONS INV-038, INV-062: left, needs a capture) |
| I12 | Right click on an item with `Can & 0x10` | the use is not sent; GlobalSignals `+0xa0` is raised instead (a confirm). Interfaces' wrapper 0x1000a73f does the same for stat 0x184 / shield disablers (doc [V IF 1000a73f], not re-read) | [V GC 10028883 line 297 `& 0x10` -> `+0xa0`] [NR receiver of +0xa0] | no such test: the use is sent | net.rs 11657-11731 (no `& 0x10`, no stat 0x184) | MISSING |
| I13 | Ctrl + right click while a trade or shop is open | `TradeAddItem(partner, item)` (any trade view); without a trade open Ctrl + right is an ordinary use | [V 100cae1b] | `trade_adds` when `shop::trading()` and Ctrl; ui.rs sends `ShopCommand::Sell` only if a shop is open and not buy-only without partner | invwindows.rs 920; ui.rs 1718-1723 | MATCH (player trade via `shop.partner` assumed, shop.rs not read) |
| I14 | Item in hand, right click on another item | no Shift: `UseItemOnItem` (use message action 5), Accept. Shift, both in the inventory: `TradeskillCombine` (0x51), else cancel. On itself nothing. With a split in hand: cancel | [V 100cc855 `+0x10 == 2` branch] | `held_right_click`: UseOn / Combine / Nothing, hand emptied; split -> put down | invwindows.rs 198-205, 895-914; net.rs 10987, 11417 | MATCH |
| I15 | Item in hand, right click on an EMPTY cell | Cancel (the item goes back) | [V 100cc855 `LAB_100ccc5f` when `this_00 == 0`] | nothing happens, the item stays in hand (the branch needs `hovered_item`) | invwindows.rs 891-926 | FIXED 2026-11-25 (was DIFFER): the drag is given up, `invwindows.rs` `right_beside` |
| I16 | Right button held 0.3 s on an item | menu signal for the item PRESSED on (`FUN_10042a09`), wherever the pointer is by then; release then does nothing | [V 10041aa4, 100420fd] | menu opens after 300 ms only while the pointer is still over the same item; release after a menu does nothing (`self.menu.is_none()` guard) | invwindows.rs 866-890, 891-893 | DIFFER (small: pointer moved off) |
| I17 | Menu entry "LookAtItem" | `ShowURL("itemid://type/id/containerType/containerInst")` | [V 100ca5cb] | `ShowURL("itemref://low/high/ql")` from the template | invwindows.rs 962 | UNSURE (same page, but the original asks for the live item, the client for template + QL) |
| I18 | Entry "CreateReference" | `RefMan_t::AddItemRef` of `N3Msg_GetReferenceData` | [V 100ca55c] | an `<a href="itemref://..">name</a>` into the chat line | invwindows.rs 182-184, 963-968 | UNSURE (link format of `RefMan` not read) |
| I19 | Entry "DeleteItem" (not read-only, not a wear page) | question `ReallyDeleteItemX` (Warning, Yes/No) -> `N3InterfaceModule_t::DeleteItem(item_id, template_id)` | [V 100ca997, 100ca649] | question with the game's text, Yes -> `events.deletes` | invwindows.rs 977-991 | MATCH |
| I20 | Shift + left click (nothing in hand) | `+0x2e4` -> item info page (`itemid://`) | [V 1004141b 100414e8, 100ca730] | `Click::Info` (shift wins over all) | invwindows.rs 251-258, 1027 | MATCH (URL form as I17) |
| I21 | Shift + left click with an item in hand | the release goes to the drag object's drop (left button, Shift ignored by the left branch of 100cc855), not to the list's MouseUp; so no info | [V 10157f13 / 100cc855 reading; the dispatch Window::_HandleMouseUp -> drop vs MouseUp not read] | `click()` gives Info "whatever is in hand" AND the drop rules run on the same release | invwindows.rs 1107-1108, 1027 | FIXED 2026-11-25 (was UNSURE; settled: FUN_1004141b raises only the drop signal for a drag object): no info, `invwindows::click` |
| I22 | Shift + right click | ordinary use (right up never tests qualifiers) | [V 1004141b 10041526..] | ordinary use | invwindows.rs 915-925 | MATCH |
| I23 | Ctrl + left click on a SINGLE item | same as plain click: picks it up (left up tests only `& 3`) | [V 1004141b 10041539..1004153e] | `click(.., ctrl) -> Click::Nothing` unless it is a stack: nothing is picked up | invwindows.rs 251-258; test 1111-1112 | FIXED 2026-11-25 (was DIFFER): taken up, `invwindows::click` |
| I24 | Ctrl + left on a STACK (count > 1) | split mode on the press: mouse sideways counts 1..count-1, release -> pick-up with that count; drop on an empty cell -> `SplitItem` | [V 10041926, 100419d5, 10041ca4] | on the release a small box asks the number; OK puts that many in hand; empty cell -> `events.splits` | invwindows.rs 1028-1034, 513-515, 993-1001 | FIXED 2026-11-25 (was ASKED, INV-008; the user's answer: "Yes, as the original"): the count by the mouse's way sideways, the pointer put back after each move, the number in yellow at it; the release has that many in hand (`gear::count_begin`, `count_step`, `count_take`). The number box is gone |
| I25 | Ctrl + Shift + left on a stack | split mode (10041926 does not test Shift; the release goes the split path, no info) | [V 10041926, 100419d5] | Shift wins: info | invwindows.rs 251-254 | FIXED 2026-11-25 (was DIFFER): the split, `invwindows::click` |
| I26 | Alt with any click | no test of the Alt bits (0x30) in these handlers | [NF 1004141b, 10041aa4, 10041926, 100cae1b, 100cc855] | none | invwindows.rs | MATCH |
| I27 | Double click | no handler; first click picks up, second drops on itself = cancel | [V 100cc855 same identity] | same (INV-003) | invwindows.rs 1016; places.rs 281 | MATCH |
| I28 | Mouse wheel over the list | `WheelMoved` of the scroll view (10142820); but the wheel goes to the window's default wheel view (`_HandleMouseWheel` 10154ab9: `+0xa4` else `+0x84`), not necessarily the view under the pointer | [V 10157cec, 10154ab9; NR which view the inventory registers] | topmost scroll view under the pointer, step 10 per notch, snapped to a row | gui/host.rs 346-352; scroll.rs 276-300 | UNSURE |
| I29 | Escape | window closes if `esc_inventory` (default false in LoginPrefs) | [V 100ccefe string at 100cd5c5; LoginPrefs.xml 61] [NR the closing code] | `esc_default("esc_inventory") = false` | ui.rs 411-413, 459 | UNSURE (default agrees) |
| I30 | Escape with an item in hand | [NR] whether the original cancels a drag on Escape | [NR] | puts the item back | ui.rs 1785 | FIXED 2026-11-25 (was UNSURE; settled by the frame's reading: every drag object's `Cancel` is a slot of the Escape signal, 0x10157842): Escape puts the item back, and now does so for a real keyboard |
| I31 | Key that opens the window | [NR]: no key binding in `gui/Default/*.xml`; the menu entry exists in LeftMenu.xml | [NF in gui/Default] | `I` (Action::InventoryWindow), `Ctrl+1` for wear | settings.rs 235-236 | UNSURE |
| I32 | Tooltip | `FUN_10042193`: name in rarity colour, plus the item's info text when `ItemDescriptionTooltips` (true) | [V 10042193] | `gear::tooltip` -> `iteminfo::item_tip`; hidden while an item is in hand; not compared line by line | invwindows.rs 927-936; gear.rs 312-327 (DEVIATIONS INV-005 done) | FIXED 2026-11-25 (was UNSURE; read: `UpdateToolTip` 0x10157adc, `MoveToMouse` 0x101550de, FUN_10032c1d): the tip comes after 500 ms, is placed once and goes at a button (`gui/tooltip.rs`); its modifier list has no casts and no criteria (`spells::convert_in_tip`). The original was not found to hide a tip during a drag; here none shows with an item in hand |
| I33 | Pointer shape over items | no drag, over an own item with `Can & 8`: right-button detail 2 (not on a corpse), left detail 2 only for click-flag lists; Shift held, nothing in hand: left detail 3 (info); otherwise the normal pointer | [V 10041ca4] | none (only the wear page's "no" pointer) | invwindows.rs 816 only | MISSING |
| I34 | List mode rows, headers | rows send the same signals; header click sorts, edge drag resizes, double click on edge fits | [NR rows today; `ColumnHeaderView_c` not re-read] | `multilist` Header, the sort and widths kept; a clicked row is also `select`ed (highlight) | multilist.rs 527+; invwindows.rs 1017 | UNSURE (whether the original highlights a clicked row [NR]) |
| I35 | Window menu entries | ListMode (check), AutoArrange submenu (greyed in list mode); no Rename for the inventory | [V 100cbace] | "ListMode" with mark, "AutoArrange" with `arrange_menu`, greyed in list mode, plus "Close Window" | invwindows.rs 726-753 | MATCH (+ the Close entry is the frame's) |
| I36 | Right click on empty space / an empty cell, nothing in hand | nothing (10041aa4 returns when no item is under the pointer); the window menu comes from the frame's menu button | [V 10041aa4 `iVar4 == 0`] | opens the window menu at the pointer | invwindows.rs 604-610 (CLIENT_UI 5850) | FIXED 2026-11-25 (the ADDITION is gone): a right click beside the items does nothing (FUN_10041aa4); the window's menu is the frame's menu button's (or the rollup page's) |
| I37 | Item in hand let go over the 3D world | FUN_100cbcb5: ground drop question / `UseItemOnCharacter` / `UseItemOnItem` | [V doc only; not re-read] | `letting_go_on_world` (either button), `dropitem` | gear.rs 219-227; ui.rs 1768-1781 | UNSURE (dropitem.rs not read) |
| I38 | Drop in list mode or arranged grid: split onto an occupied row; `MoveItem` when arranged but icon mode | split: the cell test is `this_00 == 0 \|\| !icon_mode`, so a list-mode drop on a row still splits (first free place); arranged icon mode: `MoveItem` | [V 100cc855] | split on an item = `Placing::Cancel`; `Move`/`Swap` only when `hand` (not arranged) | invwindows.rs 658-662, 679-694; places.rs 284-285 | DIFFER |
| I39 | Count text on the icon | yellow, `%uM` from 10 000 000 (unless the text view has keyboard focus); drawn for count >= 1 (hidden below 1) | [V 100406a4; caller's value for a single item not traced] | plain number, only when count > 1 | gui/multilist.rs 380-382 (the `%uM` exists only for prices, 877) | DIFFER (trivial; stacks cap at 50 000) |

#### Rules to port (Inventory)

- **I11** (bag use / close): `N3Msg_UseItem(bag)` on every right click; drop the local toggle; follow the server's item action 0x66 to close the window; send `UseItem(container identity)` when the window is closed (FUN_100ca891). `ui.rs` loop 1683, `loot.rs` close. LARGE: needs a capture of a backpack used twice (INV-038).
- **I12**: before `use_object`, test `Can & 0x10` (stat 30) on the item: raise the original's `GlobalSignals +0xa0` confirm instead of sending; also stat 0x184 / shield-disabler confirm (IF 0x1000a73f). First read who listens to `+0xa0` and `+0x170`. `net.rs` fire block ~11657 / `ui.rs` 1683. SMALL to MEDIUM.
- **I15**: right button released over an empty cell (or any list cell with no item) with an item in hand and no split: `gear::put_down()`; also for a split. `invwindows.rs` the block at 891: add an `else if dragged.is_some() && secondary_clicked && over a grid` branch. SMALL.
- **I16**: remember the pressed item; at 0.3 s open its menu even if the pointer moved (position: the pointer now). `invwindows.rs` 866-890. SMALL.
- **I23 / I25**: in `click()`: Ctrl with a non-stack, or with the stack not splitting, must be `TakeUp` (only `& 3` Shift is tested on the release). Ctrl + Shift on a stack: split, not info. `invwindows.rs` `click` 251-258 and its test 1109-1116. SMALL.
- **I33**: set pointer details while hovering an item with nothing in hand: own item with `Can & 8` -> right-button detail "use" (`GFX_GUI_POINTERDETAIL_OPERATE`-class), Shift held -> left detail "info". `pointer.rs` has the `Look`/detail machinery (world only); add a way for `invwindows.rs` to hand it a Look for the frame. MEDIUM.
- **I38**: split on an occupied row/cell in a list or an arranged grid still splits to the first free place; arranged icon mode does the `MoveItem`. `invwindows.rs` 675-721 (`onto` is also used for `Placing::Split`; pass `None` for the split decision when `!hand`). SMALL.
- **I39**: use `price_text` for the count in `multilist` 380-383; decide count 1 (read FUN_100406a4's callers first). SMALL.

### 2. Wear window (four pages)

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| W1 | Left click on a worn item, nothing in hand | pick-up as I1 (the pages are `InventoryViewBase_c`, type 5/3/4/6; locked: nothing; no `CloseContainer` since type != 0) | [V 100ccde7, 100cc6e8] | `events_of` on each page's grid; locked (city or equipment lock) skipped | invwindows.rs 836-838, 1026 | MATCH |
| W2 | Item in hand, left click on a wear cell | slot = the cell's slot (FUN_100ce01d) else stat 0x58; `IsItemPossibleToWear(item, slot, page, true, inv id)`; true -> `MoveItemToInventory(item, inv id, slot)`; inv id 7 weapons, 3 clothes, 8 implants, 0xe social | [V 100e220f] | `wear_drop_place` (cell place, else template stat 0x58), `events.moves (from, to)`; the game loop asks the same check and sends | invwindows.rs 67-72, 802-835; net.rs wear_refusal 1594 | MATCH |
| W3 | Refused wear: does the item stay in hand? | 100e220f returns with no Accept/Cancel on a refusal. BUT FUN_100cc855 is connected to the same list and, for the wear view (`+0x13c != 0`) with a left drop from the inventory, ends in `ContainerAddItem` + **`Accept(2)`** (100cccf6). Accept deletes the drag object (1012b8f2). Whether it runs before or after 100e220f (the slot order of `SignalBase_c::_Connect`) is not read | [V 100cc855, 100e220f, 100ccde7, 1012b8f2] [NR _Connect order] | on a refusal the item is picked up again (stays in hand) | invwindows.rs 826-835 | FIXED 2026-11-25 (was UNSURE; settled: `_Connect` puts a slot at the head, so FUN_100e220f runs first and FUN_100cc855 after it accepts the drag, 0x100cccce..0x100cccfb): the hand is empty after a refused wear |
| W4 | Item in hand over a cell: "no" pointer | "no" for: split in hand, beside the cells, item with no wear mask (stat 0x12a 0/unset) unless over an item, cell that is no slot, slot the check refuses; otherwise the drop pointer | [V 100e1ded] | `wear_pointer_refuses` with the same cases; not judged yet: no "no" | invwindows.rs 79-88, 809-818 | MATCH |
| W5 | Right click on a worn item, nothing in hand | use -> client-side wear / take-off rule (Gamecode 10028883), `Feedback_EquipmentLocked` for a locked place | [V 100cae1b; GC 10028883 lines 150-166] | `events.uses` -> `net.rs` fire (take off into first free place, `unwear_refusal`) | invwindows.rs 915-925; net.rs 11657-11731 | MATCH |
| W6 | Item in hand, right click on a worn item | `UseItemOnItem(held, worn)` (Shift ignored), Accept; split > 0 or no item on the cell: Cancel | [V 100e220f `+0x14 == 2`] | `held_right_click` with the worn place: UseOn (Shift too), hand emptied | invwindows.rs 198-205, 895-914 | MATCH |
| W7 | Item in hand, right click on an EMPTY wear cell | Cancel (item back) | [V 100e220f `LAB_100e230f`] | nothing, item stays in hand | invwindows.rs 891-926 | FIXED 2026-11-25 (was DIFFER): the drag is given up |
| W8 | Same right click: possible second send | FUN_100cc855 also runs on the right-button branch (UseItemOnItem, Accept(1)); with 100e220f also sending it, the original may send the use twice, or not at all if the first handler deleted the drag. Order not read | [V both] [NR order] | one send | invwindows.rs 909 | UNSURE |
| W9 | Shift + left click on a worn item | info page | [V 1004141b, 100ca730] | `Click::Info` | invwindows.rs 1027 | MATCH |
| W10 | Ctrl + left click on a worn item | pick-up (as I23) | [V 1004141b] | nothing | invwindows.rs 251-258 | FIXED 2026-11-25 (was DIFFER): taken up |
| W11 | Right button held 0.3 s: menu | LookAtItem, CreateReference, then (types 3-6) LockItem / UnlockItem when `GetSkill(item, 8)` is unset or <= 0; never Delete | [V 100caf07] | same entries, lock test from the template's stat 8 | invwindows.rs 498-510 | MATCH |
| W12 | Entry Lock / Unlock | toggles the place's identity in `EquipmentLocks` (kept) and the `InventoryItemLocker_c`; a locked item can't be picked up, used, dropped on | [V 100ca3ae; doc 100cc6e8/100cae1b/100cc855] | `gear::set_equipment_lock`, bit per place, kept per character; `locked()` used by pick-up, use | gear.rs 73-127, 146-150; invwindows.rs 970 | MATCH |
| W13 | Window menu "LabelsOnTop" | check; sets the grid label position on all four pages | [V 100e20d6, 100e1c7b] | check; all four pages (the social page's kept value quirk ported) | invwindows.rs 773-777, 841-849 | MATCH |
| W14 | Window menu "UnlockAllItems" | writes an empty `EquipmentLocks` (identities 0x65-0x67, 0x73); shown only for the own character | [V 100e20d6 asm 100e2184, 100e1f9b] | `unlock_all_equipment` (all 64 bits), always shown (own window) | invwindows.rs 850, 842 | MATCH |
| W15 | Right click on empty space of a page | nothing | [V 10041aa4] | opens the window menu | invwindows.rs 794-798 | FIXED 2026-11-25 (the ADDITION is gone): a right click beside the items does nothing (FUN_10041aa4); the window's menu is the frame's menu button's (or the rollup page's) |
| W16 | Page tab click | `TabView` selection, `selected_tab` kept in `WearViewConfig` and read on build | [V 100e24b8 'selected_tab' 100e307b] | `tab_view`, `layout::set_tab("WearViewConfig")` | invwindows.rs 787-825, 529-530 | MATCH |
| W17 | Escape | `esc_wear` (default true) read in the WearView constructor | [V 100e24b8 100e3168; LoginPrefs 73] [NR closing code] | `"Wear" -> esc_wear` | ui.rs 460-461 | UNSURE |
| W18 | Pointer shapes over worn items | as I33 | [V 10041ca4] | none | invwindows.rs | MISSING |
| W19 | Tooltip over a worn item | as I32 | [V 10042193] | `gear::tooltip` | invwindows.rs 927-936 | FIXED 2026-11-25 (was UNSURE): as I32 |
| W20 | Wheel | wear pages have no scroll bars (3 x 5 fixed cells, `SetViewCellCounts`) [NR] | [NR] | nothing | invwindows.rs 315-332 | UNSURE |
| W21 | Double click, Alt | no handler / no Alt test | [NF] | none | - | MATCH |
| W22 | Worn item dragged to another cell of the same page | same-container left drop in 100cc855 (join test, `MoveItem` of the CELL in icon mode, pick up of an occupied cell) runs as well as 100e220f's wear move; not traced to an end result | [V 100cc855, 100e220f] [NR combined effect] | wear drop only (`drop_on`), cells fixed by the tables | invwindows.rs 802-806 | UNSURE |
| W23 | Drop on a wear cell whose item is locked | 100cc855 first: `DroppedItemWasLocked`, Cancel | [V 100cc855 100cc9a9..] | no lock test on the target; the use path has `Feedback_EquipmentLocked` but the move path is not tested | invwindows.rs 802-835 | DIFFER (small) |

#### Rules to port (Wear)

- **W7**: as I15, in the wear grid block (790-798): right release with an item in hand over the grid and no item under it -> `gear::put_down()`. SMALL.
- **W10**: as I23. SMALL.
- **W18**: as I33. MEDIUM.
- **W23**: in the `drop_on` block (802-806): if the cell's worn item is `gear::locked(slot)` push feedback `DroppedItemWasLocked` and `put_down()`; do not push the move. `invwindows.rs` 826. SMALL.
- **W3 / W8 / W22**: to settle, read `SignalBase_c::_Connect` (it is not in `exports_gui`: look in the base DLL) for the emit order. If 100cc855 runs first it accepts the drag, so the original does NOT keep a refused item in hand; then change invwindows.rs 832-834 to end the drag, and decide W8's double send. Needs a live look in the original (put a refused item on a wear cell). MEDIUM, needs a capture.

### 3. Inspect window (another character's wear pages, read-only)

The list is built read-only (`+0x15c != 0`): pick-up, drop and click signals are not connected; the qualifier click (Shift) and the menu are [V 100ccde7].

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| N1 | Left click on an item, no keys | nothing (no pick-up listener) | [V 100ccde7, 1004141b] | the clicked item is `select`ed (highlight) only | invwindows.rs 442-448 | MATCH (the highlight is the list's own; [NR] whether the original also marks it) |
| N2 | Shift + left click on an item | `+0x2e4` is connected: the item's info page (`itemid://`) | [V 100ccee8, 1004141b 100414e8, 100ca730] | nothing: the click only selects | invwindows.rs 442-448 | FIXED 2026-11-25 (was MISSING): the info page, `invwindows::inspect_click_shows` |
| N3 | Right button pressed on an item | no click listener -> the menu opens at once on the press, at the pointer | [V 10041aa4 `+0x2e0 == 0`, 1004266a, 100caf07] | opens the item menu on `button_pressed(Secondary)` over an item | invwindows.rs 452-459 | MATCH |
| N4 | Menu entries LookAt, CreateReference | as I17 / I18 | [V 100caf07] | `inspect_entries`: LookAt (`itemref://`), Create Reference (chat link) | invwindows.rs 485-491, 460-472 | MATCH (URL forms as I17, I18) |
| N5 | Menu entry Lock / Unlock | for view types 3-6 shown whatever read-only, when the item's stat 8 is unset or <= 0 (100cb07f jumps to the lock entry on the page-type flag alone) | [V 100caf07 asm 100cb021..100cb0be] | not offered (client's note: which item it would lock is unknown) | invwindows.rs 485-488 | DIFFER |
| N6 | Right release after the menu opened | nothing more | [V 100420fd] | popup open, no action | invwindows.rs 460-472 | MATCH |
| N7 | Right click on empty space of a page | nothing | [V 10041aa4] | opens the window menu | invwindows.rs 417-420 | FIXED 2026-11-25 (the ADDITION is gone): a right click beside the items does nothing (FUN_10041aa4); the window's menu is the frame's menu button's (or the rollup page's) |
| N8 | Window menu | LabelsOnTop only (the unlock entry hidden for another character) | [V 100e20d6 asm 100e2184] | "Labels over the items" + frame's "Close Window" | invwindows.rs 430-441 | MATCH |
| N9 | Page tab | same TabView; `selected_tab` kept in `InspectViewConfig` | [V 100e24b8 / CharPrefs 119] | `layout::set_tab("InspectViewConfig")` | invwindows.rs 385, 424-427 | MATCH |
| N10 | Title | "Inspect" whoever is inspected | [V 100e24b8 'Inspect' 100e24fe] | "Inspect" | invwindows.rs 408 | MATCH |
| N11 | Escape | `esc_wear` is read in the same constructor, so the Inspect window follows it; the closing mechanism [NR] | [V 100e24b8 100e3168] | `"Inspect" -> esc_wear` | ui.rs 461 | UNSURE |
| N12 | Drop on the list, double click, Alt, Ctrl+click, wheel | no drop connection, no handler, no Alt test | [V 100ccde7] [NF] | nothing | - | MATCH |
| N13 | Tooltip | as I32 | [V 10042193] | `gear::tooltip` over the hovered item (hidden while the menu is up) | invwindows.rs 473-480 | FIXED 2026-11-25 (was UNSURE): as I32 |
| N14 | Pointer shape: Shift over an item | left detail 3 (info) | [V 10041ca4] | none | - | MISSING |

#### Rules to port (Inspect)

- **N2**: Shift held when the left button comes up on an item: push `format!("itemref://{}/{}/{}", id, high, ql)` to `events.infos`. In the loop at `invwindows.rs` 442-448, `ItemEvent::Clicked` case: add `if shift { events.infos.push(..) }` (need `by_slot` for the item; the `ctx.input(|i| i.modifiers.shift)` as in `events_of` 1011). SMALL.
- **N5**: add `LockItem` / `UnlockItem` to `inspect_entries()` when the item's stat 8 is unset or <= 0; the effect in the original writes the OWN character's `EquipmentLocks` with that place's identity (it reads the identity from the item, FUN_100092ce, and toggles it), so the likely port is the same `gear::set_equipment_lock(slot, ..)` on the same place number. Unsure what the original means by it for another character's items; low value. SMALL, UNSURE.
- **N14**: as I33.

### 4. The doc's section 10, row by row, against today's code

| Doc row | State today |
|---|---|
| 1 wear/take-off by use | DONE: net.rs 11657-11731 (INV-022). Missing the `Can & 0x10` confirm (I12). |
| 2 held item on empty cell / swap, cells per container | DONE: places.rs, invwindows.rs 650-722. Gaps I38. |
| 3 stacks | DONE with a number box instead of the mouse (ASKED, INV-008). Join and split messages sent (net.rs 11425+). |
| 4 held + right click = UseItemOnItem, Shift = combine | DONE (net.rs 10987 use action, 11417 combine 0x51). |
| 5 held item on the ground | DONE via ui.rs 1768-1781 / dropitem.rs (not read today). |
| 6 bag close / second use sends UseItem | OPEN (INV-038, INV-062). |
| 7 bag picked up closes its window | DONE ui.rs 1677-1682. |
| 8 Ctrl + right click adds to trade | DONE invwindows.rs 920, ui.rs 1718. |
| 9 item menu | DONE for inventory and wear (0.3 s, entries, Shift+right = use). Looter and container windows are loot.rs (not mine, not read). |
| 10 wear drop refused: stays in hand, "no" pointer, own place | DONE in code (invwindows.rs 802-835, 79-88), but see W3: the original may not keep it in hand. |
| 11 corpse right click | loot.rs, not read. |
| 12 press-move-release | DONE (`click_only`). |
| 13 bank item on a bag window | loot.rs, INV-060 open, not read. |
| 14 window menus | DONE for inventory, wear, inspect (Rename is loot.rs). |
| 15 inventory width | OPEN (INV-027, five columns; the original's first width unknown). |
| 16 tooltip | PORTED (INV-005); not compared line by line today. |
| 17 count text `%uM` | OPEN (I39). |

### 5. Counts

Inventory 39 rows, Wear 23, Inspect 14: 76 rows.

- MATCH: **37** (Inventory 17: I1-I8, I10, I13, I14, I19, I20, I22, I26, I27, I35; Wear 12: W1, W2, W4, W5, W6, W9, W11, W12, W13, W14, W16, W21; Inspect 8: N1, N3, N4, N6, N8, N9, N10, N12).
- DIFFER: I11, I15, I16, I23, I25, I38, I39, W7, W10, W23, N5 = **11**.
- MISSING: I12, I33, W18, N2, N14 = **5**.
- ADDITION: I36, W15, N7 = **3**.
- ASKED: I24 = **1**.
- UNSURE: I9, I17, I18, I21, I28, I29, I30, I31, I32, I34, I37, W3, W8, W17, W19, W20, W22, N11, N13 = **19**.

Total 37 + 11 + 5 + 3 + 1 + 19 = 76.

### 6. Not read / not found

- [NR] Which view receives the wheel for the inventory (`SetDefaultWheelView`), and `esc_*` closing code for inventory / wear / inspect (only that the preference is read in the constructors).
- [NR] `SignalBase_c::_Connect` (not in `exports_gui`): the order of slots on a drop, which decides W3, W8, W22.
- [NR] `Window::_HandleMouseUp` generic view dispatch: whether a left release over a window that does not accept (I9, I21) cancels the drag.
- [NR] The receiver of GlobalSignals `+0xa0` and `+0x170` (the confirm of I12).
- [NR] FUN_100cbcb5 (drop on the world) and `dropitem.rs`, `shop.rs`, `loot.rs` (corpse, chest, bank, bag windows), the hotbar's drop, Gamecode `N3Msg_JoinItems`/`SplitItem` bodies (taken from the doc), `MultiListView_c::MouseUp` 0x10133d04 (taken from the doc and the client test).
- [NR] Callers of FUN_100406a4 (what count a single item carries), the tooltip delay and whether a tooltip hides during a drag, the key bindings of the original (none in `gui/Default`).
- [NR] `RefMan_t::AddItemRef` link format; whether a list row is highlighted on click in the original.
- [NF] Any Alt handling in the item lists; any "take all" in GUI.dll (doc); any pointer-detail code in the client's inventory.

## Backpacks, chests and corpses, the bank, reclaim


Original = GUI.dll decompile (`exports_gui`), Gamecode only where named. Client = `modern-anarchy-online/crates/mao-client/src` read from disk today.
`loot.rs` = `crates/mao-client/src/loot.rs`. Line numbers are those of the file as read.
Marks: **[V addr]** read and confirmed by me; **[NF]** looked, not found; **[NR]** not read (stated, or leaned on `docs/INVENTORY_ORIGINAL.md` = "doc").

### 0. What I spot-checked in `INVENTORY_ORIGINAL.md` (not copied)

- Wiring: `ItemContainerView_c` FUN_100ce55d owns the list at +0x158; type-1 views are built by `InventoryView_c` FUN_100ccefe via `SlotContainerOpened` 0x100c7bef, which builds a view only if the identity has none in its map, and `SlotContainerClosed` 0x100c757b deletes the view **[V 100c7bef, 100c757b]**. Handlers pick-up 100cc6e8, drop 100cc855, click 100cae1b, menu 100caf07, qualifier 100ca730 **[V read each]**.
- The list's old mouse scheme **[V 1004141b, 10041aa4, 100419d5]**: left button down does nothing (only button 2 acts on the way down); left up on the same cell: Shift (`GetQualifiers & 3`) -> qualifier signal `+0x2e4`; else with feature flags `& 0xc0` -> select; else with flag `& 2` -> click `+0x2e0`; else pick-up `+0x2d8` with count 0. Right up: flag `0x400` -> nothing, else click `+0x2e0`. Right down: no click listener -> menu at once, else timer `+0x320` of `MouseButtonHoldDelay`. Confirmed. (The `+0x319` "new" scheme is only for the five lists the doc names; I did not re-scan for callers of FUN_10041248.)
- **Corrected / added versus the doc:**
  - The bank's "own" flag `+0x14c` **is 1** (set at 100ce94e after `SetMaxItemCount(0x66)`): the bank is not auto-sorted and a left click picks it up, but 100cae1b still treats it specially (`+0x14c==0 || identity==0xdead` -> take out) **[V 100ce9d6..100ce94e, 100cae1b]**.
  - Reclaim (0xdeae): `+0x14c = 0`, no sort, no `0x400`, max items not set **[V 100cea32..100cea87]**. FUN_100cbace passes `own || identity==0xdeae` as the Auto Arrange flag, so reclaim's Auto Arrange submenu is the "own" flavour (with "Don't Auto Arrange") **[V 100cbace]**.
  - Escape flag chosen in the ctor: type 0 -> `esc_inventory`; type 1 with identity 0xc749 -> `esc_backpacks` if `+0x14c` else `esc_chests`; identity 0xc76a -> `esc_corpses`; every other container (bank 0xdead, reclaim 0xdeae, 0xdeaa) reads none **[V disasm 100cd5ba..100cd7c1]**; the registered callback is FUN_1004a1b1 = vtable +0xfc (the close). That the registration is bound to the Escape key I take from the `esc_*` naming pattern; the key dispatch was not traced **[NR]**. Defaults from LoginPrefs.xml: `esc_backpacks`, `esc_chests`, `esc_corpses` true, `esc_inventory` false **[V LoginPrefs.xml 56-63]**.
  - Empty-container timer (1 s, 0xf4240 µs): armed at creation only when `+0x14c==0` **and** identity is 0xc76a or 0xc749, and re-armed by the list's change signal FUN_100ca8d9 when the list is empty; reclaim, bank, 0xdeaa never close themselves **[V 100cd53c..100cd5b0, 100ca8d9]**.
  - Window close, vtable +0xfc FUN_100ca891: for container type 1 sends `N3Msg_UseItem(container identity, false)` after closing the view **[V 100ca891]**.
  - Ctrl+left down on a stack (count > 1, `FUN_1003d484(2) > 1`, `GetQualifiers & 0xc`) enters split mode in **every** item list, own or not **[V 10041926]**; but FUN_100cc6e8 only uses the count in its "own" (drag) branch, so on a corpse/chest the split UI ends with the whole stack taken **[V 100cc6e8]**.
  - Alt: no handler among 10041926, 100419d5, 10041aa4, 1004141b, 100cc6e8, 100cae1b, 100cc855, 100caf07, 100ca730 tests bits `0x30`; FUN_10041ca4's only `GetQualifiers` use (100420be) tests `& 3` **[NF Alt]**.
  - Qualifier tests that exist: Shift (`&3`) in 1004141b, 100cc855 (right drop: plain vs Shift); Ctrl (`&0xc`) in 10041926 (split) and 100cae1b (trade add).
  - Drop handler 100cc855 order **[V]**: (1) source container 0xdead, not the same container, this view's identity non-zero -> text `ItemsDraggedFromBankCanOnlyBeDroppedToBank` on GlobalSignals +0x17c, cancel; (2) target item locked -> `DroppedItemWasLocked`, cancel; (3) button 2: only with split<1 and an occupied cell: no Shift -> `UseItemOnItem(held, target)`, accept(1); Shift -> `TradeskillCombine` only when `+0x13c==0` and same container, else cancel; empty cell -> cancel; (4) button 1 same container: split<1: occupied cell: same item -> cancel; else `JoinItems(target, held)` true -> accept; then if icon mode (`LayoutMode != 1`) `MoveItem(held cell -> cell)`, accept, and the displaced item is picked up (FUN_100cc6e8); empty cell -> same move; split>=1: empty cell or list mode -> remember cell (first free pos in list mode), `SplitItem(item, count)`, occupied icon-mode cell -> cancel; (5) button 1 other container, split<1: remember cell (first free if occupied); `+0x13c != 0` -> `ContainerAddItem(this container, item)`, accept(2); else (inventory) `MoveItemToInventory(item)`, accept only if true; split>=1 -> cancel. Any other button: returns without accepting.

### 1. Own backpack window (`Container_<type>x<instance>`, identity 0xc749 and `N3Msg_IsItemMine`; `+0x14c`=1, 21 places, not auto-sorted)

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| A1 | Left click an item, nothing in hand | On button **up** on the cell it went down on: pick-up signal -> FUN_100cc6e8: not locked; (inventory type only closes an open bag); own -> builds the item icon + drag object, `BeginDrag`; item is on the pointer until the next mouse-up | 10041568..10041578, 100cc6e8 | `container_click(false, own=true, hands_free, false)` -> `TakeUp`; `dragging = Some(slot)`; item follows the pointer | loot.rs `container_click` 1178; `show` 849-855, 868-875 | MATCH |
| A2 | Left press alone | nothing (no left-down action in the old scheme) | 10041aa4 (button != 2 -> exit) | no action until release (`click_only`) | loot.rs 836 | MATCH |
| A3 | Press, move to another cell, release | Nothing: the list signals only when up cell = down cell (relies on `MultiListView_c::MouseUp` 0x10133d04, which I did not re-read) | 1004141b (pick-up only from the up handler); 10133d04 [NR] | `multilist::click_only` | loot.rs 836 | MATCH (10133d04 not read) |
| A4 | Item in hand (from this bag), left click an **empty** cell, grid placed by hand | `MoveItem(held cell -> cell)`, accept; no message to the server | 100cc855 (same-container, split<1, icon mode) | `Placing::Move` -> `places::put(identity, place, cell)` | loot.rs 916-921 | MATCH |
| A5 | Same, left click **another item** | `JoinItems(target, held)` first (true -> accepted, stacks merged); else icon mode: `MoveItem`, then the displaced item is picked up | 100cc855 | `Placing::Join` -> `Take::Join` (only for `view.own`); else `Placing::Swap` -> `places::exchange`, other item in hand, `Feedback_CanOnlyStackItemsOfSameLevel` when levels differ. Client's join test is its own (`places::joins`: same low/high/quality, both stackable, sum <= 50000), the original's is `N3Msg_JoinItems` GC 1001bb2a (doc: stats 0x2be/0x2bf/0x2bd) which I did not re-read | loot.rs 906-960; places.rs 266 | MATCH (join rule not compared with GC 1001bb2a) |
| A6 | Same, left click on the item's own cell | cancel, back in place | 100cc855 (same id -> `DragObject::Cancel`) | `Placing::Cancel` -> nothing, `dragging` cleared | loot.rs 881-883, 957 | MATCH |
| A7 | Same in **list mode** (or auto-arranged grid) | Join tried (occupied row only); else cancel. Split count: first free place regardless of target | 100cc855 (`bVar12`/`GetLayoutMode != 1`) | `Move`/`Swap` need `hand`; Join works; a split onto an occupied row is `Placing::Cancel` (original sends `SplitItem` to the first free place in list mode) | loot.rs 906-958; places.rs 285 | DIFFER (split onto an occupied row in list mode) |
| A8 | Right click an item, nothing in hand | click signal -> FUN_100cae1b: not locked; trade open **and Ctrl** -> `TradeAddItem` (row A15); own and not bank -> `N3Msg_UseItem(item, false)`; Gamecode then wears/unwears an item lacking `Can` bit 8, uses one with it (doc §3, GC 10028883) | 100cae1b; 10028883 [NR, doc] | `right_click(kind, own, can, ..)`: `can & 8` -> `Take::Use` (use message); otherwise `Take::One` = **take the item into the inventory** | loot.rs 209-217, 789-803; viewer.rs 3448 | DIFFER |
| A9 | Right click another item with a **bag item** in hand | `UseItemOnItem(held, target)`, accept | 100cc855 (button 2, no Shift) | `crate::gear::held()` is only an **inventory** item. With a bag item in hand (`self.dragging`) the right click falls into `right_click` and uses/takes the clicked item; the held item stays in hand | loot.rs 793-813 | FIXED 2026-11-25 (was DIFFER) within one window: `loot::held_item_right_click`, `used_within`. A bag item in hand right-clicked on an item of ANOTHER window still acts on the item clicked: left (one hand for all containers) |
| A10 | Right click another item with an **inventory item** in hand | `UseItemOnItem(held, target)` | 100cc855 | `used_on = (from, slot)` -> `item_use_ons` with target `(0x6b, handle<<16 | place)`. Whether the target identity the original sends is that, or the list item's own Variant identity `+0x20`, was not compared | loot.rs 807-812; ui.rs 1374-1377 | UNSURE (target identity bytes) |
| A11 | Same with Shift | cancel (combine only inside the inventory view) | 100cc855 (`+0x13c != 0` -> cancel) | no use, `put_down()` | loot.rs 808-812 | MATCH |
| A12 | Right click an **empty cell** / blank area with something in hand | button 2 on an empty cell: cancel the drag. (Right click on blank area with nothing in hand: nothing; the window menu is the frame icon's.) | 100cc855 (`this_00 == 0` -> cancel) | right click where no item is hovered opens the window menu (`menu_at`), even with an item in hand (hand not emptied) | loot.rs 817-822 | FIXED 2026-11-25 (was DIFFER): what is in hand goes back; with nothing in hand the click does nothing (the window's menu by that click, an ADDITION, is gone) |
| A13 | Right button held 0.3 s on an item | menu signal for the item the button went down on: timer = `MouseButtonHoldDelay` (default 0.3, LoginPrefs 19-20 per doc); the click is ended | 10041aa4, 100420fd | 300 ms (`Duration::from_millis(300)`), item = the one pressed on; not opened while an item is in hand (`dragging.is_none() && gear::held().is_none()`) - the original has no such test in what I read | loot.rs 989-1013 | MATCH (the in-hand condition is client-only; original behaviour with a drag active not read) |
| A14 | Item menu entries (own bag) | `LookAtItem`, `CreateReference`, `DeleteItem` (not read-only, not wear page); no `Use` entry; locks only for type 3-6 | 100caf07 | `item_entries(BAG,..)` = LookAt, Reference, Delete | loot.rs 257-266 | MATCH |
| A15 | menu: Look at Item | FUN_100ca5cb -> `itemid://` info page | 100caf07 (callback), FUN_100ca5cb [NR body] | `infos.push(itemref://id/high/ql)` | loot.rs 1023 | MATCH (callback body not read) |
| A16 | menu: Create Reference | `RefMan_t::AddItemRef` (reference board) per doc FUN_100ca55c [NR body] | 100ca55c [NR] | puts the item link into the chat line | loot.rs 1024; DEVIATIONS INV-050 | DIFFER (left by decision in INV-050) |
| A17 | menu: Delete Item | question `ReallyDeleteItemX` (Yes/No), then `N3InterfaceModule_t::DeleteItem` (FUN_100ca997 -> FUN_100ca649) | 100caf07 (adds entry when `+0x15c==0` and not wear); 100ca997/100ca649/DeleteItem bodies [NR] | same question, then `Take::Delete` -> `CharacterAction 0x70` with the item's own identity | loot.rs 1025, 1040-1049; net.rs 11453 | MATCH (message body of `DeleteItem` not read by me) |
| A18 | Shift + left click | up handler: Shift -> qualifier signal -> FUN_100ca730: `itemid://%d/%d/%d/%d` to `InfoViewModule_c::ShowURL` | 1004141b (100414e8), 100ca730 | `ContainerClick::Info` -> `infos.push(...)`. Fires even when an item is already in hand (the same click also lets it go); in the original a click with a drag object is a drop, not a qualifier click | loot.rs 1178-1180, 843-848 | FIXED 2026-11-25 (was DIFFER): no info with something in hand, `loot::container_click` |
| A19 | Ctrl + left click, single item | same as plain click (up handler tests Shift only) | 1004141b (`& 3`) | Ctrl ignored unless stack > 1 | loot.rs 841 | MATCH |
| A20 | Ctrl + left press on a stack (count > 1) | split mode: pointer held, count changes with sideways mouse movement, clamp 1..count-1; release -> pick-up with that count | 10041926; 10041ca4 (clamps) | number box ("Split stack" dialog); OK -> that many in hand | loot.rs 841, 857-862, 1134-1153; invwindows `split_dialog` | FIXED 2026-11-25 (was ASKED, INV-008): the count by the mouse, as the inventory's (I24) |
| A21 | Split in hand, left click an **empty** cell of this bag | place remembered (`+0x150`), `SplitItem(item, count)` | 100cc855 | `Placing::Split` -> `Take::Split`; free place looked for from 0; refusals `Feedback_YouCantSplitThisItem` / `Feedback_CantSplitAsInventoryIsFull` | loot.rs 943-955 | MATCH |
| A22 | Split in hand, left click an occupied cell / another window / the world | cancel, nothing done | 100cc855 | `_ => {}`; `dropped_out` and inventory-take only when `split == 0` | loot.rs 889-897, 957 | MATCH |
| A23 | Ctrl + right click an item with a trade/shop open | `TradeAddItem(partner, item)` (works from any `InventoryViewBase_c`, this one included) | 100cae1b (`GetQualifiers & 0xc`, `__RTDynamicCast` to `TradeView_c`) | container windows have no trade add; only the inventory (`invwindows.rs` 921, `ui.rs` 1720) | loot.rs (absent) | MISSING |
| A24 | Alt + any click | no Alt test anywhere in these handlers | [NF] 10041926, 100419d5, 10041aa4, 1004141b, 100cc6e8, 100cae1b, 100cc855, 100caf07 | no Alt use in loot.rs | loot.rs (grep) | MATCH |
| A25 | Double click | no handler; it is two clicks: first picks up, second is a drop on the same cell = cancel | 100cc855 (same id) | the second release is a "let go on itself" -> `Placing::Cancel` | loot.rs 881, 957 | MATCH |
| A26 | Mouse wheel over the grid | `ScrollView_c::WheelMoved` 0x10142820, snap on (`SetSnapToScrollWheelStep`) per the index summaries | 10142820, 101364ff [NR bodies] | `ScrollView` of the multilist; wheel notch 10, snapping | gui/scroll.rs 276 | UNSURE (original body not read) |
| A27 | Bag item let go on the **inventory window** | `FUN_100cc855` of the inventory view: other container -> `MoveItemToInventory(item)`; only the inventory's list (a place on its title bar is no list) | 100cc855 (`+0x13c==0` branch) | `Take::One` when the pointer is inside the inventory window's whole rect | loot.rs 892-898 | MATCH (window rect vs list area is a small difference) |
| A28 | Bag item let go on **another container's window** (another bag, bank, chest) | `ContainerAddItem(that container, item)` at the cell let go on | 100cc855 (`+0x13c != 0`) | not handled: `Take::Put` is only for an inventory item (`gear::DRAGGED`); bag item let go on another window does nothing (CLIENT_UI 1806 lists "from one bag to another" as not done) | loot.rs 972-985 | MISSING |
| A29 | Bag item let go on the **wear window** (cell) | FUN_100e220f: `IsItemPossibleToWear`, then `MoveItemToInventory(item, inventory id, slot)` (doc; 100e220f not re-read) | 100e220f [NR, doc] | wear window only knows `gear::held` (inventory items); bag item is not offered to it | loot.rs; invwindows.rs | MISSING |
| A30 | Bag item let go over the 3D world | FUN_100cbcb5 (set by `FUN_100cdbe3` in 100cc6e8): terrain drop question/`DropItem`, character -> `UseItemOnCharacter`, else `UseItemOnItem` | 100cc6e8 (FUN_100cdbe3(this,FUN_100cbcb5)); 100cbcb5 body [NR, doc] | `dropped_out` -> `use_on` + `dropitem::let_go` | loot.rs 889-891; ui.rs 1378-1384 | UNSURE (100cbcb5 not re-read) |
| A31 | Inventory item let go (left) on this window | `ContainerAddItem(this container, item)`; cell kept (first free if occupied) | 100cc855 | `Take::Put`; `awaited_cell` when the cell is empty | loot.rs 972-985 | MATCH |
| A32 | Escape | closes when `esc_backpacks` (default true) | 100cd63c, 100cd7c1 (flag + FUN_1004a1b1) | `Ui::close_with_escape`: `esc("esc_backpacks")` closes every own-bag window | ui.rs 759-764 | MATCH (Escape key binding not traced in the original) |
| A33 | Window close button | closes the view **and** sends `UseItem(container identity)` to the server | 100ca891 | window hidden; nothing sent (INV-038: bytes of that use were not identified) | loot.rs 744-827; DEVIATIONS INV-038 | MISSING |
| A34 | Same bag used again from the inventory | the use is sent again; the server's item action 0x66 closes the view (`SlotContainerClosed` 0x100c757b) | 100c757b; GC doc | window toggled locally (INV-038) | ui.rs (bag windows) | DIFFER (INV-038, needs capture) |
| A35 | Bag picked up in the inventory | `CloseContainer` closes its window | 100cc6e8 (inventory type branch) | not done (doc table #7) | invwindows.rs / ui.rs | MISSING |
| A36 | Bag empty | stays (no timer for own) | 100cd53c (needs `+0x14c==0`) | `!view.own` excluded from the 1 s close | loot.rs 610 | MATCH |
| A37 | Hover tooltip | name in rarity colour (`N3Msg_GetRarityColor`) + description when `ItemDescriptionTooltips` | FUN_10042193 [doc, NR] | plain name, QL line, description (300 chars) | loot.rs 732-743 | FIXED 2026-11-25 (was DIFFER): the item lists' own tip, the name in its rarity's colour over the page built for a tip (`gear::tip`), by the window controller's rule |
| A38 | Right click on blank area of the window | no handler in the list (window menu is the frame icon's, row E1) | [NF] in 100cae1b/1004141b | opens the window menu | loot.rs 817-822 | FIXED 2026-11-25 (the ADDITION is gone): a right click beside the items does nothing (FUN_10041aa4); the window's menu is the frame's menu button's (or the rollup page's) |
| A39 | A bag opened twice | one view per identity (`SlotContainerOpened` makes one only if none) | 100c7bef | one `LootWindow` per identity | ui.rs 1355-1362 | MATCH |

#### Rules to port (A, DIFFER / MISSING)

- **A7**: in list mode, left release with a split on an occupied row -> `SplitItem` to the first free place (100cc855 `(this_00 == 0 || !bVar12)`). Change `places::placing` call at loot.rs 915 so `(split>0, Some(_))` is `Split` when `!hand`. SMALL.
- **A8**: right click on own-bag item, nothing in hand: always send the use (`N3Msg_UseItem(item,false)`); Gamecode's rule (doc §3): item lacking `Can` bit 8 -> wear it (page stat 0x4c, slot stat 0x58) / unwear into the first free place, else use. Change `loot::right_click` (loot.rs 209) so the non-use case sends a wear move instead of `Take::One`; needs the wear-move helper the inventory uses (invwindows/net.rs `events.uses` block). MEDIUM.
- **A9**: a right click on an occupied cell with an item of this window in hand: `UseItemOnItem(held, clicked)` (not Shift), cancel with Shift, never use/take the clicked item. In loot.rs 789-814, test `self.dragging` first: send `item_use_ons` with the held bag item's identity `(0x6b, handle<<16|place)` as source, clear `dragging`. SMALL-MEDIUM (DEVIATIONS line 2104 says "not ported").
- **A12**: right click on an empty cell / blank with an item in hand: cancel the drag (`put_down` / `dragging = None`) instead of opening the menu. loot.rs 817. SMALL.
- **A18**: in `show` 843, give `ContainerClick::Info` only when `hands_free`; with an item in hand a Shift click is a plain drop. SMALL.
- **A23**: Ctrl held on a right click with a trade open -> `TradeAddItem(partner, item)` (not locked): add `trade_adds` to the container path (loot.rs right-click block, ui.rs 1720 consumer). SMALL.
- **A28/A29**: bag/bank item in hand let go on another container window -> `ContainerAddItem(that container, item)` (inventory-style `Take::Put` with a bag/bank source identity `(0x6b, handle<<16|place)` / `(0x69, place)`); on a wear cell -> wear check then `MoveItemToInventory(item, inv id, slot)`. Needs the source identity in `Take::Put` (loot.rs 185) and the wear window to look at `loot::item_in_hand`. MEDIUM-LARGE (message for an item named by a bag identity needs a capture).
- **A33/A34**: window close of a type-1 container sends `N3Msg_UseItem(container identity, false)`; server's 0x66 closes the view. Needs the Use action bytes for `(0xc749, instance)` (not traced; INV-038). Needs a capture. LARGE.
- **A35**: picking a bag up closes its window. `invwindows.rs` `events_of` (`Click::TakeUp`) -> hide the `bag_windows` entry. SMALL.
- **A37**: tooltip = name in rarity colour + description; use `gear::tooltip` as the inventory. SMALL.

### 2. Other container: chest (0xc749 not mine), corpse (0xc76a = the loot window), organization contract (0xdeaa) (`TempContainer_<n>`; `+0x14c`=0, sorted by name)

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| B1 | Left click an item (hands free) | pick-up -> FUN_100cc6e8: non-own -> `MoveItemToInventory(item)` at once (no drag); the item stays in the list until the server answers | 100cc6e8, 10041568 | `ContainerClick::Take` -> `Take::One` -> `keys.take`; the viewer **removes the item from the window immediately** (so a fast second click hits the next item) | loot.rs 856; viewer.rs 3426-3434 | MATCH (optimistic removal is a client detail) |
| B2 | Left click while an inventory item is in hand | the release is a drop: `ContainerAddItem(this container, item)` (chest, corpse and 0xdeaa alike) | 100cc855 | chest: `Take::Put`; corpse/overflow excluded; organization contract filtered out at the end | loot.rs 975; 1157 | DIFFER (corpse and contract not sent; server probably refuses, not checked) |
| B3 | Right click an item, chest | click signal: `+0x14c==0` -> `MoveItemToInventory` | 100cae1b | `right_click(BAG, false,..)` -> `Take::One` | loot.rs 209-217 | MATCH |
| B4 | Right click an item, **corpse** | nothing (feature flag 0x400 set on the corpse list) | 1004141b (10041529), 100ce55d (100cea46) | `right_click_takes(CORPSE) == false` -> `None` | loot.rs 486, 211 | MATCH |
| B5 | Right click another item with an inventory item in hand | `UseItemOnItem(held, target)`, any container incl. corpse | 100cc855 | `used_on` (all containers, also corpse) | loot.rs 807-812 | UNSURE (target identity for a corpse item `(0x6b, ..)` not compared; corpse items' own identity bytes not read) |
| B6 | Right button held 0.3 s | item menu: LookAtItem, CreateReference, DeleteItem (+ `Looter` on a corpse in a team) | 100caf07 | as above; no Delete for the contract | loot.rs 257-266 | MATCH (contract: DIFFER, below) |
| B7 | menu: Delete Item on the organization contract | listed (not read-only, not wear) | 100caf07 | not offered ("identities not read") | loot.rs 259-261 | DIFFER |
| B8 | menu: `Looter` submenu on a corpse | shown when `+0x140==0xc76a` and `N3Msg_IsInTeam`; entries (literal strings) `Free For All`, `Random Looter`, then, not a raid: all `GetTeamMemberList(-1)` members; raid: `Team %d` (1-based) submenus of that team's members; a member enabled only if `N3Msg_isDynelInTree` | 100caf07 (strings and loops) | `Looters` submenu: "Free for all", "Random looter", members / `Team n` (enabled while the client knows the character). Labels differ in letter case ("Free for all", "Random looter") | loot.rs 245-249, 271-285 | DIFFER (labels) |
| B9 | menu: choosing a looter | FUN_100ca7ab: `N3Msg_SetLootAccess(container identity, item, looter, do_roll)` (Free For All: roll false, identity 0; Random: roll true; member: identity of member) | 100ca7ab, 100caf07 | `net::raid_command(5, [corpse, (place&0xffff, roll), looter])`; wire layout of GC 0x100184cf not compared here | loot.rs 1028-1032, 340-348 | UNSURE (message bytes) |
| B10 | Shift + left click | info page (`itemid://`) | 1004141b, 100ca730 | `ContainerClick::Info` | loot.rs 1178 | MATCH (also fires with an item in hand: see A18) |
| B11 | Ctrl + left click on a stack (count > 1) | enters split mode (message UI) and on release takes the whole stack (`count` ignored for non-own) | 10041926, 100cc6e8 | plain take; no split UI (`split = ctrl && view.own`) | loot.rs 841 | DIFFER (cosmetic: same end result) |
| B12 | Alt, Ctrl+right (trade), wheel | Alt [NF]; Ctrl+right: trade-add on a chest item (not corpse: no click signal); wheel as A26 | 100cae1b | Ctrl+right: none | loot.rs | MISSING (Ctrl+right on a chest item with a trade open; same as A23) |
| B13 | Empty chest / corpse | list change signal / creation arms a 1 s timer -> close (vtable +0xfc) | 100cd53c..100cd5b0, 100ca8d9 | 1 s after the list is empty -> `open = false`. Client's rule is "not bank/organization/overflow/own", so an unknown identity type (the original only closes 0xc76a and 0xc749) would also close | loot.rs 610-621 | MATCH for corpse/chest |
| B14 | Escape | corpse -> `esc_corpses`, chest -> `esc_chests` (default true), contract reads none | 100cd719, 100cd768 | `escape_option("loot", kind)`: CORPSE/BAG only, others none | ui.rs 428-438, 750-757 | MATCH |
| B15 | Window close button | closes and sends `UseItem(container)` | 100ca891 | hides only (not the bank) | loot.rs | MISSING (same as A33; needs the bytes, a capture) |
| B16 | Right click item in blank area, press-drag | nothing | 1004141b | nothing (`click_only`); blank right click opens window menu (ADDITION, row E1) | | MATCH |
| B17 | Auto-loot of a fresh corpse | [NF] in the container window | [NF] 100ccefe/100ce55d | `fresh_corpse` + `keys.auto_loot` sends a take for every item when the setting is on | net.rs 11494-11500 | ASKED (kept on the user's word of 2026-11-25: asked "Keep the five extras you asked for earlier, which the original does not have?", the answer was "Keep them"): auto-loot, the `auto_loot` setting, off by default (INV-079) |
| B18 | "Take all" button | none | [NF] (doc; INV-080) | removed 2026-11-19 | DEVIATIONS INV-080 | MATCH |
| B19 | Window name / number | `TempContainer_%u`, lowest free number FUN_100cb7f9; title = container name | 100cd2b2, 100cd2c1 | `window_name`, `free_number` | loot.rs 159-173; ui.rs 1358 | MATCH |
| B20 | Items sorted by name on opening; Auto Arrange default | `SetListSortColumn(1,0)` + `FUN_10041258(1)` for non-own except reclaim | 100ceaab..100cea8d | `first_sort` -> `Some((1,0))` | loot.rs 497-499 | MATCH |

#### Rules to port (B)

- **B2**: ContainerAddItem to a corpse / contract on a left click with an item in hand. The original does send it; the server decides. Remove `CORPSE | OVERFLOW` from the exclusion at loot.rs 975 (and the contract filter at 1157 once its identity is known). SMALL, but the reply of a live server is not known; try before shipping.
- **B7**: show `Delete` for the contract (needs the item's identity for `DeleteItem`). `item_entries` loot.rs 259. MEDIUM (needs reading how the contract's items are named).
- **B8**: change the literals to `Looter`, `Free For All`, `Random Looter` (exactly as the original's strings) in `Entry::label` (loot.rs 245-247). SMALL.
- **B11**: no change needed for the result; if wanted, show the split box and ignore the count for a non-own container. SMALL (cosmetic).
- **B12**: see A23.
- **B15**: see A33.

### 3. Bank window (`bank_window`, identity 0xdead; `+0x14c`=1, 102 places, not auto-sorted)

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| C1 | Left click an item (hands free) | pick-up: own flag set -> drag object, item on the pointer | 100ce94e, 100cc6e8 | `container_click(.., own || BANK, ..)` -> `TakeUp` | loot.rs 842 | MATCH |
| C2 | Right click an item | click signal: `identity == 0xdead` -> `MoveItemToInventory` (take out) | 100cae1b | `right_click(BANK, ..)` -> `Take::One` (bank branch in net: item `(0x69, place)` to inventory `0x6f`) | loot.rs 209-217; net.rs 11510-11556 | MATCH |
| C3 | Held item let go on an empty bank cell | `MoveItem` (client side); no message | 100cc855 | `places::put` in the bank (placed by hand) | loot.rs 916-921 | MATCH |
| C4 | Held bank item let go on another bank item | `JoinItems(target, held)`, else swap and the other picked up | 100cc855 | swap works (placed); **Join not done** (`Placing::Join ... if view.own`, bank excluded: "how its stacks' joining goes on the wire is not known") | loot.rs 922-925 | MISSING (join) / MATCH (swap) |
| C5 | Ctrl + left on a bank stack, then an empty bank cell | split mode, then `SplitItem` | 10041926, 100cc855 | `split = ctrl && view.own` is false for the bank: no split | loot.rs 841 | MISSING |
| C6 | Bank item let go on the inventory | `MoveItemToInventory`, accepted only on success | 100cc855 (type-0 branch) | `Take::One` when the pointer is within the inventory window | loot.rs 892-898 | MATCH |
| C7 | Bank item let go on another container (bag, chest, corpse) | message `ItemsDraggedFromBankCanOnlyBeDroppedToBank`, cancel | 100cc855 (first branch) | `feedback.push("ItemsDraggedFromBankCanOnlyBeDroppedToBank")` on left release over another container window; the drag ends | loot.rs 965-971 | MATCH |
| C8 | Bank item let go on the wear window | `FUN_100e220f`: bank message, cancel (doc; body not re-read) | 100e220f [NR, doc] | no message, nothing | | MISSING |
| C9 | Bank item let go on the world | not read for a 0xdead source | 100cbcb5 [NR] | `dropped_out` -> `use_on`/drop with identity `(0x6b, handle<<16|place)` built even for the bank (handle 0): bank identity bytes wrong | loot.rs 889-891; ui.rs 1378-1384 | UNSURE |
| C10 | Inventory item let go on the bank (left) | `ContainerAddItem(bank, item)` (cell kept) | 100cc855 | `Take::Put`, bank id = own character id; `awaited_cell` | loot.rs 975-985; net.rs 11502-11509 | MATCH |
| C11 | Right click on a bank item with an inventory item in hand | `UseItemOnItem(held, target)` | 100cc855 | `used_on` target `(0x69, place)` | ui.rs 1375 | UNSURE (target identity) |
| C12 | Right button held 0.3 s | item menu Look / Reference / Delete | 100caf07 | same, `Delete` -> `(0x69, place)` identity | loot.rs 257-266 | MATCH (Delete on the bank: the identity the original sends was not read; client's own) |
| C13 | Shift + left click | info | 100ca730 | `Info` | | MATCH |
| C14 | Escape | none for the bank | 100cd5ba..100cd7c1 | none (`escape_option` -> `None`) | ui.rs 437 | MATCH |
| C15 | Close button | window closed + `UseItem(0xdead)`; Gamecode's bank branch closes the bank (doc: CharacterAction 0x6a) | 100ca891; GC 10028883 [NR] | `bank_shut` -> `net::close_bank()` -> `CharacterAction 0x6a` | loot.rs 440; ui.rs 1389 | MATCH (0x6a body from doc) |
| C16 | Empty bank | stays | 100cd53c (own flag) | stays (`BANK` excluded) | loot.rs 610 | MATCH |
| C17 | Window menu | List Mode, Auto Arrange with the own-flavour ("Don't Auto Arrange"), no Rename | 100cbace | `kind = Arrange::Own` for `placed()` (own or bank), no Rename | loot.rs 1058-1064 | MATCH |
| C18 | Ctrl + right (trade), Alt, wheel | as A23, A24, A26 | | | | MISSING (Ctrl+right) |
| C19 | Out of the bank only into the inventory | rule C6/C7 | 100cc855 | rule implemented for the inventory and other container windows; not the wear window, not the world | | DIFFER (see C8/C9) |

#### Rules to port (C)

- **C4/C5**: JoinItems and SplitItem inside the bank. Needs the wire form of bank item identities for the join/split messages `(0x69, place)` (the client's own `n3::join_items`/`split_item` accept arbitrary identities, so `Take::Join`/`Split` with a bank identity is the change at loot.rs 924, 841, net.rs 11439-11451). MEDIUM, one live try.
- **C8**: bank item over the wear window: push the same feedback. The wear window must know a bank item is in hand (`loot::bank_item_in_hand`). SMALL.
- **C9**: do not build a `(0x6b, ..)` identity for a bank item; check what 100cbcb5 does with a 0x69 source before porting. MEDIUM.

### 4. Reclaim window (`reclaim_window`, identity 0xdeae; `+0x14c`=0, not sorted, no 0x400)

The client has no reclaim window: no code creates a container view for 0xdeae (only `first_sort` knows the constant, loot.rs 491/498), `/reclaim` is unsupported (DEVIATIONS CHT-055).

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| D1 | Open (`/reclaim` at a terminal, `N3Msg_ToggleReclaim`) | opens `reclaim_window` (`Reclaim.xml`) | 100cd16d, 100ce55d | not opened | DEVIATIONS CHT-055 | MISSING |
| D2 | Left click an item | non-own -> `MoveItemToInventory` | 100cc6e8 | n/a | | MISSING |
| D3 | Right click an item | `MoveItemToInventory` (no 0x400, `+0x14c==0`) | 100cae1b, 1004141b | n/a | | MISSING |
| D4 | Item let go on it | `ContainerAddItem(0xdeae, item)` | 100cc855 | n/a | | MISSING |
| D5 | Menus / Escape | item menu Look/Reference/Delete; window menu List Mode + Auto Arrange in the **own flavour** (`own || 0xdeae`); no Escape option | 100caf07, 100cbace, 100cd5ba | n/a | | MISSING |
| D6 | Empty | does not close | 100cd53c (type not 0xc76a/0xc749) | n/a | | MISSING |
| D7 | Close | `UseItem(0xdeae)`; server closes with action 0x6e | 100ca891; doc/INV (GC FUN_10046e7a) | n/a | | MISSING |

Rule to port: build the window from the server's 0xdeae list through the existing `LootView`/`LootWindow`, with `placed()` false but Arrange flavour Own (loot.rs 1058: `kind = if view.placed() || identity == RECLAIM`), no sort (`first_sort` already), no empty-close (loot.rs 610 includes it today: add `RECLAIM` to the exclusion), no Escape; click and right click take (already the non-own path), drops via `Take::Put`. LARGE (it needs the terminal open path and the list message captured).

### 5. Window menus (container windows)

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| E1 | Open the menu | the frame's menu icon (`SlotIconMenuOpening` 0x1015b34f appends the frame's own entries after the view's) | 100cbace (view part), 1015b34f (index summary: a fade toggle and a close entry) | frame menu button (`chrome::frame_menu`) **and** a right click on a blank area of the window | loot.rs 817-822, 1056 | FIXED 2026-11-25 (the ADDITION is gone): the frame's menu button only |
| E2 | `ListMode` entry | first entry, checked when in list mode (`Check(layout==1)`) | 100cbace, 1004157f | "As a list" (game text `ListMode`), marked when `in_list` | loot.rs 1060-1061 | MATCH |
| E3 | `AutoArrange` submenu | second entry; disabled in list mode (`Enable(false)` when `GetLayoutMode==1`); built by FUN_10041637 with flag = `own || 0xdeae`: "Don't Auto Arrange" (flag only), separator, "Unsorted" + "By <column>" entries (checked by the grid sort column), separator, "Sort Ascending" (checked by order). Exact enable/check logic of each entry not re-read | 100cbace, 10041637 | `arrange_menu(arranged, kind)`: same entries and order, disabled in list mode; `kind` from `placed()` (own or bank), reclaim absent | loot.rs 525-542, 1056-1062 | MATCH (structure; per-entry enable rules not compared, reclaim n/a) |
| E4 | `Rename` entry | third entry, only for an own backpack (type 1 and `IsItemMine`); starts FUN_100cb852 | 100cbace (100cbc29..100cbc9f) | only when `view.own` | loot.rs 1062-1065 | MATCH |
| E5 | Rename dialog | `TextInputDialogForm_c` "Enter backpack name.", initial text = the container's current name; buttons `MsgBox_OK`, `Reset2Default`, `MsgBox_Cancel`; OK -> `SetBackpackName(identity, text, false)`, Reset -> empty name, Cancel -> nothing | 100cb852, 100ca90f | same texts and three buttons, Enter = OK; name kept in `backpacks.ini` | loot.rs 1105-1133 | MATCH |
| E6 | Frame's own entries | fade toggle (`SlotFadeMenuEntry`) and close (`SlotCloseButton`) | 1015b34f (index only) [NR body] | `Close` only; no fade (WIN-109: "client's windows do not fade") | loot.rs 1066-1068 | MISSING (fade entry; reading of 1015b34f not done) |
| E7 | Choosing an entry in list mode, header click, column resize | `MultiListView_c` / `ColumnHeaderView_c` | [NR] | `multilist` header sort and widths kept in `Keeper` | loot.rs 654, 828 | UNSURE (original not read) |
| E8 | Menu saved state | `listview_config`, `item_position_map`, `container_name` per container file | 100ccefe (strings) | `crate::layout::tab`, `places.rs`, `backpacks.ini` | loot.rs 639-644 | MATCH in kind (files differ) |

### 6. Counts

MATCH 49, DIFFER 13, MISSING 19, ADDITION 3, ASKED 1, UNSURE 8 (total 93 rows; a row whose verdict reads "DIFFER / ADDITION" or "MISSING (join) / MATCH (swap)" is counted under its first word).

### 7. Not read / not found

- `MultiListView_c::MouseUp` 0x10133d04 (whether a press-move-release really signals nothing) and `ScrollView_c::WheelMoved` 0x10142820: relied on the doc and the client's own notes.
- Gamecode: `N3Msg_UseItem` 0x10028883 (what it does for identity `(0xc749, instance)` / `(0xc76a, instance)` when the window closes), `N3Msg_JoinItems` 0x1001bb2a, `N3Msg_SetLootAccess` 0x100184cf, `N3InterfaceModule_t::DeleteItem`, `MoveItemToInventory` for a corpse item: not re-read. So what a corpse/chest/bag window close puts on the wire, and whether the client's join test matches the original, are open.
- FUN_100cbcb5 (drop on the world), FUN_100e220f (wear drop), FUN_100ca5cb / FUN_100ca55c / FUN_100ca997 / FUN_100ca649 (menu callbacks), FUN_10042193 (tooltip), the pointer shape in FUN_10041ca4 beyond the Shift test: not re-read.
- Whether the `esc_*` flag binds the close to the Escape key: the flag read and FUN_1004a1b1 = close are confirmed; the Escape dispatch was not traced.
- The item list's behaviour while a drag object is active and the right button is held (menu timer): not read.
- Locked items (`FUN_1003d409`, EquipmentLocks): the "locked" tests in 100cc6e8 / 100cae1b / 100cc855 exist, but I found no way to lock an item in a container window (`LockItem` is only for types 3-6 per 100caf07), so I did not make rows for them. The client has no locks in containers.
- Overflow (identity 0x6e) window: the brief did not list it; the client treats it as a non-own container that does not close. The original's view for 0x6e (non-mine, not 0xdeae) would be sorted, with `+0x14c`=0, and not closing (not 0xc76a/0xc749): consistent, not otherwise compared.
- Middle button: 100cc855 returns without accepting; not compared with the client.
- Pointer shapes over container items (usable-item cursor): not compared.
- Dragging an item out of the window past its edge, `Escape` with an item in hand (original), keyboard focus: not read.

## Shop, player trade, NPC chat and trade


Original = GUI.dll decompile (`exports_gui`), read with gf.py / calls.py / gdis.py / gxref.py / vtable.py. Client = `crates/mao-client/src` as on disk 2026-11-25.
Marks: **[V addr]** read at that address, **[NF]** looked, not found, **[NR]** not read. Nothing here rests on a note saying "confirmed live".

### How the original's trade window is wired (read first)

`TradeView_c` ctor = FUN_100e121e (`SlotStartTrade` 0x100c7a40 news 0x220 bytes, calls it). Mode at `+0x1a4`: 0 PlayerTrade, 1 ShopTrade, 2 ShopBuy (view names picked in FUN_100e1074 [V 100e1074: 0x101bfa7c/88/94]). `+0x1a8` (drop allowed) is 1, put to 0 for mode 2 [V 100e121e]. Identities: `+0x1c8` = the shop / partner, `+0x1d0` = own, `+0x1d8` = third.

Lists (all `ItemContainerView_c`, FUN_100ce55d, kind 2 = trade list, except the shop's own which is kind 1; column flags 0xf shop stock / 0xb the others (+Price only in shop modes), **list feature flags 6 (shop stock, partner/bought) and 0xe (own/sold)**) [V 100e07b9 push 6; 100e0942 push 6; 100e0b00 push 0xe]:

| list | field | made in | connected handlers [V] |
|---|---|---|---|
| ShopInventoryDock (modes 1, 2 only, `+0x1e8`) | stock | FUN_100e07b9 | `+0x2e0` click -> FUN_100dffe8 (`N3Msg_TradeAddItem(+0x1c8, item)`); `+0x2e4` qualifier click -> FUN_100e0096 (`shopitemid://%d/%d` to InfoView `ShowURL`) |
| PartnerInventoryDock (`+0x1e0`) | the partner's / "bought" items | FUN_100e0942 | `+0x2e4` -> FUN_100e002d (`itemid://%d/%d` page) **always**; `+0x2e0` -> FUN_100dffa3 (`N3Msg_TradeRemoveItem(+0x1c8, item)`) **only when mode != 0** (`cmp [esi+0x1a4], edi` with edi = 0 at 100e0a87) |
| OurInventoryDock (`+0x1e4`) | own / "sold" items | FUN_100e0b00 | `+0x2e0` -> FUN_100dff6d (`N3Msg_TradeRemoveItem(+0x1d0, item)`); `+0x2e4` -> FUN_100e002d |
| all three, only when `+0x1a8 != 0` (modes 0, 1) | | ctor 100e1799..100e17f7 | `+0x2dc` drop -> FUN_100e015f |

No handler is connected to `+0x2d8` (pick-up) or `+0x2e8` (menu) on any trade list **[NF in all three init functions and the ctor]**.

What the list's mouse code does with flags 6/0xe (`ItemListViewBase_c`, FUN_1004141b old scheme; the trade lists never go through FUN_10041248 so the new scheme is off [V INVENTORY_ORIGINAL.md section 0/1, which I cross-checked in the disassembly 10041537..1004157d]):
- left button up, **Shift not held** (`GetQualifiers & 3`): if flags & 0xc0 select; **else if flags & 2 -> click signal `+0x2e0`** (flags 6 and 0xe have bit 2) else pick-up. So for the trade lists a plain left click is the *click* signal, unlike the inventory (flags 0x15, bit 2 clear) where it is a pick-up.
- left up with Shift -> qualifier signal `+0x2e4`. Ctrl/Alt are not tested on the way up.
- right up (not a corpse list) -> click signal `+0x2e0`, no qualifier test.
- right down with a click listener -> hold timer (`MouseButtonHoldDelay`, FUN_10041aa4); on expiry FUN_100420fd ends the click and raises `+0x2e8` (nobody listens): letting go afterwards does nothing.
- left down with **Ctrl held on a row whose count field > 1** -> split mode (FUN_10041926); the release raises the pick-up signal `+0x2d8` (nobody listens) and no click is raised (FUN_100419d5).
- Alt: no test anywhere in these handlers **[NF]**.

Other connections in the ctor [V 100e121e, 100e0c89]: GlobalSignals `+0xd8` -> FUN_100e04e2 (shop: cost views), `+0xdc` (int) -> FUN_100e074d (partner credits / shop costs), `+0xe0` (bool) -> FUN_100e0260, `+0xe8` -> FUN_100e0f5b (the "Are you sure" dialog), `GUIModule` Escape signal -> FUN_100e00ff **only if** pref `esc_trades` (mode 0) / `esc_shops` (modes 1, 2) is true (both true in LoginPrefs.xml lines 66-67); `OurCashView` text-renderer signal (`+0x84`) -> 0x100e011c (start 1,000,000 us timer on `+0x200`, `Start(1000000,0,1)`); timer -> FUN_100e0231 (`N3Msg_TradeSetCash(atol(text))`); AcceptButton click (`+0x148`) -> FUN_100e0622; DeclineButton click -> FUN_100e00ff (stop timer, `N3Msg_TradeAbort(true)`). Vtable `+0xfc` (the window's close) = 0x100e015a = thunk of FUN_100e00ff [V vtable.py TradeView_c], i.e. closing the window = TradeAbort(true). Vtable `+0xf8` (the window's menu) = FUN_100e03c7. `EscapePressedMessage` 0x100294d2 just emits the GUIModule signal [V].

Client files: `shop.rs` (`ShopWindow::show`, `row_click`, `accept_player`, `cash_timed`), `net.rs` ~10812-10850 (ShopCommand to messages), `invwindows.rs` 893-935 (Ctrl+right), `ui.rs` 1573 and 1700-1725, `escape_option` ui.rs 427-470, `npcchat.rs`.

### Table 1: the shop window (modes 1 and 2) and what is shared with the player trade

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| 1 | Plain left click on a stock row | click signal `+0x2e0`: `N3Msg_TradeAddItem(+0x1c8, item)`, then list refresh (FUN_100ce44c). One of the item per click | 100e07b9 (push 6, connect `+0x2e0`), 1004141b (10041537-10041568), 100dffe8 | `Buy(index)` -> `trade` action 5 `[booth, 0x6f, index]` | shop.rs `row_click` 120, `show` 710-725, net.rs 10828 | MATCH |
| 2 | Right click (released before 0.3 s) on a stock row | same click signal -> same add | 10041526-10041535, 100dffe8 | `secondary_clicked` + `row_click(true,..)` = Act -> Buy | shop.rs `show` 640-655, 726 | MATCH |
| 3 | Right button held 0.3 s on a row | click ended, menu signal `+0x2e8` raised, no listener: nothing, and the release does nothing | 100420fd, 10041aa4; no `+0x2e8` connect (NF) | `held >= HOLD` -> `Nothing` | shop.rs `row_click` 120-126 | MATCH |
| 4 | Shift + left click on a stock row | qualifier signal `+0x2e4` -> `shopitemid://%d/%d` page in Info | 100e0096, 100e07b9 | `Page` -> `shopitemid://low/high/ql/price` to `info_url` (own URL form with the price; the original's two numbers are type/instance of the stock item) | shop.rs 735-740 | MATCH (URL contents UNSURE) |
| 5 | Shift + right click on a row | right-up raises the click signal with no qualifier test: same as plain right (adds / removes) | 10041526-10041535 | `row_click(true, shift, ..)` ignores Shift for the right button = Act | shop.rs 120-126 | MATCH |
| 6 | Ctrl + left click on a row whose count > 1 | down starts split mode, no click is raised; release raises pick-up (unconnected): **nothing happens**. On a count-1 row the Ctrl is ignored and it is an ordinary click | 10041926 (`FUN_1003d484(2) > 1` and `GetQualifiers & 0xc`), 100419d5 | Ctrl is not looked at: always Act | shop.rs `row_click` (no ctrl arg) | DIFFER (stacks in the bought/sold lists only; stock rows have count 1) |
| 7 | Alt with any click | no test | [NF] in 1004141b, 10041aa4, 10041926, 100419d5 | Alt ignored | shop.rs | MATCH |
| 8 | Double click on a row | no handler: two clicks, two actions | [NF] (no double-click signal on `ItemListViewBase_c`) | each `Clicked` event acts | shop.rs 704-712 | MATCH |
| 9 | Press, move to another cell, release | nothing: the list signals only when up cell = down cell | INVENTORY_ORIGINAL.md (0x10133d04, not read by me) | `multilist::click_only` | shop.rs 700-702 | MATCH (leans on the doc) |
| 10 | Click on a bought row (`PartnerInventoryDock`, shop modes) | `N3Msg_TradeRemoveItem(+0x1c8, item)` + refresh | 100e0942, 100dffa3 | `Remove(bought[i].item)` -> action 6 | shop.rs 726-731, net.rs 10840 | MATCH |
| 11 | Click on a sold row (`OurInventoryDock`) | `N3Msg_TradeRemoveItem(+0x1d0, item)` | 100e0b00, 100dff6d | `Remove(sold[i].item)` | same | MATCH |
| 12 | Shift + left on a bought / sold row | qualifier -> `itemid://%d/%d` page | 100e002d | `page_of` -> `itemref://` page | shop.rs 733-756 | MATCH |
| 13 | Let go a held inventory item on any of the three lists (modes 0, 1; not mode 2) | FUN_100e015f: drag mime `inventory/item`, `container_id` and `item_id` present, `split_count` absent or 0 -> `N3Msg_TradeAddItem(+0x1d0, item)`, `Accept(2)`; otherwise nothing accepted. Any mouse button; no cell test | 100e015f (read), connects at 100e1799-100e17f7 | `dragged != MAX && !buy_only && dropped` -> `Sell(slot)` for any held item, over the three docks | shop.rs 673-679 | MATCH (checked 2026-11-25: the drop asks `gear::letting_go`, which is false for a split in hand) |
| 14 | Ctrl + right click on an item in the **inventory window** while a trade is open | FUN_100cae1b: item not locked, a trade view exists (FUN_1002fcb7 cast to `TradeView_c`) and `GetQualifiers & 0xc` -> `N3Msg_TradeAddItem(tradeview+0x1d0, item)`; else normal use | 100cae1b | `shop::trading() && ctrl` -> `trade_adds` -> `Sell(slot)` | invwindows.rs 918-921, ui.rs 1573, 1717-1722, net.rs 10834 | MATCH |
| 15 | Same Ctrl + right click on an item in a bag, the bank, a corpse, a chest or a wear page | the handler is shared by every `InventoryViewBase_c` list (not own-checked): adds to the trade too | 100cae1b (no list-kind test), INVENTORY_ORIGINAL.md section 0 | only the inventory window has the code; `grep trade` finds nothing in loot.rs / bag code / the wear branch of invwindows.rs | invwindows.rs (only 920), loot.rs 791/818 | MISSING |
| 16 | The same Ctrl + right click when the shop is the buy-only `ShopBuy` | no test in 100cae1b; `TradeAddItem` itself decides (Gamecode) | 100cae1b; Gamecode 0x1001925e not read in full | blocked in the client (`!(buy_only && partner none)`) | ui.rs 1717-1719 | UNSURE (client refusal is probably what Gamecode does; not read) |
| 17 | Accept button in a shop | FUN_100e0622: stop timer, **`View::Enable(+0x1f4, false)` (Accept button greyed)**, `N3Msg_TradeAccept` (mode != 0). The bool signal `+0xe0` = false re-enables it | 100e0622, 100e0260 | sends `Accept`; the button is never disabled | shop.rs 776-777 | DIFFER |
| 18 | Decline button | stop timer, `N3Msg_TradeAbort(true)` | 100e00ff, connect at 100e0e6a-100e0e7a | `is_open = false` -> `Close` -> `trade_decline` | shop.rs 779, 818-824, net.rs 10812 | MATCH |
| 19 | Window close (frame X) | vtable `+0xfc` = 0x100e015a = abort | vtable.py TradeView_c, 100e015a | frame close sets `is_open=false` -> `Close` | shop.rs 818-824 | MATCH |
| 20 | Escape | GUIModule Escape signal -> FUN_100e00ff, only if `esc_shops` (modes 1, 2) / `esc_trades` (mode 0); both default true | 100e1699-100e1742, LoginPrefs.xml 66-67, 100294d2 | `escape_option("Shop", .., trading)` picks `esc_trades` / `esc_shops`, closes -> Close | ui.rs 427-470 | MATCH |
| 21 | Walking away | no slot on the character's place in the ctor | all connects of 100e121e listed above (NF for any movement signal) | rule removed (INV-001 done) | net.rs ~10800 comment | MATCH |
| 22 | The two cash fields in a shop | FUN_100e04e2: `PartnerCashView` = cost of the shop's side (bought items); `OurCashView` = **own cost minus shop cost** (sold - bought), written with `FormatNumeric`, **green (0x44dd44) when >= 0, red (0xdd4444) when < 0** | 100e04e2 (disasm 10e052a sub, 100e05f6-100e0609) | `PartnerCashView` = sum of bought prices; `OurCashView` = sum of the sold prices alone, always green, hand-painted over the field | shop.rs 600-612 | DIFFER |
| 23 | Window menu (vtable `+0xf8`) | two entries: `GTK_Container_ListMode` and `GTK_Container_AutoArrange` (disabled when the list is in list mode) for the shop list (modes 1/2) or the partner list (mode 0) | 100e03c7 | "As a list" (marked) and "Order" submenu, dead in list mode, for the same list | shop.rs 760-790, `config` 505 | MATCH |
| 24 | Extra menu entry "Close" and opening the menu by a right click beside the rows | none in 100e03c7; how the frame opens the window menu: [NR] | 100e03c7 | `Entry::text(close_entry)`; right click off the rows opens the menu | shop.rs 646-651, 770-776 | FIXED 2026-11-25 (the ADDITION is gone): beside the rows the right button does nothing. The "Close" entry is the frame's own "Close Window", which the original appends too (FUN_1015b34f); its "Autofade Window" before it is not in the client's window menus (left) |
| 25 | Hover tooltip on a row | no tooltip code in the TradeView functions; the list's own item tooltip: [NR] | [NR] | stock row shows its description; bought/sold rows none | shop.rs 681-685 | UNSURE |
| 26 | Mouse wheel over a list | the list's `ScrollView_c` (vertical bar mode 1, `SetVScrollBarMode`) [NR for the wheel code] | 100e07b9 (bar modes) | not examined in shop.rs (multilist's generic wheel) | multilist.rs | UNSURE |
| 27 | A quantity question for stacks | none: Add/Remove move the whole entry. No dialog function in the TradeView functions | [NF] | none | shop.rs | MATCH |
| 28 | Refusals before an item goes in (Gamecode `N3Msg_TradeAddItem` 0x1001925e): not-tradable item feedback, and the "cannot afford" line built from LDB text 0x6e with the amount and the stat name (shop mode, pay stat) | read the head of the function: the checks exist and raise feedback instead of sending the add | Gamecode 0x1001925e (partly read, not traced to each text) | none: the add is always sent (NPC-014 says "not ported: refusals before an add") | net.rs 10828-10836 | MISSING |
| 29 | Left click on a row while an item is held in hand | with a drag object, a mouse-up over a list raises the list's *drop* signal (-> sell the held item), not the click | INVENTORY_ORIGINAL.md section 2 (10133d04, 10157f13), not re-read | `command = Sell` is set in the drop test and then overwritten by `Buy`/`Remove` from the same release if `Clicked` also fires; whether `click_only` rows fire `Clicked` under a drag was not checked | shop.rs 673-679 then 722-730 | UNSURE |
| 30 | A "Trading with <name>" line at the start of a trade with a player | ctor emits GlobalSignals `+0x17c` with LDB text 0x2713 (`ShopView_TradingWith`) fed with the partner's name (mode 0) | 100e13f9-100e1508 (calls.py) | no such line found (`grep` for the text id / name in net.rs, feedback.rs, chatwindow.rs) | - | UNSURE (where the signal lands, chat or a status line, was not read) |

#### Port rules for Table 1 (DIFFER / MISSING)

- **#6 Ctrl + left on a stack.** Rule: on the left button's press over a row with count > 1 and `GetQualifiers & 0xc`, take no click; on release do nothing (pick-up has no listener). With count 1 Ctrl does nothing special. Change `shop::row_click` to take `ctrl` and the row's count, and have the `Clicked` loop at shop.rs ~704 drop the event when ctrl && count > 1. SMALL. Same change for the NPC bar (N5).
- **#13 drop with a split.** Rule: the drop is taken only if the held item's split count is 0; otherwise nothing is accepted and the drag is left to the normal cancel. Change: add `crate::gear::split_count() == 0` to the `dragged != u32::MAX && !buy_only && dropped` test (shop.rs ~675), as `npcchat::trade_window` already does. SMALL.
- **#15 Ctrl + right click in other lists.** Rule (FUN_100cae1b): item not locked, trade view up, Ctrl bits 0xc -> `TradeAddItem(own, item)` for the lists of bags, bank, corpse/chest and wear pages as well. Change: raise a `trade_adds`-style event from the bag/loot windows (loot.rs 791/818) and the wear branch (invwindows.rs 794), mapped to the right `Sell` source (a bag item is `(0x6b..)` not a `0x68` inventory slot: needs the message form for non-inventory items, which the capture-based `Sell(slot)` does not have). MEDIUM (needs the wire form for items outside the main inventory; may need a capture).
- **#17 / P3 Accept disables the button.** Rule: in 100e0622, after the cash step, `Enable(false)` on `AcceptButton`; `Enable(true)` again when GlobalSignals `+0xe0` carries false (100e0260). Change: a `accept_enabled` flag in `ShopWindow` set by Accept and cleared by whatever the client maps to `+0xe0` (see P8); draw the XmlWindow button disabled. SMALL for the disable; MEDIUM for the re-enable (needs the emitter, see Not read).
- **#22 cash fields.** Rule: shop modes only; `PartnerCashView` text = FormatNumeric(sum of bought prices); `OurCashView` text = FormatNumeric(sum sold - sum bought), default colour 0x44dd44 if >= 0 else 0xdd4444. Change: `ShopWindow::show` 600-612 (`for (field, rows) in [...]`): replace the painter text by `textinput::set_text` on the two fields with these numbers (the player-trade path already does `set_text` for `PartnerCashView`), colour red when negative. The prices are the client's own port of `VendingMachine_t` (NPC-013), so the numbers are only as right as that port. SMALL.
- **#28 refusals.** Rule: port the checks at the head of Gamecode `N3Msg_TradeAddItem` (feedback instead of send): item flag checks (FUN_1004bd28 / FUN_100474a9 branches), item not in a tradable container, and, for a stock item of a shop, `FUN_100667fd(...) > 0` -> LDB text 0x6e with (amount, stat name of `FUN_10099ae4`, default 0x3d). Change: before `ShopCommand::Buy/Sell` is sent in net.rs 10828-10836, and the feedback through `feedback.rs`. MEDIUM-LARGE (the function is 6111 chars of undecoded helper calls; needs reading FUN_10066885/100667fd/1002a371 first).

### Table 2: trade with a player (mode 0) rows that differ from the shop

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| P1 | Click (left or right) on a row of the partner's list | **no click handler in mode 0**; only Shift + click shows the item's page. The partner's offered items are listed (the list shows `TradeGetInventory(+0x1c8)`) | 100e0a87 (`cmp [esi+0x1a4], edi`, edi=0), 100e0942, 100ce44c kind 2 | the partner's items are **never listed**: `Change::Bought` is only made for the booth (`item[0] == 0xc75b`); the code for `Sold` matches any character's `0x68` add and looks the slot up in the player's own bag. If a row did exist, a click would send `Remove` (no mode-0 test in `row_click`) | net.rs 9662-9677, shop.rs 726-731 | MISSING (INV-055 "partner's items not named") |
| P2 | Click on own offered row | `TradeRemoveItem(+0x1d0, item)` | 100dff6d | `Remove(sold[i].item)` | shop.rs 728-731 | MATCH |
| P3 | Accept (mode 0) | FUN_100e0622: if the cash timer runs, stop it and send `TradeSetCash` first; clear the HTML flag on `OurCashView` and re-write its text with `FormatNumeric(atol(text))` (regroup, **field locked**); Accept button disabled; the partner-status view `+0x1f8` set to 1; then **`N3Msg_TradeConfirm`** (a shop gets `TradeAccept`) | 100e0622 | `accept_player`: sends the credits first when the timer ran or the number differs, then `Confirm` (action 3) | shop.rs 832-846 | DIFFER (no regroup/lock of the field, no disable, no status change; sibling notes already list this as left) |
| P4 | Both sides confirmed: "Are you sure you want to complete this trade?" (title Trade, buttons Yes/No, name `TradeDoubleConfirmDlg`) | Yes -> `TradeAccept` (FUN_100e0132 arg 0); No -> FUN_100e00ff (abort); Escape/close = `SlotSelected(-1)`, ignored | 100e0f5b, 100e0132, 100e1bc8 | `asking` when both confirmed; Yes -> `Accept`, No -> `Close` and window closed; closing the dialog keeps the trade | shop.rs 545-563 | MATCH (the emitter of GlobalSignals `+0xe8` is [NR]; the client's "both confirmed" trigger is its own reading of the messages) |
| P5 | Typing credits in `OurCashView` | each text change restarts a 1 s timer, FUN_100e0231 sends `N3Msg_TradeSetCash(atol(text))` | 100e0c89 (0x100e011c connect 100e0e28-100e0e43), 100e0231 | `cash_edited` -> `CASH_DELAY` 1 s -> `Credits(typed)` | shop.rs 813-831 | MATCH |
| P6 | Enter in the credits field | no handler connected | [NF] in 100e0c89 / ctor | `Signal::Enter` sends at once | shop.rs 782-788 | FIXED 2026-11-25 (the ADDITION is gone): Enter ends the typing and sends nothing (`TextInputView_c::SlotEnterPressed` 0x1014896f); the credits go by the timer or by Accept |
| P7 | Partner's credits | GlobalSignals `+0xdc` (int) -> FUN_100e074d writes `FormatNumeric(value)` into `PartnerCashView` | 100e074d | `PartnerCashView` set to `vitals::grouped(partner_credits)` | shop.rs 601-607 | MATCH |
| P8 | State signal `+0xe0` (bool) | false: stop the timer, set the HTML flag again, status view 0, re-enable Accept, close the "TradeDoubleConfirmDlg" window. true: lock the cash text, status view 1 | 100e0260 | not ported (client closes nothing, re-enables nothing) | - | UNSURE (what raises it is [NR]; probably "the other side changed the offer" / "partner confirmed") |
| P9 | Partner status indicator | `BitmapView_c` (bitmaps 0x15b / 0xd9) in `PartnerStatusDock`, value 0/1 | 100e0c89 (100e0eaf-100e0f46) | draws the words "Partner accepted" / "Partner trading" | shop.rs 590-596 | DIFFER (look only) |

#### Port rules for Table 2

- **P1 partner list.** Rule: the partner's list shows `N3Msg_TradeGetInventory(partner)`; in mode 0 neither mouse button acts on it, Shift + left shows the item page. Change: (a) in `row_click` callers, ignore `Act` for list 1 when `self.partner.is_some()`; (b) feed partner rows from the `Trade` action 5 sent under the partner's identity (needs a capture of a trade with an item put in by the partner, as INV-055 says) and stop `Change::Sold` from taking the partner's adds (net.rs 9667 needs `item[1] == own`). (a) SMALL, (b) needs a capture.
- **P3 Accept.** Rule: after the cash step: re-write `OurCashView` text as `FormatNumeric(atol(text))` and make it read-only, disable `AcceptButton`, show the partner view's state 1, send `TradeConfirm`. Change `ShopWindow::accept_player` and the draw of both buttons. SMALL once P8's un-lock is known; the lock is undone by `+0xe0`.
- **P9** cosmetic: draw the two bitmaps in `PartnerStatusDock`. SMALL.

### Table 3: NPC chat window and the NPC trade bar (items handed to an NPC)

Original: `NPCChatView_c` FUN_100596c2 (answers text view, button bar FUN_10059eee), `NPCChatTradeBar_c` FUN_10057d8b, window FUN_1005a674 / FUN_10059236. The four buttons' handlers (connect list 10059b9e-10059c0a): Desc 0x10058cdb, Info 0x10058ad5, Give 0x10058d16, Shop-position button 0x10058b34. **0x10058b34 is not a function start: it is the tail after FUN_10058ad5's `ret`: `add ecx,0x144; push 0; push ecx; GetInstance; call N3Msg_UseItem`** (import 0x101a75f4 resolved with pefile) = `N3Msg_UseItem(npc identity, false)`. So the task text's "Shop button" handler is a plain use of the NPC; `N3Msg_NPCChatStartTrade` is the **Give** button (FUN_10058d16).

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| N1 | Let go a held inventory item on the bar's list | FUN_10057adc: if the list holds fewer items than the bar's slot count (`+0x1bc`) then, for mime `inventory/item` with `container_id`, `item_id` and `split_count` absent/0 -> `N3Msg_NPCChatAddTradeItem(own, npc, item)`, list refresh, `Accept(2)`; list full -> `Cancel` on the drag. Nothing stops the same item twice at this level | 10057adc, connect 10058009-10058028 (`+0x2dc`) | adds when not already in the bar, fewer than the slots, `split_count() == 0`, over the grid; when full nothing is accepted or cancelled here | npcchat.rs `trade_window` ~455-470 | DIFFER (client refuses a duplicate place; full case does not Cancel) |
| N2 | Click (plain left, or right released early) on a bar row | click signal -> FUN_10057c0b `N3Msg_NPCChatRemoveTradeItem(own, npc, item)` (list flags 0xe, so left click = click) | 10057c0b, 10057f87 (push 0xe), 10058039 (`+0x2e0`) | `Clicked` / right click -> Act -> `TradeItem(.., false)` | npcchat.rs 478-505 | MATCH |
| N3 | Shift + left on a bar row | qualifier -> FUN_10057c5d `itemid://` page | 10057c5d, 10058059 (`+0x2e4`) | `Page` -> info URL | npcchat.rs 489-497 | MATCH |
| N4 | Right held 0.3 s on a bar row | nothing (menu signal unconnected) | 100420fd, no `+0x2e8` connect | `row_click(true,..)` held -> Nothing | npcchat.rs 450-455 | MATCH |
| N5 | Ctrl + left on a bar row with count > 1 | split mode, no click | 10041926 | Ctrl ignored: removes | npcchat.rs 489-505 | DIFFER (same as #6) |
| N6 | V button (give) | FUN_10057cc6: credits = atoi(field); if > own credits (stat 0x3d) write the own credits into the field and use them; `N3Msg_NPCChatEndTrade(own, npc, credits, true)`; bar stays | 10057cc6 | `credits_given`, text written back, `FinishTrade(npc, credits, true)`, bar stays | npcchat.rs 514-530, `credits_given` 131 | MATCH |
| N7 | X button (decline) | bar's own signal -> 10058bf2: state 3, `NPCChatEndTrade(own, npc, 0, false)`, FUN_10058aa1 deletes the bar and its window | 10057a73, 10058bf2, 10058aa1 | `close = true` -> `FinishTrade(npc, 0, false)`, bar gone | npcchat.rs 531-541 | MATCH |
| N8 | Give button | FUN_10058d16: bar not up -> `N3Msg_NPCChatStartTrade(own, npc)`; bar up -> FUN_10058bf2 (same as X) | 10058d16 | `trade.take()` -> FinishTrade false, else `StartTrade` | npcchat.rs 322-330 | MATCH |
| N9 | Desc button | `N3Msg_NPCChatRequestDescription(own, npc)` | 10058cdb | `Command::Describe` | npcchat.rs 320 | MATCH |
| N10 | Info button | `charid://%u/%u` of the NPC identity to Info `ShowURL` | 10058ad5 | `charid://type/instance` | npcchat.rs 321 | MATCH |
| N11 | Fourth button (the client calls it "Shop") | `N3Msg_UseItem(npc identity, false)` | 10058b34 (disasm, import resolved) | `Command::Shop` -> `n3::use_object(own, uses, npc)` (a Use on the NPC; whether `use_object` is the same message as `N3Msg_UseItem` for an NPC identity was not read) | npcchat.rs 90, net.rs 10790-10796 | UNSURE (very likely the same) |
| N12 | Click an answer link | FUN_10059504: index < answer count: **if pref `ShowNPCQuestions` != 0 (default 1)** the answer is appended as the player's question line (kind 2), `N3Msg_SendNPCChatAnswer`, answer view cleared, list emptied | 10059504, 10058ee7 (LAB_10059142) | echo always appended; `Command::Answer`; answers cleared | npcchat.rs 301-314 | DIFFER (pref `ShowNPCQuestions` is declared in dvalue.rs 74 but never read by npcchat.rs). Note: DEVIATIONS NPC-040's recommendation line says "no local echo"; its Done text correctly says the original echoes (FUN_10059504 -> FUN_10058ee7(.., 2)) |
| N13 | Close the chat window (X) | `NPCChatWindow_c` destructor: if the "closed by the server" flag (`+0x98`) is 0 -> `N3Msg_NPCChatCloseWindow(own, npc)`. Whether the trade bar's window is closed with it is [NR] | 1005a569 | `Close(npc)` and `FinishTrade(false)` first when the bar is up | npcchat.rs 337-343 | UNSURE (the extra `FinishTrade` is the client's own) |
| N14 | Escape | closes the chat window when pref `esc_npcchat` is true (FUN_1005a90d connect at 1005a884) | 1005a674 | `escape_option("NPCChat") = esc_npcchat` | ui.rs 456 | MATCH |
| N15 | Ctrl + right click on an inventory item while only the NPC bar is up | FUN_100cae1b looks only for a `TradeView_c` (FUN_1002fcb7), so nothing is added to the NPC bar | 100cae1b | `shop::trading()` is `shop.open` only | ui.rs 1573 | MATCH |
| N16 | Tooltip / wheel on the bar or answers | [NR] | [NR] | name on hover | npcchat.rs 470-473 | UNSURE |

#### Port rules for Table 3

- **N1.** Rule: when the bar's list already holds `slots` items call `Cancel` on the drag (the item returns to its place); otherwise add whatever the conditions allow, with no duplicate test (the server answers a repeat). Change `trade_window` in npcchat.rs (drop `!put.contains(&dragged)`, and when full and `letting_go` over the grid call the put-down). SMALL. Low value.
- **N5** see #6, SMALL.
- **N12.** Rule: append the kind-2 line only if `IndependentPrefs ShowNPCQuestions` (default 1) is on. Change npcchat.rs 301-308 to test the pref (dvalue.rs already has it). SMALL.

### Counts

Table 1 (30 rows): MATCH 18, DIFFER 4, MISSING 2, ADDITION 1, ASKED 0, UNSURE 5.
Table 2 (9 rows): MATCH 4, DIFFER 2, MISSING 1, ADDITION 1, ASKED 0, UNSURE 1.
Table 3 (16 rows): MATCH 10, DIFFER 3, MISSING 0, ADDITION 0, ASKED 0, UNSURE 3.
Total (55): MATCH 32, DIFFER 9, MISSING 3, ADDITION 2, ASKED 0, UNSURE 9 (one of the DIFFER rows, #22, is the two cash fields; one UNSURE of Table 1 is the held-item click, #29).

Corrections to the notes (judged by the code): NPC-014's "Done" text is right about the original (flags 6/0xe make a plain left click the click signal; I verified the branch in 10041537-10041568 and the pushes of 6/6/0xe in 100e07b9 / 100e0942 / 100e0b00). The comment at `gear.rs` 242-244 still says "Ctrl+double-clicked"; the code is Ctrl + right click (INV-009 done).

### Not read / not found

- What raises GlobalSignals `+0xe0` (bool) and `+0xe8` in Gamecode/GUI (the trade state signals): only their handlers were read. Rows P4 trigger and P8 rest on that.
- The window frame's own menu button and how a right click on a window opens FUN_100e03c7 (vtable `+0xf8` caller); the Close entry's origin.
- The item list's hover tooltip and the mouse wheel code (MultiListView / ScrollView), for all three windows.
- `MultiListView_c::MouseUp` 0x10133d04 (up cell = down cell) and `WindowController_c::HandleMouseUp` (drop signal vs click signal under a drag) were taken from `INVENTORY_ORIGINAL.md`, not re-read (rows 9, 29).
- Gamecode `N3Msg_TradeAddItem` 0x1001925e and `N3Msg_TradeRemoveItem` 0x10017284 beyond their heads (refusal texts, the no-room feedback), the server side of the `Trade` actions, what Complete does to the items.
- The GlobalSignals `+0x17c` emission at trade start: where the text lands.
- `TextInputView_c` key handling in `OurCashView` (Enter, Escape, digits only) and the NPC bar's credits field.
- The NPC trade bar's own window (FUN_10059236): close button, whether closing the chat window closes it.
- Whether `n3::use_object` equals `N3Msg_UseItem` for an NPC (row N11).

## Shortcut bars and the Actions window


All addresses are GUI.dll unless said. Client paths are under `modern-anarchy-online/crates/mao-client/src/`.
Judged by the code only. Nothing here leans on a note saying a Shift/Ctrl click was "confirmed live".

### 0. How the original's mouse works for these two lists (read first)

Both `ShortcutBarView_c` (FUN_100d9588) and `SpecialActionView_c` (FUN_100dc9b1) make an `ItemListViewBase_c` (FUN_100423fc) and call FUN_10041248(1) on it [V 100d982e, 100dcb7e]. That sets `+0x319`. `UseNewMouseFunc` is true in LoginPrefs.xml line 20, and `MouseButtonHoldDelay` is 0.3 [V LoginPrefs.xml]. So the **new scheme** is on.

The list's own mouse down (vtable +0x3c = FUN_10041926) does one thing before the item sees the press: **Ctrl + left down** on an item whose stack count (item +0x78, FUN_1003d484(2)) is above 1 starts a "split count" mode. It shows a yellow number as a tooltip, the mouse move changes the number, and the mouse up emits pick-up with the count [V 10041926, 10041ca4, 100419d5]. Otherwise the press goes on to the item.

The item's mouse down (FUN_10041aa4) in the new scheme [V 10041aa4]:

- Only the **left** button counts (button 1), and only if the list's flags (`+0x150`) do not have bit 2.
- If the click signal (`+0x2e0`) has no listener: without Shift it emits pick-up (`+0x2d8`) at once.
- If the click signal has a listener (both lists here do): it remembers the item (FUN_101508bf) and starts the hold timer `+0x340` with `MouseButtonHoldDelay`.
- The timer's function (FUN_1004213e) emits pick-up `+0x2d8` for the remembered item. It does not look at Shift or at movement.
- Right down does nothing in the new scheme.

The item's mouse up (FUN_1004141b) [V 1004141b, disassembly 100414b9..10041578]:

- If a drag object exists: it emits drop `+0x2dc` (arguments: grid pos, drag, button).
- Otherwise, **left up**: Shift (`GetQualifiers & 3`) emits qualifier click `+0x2e4`. If flags have 0xc0 it toggles selection. Otherwise it emits click `+0x2e0`.
- **Right up** emits menu `+0x2e8`.
- Ctrl (4|8) and Alt (0x10|0x20) are tested nowhere here. Ctrl+click and Alt+click are plain clicks.
- The list-level mouse up (FUN_100419d5) stops both timers.

The flags (`+0x150`) are the 2nd argument of the MultiListView ctor [V 101365ff]. The shortcut bar passes 0x10 and the Actions list passes 0. I assume the ItemListViewBase ctor passes them on and did not read FUN_100423fc for that. If they are as passed, none of bits 2, 0xc0 or 0x400 is set.

What is connected (each is a slot on the signal):

| View | `+0x2d8` pick-up | `+0x2dc` drop | `+0x2e0` click | `+0x2e4` Shift-click | `+0x2e8` menu |
|---|---|---|---|---|---|
| Shortcut bar [V 100d9ae0..100d9b47] | FUN_100d8837 | FUN_100d89bd | FUN_100d833f | FUN_100d7fd3 | **none** |
| Actions [V 100dcb8a..100dcbdc] | FUN_100dc1ab | FUN_100dc274 | FUN_100dc033 | FUN_100dc3ae | **none** |

So neither list has a context menu. A right click does nothing on either (the view-level "Don't Auto Arrange / Sort" popup, FUN_10041637, is called only from 100cbace, 100d3027, 100d4f90, 100d758a and 100e03c7. None is these two views).

### 1. Shortcut bars

Rows: client = `shortcutbar.rs` (SB) and `ui.rs shortcut_layer` (UI, fn at line 2377). The smart hotbar (`hotbar.rs`) is ASKED, see row S1.

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| 1 | Left click on a filled cell (release) | Click: item (type 1): `N3Msg_UseItem(identity,false)`. Nano (4): `N3Msg_CastNanoSpell(nano, target from InputConfig +0xc0/+0xc4)`. Special action (6): `N3Msg_PerformSpecialAction`. Macro (7): `GetMacro`; if found, emit the global signal `+0x180` with the macro text. Does nothing if the item's `+0xf0` flag is set. No branch for type 2 (the class 0xdac3) | FUN_100d833f, FUN_1003d409, 10041aa4/1004141b | `response.clicked()` then `requests.action = Some(action)`, then `fire_action`: Attack/Special/Social/Nano/Macro/Item (an item is found by template in the inventory; a message if not carried) | ui.rs:2522; viewer.rs:1154 | MATCH for items, specials, nanos, macros. DIFFER small: the client also has Social and Attack kinds that the original's handler has no branch for. UNSURE: the original's `+0xf0` "unusable" flag and the nano target (what InputConfig +0xc0 is) are not read on either side |
| 2 | Left click, empty cell | Nothing (no item hit) | 10041aa4 (item only) | Nothing | ui.rs:2465 | MATCH |
| 3 | Shift + left click on a filled cell | Qualifier click: show info `itemid://%d/%d` with the shortcut's identity via `InfoViewModule_c::ShowURL`. Not for a macro (type 7). The action is **not** used | FUN_100d7fd3, 1004141b | Shift is ignored: the click **uses** the shortcut | ui.rs:2522 | FIXED 2026-11-25 (was DIFFER): the info page, `hotbar::info_url` |
| 4 | Ctrl + left click | A plain click (no Ctrl test). Exception: Ctrl+left **down** on an item with stack count above 1 starts the split-count mode | 1004141b, 10041926 | Plain click | ui.rs:2522 | MATCH for the click. UNSURE for split mode: whether a shortcut item's count (`+0x78`) is ever above 1 is not read. WIN-001 says stack counts are not ported |
| 5 | Alt + left click | A plain click (no Alt test anywhere in these handlers) | 1004141b | Plain click | ui.rs:2522 | MATCH |
| 6 | Right click (down and up) | Right up emits the menu signal, which has no listener: nothing. Right down: nothing | 1004141b, 10041aa4, connection list in section 0 | Nothing for a cell | ui.rs:2465 (only click_and_drag sensed) | MATCH |
| 7 | Right button held | Only the old scheme (`UseNewMouseFunc` off) arms the right-hold menu timer `+0x320`. Not used here | 10041aa4, FUN_100420fd | Nothing | - | MATCH |
| 8 | Double click on a cell | No double-click handling: two clicks, two uses | 1004141b | Two `clicked()` events: two uses | ui.rs:2522 | MATCH |
| 9 | Press and hold still on a filled cell for 0.3 s (`MouseButtonHoldDelay`) | The timer fires and **picks the shortcut up** (drag begins) even without any movement. Not when `LockHotbars` is true, nor when the item's `+0xf0` flag is set | 10041aa4, FUN_1004213e, FUN_100d8837 | A hold without movement does nothing. egui's drag starts on movement, and Shift/Ctrl/Alt do not matter | ui.rs:2516 | FIXED 2026-11-25 (was DIFFER): held 0.3 s the shortcut is taken up, `shortcutbar::cell_mouse` |
| 10 | Press, move, release in under 0.3 s (a quick flick) | The timer has not fired. The mouse up arrives with no drag object, so it is a **click** (Shift-click info with Shift) on the remembered item | 10041aa4, 100419d5, 1004141b | The first movement past egui's threshold starts a drag at once | ui.rs:2516 | FIXED 2026-11-25 (was DIFFER): a press let go sooner is a click however the pointer moved (the release must be over the cell it went down on) |
| 11 | Pick-up leaves the cell? | After the drag ends (both `DragObject_c::Accept` 0x1012b8f2 and `Cancel` 0x1012b989 emit the signal at `+8`) FUN_100d8045 deletes the cell's item if the item at `container_position` is the dragged one. Whether the signal FUN_100db025 connects (`+0xc` of the drag) is this one is not settled. The drag carries mime `inventory/item`, `container_id` zero, `container_position`, `item_type`, `item_id`, `split_count`. For an item (type 1) with a dynel id, `CloseContainer` is called first | FUN_100d8837, FUN_100db025, FUN_100d8045, FUN_1003d4a4 | The item leaves the cell at the start of the drag and is gone unless dropped on a cell; blocked while locked | ui.rs:2516-2521 | UNSURE. The result of "dropped on nothing" matches the user's old notes (the cell goes). What the original does with a drop on **its own cell** is not settled: 100d89bd calls `Cancel`, which also runs FUN_100d8045, which would delete it. The client keeps it. Needs the WindowController side read |
| 12 | Drop a held thing on a cell: an own item / nano / action (mime `inventory/item`, `item_type` not 3, `container_position` present, drop button 1) | From **another** view: `Accept(1)`. If the cell holds an item, that item is dragged out (new drag) and deleted from the bar. Then an item (type 1) must be `N3Msg_IsItemMine`, else nothing is made. Then `FUN_100d825b` makes the shortcut: id 0x6b makes nothing; 0xdac3 type 2; 0xcf1b type 4 (nano); 0xdeb0 type 6; 0xc789 type 7 (needs the macro to exist); else type 1 | FUN_100d89bd, FUN_100d825b, FUN_1003d6f2 | Dropped payload replaces the cell; the old one goes into the hand | ui.rs:2508-2515 | MATCH in effect. UNSURE: the `IsItemMine` test (is a not-own item ever offered as a payload?) is not read on the client side |
| 13 | Drop from the **same bar** onto another cell | Same cell: `Cancel`. Else `MoveItem(from,to)`, `Accept(2)`, and the displaced item (if any) is picked up again through FUN_100d8837 (so a swap goes into the hand, and a locked bar does not) | FUN_100d89bd | Taking up empties the source cell, so this is the same as row 12 | ui.rs:2508 | MATCH in effect |
| 14 | Drop with `LockHotbars` on, onto an occupied cell | Dropping is not tested for the lock. From another view the old item is still dragged out (FUN_1003f67b path, no lock test). Only a re-pick of a swap (FUN_100d8837) is blocked | FUN_100d89bd, FUN_100d8837 | The replaced shortcut is thrown away when locked (`before.filter(|_| !locked)`) | ui.rs:2511 | FIXED 2026-11-25 (was DIFFER): the shortcut pushed out is in hand on a locked bar too |
| 15 | Drop a text macro (mime `inventory/textmacro`) on a cell | Existing item in the cell is deleted, macro shortcut made, `Accept(1)` | FUN_100d89bd | Takes any `hotbar::Action` payload (macro windows' drags are the same type) | ui.rs:2508; macros.rs | MATCH in effect. UNSURE: not checked that the macro window's drag is a payload |
| 16 | The macro window creating a macro and dragging it (FUN_100d8c4a: a global signal `+0x1a0`) | The bar creates the macro (`TextMacroSystem_t::CreateMacro`) and starts a `inventory/textmacro` drag from the bar | FUN_100d8c4a, ctor line 100d9b61 | Not found on the client side (not read) | - | UNSURE [NR] |
| 17 | Wheel over the bar | The view's wheel slot (`+0x90`) forwards to the strip: FUN_100d7ed9. The strip's wheel (FUN_100dbef9): float at `+4` above 0 gives page `(p+9)%10`, else `(p+1)%10`. Emits the page-changed signal | FUN_100d7ed9, FUN_100dbef9; slot 0x90 is the wheel by `_HandleMouseWheel` 10154ab9 | Wheel over the bar rect: up is the page before, down the next, wrapping 0..9 | ui.rs:2450-2456; SB `wheel` line 82 | MATCH. UNSURE: whether a wheel over a cell reaches the strip (bubbling of the list's `+0x90` = 0x1014a8ef) |
| 18 | Page arrow buttons | Button at `+0x148` goes to `(p+9)%10`, the one at `+0x14c` to `(p+1)%10`. The `+0x148` button is placed at the top (horizontal) or left (vertical) | FUN_100db7d9, FUN_100db7f1, FUN_100db477, connections at 100db9a1, 100dba48 | The upper/left button goes back, the other forward | ui.rs:2423-2443 | MATCH (the client's comment "not known" can now be settled by 100db7d9/100db7f1) |
| 19 | Page number text | `(page+1)%10` in yellow | FUN_100db22c | Same | ui.rs:2439 | MATCH |
| 20 | Left press and drag on the strip | Starts a window drag; movement moves the bar unless `LockHotbars`, then keeps it on screen (`MoveInsideScreen`) | FUN_100dbe17, 100dbc76, 100dbc5a | Dragging the strip moves it unless locked; position clamped to the screen | ui.rs:2435-2443 | MATCH |
| 21 | Right click on the strip | **Toggles orientation** (horizontal/upright) when not locked: `FUN_100db477(!orientation,1)`. Locked: nothing | FUN_100dbe17 (button 2 and not `LockHotbars`) | Right click does nothing | ui.rs:2412-2449 | FIXED 2026-11-25 (was MISSING): the right button pressed on the strip turns the bar, `ui::strip_turns`; and a locked bar is moved by the left button as FUN_100dbe17 has it |
| 22 | Double click on the strip | No such handling | FUN_100dbe17 | **Toggles orientation** (not locked) | ui.rs:2444 | FIXED 2026-11-25 (the ADDITION is gone: a double click no longer turns the bar) |
| 23 | Orientation change effects | Re-lays cells (10 by 1 or 1 by 10), swaps cell positions (i,j with j,i) so the contents keep their (page, slot), re-applies the page | FUN_100d9003 | Cells are kept in one `(page,slot)` order, so contents keep their place | SB `Bar` | MATCH in effect |
| 24 | Primary radio toggle: click | A click when the bar is not primary: emits (FUN_100d84ae) which reaches the container: `FUN_100d8622(n)` hides the old selected bar's flag, sets the new. A click when it is primary: nothing | FUN_100d84ae, FUN_100d8622, FUN_100d819f (the toggle's value is `IsPrimary`) | Click sets `primary = b` | ui.rs:2407; SB `select` | MATCH |
| 25 | Key `1`..`0` (`SHORTUCT_BAR_ACTIVE_n`, default input 108..117) | Uses cell `(page*10+n)` of the **selected** bar's grid: FUN_100d8f45 then FUN_100d8665 then FUN_100d833f (the click handler). Acts on key down | FUN_100d8f45, FUN_100d8665, CharPrefs.xml hashes | `press(n)` of the primary bar's shown page then `fire_action` | SB `press` 62; viewer.rs:1700 | MATCH |
| 26 | Key Shift+`1`..`0` (`SHORTUCT_BAR_ROW_n`, default Shift+108..117) | Calls FUN_100d8f71(n) on the selected bar: sets page n (scroll), emits page-changed | FUN_100d91c2, FUN_100d8f71 | `row(n)` sets the primary's page `n % 10` | SB `row` 74; viewer.rs:1694 | MATCH |
| 27 | Key Alt+`1`..`0` (`SHORTUCT_BAR_SELECT_n`, default Alt+108..117) | `FUN_100d8622(n)`: only if n is below the number of bars shown | FUN_100d8705, FUN_100d8622 | `select(n, count)` | SB `select` 67; viewer.rs:1684 | MATCH |
| 28 | Keys `SHORTCUT_BAR_<bar>_<slot>` (100 providers, no default key) | Each bar's window connects its ten slot keys: FUN_100d86e3 then FUN_100d8665 on that bar. Not the selected bar | FUN_100d9e5f (100da228..100da234), FUN_100d86e3 | Not offered (no action) | gui/bindview.rs:143 (comment) | MISSING (only matters once a key is bound) |
| 29 | Key `WINDOW_SHORTCUTBAR` (default key number 106 = `Y`, no qualifier) | A provider with this name exists and has a default key. The handler is not read | CharPrefs.xml hash 0xe3ffcaba; client key table keybindings.rs:221 | No action for it | keybindings.rs doc comment | MISSING [NR for what it does: probably show/hide the bar windows] |
| 30 | New nano learned (`ShortcutNewNanos` true) | The nano is added to the **primary** bar's first free cell (FUN_100d830f, only if FUN_100d819f says the bar is primary) | FUN_100d830f, FUN_100d819f, ctor 100d9c52..100d9c98 | Not done (WIN-001 lists it as not ported) | - | MISSING |
| 31 | First run: default shortcuts | `IsFirstTime`: cell (0,0) 0xc1a5, (1,0) 0xc1a2, (2,0) 0xc1a8, (3,0) macro "Follow" `/follow`, (9,0) 0x14124 | FUN_100d9e5f | Same five cells | SB `first_run` 90 | MATCH |
| 32 | Bar window frame / tab / close | The bar is a `Window` (flags 0x183c) with a tab "ShortcutBar" (`AppendTab`) | FUN_100d9e5f | The bar is an egui Area with no frame, tab or close (WIN-001: "the bars' close" not ported) | ui.rs:2398 | UNSURE [NR what the window frame lets a player do] |
| 33 | `ShortcutShortTooltips` hover text | Preference read in the ctor; the tooltip text itself not read | ctor 100d9648 | Hover text is the action's name | ui.rs:2497 | FIXED 2026-11-25 (was UNSURE; read: FUN_100d8129 clears the list's +0x368 while `ShortcutShortTooltips` is on, as shipped): the name in its rarity's colour alone; a macro's name with its command, "Broken macro" when it is gone (`hotbar::tip`) |
| 34 | Hover cursor | FUN_10041ca4 changes the pointer over an item whose skill flag (`GetSkill` stat 0x1e, bit 8) is set, and for Shift | FUN_10041ca4 | Default pointer | - | UNSURE [NR the pointer shapes] |
| S1 | Smart hotbar (own bars: 6 bars, priority lists, queued presses, right click edit) | n/a | n/a | `hotbar.rs`, `hotbar_layer`; setting `smart_hotbar` (default off) | hotbar.rs; ui.rs:3056; viewer.rs:1684-1700 | ASKED (kept on the user's word of 2026-11-25: asked "Keep the five extras you asked for earlier, which the original does not have?", the answer was "Keep them"): the smart shortcut bars, the `smart_hotbar` setting, off by default (WIN-001, WIN-152) |

#### Rules to port for the shortcut bars (DIFFER / MISSING rows)

- **Row 3 (Shift+click).**
  - Order: the click signal fires only if Shift is not held; with Shift (`GetQualifiers & 3`) do the info instead.
  - Info for type 1/4/6 = `ShowURL("itemid://<id0>/<id1>")` for the shortcut's identity (two numbers). Macro (7): nothing.
  - Change `shortcut_layer` (ui.rs:2522): test `ui.input(|i| i.modifiers.shift)` and call the info viewer (the same `info_view.show_url` used in ui.rs ~1533) instead of `requests.action`.
  - Size: SMALL. The identity numbers of a shortcut are not stored in `hotbar::Action` for nano/special: check `id`.
- **Rows 9 and 10 (hold timer).**
  - Left press on a filled cell starts a timer of `MouseButtonHoldDelay` (pref, default 0.3 s). When it elapses with the button still down: pick up (not locked, item not disabled) and begin the drag. A release before it is a click (Shift: info).
  - The timer does not look at movement or Shift.
  - Change the cell loop in ui.rs (~2465-2530): do not use `drag_started` for shortcuts; use `is_pointer_button_down_on` plus the elapsed time.
  - Size: MEDIUM. A click that moves under 0.3 s needs the open question in row 10 settled first.
- **Row 21 (right click turns the bar).** When not `LockHotbars`, a right click (button 2 down) on the strip toggles orientation. Locked: nothing. Change ui.rs:2444: use `strip_response.secondary_clicked()` (or `clicked_by(Secondary)`). The double click is the client's own; remove or keep as an addition (row 22). Size: SMALL.
- **Row 14.** Do not drop the replaced shortcut when locked: `before` always goes to the hand when the drop came from another view (FUN_100d89bd has no lock test). Only the re-pick of a swap inside the bar is blocked, and that cannot happen when locked. ui.rs:2511. Size: SMALL.
- **Row 28/29.** Add `SHORTCUT_BAR_<b>_<s>` (b, s in 0..9): activate slot s of the current page of bar b (page*10+s, FUN_100d8665). Add `WINDOW_SHORTCUTBAR` (default `Y`) once its handler is read. Files: settings.rs `Action`, keybindings.rs `PROVIDERS`, viewer.rs near line 1684. Size: SMALL for the slot keys; the window key needs a read first.
- **Row 30.** On a new nano learned, if `ShortcutNewNanos` (LoginPrefs.xml line 11, true) and bar is primary: put it in the first free cell (FUN_100d830f: `GetFirstFreePos`, then FUN_100d825b). Where the client learns a nano: net.rs. Size: SMALL to MEDIUM (needs the "free cell" scan over the current page's grid only? `GetFirstFreePos` scans the whole 10 by 10 grid, order not read).

### 2. Special actions window ("Actions", `SpecialActionView_c`)

Client: `actions.rs ActionsWindow::show` (line 434) on the generic `gui/multilist.rs` list (mouse code at multilist.rs:486-520; host mouse in gui/host.rs:225-285, which feeds the primary button only).

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| A1 | Left click on a tile | `N3Msg_PerformSpecialAction(identity)`. No disabled test, no lock test (the server answers a locked one) | FUN_100dc033 | `ItemEvent::Clicked` sets `clicked`; ui.rs sends the action (the network thread says the wait line) | actions.rs:503-511; ui.rs:1542 | MATCH |
| A2 | Shift + left click | Info `itemid://%d/%d` (no macro check here) | FUN_100dc3ae, 1004141b | Shift is ignored: the click **performs** the action | actions.rs:503 | FIXED 2026-11-25 (was DIFFER): the info page |
| A3 | Ctrl or Alt + left click | A plain click | 1004141b | A plain click | actions.rs:503 | MATCH |
| A4 | Right click / right hold | Menu signal has no listener: nothing | connection list (section 0) | Nothing (host.rs reads only the primary button) | gui/host.rs:246 | MATCH |
| A5 | Double click | Two clicks, two performs | 1004141b | The click count is dropped; both clicks perform | actions.rs:503 | MATCH |
| A6 | Press and hold 0.3 s | Pick-up: a drag of mime `inventory/item`, `item_type` 5 (the code sets 5 there; see row note), the tile's identity. No `LockHotbars` test. The tile stays in the Actions list | FUN_100dc1ab | A press moved more than 3 points starts the drag (`ItemDragged`) | gui/multilist.rs:513 | FIXED 2026-11-25 (was DIFFER): held 0.3 s, `multilist::hold_to_pick` |
| A7 | Drop a thing on the window | Accepts only a drag of `item_type` 6 from this same view (`drag +0x60 == this`), left button, when `LayoutMode` is not 1: finds the dragged tile, `MoveItem(from, drop cell)`, `Accept(2)`. Dropped on its own tile: `Cancel` | FUN_100dc274 | No drop target: tiles are in list order and cannot be moved | actions.rs:434+ | MISSING (WIN-022: "kept places not ported") |
| A8 | Kept places | The grid places are saved/loaded as `item_position_map` in `SpecialActions.xml` | FUN_100dc70a, FUN_100dc9b1 | Not kept | DEVIATIONS.md:1103 | MISSING (same fix as A7) |
| A9 | Wheel | Scrolls the grid (vertical scroll bar mode set by FUN_100dc005) | FUN_100dc005 (summary only) | Wheel moves a row at a time and lands on one | gui/multilist.rs:680 | UNSURE [NR the original's step] |
| A10 | Key to open | `WINDOW_SPECIALACTION` default input 262253 = Ctrl + key 109 (= `2`) | CharPrefs.xml hash 0xcb70c086 | `Ctrl+2` | settings.rs:238 | MATCH |
| A11 | Lock/cooldown look | Timer text and fade by `lock` per tile | not read (FUN_1004300e etc.) | Fade by share, text from `actions::timer_text` | actions.rs:192-200, 470 | UNSURE [NR the original's draw] |
| A12 | Toggle tiles (Sit/Stand and the like) | The tile shows the other half after a state change | WIN-113, FUN_1004300e (not re-read) | `shown_number` | actions.rs:264 | UNSURE [only the notes read] |
| A13 | Window frame / title / close / rollup | Named "Actions"; `RollupArea`; dock | FUN_100dc9b1 strings | `chrome::window` "Actions", resizable | actions.rs:481 | UNSURE [NR] |

#### Rules to port for the Actions window

- **A2.** Same as shortcut row 3: Shift down at the release shows `itemid://<template>/<instance>` info rather than performing. Change actions.rs:503; the multilist control must pass the Shift state with the click (multilist.rs:506 `Signal::ItemClicked`). Size: SMALL.
- **A6.** Same as shortcut row 9. Size: MEDIUM (host.rs and multilist.rs timer).
- **A7/A8.** The list needs free grid places (move a tile onto a free or occupied cell, swap) and a save of `item_pos` per `item_id` in a `SpecialActions.xml`-like file. Size: LARGE, judged by eye (as DEVIATIONS.md:1103 says).

### 3. Not read / not found

- The MultiListView base (`MouseDown` 0x1013xxxx `MultiListView_c::MouseDown/MouseUp/MouseMove`, `HitTest`, FUN_10042a09/FUN_100e6f7f remembered-item logic): which item a mouse up reaches after the cursor moved (rows 10, 11 depend on it).
- `WindowController_c::BeginDrag` and the drop-result rules: whether a drop on nothing calls `Cancel` (row 11).
- FUN_100423fc (ItemListViewBase ctor): that arg 2 reaches `+0x150` unchanged.
- The pointer shapes (FUN_10041ca4) and the tooltip text (`ShortcutShortTooltips`, FUN_10042193 is the cell text).
- The handler of `WINDOW_SHORTCUTBAR`; the window frame of a bar (`ShortcutBarWindow_c`, FUN_100d9e5f beyond its first-run lines); FUN_100d8e6f and the other state-following callbacks (item moved/used/removed in the inventory update the shortcut).
- What the item `+0xf0` flag is exactly (set by FUN_100d7eeb via FUN_1003d3e2(FUN_1003d070)); the client has no counterpart that I found.
- Nano cast target (`InputConfig_t` +0xc0/+0xc4) against the client's `keys.cast`.
- The old-scheme paths (only reached if the player turns `UseNewMouseFunc` off): read, but not compared to the client.
- Client side not read: `hotbar.rs` rows (ASKED), how inventory/nano/macro windows create their drag payloads, `charprefs`/saving of bars.

## Nano programs, NCU, perks


Original: GUI.dll decompile (`exports_gui/`), read today. Client: `modern-anarchy-online/crates/mao-client/src/`, read today (working tree).
Shift/Ctrl/Alt note from the brief respected: no row below rests on a "confirmed live" Shift/Ctrl note; every client statement is from the code.

### 0. The mechanism every row leans on (all [V], read today)

Both nano lists are `ItemListViewBase_c` (ctor FUN_100423fc). It connects its own view signals: +0x130 mouse down -> FUN_10041aa4, +0x134 mouse up -> FUN_1004141b, +0x138/+0x13c -> FUN_100412a4/FUN_100412b8, +0x140 tooltip -> FUN_10042193, timer +0x320 -> FUN_100420fd, timer +0x340 -> FUN_1004213e (asm 10042525-1004256a). Its five outgoing signals: +0x2d8 pick-up, +0x2dc drop, +0x2e0 click, +0x2e4 qualifier click, +0x2e8 menu. Emitters: FUN_1004258f (3 args), FUN_100425fe (2 args), FUN_1004266a (1 arg); each does nothing when the signal has no slot.

"New" scheme = byte +0x319 set by FUN_10041248(1) AND pref `UseNewMouseFunc` true. Its reading (asm 10041aa4..10041c94 and 1004141b..10041568):

- Mouse DOWN, new scheme [V 10041aa4]: only button 1 (left), and only if view flags (+0x150) & 2 == 0. If the click signal (+0x2e0) has NO slot: without Shift (qualifiers & 3 == 0) it emits pick-up at once. If the click signal HAS a slot: remembers the pressed item (FUN_101508bf) and starts the left hold timer (+0x340) for pref `MouseButtonHoldDelay`. Right button down: nothing.
- Hold timer +0x340 fires [V 1004213e]: EndMouseClick (releases mouse capture), then emits pick-up (+0x2d8) for the pressed item. Hold timer +0x320 [V 100420fd]: emits menu (+0x2e8).
- Mouse UP, new scheme [V 1004141b, asm 100414c6-10041503]: if the 4th arg (dragged object) is non-null: emits drop (+0x2dc). Else, left (1): Shift (qualifiers&3) -> qualifier click (+0x2e4); else if flags&0xc0 (multi-select) -> toggle selection; else click (+0x2e0). Right (2): emits menu (+0x2e8) with the item. Only Shift is ever tested (mask 3); Ctrl and Alt change nothing in these lists.
- "Old" scheme (the NCU list, which never calls FUN_10041248) [V 10041aa4 branch at 10041bfa, 1004141b branch at 10041520]: DOWN acts only for button 2 (right): no click slot -> menu at once; click slot present -> hold timer +0x320. UP: left (1): Shift -> qualifier click (+0x2e4); else multi-select toggle if flags&0xc0; else (flags&2 == 0) emits PICK-UP (+0x2d8); (flags&2 != 0) click. Right (2): if flags&0x400 nothing, else click (+0x2e0).
- Flags (+0x150) are the 2nd ctor arg: 0 for the nano list (100d56ea pushes 0) and the own NCU list (100d78a0 pushes 0), 0x310 for the target NCU list (100ddcf7). None of them has bits 2, 0xc0 or 0x400, so the "flag" branches above are not taken. [V asm; the flag meaning itself is from the view code, not re-derived]
- MouseMove (FUN_10041ca4) only changes the mouse pointer (Shift held with nothing dragged -> pointer type 3; some usable inventory items -> other types) and runs the stack-split counter; it starts no drag by itself. The base `MultiListView_c::MouseMove` (10133dba, grid cell hover) was NOT read: whether a plain press-and-move raises pick-up there is [NR].
- `ButtonBase_c::MouseDown` (101298db) fires a `Button_c` on the LEFT PRESS (button 1 only; other buttons fall to `View::MouseDown`); `MouseUp` (101294c9) only releases. A toggle button toggles and calls vtable +0xf0 on the press.

Client side common code: `gui/multilist.rs` `MultiList::mouse` (lines ~475-520): `Down` remembers the row; `Move` raises `ItemDragged` once the pointer is more than 3 px from the press; `Up` inside the same row raises `ItemClicked(id, clicks)` unless it was dragged. There is NO hold timer, NO qualifier information on the signal (`take_events` -> `ItemEvent::Clicked(i64,u32)`, `Dragged(i64)`, multilist.rs:1115-1155), and no button information (right clicks are read separately with egui `secondary_clicked`). A grep for `modifiers|shift|ctrl|alt` in `nanoprograms.rs`, `ncu.rs`, `perks.rs`, `gui/multilist.rs` finds nothing (only `gui/host.rs:305` for key events). So no Shift/Ctrl/Alt handling exists in any of these windows.

---

### 1. Nano programs window (`NanoView_c`, FUN_100d5328; client `nanoprograms.rs`)

Handlers connected on the selected school's list (asm 100d5b95-100d5c16; re-connected on a tab change by FUN_100d4b49): pick-up 100d41c3, drop 100d4514, qualifier click 100d42ab, click 100d44cd, menu 100d49a0.

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| N1 | Left click on a program (press, release within the hold delay, no move) | Casts the nano: `N3Msg_CastNanoSpell(program identity, InputConfig target at +0xc0/+0xc4)` | [V 1004141b, 100d44cd] | Casts on the selected target (or self), with the client's own cast checks | `nanoprograms.rs` `ItemEvent::Clicked` (l.473-482) -> `ui.rs:1421` -> `net.rs:~11600` | MATCH (effect). Timing differs, see N2 |
| N2 | Left press and HOLD for MouseButtonHoldDelay (0.3 s, no move), then release or keep holding | At the delay: capture released, pick-up signal -> FUN_100d41c3: `View::BeginDrag` with an "inventory/item" drag object of the program (tab selection must be 0..6). The later release is NOT a click | [V 10041aa4, 1004213e, 100d41c3, 1003d4a4] | No hold timer. A held press without moving is still a click when released: it casts. A drag starts only after moving more than 3 px | `gui/multilist.rs` `mouse` l.~475-520; `nanoprograms.rs:484` | FIXED 2026-11-25 (was DIFFER): held 0.3 s the program is taken up and the release is no click, `multilist::hold_to_pick` |
| N3 | Press, move, release (drag) | Not read in the lists' own code: pick-up comes only from the hold timer in the new scheme. Whether the base `MultiListView_c::MouseMove` (10133dba) also starts one is not known | [NR 10133dba] | Drag after 3 px, payload `hotbar::Action::Nano` | `multilist.rs:510-515`, `nanoprograms.rs:484-489` | FIXED 2026-11-25 (was UNSURE): a press that moves no longer drags; only the hold does |
| N4 | Shift + left click | Qualifier click (+0x2e4) -> FUN_100d42ab: info page `itemid://<identity type>/<id>` (type 0xcf1b = 53019 for a nano) through `InfoViewModule_c::ShowURL`. No cast | [V 1004141b asm 100414dd-100414e8, 100d42ab] | Casts the program (Shift is not looked at) | `nanoprograms.rs:473-482` | FIXED 2026-11-25 (was DIFFER): the info page, no cast |
| N5 | Ctrl + left click, Alt + left click | Same as plain left click (only Shift is tested) | [V 1004141b: only `& 3`] | Cast | same | MATCH |
| N6 | Shift + hold | The hold timer ignores Shift: pick-up as in N2 | [V 1004213e] | n/a | | covered by N2 |
| N7 | Right click on a program | Menu on the RELEASE (right down does nothing; hold does nothing extra) -> FUN_100d49a0: PopupMenu with entries below, shown at the pointer | [V 1004141b asm 100414cd-100414d6, 10041aa4 new-scheme exit for button != 1, 100d49a0] | Menu on `secondary_clicked` (release) over a program | `nanoprograms.rs:506-535` | MATCH |
| N8 | Menu entry "LookAtNano" (always) | Info page `itemid://type/id` | [V 100d4314] | Same, `look:<id>` -> `itemid://53019/<id>` | `nanoprograms.rs:540` | MATCH |
| N9 | Menu entry "HideNano" (item is in lists 0-5, found by FUN_100d40ce) | Item removed from its school's list and added at the first free cell of list 6 (the hidden list) | [V 100d49a0, 100d4380] | Program added to a hidden set (saved in `hiddennanos.ini`); `shown()` puts it in list 6 | `nanoprograms.rs:541-548`, `shown` l.91 | MATCH (effect; storage differs, not a player difference) |
| N10 | Menu entry "UnHideNano" (item is in list 6) | Item moved from list 6 to the list of its school (skill 0x195 value - 1, out of 0..6 -> 5) at the first free cell | [V 100d43fe] | Removed from the hidden set; program returns to `p.school` | `nanoprograms.rs:541-548` | MATCH |
| N11 | Menu when the item is in no list (FUN_100d40ce returns 8) | Only "LookAtNano" | [V 100d49a0] | n/a (always found) | | MATCH |
| N12 | Right click on the empty list / window background | The window's menu FUN_100d4f90: "ListMode" (check = layout mode 1, toggles all 7 lists, FUN_100d4182), "ShowHidden" (FUN_100d4b49(6,0): shows list 6), "AutoArrange" with the sub-menu FUN_10041637(1) (only when a list is selected; dead in list mode) | [V 100d4f90, 100d4182, 100d4d3a asm, 10041637]. Where the original opens it from (title/frame menu or list background) was NOT found: no connect of 100d4f90 in the exports ([NF]) | Opened on a right click that is not over a program, and from the frame's menu button with "Close Window" after | `nanoprograms.rs:506-525`, `chrome::frame_menu` | FIXED 2026-11-25 (was UNSURE for the trigger; read: FUN_10041aa4 returns with no item under the pointer, and the view's menu is reached by the frame's button through `DockWindow_c`'s slot 0x1003c2b8 or by the rollup page's FUN_10048c5c): the right click beside the programs does nothing |
| N13 | "AutoArrange" sub-menu entries | "Don't Auto Arrange" (check when +0x318 clear) + separator; one entry per column: id 0 "Unsorted", else "By <label>", check on the grid sort column, dead unless grid layout; separator; "Sort Ascending" (check when order 0, dead when list mode / not arranged / column 0). Choosing a column: FUN_10041258(col, order) -> arranged by it; "Don't Auto Arrange": +0x318=0, SetAutoArrange(false); "Sort Ascending": toggles the order | [V 10041637, 10041303, 10041331, 10041258] | Same entries and rules; "none" keeps each program in the cell the arranging had | `nanoprograms.rs:71-89` (`arrange_menu`), l.556-566 | MATCH |
| N14 | School buttons (Favorites=5, Combat 0, Medical 1, Prot 2, Psi 3, Space 4; list 6 is the hidden list) | A click selects that school's list (FUN_100d4cc4 -> FUN_100d4b49(school,0), no change when already selected). Button_c acts on the left PRESS | [V 100d6f3d, 100d4cc4, 100d4b49, 101298db] | Selects the school on `Clicked`/`Toggled` (release) | `nanoprograms.rs:492-503` | MATCH (effect); press vs release is a tiny difference (SMALL, see P12) |
| N15 | Drop a dragged nano ("item_type" 4, from a nano list) on a grid cell, grid layout | Same list: if the cell is not the item's own, `MoveItem(from, cell)`, drag accepted (2), and the item that sat there is picked up again (FUN_100d41c3); on the item's own cell the drag is cancelled. Other list: item removed from the source list and added at the cell. List layout (mode 1): nothing | [V 100d4514] | Only in a grid "placed by hand" (not arranged, not list): empty cell -> put; occupied -> exchange and the other is in hand next; own cell -> nothing | `nanoprograms.rs:452-466` | MATCH for the hand grid; UNSURE for an arranged grid (original does not test the arranging flag in this handler; whether `MoveItem` is a no-op there was not read) |
| N16 | Drop an inventory item (drag "item_type" 1) on a program | `N3Msg_CastNanoSpell(program, dropped item's identity)` after emitting the chat/system line `ExeNanoXonY` (nano name, item name); drag accepted | [V 100d4514 asm 100d45a7-100d46d2] | Not found: no handler for a drop of an inventory item on a program (`ExeNanoXonY` is nowhere in the client) | `nanoprograms.rs` (none); grep over the crate | MISSING |
| N17 | Shift held, pointer over a program (no drag) | Mouse pointer changes to the pointer type 3 (info cursor) | [V 10041ca4 tail] | Not read in the client | [NR client] | UNSURE |
| N18 | Hover tooltip | Tooltip signal +0x140 -> FUN_10042193 (nano text) | [V 10042193 partly] | Own layout (name, NCU, description, changes) | `nanoprograms.rs:573-600` | FIXED 2026-11-25 (was ASKED, NAN-059): the nano's page built for a tip (FUN_1003683d with its flag: no header, no skills to use, no requirements) under its name in its rarity's colour, `iteminfo::nano_tip` |
| N19 | Double click | No double-click signal; a second click is a second cast | [NF in the five signals] | `clicks` ignored: a second click also casts | `nanoprograms.rs:473-482` | MATCH |
| N20 | Escape | Window registers its close for Escape when pref `esc_nano` is true (shipped value false) | [V 100d5328 ('esc_nano', FUN_1004a443); LoginPrefs.xml:77] | Escape closes it when `esc_nano` | `ui.rs:411-451` ("Nano programs" => "esc_nano") | MATCH |
| N21 | List-mode column header click (sort), resize, label drag to swap columns | `ColumnHeaderView_c` | [NR] | Header sort and resize exist; label swap "not done here" | `gui/multilist.rs` header section, l.11 | UNSURE (original not read) |
| N22 | Mouse wheel | scroll view | [NR] | scrolls a row at a time | `multilist.rs:680` | UNSURE |

#### Differences to port (nano window)

- N4 Shift click -> info page. Rule: on a left release with Shift (qualifiers & 3 != 0) emit the info page `itemid://53019/<id>` and do NOT cast; Ctrl/Alt are ignored. Client: the qualifier must reach the list: add `shift: bool` to `Signal::ItemClicked` / `ItemEvent::Clicked` (`gui/multilist.rs` `Mouse::Up` and `take_events`, the host `gui/host.rs` passing the egui modifiers), then in `nanoprograms.rs` `show()` (l.473) push the info URL instead of `cast`. SMALL (an hour; the plumbing touches every multilist user, so keep the new field defaulted).
- N2 hold-to-pick-up. Rule: left press on a row (new scheme, click slot present) starts a one-shot timer of `MouseButtonHoldDelay` (0.3 s); when it fires with the same item still pressed, release capture and begin the drag (payload `Action::Nano`), and the later release must not click. A release before the timer is the click. Client: `MultiList::mouse` needs the press time and a tick (`Down` stores `Instant`; `host` or `update` checks it each frame and raises `ItemDragged`); `Up` after a fired hold returns None. Decide with N3 whether the 3 px move-drag stays. MEDIUM (timer plumbing in the view layer and repaint scheduling; also affects the Actions list, inventory lists with the new scheme, shortcut bars).
- N16 drop an inventory item on a program -> cast it at that item. Rule: on drop with drag item_type 1: print the `ExeNanoXonY` line (nano name, item name), `N3Msg_CastNanoSpell(nano, item identity)`, accept. Client: `nanoprograms.rs` needs a drop target for an inventory-item payload and a cast message whose target is an inventory item (wire form unknown: capture of the original casting a nano on an item). LARGE / needs a capture.

---

### 2. NCU window (`NCUView_c`, FUN_100d774b, list class `NCUListView_c` FUN_10045d42) and the target NCU windows (`TargetNCUView_c`, FUN_100dda60; client `ncu.rs`)

The list is in the OLD mouse scheme (FUN_10045d42 calls FUN_10041258 and never FUN_10041248). It connects: +0x2e4 -> FUN_10045259, +0x140 -> FUN_10045b79 (tooltip), +0x2e0 -> FUN_1004561f, +0x2e8 -> FUN_100457d1 (asm 10045df3-10045e32). Nothing on +0x2d8 (pick-up) or +0x2dc (drop). The ctor stores an identity at +0x370/+0x374 from its first argument: both windows pass a zero identity at creation (own: 100d789c, target: 100ddcff); how the target window later sets it was not traced.

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| C1 | Left click (plain) on a running nano | Nothing: old scheme left release emits pick-up (+0x2d8), which nothing is connected to | [V 1004141b asm 10041520-10041578, 10045d42 connects] | Opens the info page | `ncu.rs:296-297` | FIXED 2026-11-25 (was DIFFER): a plain click does nothing, `ncu::left_click_shows` |
| C2 | Shift + left click | Qualifier click -> FUN_10045259: info page `itemid://type/id` | [V 1004141b, 10045259] | Info page (any left click) | `ncu.rs:296` | MATCH for Shift (but see C1) |
| C3 | Ctrl / Alt + left click | Same as plain left: nothing | [V 1004141b: mask 3 only] | Info page | `ncu.rs:296` | FIXED 2026-11-25 (was DIFFER): nothing |
| C4 | Right click, released before MouseButtonHoldDelay (own NCU) | The click signal fires -> FUN_1004561f: when the list's stored identity (+0x370/+0x374) is 0,0: `N3Msg_RemoveBuff(nano identity)`; true -> system line "Deactivating friendly timed nanoprogram."; false -> "Can't remove hostile timed nanoprogram." (hard-coded English, emitted on GlobalSignals +0x17c). Qualifiers are NOT tested for the right button. This is the original's cancel | [V 10041aa4 old branch, 1004141b asm 10041520-10041535 (button 2, flags&0x400 clear -> 100414f9), 1004561f] | Opens a context menu (window entries + client entry "Cancel this nano") | `ncu.rs:303-320` | FIXED 2026-11-25 (was DIFFER): the right click takes the nano off and says the original's line, `ncu::right_click` |
| C5 | `N3Msg_RemoveBuff` rule | Refused (returns false) when the nano's skill flags (GetSkill(identity, 0, 2)) & 0xc100 != 0 (hostile 0x8000, 0x4000, 0x100); otherwise sends an info-item-remote message with code 0x41 and returns true | [V Gamecode 1001bee9] | Cancel sends `n3::cancel_nano` unconditionally and takes the nano off the list at once; no system line | `net.rs:11407-11416`, `ncu.rs:355` | FIXED 2026-11-25 (was DIFFER): a nano with flags 0x8000, 0x4000 or 0x100 stays, with the other line (`Columns::stays`). The nano is still taken off the list at once (the original waits for the server): left |
| C6 | Right press and HOLD for MouseButtonHoldDelay (own NCU) | Menu FUN_100457d1: ONE entry "LookAtNano" -> FUN_100452c2: info page. The later release does not click | [V 10041aa4 (timer +0x320), 100420fd, 100457d1, 100452c2] | Menu at once on release; entries: "List Mode", "Auto Arrange", and the client's "Cancel this nano" | `ncu.rs:303-320` | FIXED 2026-11-25 (was DIFFER): the held button's menu with "LookAtNano" alone |
| C7 | The extra "Cancel this nano" menu entry | Not in the original menu | [V 100457d1: only LookAtNano] | Present | `ncu.rs:313-316` | FIXED 2026-11-25 (was ASKED, NAN-060): the entry is gone; the right click cancels (C4) |
| C8 | Right click with Shift / Ctrl / Alt (own NCU) | Same as C4 (button 2 ignores qualifiers) | [V 10041520-10041535] | Menu | `ncu.rs:303` | FIXED 2026-11-25 (was DIFFER): as C4 |
| C9 | Window menu | FUN_100d758a: "ListMode" (FUN_1004157f: check = layout 1) and "AutoArrange" = FUN_10041637(0) (no "Don't Auto Arrange"/"Unsorted"), dead in list mode. Where it opens from was not found | [V 100d758a, 1004157f, 10041637] | "List Mode" + "Auto Arrange" sub-menu opened on any right click over the window area, even over a nano | `ncu.rs:303-320`, `arrange_menu(.., false)` | FIXED 2026-11-25 (was UNSURE for the trigger): the frame's menu button, or the page's in the rollup column; a right click beside the nanos does nothing |
| C10 | Drag a nano out / drop on the list | No pick-up or drop slot connected: nothing | [V 10045d42 connects] | Dragged events are not used by `ncu.rs` | `ncu.rs:295-298` | MATCH |
| C11 | Hover tooltip | FUN_10045b79 ("Modifies", "NCU usage", "Remaining") | [V summary and strings of 10045b79; not read line by line] | Own layout | `ncu.rs:322-345` | FIXED 2026-11-25 (was ASKED, NAN-059): the NCU list's own tip, FUN_10045b79: the type's word, "NCU usage", "Remaining" as hh:mm:ss, "Modifies" (`ncu::tip`) |
| C12 | Escape | NCU reads no `esc_*` | [NF in 100d774b strings] | Not closable by Escape | `ui.rs:420-424` comment | MATCH |
| C13 | Target NCU window ("Fight Target NCU" / "Nano Target NCU"): left click | Same list class and rules as C1-C3: plain left nothing, Shift left info | [V 100dda60 builds FUN_10045d42; rules as C1] | Any left click opens the info page | `ncu.rs:496-499` | FIXED 2026-11-25 (was DIFFER): the info with Shift only |
| C14 | Target NCU: right click | Quick: FUN_1004561f (gate: stored identity 0,0, which the target window sets later: not traced; if it holds the target the click does nothing); hold: "LookAtNano" menu (100457d1) | [V 10045d42, 1004561f; identity set-up NR] | Menu with "ListMode" and "HostileOnly" on any right click | `ncu.rs:502-512` | FIXED 2026-11-25 (was UNSURE): the menu is the frame's icon menu's (`GetIconMenu`, 0x100de008): the frame's menu button opens it; a right click does not |
| C15 | Target NCU window menu | FUN_100dd94d: "ListMode", "HostileOnly" (check), handler FUN_100de3bd | [V 100dd94d calls] | Same two entries | `ncu.rs:502-515` | MATCH (entries) |
| C16 | Alt + double click borderless style of the target windows (FUN_100dd7ea) | Toggles borderless | [NR; doc says so in `NAN-053`] | Not ported | `ncu.rs` comment l.~422 | MISSING (known, NAN-053) |
| C17 | Column header click / wheel | | [NR] | | | UNSURE |

#### Differences to port (NCU)

- C1/C3/C13 plain (and Ctrl/Alt) left click opens the info page; the original does nothing. Rule: info page only when Shift is held on release. Same plumbing as N4. SMALL once the qualifier reaches the signal.
- C4/C5/C6/C8 right click = cancel, hold = Look-at menu. Rules, in order: right button DOWN starts a 0.3 s timer (MouseButtonHoldDelay); release before it: if the list is the own NCU (identity 0,0): refuse with the line "Can't remove hostile timed nanoprogram." when the nano's skill flags & 0xc100 != 0, else send the remove (IIR code 0x41; `n3::cancel_nano`) and print "Deactivating friendly timed nanoprogram."; timer fires first: popup with the single entry "LookAtNano" (info page itemid). Qualifiers do not matter for the right button. Client: `ncu.rs` `show()` right-click block (l.303-320) and the `cancel` plumbing (`ui.rs:1457`, `net.rs:11407`); needs the nano's flags (`NanoInfo.columns.kind == 2` is hostile 0x8000 only; 0x4000 and 0x100 are not kept in `Columns`) and a hold timer. MEDIUM. The window menu (List Mode / Auto Arrange) would then move to wherever the original attaches FUN_100d758a (not found: needs one more read of who registers it, probably the window's title-bar menu, `chrome::frame_menu`).
- C7 NAN-060 text should be corrected before the user answers it again.

---

### 3. Perks window (`PerkWindow_c`, FUN_10063ff8, `PerkView_c` FUN_10061a07; client `perks.rs`) and the Actions list with the perk actions (`SpecialActionView_c`, FUN_100dc9b1; client `actions.rs`)

The perk buttons are `Button_c`/`PerkLineView_c` (FUN_1005f134): they act on the left PRESS (ButtonBase_c::MouseDown 101298db, button 1 only). No list-view mouse scheme is involved. Which handler function each button signal reaches was not captured for the line buttons (the connect at 10061af5/10061b39 goes through FUN_10062c1e -> FUN_10062b97 with the target pushed in a register not shown by the decompile): the rank handler is FUN_10061021 by its body (reads the chosen list, checks points, owned, criteria); the line handler FUN_100607e3/FUN_10060f2d by its body (select index, set the header). Marked [V body] below.

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| P1 | Left press on a line button (left list) | The line becomes the chosen one of its tab (FUN_100607e3, header text via FUN_10060f2d); toggle button | [V body of 100607e3, 10060f2d; 101298db] | Chosen line set on `clicked()` (release) | `perks.rs:495-497` | MATCH (effect); press vs release SMALL |
| P2 | Left press on a rank button | FUN_10061021, in this order: alien perk with no alien points -> message (`HowToGetAPerks`); normal perk with no points -> message `HowToGetPerks`; already owned -> `YouKnowThisPerk`; criteria not met (or special) -> `DoNotMeetPerkReq`; rank before not owned -> `MustTrainPrecursoryPerk`; else a Yes/No question (`ReallyTrainPerk`, name, hours) | [V 10061021 (first branches read; the alien branch's text from the function's strings)] | `train()` returns the same answers in the same order; dialogs | `perks.rs:298-325`, `523-525`, `596-610` | MATCH |
| P3 | Yes on the train question | FUN_100605d3: `N3Msg_TrainPerk(id)` only when the dialog result is 0 (Yes) | [V 100605d3] | Returns `(id, true)` to the net layer (CharacterAction 0xbb per the module notes) | `perks.rs:573-594` | MATCH (client wire from the module's notes, not re-read) |
| P4 | "RemovePerk" button press | FUN_100616d2: first rank of the chosen line not owned -> `MustKnowPerk2Untrain`; else `ReallyRemovePerk` Yes/No (hours 2) | [V 100616d2] | `untrain()` same two answers | `perks.rs:328-342`, `548-552` | MATCH |
| P5 | Yes on the remove question | FUN_1006089f: `N3Msg_UntrainPerk(id)` then FUN_10060796 (button state) | [V 1006089f] | Sends untrain, sets `waiting` | `perks.rs:586-590` | MATCH |
| P6 | RemovePerk enabled state | FUN_10060796 (not read) | [NR] | on while not waiting and any perk point is used | `perks.rs:545` | UNSURE |
| P7 | Which rank the remove button takes off | "highest owned rank" as the module notes say; FUN_100616d2 uses FUN_1005ff0a(…,1) for the name | [partly V 100616d2] | Last owned rank of the run | `perks.rs:331-341` | UNSURE (the choice of rank was not traced line by line) |
| P8 | Right click / Shift / Ctrl / Alt + click on a perk button | Nothing: `ButtonBase_c::MouseDown` acts only on button 1 and ignores qualifiers | [V 101298db] | egui `clicked()` is the primary button only; no qualifier use | `perks.rs:495,523` | MATCH |
| P9 | Tabs (ProfessionPerks / GroupPerks / GeneralPerks) | Click selects; the selected tab is saved in `PerkWindowConfig` `SelectedTab` when the window goes (FUN_10064a67), restored at creation (FUN_10063ff8) | [V strings of 10063ff8, 10064a67] | Same, kept via `layout::set_tab("PerkWindowConfig")` | `perks.rs:565-570` | MATCH |
| P10 | Hover tooltips | FUN_1005f908 (Requirements, Wear, Modifies lines) on a `PerkLineView_c` | [V 1005f908 calls] | Tip on rank buttons only; line buttons have none | `perks.rs:522` | UNSURE (whether the line buttons carry the same tooltip was not read) |
| P11 | Escape | `esc_perkwindow` (shipped true) | [V 10063ff8 strings; LoginPrefs.xml:70] | `"Perks" => "esc_perkwindow"` | `ui.rs:449` | MATCH |
| P12 | Press vs release | Every Button_c acts on press | [V 101298db] | egui acts on release (all buttons in all three windows) | `perks.rs`, `nanoprograms.rs:492`, | MATCH for the plain buttons (corrected 2026-11-25: a plain button of the original acts on the RELEASE over it, `ButtonBase_c::MouseUp` 0x101294c9 with the `ModifyWhenClicked` flag `Initialize` sets at 0x10129bd0; the first reading took `MouseDown`'s other branch for the rule). FIXED the same day: a perk line's button, which is a toggle told `SetModifyWhenClicked(false)` (FUN_1005f134), acts on the press; and a plain button's release counts however long it was held (`chrome::on_release_inside`) |
| P13 | Drag from or drop on the perk window | None found (no BeginDrag in the perk view functions) | [NF] | none | | MATCH |
| P14 | "Perk actions" list = the Actions window (`SpecialActionView_c`): left click | `N3Msg_PerformSpecialAction(identity)` (FUN_100dc033, click signal +0x2e0) | [V 100dc033, ctor connects asm 100dcb8a-100dcbe2] | `self.clicked = Some(action)` -> performed | `actions.rs:503-510` | MATCH (effect) |
| P15 | Actions list: Shift + left click | Qualifier click -> FUN_100dc3ae: info page `itemid://type/id` | [V 100dc3ae, 100dcbbc] | Performs the action | `actions.rs:503` | FIXED 2026-11-25 (was DIFFER): the info page |
| P16 | Actions list: left hold 0.3 s | Pick-up -> FUN_100dc1ab: BeginDrag of the action onto a bar | [V 100dc1ab, 10041aa4] | Drags after 3 px, payload `Action::Special` | `actions.rs:512-516` | FIXED 2026-11-25 (was DIFFER): held 0.3 s |
| P17 | Actions list: right click | No menu slot connected (only +0x2d8,+0x2dc,+0x2e4,+0x2e0): nothing | [V ctor 100dcb8a-100dcbe2] | none | `actions.rs` | MATCH |
| P18 | Actions list: drop a dragged action on a cell (grid layout) | FUN_100dc274: item_type 6 drag: MoveItem to the cell (or remove/add across lists), accept | [V partial 100dc274] | No cell placing; items in the list's order | `actions.rs:455-470` | MISSING (probably; `WIN-022` covers the list contents, not placing) |

#### Differences to port (perks / actions)

- P15 Shift click on an action -> info page instead of performing. Same plumbing as N4; client `actions.rs:503`. SMALL.
- P16 as N2 (hold-to-pick-up). MEDIUM, shared.
- P18 cell placing in the Actions grid: rule as N15 (`MoveItem(from, cell)`, accept, displaced item picked up). Client `actions.rs` would reuse `places::` as `nanoprograms.rs:452-466` does. MEDIUM.
- P12 press-vs-release for Button_c: buttons should fire on left press. Client: `chrome::button`/`perk_button` use `Response::clicked()`; changing to `pointer.primary_pressed()` inside the rect for these controls. SMALL, but it would be a global change in `chrome.rs` (check with the user: it changes "press, move away, release to cancel").

---

### Counts per verdict (rows above; "covered by" and pure notes not counted)

- Nano window (22 rows): MATCH 12 (N1, N5, N7, N8, N9, N10, N11, N13, N14, N19, N20, + N12 entries), DIFFER 2 (N2, N4), MISSING 1 (N16), UNSURE 6 (N3, N15 arranged-grid, N17, N21, N22, N12 trigger), ASKED 1 (N18).
- NCU / target NCU (17 rows): MATCH 5 (C2, C9 entries, C10, C12, C15), DIFFER 7 (C1, C3, C4, C5, C6, C8, C13), ASKED 2 (C7, C11), MISSING 1 (C16), UNSURE 2 (C14, C17).
- Perks + Actions list (18 rows): MATCH 10 (P1-P5, P8, P9, P11, P13, P14, P17), DIFFER 3 (P12, P15, P16), MISSING 1 (P18), UNSURE 3 (P6, P7, P10).
- ADDITION rows: none found beyond C7 (client-only "Cancel this nano" entry, counted ASKED) and the client's move-drag at 3 px (inside N3, UNSURE).

### Not read / not found

- Where the nano window's menu FUN_100d4f90 and the NCU window's menu FUN_100d758a are attached (no connect found; probably the window frame's icon menu: `GetIconMenu` at 100de008 is used for the target windows). Whether a right click on empty list background opens it in the original is not established.
- `MultiListView_c::MouseMove` 10133dba and `MouseDown/MouseUp` of the base list view: whether a plain press-move raises pick-up, and how a quick release cancels the hold timer (the up handler shown does not stop the timers; the base class or the mouse capture release presumably does).
- `ColumnHeaderView_c` header clicks (sort), column resize, label drag: not read in the original.
- Mouse wheel in all three windows (scroll view): not read.
- The target NCU window's setting of the list's identity (+0x370/+0x374) and so whether right click removes anything there; FUN_100dd7ea (Alt + double click borderless), FUN_100de3bd (HostileOnly handler), 100de31f/100de2d0 (window hooks).
- The tooltip texts FUN_10042193 / FUN_10045b79 / FUN_1005f908 line by line (client tips are the client's own, ASKED NAN-059).
- Perk window: the alien branch of FUN_10061021 (read only from its strings), FUN_10060796 (remove button enabled state), the exact signal target of the line and rank buttons (FUN_10062c1e -> FUN_10062b97 slot target address), whether line buttons show a tooltip, the blink/timer functions (FUN_10060b04, 10060514, 10060d7e).
- `research.rs`: not read (not part of the perk window).
- The pointer-type changes of FUN_10041ca4 (Shift cursor etc.) against the client.
- Keys: only Escape was checked; no default `CharPrefs.xml` key for these windows' interactions was looked up beyond the window-open keys the client already has.

## Chat windows, tabs, tell windows


Read-only comparison. Original = GUI.dll decompile (`exports_gui/<addr>.json`, helper scripts `gf.py`, `calls.py`, `gdis.py`, `vtable.py`, `gxref.py`).
Client = `crates/mao-client/src/` as on disk on 2026-11-25 (uncommitted changes included).
Marks: **[V addr]** read at that address, **[NF]** looked and not found, **[NR]** not read.

Key facts I established first (everything below leans on them):

- Mouse routing of a chat window. `ChatView_c` (ctor 0x10090d87; flags word at +0x1b0: chat windows pass 0xffff, `TellWindow_c` passes 1 [V 0x100ac79b, 0x100b2515]) has MouseDown 0x1008fa92, MouseUp 0x1008dd7b, MouseMove 0x1008ddad [V vtable.py ChatView_c]. Its text child is a `TextView_c` with feature flags 0xe6c [V 0x10092c6a]: bit 4 (selectable) set, bit 1 (editable) not set. Press on a link is raised by `TextRenderer_c::MouseDown` 0x10163bcd, relayed by `TextView_c::SlotHyperLinkSelected` 0x10002c98 (signal +0x1c4) to `ChatView_c` handler 0x1008e7d7 [V connect at 0x10091024].
- Qualifier bits (`WindowController_c::GetQualifiers` 0x10156df6): the name and group handlers test `& 3`, so only **Shift** matters; Ctrl and Alt are ignored there [V 0x100ac4c4, 0x100ac4f0]. The client's `controls::qualified` (gui/controls.rs:747) is Shift only, which agrees. Since 2026-11-24 `ui.held_keys` (ui.rs:2018, viewer.rs:1462) feeds Shift/Ctrl/Alt to egui, so these clicks can work now; nothing written before that proves them.
- Key codes of the original's text input (from the key-name table in GUI.dll, ascending): 0xe ESC, 0xf TAB, 0x18 ENTER, 0x19 BACKSPACE, 0x1a INSERT, 0x1b DELETE, 0x1c HOME, 0x1d END, 0x1e PGUP, 0x1f PGDN, 0x20 Left, 0x21 Right, 0x22 Up, 0x23 Down.
- Default keys that touch chat (GUI.dll key table text, read from the binary; `CharPrefs.xml` has no key entries, only `ChatLastActiveWindow`, `ChatPGInviteAction`, `ChatTextShadowOffset`, `ChatWarnWhenSpeakingToUnsubGroups`; `OptionPanel/HotKeys.xml` just lists the three fixed chat keys):
  - PGUP / PGDN `[RepeatMode & AnyPlayMode]` (work while typing too) = chat history page up / down
  - ENTER, SHIFT+ENTER, ALT+ENTER `[AnyPlayMode ! TextInputMode]` = `COMMAND_TEXT_INPUT_START_TEXT_INPUT`
  - SLASH, NUMPAD_DIV = `COMMAND_START_CHAT_CMD_INPUT`
  - SHIFT+R = `COMMAND_START_CHAT_REPLY_INPUT`
  - SHIFT+C = `COMMAND_OPEN_CHAT_CONFIG_WINDOW`
- Client key table: `settings.rs:320-324` (ChatInput Enter/Shift+Enter, ChatCommand Slash/NumpadDivide, ChatReply Shift+R, ChatPageUp/Down). Only ChatInput and ChatCommand have a handler (`viewer.rs:1711-1721`); `grep Action::Chat` over the crate finds nothing else.

---

### 1. Tabs (tab strip of a chat window; the tabs sit in the window's title row since 2026-11-24)

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| T1 | Left press on a tab | Selects the tab: `Tab` MouseDown emits the "pressed" signal (+0x1a8) whatever the button, then falls to `View::MouseDown` for the left button | [V 0x10146d91] (select slot itself 0x1009740f has no export: [NR] what it does besides switching) | Tab opens on press (and on click) | `chrome.rs:1304` (`tab_view_in`) | MATCH |
| T2 | Right press on a tab | Selects the tab too (the emit comes before the button test), then remembers the press (+0x1d5=1, point) | [V 0x10146d91] | Same line selects on `is_pointer_button_down_on` / `clicked`; whether egui counts the right button for `is_pointer_button_down_on` was not checked | `chrome.rs:1304` | UNSURE (does a right press select the tab first?) |
| T3 | Right button let go on the same tab: take the tab up | `Tab` MouseUp: if the release is inside the tab, +0x1d4==0, +0x1d5 set and button==2: emits +0x1ac, `TabView::SlotBeginDrag` 0x101459dc, `ChatWindow` FUN_10097af7 starts a `DragObject_c` (mime `chat_gui/group_chat_view`, int `tab_index`, image `CreateDragImage`). No menu comes | [V 0x10146ef3, 0x101459dc, 0x10097af7; connections [V 0x1009829a]] | `secondary_clicked` on a tab sets `carried` with the grab offset; the window menu is not opened for it | `chrome.rs:1307`, `chatwindow.rs:1989-1993` | MATCH |
| T4 | Put the carried tab down on a tab / on a strip | Any release OR press of any button over a tab while a drag object exists emits +0x1b0 (`TabView::SlotItemDropped` 0x10145a95, then +0x150) and `ChatWindow` FUN_10098038 moves that chat window into this window's tab view at the place; a window left without tabs goes | [V 0x10146d91 (press branch), 0x10146ef3 (release branch), 0x10098038 (mime check, `tab_index`)] | Only the **left click** (`primary_clicked`) puts it down: before the tab under the pointer, or last on a strip; a frame left empty is gone; dropped on itself does nothing | `chatwindow.rs:2041-2102` (`carry`, 2066, 2072-2088) | DIFFER (button) |
| T5 | Put the carried tab down elsewhere | Only when the release finds the pointer over **no window** (`WindowController_c::HandleMouseUp` runs the drag object's +0xc signal when +400==0) the source's FUN_100984c2 runs: with fewer than 2 tabs `DragObject_c::Cancel`; else a new `ChatWindow` is made with the source window's bounds moved by the drop offset and put inside the screen | [V 0x10157f13, 0x100984c2]; what +400 means is my reading of HandleMouseMoved (0x10158872 writes it): window under the pointer, [NR] checked further | Any left click not on a strip makes a window of its own at `pointer - grab` in the default size, unless the tab is the only one of its frame; also when the pointer is over another window | `chatwindow.rs:2089-2096` | DIFFER (pointer over another window; size of the new window; button) |
| T6 | Escape while a tab is carried | No Escape handling found in the drag code | [NF in 0x10157842, 0x1012ba1c, 0x10157f13] | Escape lets go of the tab | `chatwindow.rs:2062-2065` (doc line 2039 says it is the client's own) | MATCH (corrected 2026-11-25: Escape giving a carried tab up IS the original's, `WindowController_c::BeginDrag` 0x10157842 connects the drag's `Cancel` to the Escape signal; and FIXED the same day for a real keyboard, which never reached the client's test: `Ui::close_with_escape` -> `ChatWindow::cancel_carry`) |
| T7 | Double click on a tab | Nothing found (Tab has MouseDown/MouseUp only) | [NF 0x10146d91, 0x10146ef3; View double-click slot not read] | Nothing | - | UNSURE |
| T8 | Tab's own menu | None: right button on a tab begins a drag instead | [NF in Tab handlers] | None | `chatwindow.rs:1989` | MATCH |
| T9 | Window menu entries that move a tab ("Move to a window of its own", "Move into the window of X") | Not in the original's menu | [V 0x100abb98 entry list has none] | Present in the window menu | `chatwindow.rs:873-886`, choose 1003-1013 (CHT-001 "menu entries stay as the client's own") | FIXED 2026-11-25 (the ADDITION is gone, CHT-001): a tab is carried by the mouse |

**T4, T5 rule to port.** Drop on a tab strip: any mouse press or release over a tab (or the strip) while a drag object exists, not only a left click. Drop outside: only when no window is under the pointer at the release (a press/release over another window does not make a new window), the source frame must have 2 or more tabs, and the new window takes the source window's size and is clamped to the screen. Change `ChatWindow::carry` (chatwindow.rs:2041): test `pointer.any_released()` / `any_pressed()` instead of `primary_clicked`, and for the empty-screen case test that `ctx.layer_id_at(pos)` is none of the client's windows; size from `crate::layout` of the source frame. SMALL.
**T2** to settle: check egui's `is_pointer_button_down_on` for a right press, or select explicitly in `tab_view_in` when `response.secondary_clicked()` / right press. SMALL.

---

### 2. Text area (lines, links, names)

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| X1 | Left press on a `user://` link (character name), no Shift | Open the tell window (type-2 request to the chat server; the server's answer opens it, own name refused) | [V 0x10163bcd (emits link on MouseDown), 0x1008e7d7 (`user://` -> signal +0x130), 0x100ac4c4 (`GetQualifiers & 3 == 0` -> 0x100ac2fa)] | `ask_tell(name)` | `chatwindow.rs:2210`, `1322` | MATCH |
| X2 | When the name link fires | On the button **press** | [V 0x10163bcd, link branch under `case 1`] | On the **click** (release) of the line widget | `controls.rs:783-789` (`response.clicked()`) | FIXED 2026-11-25 (was DIFFER): the link acts on the press, `controls::markup_click` through `chrome::on_press` |
| X3 | Shift + left on a name | Types `/tell <name> ` into the input and starts typing | [V 0x100ac42b: "/tell " + name + " ", then FUN_1008f24c(1, text)] | `type_in(frame, "/tell {name} ")` | `chatwindow.rs:2209` | MATCH |
| X4 | Ctrl or Alt + left on a name | Same as plain (only bits 0-1 are tested) | [V 0x100ac4c4] | `qualified` is Shift only, so same as plain | `controls.rs:747` | MATCH |
| X5 | Right press on a name (chat window) | Name menu at the pointer, no window menu: `IgnoreUser` (flag 1 -> +0x148 -> FUN_100ac39e), `OpenChat` (flag 2 -> +0x14c -> FUN_100ac2fa), `SendTell` (flag 4 -> +0x150 -> FUN_100ac42b); a chat window's flags are 0xffff so all three | [V 0x1008fa92 (right branch: `user://` -> FUN_1008e5ea -> `PopupMenu_c::Go`, then returns), 0x1008e5ea, 0x1008e1d9/0x1008e27b/0x1008e31d, connects 0x100ac870-0x100ac8a9] | `open_name_menu`: the same three entries; ignore toggles, chat -> `ask_tell`, tell -> `type_in("/tell name ")`; the window menu is suppressed (`on_name`) | `chatwindow.rs:807-812`, `choose` 957-965, `1999` | MATCH |
| X6 | Right press on a name in a **tell window** | Only `IgnoreUser` (the tell window builds its ChatView with flags 1) | [V 0x100b2515 pushes 1 as the 4th argument, stored at +0x1b0 by 0x10090eb6; menu built from flags in 0x1008e5ea] | All three entries (`open_name_menu` with the tell frame) | `chatwindow.rs:2208`, `807` | DIFFER |
| X7 | Left press on a name in a tell window | Nothing: the tell window's ctor connects only +0x12c, +0x138, +0x148, +0x154 of its ChatView, not the +0x130 name signal | [V gdis 0x100b243e-0x100b2590] | `ask_tell` (brings up / asks for a window) | `chatwindow.rs:2210`, `1322` | DIFFER (low) |
| X8 | Left press on a `chatgroup://` link | Shift not held and the window shows the group and can talk to it: that window's output group becomes the group (FUN_1009a7c0); else types `/group name ` (name in quotes when it has white space) and starts typing | [V 0x100ac4f0, 0x10099fb0, 0x1009a7c0] | `group_pressed`: same two branches, same quoting | `chatwindow.rs:1246-1260`, `104-106`, `2211-2216` | MATCH |
| X9 | Left press on `itemref://`, `itemid://`, `text://` | `ChatGUIModule_c::ShowItemRefLink` -> `InfoViewModule_c::ShowURL`; no qualifier is read | [V 0x1008e7d7, 0x100861e0] | Link goes to the info view (any key state) | `chatwindow.rs:2217`, `ui.rs:1349`, `gui/infoview.rs:59-83` | MATCH |
| X10 | Left press on a `charref://` link | Also given to the info view (character page) | [V 0x100861e0] | `InfoView::show_url` has no `charref://` branch (it only opens the window) | `gui/infoview.rs:59-83` | MISSING |
| X11 | Left press on a `chatcmd://` or `file://` link in chat | Nothing from the chat view: `ShowItemRefLink` returns for every other scheme; `chatcmd://` is parsed only in the info view's own handlers | [V 0x100861e0; strings `chatcmd://` only in 0x100ee194, 0x100ee973, 0x1003482c, 0x1004cb15; [NF] any other slot on this view's link signal] | Every link that is not a name or a group is handed to `info_view.show_url`, which runs `chatcmd://` as a chat command and opens `file://` pages | `chatwindow.rs:2217`, `gui/infoview.rs:73-77` | ADDITION |
| X12 | Right press on an item/other link or plain text | Window menu at the pointer (the window's `GetIconMenu`), and when the chat view has no input bar an "Edit" submenu is put in at index 2 | [V 0x1008fa92: the "Edit" entry is made only when FUN_10091198() == 0 and +0x1a8 == 0; FUN_10091198 returns 1 while an input bar (+0x158/+0x15c) exists; `GetIconMenu` 0x101545b3] | Main window menu at the pointer; no Edit submenu | `chatwindow.rs:2008-2013`, `2200-2203` | DIFFER (see X14) |
| X13 | Right press on the window outside the text frame (input bar, margins) | Same window menu | [V 0x1008fa92: Looper vtable +0x3c is called whatever the point] | Window menu for any right press inside the window rectangle that is not a name or tab | `chatwindow.rs:2008-2013` | MATCH |
| X14 | "Edit" submenu entries | `Select All` (disabled when the text is empty), `Copy` (disabled when nothing is selected), `Paste` only for editable views (not the chat text) | [V 0x1016305c: texts `GTK_ChatView_EditMenu_SelectAll`, `_Copy`, `_Paste`] | None | - | MISSING |
| X15 | Left press on text (no link): start selecting | Cursor placed, selection begun, mouse captured; selection drag extends it (MouseMove 0x10163da4, auto-scroll timer 0x10160964); the left release ends it, an empty selection is dropped | [V 0x10163bcd (case 1 with flag 4), 0x10160167] (MouseMove not read) | No selection: each line is a click-sensing widget, drawn with `paint` only | `chatwindow.rs:603-614`, `controls.rs:773-789` | MISSING |
| X16 | Copy of the selected text | Menu `Copy`; key chord [NR] (`TEXT_COPYTOCLIPBOARD` is a `TextInputModule_t` message, 0x10022917) | [V 0x1016305c] | None | - | MISSING |
| X17 | Left click on text with no selection and no link | Starts typing (FUN_1008f24c(1,0): focus the input bar, show a hidden one) | [V 0x1008f80f: `GetSelectionRange` false and `GetHyperLink` false] | Only for a window that hides its bar (`look.hide_input`), never for a visible bar | `chatwindow.rs:2015-2017` | DIFFER |
| X18 | Press anywhere outside the chat window | Typing ends (FUN_1008f24c(0,0) when the point is outside the window's frame) | [V 0x1008f86a, connected at 0x1009115e from the global mouse-down signal] | egui drops the field's focus on a press elsewhere | (egui behaviour) | MATCH |
| X19 | Mouse wheel over the text | `HandleMouseWheel` -> `Window::_HandleMouseWheel` -> the default wheel view or the focus child (`+0x90`); `ScrollView_c::WheelMoved` scrolls by `delta.y * step (+0x154)` when the content is taller than the view, else hands the wheel to its parent | [V 0x10157cec, 0x10154ab9, 0x10142820]; the step value is [NR] (no store of +0x154 in the ScrollView ctors 0x101436f2, 0x101437ae; set elsewhere) | egui `ScrollArea` with its own speed, `stick_to_bottom` | `chatwindow.rs:571` | UNSURE (lines per notch unknown) |
| X20 | Shift / Ctrl / Alt + wheel | No qualifier test in `WheelMoved` | [NF 0x10142820] | egui defaults (Ctrl+wheel may be taken by egui for zoom, not checked) | - | UNSURE |
| X21 | Page Up / Page Down | Scroll the active chat window by a page: `TextInputModule_t::PageUpMessage` 0x100225ca emits GlobalSignals +0x1b4 with 1.0 (Down: presumably -1.0, [NR]); `ChatGUIModule_c::SlotScrollPage` 0x10086253 picks the chat window (FUN_10093ff2, else FUN_1009401d: first one with input enabled) and FUN_100925e9 scrolls by `max(lineH, floor((viewH + k) / lineH) * lineH - 2 * lineH) * amount`, with `lineH` = height of "-" + k | [V 0x100225ca, 0x10086253, 0x100925e9, connect 0x10087bee; keys: key-table lines] | Actions exist, nothing handles them | `settings.rs:323-324`, no handler in `viewer.rs` | MISSING |
| X22 | Drag the border between text and input bar | With two children shown (+0x1ac==2) a left press between them begins a resize drag (`+0x1bb`), MouseMove changes the input area's height (+0x1b4, clamped); mouse pointer changes | [V 0x1008fa92 (button 1 branch), 0x1008ddad (partly read)] | None | - | MISSING |
| X23 | Drop an item or a character reference on the chat window | The whole chat view is a drop target (0x100900bf): item (type 1) gives an `itemref://` link, character (type 3) a `charref://` link, inserted at the cursor of the input, input started, drag accepted | [V 0x1008f98f, 0x1008f0e2, 0x1008edd0, 0x100900bf] | No drop target; an item's menu entry "Create Reference" adds the link instead | `chatwindow.rs:1200-1207` (`add_text`), `ui.rs:1656`, `ui.rs:1715` | MISSING (the menu route is the client's own, ADDITION) |
| X24 | Double click / word select in the text | No double-click branch in `TextRenderer_c::MouseDown`; view double-click slot not read | [NF 0x10163bcd] | None | - | UNSURE |
| X25 | Timestamps, fading, colours | Not interactions | - | - | - | (skipped) |

**Rules to port.**
- X10: in `InfoView::show_url` (gui/infoview.rs:59) add `charref://<id>/<id>/<name>` and show the character page (the original goes through `InfoViewModule_c::ShowURL` after `ShowItemRefLink`'s four-scheme test). SMALL once the character page exists; otherwise MEDIUM.
- X11: nothing the player misses; if wanted exact, the chat window should pass on only `itemref://`, `itemid://`, `charref://`, `text://` (`chatwindow.rs:2217`). SMALL.
- X12/X14: when a chat window has no input bar (`look.input == false`), put a submenu "Edit" at index 2 of the window menu (after Visual and its separator): `Select All` (disabled when the window's text is empty), `Copy` (disabled with no selection). Needs a selection (X15). Client function: `ChatWindow::menu_tree` (chatwindow.rs:815). MEDIUM.
- X15/X16: selectable chat text: left press on text without link places a cursor, drags extend, drag beyond the edge scrolls (timer), a release with an empty range clears; selection painted with `SELECTION` 0xc0c0c0 as `TextView` already does for views that read (controls.rs:793-830); copy to the clipboard. Replace the per-line `markup_click` sensing (controls.rs:754) with one selection model across lines. LARGE (the lines are separate widgets and are rebuilt each frame).
- X17: in `frame_body` (chatwindow.rs:2015) drop the `look.hide_input` condition: a left press in the text area that is not on a link and with no selection calls the same code `focus_frame = frame` and shows the bar (`bar` at 1968). SMALL.
- X21: handle `Action::ChatPageUp/Down` in `viewer.rs` next to ChatInput (1711), valid also while typing and with key repeat, and scroll the active chat window by the page above; add a `scroll_by` to `draw_lines`' ScrollArea (chatwindow.rs:571). SMALL-MEDIUM.
- X22: low value; skip unless the input height is saved (`input_height` key in the window config [V 0x10090fbd]). MEDIUM.
- X23: make the chat window a drop target for the client's item-in-hand/dragged item and for a dragged character: insert `itemref://<low>/<high>/<ql>` anchor (`<a href="itemref://%u/%u/%u">name</a>` [V 0x1008f0e2]) or `charref://` anchor (0x1008edd0) at the cursor, start typing. Client: `ChatWindow::add_text` already inserts at the end. MEDIUM (needs the drag payload from invwindows/gear).
- X6/X7: tell window frames: name menu only `IgnoreUser`; left press on a name does nothing. `chatwindow.rs:2208-2210`: when the click came from a tell frame (`frame` starts with `Tell:`), filter. SMALL.
- X2: only if wanted: raise the link on press. SMALL.

---

### 3. Input line

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| I1 | Enter / Shift+Enter (not typing) | Starts typing (`COMMAND_TEXT_INPUT_START_TEXT_INPUT`) | key table | `Action::ChatInput` -> `ChatWindow::focus` | `settings.rs:320`, `viewer.rs:1711`, `chatwindow.rs:1216` | MATCH |
| I2 | Alt+Enter (not typing) | Starts typing | key table | Alt+Enter is `CycleWindowMode` | `settings.rs:317-320`, `viewer.rs:1885` | FIXED 2026-11-25 (was DIFFER): Alt+Enter starts typing (`chat_input`'s third key); the window mode keeps F11, the client's options window and the launcher. Typing starts in the chat window itself, where the original picks the default window or the one last typed in (FUN_1009401d): left |
| I3 | `/` or NumpadDivide | Starts typing with the text `/` | key table, FUN_100225a... `StartChatCmdMessage` 0x10022785 emits "/" | `type_text("/")` | `viewer.rs:1716`, `chatwindow.rs:1209` | MATCH |
| I4 | Shift+R | Types `/tell <name> ` of a character from the reply history (`ChatGUIModule` +0x288); `StartChatReplyMessage` 0x1002283d emits GlobalSignals +0x1b0, handler FUN_10094dd5 builds "/tell " + name from the list | [V 0x1002283d; 0x10094dd5 by its summary and the +0x288 uses; cycling on repeated presses [NR]] | Action exists, no handler (only the `/reply` command works) | `settings.rs:322`, `chatwindow.rs:1546` | MISSING |
| I5 | Shift+C | Opens the chat configuration window (`COMMAND_OPEN_CHAT_CONFIG_WINDOW`; handler toggles `chat_group_window` as the menu entry does, 0x100ab022) | key table; [V 0x100685fa `OpenChatConfigWindowMessage`] | No key | `settings.rs` (absent) | MISSING |
| I6 | Enter in the field | Line stripped; if not empty: kept in the history (an equal one moved last, oldest dropped at 200), sent, field cleared; Enter on an empty line only ends typing; "Deactivate On Send" decides if the bar keeps typing | [V 0x1008f5b5, 0x100ab37b] | `run_in`, history kept the same way; surrender focus unless `look.deactivate` is off | `chatwindow.rs:1876-1897`, `1374-1386` | MATCH |
| I7 | Shift+Enter in the field | `TextRenderer_c::KeyDown` case 0x18: Shift held (or flag 0x4000) -> deletes the selection and inserts a newline instead of sending. (Whether the chat field passes this case: its flags are 0x800f, no 0x4000; whether `InputBar`'s key signal 0x100902ac sees Enter first: only the Alt case is handled there) | [V 0x10164224 case 0x18; 0x100902ac key 0x18] | egui single-line field: Shift+Enter sends | `chatwindow.rs:1876` | UNSURE |
| I8 | Alt+Enter in the field | Raises +0x138 (arg true; false with Shift) -> `ChatWindow` FUN_100ab0d7 -> FUN_10094a78 / FUN_10094c27: goes through the chat windows (those with input enabled, +0xd0 / +0x167) and moves typing to the next / previous one (FUN_10099084(1) opens its input) | [V 0x100902ac (key 0x18, `& 0x30`), 0x100ab0d7, 0x10094a78, 0x10094c27 by structure; direction names are my reading] | Alt+Enter is taken by the window-mode key before the field | `viewer.rs:1885` | FIXED 2026-11-25 (was MISSING): the typing goes to the next window that takes text, by name, with Shift the one before (`chatwindow::next_typing`) |
| I9 | Escape in the field | Clears the text and the selection and ends typing | [V 0x100902ac key 0xe] | egui's field gives up focus on Escape and keeps its text (egui behaviour, not read here); Escape otherwise closes windows | `chatwindow.rs:1876-1897`, `ui.rs:739` | DIFFER |
| I10 | Up / Down | History through the lines sent: Up (0x22) saves the line being typed (and its cursor) as a draft when nothing was recalled, then goes to the older one; Down goes newer and **past the newest restores the draft**; each history line keeps its cursor | [V 0x1008e9c5] | Up/Down recall (when no command list is open); past the newest it sets an empty line, no draft | `chatwindow.rs:1852-1863`, `recall` 1389-1402 | DIFFER |
| I11 | Tab with text | Key 0xf, no Ctrl: command / name completion by the collector signal (+0x13c): one match completes the word, several show a list (FUN_10093d1e / FUN_1008f3d8); a Space after `/word` also completes the command word (key 0x17) | [V 0x100902ac keys 0xf, 0x17] | The client's own list of commands: Up/Down move, Tab or Enter takes it, mouse picks and scrolls (asked 2026-11-21) | `chatwindow.rs:1840-1938`, doc CHT-035 | ASKED (kept on the user's word of 2026-11-25: asked "Keep the five extras you asked for earlier, which the original does not have?", the answer was "Keep them"): the chat's command list while a command is typed (CHT-035); the original completes by Tab into a pop-up of the candidates (FUN_100a44e8, FUN_1008f3d8) |
| I12 | Tab on an empty field, or Ctrl+Tab | Opens a pop-up of the channels this window can talk to (sorted, the current one ticked and highlighted) at the field's top left; a pick sets the output group | [V 0x100902ac (key 0xf: empty text or Ctrl -> +0x144), 0x1009aae8, 0x1009a9fc] | None; only the window menu's "Talk to Channel" | `chatwindow.rs:851-855` | FIXED 2026-11-25 (was MISSING): the list of the groups the window can talk to, the current one marked and highlighted, over the field (`open_channel_list`) |
| I13 | Ctrl+V / Shift+Insert | Paste (`PasteFromClipboard`; Ctrl bits `& 0xc`, Shift bits `& 3`) | [V 0x10164224 cases 0x67, 0x1a] | egui text field (Ctrl+V, Shift+Insert as egui does) | `chatwindow.rs:1871` | MATCH |
| I14 | Click on the channel name label of the bar | No click handler: the label's signals only re-lay out the bar | [V 0x10092101-0x10092160 all connect FUN_10091426] | No label | - | MATCH |
| I15 | Left click in the field | Cursor placed (text renderer, flag 1) | [V 0x10163bcd] | egui field | `chatwindow.rs:1871` | MATCH |

**Rules to port.**
- I4: `Action::ChatReply` in `viewer.rs` (next to 1716): `type_text("/tell <last teller> ")` using the newest sender of `HandlePrivateMessage` (`last_tell`, chatwindow.rs:1072); the original keeps a list (`GetReplyHistory`), repeated presses probably step back through it [NR]. SMALL.
- I5: `Action::OpenChatConfig` on Shift+C when not typing -> `self.config.open = !self.config.open`. SMALL.
- I8: Alt+Enter while the field has the keys must reach the chat, not `CycleWindowMode`: let the viewer's window-mode shortcut yield while `frame_typing`; then move `focus_frame` to the next (Alt) or previous (Alt+Shift) frame whose bar is shown. `input_bar` (chatwindow.rs:1819), `viewer.rs:1885`. SMALL-MEDIUM.
- I9: on Escape in the field: `self.input.clear()` and drop focus before egui does (`input_bar`, 1871). SMALL.
- I10: keep `draft: Option<String>`; at the first Up from the end store the input; `recall` returns the draft when Down passes the newest (chatwindow.rs:1389). SMALL.
- I12: Tab/Ctrl+Tab on an empty field opens `Menu::Talk` at the field (the entries exist in `menu_tree`, chatwindow.rs:851): use `open_menu(ctx, at, Some(Menu::Talk))`. SMALL.
- I2: leave Alt+Enter with the field, move window-mode to the original's own key if one exists; otherwise keep. A decision for the user.

---

### 4. Window menu of a chat window (right press on the window, or the menu button of the frame)

Original menu body: FUN_100abb98 builds, in order: `ChatWindowMenu_TalkToChannel` (sub), `ChatWindowMenu_ChannelSubscribeMenu` (sub), `ChatWindowMenu_AutoSubscribeChannels` (tick), separator, `ChatWindowMenu_ChatConfiguration` (tick), separator, `RenameWindow`, `DeleteWindow`, `NewWindow`, separator; `FUN_10097a40` then inserts `ChatWindowMenu_Visual` (sub) and a separator in front [V 0x100abb98, 0x10097a40]. The client's list is the same order (`menu_tree`, chatwindow.rs:859-872).

| # | Entry / interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| M1 | Visual submenu entries (Mode: Normal / Borderless; Transparency: two sliders; ShowTimestamps; DisableTextInput; HideInputBarWhenInactive; FadeMessages; AlwaysBehind; AlwaysOnTop) | Names as listed; the effect of each is its slot | [V strings of 0x1009a00d and 0x10097756] (slots and enable rules [NR]) | Same names and order; effects in `choose` | `chatwindow.rs:822-848`, `954-1034` | UNSURE (names equal; effects not read in the original) |
| M2 | Talk to Channel | Entries = groups the window can talk to (FUN_10099fef), sorted, current output ticked; `NoSelectableGroups` when empty | [V 0x100ab5dc, 0x1009aae8] | Same: sorted groups the pane selects and that take lines, ticked, "No groups to choose" | `chatwindow.rs:850-854` | MATCH |
| M3 | Channels shown submenu | Three kinds: `Public Groups`, `Private Groups`, `Combat Messages` | [V 0x100ab758 strings] | Two: "Public Groups", "Combat Messages" (private groups listed with the public ones); the list stays up after a pick (client's own) | `chatwindow.rs:855-857`, `1025-1029` | DIFFER |
| M4 | AutoSubscribeChannels | Toggle (slot 0x100a8477, no export: effect [NR]) | [V 0x100abd91] | Toggles `pane.auto` | `chatwindow.rs:1017` | UNSURE |
| M5 | ChatConfiguration | Toggles the stored value `chat_group_window`; the tick follows it | [V 0x100ab022] | Toggles `config.open`, ticked while open | `chatwindow.rs:968`, `867` | MATCH |
| M6 | RenameWindow | Text-input dialog (`EnterWindowName`, OK/Cancel) | [V 0x10099d7a] | `config.rename`: the chat configuration window asks the new name; a taken name is refused | `chatwindow.rs:980`, `chatconfig.rs:176`, `163` | MATCH |
| M7 | DeleteWindow | The default window (byte +0xd8) cannot be deleted: a warning box `NotPossible2DeleteDefaultWnd`; any other: question `Sure2DeleteWindow` Yes/No | [V 0x100993c4] | Deleted at once; entry greyed when only one window exists | `chatwindow.rs:870`, `999-1002` | DIFFER |
| M8 | NewWindow | Dialog `EnterNameOfNewWindow` (OK/Cancel); an existing name re-asks with `WindowWithNameExists`; then the window is made (FUN_10094701) | [V 0x1009b901, 0x1009b711] | Makes `Window%d` at once with no groups, no question | `chatwindow.rs:990-998` | DIFFER |
| M9 | Edit submenu | See X14 (only without an input bar) | [V 0x1008fa92] | none | - | MISSING |
| M10 | The frame's menu button (title icon) | Opens this same menu (`GetIconMenu`, `SlotIconButton` 0x1015aa2f) with the frame's `Autofade Window` and `Close Window` after the window's entries | [V 0x101545b3, 0x1015aa2f; the two frame entries are from the client's doc at chrome.rs:182, FUN_1015b34f not re-read by me] | The chat window does not ask for the frame menu (`chrome::frame_menu` is not called in chatwindow.rs), so the button gives only Autofade and Close | `chrome.rs:721-748`, `chatwindow.rs` (no `frame_menu` call; other windows use it, e.g. `invwindows.rs:430`) | DIFFER |
| M11 | The friends list's right button on a chat window (menu FUN_100a8a69) | Not read by me | [NR] | `window_menu_tree` | `chatwindow.rs:901-937` | UNSURE |

**Rules to port.**
- M3: add the middle kind: `Private Groups` (kind 0xe in the original's group-kind loop, [V 0x100ab7a3-0x100ab826]); `chatgroups` must tell private groups from public ones (the invitation code knows them: `invites`, `PrivateJoin`). SMALL-MEDIUM.
- M7: Delete: if the pane is the first/default window show the warning box with `NotPossible2DeleteDefaultWnd` and `Warning` title; else a question `Sure2DeleteWindow` (Yes/No); only on Yes remove. Use `chrome::question` (already used in `show`, chatwindow.rs:2307). `choose` (chatwindow.rs:999). SMALL.
- M8: New: ask `EnterNameOfNewWindow` in a text box; empty or existing -> `WindowWithNameExists`; then `FUN_10094701`'s creation (what exists in `choose` today). The rename box in `chatconfig.rs` (163-181) can be reused. SMALL.
- M10: in `ChatWindow::show` call `crate::chrome::frame_menu("Chat")` (and `"Chat:<frame>"`) and open the same `menu_tree` there at the given point, followed by the frame's `Autofade Window` / `close_entry()` (see `invwindows.rs:430` for the pattern). SMALL.

---

### 5. Tell windows

A tell window is `TellWindow_c` (ctor 0x100b243e): a `Window` 219 x 389 holding one `ChatView_c` with flags 1; created by `FUN_100a7ca7` after the chat server has confirmed the name.

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| W1 | Open (name left press, `OpenChat`, friends row) | Request goes to the chat server; own name refused; the window opens when the name is known (`OpenTellWindow` 0x10086323 -> FUN_100a6b75 -> FUN_100a7ca7(1,1)), centred, config loaded | [V 0x10086323, 0x100a6b75, 0x100a7ca7, 0x100b243e (MoveToCenter, LoadWndConfig)] | `ask_tell` -> `Out::TellWindow` -> `open_tell`; centred first place | `chatwindow.rs:1322-1334`, `1295-1308`, `2181` | MATCH |
| W2 | Enter in a tell window's field | `ExpandChatTextArgs`, then the type-2 request with the window's character (+0x78) | [V 0x100b22fa, connected at 0x100b252c] | Plain line is a tell to the character | `chatwindow.rs:2022-2031` | MATCH |
| W3 | Right press on tell window text | Window menu (the tell window's icon menu, via `GetIconMenu`); its entries are the frame's, a slot at +0x154 is connected (0x100b22c7) | [V 0x1008fa92, 0x100b2577-0x100b258c]; contents of the menu [NR] | No menu from the text | `chatwindow.rs:2022-2031` (no `menu_at`) | UNSURE (likely MISSING: Autofade / Close at least) |
| W4 | Name menu and name click inside it | See X6, X7 | | | | DIFFER |
| W5 | Group links, item links in a tell window | Group signal +0x134 not connected: nothing; item links as X9 | [V gdis 0x100b243e] | Group press handled as in a chat window | `chatwindow.rs:2026`, `2211` | DIFFER (low) |
| W6 | Tab dragging | A tell window has one tab (`AppendTab`), no `GroupChatView` window class: not dragged | [V 0x100b25aa; ChatWindow's drag slots belong to `ChatWindow` 0x1009829a only] | No tabs | - | MATCH |

---

### Counts

Rows in the tables (the W4 line is a pointer to X6/X7, counted there; X25 not counted):
MATCH 23, DIFFER 15, MISSING 12, ADDITION 3, ASKED 1, UNSURE 10 (64 rows).

### Not read / not found

- `TextRenderer_c::MouseMove` 0x10163da4 beyond its role: how a drag selection grows, double or triple click word/line selection, auto-scroll speed (`SlotScrollTimer` 0x10160964 by summary only).
- The scroll-wheel step of the chat text (`ScrollView_c` +0x154) and any chat-specific `SetScrollWheelStep`; lines per notch and any Shift/Ctrl effect on it.
- Key chord for copying selected chat text (`TEXT_COPYTOCLIPBOARD` message; the chord lives outside GUI.dll's key text).
- What the tab-select slot 0x1009740f does besides switching tabs; what `Tab` field +0x1d4 is (blocks the begin-drag).
- `PopupMenu_c::Go` semantics for holding the right button (menu chosen on release or on a second click) - not read for either side.
- Effect handlers of the Visual submenu entries and of `AutoSubscribeChannels` (0x100a8477 has no export), the friends-list window menu (FUN_100a8a69), the tell window's icon-menu contents, what 0x100b22c4/0x100b22c7/0x100b23bd do in a tell window.
- Shift+Enter in the chat field: whether the `InputBar` key signal swallows Enter first (I7).
- PageDown amount sign and what picks the "active" chat window in FUN_1009630b.
- Original's text for `Edit` is a literal string in code (not a text id), no localisation read.
- `Chat window` close button (X) behaviour in the original; window docking of chat frames; the `ChatShowOGrpInTitleBar` / `InInputBar` display (not interactions).
- Nothing was run or launched; the client claims in rows T2, X18, X19, X20, I9, I13 about egui's own behaviour come from the client code that uses egui, not from reading egui.

## Info window, missions, mission booth, reference board


Read only. Original = GUI.dll decompile (`exports_gui`), cited by address. Client = `crates/mao-client/src` as on disk (2026-11-25).
Marks: **[V addr]** read there and it says this; **[NF]** looked, not found; **[NR]** not read.
Facts from `docs/INVENTORY_ORIGINAL.md` sections 0 and 1 that I re-checked in the code: qualifier bits (`& 3` = Shift), the five
`ItemListViewBase_c` signals, the "new" scheme only for lists that called FUN_10041248.

### How the lists behave (read again for this report)

- **New scheme** (list flag `+0x319` set by FUN_10041248 and `UseNewMouseFunc` on; here the journal and the knowledge board)
  **[V 10041aa4, 1004141b]**:
  - Left press, no Shift: if nobody listens to the click signal (`+0x2e0`) the pick-up signal goes at once; else a hold timer
    (`MouseButtonHoldDelay`) starts and pick-up goes after it **[V 10041aa4 0x10041b41.., 1004213e]**. Shift held on the press: nothing.
  - Left release: Shift (`GetQualifiers & 3`) -> qualifier-click signal `+0x2e4`; else if list flags `& 0xc0` is 0 -> click `+0x2e0`;
    else toggle the item's selection **[V 1004141b at 100414c6..1004154c]**. Ctrl and Alt are never tested **[NF]**.
  - Right release: menu signal `+0x2e8` at once (no hold needed) **[V 1004141b 100414cd..100414d6]**.
- **Old scheme** (the booth's offer list: `FUN_10041248` is not called in FUN_100d10fc) **[V 10041aa4 first branch, 1004141b 10041520..]**:
  - Right press: with no click listener (`+0x2e0` empty) the menu signal goes **on the press**; else the hold timer.
  - Left release: Shift -> `+0x2e4`; else if list flags `& 0xc0` -> toggle selection (`MultiListViewItem_c::Select(!IsSelected, 1)`);
    else pick-up/click. The booth list is built with flags 0x40 **[V 100d1468..100d149c: `FUN_100423fc(rect, 0x40, 0, 0, size)`]**,
    and 0x40 is the single-selection flag (client note in `gui/multilist.rs` line 17: `ItemSelectionStateChanged` drops the others).

---

### 1. The info window (`InfoView_c`, FUN_100ef289; XML `Views/InfoView.xml`)

The window is one `TextView` named BrowserView (flags `TVF_MULTILINE|TVF_WORD_WRAP|TVF_ACCEPT_MOUSE_INPUT|TVF_ALLOW_TEXT_SELECTION`,
vertical scroll bar AUTO, no horizontal one) plus BackButton and ForwardButton **[V InfoView.xml, 100ef289]**. All mouse work in the text is
`TextRenderer_c` (`MouseDown` 10163bcd, `MouseUp` 10160167, `MouseMove` 10163da4); `TextView_c` has no mouse handler of its own
(vtable slot +0x3c is `View::MouseDown`, per the client's own note WIN-121).

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| I1 | Left click on a link in the page | The link goes out **on the button press**, not the release: `MouseDown` button 1, only if view flag bit 4 (ACCEPT_MOUSE_INPUT); finds the text attribute under the point; if it has a link (attrib +0x20) emits the TextView's link signal with the URL string. InfoView connected that signal to FUN_100ef278 which calls the URL slot FUN_100ee973 with flag 0 (no toggle). | V 10163bcd; V 100ef289 (disasm 100ef56b..100ef571 connects `TextView+0x1c4` to 100ef278); V 100ef278 | `TextView::mouse(Mouse::Down)`: `link_at` hit gives `Signal::Link(url)` on the press; `InfoView::show` turns it into `show_url(&url)` | `gui/controls.rs` `TextView::mouse` ~1043-1071; `gui/infoview.rs` `show` line 177 | MATCH |
| I2 | Shift, Ctrl or Alt held while clicking a link | Same as plain: `MouseDown`'s third parameter is never read; no `GetQualifiers` in 10163bcd/10160167/10163da4 | V 10163bcd (whole function); NF qualifiers | Modifiers not looked at | `gui/controls.rs` ~1043-1071 | MATCH |
| I3 | Right click (anywhere in the page, link or not) | Unless view flag 0x200 (DISABLE_RC_MENU, not set in InfoView.xml): builds a `PopupMenu_c` with `PopulateRightClickMenu`: text.mdb 10000 `GTK_ChatView_EditMenu_SelectAll` (enabled when the renderer has text, +0x15c != 0), `GTK_ChatView_EditMenu_Copy` (enabled when a selection exists, +0x1cc != -1), and `..._Paste` only for an editing view (flag bit 1). Shown at the pointer. A right click on a link does **not** follow the link. | V 10163bcd (button 2 branch), V 1016305c | Right button is not delivered to views at all (host takes `primary_pressed/released` only); no menu, so no Select All / Copy from the info page. Own note: WIN-147 (written about text inputs), WIN-121 "left: the right-click menu". | `gui/host.rs` 245-266; `docs/DEVIATIONS.md` WIN-121, WIN-147 | MISSING |
| I4 | Press on text off a link, drag, release | Cursor placed at the buffer place (`GetBufferLocation`), `BeginSelection`; `MouseMove` while held `SetCursorPosition` + `ExpandSelection`, with an auto-scroll timer (20000 us?) when the pointer leaves the bounds; release stops the timer. Only with ACCEPT_MOUSE_INPUT. | V 10163bcd (button 1 branch), V 10163da4, V 10160167 | Selection by press and drag, shaded 0xc0c0c0, `copy()` gives it. No auto-scroll while dragging outside is visible in the code. | `gui/controls.rs` 1073-1088, `shown_index_at`; WIN-121 | MATCH (auto-scroll on drag out: UNSURE, not read in the client) |
| I5 | Double click on text | No handler: `MouseDown` only tests button 1 and 2, nothing about click counts; two presses | V 10163bcd; NF double-click | Nothing | `gui/controls.rs` | MATCH |
| I6 | Drag an item picture out of the page | Not possible: the only drag in `TextRenderer_c` is the text selection (`MouseMove` grows the selection, no `BeginDrag`) | V 10163da4; NF `BeginDrag` in 10163bcd | Nothing | - | MATCH |
| I7 | Back button | Needs a current page that is not the first: saves the scroll place (`GetBufferLocation` of the view's top) in the current history node; current = previous; `+0x1a0` (current URL) = its URL; `SetText` its text; scrolls to its saved place; enables Forward; disables Back when it is now the first. | V 100edf3e (connected: 100ef58e, `Button+0x148` -> 100edf3e) | Back: saves scroll in pixels, `at -= 1`, `refresh()` sets text, scroll, Back/Forward enabled flags | `gui/infoview.rs` 167-171, 122-147 | MATCH (scroll saved as a pixel offset, not a text position: differs only after a width change) |
| I8 | Forward button | Mirror image of Back (next node; Back enabled; Forward disabled at the last) | V 100ee06e | Same | `gui/infoview.rs` 172-176 | MATCH |
| I9 | Back/Forward enabling at start | Both buttons made disabled; when reopened with a history, text of the last page is put back, Back enabled if more than one page | V 100ef289 (the `DAT_10276630` branch, 100ef2.. end) | `refresh()` on first show; the window keeps its pages in the struct | `gui/infoview.rs` 149-159 | MATCH |
| I10 | Mouse wheel / scroll bar | Wheel goes to the window controller -> `Window::_HandleMouseWheel`: the window's default wheel view (+0xa4) else its +0x84 view, virtual +0x90 = `WheelMoved`; `ScrollView_c::WheelMoved` scrolls by `delta * step (+0x154)` when the content is taller than the frame, else passes it to the parent. Scroll bar AUTO. | V 10157cec, 10154ab9, 10142820 | Wheel scrolls the topmost view under the pointer that uses it | `gui/host.rs` 346-358; `gui/controls.rs` `wheel` 1090 | UNSURE (which view the wheel reaches when two scrollable views overlap is not the same rule; for this window both reach the text; step size `+0x154` for a TextView not read) |
| I11 | A second info request while the window is open | Same window and same history: `InfoViewModule_c::ShowURL` sets the DValue `info_window` true (opens the window) and emits one signal (url, 1); there is one `InfoView_c`. A different URL pushes a history entry (forward entries are cut, FUN_100ef9da). | V 100efa86, 100ee973, 100ee39b | One `InfoView`; `show_page` pushes, truncates after `at` | `gui/infoview.rs` 59-104 | MATCH |
| I12 | The same URL requested again while it is the current page, from a `ShowURL` caller (journal click, offer menu, Shift+click on an item, knowledge board...) | **The window closes**: the slot FUN_100ee973 is called with flag 1 by `ShowURL`; for any URL but `text://`/`charref://` it first compares to the current URL string (`+0x1a0`) and if equal calls vtable +0xfc (`FUN_100393eb`: sets the window's DValue to false, i.e. closes it). Links clicked **inside** the page call with flag 0 and never toggle. For `text://` the compared string is `text://<byte sum>`; for `charref://` the part before the name. | V 100efa86 (emits arg 1), V 100ee973 start (flag test at 0xc, `compare`, `+0xfc`), V 100393eb | `show_url` always opens and reloads; no flag | `gui/infoview.rs` 59-81 | FIXED 2026-11-25 (was DIFFER): asked for again from outside, the page's window closes, `infoview::asked`; a window that was shut keeps no URL (the open point, taken as a newly made view) |
| I13 | Escape | If DValue `esc_infoview` (LoginPrefs true) was true when the window was made, the window's close is hooked to the GUI escape signal (`LAB_1004a1b1` = jmp to vtable +0xfc = close) | V 100ef289 (end), V 1004a1b1, LoginPrefs.xml line 60 | "Info" is in `ESCAPABLE`, option `esc_infoview` | `ui.rs` 417-428, 451 | MATCH |
| I14 | `charref://<a>/<b>/<markup>` URL (a character reference; also what a notification link and the knowledge board use) | A self-contained page: the text after the second slash's `%n` is the page body, the part before it is the history key; flag-1 toggle compares the key | V 100ee973 (the `charref://%u/%u/%n` branch) | Not handled: `show_url` ignores the URL | `gui/infoview.rs` 59-81 (no `charref`); WIN-056, WIN-136 | MISSING |
| I15 | `charid://<type>/<id>` URL (info on a character) | Sets the page to "Please wait / Transferring information" at once, asks the server (`N3Msg_RequestInfoPacket`), remembers the identity (+0x1c8); when the info arrives (FUN_100ee8e8) the page is pushed under the key `charid://%u/%u` | V 100ee973, V 100ee8e8, V 10031a89 | The client builds the page itself when the packet arrives and calls `show_page`; `show_url` knows no `charid://`, no "please wait" page | `net.rs` ~3870-3900; `gui/infoview.rs` | DIFFER (SMALL; only the interim page and the history key) |
| I16 | `itemid://a/b/c/d`, `itemid://a/b`, `shopitemid://`, `itemref://a/b/c`, `skillid://n`, `text://`, `file://` | Pages built by FUN_10038f6b / FUN_10033eb7; `itemid` and `itemref` also fire the tip event `OnLookatItem` with the name; `file://` tries the path as it is, then `<scripts>/text/`, then `<cd>/text/help/`, then reports "Infoview failed to load..." on the global message signal | V 100ee973 | Same URL kinds; `OnLookatItem` fired; `file://` path, then help, then text (WIN-167) | `gui/infoview.rs` 59-81; `gui/iteminfo.rs` 645 | MATCH (file search order differs: ADDITION WIN-167) |
| I17 | `chatcmd://<cmd>` link | Emits GlobalSignals +0x180 with the text after `chatcmd://` (the chat module runs it); no page change; the URL string still becomes `+0x1a0` | V 100ee973 (tail) | Pushes the command; `chat_window.run` | `gui/infoview.rs` 76-77; `ui.rs` 1409-1413 | MATCH (what the original's listener does with the text: NR) |
| I18 | Hover a `chatcmd://` link (tooltip) | Tooltip hook on the text renderer (`+0xa4`, FUN_100ee194): not for `file://...TipOfTheDay.html`; for a `chatcmd://` link whose command does not start with `/inspect`, the tooltip is text.mdb 10000 `ChatCommand` + the command | V 100ee194, 100ef558 | Pointing-hand cursor only for `markup_click` views; no tooltip on links in a `TextView` | `gui/controls.rs` 781, `gui/host.rs` 376-383 | MISSING |
| I19 | Keys in the window | No key handler found for InfoView itself; Ctrl+C etc. would be `TextRenderer_c::KeyDown` (10164224) for a focused view; a non-editing view does not take the keyboard (WIN-121) | NR 10164224 | Not focusable (`focusable` = editable), so no Ctrl+C | `gui/controls.rs` 990 | UNSURE (original Ctrl+C on a selection not read) |

#### What to port for I3, I12, I14, I18 (DIFFER / MISSING)

- **I3 (menu in text views)**: in `gui/host.rs` pass the right press to the view under the pointer (new `Mouse::RightDown`), and in `TextView::mouse` for a non-editing view that has ACCEPT_MOUSE_INPUT and not DISABLE_RC_MENU open a popup (`chrome::menu_at`) with two entries from text.mdb 10000: `GTK_ChatView_EditMenu_SelectAll` (enabled when the text is not empty; selects all shown text), `GTK_ChatView_EditMenu_Copy` (enabled when a selection exists; `ctx.copy_text(copy())`). Paste only for editing views (already left out in WIN-147 for want of a clipboard reader). Per 10163bcd the right press never follows a link. **MEDIUM** (host plumbing for the second button; the same hook would fix chat windows).
- **I12 (toggle)**: give `InfoView::show_url` a second parameter `toggle: bool` (true from `ui.rs` callers that mean `InfoViewModule_c::ShowURL`: the journal, offer menu, knowledge, Shift-click, the item right-click Info; false for `Signal::Link`). In the non-`text://`/`charref://` branches: `if toggle && url == self.current_url { self.open = false; return }` where `current_url` is the string of the last URL handled (original assigns `+0x1a0` for every URL that is not `text://`/`charref://`, `chatcmd://` included). For `text://` compare the `text://<sum>` key; for `charref://` the key. **Open question:** the original's `+0x1a0` starts empty when `InfoView_c` is made; if closing the window destroys the view, reopening with the same URL shows it. Whether the view is destroyed on close (the DockableView_c from FUN_100395fd) was not read, so decide by reading that or by a live check. **SMALL** (one hour) once that is known.
- **I14 (charref)**: in `show_url` add `charref://%u/%u/` + rest: key = the URL up to the second slash after the numbers, page = the rest (markup, as `text://`), push through `show_page(key, rest)`. **SMALL**. Needs character references to exist (WIN-056).
- **I18 (link tooltip)**: when the pointer rests on a `chatcmd://` link whose command does not start `/inspect` (not on a TipOfTheDay page), show a tooltip `ChatCommand` text (text.mdb 10000) followed by the command string. In `TextView::paint`/`host.rs` tooltip lookup. **SMALL**.

---

### 2. Mission journal (`MissionView_c`, FUN_100d3644, window name `mission_window`, "Missions")

One `MissionListView_c` (FUN_100d3df7) in list layout, columns Icon, Name, Tab_Remaning; `FUN_10041248(1)` makes it a "new scheme" list.
The view connects three signals of the list **[V 100d3787..100d37dd disasm]**: `+0x2e4` and `+0x2e0` both to FUN_100d2cfa, `+0x2e8` to FUN_100d3123. `+0x2d8` (pick-up) and `+0x2dc` (drop) are not connected.
There are no buttons and no tabs in the window **[V 100d3644 calls: only the list and a layout node]**. No `esc_*` option is read **[V 100d3644 call list]**.

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| M1 | Left click on a mission row | Release without Shift, list flags 0 (ctor args: all three `EDI`; assumes 0, see Not read) -> click `+0x2e0` -> FUN_100d2cfa: `ShowURL("itemid://%d/%d")` of the row's identity (row `+0x20` variant). Selection is not changed by a click. | V 1004141b, V 100d2cfa, V 100d37b1..100d37c2 | `ItemClicked` -> `shown.info = m.url()` (`itemid://<type>/<instance>`), shown by `show_url` | `missionlog.rs` 449-453, 102-104; `ui.rs` 1347.. | MATCH (except I12) |
| M2 | Shift + left click | Qualifier click `+0x2e4` -> the same handler FUN_100d2cfa | V 1004141b (100414e8), V 100d37ac | Modifiers ignored: same page | `missionlog.rs` 449-453 | MATCH |
| M3 | Ctrl or Alt + left click | As plain (`& 3` is the only test) | V 1004141b; NF other bits | Same | same | MATCH |
| M4 | Left press and hold (0.3 s) | Pick-up signal after the hold: nothing is connected to it | V 10041aa4 (timer +0x340), V 1004213e, V 100d3644 (only 2e0/2e4/2e8 connected) | Press-drag gives `ItemEvent::Dragged`, which the journal ignores | `missionlog.rs` 448-454 | MATCH |
| M5 | Click the same row again / double click | Each click calls ShowURL again with the same URL: the second **closes** the info window (I12) | V 100d2cfa -> 100efa86 -> 100ee973 | Reloads the same page (a double click reports `clicks = 2`, ignored) | `gui/multilist.rs` 493-507; `missionlog.rs` 450 | FIXED 2026-11-25 (was DIFFER): with I12 |
| M6 | Right click (release) on a row | Menu signal `+0x2e8` -> FUN_100d3123: a `PopupMenu_c` at the pointer | V 1004141b (100414cd), V 100d3123 | `secondary_clicked` over a hovered row opens the menu at the pointer | `missionlog.rs` 455-473 | MATCH |
| M7 | Right press alone, hold | Nothing more: the menu comes on release in the new scheme | V 1004141b | Menu on `secondary_clicked` (egui: release) | same | MATCH |
| M8 | Menu entries and their order | 1 `Look@Mission` always; 2 `Upload2Map` only if `N3Msg_GetQuestWorldPos` finds a place; 3 `UpdateTeamMembers` only if `N3Msg_IsTeamMission`; 4 `DeleteMission` always (all text.mdb 10000) | V 100d3123 calls | Same four entries, same conditions | `missionlog.rs` 459-470 | MATCH |
| M9 | Entry Look@Mission | FUN_100d2d63: `ShowURL("itemid://%d/%d")` (so toggles, I12) | V 100d2d63 | `shown.info = url` | `missionlog.rs` 476 | MATCH (except I12) |
| M10 | Entry Upload2Map | FUN_100d2eb2 -> FUN_100d2dcf: `N3Msg_GetQuestWorldPos`, then emit GlobalSignals +0x158 (id, position, playfield) | V 100d2eb2, 100d2dcf | `upload(...)` with the usual map and compass upload | `missionlog.rs` 477 | MATCH |
| M11 | Entry UpdateTeamMembers | FUN_100d2ab2 -> `N3Msg_UpdateNearbyTeamMembers(identity)` | V 100d2ab2 | `Command::UpdateTeamMembers` | `missionlog.rs` 479 | MATCH (WIN-119) |
| M12 | Entry DeleteMission | FUN_100d2edd: `DialogBox_c` (4 texts), result handler FUN_100d2ae0: when result 0, `N3Msg_RemoveQuest(identity)` | V 100d2edd, 100d2ae0 | `chrome::question` "ReallyWant2DeleteMission", then `Command::Delete` | `missionlog.rs` 480-495 | MATCH (which dialog button is result 0: NR) |
| M13 | Selected row | The row of the active mission (+0x1a4) is kept selected by FUN_100d3366 after each list update | V 100d3366, 100d347d | The list's selection is forced to the active mission each frame | `missionlog.rs` 428-431 | MATCH |
| M14 | Column header click | Header sorting (MultiListView) | NR | `SORTABLE` columns, `ColumnSelected` | `missionlog.rs` 391-393; `gui/multilist.rs` 1146 | UNSURE (not read on the original side) |
| M15 | Escape | No close: the window reads no `esc_` option | V 100d3644 (call list); NF | "MissionView" is not in `ESCAPABLE` | `ui.rs` 417-420 | MATCH |
| M16 | Wheel | `ScrollView` wheel (see I10) | V 10142820 | Wheel on the list | `gui/host.rs` 346 | UNSURE |

---

### 3. Mission booth window (`MissionSelectionView_c`, FUN_100d10fc, `missionselection_window`)

Built from: the offer list (an `ItemListViewBase_c`, flags 0x40, a 3 by 2 grid per WIN-029), seven sliders (difficulty and six advanced ones), an
"advanced options" toggle button, buttons `RequestMissions`, `AcceptMission`, `MsgBox_Cancel`. List signals connected **[V 100d1517..100d1568 disasm]**:
`+0x2e4` (qualifier click) -> FUN_100d03c2, the list's selection signal `+0x144` -> FUN_100d0704, `+0x2e8` (menu) -> FUN_100d0f79.
The click `+0x2e0`, pick-up and drop are not connected.

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| B1 | Left click on an offer (no Shift) | Old scheme release: list flags 0x40 -> `MultiListViewItem_c::Select(!IsSelected, 1)`: a click **toggles** the offer's selection; selecting one drops the others (single selection). Selection change signal `+0x144` -> FUN_100d0704. | V 1004141b (10041537..10041566), V 100d1468..100d149c (flags 0x40) | `response.clicked()` sets `selected = Some(slot)`; a second click keeps it selected | `mission.rs` 523-525 | FIXED 2026-11-25 (was DIFFER): a click turns the selection round, `mission::offer_click` |
| B2 | Click on the already selected offer | Deselects it: FUN_100d0704 finds no `MissionListEntry_c` selected: emits GlobalSignals +0x15c (the offer's waypoint is taken away), `Accept` disabled | V 100d0704 | No way to deselect; selecting another one only | `mission.rs` 404-420 (`mark`) | FIXED 2026-11-25 (was DIFFER): the second click leaves none selected (the waypoint goes, Accept is dead) |
| B3 | Shift + left click on an offer | Qualifier click `+0x2e4` -> FUN_100d03c2: `ShowURL("itemid://%d/%d")` of the offer: opens its page (the reward items are listed in it) | V 1004141b (100414e8), V 100d03c2 | Shift not looked at; only the menu entry Look at Mission opens the page | `mission.rs` 523-525 | FIXED 2026-11-25 (was MISSING): the offer's page |
| B4 | Ctrl or Alt + click | As plain (toggle) | V 1004141b | As plain click | same | MATCH (modulo B1) |
| B5 | Double click on an offer | Two toggles: selected, then not selected; no handler of its own | V 1004141b | Selects | same | FIXED 2026-11-25 (was DIFFER): two clicks, selected then not |
| B6 | Right click on an offer | Old scheme, no click listener: the menu signal goes **on the button press** (FUN_1004266a straight from the MouseDown). `FUN_100d0f79` shows a `PopupMenu_c`: `Look@Mission`; and `Upload2Map` only when `N3Msg_GetQuestWorldPos` finds a place. The right click does not select the offer. | V 10041aa4 (first branch, `+0x2e0 == 0`), V 100d0f79 | `secondary_clicked` (release) opens "Look at Mission" and, when `destination.0 != 0`, "Upload to map" at the pointer. It does not select. | `mission.rs` 528-536, 604-625 | MATCH (press versus release only; entries and conditions the same, WIN-134) |
| B7 | Entry Look@Mission | FUN_100d042b: `ShowURL("itemid://%d/%d")` | V 100d042b | `info_url = itemid://type/instance`, page built from the offer | `mission.rs` 614-618, 422-427 | MATCH (except I12) |
| B8 | Entry Upload2Map | FUN_100d08be: `N3Msg_GetQuestWorldPos` then emit +0x158; selection untouched | V 100d08be (WIN-134 notes) | `upload = Some(id)`; `mark()` sends it up again | `mission.rs` 404-408 | MATCH |
| B9 | Selecting an offer | FUN_100d0704: with a `MissionListEntry_c` selected: `GetQuestWorldPos`, if found emit +0x158 (upload to map and compass), then `Accept` enabled; with none: emit +0x15c, `Accept` disabled | V 100d0704 | `mark()`; Accept `can` flag | `mission.rs` 404-420, 632 | MATCH |
| B10 | Request Missions button | FUN_100d09cb: if stat 0x3d < stat 0x36 an error box with the number; else if free inventory slots < 2 the box `MustHave2FreeSlots`; else flag +0x1b0 = 1, list cleared, `Accept` disabled, the ten slider words copied to +0x1dc, `N3Msg_GenerateMissions` | V 100d09cb | Refusals `request_refused` (stat 61 vs 54) and `slots_refused`, then request | `mission.rs` 42-52, 837-841; `net.rs` 10927-10928; `mission.rs` 561-566 | MATCH |
| B11 | Accept button | FUN_100d0c61: needs a selected offer; free slots < 2 -> `MustHave2FreeSlots` box and the window stays; else `N3Msg_SelectMission(offer+0x120, info+0x1dc)` then deletes the window (vtable slot 0) | V 100d0c61 | `Command{what:2}`; `accepted()` closes and empties | `mission.rs` 632-637, 393-398; `net.rs` 10958 | MATCH |
| B12 | Cancel button | FUN_100d04ab: virtual slot 0 (delete) on the window | V 100d04ab (disasm) | `close = true` | `mission.rs` 638-640 | MATCH |
| B13 | Slider drag | `Slider` change -> FUN_100d0497 stores the value at `+0x1b4 + index*4`; ranges difficulty 0..100 (the difficulty is made with `6`), others -100..100 (`new Slider(0x140, -100, 100)`) | V 100d0497; V 100d1670 (args -0x64, 0x64); ranges per WIN-135 text | egui sliders, same ranges | `mission.rs` 77-80, 570-578 | UNSURE (slider's click-in-the-trough step, wheel, keys not read) |
| B14 | Advanced options toggle button | A toggle `Button_c` (`SetToggleButton`, gfx 0x47 off, 0x46 on) shows or hides sliders 1 to 6; its state is kept as `show_advanced_options` | V 100d1966..100d19c2; WIN-135 notes | None; all seven sliders always shown | `mission.rs` 59-63 note; WIN-135, NPC-036 | MISSING |
| B15 | Escape | `esc_missionselection` (LoginPrefs true) hooks close to the GUI escape signal | V 100d2064..100d20c4 | "Missions" in `ESCAPABLE`, option `esc_missionselection` | `ui.rs` 417-420, 449 | MATCH |
| B16 | Closing the window (X, Escape, Cancel) | Destructor FUN_100d04fe writes the sliders into `MissionSelectionViewConfig` and takes the selected offer's waypoint away | V 100d04fe (per mission.rs note) | Sliders kept in `tabs.ini`; `mark()` returns `Remove` when `open` goes false | `mission.rs` 404-420, 448-456 | MATCH |
| B17 | Extra widgets: list of offers as text lines (click selects), "Stop asking", "Looking for" boxes, status line, hover title | none in the original | NF in 100d10fc call list | Present | `mission.rs` 557-625, 580-586 | ASKED (kept on the user's word of 2026-11-25: asked "Keep the five extras you asked for earlier, which the original does not have?", the answer was "Keep them"): the mission booth's helper (NPC-039, WIN-029) |
| B18 | Offer reward item icons (Shift click = info on the item) | No separate item widgets are made in FUN_100d10fc (the offer's items are shown in the info page, B3) | NF in 100d10fc calls | Reward items shown as plain icons+text in the window; not clickable | `mission.rs` 596-601 | MATCH (no click in either) |

#### What to port for B1, B2, B3, B14

- **B1/B2**: make the offer boxes toggle: `if response.clicked() { self.selected = if self.selected == Some(slot) { None } else { Some(slot) } }` in `MissionWindow::show` (`mission.rs` ~523). `mark()` already removes the waypoint and `can` already disables Accept when `selected` is `None`. Do the same for the text-line list (581). **SMALL**.
- **B3**: in the same place, `if response.clicked() && ui.input(|i| i.modifiers.shift)` set `self.info_url = Some(format!("itemid://{}/{}", id.0 as i32, id.1 as i32))` and do **not** change the selection (the qualifier branch comes before the toggle in 1004141b). **SMALL**.
- **B14**: see WIN-135; needs the toggle's picture names. **MEDIUM**.
- Optional: B6 menu on the press instead of the release (`response.is_pointer_button_down_on() && pointer.secondary_pressed()`). **SMALL**.

---

### 4. Knowledge window / reference board (`KnowledgeView_c`, FUN_100cfa6c, `knowledge_window`)

An `ItemListViewBase_c` in grid mode, `FUN_10041248(1)` (new scheme), flags 0. Connected **[V 100cfc0c..100cfc88 disasm]**: pick-up `+0x2d8` -> FUN_100cf1d6, drop `+0x2dc` -> FUN_100cf299, qualifier click `+0x2e4` -> FUN_100cf49f, menu `+0x2e8` -> FUN_100cf89e. The plain click `+0x2e0` is **not** connected. Two further signals (FUN_100cf9d4 on the reference's name, FUN_100cf019 adds a reference, FUN_100cf17f deletes). No `esc_` option **[V 100cfa6c call list]**.

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| K1 | Plain left press on a reference | With no click listener the pick-up signal goes **on the press** (no hold): FUN_100cf1d6 makes an `ItemIconView`, a `DragObject_c` carrying the reference's identity and `BeginDrag`: the reference is on the pointer | V 10041aa4 (the `+0x2e0 == 0` branch), V 100cf1d6 | Press does nothing; the release shows the info page | `knowledge.rs` 185-197 | DIFFER |
| K2 | Plain click (press and release on the same cell) | Pick-up then drop on itself: FUN_100cf299 (button 1, type-3 drag, occupied cell holding the same item) -> `DragObject::Cancel`: nothing happens, no info page | V 100cf299 | Shows the item's info page | `knowledge.rs` 195-197, 213-217 | DIFFER |
| K3 | Shift + left click (release) | Qualifier click `+0x2e4` -> FUN_100cf49f: if the reference `IsCharacter` `ShowURL("charref://%d/%d/%s")` else `ShowURL("itemid://%d/%d")` | V 100cf49f, V 1004141b | Any click: `itemref://low/high/quality` page | `knowledge.rs` 213-217 | DIFFER (needs Shift; `itemid` not `itemref`) |
| K4 | Right click (release) | New scheme right release -> menu FUN_100cf89e at the pointer: `Look@Item`, `DeleteReference` (both text.mdb strings; the decompile shows category 0x2710 = 10000 for both) | V 100cf89e, V 1004141b | Menu at the cell's top right with the same two entries, text category **10001** | `knowledge.rs` 198-201, 221-227 | MATCH (position and text category differ: category UNSURE, original shows 0x2710) |
| K5 | Entry Look@Item | FUN_100cf583: same as K3 | V 100cf583 | `itemref` page | `knowledge.rs` 232-236 | MATCH (modulo I12) |
| K6 | Entry DeleteReference | FUN_100cf464: `RefMan_t::DeleteRef`; the list item goes via the delete signal FUN_100cf17f | V 100cf464, 100cf17f | `remove(cell)` | `knowledge.rs` 228-231 | MATCH |
| K7 | Drop a dragged item (from the inventory) on a cell | FUN_100cf299, button 1, drag type not 3: the place is the cell (first free place if occupied), `Accept(2)`, `N3Msg_GetReferenceData` then `RefMan_t::AddItemRef` | V 100cf299 | `drop(cell, entry)` on release over the cell: cell, or first free if taken | `knowledge.rs` 104-119, 204-210 | MATCH (any mouse button releases in the client; original only button 1: UNSURE what `letting_go` accepts) |
| K8 | Drop a board reference on another cell (move) | Type-3 drag: on an empty cell and not in list mode -> `MoveItem`, `Accept(2)`; on an occupied cell of another item: nothing (not accepted); on itself: cancel | V 100cf299 | `drop()` moves an item already on the board, but the board's entries cannot be picked up (no drag from the board) | `knowledge.rs` 104-119 | MISSING (follows from K1) |
| K9 | Drop on a cell with Shift/Ctrl/Alt | Not tested (only the button) | V 100cf299; NF qualifiers | not tested | - | MATCH |
| K10 | Character references (drop a character, `charref://` page) | Supported through `N3Msg_GetReferenceData` and `Reference_t::IsCharacter` | V 100cf299, 100cf49f | Only items | WIN-056 | MISSING (the client has no character drag source) |
| K11 | Drag a reference out to the chat or elsewhere | The drag object lives until the release; what the chat window or the world do with a dropped reference was not read | NR | Not possible | - | UNSURE |
| K12 | Escape | None | V 100cfa6c | None | - | MATCH |
| K13 | Hover on a reference | FUN_100cf9d4 (a signal on a reference's name; sets a character's name) | NR (what it feeds) | Tooltip with name and QL | `knowledge.rs` 194 | UNSURE |
| K14 | Board grows when full | `GetFirstFreePos` makes new rows | V 100cf299 | Rows grow, 6 columns fixed (WIN-057) | `knowledge.rs` 52-70 | MATCH (columns fixed by WIN-057) |

#### What to port for K1, K2, K3, K8

- **K1/K2/K3**: make the plain press pick the entry up, as the pick-up is immediate when there is no click listener: on the press of the left button without Shift over an occupied cell, set `gear::DRAGGED`-like state (a drag of a board entry, carrying low/high/quality) and let the existing release code do the drop; a release on the same cell cancels. Show the info page only for Shift held at the release (`ui.input(|i| i.modifiers.shift)` on the release, before the drop): `ShowURL("itemid://low/high")` (the original's string, not `itemref`). Without Shift nothing opens. Functions: `KnowledgeWindow::show` (`knowledge.rs` 185-217). **MEDIUM** (the board needs its own drag state beside the inventory's `DRAGGED`, and the drop code must tell the two apart: type 3 versus the rest).
- **K8**: follows from the above; the `drop()` function already moves an entry on the board. **SMALL** after K1.

---

### Counts (rows)

| Window | MATCH | DIFFER | MISSING | ADDITION | ASKED | UNSURE |
|---|---|---|---|---|---|---|
| Info window (I1-I19) | 12 | 2 (I12, I15) | 3 (I3, I14, I18) | 0 | 0 | 2 (I10, I19) |
| Journal (M1-M16) | 13 | 1 (M5) | 0 | 0 | 0 | 2 (M14, M16) |
| Booth (B1-B18) | 11 | 3 (B1, B2, B5) | 2 (B3, B14) | 1 (B17) | 0 | 1 (B13) |
| Knowledge (K1-K14) | 7 | 3 (K1, K2, K3) | 2 (K8, K10) | 0 | 0 | 2 (K11, K13) |
| **Total (67 rows)** | **43** | **9** | **7** | **1** | **0** | **7** |

### Not read / not found

- Whether closing the info window destroys the `InfoView_c` (so `+0x1a0`, the current URL, starts empty on reopening). It decides how the I12 toggle behaves right after a reopen. (The DockableView_c made by FUN_100395fd and the DValue name at +0x14c were not read.)
- The scroll step of a TextView (`ScrollView_c +0x154` for BrowserView) and the vertical scroll bar's own mouse code (`ScrollBar` views): not read; the client's bar is its own.
- `TextRenderer_c::KeyDown` (10164224) for Ctrl+C on a selection in a view that does not edit; the auto-scroll timer in `MouseMove` (20000 us) and what it does to the selection.
- The meaning of GlobalSignals +0x180 (chatcmd), +0x158/+0x15c (upload/remove waypoint) on the receiving side; I took the client's own notes for what they do.
- The MissionListView's list flags: the decompile call `FUN_100423fc(this, EDI, EDI, EDI, ...)` hides the register values; I read them as 0. If `flags & 0xc0` were set, a plain click would toggle selection instead of showing the page. The client's notes (CLIENT_UI.md 3111) say a click shows the page, which is consistent with 0.
- `MissionListEntry_c` (FUN_100d2a38) cell drawing, hover, and whether a booth offer cell is itself clickable beyond the list (vtable lookup by RTTI returned garbage for that class).
- Which `DialogBox_c` button gives result 0 in the DeleteMission and MustHave2FreeSlots boxes; the Delete result path was taken from the client's note.
- Header clicks (sort) in the journal list (`MultiListView_c`), the booth sliders' own mouse code, key handling in sliders.
- What a knowledge-board drag does when released outside the board (chat, world); `FUN_100cf9d4`, `FUN_100cf019`, `FUN_100cf17f` triggers (global signals) were read only for their calls.
- Keyboard bindings that open these windows (`Ctrl+4` etc.): the default key table was not read in this pass.
- Info page contents (item, nano, character pages) and their inner links were not compared; only link dispatch was.
- `ShowShadowKnowledgeInfo` (FUN_1003045b): a dialog, not read.

## Planet map, playfield map, team, friends, target bars


Read-only comparison. Original = GUI.dll decompile (`exports_gui`), cited as **[V addr]** (read there), **[NF]** (looked, not found), **[NR]** (not read). Client = `modern-anarchy-online/crates/mao-client/src`, read from disk on 2026-11-25.

### Facts the rows lean on (all read)

- `View::MouseDown(Point, button, count)` is slot +0x3c. The count comes from `WindowController_c::HandleMouseDown` (0x10157d8f) -> `Window::_HandleMouseDown` (0x10154ea2, passes `[ebp+0xc]`=button, `[ebp+0x10]`=count to slot +0x3c). `NewGuiSpot_t::CheckHotSpot` (0x1001db81) maps hot spot 1 to `MouseDownMessage(1,1)`, 3 to `(1,2)`, 5 to `(2,1)`, 7 to `(2,2)`, 9 to `(3,1)`, b to `(3,2)`. So **count 2 = a double click, reported on the second press**. Whether a plain `(1,1)` comes first as well is not in the decompiled DLLs (the DEVIATIONS note at line 5764 says the same).
- `View::MouseDown` (0x1014ca5d) is a default that only emits the view's own mouse-down signal at +0x74. A class that does not override slot +0x3c does nothing with a click unless someone connected that signal.
- Vtable comparison (script over `vtable.py`, slots 0..0xf0) against `PlanetMapView_c`: `TeamBar_c` differs from a plain View only at slot 0 and +0x3c (MouseDown 0x1007a23a). `ListViewBase_c` overrides only 0, +0x3c (0x101313ad), +0x6c, +0x70, +0x7c, +0x84 (no MouseUp, no MouseMove, no KeyDown). `PFMapRenderer_c` overrides +0x3c/+0x40/+0x44 (0x100ea0ea/0x100ea145/0x100ea177) and +0x98 (tooltip 0x100478c7). `BitmapTileView_c` (the planet map's picture) overrides +0x3c/+0x40/+0x44 (0x1005077c/0x1005069e/0x100505d2). `PlanetMapView_c`, `PlayfieldMapView_c`, `TeamView_c`, `FriendListView_c`, `MapControlView_c`, `TargetHealthBar_c`, `TargetHeader_c`, `CCTargetControl_c` keep the View defaults for mouse slots.
- A window content view's context menu is the virtual at +0xf8. The only caller of `[view+0xf8](menu)` in the whole DLL is `FUN_10048de9` (the rollup page's menu, created by `RollupPage_c` FUN_10049a3c, opened by `FUN_10048c5c`). The window frame's own icon menu (FUN_1015b34f) has only the autofade toggle and Close. **[NF]: any path that opens a view's +0xf8 menu for a window that is not docked in a rollup.** `PlanetMapView_c`'s +0xf8 is FUN_1004ce6b (pointer at 0x101b169c), `PlayfieldMapView_c`'s is FUN_100ec16b, `TeamView_c`'s and `FriendListView_c`'s are the empty default 0x100393e8.
- Qualifier reads: none of 0x1005077c, 0x100505d2, 0x100ea0ea, 0x1007a23a, 0x101313ad, 0x100aa208 calls `GetQualifiers`. Shift/Ctrl/Alt do nothing in any of these handlers.
- The pan constant `_DAT_101b1370` read from the DLL image = **1.5** (double).

---

### 1. Planet map window (`PlanetMapView_c`, ctor FUN_1004d92d; picture `BitmapTileView_c`, FUN_1004c1c5)

Client: `planetwindow.rs` `PlanetWindow::show` (lines 82-230), `zoom` (65-78); `planetmaps.rs` (maps and tiles only).

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| P1 | Left press, move, release on the map | Press (count 1) starts a grab: `AFCM::Send(0x12,0x3d)`, mouse capture (vtable +0xac), the mouse place saved at +0x16c, flag +0x168=1. Each move: scroll offset += (mouse now - saved) x **1.5**, floored, and the cursor is put back to the saved place (`SetMousePosition`), so the pointer stays still. Release: `AFCM::Send(0x12,6)`, flag cleared. | [V 1005077c else-branch, 100505d2, 1005069e] | Drag moves the map by `drag_delta` x 1.0 (`centre -= drag_delta`), switches follow off. Pointer not held still. | planetwindow.rs 136-139 | DIFFER (the pointer is not held still: left). The speed FIXED 2026-11-25: one and a half times the mouse's way |
| P2 | Right or middle press, move, release on the map | Same grab as P1 (the handler tests only the count, not the button). | [V 1005077c: only `param_3==2` is tested first, else grab] | Right/middle drags do nothing (`Sense::click_and_drag` gives drags to the primary button only). | planetwindow.rs 120, 136 | MATCH (checked 2026-11-25: the interface library's drag is any button's, so the right and the middle button grab the map too) |
| P3 | Double left click on the map (count 2, button 1) | Emits the tile view's signal +0x130 with the click's place as a fraction of the picture ((point - left top) / size), then signal +0x128. +0x130 -> FUN_1004bae5 stores the fraction at +0x1ec/+0x1f0; +0x128 -> FUN_1004bb0a: if level < count-1, level += 1 and FUN_1004c2e2 (builds a new tile view for the level, then scrolls so the clicked fraction is in view, scroll clamped to <= 0). **A zoom in one level, aimed at the click.** | [V 1005077c (count==2, button==1), 1004bae5, 1004bb0a, 1004c2e2 (connects at 1004c4b2..1004c503, edi=0x128)] | Nothing. A double click is only two press-and-release of a drag area. | planetwindow.rs (no `double_clicked`) | FIXED 2026-11-25 (was MISSING): a level closer, the point clicked brought to the middle (`planetwindow::aimed`) |
| P4 | Double right click on the map (count 2, button 2) | Emits +0x130 with the place, then +0x12c -> FUN_1004bb2b: if level > 0, level -= 1, same re-centre on the click. **A zoom out one level, aimed at the click.** | [V 1005077c (button==2 branch), 1004bb2b] | Nothing. | planetwindow.rs | FIXED 2026-11-25 (was MISSING): a level further, aimed the same way |
| P5 | Mouse wheel over the map | The tile view keeps `View::WheelMoved`, which hands the wheel to the parent; the parent is the `ScrollView_c` (ctor line 1004df70) and `ScrollView_c::WheelMoved` scrolls the client **vertically** by wheel x step (+0x154), only when the content is taller than the frame. **No zoom.** | [V 1014a8ef, 10142820; BitmapTileView_c +0x90 not overridden] | A notch of the wheel is one level, zooming in or out around the pointer. No vertical scroll. | planetwindow.rs 140-147 | FIXED 2026-11-25 (was DIFFER): the wheel scrolls up and down, 10 a notch, while the level is taller than the view; it does not zoom (`planetwindow::wheeled`). What a notch is in the scroll view's units was not read |
| P6 | "Zoom in" / "Zoom out" buttons (`PlanetMap_ZoomIn`, `PlanetMap_ZoomOut`) | ZoomIn: level+1 (FUN_1004bb0a), ZoomOut: level-1 (FUN_1004bb2b); FUN_1004c2e2 holds the level to 0..count-1 and **disables** ZoomIn at the last level and ZoomOut at 0. A button zoom leaves +0x1ec/+0x1f0 at 0, so the new tile view is not re-aimed (where it then sits is [NR]). Tips `Click2ZoomEtc`, `Click2ZoomOutEtc`. | [V 1004e48c.., 1004bb0a, 1004bb2b, 1004c2e2] | Same +1/-1 by `zoom(by, None)` around the middle, clamped; both buttons always enabled. | planetwindow.rs 65-78, 188-203 | FIXED 2026-11-25 (was DIFFER): each button is dead at its end of the levels, and a button's zoom brings the character to the middle (FUN_1004bd97), the level's corner when the character is not on the map |
| P7 | "Character" button (`PlanetMap_CentreOnChar`) | FUN_1004bd97: if the level's marker is valid and its flag +0x3c is 0, scrolls the character's pixel into the middle (FUN_1004edb1 -> FUN_1004bb43) and sets the tile view's byte +0x174 = 1 (use of +0x174 [NR]). Enabled only while that flag is 0 (FUN_1004c2e2 `View::Enable(+0x1e4, flag==0)`). | [V 1004bd97, 1004c2e2] | Sets `follow` so the map keeps the character in the middle every frame; enabled when the character has a place. | planetwindow.rs 197, 204-206 | UNSURE (one-shot centre against a continuous follow: depends on what +0x174 does) |
| P8 | "Mission" button (`PlanetMap_CentreOnQuest`) | FUN_1004bdee: if +0x1cc != -1 (a mission waypoint is shown) scrolls its pixel into the middle. | [V 1004bdee] | `pending_centre = waypoint`, follow off, button disabled without a waypoint. | planetwindow.rs 197, 207-210 | MATCH |
| P9 | Right click / the view's menu (entries: Center on player, Center on mission, Select Map, Show buttons) | Not a click on the map: FUN_1004ce6b is the +0xf8 virtual; `Center on player` -> FUN_1004bd97, `Center on mission` -> FUN_1004bdee, `Select Map` -> FUN_1004cb15, `Show buttons` (boolean, checked from +0x1e0) -> FUN_1004bd5f (shows or hides the two button rows +0x1d8/+0x1dc). Reached only through a rollup page's menu (see Facts). A right press on the map itself is a grab (P2). | [V 1004ce6b, 1004ce66 jmp 1004cb15, 1004bd5f, 10048de9, vtable ptr at 0x101b169c] | No menu at all. | planetwindow.rs | MISSING |
| P10 | "Show buttons" (hide the two button rows) | Boolean menu item above; the setting is kept in `PlanetMapViewConfig` (`ShowButtons`, read at 1004e99a). | [V 1004bd5f, 1004e99a] | The two rows are always shown. | planetwindow.rs 183-210 | MISSING |
| P11 | "Select Map" (choice between maps of one planet) | FUN_1004cb15 builds an HTML info page of the map index files (`PlanetMapIndexFile`, `ShadowlandMapIndexFile` list in `textures/PlanetMap/`) whose links run `/close infoview`, `/close planetmap`, `/open planetmap` after setting the choice. | [V 1004cb15 (strings and chatcmd lines per index)] | A combo box under the buttons, shown when the planet has more than one map; the window re-picks. | planetwindow.rs 111, 211-226 | DIFFER (same effect, different control; the original's opens an info page) |
| P12 | Pointer's world place on the map | None found: the tile view has no tooltip override (slot +0x98 default). | [NF] | Draws "x x z" at the lower left of the map while hovered. | planetwindow.rs 177-182 | FIXED 2026-11-25 (the ADDITION is gone) |
| P13 | Shift / Ctrl / Alt with any of the above | No qualifier is read. | [V 1005077c, 100505d2] | None. | planetwindow.rs | MATCH |
| P14 | Keys: Escape closes (`esc_planetmap`), the window's key binding | `esc_planetmap` read in the ctor (1004eb9f `GetDValue`); LoginPrefs.xml line 75 default true. The key binding for opening the window comes from the per-user key list, not from CharPrefs.xml (no map keys there). | [V 1004eb9f; LoginPrefs.xml; CharPrefs.xml has no key lines] | `WINDOW_PLANETMAP` bound in keybindings.rs line 49; `esc_planetmap` listed in GUI_DLL_PARITY.md line 60. Code of the Escape path not read by me. | keybindings.rs 49 | UNSURE (client side read from docs only) |

#### To port (P1-P12)

- **P3/P4 (double click zoom).** In the planet map's drag area: on the second press (egui `double_clicked()` for button 1, and a secondary double click for button 2), compute `f = (pos - rect.min_of_level_picture) / level_size` (fraction of the level's picture, not of the view). Left: `level = min(level+1, levels-1)`; right: `level = max(level-1, 0)`. After the level changes, scroll so that `f` of the new level's picture sits where the old fraction sat in the original, i.e. FUN_1004c2e2's formula: new scroll = -(f x newSize - viewSize/2)... precisely `scroll = -( (f * level_size) - (view_size / something) )` clamped to <= 0 (the code at 1004c508-1004c66f computes `Rect::Size(bounds)/_DAT_101ae184` as the view half-size; the constant's value is [NR], almost certainly 2). Clear +0x1ec/+0x1f0 afterwards. Client function: `PlanetWindow::show` (add after the drag handling) and `zoom` with an `at` that is the click. MEDIUM-small (an hour or two).
- **P1/P2 (drag).** Multiply the drag delta by 1.5, accept any mouse button (egui: `response.dragged_by(...)` for primary, secondary and middle, or read `i.pointer.button_down(...)`), and in the egui frame hide the cursor and re-set it. SMALL for speed and buttons; the cursor lock is MEDIUM (needs a platform call). The pan direction convention of the original (offset += delta x 1.5 on a scroll view) must be checked against the client's `centre -= drag_delta`: [NR] whether it grabs the map or pushes it.
- **P5 (wheel).** Remove the wheel zoom and give the wheel the ScrollView's vertical scroll (step +0x154, default in `View`/`ScrollView_c`), only when the level picture is taller than the view. SMALL, but it removes a feature players may like; say whether to keep it as a setting.
- **P6.** Disable the ZoomIn button at the last level and ZoomOut at level 0 (`crate::chrome::button(.., enabled, ..)` second argument). SMALL.
- **P9/P10/P11.** Add a menu with the four entries on the window's menu button (or on right click) only if the user wants it; the original reaches it only from a rollup page. If ported: `Center on player` = `follow` once, `Center on mission` = centre on waypoint, `Select Map` = open the maps list (info page or the combo), `Show buttons` = hide `buttons_height` rows, kept as `ShowButtons` in `PlanetMapViewConfig`. SMALL-MEDIUM.

---

### 2. Playfield map ("PF Map", `PlayfieldMapView_c` FUN_100ec21c; renderer `PFMapRenderer_c`; control row `MapControlView_c` FUN_100ecdf7)

Client: `automap.rs` `pf_window` (748-826), `control_bar` (676-735). The round corner map and the overlay (`AutoMap::show`, 831-939) are ASKED, see the last two rows.

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| F1 | Left press, move, release on the map | Press: capture (+0xac), `AFCM::Send(0x12,0x3d)`, mouse place saved at +0x470, flag +0x46c=1, +0x478=0. Move: scroll += (mouse - saved) x 1.5, floored, the cursor put back (`SetMousePosition`). Release: `AFCM::Send(0x12,6)`, +0x478=1 (the "come back to the character" flag), +0x46c=0. | [V 100ea0ea, 100ea177, 100ea145] | Drag moves the view by `drag_delta` x 1.5; on release `pf_pan` is zeroed (back to the character). Cursor not held still. | automap.rs 761-767 | MATCH (cursor lock and `AFCM 0x12` absent, same as P1) |
| F2 | Right or middle press and drag | Same grab: the handler tests neither button nor count. | [V 100ea0ea] | Only the primary drags; a right release opens the menu (F5). | automap.rs 759-767, 814 | MATCH (checked 2026-11-25: any button drags; the right button's release no longer opens a menu) |
| F3 | Double click on the map | Nothing special (no count test). | [V 100ea0ea] | Nothing. | automap.rs | MATCH |
| F4 | Mouse wheel over the map | `PFMapRenderer_c` does not override the wheel; it goes to the parent `ScrollView_c` (renderer is its client, ctor 100ec4d9..100ec533). Whether the renderer's preferred size makes the view scrollable [NR]. | [V vtable +0x90 default 1014a8ef; 100ec462..100ec533] | Nothing on the PF window (the wheel zooms only the corner map and overlay). | automap.rs 748-826 | UNSURE |
| F5 | Right click on the map / the view menu | Not a click on the map (F2 is a grab). The menu is FUN_100ec16b, the +0xf8 virtual: one boolean entry "Show buttons" (checked from +0xb8-style flag), reached only through a rollup page. | [V 100ec16b (calls AppendMenuEntry, MakeBoolean, Check; string "Show buttons"), vtable 0x101c0964 +0xf8] | A right release on the map opens a one-entry menu labelled "Buttons" (checked) that shows or hides the control row. | automap.rs 814-817, 822-824 | FIXED 2026-11-25 (was DIFFER): "Show buttons" from the frame's menu button |
| F6 | "Info" button (land control view) | `Button_c` with text 10000 "Info", tip `ShowLConMap`, toggle; its handler FUN_100ec034 -> FUN_100ebac9 (starts an EventTimer 0x7a120 us, FUN_100eb04b draws the district text, FUN_100eb5f1 map availability). Enabled by `N3Msg_GetSkill(0x185)` (= stat 389). Initial state from `RenderLandControlData`. | [V 100ecf74/100ec034, 100ebac9, 100ed2fe, 100ed189] | Toggle `Info`, tip `ShowLConMap`, saved as `RenderLandControlData`, disabled without stat 389 bit 0. | automap.rs 676-706 | MATCH |
| F7 | "Build" and "War" buttons | Both emit the global signal at GlobalSignals +0x170: Build with argument 1 (FUN_100ec95f), War with 0 (FUN_100ec9f3); tips `OpenLCbuild`, `OpenLCwar`; disabled without Notum Wars. | [V 100ec95f, 100ec9f3, 100ed063, 100ed167] | Both open the tower menu (`tower_menu = Some(true/false)`), same tips, same disabling. | automap.rs 691-731 | MATCH |
| F8 | X and Z text fields, "Add Waypoint" and "Remove Waypoint" buttons | Add (FUN_100ecc19): reads both fields with `atof`; if either is <= 0 the chat line "Coordinates not in range." (GlobalSignals +0x17c, type 0x52); else the waypoint signal with the playfield resource (`N3Msg_GetPlayfieldResource`) and both numbers. Remove (FUN_100ec8db): signal at GlobalSignals +0x15c. | [V 100ecc19, 100ec8db, 100ed607..100eda67 (two `TextInputView_c`, buttons 'Addwaypoint', 'Removewaypoint')] | None; the window has no fields and no waypoint buttons. | automap.rs 676-735 | MISSING |
| F9 | Pointer tip | Tool-tip slot (+0x98) 0x100478c7: text 10000 `PointerPosition` plus "%.1fx%.1f" of the pointer's world place. | [V 100478c7, vtable PFMapRenderer_c +0x98] | Tip "Pointer at" and `{x:.1}x{z:.1}` under the pointer. | automap.rs 809-813 | MATCH |
| F10 | Shift / Ctrl / Alt | No qualifier read in 100ea0ea / 100ea177. | [V] | None. | automap.rs | MATCH |
| F11 | The client's round corner map: drag to move, wheel zoom, Ctrl+wheel and the corner grip to resize, overlay (toggle key, drag, wheel, Space) | Not in the original (the original's PF Map is the square window). | [NF] | `AutoMap::show` | automap.rs 831-939 | ASKED (kept on the user's word of 2026-11-25: asked "Keep the five extras you asked for earlier, which the original does not have?", the answer was "Keep them"): the round corner map and its overlay, the `own_map` setting, off by default (WIN-151) |
| F12 | Window key / Escape for the PF map | Escape: no `esc_*` option for `map_window` found in LoginPrefs.xml; key binding is per user. | [NF esc for map_window; NR key list] | `WINDOW_MAP` bound (keybindings.rs 50). | keybindings.rs 50 | UNSURE |

`CAM-001` in DEVIATIONS.md is the camera (WoW-style controls), not a map; nothing here depends on it.

#### To port (F2, F4, F5, F8)

- **F2.** Let primary, secondary and middle buttons drag in `pf_window`, keep the 1.5 factor, keep "back on release". Right click must then not open the menu on release of a drag (egui: use `secondary_clicked` only when no drag). SMALL.
- **F5.** The original has no right-click on the map for this menu; it is a rollup-page menu. If it is kept on right click, rename the entry to text "Show buttons" (the exact text of 100ec16b). SMALL.
- **F8.** In `control_bar` add the two `TextInputView_c` fields (X, Z) and the buttons `Add Waypoint` and `Remove Waypoint`; Add: parse both as floats; if either <= 0 write the chat line "Coordinates not in range." (system line type 0x52); else set the waypoint to (current playfield, x, z) as the mission/waypoint signal does (what `+0x15c`/the add signal feed in the client's waypoint code is [NR]); Remove: clear it. MEDIUM (needs the waypoint plumbing and a text-input view in the control row).
- **F4.** Needs a decision after reading `FUN_100eb7d5` (the renderer's size): [NR].

---

### 3. Team window (`TeamView_c` FUN_10078c14, member views `TeamBar_c` FUN_1007981b, module `TeamViewModule_c` FUN_1007bfe3)

Client: `team.rs` `TeamWindow::show` (374-473), `member_menu` (302-310), commands (225-237).

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| T1 | Left press on a member's bar (any count) | If the bar has an identity (+0x128/+0x12c not both 0, FUN_10077b1c): `AFCM::Send(0x1e, 0x127, identity)`, the select-target message. On the press. | [V 1007a23a (button==1), 10077b1c] | A left click (on release) sends `Command::Select(id)`. | team.rs 438-444 | MATCH (effect; the client acts on release, the original on press) |
| T2 | Right press on a member's bar | Builds a `PopupMenu_c(2)` and opens it at the pointer (`PopupMenu_c::Go`, 0x1013c56b). Entries: "Info on <name>" always; then, only when the bar's byte +0x14c (= is the leader's) is 0 **and** `TeamViewModule_c::IsTeamLeader()`: "Make <name> the leader" and "Put out <name>". | [V 1007a23a (button==2)] | Right click (on release) opens the menu: Info always, then the two leader entries when the member is not the own character and the client's character leads. | team.rs 445-461, 302-310 | MATCH (own-leader test replaces the original's "bar is not the leader's" test: the two agree once 0x17 has named the leader; before that, the original with an unnamed leader would show the two entries on every bar when the character leads, the client on every bar but its own) |
| T3 | Menu "Info on" | `FUN_1007a04e`: `String::FormatString("charid://%u/%u", +0x128, +0x12c)` -> `InfoViewModule_c::ShowURL`. | [V 1007a04e] | `Command::Info(id)` (the info page of the member). | team.rs 456 | MATCH (client's handler for `Info` not read) |
| T4 | Menu "Make X the leader" | `FUN_1007a0ad`: `N3Msg_TransferTeamLeadership(identity)`. | [V 1007a0ad, import 0x101a77b8] | `Command::Lead(id)` (`N3Msg_TransferTeamLeadership` per the file's doc line 289). | team.rs 457 | MATCH |
| T5 | Menu "Put out X" | `FUN_1007a0c3`: `N3Msg_KickTeamMember(identity)`. | [V 1007a0c3, import 0x101a77bc] | `Command::Kick(id)`. | team.rs 458 | MATCH |
| T6 | Double click on a member | Second press (count 2) takes the same path as T1: one more select message. | [V 1007a23a tests button only] | Nothing extra (the select is already sent). | team.rs | MATCH |
| T7 | Shift / Ctrl / Alt / middle button on a member | Not read; middle button (3) is not handled (only 1 and 2). | [V 1007a23a: tests `==1`, `==2`] | None. | team.rs | MATCH |
| T8 | Dropping a held item on a member (use on, trade) | No drop handler: `TeamBar_c`'s vtable equals a plain View's except slot 0 and +0x3c. | [V vtable 0x101b66c4 compare] | None. | team.rs | MATCH |
| T9 | Dragging a member | None (no MouseUp/Move override). | [V same] | None. | team.rs | MATCH |
| T10 | NCU icons of members | None: a `TeamBar_c` holds two text views (leader mark, name) and two `PowerbarView_c` (health 0x1b/0x1a, nano 0x1d/0x1c). | [V 1007981b (calls list)] | None. | team.rs 318-350 | MATCH |
| T11 | "Recruit" button | FUN_100782b2 (`+0x148` click): needs a target (`InputConfig_t::isTargetSet`, else line `MustSelectChar2Join`), not oneself (`CantInviteSelf`), then `N3Msg_TeamJoinRequest(target)` and the line `JoinTeamRequestSentTo`. | [V 100782b2, connect at 1007871f] | `Command::Invite`: the three cases with those texts. | team.rs 468; net.rs 10184-10193 | MATCH |
| T12 | "Leave" button | Connected to FUN_10077e0a (`+0x148`), not exported as a function of its own in `exports_gui`. | [NR] | `Command::Leave`. | team.rs 467 | UNSURE (original handler body not read) |
| T13 | Keys `TARGET_SELECTTEAMMEMBER1`..`5` | `TeamViewModule_c` registers five key providers (strings `TARGET_SELECTTEAMMEMBER1..5`); each calls `SlotTargetTeamMember` (0x1007a6be) -> `TargetMember(n)` (0x1007a67a): when the team state (+0xd8) is 3 it walks the member list n places and sends `AFCM(0x1e,0x127, entry+8)`. The Variants given at registration (0x1007c10c..) read 0, 2, 3, 4, 5 as pushed; the exact numbering is unsure. | [V 1007a6be, 1007a67a, 1007bfe3] | The five names appear only in the key-binding list (`gui/bindview.rs` 131-135); `keybindings.rs` has no action for them, so pressing one does nothing. | bindview.rs 131-135; keybindings.rs (absent) | MISSING |
| T14 | A member's name colour | FUN_10077b3d: leader 0x11eeaa; any other: white (0xffffff) when `N3Msg_isDynelInTree` (member present in the playfield), else grey 0x888888. | [V 10077b3d] | Leader 0x11eeaa, all others white. | team.rs 420-421 | DIFFER (a member who is not in the playfield is grey in the original) |

#### To port (T13, T14)

- **T13.** Add five `Action`s `TargetTeamMember1..5` in `keybindings.rs` (names `TARGET_SELECTTEAMMEMBER1..5`) and a handler that, when in a team, sends `Command::Select(team.members[n].id)` for the place the original's `TargetMember` reaches (list index from 0; which index a key maps to is the `Variant` pushed at 0x1007c10c.. for each key: **read those pushes before choosing**, the register print shows 0,2,3,4,5 and may be misleading). Client function: `team::send`, `keybindings.rs`, the key dispatch in `ui.rs`. SMALL-MEDIUM.
- **T14.** In `TeamWindow::show` pick the colour `0xffffff` when `team::present(m.id)` else `0x888888` (the client already has `team::present`, line 249). SMALL.

---

### 4. Friends window (`FriendListView_c` FUN_100aa315, `ListViewBase_c`, nodes `FriendListNode_c`)

Client: `friends.rs` `FriendsWindow::show` (336-470), `press` (307-317), `entries` (100-111); `gui/treelist.rs` `mouse` (182-209).

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| R1 | Left press on a friend / recent row, count 1 | `ListViewBase_c::MouseDown` finds the row, selects it if the list selects, and emits signal +0x14c(item, view, point, button, count) -> FUN_100aa208: for a row whose parent is not the chat-windows folder: reads the node by the row's id; button 1: toggles the character's tell window (`FUN_100a7ca7(1 - FUN_100a74bc(1))`). | [V 101313ad, 100aa208, connected at 100aa97c] | Left press (count 1): `press()`: the invitation's question when one waits, else the tell window closes or opens. | friends.rs 307-317, 433-446 | MATCH (the invitation branch of FUN_100a7ca7 not read by me; the client's own note says it is there) |
| R2 | Left double click on a friend row | The second press arrives as `MouseDown(1,2)` and runs R1 again: the tell window opened by the first press is closed by the second (toggle again). Whether a `(1,1)` press precedes the `(1,2)` is [NR]. | [V 100aa208 has no count test; 1001db81] | `clicks == 1` only; a second click does nothing. | friends.rs 436 (`clicks == 1`) | UNSURE (no count test in the original; the net result depends on whether both presses arrive) |
| R3 | Left press on a chat-window row (child of the "Chat Windows" folder) | `FUN_1009b4ff(window +0xd0 == 0)`: shows or hides **that** chat window. | [V 100aa208, branch `+0xc == +0x1b0`] | `Act::ChatWindow(!chat_open)`: shows or hides all chat windows together (stated in the file's head, "this client shows its chat windows together"). | friends.rs 434-435 | DIFFER (documented; one window against all) |
| R4 | Right press on a friend / recent row | Same emission with button 2 -> FUN_100aa208 -> FUN_100a99cb opens the character's menu at the pointer. On the press. | [V 100aa208, 100a99cb] | `secondary_clicked()` (release) opens the menu at the pointer. | friends.rs 376-411 | MATCH (release against press) |
| R5 | Menu entry "Add %s as a friend" (BefriendX) | Present only when the node's kind (+0x2c) is 2 (recent). -> FUN_100a8530: node by user_id, then FUN_100a6fff (buddy add). | [V 100a99cb (cmp [+0x2c],2), 100a8530] | Entry only for `recent`; `Out::Befriend`. | friends.rs 100-103, 433 | MATCH (FUN_100a6fff not read) |
| R6 | Menu entry "Invite %s" (InviteX) | Always present; **disabled** (`Enable(0)`) when the kind is 3 (a tell window remembered by the window's config, no friend). -> FUN_100a856f: user_id -> FUN_1016d595 (private-group invite). | [V 100a99cb (cmp [+0x2c],3; Enable(0)), 100a856f] | Always enabled; there is no kind 3 in the client's `Friend`. `Out::PrivateInvite`. | friends.rs 104, 394-409 | DIFFER (small: the disabled state of kind 3) |
| R7 | Menu entry "Remove %s" (DeleteX) | -> FUN_100a93fd: question `ReallyWantToRemoveXfromFriends` with `MsgBox_Yes`/`MsgBox_No`. What Yes sends is [NR]. | [V 100a93fd] | Question with that text; Yes -> `Out::Unfriend`. | friends.rs 413-440 | MATCH (Yes action not read on the original side) |
| R8 | Menu entry "Ignore %s" / "Stop ignoring %s" | Text by `IgnoreSystem_t::IsCharacterIgnored`; -> FUN_100a961f: when already ignored it switches at once (FUN_100a85b0), otherwise asks `ReallyWant2IgnoreX`. | [V 100a99cb, 100a961f] | Ignored: `Act::Ignore` at once; else the question first. | friends.rs 433-436, 420-437 | MATCH |
| R9 | Menu entry "Mail %s" | Present only when a `MailWindow` exists (`FindWindowName("MailWindow")`). -> FUN_100a989a: emits a global signal with the user_name. | [V 100a99cb (0x100a9e9e..), 100a989a] | Present when `around.mail`; `Act::Mail`. | friends.rs 105-107, 435 | MATCH |
| R10 | Right press on a chat-window row | FUN_100aa208 button 2 -> FUN_100a8a69(window): that window's menu. | [V 100aa208] | `Act::WindowMenu(place, at)`. | friends.rs 380-381 | MATCH (content of that menu is another window's matter) |
| R11 | Press on a folder row ("Chat Windows", "Online Friends", "Offline Friends", "Recent Messages") with **any button** | `MouseDown` opens or closes the folder (`OpenFolder(!open)`) for every button and count, then emits the signal (FUN_100aa208 ignores folder rows: their Variant is 0). | [V 101313ad (`pLVar7[0x40]` folder then OpenFolder), 100aa208] | Left press toggles (treelist.rs 189-191) and the open state is kept in `FriendsWindowConfig`; a right press on a folder does nothing. | treelist.rs 184-201; friends.rs 372-390, 425-430 | DIFFER (right/middle press on a folder) |
| R12 | Dragging a name between folders, or reordering | None: `ListViewBase_c` has no MouseUp/Move override. | [V vtable compare] | None. | friends.rs | MATCH |
| R13 | An "add friend" text field | None in the window: the ctor creates the list, four folder items and the `ScrollView_c` only. Adding is `/befriend`. | [V 100aa315 calls] | None (the older text in `docs/CLIENT_UI.md` line 1727, "a Tell button, x and a field to add", does not describe the code today). | friends.rs 1-30 | MATCH |
| R14 | Keys in the window (Delete, Enter, arrows) | None: no KeyDown override in `ListViewBase_c` or `FriendListView_c`. | [V vtable compare] | None. | friends.rs | MATCH |
| R15 | Shift / Ctrl / Alt | Not read by `ListViewBase_c::MouseDown`, FUN_100aa208. | [V] | None. | friends.rs | MATCH |
| R16 | Mouse wheel | `ScrollView_c::WheelMoved` of the list's scroll view (list is the scroll view's client, ctor lines 100aa993..100aaa3c). | [V 10142820 by the same default as P5] | The list sits in a `gui::scroll::ScrollView`; its wheel code not read. | friends.rs 318-333 | UNSURE |
| R17 | The key that opens the window | Per-user key list, not CharPrefs.xml. Menu entries `friends_window` in LeftMenu.xml. | [V CharPrefs.xml line 13; LeftMenu.xml] | `WINDOW_FRIENDS` bound. | keybindings.rs 54 | UNSURE |

#### To port (R2, R3, R6, R11)

- **R11.** In `friends.rs` the right-click block (372-390): when `id` names a folder (`FOLDERS.contains`), toggle it as the left press does and save the mask (`mask_of`). SMALL.
- **R6.** Add a `kind` to `Friend` (3 = remembered tell window of a non-friend) and pass `enabled = false` for `Entry::Invite` when kind 3; SMALL once the kind exists (a friend row built from the saved tell windows: where the client keeps these [NR]).
- **R3.** `Act::ChatWindow(!chat_open)` -> pass the clicked window's place (the row's `CHAT - id`) and show/hide only that one; MEDIUM because the client's chat windows are one panel.
- **R2.** Decide after a capture whether the original sends one or two presses on a double click; if two, the second press of `(1,2)` must toggle again. Needs a capture or a test of the original (cannot be settled from the DLLs).

---

### 5. Control-center target bars and buttons (`CCTargetControl_c` FUN_10074d26, buttons at +0x148; `TargetHealthBar_c`, `TargetHeader_c`, `TargetTargetButton_c`)

Client: `vitals.rs` `target_buttons` (84-104), `gui/controlcenter.rs` (target docks 249-250, buttons 555-570), `ui.rs` `target_frame` (2849-2940).

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| V1 | Click the first button of the control built with flag 0 ("self" icon) | FUN_10073651: byte +0x154 == 0 -> `FUN_10027988` then `FUN_10026509` = `TargetingModule_t::SelectSelf`. | [V 10073651; DEVIATIONS.md TGT-011 gives the same] | The left button `target_self` -> select self (ui.rs 1792 `requests.target = 0`). | vitals.rs 90, 102-104; ui.rs 1792 | MATCH |
| V2 | Click the first button of the control with flag 1 ("other" icon) | `N3Msg_PerformSpecialAction(0xb)` (start combat). | [V 10073651 (push 0xb, import 0x101a7650)] | The right button `target_next` = the attack action. | vitals.rs 92 | MATCH |
| V3 | The control's second and third buttons | Second (FUN_10073605): `AFCM::Send(0x1e, 0x105)` (flag 0) or `0x106` (flag 1). Third (FUN_1007362b): `AFCM::Send(0x1e, 0xe8)` or `0xe9`. What these two message numbers do is [NR]. Pictures `SetGfx 0x93..0x98`. | [V 10073605, 1007362b, ctor 1007526c-100753c4] | None drawn. | vitals.rs, controlcenter.rs | MISSING |
| V4 | The target-of-target button (`TargetTargetButton_c`, ctor line 10074f8e..) | If the target-of-target identity (+0x13c) is set and `N3Msg_CanClickTargetTarget(identity, ...)` is true: `AFCM::Send(0x1e, 0x126, identity)` (select it). Its mouse handlers 0x101298db/0x101294c9/0x10129a37 are a button's. | [V 10073b8e, vtable TargetTargetButton_c] | The target's target is shown as text under the bar (not clickable). | ui.rs 2905-2912 | MISSING |
| V5 | Click on a target bar's header or health bar | `TargetHeader_c`, `TargetHealthBar_c` and `CCTargetControl_c` keep the default mouse slots; the creating function FUN_10074d26 connects only the buttons' +0x148 signals. No handler found. | [V vtables; FUN_10074d26 connects (10074f8e, 10075139, 100752a6, 1007534d)]; [NF] a click handler on the bars | The frame is an `egui::Area` with `Sense::hover` and `movable(true)` (dragging moves it). No click. | ui.rs 2858, 2861, 2868 | MATCH (nothing on both sides; the original's bars carry `FadeGroup`, not a drag: whether the original lets the bars be moved is [NR]) |
| V6 | The player's own health / nano / experience bars in the wings (`CharacterBar_c`): click = select self? | Not read: `CharacterBar_c`'s vtable and its views were not opened. | [NR] | Bars are painted only (vitals.rs). | vitals.rs | UNSURE |
| V7 | Raid bar (`RaidBarView_c`): left press | `FUN_101016aa` button 1: marks the drag start (+0x4e/+0x4f/+0x50) with capture, then looks the member up (`N3Msg_GetTeamMemberList(team +0x51)`, entry +0x52) and sends `AFCM::Send(0x1e,0x127,identity)` (select). Moving drags the **window** unless `LockRaidBars` is on (FUN_10100976 -> `Window::MoveBy`). | [V 101016aa, 1010096c, 10100976] | Raid bars are not ported (raid.rs lines 17-18). | raid.rs 17 | MISSING |
| V8 | Raid bar: right press | `PopupMenu_c(2)` with one entry (text key 0x2710, value 1) -> LAB_1010093c, opened with `Go`. Entry's text and effect [NR]. | [V 101016aa button==2] | None. | raid.rs 17 | MISSING |

#### To port (V3, V4)

- **V4.** Add a small button in `target_frame` for the target-of-target (when `bar.target_of` is set and the target-of-target can be clicked): send the select-target message for that identity (`select` as the team Command::Select does, `AFCM 0x1e/0x126` on the wire is the original's). The client's `TargetBar` has the name only (`target_of`); it needs the identity. MEDIUM.
- **V3.** Read what `AFCM 0x1e` messages `0x105/0x106` and `0xe8/0xe9` are in `Gamecode.dll` (`gcf.py` over the AFCM handler table) before porting; then add two buttons to each control. MEDIUM, needs reading first.
- **V7/V8.** Port after the raid bars (`RaidBar.xml`); the select message is the same as T1. MEDIUM.

---

### Counts (rows above, 65 in all)

| Window | MATCH | DIFFER | MISSING | ADDITION | ASKED | UNSURE |
|---|---|---|---|---|---|---|
| Planet map (P1-P14) | 2 | 5 | 4 | 1 | 0 | 2 |
| PF map (F1-F12) | 6 | 2 | 1 | 0 | 1 | 2 |
| Team (T1-T14) | 11 | 1 | 1 | 0 | 0 | 1 |
| Friends (R1-R17) | 11 | 3 | 0 | 0 | 0 | 3 |
| Target and raid bars (V1-V8) | 3 | 0 | 4 | 0 | 0 | 1 |
| Total | 33 | 11 | 10 | 1 | 1 | 9 |

### Not read / not found

- What `AFCM::Send(0x12, 0x3d)` and `(0x12, 6)` do (the hold of the cursor and its release?); the client has no counterpart.
- `BitmapTileView_c` byte +0x174 (set by "Center on player") and the constant `_DAT_101ae184` in FUN_1004c2e2's re-centring formula; the pan sign convention (does the map follow the hand?).
- Whether the PF map and the planet map scroll views really scroll under the wheel (their preferred sizes); `ScrollView_c::WheelMoved`'s step source (+0x154) and its use of the `MouseWheel` option.
- The Yes-actions of friend removal and ignoring (FUN_100a93fd / FUN_100a961f bodies after the question), `FUN_100a6fff` (buddy add), `FUN_1016d595` (private invite).
- `FUN_10077e0a` (the team Leave button's handler); the exact numbers the five `TARGET_SELECTTEAMMEMBER` keys carry.
- Which messages `AFCM 0x1e/0x105, 0x106, 0xe8, 0xe9` are, and what the raid bar's menu entry (LAB_1010093c) does.
- The "Select Map" info page's per-map link text beyond the chat commands read; `FUN_1004d1cf` map-index parsing was not read as interaction.
- `CharacterBar_c` (the own bars) mouse behaviour; the `MapControlView_c` waypoint row's signal targets (GlobalSignals +0x15c, +0x17c).
- Key bindings of the maps/team/friends windows: they are in a per-user `Prefs.xml` KeyBindings (the key list is hash-keyed), not in `CharPrefs.xml`; the default hashes for `WINDOW_PLANETMAP`, `WINDOW_MAP`, `WINDOW_FRIENDS`, `WINDOW_TEAM` were not decoded by me. On the client side keybindings.rs names them (lines 49-60) but their default keys were not compared.
- Client code for Escape closing of these windows (`ui.rs` `escape_option`) was not read (GUI_DLL_PARITY.md line 60 lists `esc_planetmap`).
- Planet map tile hot spots (clicking a place name): none found in the tile view; not searched in `FUN_1004edb1`'s marker list.
- Window frame behaviour (title bar menu, drag, resize, close) is a generic window's, outside this group.

## The window frame, docking, rollup, Escape, menus


Read-only comparison, 2026-11-25. Original = GUI.dll decompile (`exports_gui`), cited as **[V addr]** (read and says this),
**[NF]** (looked, not found), **[NR]** (not read). Client = `crates/mao-client/src/` as on disk today.
Because Shift/Ctrl/Alt never reached the interface library before 2026-11-24, no "confirmed live" note was trusted; everything is by code.

### 0. How the original is wired (facts the rows lean on)

- `WndBorder` (ctor 0x1015b72c) is the window frame. It is a `BorderView_c` that **owns the tab view** (`TabView` named `window_top_view`, +0x1b8; `SetLeftMargin(20)` [V 1015b72c, _DAT_101b4e68=20]; `Layout` 0x1015a4ba sets its right margin to the width of the right-hand buttons). So tabs in the title row are the frame's own `TabView`, not the dock window's [V 1015b72c, 1015a4ba].
- Hit test [V 101596b7, sizes from SetStyle 1015b4e6]: window flag 1 = nothing is hit. Items 6/7/8/9 = corners TL/TR/BL/BR (9x9, `_DAT_101bfce8`=9.0), 2/4/3/5 = left/right/top/bottom edges (4 thick, `_DAT_101b0848`=4.0), 1 = the title strip. Edges and corners only when `flags & 0x30 == 0`; the title strip only when `flags & 8 == 0`. The strip is `TranslateBorderTop` of the 4x4 top border between the corners, top = bottom of the top border + 1.0 (`_DAT_101a87f0` is the double 1.0), bottom = 18.0 (`_DAT_101c89ac`) [V 1015a4ba] -> x 9..W-9, y 5..18 (geometry of `TranslateBorderTop` inferred from its name).
- `WndBorder::MouseDown` 0x101598d8 / `MouseUp` 0x10159988 / `MouseMove` 0x10159f08 **never test the button number**: any button starts and ends a move or resize.
- `View::MouseDown` (0x1014ca5d) and `View::MouseUp` (0x1014cb90) by default pass the event to the **parent view** (after the view's own filter signal): a press nobody handles bubbles up to the `WndBorder`.
- Click count is the third `MouseDown` argument (double click = 2, sent on the second press; `GUIModule_c::MouseDownMessage` 0x1012f951; DEVIATIONS WIN-0xx note on `NewGuiSpot_t::CheckHotSpot`).
- `WindowController_c` (0x1015867f HandleMouseMoved): the window under the mouse is the first top-level window, from the top of the z-order, that has no flag 1 and whose `Window::FindView(point)` finds a visible view; windows whose root does not intersect the point are skipped (so click-through only where the root view does not intersect). Press (0x10157d8f): that window gets `MakeFocus(true)` and, unless flag 0x400, `MoveToFront`; no window under the mouse: the focus window loses focus and the global "world click" signal (+0x3c) is emitted.
- Escape: `FlowControlModule_t::EscapePressedMessage` 0x100294d2 emits one signal on `GUIModule_c` (offset 0). Every window that registered at creation (an `esc_*` preference true), every open `DialogBox_c` (`esc_dialogs`, 0x1012a871) and a carried `DragObject_c` (`WindowController_c::BeginDrag` 0x10157842 connects `DragObject_c::Cancel` to the same signal) are slots of it, so Escape acts on all of them at once.

### 1. Window frame: moving, sizing, staying on screen, front

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| 1 | Press on the title strip and drag | Moves the window; the frame follows the mouse delta; ANY button (left, right, middle) | 101598d8 (no button test), 101596b7 item 1, 10159f08 case 1 | Drag strip, egui `Sense::drag()` = left only; strip x from skin `BORDER_TL` width (3 in the Default skin) to right minus `BORDER_TR`, y from skin top border height +1 (9) to 18 | chrome.rs `window` 714-720 | DIFFER (button; the strip's numbers are the picture sizes, the original's are the frame constants x 9..W-9, y 5..18) |
| 2 | Window with flag 8 (no move) | Strip not hit, so no move | 101596b7 | No per-window no-move; which windows carry flag 8 not read | - | UNSURE ([NR] which windows set flag 8) |
| 3 | Press on an edge or corner, drag | Resizes: item 2 left, 4 right, 3 top, 5 bottom, 6 TL, 7 TR, 8 BL, 9 BR; opposite side stays; edges 4 wide and corners 9x9, inside the frame; pointer shapes set per item (7 move, 11/10 horizontal/vertical, 9/8 diagonals) | 101596b7, 10159f08, 101597dd, 10159b69 | egui's own resize handles (`resizable(true)` on 51 builder calls), grab radius from egui style (`resize_grab_radius_*`), reaching outside the frame | chrome.rs `window` 603-621; ui.rs `on_window_edge` 713-722 | DIFFER (geometry and button) |
| 4 | Window that forbids sizing | `flags & 0x30 != 0` disables all of items 2-9; flags 0x10/0x20 are tested per axis only inside the drag | 101596b7, 10159f08 | Per window `resizable(false)` or `[x,y]` | chrome.rs 607-612 | UNSURE (which windows use which flag not read) |
| 5 | Size limits while sizing | Client size clamped to `SetSizeLimits` min/max (default 0..0x7fffffff), then the frame's own minimum (width of the buttons, `+0x2ec - 1`) | 10159b69, 10159310, 1015a4ba | `min_size` on some windows only; no button-width minimum | e.g. browser.rs:173 | DIFFER (SMALL) |
| 6 | Size snapping to a grid | `AlignRect` rounds the new client width/height to the window's alignment (default 1x1 = none); `SetAlignment` called from two places (0x1003c195 the dock window, 0x100dd42d) | 10159518, 10154562 callers | No grid snapping on window sizes | - | UNSURE (the alignment values not read; probably the dock window and one list window) |
| 7 | Dragging a window partly off screen | Allowed: no clamp in `MouseMove`/`DoSetFrame`; only the mouse is clamped to the screen (0..width/height). Keeping inside the screen happens only at `LoadWndConfig` (`MoveInsideScreen(false,true,true)`), `MoveToMouse` (dialogs) and popup `Go` | 1015867f, 10159b69, 10155036, 101550de, 1013c13a | egui `Window` default `constrain(true)`: the whole window is held inside the screen at all times (no `constrain(false)` in `chrome::window`) | chrome.rs `window` 689 | DIFFER |
| 8 | Snapping to screen edges or other windows | None found | 10159f08, 10159b69, 1015867f | None | - | MATCH ([NF] both) |
| 9 | Click anywhere on a window | `MakeFocus(true)` and `MoveToFront` on a press of any button, unless flag 0x400 | 10157d8f | egui brings an area to the front on a press in it | (egui) | MATCH (UNSURE for flag 0x400 windows) |
| 10 | Double click on the title strip | Nothing: `WndBorder::MouseDown` ignores the click count; no roll-up of the window | 101598d8 | Nothing | - | MATCH |
| 11 | Click on a window's empty area | Eaten by that window (its root view intersects); no click-through to the world or a window below. Click-through only where `FindView` finds no view (a point outside the root's shape) | 1015867f, 10154aa1, 101549f2 | egui area under the pointer consumes the event (`consumed`), the world does not get it | ui.rs `event` 696-711 | MATCH (UNSURE for windows with a custom `DoIntersect`) |
| 12 | Mouse wheel | Goes to the window under the pointer, not the view under it: `Window::_HandleMouseWheel` calls `WheelMoved` (slot 0x90, bubbles to the parent) on the window's default wheel view (+0xa4, `SetDefaultWheelView`) else on the object at +0x84; with no window under the pointer the global wheel signal (+0x21c) is emitted | 10157cec, 10154ab9, 1014a8ef | egui: the scroll area under the pointer takes it | (egui) | UNSURE ([NR] what +0x84 is and which windows set a default wheel view); probably DIFFER where a window has a default wheel view |

**Rules to port (DIFFER / UNSURE rows)**
- Row 1: the move strip is the window-local rect x 9..W-9, y 5..18 (frame constants, not the skin's piece sizes), and it must start on press of ANY button and end on release of any. Change `chrome::window` lines 714-720 (`Sense::drag()` accepts only the primary button; use `Sense::click_and_drag()` and read `ctx.input(|i| i.pointer.any_down())` or read each button's drag). SMALL.
- Row 3: use the frame constants: edges 4 px, corners 9x9 inside the frame, any button, from `chrome::window` (replace `.resizable(..)` handles by own `ui.interact` rects like the move strip, then set the area size). MEDIUM.
- Row 5: minimum width = widths of the border buttons (+tabs margin); SMALL once row 3 is own code.
- Row 7: `constrain(false)` in `chrome::window`'s builder and clamp the pointer to the screen; keep `MoveInsideScreen`-style placement only when a window's frame is loaded (layout.rs `saved`) and for dialogs/menus. SMALL.

### 2. Frame buttons and the frame menu

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| 13 | Frame menu button (left end of the title) click | Toggle button; ON opens `PopupMenu_c::Go` at the button's left and bottom + 5 (`_DAT_101a9da8` = double 5.0), screen coordinates; closing the menu turns it OFF. Exists only for window styles 0, 1, 3 without flag 4 | 1015ae85, 1015aa2f, 1015adff | Click on `GFX_GUI_WINDOW_ICON_I` asks the menu at (rect.left, rect.bottom + 5) | chrome.rs 724-737 | MATCH; and FIXED 2026-11-25: the button is a toggle and opens the menu as the left button goes down (`chrome::on_press`) |
| 14 | Frame menu: entries | "AutofadeWindow" (boolean, checked while NOT pinned, i.e. `1 - pin`; only when the pin button exists = flag 0x800 unset) then "CloseWindow"; text ids from category 10000 | 1015b34f (calls.py names), 1015b2ed | The two entries, "Autofade Window" with check `!pinned`, "Close Window" | chrome.rs 739-756, 743-744 | MATCH |
| 15 | Window's own menu entries | The window's view adds entries by connecting to the border's "menu opening" signal; relative order to Autofade/Close depends on `SignalBase_c::_Connect` insertion order, which is not in GUI.dll | 1015b72c connects SlotIconMenuOpening | Own entries first, then Close (`close_entry`) | chrome.rs 195-219 | UNSURE ([NR] signal insertion order) |
| 16 | "Autofade Window" chosen | `SlotFadeMenuEntry` -> `SetPinButtonState(!state)`: flips the pin, updates the check, `FadeTo(1.0, 0, 200000)` | 1015b2ed, 1015b23a | `set_pinned(win, !pinned)` -> fade to 1.0 over 0.2 s | chrome.rs 33-46, 749 | MATCH |
| 17 | Pin button click | Toggle button with no slot: only its own state flips (visual); `SetPinButtonState` (the immediate fade-in) runs from the menu entry and `LoadWndConfig` only; state read by `_CanFade` and saved as `WindowPinButtonState` | 1015ae85 (no `_Connect` for the pin), 10159181, 10154b6f | Click flips the pin and also fades to 1.0 | chrome.rs 789 | MATCH; and FIXED 2026-11-25: the pin is a toggle and flips on the press |
| 18 | Close button / "Close Window" | Posts message 0x98968b to the window's looper (the window's own handler decides: dock windows ask every view to close, shops tell the server) | 101599e6 | `close = true` -> `*open = false`; dock windows `close_others` | chrome.rs 795, 808-813 | MATCH (the window-specific close handlers not re-read here) |
| 19 | "?" help button | Only on windows that call `SetHelpFile` (nine call sites); click emits `file://<page>` on GlobalSignals +0x188 for the info view | 1015b139, 1015a103, callers of 10154be2 (10063ff8, 1007e9d7, 100f0d2e, 100f2e32, 100f3e80, 100f499a, 100f750e, 100fcbac, 10100022) | `HELP_PAGES` (nine windows), click queues `file://page` | chrome.rs 150-160, 794 | MATCH (the pages' names not compared one by one) |
| 20 | Window pin `0x800` windows | No pin button, no menu entry, never fades (`_CanFade` false); borderless chat windows | 1015ae85, 10154b4d | `bare_window` | chrome.rs 827 | MATCH |
| 21 | Dock button in the title | None in the original | 1015ae85 | Extra third button "dock into the column" for non-dock-area windows | chrome.rs 761, 779-792 | FIXED 2026-11-25 (the ADDITION is gone, WIN-180): no button docks; Faction and Stats are dock views now (`docks::VIEWS`), carried by their tab or page, and the pet window is no page |

### 3. Fading when the pointer leaves

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| 22 | Pointer leaves a window (no mouse button held) | Old window: if `_CanFade` (flag 0x800 unset and pin off), not modal, no modal children: `FadeTo(0.3, start after 2,000,000, duration 1,000,000)`; the old window's whole fade group (all windows with the same non-empty group name) | 1015831b (calls.py: 0x1e8480 and 0xf4240), 10154b4d | `fade_to(name, 0.3, 2.0, 1.0)` if not pinned and not a dialog; ignores buttons held | chrome.rs 125-145 | MATCH (time unit assumed microseconds: 200000 = the pin's 0.2 s, same unit; `get_system_time` unit not read) |
| 23 | Pointer enters a window | If `_CanFade(new)`: `FadeTo(1.0, 0, 200000)` for it and its group | 1015831b | `fade_to(name, 1.0, 0.0, 0.2)` | chrome.rs 133 | MATCH |
| 24 | Moving between two windows of the same fade group, or a dock window and its dock base | Fade groups compared; `GetDockBase` chain followed first; same group = nothing fades | 1015831b | No groups; the area window's own hover | chrome.rs 125-145 | UNSURE ([NR] whether any window has a fade group: `Window::SetFadeGroup` has no static caller; `View::SetFadeGroup` has two) |
| 25 | What counts as "the window under the pointer" | FindView of each top-level window from the top; a window whose root misses the point is skipped | 1015867f | `ctx.layer_id_at(pointer) == layer` | chrome.rs 129 | UNSURE |
| 26 | Window with a modal dialog open | Does not fade (`IsModal`, `HasModalChildren`) | 1015831b, 101544fa, 10154bf4 | Dialogs themselves excluded; a parent with a dialog open can still fade | chrome.rs 90-92 | DIFFER (SMALL: skip fading while a `DIALOGS` window is open) |

### 4. Title-row tabs, docking, carrying a tab

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| 27 | Tabs live in the title row | `WndBorder`'s `TabView`; left margin 20 (past the menu button), right margin = the right buttons' width, set by `Layout`; the dock window gets an empty title (FUN_1003c78b) | 1015b72c, 1015a4ba | `title_tab_view_hits`: strip from past the icon to short of the buttons (`TITLE_ROOM`) | chrome.rs 696-703, 1246 | MATCH (by structure; exact pixel layout of the strip not read: the client's note says by pictures) |
| 28 | Left press on a tab | Any-button press on an unselected tab emits the select signal (+0x1a8); then, for the left button, `View::MouseDown` passes the press UP through TopBorderView and TabView to `WndBorder::MouseDown`, whose hit test finds the title strip: **so a left press-and-drag on a tab also moves the window** | 10146d91, 1014ca5d, 101598d8, 101596b7 | Tab selected on press; the move strip begins past the last tab, so dragging a tab does not move the window | chrome.rs 1285-1290, 716 | DIFFER |
| 29 | Right press on an unselected tab | Selects it (same signal, any button); records the press point (+0x1d8) and flag +0x1d5; captures; does NOT bubble | 10146d91 | `is_pointer_button_down_on` selects on any press | chrome.rs 1285-1290 | MATCH |
| 30 | Right button released on the same tab (no threshold on time or distance, release inside the tab's bounds, no other drag object active) | Emits +0x1ac -> `TabView::SlotBeginDrag` (0x101459dc) -> signal (index, press point, view) -> `DockWindow_c` FUN_1003c428: **picks the tab up** (a `DragObject_c` of mime `dockableview/view` with `dock_name`, `view_id`, `view_index`, a picture from `CreateDragImage`); picture's top-left = mouse minus press point; only for `DockableView_c` views | 10146ef3, 10146c40-10146c72 (connection), 101459dc, 1003c428, 1012ba1c | `response.secondary_clicked()` per tab (egui click = press and release on the same widget within about 0.8 s and 6 points) -> `carried` | chrome.rs 1291-1294; docks.rs `after` 349-362 | DIFFER (SMALL: egui drops a slow or drifting click; original has no limit) |
| 31 | Left button on a tab (selected tab MouseMove) | `Tab::MouseMove` forwards to `View::MouseMove` only for the selected tab | 10146cbe | n/a | - | MATCH ([NF] any other effect) |
| 32 | Carrying: what lets go | The object stays until `DragObject_c::Cancel` or `Accept`. Drops are tried on ANY button press over a `Tab`, the `TopBorderView` strip or the `RollupController`, and on ANY button release over empty space; Escape cancels | 10146d91 (press with drag object), 10147d98, 10049121, 10157f13 (end), 10157842 | Only a left click (press and release, `primary_clicked`) puts it down; Escape cancels (client doc calls this "its own") | docks.rs 467-476 | DIFFER (the button and the press-against-click part: left). The Escape part FIXED 2026-11-25 for a real keyboard: `Ui::close_with_escape` -> `docks::cancel_carry` |
| 33 | Put down on another window's tab | Source `dock_name` found; `view_id` found; removed from the source (`RemoveView`); inserted into the target area at the index of the tab it is on (position of that view in the area's list), else last; `DragObject::Accept(2)` | 1003c5a9, 10145a95 | `drop_on(Target::Tab)`: before that tab; selected after | docks.rs `drop_on` 214-232 | MATCH (selection after the drop: the original's insert call (area vtable +8) not read: UNSURE whether the moved tab becomes selected) |
| 34 | Put down on the strip past the last tab | `TopBorderView` hit when the pointer is in its bounds and right of the last tab's frame (or x > 20 and y < 100 when empty); signal with tab null -> index = tab count (last) | 10147cca, 10147d98, 10145a95 | `Target::Strip` -> last | docks.rs 197-232 | MATCH |
| 35 | Put down on the only tab of its own window | Cancelled | 1003c5a9 (`GetTabCount == 1` -> `Cancel`) | `Dropped::Cancelled` | docks.rs 214-216 | MATCH |
| 36 | Put down on a window's body (not its tab strip) | No handler in the code read: the view's `MouseDown`/`MouseUp` bubble to the root; the drag object is not cancelled by `HandleMouseUp` because a window is under the mouse. The object stays carried (item lists cancel on their own rule: 0x100cc855) | 10157f13 (cancel only when no window under the mouse), 1014ca5d | `Target::Nowhere`: a new area is made wherever the left click lands, including over another window's body | docks.rs 438-444 | DIFFER (UNSURE for views whose drop handlers cancel; for plain windows the original keeps carrying) |
| 37 | Put down on no window at all | A drop signal (+0xc of the drag object, connected to FUN_1003b263 only when the source area has more than one view, `FUN_100399fa() > 1`): a new `DockAreaN` (first free) whose window frame is `Bounds` of the view (its size) + the DROP POINT, i.e. top-left at the pointer; view inserted last; `Accept(2)` | 1003c428, 1003b263 (`Rect::operator+(point)` argument is the signal's point), 1003aca6 | New area when its area lists more than one view; window placed at `pointer - grab` | docks.rs `drop_on` 233-243, 482 | DIFFER (SMALL: top-left at the pointer, not minus the grab offset) |
| 38 | Close a dock window | Asks every view of the area to close (FUN_1003c94d per the client's note) | not re-read | `close_others` | docks.rs 394-402 | UNSURE ([NR] FUN_1003c94d) |
| 39 | Chat windows' tabs | Same `Tab` class: same gesture (right press+release picks up) | 10146ef3 | Same path (chat windows use `tab_view_hits`) | docks.rs / chatwindow.rs | MATCH (CHT-001 done) |

**Rules to port**
- Row 28: left press on a tab = select it AND start the window move (hit item 1) for any tab inside x 9..W-9, y 5..18; implement by making the title strip cover the tab row too (the strip sits UNDER the tabs and tabs do not consume a left-drag). Change `chrome::window` 714-720 (strip must run from `outer.left()+9` to `outer.right()-9`, not start past the last tab) and let the tab `ui.interact` use `Sense::click()` only (it does) while the strip uses drag. SMALL.
- Row 30: record the right-button release as "press began on this tab and release is inside its rect", with no time or distance limit (read `pointer.button_released(Secondary)` and the press origin instead of `secondary_clicked`). SMALL.
- Row 32: while a tab or page is carried, drop on (a) any-button PRESS over a tab, the strip, or the rollup column, (b) any-button RELEASE with no window under the pointer. `docks::end_frame` currently tests `primary_clicked`. Fix the CHT-001 doc line (Escape IS the original's rule, 0x10157842 + 0x10158eea). SMALL.
- Row 36: only run `Target::Nowhere` when no window layer is under the pointer (`ctx.layer_id_at(at)` is None or a non-window layer); over a window body keep carrying. SMALL, but check item-list windows that cancel on a foreign drag.
- Row 37: place the new window's top-left at the pointer. SMALL.
- Row 26: add "no fade while a dialog is open over it".

### 5. The rollup column (`RollupController_c`, `RollupPage_c`)

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| 40 | Left press on a page's header or view (above the view's bottom) | Single press: starts "scroll the column with the pointer" (state 1; the controller's `ScrollBy(0, dy)` follows the mouse until release) | 10049e00, 100484da, 100484be | `Held::Scroll` | rollup.rs `press` 401-416, 554-567 | MATCH |
| 41 | Left double click (second press, count 2) there | `FUN_1004aafd` -> `FUN_100496ee(!expanded)`: expand/collapse; stores `is_page_expanded` | 10049e00, 1004aafd, 100496ee | `button_double_clicked(Primary)` -> toggle | rollup.rs 410-412 | MATCH |
| 42 | Left press below the view's bottom (the 5 px grip) | State 2: page height follows the pointer, clamped between the view's own min (+0x16c) and max (+0x170) and snapped by +0x174 | 10049e00, 100484da, 100483ca | `Held::Size`, height only kept above 0; client doc says views' min/max/grid not done | rollup.rs 405-408, 559-571 | DIFFER (SMALL-MEDIUM: min/max/grid steps; [NR] where each view sets them) |
| 43 | Right PRESS on a page | Emits the page signal (+0x128) -> FUN_100493f2: starts a carry (`dockableview/view` with `dock_name`, `view_id`, `view_index`) at once, on press (not release as for tabs) | 10049e00, 100493f2 (summary; body not re-read) | `Secondary` pressed -> `carry` | rollup.rs 403-406 | MATCH (UNSURE: FUN_100493f2 read by summary only) |
| 44 | Pointer shape over the page | Resize cursor (id 10) over the grip, normal otherwise; hand state while held | 100484da | `ResizeVertical` over the grip | rollup.rs ~535 | MATCH |
| 45 | Header menu button | Menu at the button's left, bottom + 5 (`_DAT_101a9da8`): the view's own entries (view vtable +0xf8) then "CloseView" (view vtable +0xfc) | 10048c5c, 10048de9, 1004a1b1 | `ask_frame_menu` at (left, bottom+5); view's own menu or "CloseView" | rollup.rs 509-516, chrome.rs 204 | MATCH |
| 46 | Header roll and close buttons | Three toggle `HeaderButton_c`s (gfx 0x15e/0x15d/0x15f roll, 0x1c0-0x1c2 close, and the menu icon); roll toggles expansion, close closes the view | 1004a740, 1004aafd | Menu, roll, close buttons | rollup.rs 469-510 | MATCH; and FIXED 2026-11-25: the three are toggles (`PageHeaderView_c`, FUN_1004a740) and act on the press |
| 47 | Put a carried tab or page on the column | `RollupController_c::MouseDown` (any button) with a `dockableview/view` object: source removed, inserted before the page whose frame contains the point, else last; cancelled when it is the column's only page and the source | 10049121 | `column_target` + `put`: before the page put on, else last; only-page cancel | docks.rs 477-481, rollup.rs 346-398 | MATCH (press versus click differs as in row 32) |
| 48 | Mouse wheel | Controller's `WheelMoved` scrolls by `dy * 40` (`_DAT_101b0be0` = double 40.0); the global wheel signal (+0x21c, sent when NO window is under the pointer) is connected to it only while preference `MouseWheel` == 1 | 100487aa, 10048822, 10157cec | `rollup::wheel(lines)`: 40 a notch with `MouseWheel` 1 over the world | viewer.rs 1928-1934, rollup.rs 313-321 | MATCH |
| 49 | Reorder by dragging | By carry and drop only (row 43, 47) | 10049121, 100493f2 | Same | rollup.rs | MATCH (WIN-034's "no drag to reorder" is out of date) |

### 6. Escape and the dialog boxes

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| 50 | Escape pressed | One signal; every registered slot runs: windows with their `esc_*` preference true close, open dialogs answer -1, a carried drag cancels. Pin is not tested | 100294d2, 1002c65c, 1012a871, 10157842 | Escape asks every window whose option is set to close the next time it is drawn; docks carry cancel | ui.rs `close_with_escape` 741-770, docks.rs 467 | MATCH; and FIXED 2026-11-25: Escape also puts back what is in hand (an item, a split, a carried tab), which a real keyboard's Escape never did (`Ui::close_with_escape`) |
| 51 | Order of closing under Escape | Order of signal connection (creation/open order); effects are independent | 1002c65c | Not ordered (all at once) | ui.rs 744 | MATCH (UNSURE: dialogs and windows that interact) |
| 52 | Escape with a popup menu up | The menu's own `KeyDown` takes the key (codes 0x1b, 0x0e, 0x19 = cancel with -1); whether the flow-control signal fires too is [NR] | 1013d1cc | `take_keyboard()` while a menu is up | gui/popup.rs 275 | UNSURE |
| 53 | Dialog placement | `DialogBox_c::_Init`: `MoveToMouse(true)` = centred on the mouse (position = mouse - floor(size * 0.5)), then `MoveInsideScreen(true,true,true)`; `AddModalWindow` | 1012a7a8, 101550de | Pivot centre at the pointer when new, kept on screen by egui | chrome.rs `dialog_placed` 316-345 | MATCH |
| 54 | Dialog Escape / close | `SlotEscPressed` -> `SlotSelected(-1)`: signals -1, then hides (autoclose off) or deletes | 1012a5ee, 1012a49b | `Some(None)` | chrome.rs 380-385 | MATCH |
| 55 | Dialog Enter | No default-button use found (`SetDefaultButton` has no caller); a text dialog's Enter in the field answers as the first button | 10154509 (no callers), 101484a4 | Enter handled only in `text_dialog` | chrome.rs 389-460 | MATCH ([NF] default key) |
| 56 | Parent window with a dialog open | `Window::_HandleMouseDown`: when the window has modal children (+0xac) a press on its tab bar is ignored; other presses go through; no blanket modal block found | 10154ea2, 10154bf4 | No modal blocking | - | UNSURE (the original's blocking of the rest, if any, is [NR]) |

### 7. `PopupMenu_c`

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| 57 | Open | Window flags 0xd3c, modal, `MoveInsideScreen`, shows, takes focus; the global "mouse down" slot is connected one frame later (frame timer + `SlotMouseButtonGraceTimer`) | 1013c13a, 1013ce3f | Takes the keyboard; "outside" test only after `opened + 1` frames | gui/popup.rs 273-285 | MATCH |
| 58 | Press (any button) in the menu | Captures; `SelectHighlightedItem` (opens a sub-menu at once); sets pressed flag +0x259; with the keep-open flag (+0x258, multi-select) and Shift, activates without closing | 1013d179 | `pressed = true`, `open_sub_now`; presses ignored for the first 2 frames | popup.rs 449-452 | MATCH (ADDITION small: the 2-frame guard on presses INSIDE the menu) |
| 59 | Release (any button) | Only if the press began in this menu (+0x259): `ActivateHighlightedItem`; so press-hold-RELEASE on an entry from a button pressed elsewhere (the opening right button held) does NOT choose; click-click works | 1013ca36 | `up && pressed` -> `activate()` | popup.rs 482-487 | MATCH |
| 60 | Activate | Entry with a sub-menu: nothing; disabled entry: closes with -1; else item's own signal then `_ItemSelected` (clears highlight, signals, closes) | 1013c90f, 1013c848 | `activate` | popup.rs 320-327 | MATCH |
| 61 | Press outside every menu of the chain | `SlotGlobalMouseDown`: `_ItemSelected(-1)`: closed, no choice; the press still goes to what is under it (the slot is a side signal) | 1013cafc | `outside` -> `Event::Closed` | popup.rs 278 | MATCH (UNSURE whether the client lets the same press reach the window below) |
| 62 | Mouse move | Highlights the enabled non-static entry under the pointer on every move; pointer entering a sub-menu re-highlights the parent's entry | 1013bdc2 | Same | popup.rs 390-401 | MATCH |
| 63 | Keys | 0x1b / 0x0e / 0x19 cancel; 0x18 activate; 0x22 up, 0x23 down (skip static; Up from top = first, Down at end stays); 0x21 right = open sub-menu; 0x20 left = close sub-menu if one is open; 0x0f page by (menu height / entry height), reversed with Shift | 1013d1cc | Escape/Delete/Backspace, Enter, Up/Down, Right, Left (depth > 0), Tab with Shift | popup.rs 503-520 | MATCH (key-code to key mapping 0x1b/0x0e/0x19 = Esc/Del/Backspace taken from the client's doc, not re-verified) |
| 64 | Typing | `TextInput` appends to a buffer (+0x220), restarts a timer of 1,000,000 units; `HighlightSearchMatch` highlights the first entry whose label begins with it, no case, leading `/` stripped | 1013c59b, 1013bf6f | Same, `SEARCH_FOR` | popup.rs 494-503 | MATCH |
| 65 | Sub-menu open delays | Timer-driven (`SlotSubMenuTimer`); the 300 ms / 200 ms values are in `HighlightItem` 0x1013bc01 | 1013cbb6 (placement read; delays not) | 300 / 200 ms | popup.rs `OPEN_AFTER`/`CHANGE_AFTER` | UNSURE ([NR] HighlightItem) |
| 66 | Shift-click keep-open (`EnableMultiselect`) | Flag +0x258; client doc says no caller | 1013d179 | Not present | popup.rs header | UNSURE ([NR] caller scan) |

### Counts (66 rows)

MATCH 42, DIFFER 11, MISSING 0, ADDITION 1, ASKED 0, UNSURE 12. (Some MATCH rows carry a caveat in their text.)

### Not read / not found

- Which windows have `Window` flags 8, 0x10, 0x20, 0x400, and the meaning of every `WindowStyle_e` value (only 0, 1, 3 seen in `CreateBorderIcons`).
- The exact layout of the `TabView` strip inside the title row (heights, overlap) and `TabView::GetBorderSizes`; `DoSetFrame` sets the TabView to the client rect, which was not reconciled with the strip's place.
- `SignalBase_c::_Connect` (not in GUI.dll): order of slot calls, so the order of a window's own frame-menu entries against "AutofadeWindow" and "CloseWindow".
- `get_system_time`'s unit (the 200000 / 1000000 / 2000000 are taken as microseconds, consistent with the pin).
- Fade groups (`Window::SetFadeGroup` has no static caller) and `GetDockBase` use.
- FUN_1003c94d (closing a dock window), FUN_100493f2 (page carry) bodies, `FUN_1004a1ee` (page close), `DockArea_c::AddView` result on selection.
- `SetAlignment` arguments at 0x1003c195 and 0x100dd42d; `WindowController_c` +0x84 (the wheel's fallback object); `SetDefaultWheelView` callers (indirect).
- `PopupMenu_c::HighlightItem` timings; `EnableMultiselect` callers; the key-code table behind 0x0e/0x18/0x19/0x1b/0x20-0x23.
- Whether anything blocks clicks on other windows while a modal `DialogBox_c` is open (only the tab-bar case was found).
- The client's per-window Escape table against LoginPrefs.xml `esc_*` (24 entries) was not compared entry by entry; per-window close handlers of the original (0x98968b) not read.
- Modifiers: no Shift/Ctrl/Alt rule was found in `WndBorder`, `Tab`, `TopBorderView`, `RollupPage_c` or the window controller's press path ([NF]); the only Shift test in this group is `PopupMenu_c`'s keep-open and Tab-page-reverse.

## Clicks in the world


Read-only comparison. Original = GUI.dll / Gamecode.dll / N3.dll decompile (`exports_gui`, `exports_gamecode`, `n3.c`);
client = `modern-anarchy-online/crates/mao-client/src` as on disk 2026-11-25.
Short names: `vw` = viewer.rs, `pk` = picking.rs, `pt` = pointer.rs, `di` = dropitem.rs, `net` = net.rs.

### 0. How the original wires the world (read first; every row leans on this)

All **[V]** below.

- **The handler object** is `ActionViewMouseHandler_c`, made by FUN_1002d1ae (ctor). It connects to `WindowController_c`
  signals (offsets read from the disassembly at 1002d1e8-1002d219): `+0x3c` MouseDown(Point, button, clickcount) -> **FUN_1002ce31**;
  `+0x40` MouseUp(Point, button) -> **FUN_1002cfac**; `+0x38` MouseUp -> **FUN_1002cc90** (ends the look). It also connects
  GlobalSignals `+0x220` (per-frame mouse movement, a Point) -> **FUN_1002ccbe**, and `+0x10`, `+8`, `+0x48` -> **FUN_1002cc28** (end look).
- **What reaches the world.** `WindowController_c::HandleMouseDown` (10157d8f) emits `+0x3c` only when `this+400` (the window under
  the pointer) is null. `HandleMouseUp` (10157f13) emits `+0x40` only when `this+400` is null, but emits `+0x38` always. The wheel
  (`HandleMouseWheel` 10157cec) goes to GlobalSignals `+0x21c` only when no window is under the pointer. A drag object let go with
  no window under the pointer (any button) gets its `+0xc` signal, then WindowController `+0x44`, then `DragObject_c::Cancel`.
- **Event codes** (CheckHotSpot 1001db81): state 1 = left down (click count 1), 3 = left **double** down (count 2), 4 = left up;
  5/7/8 the same for the right button; 9/0xb/0xc middle; 0x79-0x7e the X buttons. All call `MouseDownMessage(button, count)`.
- **What is "under the pointer"** is `InputConfig_t` `+0xb0/+0xb4` (an Identity), set every frame by
  `InputGameSubmodeActionViewONMessage` (1001c024: `N3Msg_SetMousePos` -> `+0xa4..+0xac` the ground point, the identity ->
  `InputConfig_t::CheckObjectUnderMouse` 1001a33f -> `SetCurrentReference` 1001a191, which keeps the last non-0x9c47 identity for 500 ms).
  The identity comes from the camera: `n3Camera_t::GetObjectUnderColLine` (1002069c: the camera's selected target `+0x1e4` if it is on the
  list `+0x244`, else the list's first, else 0x9c47) and the list is built by `RunCameraSelectionCollChecks` (1000d8d1: every 0.1 s, 200 m).
- `InputConfig_t +0xd4/+0xd8/+0xdc` is the own character (`SetCharacterID` 1001a171: `+0xd8 = 50000`, `+0xdc = id`).
- Identity types: 50000 (0xC350) = character, 0x9c47 = "nothing under the pointer", 0x9c52 and 0 = "nothing to act on".

### 1. Table: clicks and gestures in the world

| # | Interaction | Original does | Verified at | Client does | Client at | Verdict |
|---|---|---|---|---|---|---|
| 1 | Left click on nothing: ground, sky, or the reference identity is 0x9c47 | Nothing. No deselect. (The release handler returns when the reference type is 0x9c47, 0x9c52 or 0.) | **[V 1002cfac]** | `left_click` returns None when nothing is under the pointer; the half-second memory (`Reference`) is as `SetCurrentReference` | pk `left_click` 414, `Reference::set` 330 | MATCH |
| 2 | Left click (release, no Shift/Ctrl): which of several things on the line | The one **after the selected target** on the camera's list (list order nearest first, wraps to the first; the first when the selected one is not on the list); if that is 0x9c47/(0,0) and the reference is not 0x9c47, the reference itself. Then `AFCM::Send(0x1e, 0x126, id)` = select. Taken on the **release**, only if no drag object and (no look started or travel < 0.02) | **[V 1002cfac, n3.c GetNextTarget 10020723, GetObjectUnderColLine 1002069c]**; the 0x1e/0x126 receiver is SetTargetMessage 1002676c -> `TargetingModule_t::SetTarget(id, false)` 10026308 (that 1002676c is registered for 0x1e/0x126 was not read: **[NR]**) | `next_on_line` (wrap, first when selected is off the line) else `under`; `net` selects: `target = Some(id)`, for a character `select_target`, `set_target`, `info_request` | pk 309, 414; vw 1535-1560; net 11192-11230 | MATCH |
| 3 | What is on the line (list, rate, range, own character) | Every dynel within 200 m of the eye that the mouse line meets, rebuilt at most each 0.1 s, nearest first. One's own character is on it by distance and left out only when the camera mode is 0 (first person). Shapes: `BoundingBoxTargeting` -> `vtable+0x94(...)`; a dynel's visibility check `vtable+0x8c` comes first | **[V n3.c 1000d8d1 (0.1 s, float 0x1003ce54), FUN_10020a3c]**; the shape maths (box/triangles) **[NR]** | `on_line`, 200 m, 0.1 s, own character left out only in first person; shapes box / mesh / post | pk 296, `REBUILT`, `GATHER`; vw 3498-3511 | MATCH (routing); shapes UNSURE (not re-verified here); the visibility check is not ported (pk header says so) |
| 4 | Left click when the reference is type 0x9c52 (or 0) | Nothing | **[V 1002cfac]** | A table dynel of kind 0x9c52 is a pickable (`Class::Nothing`) and `left_click`/`right_click` do not test the kind: it can be selected, and a right click sends a use to it | pk 414, 426; net 13097 | DIFFER |
| 5 | Shift + left click | `GetQualifiers & 3` -> `InfoViewModule_c::ShowURL`: `charid://50000/<id>` for a character, `itemid://<type>/<id>` for anything else. Acts on the **reference** (under the pointer), not on the next one; no selection | **[V 1002cfac 1002d0b0-1002d124]** | `Deed::Info` on the reference. A character: `info_request` + `state.inspecting`; a table object: `itemref://<template>/<template>/1`; anything else: a `text://` page with its name | pk 414; net 11218-11231 | UNSURE (the Shift+click itself and its target match; the page shown differs in source, and what `itemid://type/id` renders for a world dynel was not read) |
| 6 | Ctrl + left click on a **character** | `GetQualifiers & 0xc` and type 50000: `AFCM::Send(0x1e,0x126, under)` then `N3Msg_SwitchTarget(under)` = `N3Msg_DefaultAttack(id, true)`. Ctrl wins over Shift when both are held | **[V 1002cfac; 10028422; 10028147]** | `left_click` tests `ctrl && CHARACTER` first -> `Deed::Switch`: selects, then `combat::default_attack(.., switching = true, ..)` (stop first; fight on the same target only stops; flag 2 when a fight was going; refusal lines) | pk 414-422; net 11232-11246; combat.rs 352-375 | MATCH (read both sides; the attack check `FUN_10069a9f` itself is another reader's) |
| 7 | Ctrl + left click on anything that is not a character | Plain left click (select the next on the line); with Shift too, the info page | **[V 1002cfac]** | Same (`ctrl && CHARACTER` is the only Ctrl case) | pk 414 | MATCH |
| 8 | Alt + any click | Alt (qualifier 0x10/0x20) is not read by any of the world handlers | **[V 1002ce31, 1002cfac, 1002cc90, 1002ccbe]** (no Alt test) | Alt is not looked at | pk 414 comment | MATCH |
| 9 | Where Shift/Ctrl come from in the client | (original: `WindowController_c::GetQualifiers`, which until 2026-11-24 saw no keys: any earlier "confirmed live" note for a Shift/Ctrl click proves nothing) | n/a | The world's click code reads **`self.modifiers`**, winit's `ModifiersChanged` state, **not** the interface library's keys; `ModifiersChanged` is excluded from what egui may swallow, so it is always current | vw 1462-1470 (`taken` excludes `ModifiersChanged`), 1544 (`self.modifiers.control_key()/shift_key()`) | MATCH (the keys are real) |
| 10 | Right click (release) | A character: `N3Msg_DefaultActionOnDynel(id)`; anything else: `N3Msg_UseItem(id, false)`. **No selection**, no key variants, no `Can` test on the use of a thing. Released like the left (same drag-object / travel test) | **[V 1002cfac]** | `right_click` -> `Deed::Use`; `uses()` true for any thing, for a character unless it is oneself; target unchanged | pk 426, 402; net 11247 | MATCH (the branches); the effect on a character: row 12 |
| 11 | Right click / double click on one's own character | Nothing (`DefaultActionOnDynel` returns first when the id equals the client dynel's identity) | **[V 1002939b]** | `uses(Use, character, own)` false; `double_click` filters the own id | pk 402, 432 | MATCH |
| 12 | `N3Msg_DefaultActionOnDynel` on a **character** (right click; double click) | Tree: target stat flag `0x8000000` -> nothing; own stat `0x296` != 0 -> `Feedback_NotInVehicle`; reads target stat 0x1e (unused) and flags `0x200000`; if the target's `+0x138` bit 4 is clear: if target has stat `0x300` with bit 0 -> send an `n3InfoItemRemote_t` built from (own, target) (`FUN_101284a0`, `SendIIRToServer`); else if (`+0x21c` == 0 or no flag 0x200000): `TradeStart(id)` only when the target's `+0x1d4+0x44 == 1` (not fighting); else (bit 4 set, or `+0x21c` set with 0x200000): needs `+0x44 == 1`, then if flag 0x200000 and own flags have 8 -> `TradeAbort(true)`, else `N3Msg_UseItem(id)` (which for type 50000 sends the generic use of the character, 10028883). `+0x138`, `+0x21c`, stat 0x300 and the flag bits are **not identified** | **[V 1002939b, 10028883 (50000 branch)]**; field meanings **[NR]** | The client's own tree (its docs call it "the earlier approximation"): oneself nothing; a non-player -> `KnubotOpenChatWindow`; another player not fighting -> `trade` after the 5 m / rooms check; else `use_object`. No test of 0x8000000, vehicle, 0x300, 0x200000, the target's fight state, or `TradeAbort` | net 11247-11300; CLIENT_UI.md "Guesses and gaps" | UNSURE (structure differs; I could not tell which server messages the original's IIR branch is; a capture of right-clicking an NPC, a monster, a player and a corpse in the original would settle it) |
| 13 | `N3Msg_DefaultActionOnDynel` on a **non-character** (double click only; a right click uses directly) | The target must be a `SimpleItem_t`; reads stat 0x1e (`Can`): bit 0 -> **`N3Msg_GetItem`** (pick up: inventory-full check, pick-up message); else bit 3 -> `N3Msg_UseItem`; else nothing | **[V 1002939b; 10027d76 summary only]** | `uses(Default, ..)` is `can & 9 != 0` and then sends the **use**; the pick-up message (`ClientGetItemIIR_t`, built for the Pick-Up action tile) is not used for a double click. With no known `Can` it uses | pk 402-412; net 11247; n3.rs 1182 | DIFFER |
| 14 | Double click (second press, left) | Windows delivers `WM_LBUTTONDBLCLK` (0x203), which the exe's window procedure turns into state 3 -> `MouseDown(1, count 2)`. FUN_1002ce31: unless the reference equals the own-character identity (50000, own id) and when the option `DoubleclickAction` (default true) is on -> `N3Msg_DefaultActionOnDynel(reference)` **at the press**. There is **no** test of the reference's type (0x9c47/0x9c52/0 go through; harmless: the function returns for non-items). The release is then an ordinary click (row 2: it steps on) | **[V 1002ce31; exe 00404a66 (0x203 -> 3, 0x206 -> 7)]** | `second_press` -> `double_click` -> `Deed::Default` at the press; own id and the option filtered; the release steps on as any click; skipped while an item is in hand | vw 1511-1520; pk 366, 432 | MATCH |
| 15 | **Double-click time and distance (the open point)** | **Windows' own.** The class is registered with style 8 (`CS_DBLCLKS`), so the **system** double-click time (`GetDoubleClickTime`, default 500 ms) and double-click rectangle (`SM_CXDOUBLECLK/SM_CYDOUBLECLK`, default 4 x 4 px) decide. Not a preference, not a constant of the program. | **[V exe 00405d74 (WNDCLASS style = 8); 00404a66 (WM_LBUTTONDBLCLK/WM_RBUTTONDBLCLK -> codes 3/7)]** | Fixed 500 ms and +-2 px | pk 366-370 | DIFFER (small: only for a player who changed the Windows setting) |
| 16 | Left button held and moved | With `LMBMouseLook` (default true) the press sets "pending look kind 1"; the **first mouse movement** starts it (`FUN_1002cbde`: state := 1, travel := 0, `AFCM::Send(0x12, 0x3d)`); each frame `N3Msg_CameraMouseLookMovement(dx*s, dy*s)` with `s = MouseTurnSensitivity` (counts / 1000 first); the cursor is put back to where it was each frame (`SetMousePosition`). Without the option nothing starts and the release is always a click | **[V 1002ce31, 1002cbde, 1002ccbe]** | `device_event` raw motion: left look only with `LMBMouseLook`; goes to the game's camera as `mouse_look` scaled by `MouseTurnSensitivity / 1000`; the pointer is hidden and grabbed | vw 1272-1330, 1522, 1570-1585 | MATCH (routing and scale); the camera arithmetic after that (camera.rs, N3.dll `MouseCameraControl` 10021689) not re-compared here |
| 17 | Right button held and moved | Press sets kind 2 (always, no pref). On each movement: with Ctrl **not** held and kind 2 pressed -> `N3Msg_MouseMovement(dx*s, dy*s)` (state 2); with Ctrl held (kind 2) the state switches to 1 and it is a camera look (`EndMouseLook` first). `N3Msg_MouseMovement` hands the across movement to the character as a turn and the up movement to the camera by `RMBMouseLook1st` (first person) / `RMBMouseLook3rd` (third person) (both default true); `MouseLookInverted` | **[V 1002ccbe, 100196b3 (read in full)]** | `steers = right && !ctrl`: across -> `keys.face` (character turns), up -> camera by the same two prefs, `MouseLookInverted`; with Ctrl all to the camera; flying pitch | vw 1272-1330 | MATCH (routing). UNSURE for the rest of `N3Msg_MouseMovement`: it sends movement actions 0xa / 0xd / 0x2b by the character's movement mode (0, 3, 4) and clamps by stat 0x10b; the client sends a heading, not those messages (DEVIATIONS TGT-002 / MOV-024) |
| 18 | The look is the last pressed button's; either release ends it | `FUN_1002ce31` overwrites the pending kind at each press; `FUN_1002cc90` (any release of button 1 or 2, no drag object) -> `FUN_1002cc28`: `AFCM::Send(0x12, 6)`, `EndCameraMouseLook` or `EndMouseLook` by state, state := 0 | **[V 1002cc90, 1002cc28]** | `look_right` = last pressed; a release of either clears `buttons` and the look | vw 1531-1536, 1570-1600 | MATCH |
| 19 | Click or drag? | A release is a click when no look has started or the summed movement of the look is below **0.02** (per frame, the length of the frame's summed counts / 1000; float 0x101aeafc = 0.02). Travel counts only after the look started | **[V 1002cfac, 1002ccbe; GUI.dll data 0x101aeafc = 0.02, 0x101ade10 = 1000.0 read from the file]** | `moved += |motion|/1000` per raw event while a button is down; `looked = moved >= 0.02 && (right || LMBMouseLook)` | vw 1272-1285, 1522 | MATCH (differs only in summing per event, not per frame sum; not worth porting) |
| 20 | A release that happens over a **window** after the press in the world, and a press on a window released over the world | The world gets the up event only if **no window is under the pointer at the release**; there is no "pressed in the world" test. So: press in the world, release over a window: **no click** (only the look is ended by `+0x38`). Press on a window (no look pending), release over the world: **a click** on whatever is under the pointer | **[V 10157f13, 1002cfac]** | `world_release` lets a release of a button pressed in the world through whatever is under the pointer, and `clicked` is computed from `self.buttons`: press in world + release over a window **does click** (on the 0.5 s memory); press on a window + release over the world does not | vw 1457-1475, 1520-1560 | DIFFER |
| 21 | Both buttons held | Nothing special: last pressed button's look; no forward run. The WindowController keeps the button bitmask at `+0x58` (`GetMouseButtons` 1000b68b) but nothing in GUI, Gamecode or N3 reads it | **[V 1000b68b]** (no callers found: **[NF]** of any reader) | No both-buttons movement | vw 1272-1330 | MATCH |
| 22 | Middle button | Nothing: `FUN_1002ce31` acts on buttons 1 and 2 only; no key binding for it | **[V 1002ce31]**; **[NF]** binding in the default key table (`_strings` 101aab28) | Middle press resets the camera (`cam_reset`) | vw 1499-1503; DEVIATIONS CAM-001 ("the middle-button reset is the client's own") | FIXED 2026-11-25 (the ADDITION is gone): nothing; the camera's reset keeps its key |
| 23 | Mouse wheel over the world | GlobalSignals `+0x21c` -> N3.dll `FUN_100200ef` only when preference `MouseWheel` is **0** (default 0; `FUN_100215c8` connects it for 0 and disconnects for non-zero); `MouseWheel` 1 sends it to the rollup column (`FUN_10048822`); other values nothing. Step: `ZoomSpeed / 10 * y` per notch (y = wheel delta / 120), added to a running value; in first person a wheel **down** with `ZoomTo1stPerson` true sets `3rdPersonCamera` | **[V 10157cec, 1001b253 (delta/120), n3.c FUN_100215c8, FUN_100200ef; constants 120.0 and 10.0 read from the DLLs]** | `MouseWheel` 0 -> `wheel -= lines * ZoomSpeed / 10`; 1 -> rollup; 2 nothing; first person left by wheel down with `ZoomTo1stPerson` | vw 1920-1940; camera.rs 213-244, 532, 580, 855 | MATCH (the sign of "up = in" follows from the first-person rule; camera.rs's zoom limits not re-compared) |
| 24 | Escape in play | `COMMAND_ESC_PRESSED` (`[ & AnyPlayMode ! InfoViewMode ! TextInputMode ! CharacterCreationMode ] = ESC`) -> `FlowControlModule_t::EscapePressedMessage` (100294d2) -> emits GUIModule's signal; its only slots found close a dialog (`SlotEscPressed` 1012a5ee, char creation 1011b69e). It does not clear the target and does not stop an attack | **[V 100294d2, 1002c65c, 1012a5ee]**; **[NF]** any target/attack effect | Escape (action `CloseWindows`) closes the windows that ask for it; nothing to target or fight | settings.rs 326; ui.rs 739 | MATCH (the world side: nothing) |
| 25 | A mouse press/release while an item is in hand (a drag object) | Both handlers return at once when a drag object exists | **[V 1002ce31, 1002cfac]** | `free = gear::held().is_none() && !loot::item_in_hand()` gates the double click and the click | vw 1509-1524 | MATCH |
| 26 | Item let go over the world: what decides | `FUN_100cbcb5`, the drag object's `+0xc` signal (connected by `FUN_100cc6e8` -> `FUN_100cdbe3`, so only for drags that view started), fired by `HandleMouseUp` for **any** button with no window under the pointer. Only for mime `inventory/item`. The decision is on the reference (`+0xb0`): 50000 -> `N3Msg_UseItemOnCharacter(item, id)`; not 0x9c47, not 0x9c52, not 0 -> `UseItemOnItem(item, id)`; 0x9c52 / 0 -> nothing; 0x9c47 -> the ground | **[V 100cbcb5, 1003d6f2, 10157f13, 100cc6e8]** | `requests.use_on` -> by `pointer.under`: 0x9c52 nothing; a character -> `use_item_on_character`; else `use_item_on`; none -> the ground | ui.rs 1768-1783; vw 3513-3545; net 5994-6001 | MATCH |
| 27 | Item let go over the **ground** | Ground point = `InputConfig +0xa4..+0xac` (the mouse line cut at the first ground **or surface**). `N3Msg_IsDropPossible` answer: 2 -> if pref `ShowDropItemDialog` is 0 `N3Msg_DropItem` at once, else a Yes/No box naming the item (item_id, drop_pos kept); 0xd, 0xf, 0x11 -> error boxes; 0x12 -> a box built from the item's reference data ("not a fixture", delete offered, keeps item_id/template_id); any other -> an error box | **[V 100cbcb5]**; the reason function `FUN_10048dbd` **[NR]** | `dropitem` has the same six answers and the same preference; `reason()` ports part of `FUN_10048dbd` | di header, `over_terrain` 102, `reason` 73 | MATCH (answers/boxes); the reason tests UNSURE (from the module's own notes, not compared here) |
| 28 | Item let go over the ground where the client has no terrain (dungeons, interiors, any surface) | The ground point comes from the camera's line cut at the first ground or surface, so a floor in a building or dungeon works | **[V 1001c024, n3.c 10020571]** | The point is found only against the terrain (`ground_on_line`); with no terrain "nothing is met and nothing happens" | vw 3525-3540; di header | DIFFER |
| 29 | A nano, action, shortcut or perk dragged and let go over the world | `FUN_100cbcb5` returns at once unless the mime is `inventory/item` (FUN_1003d6f2); no other world handler found for drags, and `HandleMouseUp` cancels the drag | **[V 100cbcb5, 1003d6f2, 10157f13]**; **[NF]** another handler (not searched exhaustively in the view classes) | Not read for nanos/actions/hotbar drags (nanoprograms.rs 443-464 handles a release; hotbar/shortcutbar not read) | nanoprograms.rs 443 | UNSURE |
| 30 | Nano cast / "use on" target cursor active when clicking in the world | There is no targeting cursor mode in the original world code found: `SetMousePointer` is set only by `CheckObjectUnderMouse`, `FrameProcess` (Shift details) and view-local code (inventory items over windows `FUN_100e1ded`). Nanos and actions work on the current target | **[V index search of every caller of `SetMousePointer`]**; **[NF]** a world "use on" cursor | none either | n/a | MATCH |
| 31 | Pointer over the world: which arrow and which small pictures | See rows 31a-31h | **[V 1001a33f]** | `pointer::world(class, ctrl, shift, double_click)` + `with_mode` | pt 95, 139; vw 3219-3223; net 5517 | |
| 31a | ... own character | Green arrow, no details | **[V 1001a33f]** (`PVar8 = 2`, details -1) | `Class::Own` -> green | pt 98 | MATCH |
| 31b | ... another player | Right button = Trade (7); left = Trade only with `DoubleclickAction`. Arrow: green if in one's team (stat 6 nonzero and equal); else by side (stat 33) and "vulnerable" (target flags stat 224 & 0x4000001, or `N3Msg_HasVulnerableFightMode`): same side + vulnerable purple (4), same side not vulnerable yellow (3), other side + vulnerable red (5), not vulnerable yellow (3) | **[V 1001a33f]** | `Class::Player` -> same table | pt 105-117; net 5530-5546 | MATCH |
| 31c | ... an NPC | Arrow blue (1) when it is friendly (stat 224 has 0x20000000; a null 0x499602d2 counts as attackable) or talks (stat 0 bits 22/23 value 2), else red (5); right = Talk (6) only when it talks; no left detail | **[V 1001a33f]** | `Class::Npc` same; `pointer_class` decides "NPC" by `names[id].1 == false`, not by `N3Msg_IsNpc` (not read) | pt 99-103; net 5526-5530 | MATCH (whether the client's NPC test equals `IsNpc` is UNSURE) |
| 31d | ... Ctrl held over a character | Left detail becomes Attack (0) for any non-own character | **[V 1001a33f]** (`IsCtrlDown` -> `-0x18 = 0`) | `ctrl && Npc/Player` -> Attack | pt 128 | MATCH |
| 31e | ... a thing (item, terminal, door, corpse) | Blue arrow; `Can` (stat 30) read from the live dynel: bit 0 -> PickUp (1) on the right, and on the left with `DoubleclickAction`; else bit 3 -> Operate (2) likewise; else bare blue arrow | **[V 1001a33f]** | `Class::Thing { can }` same; `can` is the **template's** stat 30 (`template_can`), not the dynel's live stat | pt 119-127; net 5547 | MATCH (source of `can` is a small difference) |
| 31f | ... Shift held | `SetMousePointerLeftDetail(LookAt = 3)` for anything but nothing/ground (the early exit for 0x9c47/0x9c52/0 skips it); also `FrameProcess` sets it in some input modes (0x25-0x2c) | **[V 1001a33f, 1001b253]** (modes' meaning **[NR]**) | `shift` -> left = LookAt for any class but `Nothing` | pt 131 | MATCH |
| 31g | ... a thing with `Can` bit 0x100000 | **Hovering selects it:** `AFCM::Send(0x1e, 0x126, id)` every frame the pointer is on it | **[V 1001a33f]** | Not done (no 0x100000 test anywhere in picking/pointer/viewer/net) | n/a | MISSING |
| 31h | ... over the ground / sky | The function returns without setting a pointer (flags `+0x42` etc. set); whether something else then restores the plain arrow is not read | **[V 1001a33f]**, restorer **[NR]** | `Class::Nothing` -> standard arrow | pt 96 | UNSURE |
| 32 | Mouse look started at press or at first move; cursor | Started at the first movement (`FUN_1002ccbe` calls `FUN_1002cbde` when pending), with `AFCM::Send(0x12, 0x3d)` (what module 0x12 / message 0x3d does, e.g. hiding the pointer, **[NR]**) | **[V 1002ccbe, 1002cbde]** | Pointer hidden and grabbed at the **press**, even for a click | vw 1573-1590 | UNSURE (cosmetic) |
| 33 | Window loses focus (Alt-Tab) while a button is held | `WM_KILLFOCUS` (exe 00404a66 case 8) -> GlobalSignals `+0x10` -> `FUN_1002cc28` ends the look | **[V 00404a66, 1002d1ae]** (the signal's identity as kill-focus is from the message number 8 in the window procedure) | `Focused` only records `self.focused`; held buttons / look / grab are not cleared | vw 1424-1427 | DIFFER |
| 34 | Dead character | not part of this reading | n/a | Every mouse event is dropped while `INPUT_STOPPED` (DTH-001, noted as the client's) | vw 1428-1450 | ADDITION (client-only; DEVIATIONS DTH-001) |

### 2. Rules to port for every DIFFER / MISSING row

**Row 4 (kind 0x9c52 / 0 are never acted on).** In `picking::left_click` and `right_click`, return None when `under.0` is 0x9c47, 0x9c52 or 0
(FUN_1002cfac tests the reference's type first). Do **not** add the test to `double_click` (FUN_1002ce31 has none). Where: pk 414, 426.
SMALL (minutes). Keep 0x9c52 table dynels in the list (the original's GetNextTarget can still return them; only the guard on `under`
exists).

**Row 13 (double click on a thing: get versus use).** `N3Msg_DefaultActionOnDynel` (1002939b) for a non-character: read the **live
dynel's** `Can` (stat 30): bit 0 -> `N3Msg_GetItem` (send `ClientGetItemIIR_t`, `n3.rs` 1182, as the Pick-Up action tile does, with the
inventory-full feedback), else bit 3 -> `N3Msg_UseItem` (`use_object`), else nothing; a right click stays a plain use. Change
`picking::uses` to return an enum (Get / Use / None) and have `net.rs` 11247 send the pick-up for Get. SMALL to MEDIUM (the pick-up message
exists; the inventory-full check is the Pick-Up tile's at net 11091). Needs a check that a pick-up of a world item by double click is
what the server expects (the tile's capture is the evidence).

**Row 12 (default action on a character).** Needs identification of `SimpleChar_t +0x138` (bit 4 = 0x10 is the PvP disgrace flag per
TGT-004's notes), `+0x21c`, stat 0x300 (bit 0) and flag 0x200000, then port the tree in the row in order: own id -> stop; target flags
0x8000000 -> stop; own stat 0x296 -> `Feedback_NotInVehicle`; the `+0x138`/0x300/0x21c branches to IIR / `TradeStart` / `TradeAbort` /
`UseItem`. Client function: the character branch of `net.rs` 11247-11300. LARGE and **needs a capture** of right-clicking an NPC with a
dialog, a monster, a player and a corpse in the original client.

**Row 15 (double-click timing).** Use the Windows values: `GetDoubleClickTime()` and `GetSystemMetrics(SM_CXDOUBLECLK / SM_CYDOUBLECLK)`
(half the rectangle each side) in `Pointer::second_press`; keep 500 ms / 4 px as the fallback off Windows. Both buttons of the original use
the class style, so a different button between two presses ends the pair (the client counts the left only: also reset `pressed` on a
right press). SMALL.

**Row 20 (releases over windows).** Original: the world gets a release (and so may click) only when no window is under the pointer at the
release; the press's place does not matter. In `vw` 1457-1475 do not let a release over an egui area reach the click: keep the look-ending
part (always end the look on a release) but compute `clicked` only if `!ui.is_pointer_over_egui()` (the same test the press path uses),
and drop the "button went down in the world" requirement for a click (a left or right release over the world with no drag object and
travel < 0.02 clicks even if the press was on a window). SMALL. Check the case of the first release after closing a window by a click:
the original has the same behaviour.

**Row 28 (drop on the ground without terrain).** The original cuts the mouse line at the first ground or surface of the playfield
(`SetMousePos`, FUN_1001fe47) and uses that point. The client's picking line already does this for the pointer (vw 3501-3506: `zones.line` +
terrain); the item-let-go path (vw 3525-3540) should reuse that cut point instead of `ground_on_line` on the terrain only. SMALL; but the
drop message (`DropTemplateIIR_t`) was never captured, so what the server does with an interior drop is unknown.

**Row 31g (hover selects `Can` 0x100000 things).** Each frame the reference is a thing whose live stat 30 has 0x100000 set (and the
pointer-mode 0x39 test passes), send the select as a left click does (`AFCM 0x1e/0x126` -> `SetTarget(id, false)`, which needs
`N3Msg_isIDOnGround`). Where: the per-frame pointer code at vw 3217-3225 and a new `keys.pick` with `Deed::Select`, or `net.rs` 11192.
SMALL once `Class::Thing` carries the dynel's live stat 30 (today `template_can`, the template's). Which objects carry 0x100000 is not known
(**[NR]**: look through the game's templates for stat 30 & 0x100000 before deciding whether it matters).

**Row 33 (focus lost).** On `WindowEvent::Focused(false)` end the look as `FUN_1002cc28` does: clear `buttons`, `look_right`,
`keys.looking`/`face`, show and release the cursor, and send the keys. SMALL (vw 1424).

### 3. Settled points and checks of the earlier notes

- **Open point settled:** the double-click time is Windows' own (class style `CS_DBLCLKS` = 8 at exe 00405d74; messages 0x203/0x206 mapped
  to event codes 3/7 at exe 00404a66). Neither a preference nor a constant: see row 15.
- **TGT-001** (own character last on the line): its header is stale; its "Done 2026-11-18" is right: `FUN_10020a3c` (n3.c) leaves the own
  character out only when the camera mode (`+0x1ec`) is 0 and otherwise tries it like any dynel (row 3).
- **TGT-003** (cylinders): header stale; the "Done" text (boxes, `BoundingBoxTargeting`) agrees with `FUN_10020a3c`; the visibility test
  `vtable+0x8c` before it is still not ported.
- **TGT-017**: its "Done" text is right except that its "Original" line says "Ctrl **or Alt** on a character switches the fight": the decompile
  tests qualifier bits `0xc` only (Ctrl); Alt is not read anywhere (row 8). It also says the original describes no double click: the double
  click is FUN_1002ce31 (row 14). Right click does not select: confirmed (row 10).
- **CAM-001**: its header ("both buttons walk forward") is obsolete; the original has no such rule (row 21). Left drag looks only with
  `LMBMouseLook`, right drag turns the character: confirmed (rows 16, 17).
- The notes' mention of a "confirmed live Shift/Ctrl click by a script": the client's world code reads winit's modifier state, which is real
  (row 9), so the *code* is right; whether the interface library saw the keys is irrelevant to the world's clicks.

### 4. Not read / not found

- **[NR]** `FUN_10048dbd` (`N3Msg_IsDropPossible`'s test) beyond what `dropitem.rs` claims; the text numbers of the error boxes.
- **[NR]** the meaning of `SimpleChar_t +0x138`, `+0x21c`, stat 0x300, state flag 0x200000 and 0x8000000 as used in `DefaultActionOnDynel`
  (only guessed from TGT-004: bit 4 of `+0x138` = PvP disgrace).
- **[NR]** `N3Msg_GetItem` body (summary only), `N3Msg_TradeStart`, `N3Msg_IsNpc`, `N3Msg_HasVulnerableFightMode` (the client's version noted, not compared),
  `InfoViewModule_c::ShowURL` (what `itemid://` / `charid://` show).
- **[NR]** what `AFCM` module 0x12 messages 0x3d and 6 do (look start/end broadcasts); the registration of 0x1e/0x126 to `SetTargetMessage`.
- **[NR]** `InputConfig_t::CheckMode(0x39)` and modes 0x25-0x2c (the pointer gating); `FUN_10020629` (`MouseCursorMode`).
- **[NR]** camera arithmetic after the mouse look reaches N3.dll (`MouseCameraControl` 10021689, the vehicle camera zoom limits); the client's
  camera.rs not compared line by line. `FineCollisionCheck` shapes (box/triangle tests) not re-verified.
- **[NR]** how the original treats drags of nanos, actions, shortcuts and perks released over the world (only `inventory/item` has a world
  handler found); the hotbar/shortcut-bar code of the client not read.
- **[NF]** any reader of the "both buttons" bitmask; any middle-button or X-button binding; any Escape effect on target or attack; any
  targeting cursor for nano or use-on in the world.
