# The original client's inventory, wear window and bags — from the decompile, compared with the new client

Read-only analysis, 2026-11-13. Sources: GUI.dll decompile (`exports_gui`), Gamecode.dll decompile (`exports_gamecode`),
disassembly of GUI.dll / Gamecode.dll / Interfaces.dll, `cd_image/gui/Default/LoginPrefs.xml`. New client:
`modern-anarchy-online/crates/mao-client/src/{invwindows,loot,gear,ui,viewer,net}.rs`.

Every statement is marked **[V addr]** (verified in the code at that address) or **[NF]** (not found). Addresses are GUI.dll
unless prefixed `GC` (Gamecode.dll) or `IF` (Interfaces.dll).

## 0. The classes and the wiring

- `ItemListViewBase_c` (ctor FUN_100423fc) is a `MultiListView_c` with five signals and two hold timers **[V 100423fc]**:
  - `+0x2d8` pick-up `(grid pos, item, split count)`
  - `+0x2dc` drop `(grid pos, drag object, button)`
  - `+0x2e0` click `(grid pos, item)`
  - `+0x2e4` qualifier click `(grid pos, item)`
  - `+0x2e8` menu `(item)`
  - timer `+0x320` -> FUN_100420fd -> emits `+0x2e8`; timer `+0x340` -> FUN_1004213e -> emits `+0x2d8` **[V 10042556..10042577, 100420fd, 1004213e]**
  - vtable: MouseDown FUN_10041926, MouseUp FUN_100419d5, MouseMove FUN_10041ca4 **[V vtable 0x101b05b4 +0x3c/+0x40/+0x44]**
- `ItemContainerView_c` (FUN_100ce55d) owns one such list at `+0x158`; `InventoryViewBase_c` (FUN_100ccde7) connects the handlers
  **[V 100cce5d..100ccee8]**:
  - pick-up `+0x2d8` -> FUN_100cc6e8
  - drop `+0x2dc` -> FUN_100cc855
  - click `+0x2e0` -> FUN_100cae1b
  - menu `+0x2e8` -> FUN_100caf07
  - qualifier click `+0x2e4` -> FUN_100ca730
  - the first three only when the view is not read-only (`+0x15c == 0`; read-only is the Inspect window).
- The same `InventoryViewBase_c` is used for: the inventory (container type 0), a container by identity (type 1: bank `0xdead`,
  own backpack, chest `0xc749`, corpse `0xc76a`, `0xdeaa`, reclaim `0xdeae`), a trade list (type 2), and the four wear pages
  (types 5 weapons, 3 clothes, 4 implants, 6 social) **[V 100ce55d, 100ce44c, 100e24b8]**. So **the wear window's cells take
  the same mouse handlers as the inventory**, plus its own drop handler (below).
- List feature flags are `0x15` for all of these (passed by FUN_100ccde7) **[V 100ccde7]**; a corpse's list gets `|0x400`
  **[V 100ce55d]**.
- Qualifier bits (`WindowController_c::GetQualifiers` 0x10156df6): 1|2 = Shift (left/right), 4|8 = Ctrl, 0x10|0x20 = Alt **[V 10156df6]**.

## 1. The mouse on an item

### Which scheme

- `UseNewMouseFunc` defaults to **true**, `MouseButtonHoldDelay` to **0.3 s** (0.1..2.0) **[V LoginPrefs.xml lines 19-20]**.
- But the list only takes the "new" path when its own flag `+0x319` is set, and that flag is set by FUN_10041248 only, which is
  called from exactly five places: KnowledgeView (FUN_100cfa6c), MissionListView (FUN_100d3df7), NanoView (FUN_100d5328),
  ShortcutBarView (FUN_100d9588), SpecialActionView (FUN_100dc9b1) **[V raw call scan for 0x10041248; ctor zeroes +0x318/+0x319 at 100423fc]**.
  **The inventory, wear, bag, bank and corpse lists never set it, so `UseNewMouseFunc` has no effect on them: they always run
  the path the code treats as "old".** (The new client's notes call its click-to-hold "the original's new mouse"; the result
  is right, the reason is not.)

