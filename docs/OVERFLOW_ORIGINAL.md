# The original client's inventory overflow (identity type 0x6e)

Read-only analysis, 2026-11-15. Sources: Ghidra decompiles of Gamecode.dll (`exports_gamecode`) and GUI.dll (`exports_gui`),
disassembly of Gamecode.dll / GUI.dll / Interfaces.dll, string scans of the DLLs and of `cd_image/text/text.mdb`,
`cd_image/gui`. Nothing was run, no source file was changed.

Addresses are Gamecode.dll unless prefixed `GUI` or `IF` (Interfaces.dll). Marks: **[V addr]** = read in the code at that
address; **[G]** = a guess / inference, said so in the sentence.

Short answer: **GUI.dll has no overflow window class and no overflow string at all.** The overflow is one more
"container" for the generic container machinery: Gamecode keeps a 20-place list, registers the identity
`(0x6e, character instance)` as an *open container*, and GUI.dll's ordinary `InventoryView_c` (the bag / corpse window) shows
it, as a container that is **not the player's own** - so a click on an item in it takes the item, exactly like looting.

---

## 1. Gamecode: where the list lives and who touches it

### 1.1 The object

- `SimpleChar_t + 0x1b8` points to the character's inventory holder (built by FUN_10048746 in the `SimpleChar_t` constructor
  FUN_1005d0b1, destroyed by FUN_1004d51b from the destructor FUN_1005f666) **[V 10048746, 1005d0b1, 1004d51b, 1005f666;
  `mov ecx,[ecx+0x1b8]` at 10028982]**.
- Holder fields **[V 10048b12]**: `+0x08` the inventory (identity types 0x68/0x65/0x66/0x67/0x73), `+0x0c` the list of type
  0x6c, **`+0x10` the overflow list (type 0x6e)**, `+0x19c` the list of type 0x6d.
- The list is a `NewInventory_t` (0x2c bytes, ctor FUN_1002b03c): `+0x04` slot array, `+0x14` slot count, `+0x18/+0x1c` its
  identity, `+0x24` flags (bit 1 = items may be put in, bit 2 = items may be taken out, tested in FUN_1004bd28) **[V 1002b03c,
  1004bfce, 1004c0e5]**.
- **Created empty** by FUN_1004b5f1: `NewInventory_t(20 slots, identity (0x6e, character instance), flags 2)` - 20 places,
  "take out only" **[V 1004b5f1: `push 2; lea eax,[ebp-0x1c]; push eax; push ebx(=0x14); call 1002b03c`]**. FUN_1004b5f1 is
  reached through FUN_10058fc4 from the `SimpleCharFullUpdateIIR_t` handler FUN_1007803b, for a character whose flag bit 0 is
  clear (a player) **[V 1007803b, vtable 0x101624a0]**. So every time the character object is made, its overflow starts
  empty.
- Holder constructor zeroes `+0x10`; destructor deletes the list **[V 10048746, 1004d51b]**.

### 1.2 Every function that reads or changes holder `+0x10` (scan of the holder's code 0x10046900..0x1004d960 for `[reg+0x10]`, plus callers)

