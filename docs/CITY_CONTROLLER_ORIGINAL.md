# The organisation city controller in the original client

Sources: Ghidra decompiles of GUI.dll (`U:`), Gamecode.dll (`G:`), city.dll (`C:`), N3.dll (`N:`), checked against
disassembly where Ghidra's arguments were unclear. All numbers on the wire are big-endian (`BinaryStream`).
"Certain" below means read from code; guesses are marked **guess** and collected again in section 5.

Short version:

- The controller is a `CityTerminal_t`, a plain `SimpleItem_t`. It is used like any other terminal: `GenericCmd` Use.
- The server answers with `AOTransportSignalIIR_c` code 5, which opens GUI.dll's `CityTerminalWindow_c`
  (`Views/CityTerminal.xml`).
- The window's buttons send five small request messages (buy, build, demolish, toggle cloak, close session) and one
  org command (list advantages).
- There is **no "start raid" control and no access/guest list anywhere in this window**. The only raid-related control
  is the cloaking device toggle.
- Demolish, cloak toggle and buy are each **one click with no confirmation dialog** in the client.

---

## 1. Targeting and using it

### What it is (certain)

| Fact | Where |
|---|---|
| Identity type 0xc418 (50200) is created by `CityTerminalRibosome_t` as a `CityTerminal_t` (0x1d0 bytes) | ribosome ctor `G:0x1012ff1b`, create `G:0x1012ff36`, class ctor `G:0x1012fe5a` |
| `CityTerminal_t` is a `SimpleItem_t` (ctor calls the SimpleItem ctor `G:0x1008985d`) | `G:0x1012fe5a` |
| Its item template is 249767 (0x3cfa7), named "City Controller", header type 0xc418 | RDB record read directly |
| Template stats: `Can` (30) = **8** (Use only), `Flags` (0) = 0x80023621, mesh (12) = 9013 (the playfield table entry overrides it with 251929) | RDB record |

Vtable compared slot by slot with `SimpleItem_t` (main vtables `G:0x1017201c` vs `G:0x10164b8c`). The only overrides
besides destructors:

| Slot | SimpleItem | CityTerminal | Meaning |
|---|---|---|---|
| main +0x50 | `G:0x10088592` returns 2 | `G:0x1012fe51` returns 2 | same |
| main +0x58 | `G:0x1008853d` (after a use: count down charges, stat 0x1a) | `G:0x1012fe4e` empty | a use changes nothing locally |
| main +0x6c | `G:0x100807e7` (hand-over/trade stats) | `G:0x1012fe57` empty | never traded |
| dynel part +0x88 | `G:0x10089c93` builds `SimpleItemFullUpdateIIR_t` | `G:0x1012fe4b` returns 0 | no full update: it is a static table dynel |

So selecting, targeting and drawing are exactly those of any world `SimpleItem_t`. `N3Msg_SelectedTarget`
(`G:0x10017790`) has no type test.

### The use path (certain)

1. Default action (double-click path), `n3EngineClientAnarchy_t::N3Msg_DefaultActionOnDynel` `G:0x1002939b`: for a
   non-character it reads the target's `Can` stat (0x1e). Bit 1 → pick up (`N3Msg_GetItem`). Bit 8 → `N3Msg_UseItem(id, false)`.
   Otherwise nothing. The controller has `Can` = 8, so it is used.
2. `N3Msg_UseItem` `G:0x10028883`: the type is none of the special ones (0xdead, 0x69, 0x6e, 0x6a, 0xdeae, 50000, 0xdac3,
   0xdeaa), it is not in the character's inventory (`G:0x10048ca7`), `Can` has no 0x10 bit (that bit raises a
   confirm-use signal instead of sending), and the mesh has no advertising action. It ends at
   `ItemActionData_t(target)` (`G:0x1003ac22`) in a `GenericCmd_t` with action 3 (`G:0x1007cea5`), queued through the
   engine (`+0x2c`, `EnqueueSelfMessage`).
3. On the wire that is **`GenericCmd_t` 0x52526858, action 3 (Use) on `(0xc418, instance)`**: the message
   `mao_net::n3::use_object` already builds. Nothing special for this type.

In an instanced playfield the dynel's instance is remapped (`GetRemappedDynelID` in `LoadPlayfieldCityData`
`G:0x101226c9`). Use the identity of the dynel as it exists in the world, not the RDB one.

