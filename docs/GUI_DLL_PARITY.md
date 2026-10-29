# GUI.dll in the new client: what it holds and where we stand

The original's GUI.dll is the whole interface: a view framework, the XML layout language the skins' `Views/*.xml` are
written in, and one module per game window. There is no drop-in of it (`AnarchyReimagined/dropin/` has none); the
reference is the Ghidra dump `AnarchyReimagined/exports_gui/` (22681 functions, `_vtables.json` 119 classes,
`_strings.json`) and the older C++ reconstruction `AnarchyReimagined/src/gui/` (51 functions). This file lists everything
GUI.dll has and what the new client has for it. Status: **done** (ported from the decompile), **ours** (the client has the
feature, not built the original's way), **part**, **no**.

Inventory taken 2026-10-28 from the vtables (classes), the `*_window` names (windows the menus and `/open` know), the
slash-command strings and `cd_image/gui/Default/Views/`.

## 0. The port, in order

The user's choice (2026-10-28): port GUI.dll's view system itself, then move the windows onto it; the info view and the HTML
text first. Steps: (1) the view tree, layouts and XML loader — done; (2) drawing the tree and routing the mouse through it, inside
the existing window frame — done for the mouse (`gui/host.rs`, `gui/window.rs`; keys and focus still to come), with
`BorderView_c`, `Button_c`, `CheckBox_c`, `BitmapView_c` and a plain-text `TextView_c` (`gui/controls.rs`) and the font
table (`gui/font.rs`); (3) `TextView_c` with `TextRenderer_c` and `HTMLParser_c` — done (`gui/text.rs`: the markup, words, wrapping, sizes,
drawing, links; not yet: text input, selection, the cursor, `<img>` pictures, bold and italic faces, shadows, the scroll
bar's art); (4) the info view
(`InfoView_c`, `Views/InfoView.xml`, `text://` and `itemref://` links) — first part done (`gui/infoview.rs`,
`gui/iteminfo.rs`, `gui/criteria.rs`: the window and its history, item pages from the inventory's Info menu with name,
rarity, quality, requirements, locations, charges, modifiers and description; the weapon page; nano pages from the nano
programs window's Info, `itemid://53019/<id>`; weapons' damage per second; to come: the nano radius, shop prices, character info); then the other XML classes and windows. Skins carry their own `Views/*.xml`
(the user's AOU skins in `70dad3e6/Anarchy Online/Gui`): all of them load (`MAO_SKINS` test).

## 1. The framework

| GUI.dll | What it is | Here | Status |
|---|---|---|---|
| `GuiSystem_c`, `WindowController_c`, `Window`, `WndBorder` | the window manager: windows, frames, z-order, focus, `is_frontmost`/`is_backmost`, docking (`DockTabbedWindow`, `dock_config`), fade groups, `AutofadeWindow` | egui windows; `chrome::window` draws the frame, pin and close | part: no docking, no auto-fade, no `?` button |
| `View`, `LayoutNode`, `HLayoutNode`, `VLayoutNode`, `LayoutSpacer`, `HLayoutSpacer`, `VLayoutSpacer`, `LayoutGrid` | the view tree and its layout (`view_layout`, `layout_borders`, preferred/min/max sizes, `SpaceOut`) | `gui/view.rs`, `gui/layout.rs` | done (not `LayoutGrid`); not yet drawn or given input |
| XML view loader (`XMLObject_c` registry: `View`, `BorderView`, `BitmapView`, `TextView`, `ScrollView`, `TextInputView`, `StringListView`, `BrowserView`, `ItemView`, `ItemIconView`, `ItemSlotView`, ...; `activate_criteria`, `fade_group`) | builds the windows of `Views/*.xml` (27 files) | `gui/xml.rs`, `gui::load_view`: all 27 files load and lay out; classes not ported yet stand in as plain views | part |
| `BorderView_c`, `BitmapView_c`, `PlainSpriteFade_t`, `Sprite_t`, `SpriteList_t` | nine-slices and pictures from the skin | `skin.rs`, `chrome.rs`; `gui/controls.rs` for the XML views | done (the three-slice and single-picture modes of `BorderView_c` only as a check box's button) |
| `Button_c`, `ButtonBase_c`, `BorderButton_c`, `TextButton_c`, `InvisibleButton_t`, `MouseDetectorButton_t` | buttons | `chrome::button`; `gui::controls::Button` (its size from `CalculatePreferredSize` 0x10128396, clicks, toggles) | part (no icons, drag, repeat) |
| `CheckBox_c`, `RadioButton_c`, `RadioButtonGroup_c` | toggles | `chrome::checkbox`; `gui::controls::CheckBox` (its button and label views as `Initialize` builds them) | part (no radio buttons) |
| `Slider_c` | sliders (`gfxid_slider`) | egui's | no |
| `ComboBox_c`, `DropdownMenu_c` | drop-down lists | egui's | no |
| `ScrollBar_c`, `ScrollBarInterface_c`, `ScrollView_c`, `ScrollViewChild_c` | scrolling | `chrome::scroll` | done (the wheel is egui's) |
| `TabView` | tabs | `chrome::tab_view` | done |
| `TextInputView_c`, `TextInputModule_t`, `TextInputDialogForm_c` | text fields, the chat input, a "type a name" dialog | `chrome::text_edit` | part (chat input plain) |
| `TextView_c`, `TextRenderer_c`, `TextLine_t`, `ATextString_t`, `FontSystem_t` | text: wrapping, colours, the bitmap and Verdana fonts | `font.rs`, `gui/text.rs`, `gui::controls::TextView`, `gui/font.rs` | part (display only) |
| `HTMLParser_c` | the text markup in chat, info and NPC text: `<font color>`, `<a href="text://...">`, `itemref://`, `chatcmd://`, `<img src=rdb://>`, `<br>`, `<div align>` | `gui/text.rs` for the XML views' text views | part (no `<img>`; chat lines with markup drawn through it, `gui::controls::markup`, their links opening the info view; no entity decoding, as the original's parser has none) |
| `ListViewBase_c`, `ListViewBaseItem_c`, `MultiListView_c`, `MultiListViewItem_c`, `ScrolledListView_c`, `StringListViewItem_c` | list and grid views: columns, sorting (`grid_sort_column`), icon views (`iconview_flags`, `listview_mode`) | egui lists per window | ours |
| `PopupMenu_c`, `PopupMenuItem_c`, `PopupMenuStringItem_c`, `PopupMenuSliderItem_c`, `PopupMenuSeparatorItem_c` | right-click menus (with sliders and separators) | egui context menus | ours |
| `ToolTip_c` | tool tips | egui's hover text | ours |
| `DragObject_c` | drag and drop of items, nanos, actions onto bars, windows, other characters | inventory and hotbar drag | part |
| `DialogBox_c`, `DialogBoxForm_c`, `DialogBoxView_c`, `DialogButtons_c`, `ProgressWindow_c`, `/messagebox` | message boxes, yes/no questions, progress | the client's own | part |
| `PowerBar_t`, `PowerbarView_c`, `/raid` bars | the health/nano/XP bars and their docks (`CC*BarConfig`) | `vitals.rs` | done for the control center |
| `ViewSelector_c`, `HotSpot_t`, `NewGuiSpot_t`, `ClearRect_t`, `RenderInfoView_t` | view switching, hot spots, debugging | | no |
| `InputConfig_t`, `hotkey_config`, the `KeyBindings` archive | keys and their actions | `settings::ACTIONS` | done (the default keymap); the per-character `Prefs.xml` not read |
| `DistributedValue` prefs (`CharPrefs.xml`, `Prefs.xml`, `*WindowConfig` archives) | every option and window place | `layout.ini`, `settings.ini` | ours (by choice) |
| Escape closing (`esc_*` options: backpacks, chests, corpses, dialogs, info view, inventory, LFT, mission selection, nano, NPC chat, option panel, perk window, planet map, raid, research, shops, tips, tower build menu, trades, tradeskill, vehicle, wear) and `AutoCloseWindows` | which windows Escape closes | `Ui::close_with_escape` (all unpinned) | part (not per window) |
| `TipSystem_t`, `TipViewConfig`, `/tipoftheday` | tips of the day | | no |
| `TimerSystemModule_t` | timers (countdown bars) | | no |
| `SoundOptionsMonitor_c` | applies the sound options | `audio.rs` | ours |
| `BrowserModule_c`, `BrowserView_c`, `BrowserWindow_c`, `LoginBrowserWindow_c` (Awesomium) | the in-game web pages (news, item shop) | | no (out of scope: an embedded browser) |

## 2. The windows

| Window (`*_window` / config) | GUI.dll module or view | Here | Status |
|---|---|---|---|
| Login, character select, activate, delete, limbo (`LoginModule_c`, `LoginWindow_c`, `CharSelectWindow_c`, `CharSelectItem_c`, `CharActivateWindow_c`, `CharDeleteWindow_c`, `LimboWindow_c`, `LoginWorld_c`) | Views `LoginWindow`, `CharacterSelection*`, `CharacterActivate/DeleteWindow`, `LimboWindow` | `launcher.rs` | part: no delete, no activate, no 3D hall |
| Character creation (`CharCreateModule_t`, `BreedScene_t`, `ProfessionScene_t`, `AppearanceScene_t`, `NameScene_t`, `SceneBase_t`) | | none | no |
| Control center (`ControlCenterModule_c`, `control_center_window`) | `ControlCenter.xml`, `ActionMenu/*` | `gui/controlcenter.rs` (the XML, its criteria and `CCMenu`s: `ControlMenu_c`, `CCMenuEntry_c`, the actions, `gui/menu.rs`); `vitals.rs` for the docks | part (the docks laid out from the XML and drawn by `vitals::show_docked`, the strips' NCU/credits and AGG/DEF slider; no mini toolbar, docking into the rollup area, drag of entries) |
| `DistributedValue_c`, `ExpressionParser_c`, `CriteriaMonitor_c` (Utils.dll) and the program's criteria context (anarchyonline.exe FUN_00408268) | the values and `activate_criteria`/`criteria` | `gui/dvalue.rs`, `gui/expression.rs`, `gui/variant.rs`, `gui/stat_names.rs` | done (values from the prefs files; the user's `Prefs.xml` not read; `id:` and `module:` contexts not ported) |
| Targeting (`TargetingModule_t`, `TargetView`, friendly/hostile health bars) | | `ui.rs::target_frame` | part |
| Chat (`ChatGUIModule_c`, `chat_group_window`, `ChatGroupWindowConfig`, `ChatConfig`, `log_window_config`, `/filter`) | | `chatwindow.rs` | part: tabs; no chat config window, no groups per window, no links, no timestamps option |
| Wear / inventory / backpacks / bank (`InventoryGUIModule_c`, `wear_window`, `inventory_window`, `bank_window`, `ItemSlotView`) | | `gear.rs` | part: wear + inventory; no backpack windows, no bank |
| Info view: item, nano, character info (`InfoViewModule_c`, `info_window`, `InfoViewConfig`, `InspectViewConfig`) | `InfoView.xml` | `gui/infoview.rs`, `gui/iteminfo.rs`, `gui/criteria.rs` | part (item, weapon, armour, implant and nano pages with DPS; no nano radius, shop price, character info) |
| Skills (`skill_window`, `SkillConfig`) | `Skills.xml` | `skills.rs` | ours (IP raising?) |
| Stats (`stat_window`, `zFStatViewConfig`) | | `stats.rs` | ours |
| NCU (`ncu_window`, `zFNCUWindowConfig`), target NCU (`target_ncu_window`, `FightTargetNCUWindowConfig`, `NanoTargetNCUWindowConfig`) | | `ncu.rs` | part: own NCU only |
| Nano programs (`nano_window`, `NanoViewConfig`) | | `nanoprograms.rs` | ours |
| Perks (`PerkWindowModule_c`, `perk_window`, `DPerkWindow`) | | `perks.rs` | ours |
| Special actions (`specialaction_window`) | | `actions.rs` | ours |
| Shortcut bars (`shortcutbar_window`) | | `hotbar.rs` | ours |
| Friends (`friends_window`) | | `friends.rs` | ours |
| Mail (`mail_window`, `MailIconWindowConfig`) | `MailWindow.xml`, `MailMessageWindow.xml` | `mailbox.rs` | ours |
| Missions, mission selection (`mission_window`, `missionselection_window`, `BMissionSelectionViewConfig`) | | `mission.rs` | part |
| Shops, NPC trade (`ShopViewConfig`, `itemshop_window`, `ItemShopWindowConfig`) | | `shop.rs` | part (the item shop is a web page) |
| Player trade (`TradeViewConfig`) | `TradeGUI.xml` | none | no |
| Tradeskill (`tradeskill_window`, `TradeSkillViewConfig`) | `TradeskillPanel.xml` | none | no |
| Team (`TeamViewModule_c`, `TeamViewConfig`, `/team`, `/invite`, `/kick`, `/leave`) | | none | no |
| Raid (`RaidViewModule_c`, `raid_window`, `RaidConfig`, `RaidBarConfig`) | `Raid.xml`, `RaidBar.xml` | none | no |
| Looking for team (`lft_window`, `LFTWindowConfig`) | `LFTView.xml` | none | no |
| Pets (`PetWindowModule_c`, `pet_window`, `PetMenu.xml`, `/petduel`) | | none | no |
| Organization (`OrganizationGUIModule_c`, `OrgNameView`, upkeep views) | | none | no |
| City terminal (`CityTerminalGUIModule_c`, `CityTerminalWindowConfig`, `CityTerminalBuyDlgConfig`, `CityMapView`) | `CityTerminal.xml` | none | no |
| Market (`MarketWindowConfig`, `OpenMarketSendWindow`) | `MarketSendWindow.xml` | none | no |
| Knowledge (`knowledge_window`, `KnowledgeViewConfig`) | | none | no |
| Factions (`faction_window`, `zFFactionWindowConfig`) | | none | no |
| Research (`research_window`, `ResearchConfig`) | `Research*.xml` | none | no |
| Planet map window (`planetmap_window`, `PlanetMapViewConfig`, `PFMapWindowConfig`, `map_window`) | | `automap.rs`: corner map and overlay | part |
| Compass (`compass_window`) | | in the control center | done |
| Notification window (`NotificationWindowModule_c`, `notification_window`) | | none | no |
| Petitions (`petition_window`, `PetitionWindowConfig`, `/petition`) | | none | no |
| Reclaim (`reclaim_window`, `/reclaim`) | | none | no |
| Daily login / check rewards (`dailylogin_window`, `checkrewards_window`) | | none | no |
| Tips (`tip_window`) | `TipView.xml` | none | no |
| Options (`OptionPanelModule_c`, `optionpanel_window`) | `OptionPanel/*` | the client's own options | ours |
| Help (`help` / info about this window) | | `help.rs` | ours |
| Notum Wars: tower build menu, laser target, airstrike, battlestation, signup terminal, mech select, teleport target | `LaserTarget.xml`, `SignupTerminal.xml`, `MechSelect.xml`, `TeleportTarget.xml` | none | no |
| Whom-pah (in GUI.dll's NPC/terminal handling) | | `whompah.rs` | ours |
| NPC chat (`NPCChatWindowConfig`) | | NPC dialogue in `ui.rs` | part (no markup) |
| Character viewer (`CharacterViewerModule_c`, `CharacterViewer_c`) | | the inventory's figure | part |
| Progress dialog (`ProgressDialog.xml`) | | | no |

## 3. The slash commands GUI.dll registers

Carried out here: `/tell` `/reply` `/whisper` `/shout` `/emote` and the emotes, `/voice`, `/camp`, `/follow`, `/inspect`,
`/open`, `/close`, `/items`, `/waypoint`, `/selectself`, `/setoption`, `/chardist`, `/viewdist`, `/quit`, `/help`,
`/version`, `/ignore` (and the client's `/befriend`).

Listed but not carried out: `/anon` `/assist` `/bank` `/born` `/duel` `/filter` `/group` `/invite` `/kick` `/leave`
`/list` `/macro` `/name` `/option` `/perk` `/perks` `/petduel` `/petition` `/played` `/raid` `/reclaim` `/script`
`/shop` `/showfile` `/stuck` `/team` `/terminate` `/text` `/tipoftheday` `/title` `/toggle` `/tower` `/dvalue`
`/fxscript` `/messagebox`.

Developer and GM commands (not for players; leave out): `/anim` `/center` `/clearunique` `/clone` `/command`
`/criterialocal` `/damagemult` `/dumphash` `/font` `/framerate` `/funcom` `/getfull` `/gethash` `/getlocal`
`/getlocalfull` `/item` `/joycamacc` `/lazyreload` `/monster` `/monsterdata` `/reload` `/reloadgfxtweak` `/resetskill`
`/section` `/setlocal` `/spawn*` `/spelllocal` `/start` `/syncdisplay` `/teleport*` `/togglegroundlightingfix`
`/tplocal` `/weather`.

## 4. What blocks what

- Windows that need only the database and data we already decode: the info view (items, nanos), tool tips that are info
  views, the HTML text, character creation's scenes, tips of the day, the planet map window, knowledge/factions (stats).
- Windows that need messages we have not captured: team, raid, LFT, player trade, tradeskill, bank, backpacks, pets,
  organization, city terminal, market, research, petitions, reclaim, daily rewards, the Notum Wars windows, target NCU.
  Each needs a capture of the original doing it (`tools/capture_proxy.py`).
- Out of scope: the embedded web browser (Awesomium) and the item shop pages.