### What the item lists of the inventory / wear / bags / bank / corpses do (flag `+0x319` clear)

| Gesture | What happens | Verified at |
|---|---|---|
| Left button down | nothing (only button 2 is handled on the way down) | FUN_10041aa4 (10041bfa) |
| Left button up, no Shift, on the cell it went down on | pick-up signal -> FUN_100cc6e8 | FUN_1004141b (10041568..10041578) |
| Shift + left button up | qualifier signal -> FUN_100ca730: the item's info (`itemid://%d/%d/%d/%d` to `InfoViewModule_c::ShowURL`) | 100414e8, 100ca730 |
| Ctrl + left button down on a stack (count > 1) | split mode (section 5) | FUN_10041926 |
| Ctrl + left click on a single item | same as a plain left click (the up handler tests Shift only, `& 3`) | 10041539..1004153e |
| Right button down | starts the hold timer (`MouseButtonHoldDelay`); a list with no click listeners (the Inspect window) opens the menu at once | 10041c04..10041c73 |
| Right button up before the timer, on the same cell | click signal -> FUN_100cae1b (use / wear / take / trade) | 10041526..10041535, 100cae1b |
| Right button held 0.3 s | menu signal -> FUN_100caf07 (the context menu, section 6); the click is ended, so letting go does nothing more | 100420fd |
| Right button up on a corpse's list | nothing (flag 0x400) — only the held menu works there | 10041529 |
| Left press, move to another cell, release | nothing: `MultiListView_c::MouseUp` only signals when the up cell is the down cell | 0x10133d04 |
| Double click | no handler exists; it is two clicks: the first picks up, the second drops the item on itself, which cancels the drag | FUN_100cc855 (same id -> Cancel) |
| Alt | no test of bits 0x30 in any of these handlers | [NF] |

The mouse-up handler stops both timers first **[V 100419d5]**.

**Pick-up, FUN_100cc6e8** `(pos, item, split count)` **[V 100cc6e8]**:
- a locked item (`+0xf0`, section 6) does nothing;
- in the inventory (type 0), if the item is an open container (`N3Msg_TemplateIDToDynelID` non-zero): `InventoryGUIModule_c::CloseContainer` first;
- if the view is not "own" (`+0x14c == 0`: corpse, chest, `0xdeaa`): `N3InterfaceModule_t::MoveItemToInventory(item)` — **a left click loots**;
- otherwise (inventory, wear pages, own backpack, bank): an `ItemIconView_c` and a drag object (`inventory/item`, with `container_id`,
  `item_id`, `item_type`, optional `split_count`) and `View::BeginDrag` — **the item is on the pointer** until the next mouse-up.

**Click, FUN_100cae1b** (right click) **[V 100cae1b]**:
- locked item: nothing;
- a trade view is open (`InventoryGUIModule + 0x758`) **and Ctrl is held**: `N3Msg_TradeAddItem(trade partner, item)`;
- else, view not own **or** the bank (`0xdead`): `MoveItemToInventory(item)` (take it out);
- else: `N3Msg_UseItem(item, false)`.

**The pointer's shape, FUN_10041ca4** **[V 10041ca4]**: over an item whose `Can` stat (0x1e) has bit 8 (usable) the right-button
detail of the pointer is set to 2 (not on a corpse); with Shift held and nothing in hand the left-button detail is 3.

### For completeness: the "new" path (lists with `+0x319` set and `UseNewMouseFunc` on — nanos, actions, shortcut bar, not items)

Left down without Shift: pick-up after the hold delay (at once when nobody listens to click); left up: click, or with Shift the
qualifier signal; right up: the menu signal **[V 10041b41..10041bdf, 100414c6..10041503]**.

## 2. Letting a held item go

The drag object lives until a mouse-up. A mouse-up over a list emits its drop signal with the button; over no window at all
`WindowController_c::HandleMouseUp` emits the drag object's own signal and then cancels it **[V 10157f13, 10133d04]**.

### On an inventory / bag / bank cell: FUN_100cc855 `(pos, drag, button)` **[V 100cc855]**

`same` = the drag's `container_id` equals this view's identity; `free` = the list is in icon mode (`GetLayoutMode != 1`).