### What comes back (client side certain; what the server chooses to send is not visible in the client)

`AOTransportSignalIIR_c` (0x62741e15) code 5, "open GUI session". Path:

- reader `G:0x1012faab`, handler `G:0x1012fbc1` → registry `G:0x1012fdaa` → payload reader `G:0x1012e0b8`
- → `PlayfieldCityHolderClient_c::SlotOpenGUISession` `C:0x10016a36`
- → `CityClientInterface_c::SlotOpenGUISession` `C:0x1000a35d` (**only if the interface's lease mode is 0**, i.e. no session open)
- → the holder's signal → `CityTerminalGUIModule_c::SlotOpenCityTerminal` `U:0x100e6c24`.

`C:0x1000a35d` compares the message's `CityID_t` with its own (`C:0x1000a59f`, all five words). On a match it stores GUI
type (+0x34), lease mode (+0x38) and org name (+0x3c), and calls its virtual +4 = `AOCityClientInterface_c`
`G:0x1012be1d`, which **registers the receivers for codes 6, 7, 10, 13, 14, 15, 16**. They are unregistered again by
virtual +8 (`G:0x1012bd22`) when the session closes. So the upkeep, cloak, charge and org-name updates are only heard
between a code 5 and a code 7.

---

## 2. The window

One window class, `CityTerminalWindow_c`, built from `Views/CityTerminal.xml`. No other GUI.dll window belongs to the
controller. (`Raid.xml`/`RaidBar.xml` are the team-raid window; `ControlCenter.xml` is unrelated.)

### Module

| Function | Address | Does |
|---|---|---|
| `CityTerminalGUIModule_c` ctor | `U:0x100e6d47` | connects `SlotPlayerCharacterAlive` and `SlotGUIShuttingDown` to global signals |
| `SlotPlayerCharacterAlive` | `U:0x100e6d17` | connects `SlotOpenCityTerminal` to the city holder's open-session signal (once) |
| `SlotOpenCityTerminal(cityID, identity, guiType, leaseMode, p5, orgName)` | `U:0x100e6c24` | below |
| `SlotCloseGUISession(cityID)` | `U:0x100e6c03` | closes the window for real (window virtual +0x20) |
| `SlotGUIShuttingDown` | `U:0x100e6bdc` | same |

`SlotOpenCityTerminal`:

- Needs `CityClientInterface_c::HasClientInterface(cityID)` and `guiType` 1 or 2; otherwise nothing happens.
- A window of another GUI type is closed first.
- No window: `new CityTerminalWindow_c(cityID, guiType, leaseMode, p5)` (`U:0x100e7847`, 0xa4 bytes).
- Window already there with the same type: only `leaseMode` is applied (`U:0x100e7193`).
- The `identity` argument and the org name argument are not used here (the name is read back from the interface).

### Window constructor `U:0x100e7847`

- `Window(Rect(100,100,500,500), "", "", 0, 0x1800)`: name and title both empty.
- Fields: +0x74 GUI type, +0x78 lease mode, +0x8c `CityID_t` (5 words), +0xa0 `p5`.
- Connects `U:0x100e7193` to the interface's lease-mode signal (fed by code 6).
- GUI type 1: frame 389 × 239, saved layout `CityTerminalBuyDlgConfig`, tabs built by `U:0x100e7339`.
- GUI type 2: frame 499 × 499, saved layout `CityTerminalWindowConfig`; lease mode 1 → `U:0x100e74ba`, else `U:0x100e755a`.
- Option `esc_cityterminal` true: Escape calls `Window::Close`.
- `MoveToCenter`, `LoadWndConfig`, `Show`.
- Close request (window virtual +0x1c, `U:0x100e7203`): sends the close-session request and **returns false**. The
  window stays until the server's code 7 arrives. Only if the city is unknown does it close at once.
- Destructor `U:0x100e6fdb` saves the layout.

Tab captions and short labels are text looked up **by name** in category 10000 (0x2710); long texts by name in
category 10002 (0x2712). XML labels are the `#Name` kind.

### Tabs by GUI type and lease mode (certain)

| GUI type | Lease mode | Tabs (caption key, cat. 10000) |
|---|---|---|
| 1 (buy) | any | `BuyLandArea` = `BuyLandView_c`; `CityMap` = `HouseBuildView_c` in view mode |
| 2 | 1 | `CityTerminal` = `InspectCityPanel_c` |
| 2 | other | `UpkeepController` = `UpkeepControllerView_c`; `BuildHouses` and `DemolishHouses` = **the same** `HouseBuildView_c` added twice; `Misc` = XML view `CloakingController` |

Lease mode 3 is the only one that enables anything. Tab change (`U:0x100e7169`, GUI type 2 only): tab 1 → build mode 0,
tab 2 → demolish mode 1 (`U:0x100e7f62`).

### `BuyLandView_c` (ctor `U:0x100e6388`, built in code, not XML)

| Control | Text | Handler | Does |
|---|---|---|---|
| text | 10002 `CityTerminal_BuyLandDialogText` fed with `GetPurchaseAreaCost()` and `GetAreaUpkeepCost()` (from the playfield's city record: 500,000,000 and 15,000,000 in 6010) | | |
| button (+0x13c) | 10002 `CityTerminal_BuyLandPurchaseButLabel` | `U:0x100e6225` | `CityAI_c::ClientRequestBuy(character)`. Disabled unless lease mode is 3 (`U:0x100e6208`) |
| button | 10002 `CityTerminal_BuyLandCancelButLabel` | `U:0x100e62ba` | close-session request (or closes the window if the city is unknown) |

### `InspectCityPanel_c` (ctor `U:0x100e893e`, XML view `InspectCityPanel`)

| Child | Filled with |
|---|---|
| `MapViewDock` | a `BuildingMapView_c` in mode 2 (view only) inside a scroll view |
| `ChargeBarDock` | `PowerbarView_c(…, 0xdb, 0xdd, 0, 0, 1, 5, 0)`, value `GetControllerCharge()` (a 0..1 fraction; `SetValue` `U:0x1013f283` clamps), updated by `U:0x100e88ea`. Tooltip is the literal "Controller Charge" |
| `OrgNameView` | `GetOwnerOrgName()`, updated by `U:0x100e88d3` |

### `UpkeepControllerView_c` (ctor `U:0x100e9374`, XML view `UpkeepController`)

| Child | Filled with | Handler |
|---|---|---|
| `InfoTextView` | 10002 `CityTerminal_UpkeepControllerHelpText` | |
| `OrgNameView` | `GetOwnerOrgName()` | `U:0x100e91af` on change |
| `UpkeepDateView` | `strftime("%#x", localtime(GetLocalTimeStampForNextBill()))`: the long local date | `U:0x100e92a4` |
| `UpkeepAmountView` | `String::FormatNumeric(GetCityUpkeepCost())` | `U:0x100e91c6` |
| `ChargeBarDock` | power bar as above, tooltip 10000 `ControllerCharge` | `U:0x100e9242` |
| `DumpAdvantagesButton` | | `U:0x100e9296`: `N3InterfaceModule_t::N3Msg_OrgShowCity()`. Disabled unless lease mode is 3 (`U:0x100e918e`) |

The next-bill stamp is `time() + seconds` taken when code 13 arrives (`C:0x1000a428`).

### `HouseBuildView_c` (ctor `U:0x100e8108(cityID, mode, readOnly)`, XML views `CityMapView` + `CityMapControlView`)

| Child | What | Handler |
|---|---|---|
| `MapViewDock` | `BuildingMapView_c(mode, cityID)` (`U:0x100e5c31`, 0x5f0 bytes) in a scroll view | |
| `InfoTextView` | mode 0: 10002 `CityTerminal_BuildMapInfoText`; mode 1: 10002 `CityTerminal_DemolishInfoText` | `U:0x100e7f62` |
| `ButtonSelector` | page 0 in mode 0 (Rotate, Delete, Construct), page 1 in mode 1 (Demolish) | |
| `RotateButton` | selected pending building turns +90° (mod 360) | `U:0x100e4843` |
| `DeleteButton` | selected pending building is taken off the map; its item is released | `U:0x100e4882` |
| `ConstructButton` | sends one build request per pending building, the HQ first, and drops them from the pending list | `U:0x100e498a` |
| `DemolishButton` | sends the demolish request for the selected existing building, deselects | `U:0x100e48bb` |

Button enabling:

- Construct (`U:0x100e7eb2`): the map's "can construct" flag (see below).
- Rotate and Delete (`U:0x100e7ec9`): a pending building is selected.
- Demolish (`U:0x100e7ec9`): an existing building is selected and it is not already being demolished.

Read-only: `readOnly` = not (lease mode 3 and `p5` ≤ 1). When set, the map runs in mode 2 whatever the tab
(`U:0x100e7e89`). In mode 2 the whole control view is removed. The flag is fixed at construction: a later lease change
does not update it (`U:0x100e7e89` ignores its argument).

### `BuildingMapView_c` (the city map)

Mode (+0x470): 0 build, 1 demolish, 2 view. It is a 128 × 128-tile ARGB4444 texture over the playfield map, redrawn by
`U:0x100e3fe7`:

| What | Colour word |
|---|---|
| buildable zone tiles (the city record's zones) | 0x400f |
| existing building | 0x8880 |
| existing, demolition started | 0x8088 |
| pending, valid | 0x8080 |
| pending, invalid | 0x8800 |
| selected pending valid / invalid | 0x80f0 / 0x8f00 |
| selected existing | 0x8ff0 |

Door and guard points of each template are drawn as markers (`U:0x100e3be3`).

Node (0x20 bytes, `U:0x100e9099`): item identity (+0; `(0,0)` = existing building), template identity (+8), tile x, z
(+0x10), rotation in degrees (+0x18), demolishing flag (+0x1c), valid flag (+0x1d).

- Existing buildings: from `PlayfieldCityHolderClient_c::FillCityHouseDataVector` at construction (`U:0x100e5b50`).
- Kept current by the interface's signals: house added `U:0x100e5b06`, removed `U:0x100e4b98`, demolish started `U:0x100e4748`.
- Two global item signals keep pending nodes in step with the inventory: `U:0x100e46fc` (item got a new identity) and
  `U:0x100e4b49` (item gone → node removed).

Mouse (`U:0x100e56a8` down, `U:0x100e4abf` move):

- Mode 2: drag scrolls the map.
- Mode 0 with a drag object: the drop handler below.
- Otherwise the node under the cursor is selected (`U:0x100e3afe`). In mode 0 a pending node can then be dragged to
  another tile.

Dropping an item (mode 0 only), `U:0x100e56a8`:

1. The drag must be mime `inventory/item` with `item_type` 1 (`U:0x1003d6f2`).
2. `N3Msg_GetSkill(item, 620)` (HouseTemplate) gives the template `(0xf4268, value)`; it must be in the template registry.
   If not: dialog 10000 `OnlyDropHousesOnMap`.
3. `N3Msg_GetSkill(item, 33)` (Side): if set (not 1234567890) and different from the character's own stat 33: dialog
   10002 `CityTerminal_HouseCanOnlyBeBuiltBy` fed with the side's name (category 2005).
4. `U:0x100e4d71` adds the pending node at the tile under the cursor, rotation 0, unless a limit is hit (title
   "Error", button `MsgBox_OK`, text key in category 10000):

| House type | Limit | Text key |
|---|---|---|
| 1 HQ | one | `AlreadyHaveHQhere` |
| 2 market | one; none at all when the playfield's model identity type is 0xc79e | `AlreadyHaveMarket` / `NoMarketPFType` |
| 3 grid | one | `AlreadyHaveGridHere` |
| 4 guard house (`IsGuardHouse` `C:0x1000dfff`) | one | `AlreadyHaveUniqueHouse` |
| 5 uplink (`IsRadarStation` `C:0x1000dfe1`) | one | `AlreadyHaveUplinkHere` |
| 6 cloaking device | one | `AlreadyHaveCloakDev` |
| 8 tax office | one | `AlreadyHaveTaxOfficeHere` |
| 9 unique house (`IsUniqueHouse` `C:0x1000e009`) | one per template (`U:0x100e398f`) | `AlreadyHaveUniqueHouse` |

   Type numbers are city.dll's (`IsHQ` 1, `IsMarket` 2, `IsGrid` 3, `IsTaxOffice` 8). Counts include existing and
   pending buildings (+0x4bc + 4 × type). The item is marked in use (`U:0x1003d103`).

"Can construct" (`U:0x100e44a2`) needs all of: at least one pending node; no two nodes' layouts intersect
(`HouseLayout_c::Intersects`); the HQ count is not zero (an HQ exists or is pending); every pending node passes
`CityAI_c::CanBuildLayout` (`C:0x10002882`: every tile of the rotated layout lies on an allowed zone tile).

### Cloak tab (`U:0x100e755a`, XML view `CloakingController`)

`EnableCloakingDeviceButton` (+0x88), made a toggle button.

- State (`U:0x100e70c5`): toggled = `GetCloakingState() > 0`; enabled = lease mode 3 **and** `IsCloakingTogglePossible()`.
- Refreshed on every cloak-state signal (code 14).
- Click (`U:0x100e72a2`): `CityAI_c::ClientRequestToggleCloakingDevice(character)`. No dialog.

### Getters used

`N3InterfaceModule_t`: only `N3Msg_GetSkill` (item stats 620 and 33, own stat 33) and `N3Msg_OrgShowCity`. Everything
else comes from city.dll's `CityClientInterface_c`:

| Getter | Address | Offset | Fed by |
|---|---|---|---|
| `GetGUIType` | `C:0x1000a210` | +0x34 | code 5 |
| `GetLeaseMode` | `C:0x1000a201` | +0x38 | code 5, code 6; 0 after close |
| `GetOwnerOrgName` | `C:0x1000a28e` | +0x3c | code 5, code 16 |
| `GetLocalTimeStampForNextBill` | `C:0x1000a256` | +0x58 | code 13 |
| `GetCloakingState` | `C:0x1000a270` | +0x5c | code 14 |
| `IsCloakingTogglePossible` | `C:0x1000a27f` | +0x60 | code 14: `state != 0 && time < 0` (`C:0x1000a46c`) |
| `GetControllerCharge` | `C:0x1000a29d` | +0x68 | code 15 |
| `GetCityUpkeepCost` | `C:0x1000a247` | +0x6c | code 10 |
| `GetPurchaseAreaCost`, `GetAreaUpkeepCost` | `C:0x1000a21f`, `C:0x1000a233` | | the playfield's city record |

---

## 3. The messages

Every message is `[u32 number][Identity: u32 type, u32 instance][u8 lead][body]`. The lead byte is the "to be passed
on" flag (`n3InfoItemRemote_t::Write` `N:0x100098f4`). All five city requests set it to 0, as does the org command.
Requests carry the **character's** identity `(50000, id)` in the header (`InputConfig_t` +0xd8).

`CityID_t` on the wire (`C:0x1001147a` / `C:0x100114a9`), 20 bytes:

| Field | Type | Value (built in `G:0x101226c9`) |
|---|---|---|
| city identity | Identity | `(0xc419, (controller instance >> 16) & 0xffff)`. For 6010: `(0xc419, 0xc000)` |
| playfield instance | i32 | the playfield proxy's instance |
| playfield model | Identity | `n3Playfield_t::GetModelID()` |

The "city instance" in every request is the second word of the city identity (disassembly at `G:0x1012e70c`).

### Server → client: `AOTransportSignalIIR_c` 0x62741e15

Vtable `G:0x10171ef8`: handler `G:0x1012fbc1`, reader `G:0x1012faab`, writer `G:0x1012fa7e`. Body: `i32 code`, then the
payload. Dispatch is by code alone (`G:0x1012fdaa`); the header identity is not looked at.

| Code | Payload | Reader | Lands in | GUI uses |
|---|---|---|---|---|
| 5 open session | `CityID_t`, `Identity`, `i32 guiType`, `i32 leaseMode`, `i32 p5`, string (u16 length + bytes) org name | `G:0x1012e0b8` | `C:0x10016a36` → `C:0x1000a35d`, `U:0x100e6c24` | all but the Identity |
| 6 lease change | `i32 leaseMode` | `G:0x1012cfdb` | `SlotUpgradeLease` `C:0x1000a3b6` | `U:0x100e7193` |
| 7 close session | `CityID_t` | `G:0x1012d0b1` | `SlotCloseGUISession` `C:0x1000a3db` (lease mode := 0, receivers dropped) | window closes |
| 10 upkeep cost | `i32` | `G:0x1012cfdb` | `C:0x1000a406` | `UpkeepAmountView` |
| 13 seconds till bill | `i32` | `G:0x1012cfdb` | `C:0x1000a428` | `UpkeepDateView` |
| 14 cloak state | `i32 state`, `i32 time` | `G:0x1012d196` | `C:0x1000a46c` | cloak button |
| 15 charge | `f32` (0..1) | `G:0x1012d287` | `C:0x1000a4e4` | charge bars |
| 16 org name | string (u16 length + bytes) | `G:0x1012d364` | `C:0x1000a4bd` | `OrgNameView` |
| 2 / 8 / 9 | house added / demolish started / house removed (see `CITIES_ORIGINAL.md`) | | | map nodes |

Notes:

- Code 5 has **three** ints after the Identity (three `operator>>(int)` calls at `G:0x1012e120`..`0x1012e12a`) and a
  16-bit string length (`G:0x10038af8`). `protocol.json`'s `CityInfo` (two ints, 32-bit string length) does not match
  the client code.
- GUI type: 1 buy dialog, 2 terminal (`TerminalGUIType_e`; the interface starts at 3 = none).
- Lease mode (`LeaseMode_e`): 0 no session, 1 inspect panel only, 3 everything enabled, anything else read-only.
- Cloak state: > 0 on, < 0 off, 0 means toggling is never possible. `time` ≥ 0 blocks the toggle; negative allows it.

### Client → server

| Class | Number | Body after lead byte 0 | Vtable; handler / reader / writer | Ctor; sender |
|---|---|---|---|---|
| `ClientRequestBuyIIR_c` | 0x55796602 | `i32 cityInstance` | `G:0x10172170`; `0x1013018d` / `0x1013015a` / `0x1013017a` | `G:0x10130196`; `G:0x1012e5d6` |
| `ClientRequestBuildIIR_c` | 0x53011416 | `i32 cityInstance`, `Identity template` (0xf4268, n), `i32 tileX`, `i32 tileZ`, `i32 rotation` (0/90/180/270), `Identity item` (the dragged inventory item's `item_id`) | `G:0x10172130`; `0x1013009d` / `0x1012ffff` / `0x10130053` | `G:0x101300a6`; `G:0x1012e62f` |
| `ClientRequestDemolishIIR_c` | 0x3f1b6f70 | `i32 cityInstance`, `i32 tileX`, `i32 tileZ` | `G:0x101721f4`; `0x10130338` / `0x101302e1` / `0x10130312` | `G:0x10130341`; `G:0x1012e695` |
| `ClientRqToggleCloakingIIR_c` | 0x3f5e4b46 | `i32 cityInstance` | `G:0x10172238`; `0x10130406` / `0x101303d3` / `0x101303f3` | `G:0x1013040f`; `G:0x1012e6f2` |
| `ClientRequestCloseGUIIIR_c` | 0x1b3c614d | `i32 cityInstance` | `G:0x101721b0`; `0x1013024d` / `0x1013021a` / `0x1013023a` | `G:0x10130256`; `G:0x1012e74b` |
| `OrgClientIIR_c` command 0x1f | 0x7f4b3108 | `u8 0x1f`, `Identity (0,0)`, `i32 0` | writer `G:0x1012653f` | ctor `G:0x101266b4`; `N3Msg_OrgShowCity` `G:0x1001a8be` |
| `GenericCmd_t` action 3 | 0x52526858 | as `n3::use_object` | `G:0x101629d4` | `G:0x1007cea5` |

The five requests go GUI → `CityAI_c::ClientRequest…` (`C:0x100029e4`..`0x10002a93`) → `AOCityInterfaceClient_c` vtable
`G:0x10171c5c` slots +0x13c..+0x14c → `n3EngineClient_t::SendIIRToServer`. The client-side handlers of the request
classes do nothing.

### The advantages list: `CityAdvantagesIIR_t` 0x365e555b

Vtable `G:0x1015e0f8`: handler `G:0x10038e61`, reader `G:0x10038dad` (→ `G:0x10038cb9`), writer `G:0x10038c65`.
Body: `i32 count` (plain, not the 0x3f1 kind), then per entry `i32 lowId`, `i32 highId`, `i32 ql`, `i32 active`.

The handler acts only when the header identity is the client's own character. Per entry it writes one chat line
`"%i) <a href="itemref://low/high/ql">item name</a>"` formatted into text 1000 `ActiveAdvantage` (active ≠ 0) or
`DisabledAdvantage`. No window. That this message is the server's answer to org command 0x1f is a **guess** (the
button is named `DumpAdvantagesButton`; the reply path is not in the client).

---

## 4. What each action does, as far as the client shows

The server's side is not in any client DLL: `CityAI_c` in city.dll only forwards to the interface, and the client's
interface only sends. So costs and effects below are what the client displays or reacts to, plus marked guesses.

| Action | Client sends | Client-side guard | What the client then expects | Risk |
|---|---|---|---|---|
| Use controller | `GenericCmd` Use | none | code 5 | none: opens a session |
| Close window | close-GUI request | none | code 7, then the window closes | none. A port must send it, or the server may consider the session still open (**guess**) |
| Dump advantages | org command 0x1f | lease mode 3 | chat lines | none: read-only |
| Toggle cloak | toggle-cloak request | lease mode 3, state ≠ 0, time < 0 | code 14 with the new state and a new time | **One click, no confirmation.** The time lock suggests it cannot be undone for a while. That switching it off exposes the city to an alien raid is game knowledge, not in the code (**guess**) |
| Construct | one build request per pending building | lease mode 3, `p5` ≤ 1, layout checks | code 2 (house added) per building; the item leaves the inventory by the normal inventory messages | **Consumes the building item.** No confirmation beyond pressing Construct. Whether a demolished building returns its item is not visible |
| Demolish | demolish request with the tile | lease mode 3, `p5` ≤ 1, existing building selected | code 8 (demolish started; the node turns 0x8088 and cannot be demolished again), later code 9 (removed) | **One click, no confirmation, no cancel message exists.** city.dll has `House_c::UpdateDemolishTime`/`GetDemolisherName` and `HouseInside_c` teleporting visitors out, so it is delayed, but nothing in the client stops it once sent |
| Buy land | buy request | lease mode 3 | not visible (probably code 7 and a new code 5 with GUI type 2: **guess**) | **Costs the purchase price shown in the dialog (500,000,000 in 6010) and starts the upkeep.** One click on the purchase button |
| Rotate / Delete / drop on map | nothing | | | local only |

Things asked about that the client does **not** have here:

- **Start raid**: no button, no message. Only the cloak toggle.
- **Access or guest lists**: nothing in `CityTerminal.xml` or the city classes.
- **Paying upkeep or moving credits**: display only (codes 10 and 13). No pay button.

For a port that must never be tried blind: Use, Close and Dump advantages are safe. The cloak toggle, Construct,
Demolish and Purchase should stay disabled or behind an explicit confirmation of the port's own until a capture shows
the server's replies.

---

## 5. Certain and guessed

Certain (read from code or data):

- Class, template stats and the generic use path of the controller.
- The window, its three layouts, every named child and handler address, the text keys.
- All enabling rules (lease mode 3; `p5` ≤ 1 for the map; cloak state and time).
- The wire layout and numbers of the five requests, the org command, `AOTransportSignal` codes 5, 6, 7, 10, 13, 14,
  15, 16 and `CityAdvantages`.
- The lead byte exists in every message (this settles the "guess (yes)" in `CITIES_ORIGINAL.md` section 3).
- The receivers for codes 6..16 exist only while a session is open.
- No client-side confirmation for cloak, demolish, construct or buy.

Guesses and unknowns:

1. **`p5`** (third int of code 5, window +0xa0): its name is nowhere. Only `p5 ≤ 1` (with lease 3) allows building and
   demolishing. It reads like the character's org rank (0 president, 1 next rank), but that is a guess.
2. **Lease mode names**: only the behaviour of 0, 1, 3 and "other" is known. That 2 means "org member without rights"
   is a guess.
3. **The Identity in code 5**: no client code reads it. `protocol.json` calls it `User`.
4. **Which messages the server sends after a Use, and in what order** (code 5 first, then 10/13/14/15?): not visible.
   Whether a non-member gets lease mode 1 or no reply at all: not visible.
5. **Server effects and costs** of buy, build, demolish, cloak (section 4), including the cloak's lock time, the
   demolish delay, and whether anything is refunded.
6. **`CityAdvantages` as the reply to org command 0x1f.**
7. **`Can`/`Flags` bit names** (Use = 8, ConfirmUse = 0x10, Carry = 1) are from general knowledge of the game's data;
   the code only shows what each bit makes the client do.
8. **Colour format** of the map texture (ARGB4444) is inferred from the values and texture format 10, not read from
   the renderer.
9. Mouse-level mapping (which click reaches `N3Msg_DefaultActionOnDynel` versus `N3Msg_UseItem`) was not re-traced: it
   is the same as for every other terminal.
