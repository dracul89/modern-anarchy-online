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
| XML view loader (`XMLObject_c` registry: `View`, `BorderView`, `BitmapView`, `TextView`, `ScrollView`, `TextInputView`, `StringListView`, `BrowserView`, `ItemView`, `ItemIconView`, `ItemSlotView`, ...; `activate_criteria`, `fade_group`) | builds the windows of `Views/*.xml` (27 files) | `gui/xml.rs`, `gui::load_view`: all 27 files load and lay out; every registered class is ported except `StringListView` (no skin uses it), which stands in as a plain view | yes |
| `BorderView_c`, `BitmapView_c`, `PlainSpriteFade_t`, `Sprite_t`, `SpriteList_t` | nine-slices and pictures from the skin | `skin.rs`, `chrome.rs`; `gui/controls.rs` for the XML views | done (the three-slice and single-picture modes of `BorderView_c` only as a check box's button) |
| `Button_c`, `ButtonBase_c`, `BorderButton_c`, `TextButton_c`, `InvisibleButton_t`, `MouseDetectorButton_t` | buttons | `chrome::button`; `gui::controls::Button` (its size from `CalculatePreferredSize` 0x10128396, clicks, toggles) | part (no icons, drag, repeat) |
| `CheckBox_c`, `RadioButton_c`, `RadioButtonGroup_c` | toggles | `chrome::checkbox`; `gui::controls::CheckBox` (its button and label views as `Initialize` builds them) | part (`RadioButton` as a check box with the radio pictures; `RadioButtonGroup_c` in `gui/radio.rs`: one choice among its buttons at any depth, `GetValue`/`SetValue`, `ValueChanged` as `Signal::Value`) |
| `MiniToolbar_c`, `ToolbarButton_c` (XML `CCMiniToolbar`) | the control center's window buttons | `gui/controlcenter.rs` `MiniToolbar` (the ten buttons, their pictures, tips and values) | yes |
| `PowerbarView_c` (XML `PowerBar`) | the limbo/progress/raid bars in the XML windows | `gui/powerbar.rs` (background, end caps, the full picture cut to the value in four directions) | yes (no labels) |
| `Slider_c` | sliders (`gfxid_slider`) | `gui::controls::Slider` (the option slider's pictures) | part (no `gfxid_slider` art choice, no steps) |
| `ComboBox_c`, `DropdownMenu_c` | drop-down lists | `gui/dropdown.rs` (`DropdownMenu_c`: the three-piece box, the widest entry's room, the arrow, the list, `SelectionChanged`) | part (no `ComboBox_c`, the typing kind) |
| `ScrollBar_c`, `ScrollBarInterface_c`, `ScrollView_c`, `ScrollViewChild_c` | scrolling | `chrome::scroll` for the egui windows; `gui/scroll.rs` for the XML views (bar modes, `scroll_client`, the client sized and moved by `LayoutChilds`, clipping, the bar's arrows, page clicks, knob drag and wheel) | done (no horizontal bar drawn; no snapping) |
| `TabView` | tabs | `chrome::tab_view` | done |
| `TextInputView_c`, `TextInputModule_t`, `TextInputDialogForm_c` | text fields, the chat input, a "type a name" dialog | `gui/textinput.rs` (the XML field: inset border, `_editor`, the label at any of its five places), `gui/edit.rs` (`TextRenderer_c::KeyDown`/`TextInput`: typing, Backspace/Delete, Home/End, arrows with Shift selecting and Ctrl by words, Enter, numeric fields, paste and copy), the view tree's keyboard focus (`gui/host.rs`: click, Tab, Escape); `chrome::text_edit` for the egui windows | part (no right-click menu, no IME, no `TextInputDialogForm_c`; chat input plain) |
| `TextView_c`, `TextRenderer_c`, `TextLine_t`, `ATextString_t`, `FontSystem_t` | text: wrapping, colours, the bitmap and Verdana fonts | `font.rs`, `gui/text.rs`, `gui::controls::TextView`, `gui/font.rs` | part (display only) |
| `HTMLParser_c` | the text markup in chat, info and NPC text: `<font color>`, `<a href="text://...">`, `itemref://`, `chatcmd://`, `<img src=rdb://>`, `<br>`, `<div align>` | `gui/text.rs` for the XML views' text views | part (no `<img>`; chat lines with markup drawn through it, `gui::controls::markup`, their links opening the info view; no entity decoding, as the original's parser has none) |
| `ListViewBase_c`, `ListViewBaseItem_c`, `MultiListView_c`, `MultiListViewItem_c`, `ScrolledListView_c`, `StringListViewItem_c` | list and grid views: columns, sorting (`grid_sort_column`), icon views (`iconview_flags`, `listview_mode`) | `gui/multilist.rs`: `MultiListView_c`'s list mode (the column header of `GFX_GUI_PROGRESS01` labels, sorting by a label, resizing and fitting columns, rows 19 apart in a scroll view, the selected row's fill, clicks with their count); the mail inbox uses it; egui lists elsewhere | part: grid mode (icon sizes 0-3, grid labels on top or under, background pictures, cells by position or auto arranged) for the item windows, NCU, nanos, actions and missions; no grid sorting; no column swapping, no horizontal scroll bar, no `listview_config` saved |
| `PopupMenu_c`, `PopupMenuItem_c`, `PopupMenuStringItem_c`, `PopupMenuSliderItem_c`, `PopupMenuSeparatorItem_c` | right-click menus (with sliders and separators) | `gui/popup.rs` (`PopupMenu_c` with string entries: the inset frame over the window background, the stacking, the highlight, mouse and keys) for the XML views' drop-downs; `chrome::menu` shows the same menu for every right-click menu of the client's windows (friends, missions, chat, stats, inventory, NCU, knowledge) | part (check marks drawn; no icons, sub-menus, sliders, separators, type-to-search) |
| `ToolTip_c` | tool tips | `gui/tooltip.rs` (`WindowController_c::UpdateToolTip`'s 500 ms rest, `ToolTip_c` at the pointer with `InfoContainer_c`'s title and word-wrapped body in a `Hover` frame) for the XML views' `tooltip`/`tooltip_body` and the control center's entries; egui's hover text elsewhere | part (the frame's surfaces drawn as lines; not the click-to-keep tip) |
| `DragObject_c` | drag and drop of items, nanos, actions onto bars, windows, other characters | inventory and hotbar drag | part |
| `DialogBox_c`, `DialogBoxForm_c`, `DialogBoxView_c`, `DialogButtons_c`, `ProgressWindow_c`, `/messagebox` | message boxes, yes/no questions, progress | `ProgressWindow_c`: `loginviews.rs` on `ProgressDialog.xml` (logging in, `GeneralNetworkTimeout`); `chrome::dialog` / `chrome::question` (`DialogBox_c` `_Init` 0x1012a7a8, `DialogBoxView_c` 0x1012aed3, `DialogButtons_c::_Initialize` 0x1012b20f: up at the pointer, the text with markup 15 from the sides and wrapped about twice as wide as tall, the buttons 8 apart at the right or the one in the middle, Escape with `esc_dialogs`) for every yes/no and OK box of the client (`chrome::DIALOGS`) | yes (no `DialogBoxForm_c` with a text field, no close timeout, not modal) |
| `PowerBar_t`, `PowerbarView_c`, `/raid` bars | the health/nano/XP bars and their docks (`CC*BarConfig`) | `vitals.rs` | done for the control center |
| `Slider_c` | the research/tradeskill/options sliders | `gui/controls.rs` `Slider` (the AGGDEF pictures, knob placement, knob-only dragging) | yes (no `gfxid_*`) |
| `ComboBox_c` | a text field with a list to pick from (login user names) | `gui/combobox.rs` (the arrow in the field's border, the list under the editor, the chosen text) | yes |
| `TextButton_c` | a button that is only its text (character selection names) | `gui/textbutton.rs` (raised/hover/pressed texts and colours, the disabled colour) | yes |
| `ScrollingTextView_c` | a one-line text that slides to show all of itself (the trade partner's name) | `gui/scrolling.rs` (the delays, the 0.4 step, the turns, flashing) | yes |
| `ViewSelector_c`, `HotSpot_t`, `NewGuiSpot_t`, `ClearRect_t`, `RenderInfoView_t` | view switching, hot spots, debugging | `gui/selector.rs` (`ViewSelector_c`: one view at a time, `use_max_size`, `GetValue`/`SetValue`) | part (`ViewSelector_c` done; no list view link; the others no) |
| `InputConfig_t`, `hotkey_config`, the `KeyBindings` archive | keys and their actions | `settings::ACTIONS` | done (the default keymap); the per-character `Prefs.xml` not read |
| `DistributedValue` prefs (`CharPrefs.xml`, `Prefs.xml`, `*WindowConfig` archives) | every option and window place | `layout.ini` (every window's place as left, `chrome::window` noting each window shown; not the fixed dialogs), `tabs.ini` (the tabs the configs keep: `PerkWindowConfig` `SelectedTab`, `WearViewConfig` / `InspectViewConfig` `selected_tab`), `settings.ini` | ours (by choice): the window sizes and the wear grids' `labels_on_top` not kept |
| Escape closing (`esc_*` options: backpacks, chests, corpses, dialogs, info view, inventory, LFT, mission selection, nano, NPC chat, option panel, perk window, planet map, raid, research, shops, tips, tower build menu, trades, tradeskill, vehicle, wear) and `AutoCloseWindows` | which windows Escape closes | `ui.rs` `escape_option`/`escape_closes`: each window by its own option as the original reads it (LoginPrefs.xml's defaults when the skin's values are not loaded): option panel, shops (`esc_shops`) and player trades (`esc_trades`), the `DialogBox_c` dialogs (`esc_dialogs`: perk messages, team invite, friend removal, mail deletion, disable tips, the trade's double confirm), perks, research, mission selection, planet map, info view, NPC chat, nanos, inventory, wear and inspect (`esc_wear`), tradeskill, tips, corpses and backpacks (`InventoryView_c` FUN_100ccefe; never the bank); each closes as its close button would (`chrome::close_soon`: a shop or NPC chat tells the server); mail, the journal, team, pets, factions, friends and the other windows without an option stay; pinned windows stay | part: `esc_chests` (a container in the world is not told from an own backpack) and the windows not ported (raid, LFT, vehicle (`MechSelect` FUN_100f3e80), tower menu, city terminal, chat config); `CharCreateModule_t::SlotEscPressed` not ported |
| `TipSystem_t`, `TipViewConfig`, `/tipoftheday` | tips of the day | `tips.rs` (`TipHeader.xml`, the events and their data patterns, the switched-off tips and `IsTipEnabled` in `tips.ini`) | part: the events the client knows (selecting, tells, team invites, attacking, playfields, item pages, the skills/inventory/shop windows, skill changes, minutes played); no `/tipoftheday`, no reset |
| `TimerSystemModule_t` | timers (countdown bars) | `ui.rs` `camp_bar` (logout) and `timer_bars`, fed by `timers.rs` (Gamecode's timer list FUN_1003d334/FUN_1003d517 and the bars, `StartTimerBar` 0x1005214f / `StopTimerbarMessage` 0x10051fd8 / `GameTimerBar_c` FUN_10051cc4) | yes: Attack (one per weapon: FUN_1006a1b1 → FUN_1006872e → FUN_10068331 on the own `Attack` echo, held full until the weapon's `AttackInfo`, FUN_1009b8ba, then recharging; `StopFight` takes the bars), Special (`QueuePerk`), Nano (own `CastNanoSpell`, FUN_1005141d's cast time, gone when it lands), Item (hotbar use, FUN_1003bb5a: stats 294/210, not for a bag or under 31 hundredths, shown from 0.5 s), Reload (FUN_10068fd2); first free of 20 places. Not ported: kind 6 (equip, FUN_100482c7), the original re-adding a restarted attack entry in a new place, items used only after the timer |
| `SoundOptionsMonitor_c` | applies the sound options | `audio.rs` | ours |
| `BrowserModule_c`, `BrowserView_c`, `BrowserWindow_c`, `LoginBrowserWindow_c` (Awesomium) | the in-game web pages (market, item store, customer service, daily rewards) | `browser.rs` (WebView2 through wry; the site's requests passed on with the game's `X-Anarchy-*` headers) | part: the market, item store and daily rewards modes (what a site sends gzipped is opened, an empty POST has its length: 2026-11-11); `LoginBrowserWindow_c` for a login's error page (`loginviews::LoginBrowser`, 2026-11-12; the dimension's own page, `url`, not shown: the list gives none); customer service not seen; no URL bar |

## 2. The windows

| Window (`*_window` / config) | GUI.dll module or view | Here | Status |
|---|---|---|---|
| Login, character select, activate, delete, limbo (`LoginModule_c`, `LoginWindow_c`, `CharSelectWindow_c`, `CharSelectItem_c`, `CharActivateWindow_c`, `CharDeleteWindow_c`, `LimboWindow_c`, `LoginWorld_c`) | Views `LoginWindow`, `CharacterSelection*`, `CharacterActivate/DeleteWindow`, `LimboWindow` | `loginviews.rs` on `LoginWindow.xml`, `CharacterSelectionWindow.xml`/`CharacterSelectionItem.xml` and `CharacterDeleteWindow.xml`; `launcher.rs` runs them (delete: system 0x14/0x15) | part: no activate, no limbo; a login's error as its page over the login (`ShowError`, 2026-11-12); the loading screen from Play until the game's window is up (2026-11-12) |
| Character creation (`CharCreateModule_t`, `BreedScene_t`, `ProfessionScene_t`, `AppearanceScene_t`, `NameScene_t`, `SceneBase_t`) | | `charcreate.rs` (the scenes' rules: breeds, professions, heads, height and build, name rules, the server's refusals; the creation message is `mao_net::create`) | part: the screens are the client's own |
| Control center (`ControlCenterModule_c`, `control_center_window`) | `ControlCenter.xml`, `ActionMenu/*` | `gui/controlcenter.rs` (the XML, its criteria and `CCMenu`s: `ControlMenu_c`, `CCMenuEntry_c`, the actions, `gui/menu.rs`); `vitals.rs` for the docks | part (the docks laid out from the XML and drawn by `vitals::show_docked`, the strips' NCU/credits and AGG/DEF slider; no mini toolbar, docking into the rollup area, drag of entries) |
| `DistributedValue_c`, `ExpressionParser_c`, `CriteriaMonitor_c` (Utils.dll) and the program's criteria context (anarchyonline.exe FUN_00408268) | the values and `activate_criteria`/`criteria` | `gui/dvalue.rs`, `gui/expression.rs`, `gui/variant.rs`, `gui/stat_names.rs` | done (values from the prefs files; the user's `Prefs.xml` not read; `id:` and `module:` contexts not ported) |
| Targeting (`TargetingModule_t`, `TargetView`, friendly/hostile health bars) | | `ui.rs::target_frame` | part |
| Chat (`ChatGUIModule_c`, `chat_group_window`, `ChatGroupWindowConfig`, `ChatConfig`, `log_window_config`, `/filter`) | | `chatgroups.rs` (the groups FUN_10084436 makes, the fight lines' groups of Gamecode FUN_10012c3e, the windows' configs FUN_1009aecc and the two default windows FUN_100950df), `chatwindow.rs` (each tab a chat window showing its selected groups; the window menu FUN_100abb98: talk to, subscribe, autosubscribe, timestamps, new, delete; the output group for typed lines) ; `chatconfig.rs`: the chat configuration window (`GroupControlView_c` FUN_100ad81c / FUN_100a19ff: the window list, the group tree in its three folders with check marks, hide-others, new / rename / clone / remove, log messages); a channel's lines in its kind's colour (FUN_1008582a) ; private chat groups (invitations by `ChatPGInviteAction`, joining, leaving, their lines and talking to them: the chat library's packets 50 to 58) ; a chat window's frame (`window_name`): the windows of one frame its tabs, each other frame a window on screen, moved by the menu's own entries in place of dragging ; the Visual menu (FUN_10097756, FUN_1009a00d: mode normal or borderless, transparency, timestamps, text input, hidden input bar, fading, always behind / on top) with an input bar in every chat window | part: tabs are not dragged; the Visual menu's transparency is two lists for the original's sliders, fading follows the option names only, the mode's rule is simplified; no window rows' own menu in the configuration, no log window; `/ch`, `/group`, `/o` and `/t` done (FUN_1009c5b8, FUN_1009c977, FUN_1009cdfe, FUN_1009d028) but not tried live; the invitation's box has no list of windows; private groups not tried live |
| Wear / inventory / backpacks / bank (`InventoryGUIModule_c`, `wear_window`, `inventory_window`, `bank_window`, `ItemSlotView`) | | `gear.rs`; `gui/itemslot.rs` (`ItemSlotView_c`/`ItemIconView_c` in the XML windows: the slot picture, the centred icon, the item's tip, clicks) ; `loot.rs`: `InventoryView_c` / `ItemContainerView_c` for the bank, corpses and bags on the grid-mode `MultiListView_c` (the 102 bank slots, counts, selection, right or double click takes, drag onto the inventory takes) ; `invwindows.rs`: the inventory (`InventoryView_c` type 0: a 30-place grid) and the wear window (`WearView_c` FUN_100e24b8: Weapon/Cloth/Implant/Social tabs, the original's slot tables and text.mdb 505 labels, `GFX_GUI_WEARVIEW_BG1/2`, 11x9 spacing), right click (the original's use) or double click uses, drag between them and to hotbars; the inspect window (`WearView_c` with the inspected identity: "Inspect", the same tabs, the items' tips, right click shows the item's page) | part: the wear and inspect windows keep their tab (`WearViewConfig` / `InspectViewConfig` `selected_tab`) but not `labels_on_top`; one's own bags have their own windows (`Container_<type>x<instance>`, 21 places, renamed from their menu) but without List Mode, Auto Arrange, hand-placed items or `esc_backpacks`, and not tried live; the right-click menu is the client's own |
| Info view: item, nano, character info (`InfoViewModule_c`, `info_window`, `InfoViewConfig`, `InspectViewConfig`) | `InfoView.xml` | `gui/infoview.rs`, `gui/iteminfo.rs`, `gui/criteria.rs` | part (item, weapon, armour, implant and nano pages with DPS; no nano radius, shop price; a character's page as FUN_1003482c lays it out, `charinfo.rs`, from the info packet: without the title's and defender rank's names, the side bonus, the city, the PvP ranks, the own character's extras) |
| Skills (`skill_window`, `SkillConfig`; `SkillWindow` FUN_100fcbac, `StatRow` FUN_100fef4c) | `Skills.xml` | `skillwindow.rs`: the skin's view, FUN_100fbfb4's groups and rows in each list and page, the details (FUN_100fa1c4), +/- and Save Changes (the `Skill` message) | part: the names coloured by cost level, the pages' bars, the cost, total cost and cap fields and the increase button's checks are in (`skillcost.rs`: Gamecode's costs and caps from the game's records); the suggested distribution from the game's `ipdist.xml` (`ipdist.rs`, FUN_100fb667 / FUN_100fb8de), the skill and full resets behind their questions (`N3Msg_ResetSkill`, built from the decompile and never sent live), the buttons acting at the press and repeating, the value green or red when buffed or debuffed; the refusal for a "temporarily changed" skill is not ported |
| Stats (`stat_window`, `StatViewConfig`; `StatView_c` FUN_1008057d) | | `stats.rs`: the name and level line (white word, value in 0xbbbbff), the health and nano bars as `PowerbarView_c`s of `GFX_GUI_HOR_BAR_RED` / `_BLUE` over `_EMPTY` labelled "value / max", the experience, alien experience and three score bars the player turns on (the config's five flags, in the right button's menu), the nine rows named from text.mdb 2003 | yes (the score bars without their next rank's score; the attack rating is the client's own sum, where the original asks Gamecode for stat 22) |
| NCU (`ncu_window`, `zFNCUWindowConfig`), target NCU (`target_ncu_window`, `FightTargetNCUWindowConfig`, `NanoTargetNCUWindowConfig`) | | `ncu.rs`: `NCUView_c` (FUN_100d774b) / `NCUListView_c`: the grid in icon size 2 (39x23, `SLOT_40X24`), sorted by time left, icons fading as the time runs out, the title "NCU (used/max)", a click shows the nano's page ; the menu's "List Mode" (FUN_100d758a): the eleven columns of FUN_100d774b written as FUN_1003d7d0 does, sorted by the time left, hostile rows red; hidden nanos (flags 0x1000000) left out ; the two target NCU windows (`TargetNCUView_c` FUN_100dda60: three columns, List Mode and Hostile Only, shown by `ShowNanoTargetNCU` / `ShowFightTargetNCU`) | part: another character's nano times are counted from the first announcement; the target windows have no borderless style; no radius column value, no "Self" type, no Auto Arrange, widths and sort not kept; the right-click cancel entry is the client's own (in the original's popup) |
| Nano programs (`nano_window`, `NanoViewConfig`, `NanoView_c` FUN_100d5328) | | `nanoprograms.rs`: the `SchoolSelector_c` buttons (Misc, Combat, Medical, Prot, Psi, Space) over each school's grid (icon size 0: 31x31 on `SLOT_32`), by name, schools from stat 405; a click casts (`UseNewMouseFunc`), drag to a hotbar ; the program's menu (FUN_100d49a0: look at, hide / unhide), the window's (FUN_100d4f90: List Mode with the ten columns of FUN_100d5328, Show Hidden, Auto Arrange FUN_10041637), the school shown kept | part: no `item_position_map` (programs not placed by hand, "Don't Auto Arrange" dead), no radius value, column widths and the list's sort not kept; menus not clicked |
| Perks (`PerkWindowModule_c`, `perk_window`, `DPerkWindow`) | `PerkWindow_c` FUN_10063ff8, `PerkView_c` FUN_10061a07, `LoadPerks` 0x1006372d | `perks.rs`: the three tabs (profession, group, general) over every perk line the profession/breed/expansions list (`LoadPerks`' filter and kinds, sorted met / owned / name), the line list and the chosen line's ranks in two columns with the status tints (owned, met, unmet) and alien labels, used/available (alien) points (`N3Msg_GetNumberOf...Perks`), the criteria (`N3Msg_MeetsPerkCriteria`: the template's wear criteria), a click on a rank with every message box of FUN_10061021 and the Yes/No question before `N3Msg_TrainPerk`, Remove Perk (FUN_100616d2, the highest owned rank) and the two-hour untraining countdown from stat 577, the rank tips (requirements, modifiers, description), the status icon's picture (`GFX_GUI_PERK_STATUS_ICON`) placed as `Button_c::LayoutLabel` does, the timer as `TimeBar_c` (FUN_10060920: the `GFX_GUI_PROGRESS01` border, background and front) | yes (training, untraining and a running wait not tried live) |
| Special actions (`specialaction_window`, `SpecialActionView_c` FUN_100dc9b1) | | `actions.rs`: "Actions", the special actions in a 31x31 icon grid; a click performs, drag to a hotbar; social actions are the control center's Social menu ; the special attacks the server lists for the character (`SpecialAttackWeapon`) by the original's number-to-template table (Gamecode FUN_10042b17, FUN_1006b14c) ; a weapon's own attacks and Reload from the weapon in the hand (`WeaponItem_t` FUN_1009e84b: its "can" flags and ammunition stat) | part: backstab (489) from the decompile only, never seen; no `item_position_map`; the toggles keep one name. Burst, Fling Shot and Reload tried live |
| Shortcut bars (`shortcutbar_window`) | | `hotbar.rs` | ours |
| Friends (`friends_window`, `FriendListView_c` FUN_100aa315) | | `friends.rs` on `gui/treelist.rs` (`ListViewBase_c`: folders with the expand pictures, rows indented 15, `StringListViewItem_c`'s picture and label colours, flashing): Chat Windows / Online / Offline / Recent folders, the status lights, sorted by name; the recent messages (temporary buddies, FUN_100a6487) in their folder; the left button starts a tell (the original opens the tell window, FUN_100a7ca7); the right button's menu (FUN_100a99cb: Befriend, Invite, Delete, Ignore / Unignore, Mail) with the two questions; the chat window's row shows and hides it; the folders' open state kept (`tabs.ini`) | the left button opens the character's tell window since 2026-11-11 (`ChatWindow::ask_tell`) | part: no flashing row for unread tells, the tell windows not listed under Chat Windows, the chat window's own menu (FUN_100a8a69) not built |
| Mail (`mail_window`, `MailIconWindowConfig`; the message window FUN_10107fc8, the inbox FUN_10109a11) | `MailWindow.xml`, `MailMessageWindow.xml` | `mailmessage.rs`: the message window on its XML (new mail and received, the selectors, read-only fields, Take All, Reply, Delete, Send, the item dropped in); `mailbox.rs`: the inbox on `MailWindow.xml` (`MailWindow_c`: the five columns, read/unread pictures, the size, a double click opens a mail, Delete asks first) | part: Return To Sender and C.O.D. not sent; the columns' widths and sort are not remembered |
| Missions, mission selection (`mission_window`, `MissionView_c` FUN_100d3644, `missionselection_window`, `BMissionSelectionViewConfig`) | | `missionlog.rs`: the `MissionListView_c` list (FUN_100d3df7: Icon, Name and time left columns; rows with the icon, the name and "hh:mm" from `N3Msg_QuestRemainingTime`; the active mission selected), a click shows the mission's page in the info view (FUN_100360d5: text, time left, team rules, credits, experience, reward items), a right click its menu (Look at, Upload to map, Update Team Members for a team mission, Delete); `mission.rs` the booth's offers | part: the list's columns and the active mission are not kept between sessions; Update Team Members not sent; no shadow knowledge reward row; the mission selection is still the client's own |
| Shops, NPC trade (`TradeView_c` FUN_100e121e modes 1/2, `ShopViewConfig`, `itemshop_window`, `ItemShopWindowConfig`) | `TradeGUI.xml` `ShopTrade` | `shop.rs` (the view from the skin; the docks' `ItemContainerView_c` lists drawn by it; Gamecode's prices) | part: `ShopBuy` mode and the list configs (`shop_listview_config`) not read; the item shop is a web page |
| Player trade (`TradeViewConfig`) | `TradeGUI.xml` | `shop.rs` in its player mode (`PlayerTrade` view): open by using a player, items in, credits, Confirm, the double-confirm dialog, complete or decline | part: the partner's items are not named (not captured); no out-of-range check (`Feedback_TargetOutsideRangeForTrade`) |
| Tradeskill (`tradeskill_window`, `TradeSkillViewConfig`) | `TradeskillPanel.xml` | combines by dragging a backpack item onto another (`UseItemOnItem` 0x51, `N3Msg_TradeskillCombine`); `tradeskill.rs`: the window (`TradeSkillView_c` FUN_100df2a7): source, target and result slots, `N3Msg_InsertSource/TargetAnalyzerItem` (0xdc/0xdd) and `N3Msg_BuildAnalyzerItem` (0xde) as the 2026-10-30 capture, the server's answers 0xdf..0xe4 into the four texts, the quality slider and labels, Clear/Build enabling, the result's page | part: the result is named by its low template, two GlobalSignals of the window (+0x1d4, +0x1d8) not traced |
| Team (`TeamViewModule_c`, `TeamView_c` FUN_10078c14, `TeamViewConfig`, `/team`, `/invite`, `/kick`, `/leave`) | | `team.rs`: the header ("Team level: %u  Team side: %s" / "No team"), Leave, six member views with the splitters, names (leader 0x11eeaa) and `GFX_GUI_BAR_HEALTH_TEAM`/`NANO_TEAM` bars (one's own from the character's stats); Recr; a member's click (target) and menu (info, make leader, kick: FUN_1007a23a) | part: no team side in the header; `/invite`, `/kick`, `/leave` are the private chat group's and not carried out. Invitation and team chat tried live with two clients |nvitation's "Want to join %s's team?" | part: no invite or kick (the join request was not captured), team side not known, `/team` commands not carried out |
| Raid (`RaidViewModule_c`, `raid_window`, `RaidConfig`, `RaidBarConfig`) | `Raid.xml`, `RaidBar.xml` | `raid.rs`: the view from the skin, Team 1 filled with the character's own team (profession picture, level, name; the leader in the team colour), the other teams empty; `/raid` and the Team menu open it | part: no raid messages captured, so Convert To Raid and Raid Locks send nothing and `RaidBar.xml` is not drawn |
| Looking for team (`lft_window`, `LFTWindowConfig`) | `LFTView.xml` | `lft.rs`: the view from the skin with the side and profession drop-downs filled; the flag and the description kept in the window; `/lft` and the Team menu open it | part: the list lives on the chat server and was not captured, so Search finds nobody and Invite / Tell send nothing |
| Pets (`PetWindowModule_c`, `pet_window`, `PetMenu.xml`, `/petduel`) | `PetMenu.xml` | `/pet <command>` (Gamecode's words) and the control center's Pets menu, to all pets; `pets.rs`: the pet window (`PetWindow_c` FUN_1007e9d7, `PetView_c` FUN_1007d817): the pets in their places (`AddPet`/`RemovePet`), each a red health bar labelled with its name (grey away, white targeted), and Attack/Heal, Follow, Wait to that pet (`N3Msg_SendPetCommand` with the pet in the list) | part: no icon list of a pet's nanos; the icon menu is a menu button in the window with Show buttons and Fixed Pet Buttons (rows by the pets' family, kept in `mao.ini`), Remember Pet Names left (no name travels with `/pet rename` yet); `/petduel` is done (section 3); the one-pet command was not captured |
| Organization (`OrganizationGUIModule_c`, `OrgNameView`, upkeep views) | | none | no |
| City terminal (`CityTerminalGUIModule_c`, `CityTerminalWindowConfig`, `CityTerminalBuyDlgConfig`, `CityMapView`) | `CityTerminal.xml` | none | no |
| Market (`MarketWindowConfig`, `OpenMarketSendWindow`, `MarketSendWindow_c`) | `MarketSendWindow.xml` | `browser.rs`, `marketsend.rs` (the deposit window, `MarketSend`) | part: not seen live |
| Knowledge (`knowledge_window`, `KnowledgeViewConfig`) | | `knowledge.rs`: `KnowledgeView_c` (FUN_100cfa6c) as the decompile shows it, a reference board: a grid items are dropped on from the inventory (the pointer's cell or the first free one; an item already there moves), a click shows the item's page, a right click a Show / Remove menu, the board kept in `knowledge.ini` (the original's `KnowledgeViewConfig` `item_id` / `item_pos`); the mini toolbar's Knowledge button opens it | part: character references (`charref://`) are not dropped; an item read back from the file shows its name until the inventory lends its picture |
| Factions (`faction_window`, `zFFactionWindowConfig`) | | `factions.rs`: `FactionView_c` (FUN_1005e3f5): the seven Shadowlands factions' `FactionBar_c`s (stats 566-572: short name, standing or band title in 0xffff88, `GFX_GUI_HOR_BAR_GREEN/RED/YELLOW` filled within the standing's band from the database's 0xf4265 records), a left click switches standings and titles; opened by the control center's `faction_window` | part: `IsFactionTitleShown` is not kept between runs |
| Research (`research_window`, `ResearchConfig`) | `Research*.xml` | `research.rs`: `ResearchWindow_c` (FUN_100f750e) on the three skin views as tabs: the sliders (SetStat 263/264, held to 100 together), started personal and the chosen global goal, the goal radio buttons (SetStat 265/266), the finished ranks, the line and description of a row, `3115534d` on opening; data from the world entry's `Perks`, `ResearchUpdate` and `435f7023` | part (no `got_tech`; its place is kept, as `ResearchConfig` keeps only the window's place, FUN_100f5461) |
| Planet map window (`planetmap_window`, `PlanetMapViewConfig`, `PFMapWindowConfig`, `map_window`) | | `planetwindow.rs` (`PlanetMapView_c`: small at the top left and open at the first start as the original's default layout has it, the map over the Zoom in / Zoom out and Character / Mission rows); `automap.rs`: corner map and overlay, with the Notum Wars tower icons and land-control circles of `PFMapRenderer_c` (FUN_100ea243; `mao_net::landcontrol` from `PlayfieldAllTowers` / `PlayfieldTowerUpdateClient`) and the land-control view: the playfield's land-control map over the map (`N3Msg_GetLandControlBitmap`, `mao_formats::landcontrolmap`, coloured as FUN_100ebc3f), the district and level range (`N3Msg_GetLCDistrictInfo`, FUN_100eb04b), MapControlView_c's `Info` toggle with its `ShowLConMap` tip (FUN_100ecdf7), remembered (`RenderLandControlData`) | part (MapControlView_c's `Build`/`War` buttons and coordinate/waypoint row; the planet map window's own PF map view) |
| Compass (`compass_window`) | | in the control center | done |
| Notification window (`NotificationWindowModule_c`, `notification_window`) | FUN_1000bf4b, `SlotAddText` 0x1000c796 → FUN_1000be0e, `FadingTextView_c` FUN_10008355 / FUN_100081e8 | `notifications.rs`: the top third of the screen, lines from the bottom up (oldest dropped when full), each held 5 s then faded over 3 s; `NotificationWindowVisibility` true draws the "Notifications" tab and a backing (style 0), false (default) only the lines (style 3); fed by the spell 0xcfff (FUN_100a9af8: string 0, range stat 11) in a `SpellList` | part: `NotificationWindowConfig` (saved frame, `ClickableLinks`) and links not ported; text colour 0 drawn white; no 0xcfff seen in the captures |
| Petitions (`petition_window`, `PetitionWindowConfig`, `/petition`) | | none | no |
| Reclaim (`reclaim_window`, `/reclaim`) | | none | no |
| Daily login / check rewards (`dailylogin_window`, `checkrewards_window`) | | `browser.rs` (the page), `n3::claims` (`RequestClaims` 0x107 when the window goes, `RequestClaimsDBCheck` 0x108 for "Pending Rewards") | yes, seen live 2026-11-11; the request lock 0x8c not ported |
| Tips (`tip_window`) | `TipView.xml` | `tips.rs` (`TipView_c`: sections, next section, previous/next tip, the two check boxes; `TipButtonWindow_c`'s `GFX_GUI_NEW_TIP` button) | yes |
| Options (`OptionPanelModule_c`, `optionpanel_window`) | `OptionPanel/*` | `gui/options.rs` (the `Root.xml` pages and option controls; the game's own Root.xml is empty, so an AOU skin's at hand supplies the pages, with the current skin's art; "Preferences", 800 by 480); the client's own options window for the display and keys behind "Client settings" | part (`HotKeys.xml` not used; most settings kept but not yet acted on) |
| Help (`help` / info about this window) | | `help.rs` | ours |
| Notum Wars: tower build menu, laser target, airstrike, battlestation, signup terminal, mech select, teleport target | `LaserTarget.xml`, `SignupTerminal.xml`, `MechSelect.xml`, `TeleportTarget.xml` | the tower sites and the land-control view on the map only (`automap.rs`, above) | no |
| Whom-pah (in GUI.dll's NPC/terminal handling) | | `whompah.rs` | ours |
| NPC chat (`NPCChatWindowConfig`, `NPCChatWindow_c`, `NPCChatView_c` FUN_100596c2, `NPCChatTradeBar_c`) | | `npcchat.rs`: the conversation and answers as text views with the original's markup and colours, the button bar (Desc, Info, Give, Shop), the give bar | part: the Shop button's trade with the NPC not carried out |
| Character viewer (`CharacterViewerModule_c`, `CharacterViewer_c`) | | the inventory's figure | part |
| Progress dialog (`ProgressDialog.xml`, `ProgressWindow_c`) | | `loginviews.rs` (while logging in) | part: not shown while a character is entered |

## 3. The slash commands GUI.dll registers

Carried out here: `/tell` (without a text: the character's tell window; `%t` `%f` `%m` in any sent text are the
target's name, the fought one's and the player's own, `ExpandChatTextArgs`) `/reply` `/whisper` `/shout` `/emote` and the emotes, `/voice`, `/camp`, `/follow`, `/inspect`,
`/open`, `/close`, `/items`, `/waypoint`, `/selectself`, `/setoption`, `/chardist`, `/viewdist`, `/quit`, `/help`,
`/version`, `/ignore` (and the client's `/befriend`), `/duel` (with `accept`, `reject`, `stop`, `draw`; the
"Duel Challenge" boxes of `GuiSystem_c::DuelChallengeReceived` / `DuelChallengeSent`; tried live with two clients),
`/petduel` (with `accept`, `reject`, `stop`; texts only, as the original; live as far as the server's refusals).
Since 2026-11-12 (the handlers by the registration at 0x100bb434 on, each command's the one pushed after its name):
`/text` (FUN_100b5f02), `/showfile` (FUN_100b7455: the info view's `file://`), `/tipoftheday [prev]` (FUN_100b74fc),
`/toggle` (FUN_100b7da5), `/messagebox` (FUN_100b805e: this client's error box, so titled "Error:"), `/petition`
(FUN_100b5e9c), `/invite` `/kick` `/leave` with a name (FUN_100bacca, FUN_100badd5, FUN_100baee0: the private chat
group's requests 3, 4 and 5, the name looked up first; not tried live, it needs a second player's consent),
`/assist [name]` (FUN_100b81b4, `N3Msg_AssistFight`: the target's opponent selected and attacked; not tried in a
fight), `/perks` and `/perk` (sent to the server as `/stuck` is). `/messagebox` was tried live; the others ran
without a fault but their lines were not read back.

Since 2026-11-12 also (`macros.rs`, `scripts.rs`, `chatfilter.rs`; each module's head says what was read):

- **`/macro <name> <command>`** (FUN_100b8c82 raises `GlobalSignals_c` +0x1a0; the shortcut bar's FUN_100d8c4a
  answers). `TextMacroSystem_t` (`CreateMacro` 0x1002338f, `GetMacro` 0x10023366, `Save` 0x10023201, `Load`
  0x1002350b, `Init` 0x100236ec): a macro is a number, a name and a command, kept in `TextMacro.bin` (a count, then
  number, name and command with their lengths, big-endian; checked against the files the original wrote on this
  machine). The macro is made and saved, and a drag of it begun; dropped on a bar it is an entry of kind 7 whose
  picture is its name in yellow (FUN_100402e8), whose tip is the name and the command (FUN_10042193, "Broken
  macro" without one), and whose press raises the signal that carries a line out as typed (FUN_100d833f, +0x180).
  Here: the file is the client's own beside its settings; the macro rides on the pointer (egui's drag payload, its
  name drawn beside the pointer) until a click on a hotbar slot sets it down; `hotbar::Action::Macro`. The right
  button on a `chatcmd://` link makes a macro of its command in the original (FUN_10065798): not done.
- **Script files** (`/script <file> [words]` FUN_1009d4e9, and any `/<word>` the list does not know; the loader
  FUN_100b1517, the runner FUN_100b1d6c): a file of chat lines in the scripts folder, `#` lines and empty ones
  left out, `%N` filled with the command line's words, one line a frame, `/delay <ms>` waiting. Limits and texts
  as the original's (the line under 0xb5 bytes, `Script2Long`; the file at most 0x1000 bytes and not empty;
  `Unable2FindScriptX`). The folder is the original's (`%LOCALAPPDATA%/Funcom/Anarchy Online/<install>/Anarchy
  Online/scripts`), after a `scripts` folder beside the client's settings. The files' names are offered with the
  commands (FUN_100a44e8), read where they are: `chat_commands::COMMANDS` no longer holds one install's.
- **`/filter list|add|del|enable|disable|clear`** (FUN_100b933d) and the filter itself (FUN_100854a8: a group's
  line that a rule matches is not shown; tells are not looked at). The rules are `RegExp`s of Utils.dll, which is
  Henry Spencer's library of 1986 (its program's mark 0234 is checked by 0x1000c469): ported as
  `chatfilter::Pattern`. The original keeps the rules in the `ChatFilterRules` value; here they are a file.
- **`/dvalue <name> [value]`** (FUN_100b7716): a stored value shown, or set to what the text works out to as an
  expression, held to the value's range; a name that is not there is made. **`/start <address>`** (FUN_100ba073,
  FUN_100b9acf): an `http://` or `https://` address opened by the system (the original asks first when the game
  is full screen: not done). **`/char&viewdist <a> <b>`** (FUN_100b6a54): both distances.
- **`/played`** (FUN_100b2906): its own line, the time here, in Greenwich and in the game, and the date. It then
  gives "played" to Gamecode's text commands, which is the part not done (below).

Not carried out, and why (the handler is the one registered after each name; an earlier note here had `/tower`,
`/terminate`, `/rp` and `/reclaim` one off):

- (Carried out later the same day, see below: `/played`'s request, `/born`, `/team`, `/bank info`.)
- `/tower` (FUN_100b375a: `create`, and `terminate` with a tower selected, go to Fanatic's own
  `ClientInterface_c::Command`; the rest to `N3Msg_TextCommand`), `/terminate` (FUN_100b3a69: a yes/no box, then
  Fanatic), `/gfx <n> <on|off>` (FUN_100b3863, `N3Msg_EnableGFX`): Notum Wars and the effects' switches.
- `/cc` (FUN_100ba6f4: `/cc info <name>` is a private message of kind 1, the rest a list of words sent to the chat
  server by FUN_1016caf7; neither packet is decoded). The original's `Befriend` script file is a `/cc` line.
- `/name <new name>` (FUN_100baba6: a private message of kind 5, "Sending name change request."): not decoded, and
  not something to try on a character.
- `/rp` (FUN_100b27c2: the character's stat 673 with bit 0x200 turned over, sent through `N3Msg_EventFeedback`
  0x46), `/reclaim` (0x100b27af: `N3Msg_ToggleReclaim(false)`, the reclaim window), `/bug` (FUN_100b6019: a
  report sent to the server), `/option` beyond what `/setoption` does.
- `/lft` FUN_100b6f9f opens its window; `/rp` and `/reclaim` need a capture to see what the server answers.

**Gamecode's text commands** (`/played`, `/born`, `/team`, `/bank`; carried out on 2026-11-12). GUI.dll registers
`/played` (FUN_100b2906) and a list (`/version`, `/bank`, `/team`, `/org`, `/born`, `/pet`, `/follow`, `/items`,
`/raid`; FUN_100b285d, registered by FUN_100b2ac9) that hand the line, without its slash and with its `%` arguments
put in, to `N3Msg_TextCommand` (Gamecode 0x10017743) with the chat window's number and the target. Its FUN_1003fe10
compares the first word (FUN_10044054) and, for most, builds a `CharacterActionIIR_t` with FUN_10072a88: the
action, the window's number where other actions have nothing, nobody as the target, a second identity, an empty
text. **They are not the server commands `/stuck` is** (Fanatic's `FanaticIIR_t`): "played" sent that way was
answered "You are not authorized to perform that command. (Your request has been logged)".

- `played`: action 0x85. `born`: action 0x86. (`version` is 0x9d; `/version` here says the client's own.)
- `team`: outside a team the feedback text `Feedback_YouAreNotMemberOfTeam` (FUN_10059047, category 110). Alone:
  `Feedback_AvailableTeamCommands`, `Feedback_TeamLoot`, `Feedback_TeamLootAll`. The next word is looked up in a
  table (FUN_1002f174; filled at 0x10033bae: team 1, loot 2, all 3, leader 4, alpha 5) and must be `loot`: alone it
  is action 0x91; with a word after it the team's leader sends action 0x7f with that word's number as the second
  identity's instance (whatever the word: a word not in the table is 0), and anyone else is told
  `Feedback_OnlyTeamLeaderCanChangeLootOrder`.
- `bank`: its word is looked up (FUN_1002f123; open 1, close 2, info 3): `info` is action 0x5c; `open` is action
  0x24 for a game master only (stat 215); `close` closes the bank's window in the client (not done here).
- `items` (read later the same day): alone or with `list`, action 0xf4 with nothing else (FUN_1004b6c9); `delete
  <number>` is action 0xf5 with the number (FUN_1004b772), which has the server delete an item of the account:
  not carried out here. Until then `/items` opened the inventory, which the original's does not.
- **`org`** (`orgcommands.rs`): **`/org` is the organization's commands; its chat is `/o`** (until 2026-11-12 this
  client sent `/org <text>` to the chat). The word after it is looked up (FUN_10032366; the table filled at
  0x100334a0 on: create 2, ranks 3, governingform 4, info 5, promote 6, demote 7, name 8, history 9, description
  10, objective 11, leave 12, invite 13, disband 14, kick 15, contract 16, tax 17, bank 18, startvote 19, vote 20,
  stopvote 21, debt 22, city 23) and what follows is its text; a text with a character outside 0x20 to 0x7e is
  refused (`CannotUseLettersX`). Each makes an `OrgClientIIR_c` (FUN_101266b4; `7f4b3108`, which is
  `n3InfoItemRemote_t::MapToKey` of the name, N3.dll 0x10009826: the name's bytes laid over one another four by
  four), packed by FUN_1012653f: the command's number as a byte, an identity (the target), the chat window's
  number, then a text for the numbers 1, 7, 9, 0xd, 0x11, 0x13, 0x14, 0x17 to 0x1c and a byte for 10. The numbers
  sent: create 1, ranks 2, contract 3, info 5, startvote 7, vote info 8, vote 9, promote 0xa, demote 0xb, kick
  0xc (the target) or 0xd (a name), invite 0xe, tax 0x11, bank 0x12 (`add` 0x13, `remove` 0x14), debt 0x16,
  history 0x17, objective 0x18, description 0x19, name 0x1a, governingform 0x1b, stopvote 0x1c, city 0x1f. `help`
  and any other word say the game's `OrgCommandHelp` ("Invalid /org command ..."); `leave` and `disband` ask
  first in boxes (`N3Msg_OrgLeaveConfirmed`, `N3Msg_OrgDisbandConfirmed`) and are not carried out here.
  **Tried live:** `info`, `ranks` and `contract` only read (outside an organization each is answered with a line;
  in one, `ranks` with a line, `info` and `contract` with `OrgServer` messages of types 2 and 1, which this
  client does not show yet). **`history` is not a question: it sets the organization's history text**, as
  `description`, `objective`, `name` and `governingform` set theirs, and the original sends them with no text
  when none is given. `/org history` alone was sent once on 2026-11-12 on a character in the player's own
  organization, in the belief that it only read; the server answered "Organization info updated.". The text was
  empty before and after (the `info` answers before and after are the same 63 bytes, three empty texts among
  them), so nothing was lost. The client sends them as the original does, with an empty text when none is
  given (for some hours it refused to; the user chose the original's way). The other numbers were not sent to a
  server.
- The answers arrive as chat texts with packed pieces in them (`~&…~` and plain text after): the client now
  unpacks them as LDB.dll's `RemoteFormat::ParseString` does (`mao_formats::ldb::parse_string`); before, they
  showed as they came.
- `raid`: its word is looked up (FUN_100323ce; create, list, listlocal, move, lootaccess, locks) and most make a
  `RaidCmdIIR_c` (FUN_100a3a86, `4e525e58`): not read further, and not carried out (`/raid` here opens the raid
  window, which the original's command does not). `tower`: needs a service tower and runs a file of `scripts/`
  (Notum Wars): not carried out. `resetskill`, `clone`, `damagemult`, `clearunique` and the `getlocal` family
  ask for a game master's level. `pet` and `follow` were read before and are carried out.

Here: `mao_net::n3::text_command` and its numbers, `chatwindow::Out::TextCommand`, the window's number sent as 0.
Tried live on a test character on 2026-11-12: `/played` and `/born` were answered by the server; `/bank info`
was sent, not refused, and nothing was shown in answer; `/team` was only tried outside a team.

`/afk` and going away by itself, carried out on 2026-11-12 (`afk.rs`, hooks in `chatwindow.rs`, `ui.rs`, `net.rs`,
`mao_net::chat`). What the original does, and so the client:

- **The state.** Being away is "the away message is not empty" (`ChatGUIModule_c` +0x294, its length at +0x2a4 is
  `IsAFK` 0x1001b7c2; the time it began at +0x2b0, `GetAFKDuration` 0x10086309). `SetAFK` 0x10087669 notes the time
  when the character was not away before, and leaves a battle station's queue (`N3Msg_IsBattleStation` /
  `N3Msg_LeaveBattle`: not ported, the client has no battle stations).
- **The command** (FUN_100b82a7; its texts are text.mdb 10001 by name, `ChatCmdFeedback_AFK_...`). `/afk status`
  (exactly that) says the message, or that the character is not away. `/afk` alone while away ends it: a line to the
  player and `ChatCmdFeedback_AFK_IsBack` as an emote to the vicinity (FUN_10085a42 with the kind 3, as `/me`).
  `/afk` alone while not away is away with `ChatCmdFeedback_DefaultAFKReply` at once and opens the box. `/afk <text>`
  is away with the text after `ExpandChatTextArgs`, or, when away already, changes the message and says the old
  and the new.
- **The box** (`AFKMessageDialog_c` FUN_1008307e, `AFKDialogView_c` FUN_100836ab, the answer FUN_100b6b65): the
  body text, a count down from 30 seconds (FUN_10083496), the message in a field, one button "Ok", in the screen's
  middle. A key in the field stops the count and blanks it (FUN_1008356e). Enter (0x18), the button and the count's
  end take the field's text: a new one is said and set, an empty one keeps the default ("Using default AFK
  message."), the same one says nothing. Escape (0xe) ends being away ("AFK off.", "is back" to the vicinity). While
  the box is up, input does not count as coming back (`InputConfig_t::AFKIgnoreInput` 0x1000b835). The window's
  title here ("AFK") is this client's: the original's window flags (0x800) were not followed to what its bar shows.
- **Tells while away** (`HandlePrivateMessage` 0x10087e59). A tell (blob's first byte 0) from a character not
  ignored is shown with " (AFK reply sent)" in white after it, and answered: `ChatCmdFeedback_AFKReplyText` fed the
  character's name, the hours and the minutes away and the message, queued as a private message of kind 6.
  FUN_10089940 sends a private message with its kind as the blob's first byte, so the answer is a tell whose blob is
  the byte 6. A tell that comes with that byte is shown through FUN_100a681d with an empty name: nothing in front of
  it, in the tells of the character it came from, with no reply, no tip and no head flash. So two characters who are
  both away do not answer each other. (`mao_net::chat::Packet::Tell` now keeps the blob; `AWAY_REPLY`,
  `away_reply_packet`; `chatwindow::Kind::AwayReply`, `Out::AwayReply`.) A tell of another kind is shown as a tell
  here; the original shows only the kinds 0, 2 (a game master's assist and summon scripts) and 6.
- **Going away by itself** (`InputConfig_t::AddUserInput` 0x1001b115, `SlotAFKTimer` 0x1001a2cb,
  `ChatGUIModule_c::SlotAutoAFK` 0x100870a8). The program hands `AddUserInput` every key and button event and every
  movement of the mouse (its FUN_00404a66 and FUN_0040383a, code 0x77); a release (flag 0x100000) is passed by. Each
  input, unless the box is up, calls `SlotAutoAFK(false)`, and restarts a timer of 180,000,000 microseconds; the
  timer calls `SlotAutoAFK(true)`. With a game master's level (stat 215) not 0 neither does anything. With the
  `AutoAFK` value on (a `DistributedValue`; `true` in the game's `LoginPrefs.xml`; the option panel has no control
  for it): `true` on a character not away sets the time and the message "Auto-away" and says
  `ChatCmdFeedback_AFK_IsAFK` to the vicinity, with no line to the player (the text `AutoAFKOn`, "automatically set
  AFK", is only said on the way out of a battle station); `false` on a character that is away, **however it came to
  be**, empties the message, says `ChatCmdFeedback_AFK_AFKOff` to the player and `..._IsBack` to the vicinity. So
  with the setting on, `/afk brb` lasts until the mouse is next moved over the window. A character in the movement
  mode 6 (sneaking, FUN_10070a2f) says neither line to the vicinity. Here the input is egui's events of the frame (a
  key or button going down, the pointer moved, the wheel, typed text), looked at before anything typed that frame is
  carried out, as the original's order has it; `/setoption AutoAFK off` sets the value.

Tried live on 2026-11-12 with the two test characters (`MAO_AFK_IDLE=6` for a timer of 6 seconds): the lines, the
emotes seen by the other client, the marked tell, the reply arriving and left unanswered by a character that was
itself away, `/afk status`, the timer and the mouse's movement ending it; and the box by its keys (a text typed
and Enter, Escape, the count's end), in runs with `MAO_AFK_QUIET=1`, which leaves the vicinity's lines unsent. The
box itself was looked at only in a preview picture (`MAO_UI_PREVIEW=chat MAO_CHAT_AFK=`). Not done: the battle
station's part. Two things the original does that look like slips are kept and written down in
`ORIGINAL_CODE_QUESTIONS.md` (11 and 12).

Developer and GM commands (not for players; leave out): `/anim` `/center` `/clearunique` `/clone` `/command`
`/criterialocal` `/damagemult` `/dumphash` `/font` `/framerate` `/funcom` `/getfull` `/gethash` `/getlocal`
`/getlocalfull` `/item` `/joycamacc` `/lazyreload` `/monster` `/monsterdata` `/reload` `/reloadgfxtweak` `/resetskill`
`/section` `/setlocal` `/spawn*` `/spelllocal` `/start` `/syncdisplay` `/teleport*` `/togglegroundlightingfix`
`/tplocal` `/weather`.

## 4. What blocks what

- Windows that need only the database and data we already decode: the info view (items, nanos), tool tips that are info
  views, the HTML text, character creation's scenes, tips of the day, the planet map window, knowledge/factions (stats).
- Windows whose messages are still not captured (the windows themselves exist where a skin view does): raid, LFT, bank,
  organization, city terminal, petitions, reclaim, daily rewards, the Notum Wars windows, target NCU. Each needs a
  capture of the original doing it (`tools/capture_session.ps1`).
- The embedded web pages are WebView2 (`browser.rs`), not Awesomium; the item shop's page is reachable but not opened by anything yet.