- Source is the bank and this view is another container (not the inventory): message `ItemsDraggedFromBankCanOnlyBeDroppedToBank`, cancel.
- The cell's item is locked: message `DroppedItemWasLocked`, cancel.
- **Right button** on an occupied cell (no split count):
  - no Shift: `UseItemOnItem(held, target)`, accept (GC N3Msg_UseItemOnItem 0x1002695c = the use message's action 5 **[V GC 1002695c]**);
  - Shift, only in the inventory and both in it: `N3Msg_TradeskillCombine(held, target)` (GC 0x10028707 = `CharacterAction 0x51` **[V GC 10028707]**);
  - otherwise cancel.
- **Left button, same container, no split count:**
  - on itself: cancel (back in place);
  - on another item: `N3Msg_JoinItems(target, held)`; true -> accepted (stacks merged);
  - else, in icon mode: `MultiListView_c::MoveItem(held's cell -> this cell)`, accept, **and if the cell was occupied its item is
    picked up in turn** (FUN_100cc6e8 on it): a swap that leaves the other item in hand (and closes it if it is an open bag);
  - an empty cell: just the move. No message to the server: places are the client's own (section 7).
- **Left button, same container, with a split count:** on an empty cell (or any cell in list mode -> first free place):
  the place is remembered (`+0x150`) and `N3Msg_SplitItem(item, count)`; on an occupied cell in icon mode: cancel.
- **Left button, another container, no split count:** the place is remembered (first free place if the cell is occupied), then
  - this view is a container (`+0x13c != 0`): `N3Msg_ContainerAddItem(this container, item)`;
  - this view is the inventory: `MoveItemToInventory(item)` (IF 0x100081be = `N3Msg_MoveItemToInventory(item, 1, 0x6f)` **[V IF 100081be]**).
- With a split count onto another container: cancel.

`N3Msg_JoinItems` (GC 0x1001bb2a) only merges stacks: same container type, both `Can & 0x200`, same stats 0x2be/0x2bf/0x2bd
(else `Feedback_CanOnlyStackItemsOfSameLevel`), sum <= 50000; sends `CharacterAction 0x35` **[V GC 1001bb2a]**. **There is no
"drop on a bag's icon puts it in the bag"**: on a bag's icon the join fails and the swap rule applies **[V 100cc855: no container test]**.

### On a wear cell: FUN_100e220f (through the thunk at 0x100e245c, connected to each page's `+0x2dc` after FUN_100cc855) **[V 100e2ccf, 100e220f]**

- split count > 0: nothing. Source is the bank: the bank message, cancel.
- Right button on a worn item: `UseItemOnItem(held, worn item)`, accept.
- Left button: slot = the cell's slot (FUN_100ce01d), or when the cell is not a slot the item's own placement (stat 0x58);
  `N3Msg_IsItemPossibleToWear(item, slot, page, true, inventory id)`; false -> **nothing at all: the item stays in hand**;
  true -> `N3Msg_MoveItemToInventory(item, inventory id, slot)` (7 weapons, 3 clothes +0x10, 8 implants +0x20, 0xe social +0x30
  **[V GC 10027bac]**), accept.
- While hovering, FUN_100e1ded (signal `+0x12c`) turns the pointer into the "no" shape over a cell the held item cannot be worn in **[V 100e1ded, 100e2cee]**.
- FUN_100cc855 also runs first and calls `N3Msg_ContainerAddItem({page type, 0}, item)`, which finds no such container list and
  returns **[V GC 10028433: `N3Msg_GetContainerInventoryList` null -> out]**.

### Elsewhere

| Target | What happens | Verified at |
|---|---|---|
| A bag's icon in the inventory | not a target of its own: stack-join fails, the two swap (the bag ends in hand and its window closes) | 100cc855, 100cc6e8 |
| An open bag's window | `N3Msg_ContainerAddItem(bag, item)` at the cell clicked | 100cc855 |
| The bank window | `N3Msg_ContainerAddItem(bank, item)`; out of the bank only to the inventory | 100cc855 |
| Shortcut bar | FUN_100d89bd: an own item (`N3Msg_IsItemMine`) becomes a shortcut there (FUN_100d825b), `Accept(1)`; a shortcut already there goes into hand; nothing sent. Taking a shortcut up is off while `LockHotbars` (FUN_100d8837) | 100d89bd, 100d8837 |
| The 3D world (mouse-up over no window): FUN_100cbcb5 | with a split count: cancel. By what is under the pointer: terrain (identity type 0x9c47) -> `N3Msg_IsDropPossible`; reason 2 -> `N3Msg_DropItem` directly when the pref `ShowDropItemDialog` is 0, else the question `Want2DropItemXonGround` (Yes/No); other reasons -> error boxes (`Feedback_ItemCantBeDroppedAtThatLocation`, `Feedback_ThisIsANoDropItem` with Delete/Cancel, `Feedback_CantDropUnique`, `Feedback_AccessingImplants`). A character (50000) -> `N3Msg_UseItemOnCharacter(item, character)`. Type 0x9c52 or nothing -> nothing. Anything else -> `UseItemOnItem(item, target)` | 100cbcb5 (100cbd39, 100cc68e..100cc6b8) |
| A player (trade) | through the world rule above: `N3Msg_UseItemOnCharacter`. On the open trade window: FUN_100e015f -> `N3Msg_TradeAddItem(partner, item)` (not with a split count) | 100e015f |
| A shop | the shop is the same `TradeView_c` (`SlotStartTrade(TradeType_e, ...)` 0x100c7a40): drop -> `N3Msg_TradeAddItem`; also Ctrl + right click on the item (FUN_100cae1b) | 100e015f, 100cae1b |
| An NPC's chat/trade window | FUN_10057adc -> `N3Msg_NPCChatAddTradeItem` | 10057adc |
| Outside any window that takes it | the drag is cancelled, the item is back in its place | 10157f13 |

## 3. Right click: use, wear or open

The GUI has no rule: for an own item outside the bank it always calls `N3Msg_UseItem(item, false)` **[V 100cae1b]**. The
decision is in Gamecode's `N3Msg_UseItem` (0x10028883) by the item's identity and its `Can` stat **[V GC 10028883, disasm 10028b0c..10028ba1]**:

- identity type 0x69 (bank): `Feedback_ItemCantBeUsedFromBank`; 0x6e: `Feedback_MoveItemToInventory`; 0x6a: `Feedback_ItemsCantBeUsedFromCorpse`.
- an own item whose `Can` (stat 0x1e) **lacks bit 8 (use)**:
  - not worn (identity type not 0x65/0x66/0x67/0x73, FUN_100474a9): **wear it** — page = stat 0x4c, slot = stat 0x58; page 1 weapons,
    2 clothes (+0x10), 3 or 5 implants (+0x20); `N3Msg_IsItemPossibleToWear` first; then the move message (FUN_100153b3);
  - worn: **take it off** (`N3Msg_IsItemPossibleToUnWear`; `Feedback_InventoryFull` when no place) into the first free place (0x6f);
  - then return: no use message.
- an item **with bit 8**: if `Can & 0x10` the confirm signal (GlobalSignals `+0xa0`) is raised instead; else the Use action
  (FUN_1007cea5 kind 3) goes to the server.
- Before that, Interfaces.dll's wrapper (0x1000a73f) raises a confirm signal (GlobalSignals `+0x170`) for items with stat 0x184
  set or shield-disabler items **[V IF 1000a73f]**.

So: armour, weapons and implants are worn / taken off by the right click, decided in the client; consumables and bags are *used*
(a bag's use is what opens it).

## 4. Bags

- **Opening.** Right click -> `N3Msg_UseItem` -> Use action to the server. The server's answer opens the chest object in Gamecode
  (the inventory message handler FUN_100a0957 with its open flag, or the item action 100 in FUN_10087f63 -> `Chest_t` vtable `+0xac`
  = FUN_1007e668), which registers it as open (FUN_1003f81c) and raises GlobalSignals `+0x84` **[V GC 100a0957, 10087f63, 1007e668, 1003f81c, 100117bd]**.
  `InventoryGUIModule_c::SlotContainerOpened` (0x100c7bef) then builds an `InventoryView_c(1, identity)` **unless one for that
  identity is in its map already** **[V 100c7bef]**.
- **How many.** A map keyed by identity, no limit **[V 100c7bef, 100c986e]**.
- **Closing.**
  - The window's close: `InventoryView_c` vtable `+0xfc` = FUN_100ca891: closes the window **and, for a container (type 1), calls
    `N3Msg_UseItem(container, false)` again** — the second use is how the server is told **[V vtable 0x101bd8b4 +0xfc, 100ca891]**.
    The identity passed is the view's container identity (`+0x140/+0x144`: e.g. `(0xc749, instance)` for a backpack, `0xdead`
    for the bank), not the bag's inventory place; for `0xdead` Gamecode closes the bank itself (FUN_10046e44) **[V 100ca891, GC 10028883]**.
    What bytes the Use action carries for `(0xc749, instance)` was not traced **[NF]**.
  - Using the bag again from the inventory: the GUI just sends the use again (FUN_100cae1b); no toggle in GUI.dll. The close then
    comes from the server (item action 0x66 -> `Chest_t` `+0xb4` = FUN_1007e6e2 -> GlobalSignals `+0x88` -> `SlotContainerClosed`
    0x100c757b deletes the view) **[V GC 10087f63, 1007e6e2, 1003f61c; GUI 100c757b]**. That the server answers a second use with
    0x66 is the server's doing **[NF in the client]**.
  - Picking the bag up in the inventory (FUN_100cc6e8) or from the shortcut bar (FUN_100d8837): `CloseContainer` (0x100c7548 -> vtable `+0xfc`) **[V]**.
  - A teleport closes all (`SlotTeleportStarted` 0x100c7523) **[V]**.
  - Moving away: Gamecode's FUN_1003f8fc walks the open containers and closes a chest that is not one's own or not reachable,
    a character's more than 3 away, the bank when not at a terminal **[V GC 1003f8fc]**.
  - Escape closes them when `esc_backpacks` / `esc_chests` / `esc_corpses` (all default true; `esc_inventory` false) **[V 100ccefe, LoginPrefs.xml]**.
  - A corpse or a chest that is not one's own closes itself **1 s after it is empty** (timer `+0x1c8` -> vtable `+0xfc`; started at
    opening when empty, or by the list's change signal -> FUN_100ca8d9) **[V 100cd53c..100cd5b0, 100ca8d9, 1004a1b1]**.
- **Window name and place.** `Container_%ux%u` (identity type x instance) for an own backpack, `TempContainer_%u` for others,
  `bank_window`, `reclaim_window`, `inventory_window` **[V strings in 100ccefe]**; the frame is remembered under that name by the
  window system. A default position for a never-seen bag **[NF]** (the view is made with an empty `Rect`).
- **Title.** The container's name (`N3Msg_GetName`), or the saved `container_name`; "Inventory" for the inventory **[V 100cabeb, 100ccefe]**.
- **Rename.** The window's menu (vtable `+0xf8` = FUN_100cbace) has `ListMode`, an `AutoArrange` submenu, and for an own backpack
  `Rename` -> FUN_100cb852: `TextInputDialogForm_c` "Enter backpack name." with three buttons -> FUN_100ca90f -> `SetBackpackName`
  (0x100c8592); saved as `container_name` in the bag's file **[V 100cbace, 100cb852, 100ca90f, 100cac66]**.
- **Size.** `SetMaxItemCount`: inventory 30, own backpack 21, bank 102, `0xdeaa` 100 **[V 100ce55d]**. No `SetViewCellCounts` for
  these lists (only the wear pages have 3 x 5): the grid follows the window, which is resizable; the default places put items
  three to a row for 21 places (`pos = (i % 3, i / 3)`) when the file has no `item_position_map` **[V 100ccefe]**.
- **"Take all".** No such button, text or handler in GUI.dll (strings searched: "take all", "loot all") **[NF]**. A corpse's only
  extras are the menu's `Looter` submenu (section 6) and the self-closing.

## 5. Stacks

- **Split:** Ctrl + left button down on a stack with count > 1 (FUN_10041926) enters split mode: the pointer is held in place,
  a small window shows the number in yellow, **moving the mouse right/left** changes it (x difference added each move), clamped
  to 1..count-1 (FUN_10041ca4). On the button's release the pick-up signal goes out with that count (FUN_100419d5), so the drag
  carries `split_count`; a left click on an **empty cell of the same container** then sends `N3Msg_SplitItem(item, count)`
  (GC 0x1001bd13: `CharacterAction 0x34`; refusals `Feedback_YouCantSplitThisItem` for `Can & 0x1000000`,
  `Feedback_CantSplitAsInventoryIsFull`) **[V 10041926, 10041ca4 (10041cfb..10041d0b), 100419d5, 100cc855, GC 1001bd13]**. No dialog.
- **Merge:** hold one stack, left click the other in the same container -> `N3Msg_JoinItems` (`CharacterAction 0x35`) **[V 100cc855, GC 1001bb2a]**.
- **Count shown:** yellow `%u` on the icon, `%uM` from 10 000 000 up; hidden below 1 **[V 100406a4, strings 0x101a92e4 / 0x101b045c]**.
  In list mode there is a "Count" column **[V 100ce55d]**.

## 6. The item context menu

There **is** one: FUN_100caf07, a `PopupMenu_c`, opened by **holding the right button 0.3 s** on an item (at once in the Inspect
window) **[V 100caf07, 10041c04]**. Entries (text names from the code):

| Entry | When | Does | Verified at |
|---|---|---|---|
| `LookAtItem` | always | the item's info page (`itemid://`) | FUN_100ca5cb |
| `CreateReference` | always | `RefMan_t::AddItemRef` (an item link) | FUN_100ca55c |
| `DeleteItem` | not read-only and not a wear page | question `ReallyDeleteItemX` (Warning, Yes/No) -> `N3InterfaceModule_t::DeleteItem` | FUN_100ca997, FUN_100ca649 |
| `LockItem` / `UnlockItem` | wear pages only (types 3-6), when the item's stat 8 is absent or < 1 | toggles the lock kept in the DValue `EquipmentLocks` | FUN_100ca3ae |
| `Looter` > `Free For All`, `Random Looter`, team members (raid: `Team %d` submenus) | a corpse (`0xc76a`) while in a team | `N3Msg_SetLootAccess` | FUN_100ca7ab |

There is no "Use" entry. A locked item cannot be picked up or clicked, and nothing can be dropped on it **[V 100cc6e8, 100cae1b, 100cc855]**.
The wear window's own menu adds `LabelsOnTop` (a check) and `UnlockAllItems` **[V 100e20d6, 100e1f9b]**.

## 7. Arranging

- `InventoryView_c` loads and saves a file per container: `Inventory.xml`, `Bank.xml`, `Reclaim.xml`, `Containers/Container_%ux%u.xml`
  (per backpack), `TempContainer_%u.xml` for others. It holds `item_position_map` (item id -> grid cell; not saved for temp
  containers or reclaim), `listview_config` (`listview_mode`, `list_sort_column/order`, `grid_sort_column/order`, the columns),
  `scroll_offset`, `container_name` **[V 100ccefe, 100cac66, 100411b8, 10041352]**. **Places are remembered per container.**
- By default an own container is **not** auto arranged (`+0x318` clear): items stay where they are put; a new item takes the
  place remembered from the drop (`+0x150`), else its mapped place, else the first free place **[V 100ce25d]**.
- Containers that are not one's own (except reclaim), `0xdeaa`, and trade lists are sorted by name with auto arrange on
  (FUN_10041258(1)) **[V 100ce55d LAB_100cea8d]**.
- The window menu's `AutoArrange` submenu (FUN_10041637): "Don't Auto Arrange" (own containers only), then "Unsorted" / "By <column>"
  for the columns, then "Sort Ascending"; disabled in list mode **[V 10041637, 100cbace]**.
- `ListMode` switches the grid for rows with columns Icon, Name, Count, Quality **[V 100ce55d (flags 0xb), 100cbace]**.

## 8. The windows themselves

- **Inventory:** `inventory_window`, title "Inventory", one `InventoryViewBase_c`; at most 30 items; both scroll bars in mode 2,
  the vertical one switched between 1 and 2 by vtable `+0xf0` (FUN_100ca6da); resizable grid with no fixed column count; menu
  `ListMode` + `AutoArrange` **[V 100ccefe, 100ccde7, 100ca6da, 100cbace]**. A fixed window size **[NF]**.
- **Credits:** no credits display in `InventoryView_c` / `ItemContainerView_c`; the string `NumCredits` is referenced only from
  FUN_100360d5 and FUN_1003882b (other views, not read) **[V strref]**. So: **[NF]** in the inventory window.
- **Wear window:** `wear_window`, title text `Wear`, a `TabView` of four pages in the order Weapon / `Tab_Clothing` / `Tab_Implants` /
  "Social", each a 3 x 5 grid, icon spacing 11 x 9, background bitmap 0x1b4 (implants 0x1b5); `WearViewConfig` keeps
  `labels_on_top` and `selected_tab`; docks in the `RollupArea`; `esc_wear`; menu `LabelsOnTop`, `UnlockAllItems`. Built with
  another character's identity it is the Inspect window (`inspect_window`, read-only) **[V 100e24b8, 100e20d6]**. The wear window is
  a separate window, not tabs of the inventory.

## 9. Other things a player notices

- **Tooltip** (FUN_10042193): the name in the item's rarity colour (`N3Msg_GetRarityColor`), and the description under it when
  `ItemDescriptionTooltips` (default true) **[V 10042193]**.
- **Over-equipped items:** each own item is given `N3Msg_GetOverEquipLevel` as its field 0xe (FUN_100cded9) **[V]**; how the icon
  is drawn for it **[NF, not read]**.
- **Locked items:** flag from `EquipmentLocks` (FUN_1003d070 -> FUN_1003d3e2) **[V 100cded9]**; the lock's picture **[NF, not read]**.
- **Cooldown text** on icons ("%2dm", "%2ds"; option `IconTimers`) **[V strings in 1003f8c2..1003f9c7]**.
- **Wear slots' pointer:** the "no" pointer over a slot the held item does not fit **[V 100e1ded]**.
- **Sounds:** none found in these handlers **[NF]** (the split mode sends `AFCM::Send(0x12, 0x3d)` / `(0x12, 6)`, meaning not read).

## 10. Where the new client differs

| # | Original | New client | File and function to change |
|---|---|---|---|
| 1 | Right click on a wearable item without the use flag wears it / takes it off, decided in the client (GC 0x10028883) and sent as a **move** | every right click sends the Use message (`n3::use_object`) | `net.rs` the `keys.fire` block (line ~7323); `invwindows.rs` `show` (where `events.uses` is pushed) |
| 2 | A held item clicked on an empty cell moves there; on another item they swap (the other goes into hand); places are kept per container (`item_position_map`), not auto arranged | the inventory packs items in slot order (`pos: None`), a click with an item in hand does nothing (`Click::Nothing`) | `invwindows.rs` `ItemWindows::show` / `events_of` / `entry`; `loot.rs` `LootWindow::show`; `gui/multilist.rs` `GridMode.auto_arrange` |
| 3 | Stacks: Ctrl+press and mouse sideways to pick a count, click an empty cell -> `SplitItem` (0x34); held stack on another -> `JoinItems` (0x35) | neither exists | `invwindows.rs` (`click`, `events_of`), `net.rs` / `mao-net/src/n3.rs` (two new messages) |
| 4 | Held item + right click on another: plain = `UseItemOnItem` (use message, action 5); **Shift** = `TradeskillCombine` (0x51), inventory only | plain right click sends 0x51 (`n3::use_item_on_item`) | `invwindows.rs` `show` (the `events.combines` push); `net.rs` the `keys.combine` block |
| 5 | Held item let go on the ground: drop question / `N3Msg_DropItem`; on a character: `UseItemOnCharacter`; on another object: `UseItemOnItem` | everything picked under the pointer gets `use_item_on`; nothing for the ground | `ui.rs` after `end_frame` (line ~1383); `viewer.rs` the `requests.use_on` block; `net.rs` the `keys.use_on` block |
| 6 | Closing a bag's window, and using an open bag again, send `UseItem(bag)` to the server (FUN_100ca891, FUN_100cae1b) | the second and third use, and the window's close, send nothing | `ui.rs` the `for place in events.uses` loop (line ~1334); `loot.rs` `LootWindow::show` (on `open` going false) |
| 7 | Picking a bag up in the inventory closes its window (`CloseContainer`) | not done | `invwindows.rs` `events_of` (`Click::TakeUp`), `ui.rs` `bag_windows` |
| 8 | Ctrl + **right click** adds the item to the open trade (player trade or shop: `N3Msg_TradeAddItem`) | Ctrl + **double left click**, and only as a shop sale | `invwindows.rs` `click` and the right-click block; `ui.rs` `events.trade_adds` |
| 9 | Menu (right button held 0.3 s): Look At, Create Reference, Delete (not on wear pages), Lock/Unlock (wear pages), Looter (corpse in a team). No "Use". Shift + right click is an ordinary use | held 0.4 s; Look At, Use, Create Reference, Delete everywhere; Shift + right click opens the menu; no locks, no Looter; no menu in bag/bank/corpse windows | `invwindows.rs` `item_menu`, `HOLD`, the Shift branch; `loot.rs` `LootWindow::show` |
| 10 | A wear cell that does not take the held item: nothing happens, the item stays in hand, pointer shows "no"; a click on a page's non-slot cell wears at the item's own placement | the move is asked, a refusal line is printed and the hand is emptied; only labelled cells take a drop | `invwindows.rs` `show` (`drop_on`), `ui.rs` (`put_down` on `letting_go`), `net.rs` `wear_refusal` |
| 11 | Right click on a corpse item does nothing (flag 0x400); left click loots; no "Take all" in GUI.dll | right click takes; a "Take all" button | `loot.rs` `LootWindow::show` (the `secondary_clicked` take; the button) |
| 12 | Press, move, release on another cell does nothing (pick-up only on a click) | a press that moves takes the item up (`ItemEvent::Dragged`) | `invwindows.rs` `click` (clicks == 0), `loot.rs` `container_click` |
| 13 | Out of the bank only into the inventory (`ItemsDraggedFromBankCanOnlyBeDroppedToBank`); into a bag at the cell clicked | bank item let go over a bag window: only the inventory rectangle is tested, no message | `loot.rs` `LootWindow::show` (the `dragging` release) |
| 14 | Window menu: List Mode, Auto Arrange (sort by column), Rename; wear menu: Labels On Top, Unlock All Items | Rename only (by a right click beside the items); no list mode / sort / labels switch | `loot.rs`, `invwindows.rs` |
| 15 | Inventory grid has no fixed column count; default places three to a row | five columns | `invwindows.rs` `build` (`GridView::new(ctx, 30, 5)`) |
| 16 | Tooltip: name in rarity colour + description (`ItemDescriptionTooltips`) | `gear::tooltip` (own layout) in the inventory, plain name/QL/description text in containers | `gear.rs` `tooltip`, `loot.rs` the `tip` closure (not compared line by line) |
| 17 | Count text `%uM` from ten million | plain number | `gui/multilist.rs` (line ~356) |

Things that already agree: a left click takes an item up and the next click puts it down; Shift + click shows the info; a click
in a corpse or chest takes the item; a click in the bank or an own bag takes it up; a right click in the bank or a chest takes
the item out; an emptied corpse/chest closes after one second; any number of bag windows, named `Container_<type>x<instance>`,
21 places, renamable; the hold-the-right-button menu exists; item on a hotbar slot sets a shortcut and the item stays.

Two notes in the new client's own documents that the code does not bear out:
- `invwindows.rs` header and `docs/CLIENT_UI.md`: "This is the original's new mouse ... raise the pick-up signal as the left
  button goes down". The item lists never take that path (flag `+0x319`); they pick up on the left button's **release**
  (0x10041568), whatever `UseNewMouseFunc` says.
- "Ctrl+double click puts it into an open trade (FUN_100cae1b)": FUN_100cae1b is the click signal, which these lists raise for
  the **right** button's release; Ctrl there adds to the trade.