| Function | What it does with the overflow | Verified at |
|---|---|---|
| FUN_1004b5f1 | makes the empty 20-slot list (above) | 1004b5f1 |
| FUN_100479e7 `(list, handle)` | handle 0x6e: deletes the old list, stores the new one at `+0x10`, sets its identity to `(0x6e, character)`; then **if that identity is not in the open-container set**: adds it (FUN_1003f81c -> GlobalSignals `+0x84` "container opened" with `(identity, false, false)`) and prints `Feedback_InventoryOverflowed`; **if it is already open**: raises GlobalSignals `+0x8c` "container changed" for it. Handle 0x68 does the same for the inventory without the feedback. Other handles: nothing | 100479e7 (disasm 100479f3..10047a59), 1003f27d, 1003f81c, 100117bd, 1001173e |
| FUN_100a0957 | `InventoryUpdateIIR_t` handler (vtable 0x10156f9c `+8`): when the message's dynel is a `SimpleChar_t` (not a `Chest_t`) it calls FUN_100479e7(list, handle) and then raises the "inventory changed" signal (FUN_1001143e(1)) | 100a0957 |
| FUN_100a0859 | the message's reader: `int slot count`, `int flags`, then the entries (FUN_1002a5db: `int n`, `(n / 0x3f1) - 1` entries, each `int slot` + FUN_1002a20b = `short flags, short count, Identity, ACGItem`), then `Identity` of the owner, `int handle`, `int` open flag. **The slot count and the flags of a server-sent overflow list come from the message**, not from the client's 20 / 2 | 100a0859, 1002a5db, 1002a20b |
| FUN_1004d68c, action 0x57 | `TemplateActionIIR_t` handler (FUN_1007aa44 -> FUN_1004d68c): for action **0x57** it adds a template item (the message's low id / high id / quality, and its count as the entry's short) at **the first free overflow slot from 0** (FUN_1002a371(0) on `[esi+0x10]`, FUN_1002aa8a), then calls FUN_1001143e(0xd) (which has no case 0xd: nothing) and raises GlobalSignals `+0x8c` for `(0x6e, character)`. **It does not open the window and prints nothing.** The message's two identities are not used for the slot (the second one is only stored in the object at `SimpleChar + 0x1e4`, `+0x14/+0x18`) | 1004d68c (disasm 1004d70a..1004d756), 1007aa44, 1007a96d, 1001143e |
| FUN_10047b5c | an item *object* (`SimpleItem_t`) that arrives with a character as its parent (FUN_100a176d): when its stat 0xdc (placement) is 0x6f: **if its stat 0x37 is 0x71 it is put into the overflow** at the first free slot from 0, otherwise into the inventory at the first free slot from 0x40; then the "inventory changed" signal. (That stat 0x37 is the item's inventory id and what 0x71 stands for is a guess **[G]**; the comparison itself is at 10047b8f: `add eax,-0x64; cmp eax,0xd`.) | 10047b5c (disasm), 100a176d |
| FUN_10047837 | **overflow -> inventory**, on the server's `ContainerAddItemIIR_t` (below): takes the item in overflow slot = the source identity's instance, puts it into the inventory at **the first free slot from 0x40** (FUN_1002a371(0x40) on `[esi+8]`), empties the overflow slot, raises `+0x8c` for `(0x6e, character)` and the "inventory changed" signal. **If the inventory has no free slot from 0x40 up it does nothing at all** (no text, the item stays in the overflow list) | 10047837 (disasm 10047846..10047965) |
| FUN_10049523 | **something -> overflow**, on the server's `ContainerAddItemIIR_t` with a *target* of type 0x6e: moves the source item into the first free overflow slot from 0 and raises only the *source* container's change signal (FUN_100493eb(source)). No open, no text | 10049523 |
| FUN_1004796c | builds the list the GUI shows: one entry per occupied slot `i`, with the identity **`(0x6e, i)`**. Called by `N3Msg_GetContainerInventoryList` for an identity of type 0x6e | 1004796c, 100175a6 |
| FUN_100479d1 | closes the overflow as a container: FUN_1003f61c on `(0x6e, character)` = remove from the open set and raise GlobalSignals `+0x88` "container closed". Called only by `N3Msg_UseItem` | 100479d1, 1003f61c, 1001185f, 10028982 |
| FUN_10048b12 | generic "which list does this identity name": type 0x6e -> holder `+0x10`. Through it every generic item function (name FUN_10049b34, item object FUN_1004aba5, `N3Msg_IsItemMine` FUN_10048ca7, delete, trade...) can address an overflow item as `(0x6e, slot)` | 10048b12 |
| FUN_100493eb | "this identity's container changed": type 0x6e -> GlobalSignals `+0x8c` with `(0x6e, character)` | 100493eb |
| FUN_1004bd28 | the client's move check, two overflow rules (1.4) | 1004bd28 |
| FUN_1004d51b | deletes the list with the holder | 1004d51b |

`N3Msg_GetName` gives an identity of type 0x6e the text key `Overflow` in text category 0x3ed **[V 10019b76: `if type == 0x6e`
-> `LDBface::GetTextPtr(0x3ed, "Overflow")`]**.

### 1.3 The server messages

- **`InventoryUpdate`** (the user's FUN_100a0957), handle 0x6e: replaces the whole overflow list, **and is the only thing in
  the client that opens the overflow window and prints `Feedback_InventoryOverflowed`** (FUN_1003f81c has four callers:
  FUN_100479e7, the reclaim FUN_100723de, the bank FUN_1007254b, a chest FUN_1007e668) **[V grep of callers]**. Note the
  code prints the feedback whenever the overflow was not open - it does not look whether the list has items **[V 100479e7]**.
- **`TemplateAction` 0x35505644** = `TemplateActionIIR_t`: reader FUN_1007a96d = `int low id, int high id, int quality,
  int count, int action, Identity, Identity`; action 0x57 = add to the overflow list (table above). So the usual
  "item given" sequence the user captured (TemplateAction 0x57, then ContainerAddItem) puts the item into overflow slot 0 and
  takes it out again without the window ever opening **[V 1007a96d, 1004d68c]**.
- **`ContainerAddItem` 0x47537a24** = `ContainerAddItemIIR_t` (vtable 0x1015e174): reader FUN_1003910c reads **source
  identity** (to `+0x20`), **target identity** (to `+0x18`), `int` target slot (to `+0x28`). Handler FUN_1003917d ->
  FUN_1004b23e(target, source, slot), which decides in this order **[V 1003910c, 1003917d, 1004b23e]**:
  1. nothing if the source type is 50000, or (source type <= 0x68 and source slot == target slot);
  2. target type 0xdead -> to the bank (FUN_10049618); target type 0x6c -> FUN_1004af49;
  3. **source type 0x6e -> FUN_10047837: overflow slot -> first free inventory slot from 0x40. The target identity and the
     slot number of the message are not looked at.**
  4. target type 0x6e -> FUN_10049523: source -> first free overflow slot.
  5. the other cases.
- **A timer:** none. No timer or tick function touches holder `+0x10` (the scan in 1.2 is complete for the holder's code), and
  the GUI's only container timer is not wired for this window (2.3). Losing overflow items is the server's doing; the text of
  `Feedback_InventoryOverflowed` itself says (paraphrased) that the items must be taken before leaving the area or they are
  deleted **[V text.mdb; the rule itself is not in the client]**.
- **Zoning / leaving the playfield:** `SimpleChar_t` vtable `+0x4c` = FUN_1005aa4f, for the client's own character, calls
  FUN_1003f65b = close **every** open container (FUN_1003f61c on each, so `+0x88` for the overflow too) **[V 1005aa4f,
  1003f65b; that slot `+0x4c` is the "leaving the playfield" hook is inferred from its neighbour `+0x48` FUN_1005997f taking
  the new playfield - [G]]**. The GUI also closes all container windows on `SlotTeleportStarted` **[V GUI 100c7523]**. The
  list itself is not emptied there; it dies with the character object, and the new object gets a new empty list (1.1).
  Whether the live server sends the overflow again after a zone is not visible in the client **[G: it does not, going by the
  feedback text]**.
- **Logout:** nothing special found; the holder's destructor frees the list **[V 1004d51b]**.
- **Moving away:** the walk-away check FUN_1003f8fc skips the overflow (it is not a chest, a character, 0xdead or 0xdeae), so
  the window stays open while walking **[V 1003f8fc]**.

### 1.4 Client rules while there is an overflow (FUN_1004bd28, the check before any move the client asks for)

- **No wearing while overflow slot 0 is occupied.** When the target is a wear slot (target slot < 0x40) the check calls
  FUN_1002a371(0) on the overflow and requires the answer 0 (slot 0 free); otherwise `Feedback_WearingDisabledWhenItemsInOverflow`
  and reason code 0x34 **[V disasm 1004cab6..1004cadc]**. (It tests "first free slot is slot 0", not "list is empty".)
- **A unique item that the character already has cannot be taken out.** Source type 0x6e and FUN_1004836c true (item flag
  0x8000000 and the same item id already in the inventory `+8` or the list `+0xc`): `Feedback_YouAlreadyHaveUniqueName` with
  the item's name, the overflow's change signal, refusal **[V disasm 1004d071..1004d0d5, 1004836c]**.
- The generic flag tests: source list without bit 2 -> text `Inv_SrcCantRemove`; target list without bit 1 -> `Inv_DstCantAdd`
  (category 0x2713, sent through GlobalSignals `+0x184`) **[V disasm 1004bfce..1004bfe9, 1004c0e5..1004c100]**. With the
  client-made list (flags 2) nothing can be put *into* the overflow by the player.
- `N3Msg_UseItem` on an overflow item `(0x6e, slot)`: only `Feedback_MoveItemToInventory`, nothing sent **[V 10028972..10028998]**.

---

## 2. The GUI side

### 2.1 Is there an overflow window class?

**No.** GUI.dll has no string containing "overflow" except a parser error text, and no code that tests an identity type
0x6e (all 0x6e constants in the GUI decompile are text categories, key codes or struct offsets) **[V string scan of GUI.dll
and Interfaces.dll; `ggrep 0x6e`]**. Nothing under `cd_image/gui` names it **[V grep]**.

The window is `InventoryView_c` built by `InventoryGUIModule_c::SlotContainerOpened` (GUI 0x100c7bef), which is connected to
GlobalSignals `+0x84` and makes `InventoryView_c(1, identity)` unless a view for that identity is in its map already
**[V GUI 100c7bef, 100c7c7b]**.

### 2.2 What that view is for `(0x6e, character)`

- **Not "own".** `N3Msg_IsItemMine((0x6e, character instance))` is false: FUN_10048ca7 finds the overflow list, then asks
  that list for slot number = the character's instance, which is out of range **[V disasm 10048cf3..10048d65; false unless
  the instance number were below the slot count]**. So `ItemContainerView_c + 0x14c` = 0, as for a corpse or someone else's
  chest **[V GUI 100ce55d]**.
- **Window name** `TempContainer_%u.xml` (sic, with the extension; `%u` = the first unused temp number, FUN_100cb7f9); its
  file `<inventory config path>/TempContainer_%u.xml` **[V GUI disasm 100cd2b2..100cd331]**. Places of items are not saved for
  temp containers (see `docs/INVENTORY_ORIGINAL.md` section 7).
- **Title:** `N3Msg_GetName(identity)` = the text `Overflow` of category 0x3ed **[V GUI 100cabeb, GC 10019b76]**.
- **Layout:** the same `DockableView_c` + `InventoryViewBase_c` + item list as a bag: columns Icon / Name / Count / Quality
  (flags 0xb), list features 0x15, both scroll bars mode 2; icon grid by default, list mode from the window's menu
  **[V GUI 100ccde7, 100ce55d]**. Because it is not own and not 0xdeae it is **sorted by name with auto arrange on**
  (`SetListSortColumn(1,0)`, FUN_10041258(1)) **[V GUI 100ce55d LAB_100cea8d]**. No `SetMaxItemCount` call for it (only
  inventory 30, own bag 21, bank 102, 0xdeaa 100) **[V GUI 100ce55d]**; the list's default limit was not read. **Size:** made
  with an empty `Rect`; a default size was not found. How many cells show is the window's size, not a fixed grid; the
  client-side list has 20 slots (1.1).
- **Content:** `N3Msg_GetContainerInventoryList(identity)` -> FUN_1004796c, items named `(0x6e, slot)`; refreshed on
  GlobalSignals `+0x8c` when the identity matches **[V GUI 100ce538, 100ce44c; GC 1004796c]**.

### 2.3 When it opens and closes

- **Opens:** only when Gamecode raises `+0x84` for it = an `InventoryUpdate` with handle 0x6e while it is not open (1.3).
  There is no key, button or menu entry for it in GUI.dll **[V no reference]**.
- **Closes:**
  - the window's close (vtable `+0xfc` = GUI FUN_100ca891): closes the view and calls `N3Msg_UseItem((0x6e, character), false)`;
    Gamecode's `N3Msg_UseItem` sees type 0x6e with instance == the client character and calls FUN_100479d1 (out of the open
    set, `+0x88`), then returns: **nothing is sent to the server** **[V GUI 100ca891; GC disasm 10028972..1002898d]**;
  - `+0x88` -> `SlotContainerClosed` deletes the view **[V GUI 100c757b]**;
  - teleport / leaving the playfield (1.3) **[V GUI 100c7523, GC 1005aa4f]**.
  - **It does not close itself when it is emptied:** the 1 s "empty" timer (`+0x1c8`) is connected only for identity types
    0xc76a and 0xc749 **[V GUI disasm 100cd53c..100cd5b0, 100ca8d9]**.
  - Escape does not close it: the `esc_*` settings are read only for the inventory, 0xc749 and 0xc76a **[V GUI 100ccefe]**.
- Once closed with items still inside, nothing in the client reopens it until the next `InventoryUpdate` 0x6e (TemplateAction
  0x57 and both ContainerAddItem paths only raise `+0x8c`) **[V 1.2; that the live server sends one then is a guess - G]**.

### 2.4 Clicks and drags in it (the generic handlers, see `docs/INVENTORY_ORIGINAL.md` section 1)

| Gesture | What happens | Verified at |
|---|---|---|
| Left click (button up on the item) | pick-up signal -> GUI FUN_100cc6e8: view not own -> `N3InterfaceModule_t::MoveItemToInventory(item)`. **No drag starts: the item is never "in hand".** | GUI 100cc6e8 |
| Right click | click signal -> GUI FUN_100cae1b: view not own -> the same `MoveItemToInventory(item)`. With a trade window open and Ctrl held: `N3Msg_TradeAddItem(partner, item)` instead | GUI 100cae1b |
| Shift + left click | the item's info page | GUI 100ca730 (as for any list) |
| Right button held 0.3 s | the item menu: `LookAtItem`, `CreateReference`, `DeleteItem` (-> `N3Msg_DeleteItem` = `CharacterAction` 0x70 on `(0x6e, slot)`) | GUI 100caf07, GC 1001c864 |
| Dragging out of it | not possible (no drag object is made for a view that is not own) | GUI 100cc6e8 |
| Dropping an item from elsewhere onto it | GUI FUN_100cc855: "another container" -> `N3Msg_ContainerAddItem((0x6e, character), item)`; Gamecode's check FUN_1004bd28 finds the overflow list without flag bit 1 -> `Inv_DstCantAdd`, nothing sent (with the client's own flags 2; a server-sent list has the server's flags) | GUI 100cc855 (line with `+0x13c != 0`), GC 10028433, 1004c0e5 |

Feedback text keys tied to the overflow (all in Gamecode, all through FUN_10059047 = look the key up, raise the chat feedback
signal): `Feedback_InventoryOverflowed` (100479e7), `Feedback_WearingDisabledWhenItemsInOverflow` (1004cac8),
`Feedback_MoveItemToInventory` (10028993), `Feedback_InventoryFull` (10027c31), `Feedback_YouAlreadyHaveUniqueName` (1004d0a7);
and the two `Inv_*` texts above.

---

## 3. Exactly what a click in the overflow window sends

GUI -> `N3InterfaceModule_t::MoveItemToInventory(item)` = IF 0x100081be = `N3Msg_MoveItemToInventory(item, 1, 0x6f)`
**[V IF disasm 100081cb..100081d2: `push 0x6f; push 1; push item`]**, with `item = (0x6e, overflow slot)`.

`N3Msg_MoveItemToInventory` (0x10027bac) **[V]**:

1. `N3Msg_IsItemPossibleToUnWear(item)`: true for type 0x6e (only 0x65/0x66/0x67/0x73 are tested) **[V 100268df]**.
2. inventory id 1 and no free inventory slot from 0x40 (FUN_1002a371(0x40) == -1): `Feedback_InventoryFull`, the overflow's
   change signal (FUN_100493eb), **nothing sent**.
3. FUN_1004bd28(character, item, 0x6f) - the checks of 1.4 (unique item, list flags). Refused: change signal, nothing sent.
4. FUN_10047732(item, 0x6f) is false for a source type >= 0x68 with slot >= 0x30, so no weight bookkeeping.
5. Builds `ClientMoveItemToInventoryIIR_t` (FUN_100153b3) and sends it (`SendIIRToObservers`). **No local move**: the item
   leaves the overflow list only when the server answers with `ContainerAddItem` (source type 0x6e -> FUN_10047837).

The message **[V writer FUN_10015387: `Identity` at `+0x20`, then `int` at `+0x28`; ctor FUN_100153b3 puts the item identity
there and the slot argument]**:

| Field | Value |
|---|---|
| N3 message id | `0x5469373f` (`ClientMoveItemToInventory`; id from the repo's `protocol.json`, not re-derived here) |
| N3 header identity | `(50000, character instance)` |
| (the header's byte) | `00` (as in the repo's `n3::move_to_inventory`) |
| Source identity | `(0x0000006e, overflow slot index)` - the slot is the index in the overflow list (0 for the first item), **not** the character's id |
| Slot `int` | `0x0000006f` ("any free place") |

That is the repo's existing `n3::move_to_inventory(character, (0x6e, slot), 0x6f)`.

There is **no drag from the overflow window**, so there is no second form. (A drop *onto* the inventory window from another
container would be the same call, GUI FUN_100cc855 -> `MoveItemToInventory(item)`, but the overflow never gives a drag object.)

The expected answer, from the client's side: `ContainerAddItem` 0x47537a24 with source `(0x6e, slot)`; the client then puts
the item at its own first free inventory slot from 0x40 and ignores the message's target and slot (1.3).

---

## 4. What is uncertain

- **What the live server sends when the inventory is full** is not in the client. From the code, the only message that
  opens the window and prints the feedback is `InventoryUpdate` with handle 0x6e; that the server sends exactly that (rather
  than, say, only TemplateAction 0x57 with no ContainerAddItem) is a **guess**. With TemplateAction 0x57 alone the original
  client would hold the item in the list invisibly and only block wearing (1.4).
- **The slot count and flags of a server-sent overflow list** come from the message (FUN_100a0859); the 20 slots / flags 2
  are only the client's empty starting list. If the server's flags lack bit 2, taking out would be refused with
  `Inv_SrcCantRemove` - not expected, but not checked against a capture.
- **Stat 0x37 == 0x71** in FUN_10047b5c (an item object sent straight into the overflow): the test is verified, its meaning
  (inventory id of the overflow?) is a **guess**; 0x71 is not the identity type 0x6e.
- **`SimpleChar_t` vtable `+0x4c` (FUN_1005aa4f) = "leaving the playfield"**: inferred, the slot has no exported name.
- **Whether overflow items survive a zone / how long they live**: server rule; the only evidence is the feedback text
  (deleted on leaving the area). No client timer exists.
- **Default size and position of the window**, and the item list's default maximum count: not found / not read.
- **The message id 0x5469373f and the header's zero byte** are taken from the repo's schema and existing code, not derived
  again from Gamecode's registration table.
- The user's note says the server's ContainerAddItem goes "from (0x6e, 0) to (0x6e, character)". The reader's field order is
  source, target, slot (FUN_1003910c), and the handler tests the source type first, so whatever the target is, the result is
  "overflow slot -> inventory" - unless the target type were 0xdead or 0x6c, which are tested before it.
