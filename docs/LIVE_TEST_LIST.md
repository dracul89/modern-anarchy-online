# What still needs trying live

Written 2026-11-10. Everything here is built and passes its tests or previews, but has not been used against the live
server. Each item says what to do and what should happen. Tick one off by deleting it (and the "not tried live" note
in `CLIENT_UI.md` it came from).

Needs: **1** = one character is enough, **2** = a second character on line, **org** = a character in an organization,
**team** = two characters teamed.

## Tried live on 2026-11-10 (scripted, one character, nobody else involved)

Run with the test script's new interface steps (`MAO_CAMERA_SCRIPT` with `open@`, `rclick@x;y`, `click@x;y`,
`type@<line>`, `key@Enter`: see `CLIENT_UI.md`, "Scripted clicks"). Nothing was sent to other players and nothing
irreversible was done. The numbers are the items below.

**Passed**
- 1: the chat window's menu, in the original's order.
- 4, 6 (in part), 8: `/ch` and `/g` with an unknown group, `/ch system` (read-only) and `/t` outside a team answer
  with the game's own texts; `/ch vic` sets the output group without a word.
- 10: New Window opens a second chat window with its own input bar; "Move into the window of Default Window" makes
  it a third tab.
- 13: Show timestamps.
- 18 (opens only): the chat configuration window with the windows' list and the channel tree.
- 22 (menu only): a friend's menu (Befriend, Invite, Delete, Ignore).
- 29: the stats window's menu; Experience turns its bar on.
- 34: the NCU window's "List Mode" (as a page of the right-edge column).
- 36: `/setoption ShowNanoTargetNCU on` and `/selectself` show "Nano Target NCU" with one's own nanos.
- 40 (in part): the nano window's menu and its Auto Arrange list; a school's button shows its programs.
- 42 (icons only): the perk window's status icons and idle timer.
- 43: three bags of the inventory used one after another each open their own window, titled with the bag's name,
  all three open together. (Two used in the same instant open only one: the server is busy with the first.)
- 44: an inventory item dragged onto a bag's window goes in (it leaves the inventory when the server confirms), and
  a right click on an item in the bag brings it back to the inventory's first free place. A bag shows its items in
  the order of the server's places, so an item put in is not always in the last cell. The two items moved for the
  test were put back where they were.

**Passed with two clients at once** (Humility on the paid account, Brasscasings on the free one; the runner starts
the second with the `.env`'s free login)
- 9: `/tell` both ways; each side's line and the other's "tells you" line are in the chat log.
- 25: a tell from a character not on the list puts it under Recent Messages in the friends window, with its light.
- 19, 20 (in part): the owner's invitation brings the "Private Group Invitation" box and the chat line; Yes joins
  ("You joined private group", and the owner is told who joined); lines typed with `/g <owner's name>` on either
  side arrive on both as `[owner] who: text`. Not tried: No, leaving, the group's end.
- 18: with the fix below, every announced channel is ticked for the main tab in the chat configuration, and a
  channel of the saved list that was not announced this session is red.
A screenshot is a grab of the screen, so with two full-screen clients it shows the one on top; the other side was
checked through the chat log (`chat/<date>.txt`), which both write.

**Found and fixed**
- The Action window's tiles were blank unless it was opened with its key (its pictures were loaded on the key press
  only): now loaded whenever it is open.
- The control center's strip said "NCU 0/51" while the NCU window said 20: the strip now sums the running nanos as
  the window does.
- **The character's main chat tab showed no system lines, speech or tells.** The original's config for a window
  that takes new channels by itself lists the groups it does NOT show (GUI.dll FUN_1009e379);
  the reader took the list for the ones shown, so "Default Window" showed only fight messages and Other Pets. Fixed
  (`chatgroups::parse_config`); the built-in groups are also read by their numbers now.
- A channel the chat server announced counted as a group only once a line arrived in it, so the menus showed it
  unticked: now from the announcement.

**Not reached this time**: ticking channels on and off (2), sending to a channel (3, 5, 7), typing in a second
window's bar (12), the other Visual switches (14 to 17), the configuration's buttons (18), friends' actions (22 to
26), renaming a bag and its kept place (45, 46), a corpse or the bank beside a bag (47), nano hide and list sorting
(39, 40), and everything that needs a second character.

## Do not test without deciding first

- **Skill reset** (skills window, "Reset" / "Reset all"; `n3::reset_skill`, action `0x9a`). The message was built from
  the decompile and has never been sent. It spends a reset point and cannot be taken back. Try it only on a character
  you do not mind changing.
- **Raising skills and "Save Changes"** spends IP for good. The numbers shown were compared live (Body Development
  247 + 249 = 496, cap 107), the spending itself was not.

## Chat (2026-11-10)

| # | Needs | Do | Should happen |
|---|---|---|---|
| 1 | 1 | Right-click the chat window | The menu: Visual, Talk to Channel, Subscribe Channels, Autosubscribe, Chat Configuration, Rename, Delete, New |
| 2 | 1 | Subscribe Channels → Public Groups, tick a channel off and on | Its lines stop and come back in that tab; the list stays up while ticking |
| 3 | 1 | Talk to Channel → a channel, then type a plain line | The line goes to that channel (comes back in its colour) |
| 4 | 1 | `/ch "<start of a channel name>"` then a plain line | Same as 3; a channel the tab does not show is refused with the game's text |
| 5 | 1 | `/g <channel> <text>` to a channel the tab does not show | The line is sent and the warning about an unshown group appears |
| 6 | 1 | `/g` with a name two channels begin with | The list of matches in white, silver and red |
| 7 | org | `/o <text>` | The line appears in the organization's channel |
| 8 | team | `/t <text>`; then leave the team and `/t` again | Team line in the team colour; afterwards the "not in a team" text |
| 9 | 1 | `/tell <name> <text>` | Still a private message (`/t` no longer is) |
| 10 | 1 | New Window, then "Move into the window of …" and "Move to a window of its own" | A second chat window appears, joins as a tab, splits off again; its place is kept after a restart |
| 11 | 1 | Close a second chat window with its X | Its tabs go back into the first window |
| 12 | 1 | Type in a second chat window's own input bar | The line goes to that window's output group; each bar keeps its own text |
| 13 | 1 | Visual → Show timestamps | Lines led by `(hh:mm)` |
| 14 | 1 | Visual → Disable text input; Visual → Hide input bar when inactive | The bar goes; with "hide", it shows on a click in the window and hides after sending |
| 15 | 1 | On a window alone in its frame: Visual → Mode → Borderless, then Transparency | No frame or title; the background more solid under the pointer than away from it; the percentages change it |
| 16 | 1 | Visual → Fade messages, then wait 8 s with the pointer elsewhere | Old lines fade out; they return when the pointer is on the window |
| 17 | 1 | Visual → Always on top / Always behind | The window stays over / under the other windows |
| 18 | 1 | Chat Configuration: tick groups in the tree, "Hide channels subscribed to other windows", New / Rename / Clone / Remove Window, Log messages | Each does what it says; a channel not heard of this session is red |
| 19 | 2 | Second character: friends window → Invite on the first | The first gets the "Private Group Invitation" box; Yes joins, No declines and the inviter is told |
| 20 | 2 | Talk in the private group both ways; right-click it under Private Channels → Leave | Lines as `[owner] who: text` in the private colour; joining and leaving are told |
| 21 | 1 | Restart the client | Tabs, frames, looks and output groups are as left (`chatwindows.ini`) |

## Friends, dialogs and menus (2026-11-10)

| # | Needs | Do | Should happen |
|---|---|---|---|
| 22 | 1 | Friends window: right-click a friend → Delete, answer Yes | The box opens at the pointer; the friend leaves the list and stays gone after a relog |
| 23 | 1 | `/befriend <name>` of somebody on line | The name appears under Online |
| 24 | 1 | Right-click a name → Ignore, then again | Their lines are hidden, then shown |
| 25 | 2 | Get a tell from a character not on the list | The name appears under Recent Messages, and is still there after a relog |
| 26 | 1 | Right-click → Mail | The mail window opens addressed to the name |
| 27 | 1 | Delete an item, a mission, a mail; disable the tips | Each asks in the original's dialog box and does it on Yes only |
| 28 | 1 | Right-click in the inventory, a bag, the NCU window, the nano list | The original's popup menu, working with mouse and keys |

## Stats, skills and character page (2026-11-10)

| # | Needs | Do | Should happen |
|---|---|---|---|
| 29 | 1 | Stats window: right-click, toggle the optional bars | Bars appear and go; the choice is kept after a restart |
| 30 | 1 | Skills window: hold a "+" button on a spare character | One point at the press, then repeating; the IP left goes down by the shown cost |
| 31 | 1 | "Suggested IP distribution" on a character below level 20 with IP | Points handed out and listed; nothing is sent until Save Changes |
| 32 | 1 | Compare a few skill costs and caps with the original client | The same numbers |
| 33 | 2 | `/inspect` with another player targeted | The character page as the original lays it out |
| 34 | 1 | NCU window with a few nanos running: right-click → List Mode | The list with its eleven columns; times count down; pressing a label sorts; the choice is kept after a restart |
| 35 | 1 | Compare the list's nano cost, NCU cost, range and delays of two nanos with the original client | The same values (the radius shows `---` here) |
| 36 | 1 | `/setoption ShowNanoTargetNCU on`, select yourself, right-click the new window → untick Hostile Only | "Nano Target NCU" lists your own running nanos with their time left; with Hostile Only ticked it is empty |
| 37 | 1 | `/setoption ShowFightTargetNCU on`, fight a creature and land a debuff or damage-over-time nano on it | "Fight Target NCU" lists the hostile nano; it goes when the fight ends; closing the window turns the option off (it stays closed after a restart) |
| 38 | 2 | Select another player who has buffs running, Hostile Only unticked | Their nanos appear within about twenty seconds; compare with the original client's window (the times here are estimates) |
| 39 | 1 | Nano programs window: right-click a program → Hide; right-click the window → Show Hidden; there right-click it → Unhide | The program leaves its school's grid, is in the hidden list, and comes back; hidden ones stay hidden after a restart |
| 40 | 1 | Right-click the window → List Mode; press a column label; → Auto Arrange → By NCU Cost, Sort Ascending (in grid mode) | The ten columns with values as in the original client; the list sorts by the label; the grid reorders; mode, school and order are kept after a restart |
| 41 | 1 | Right-click a program → Look at; left-click one | The nano's page opens; the left click still casts |
| 42 | 1 | Perk window on a character with perks: look at the rows; on a spare character remove a perk (it costs the perk; decide first) | Round status icons (green owned, grey available, red not yet); after a removal the timer bar fills and counts down two hours |
| 43 | 1 | Use two different bags in the inventory | Each opens in its own window with its 21 places and the bag's name; both stay open together |
| 44 | 1 | Drag an item from the inventory onto a bag's window; double-click an item in a bag | The item goes into that bag, and comes back to the inventory; the other bag's window does not change |
| 45 | 1 | Right-click beside the items in a bag's window → Rename; type a name; later Reset to default | The window's title changes and is kept after a restart; reset gives the bag's own name back |
| 46 | 1 | Close a bag's window, use the bag again; move a bag's window and restart | It shows again; it comes back where it was left |
| 47 | 1 | With a bag window open, open a corpse and the bank | They use the shared container window; taking from each still works |
| 48 | 1 | Open the Action window on Humility and on a character with a gun that has burst or fling shot | Dimach and Brawl are there as in the original; note whether the gun's attacks match the original's window (their source is not traced) |
| 49 | 1 | On a character with aimed shot, fast attack or sneak attack: compare the Action window with the original's, and use the extra attack on a target | The same attacks listed; the attack goes off |

## Older notes still marked untried in `CLIENT_UI.md`

These were written before this list; some may have been seen working since without the note being removed. Search
`CLIENT_UI.md` for "Not tried live" to find each one's details.

- Clicking in the world: click to select, second click to use.
- Adding and removing friends (packets 40 and 41) — covered by 22 and 23 above.
- Running a nano from the programs window or a hotbar, and the NCU window filling from the server's answer.
- Dragging items: onto a wear cell, into a bag or container; right-click Use and Delete; splitting a stack is not wired.
- Auto-run key; `/follow`; `/chardist`.
- Cancelling a running nano from the NCU window; nano times updating as nanos are added.
- "Take everything from a corpse as soon as it opens".
- Facing the target in a fight.
- `/inspect` on a creature and the "wears N items" answer.
- Open windows remembered between starts.
- Absorb and damage shield lines, the reload line, friends coming and going told in chat.
- Organization names on nameplates, worn clothes on other characters, polymorph forms.
- Cast animations, nano glows, buff auras.
- Camera modes of the Camera sub-menu.
- Talking to an NPC (`KnubotOpenChatWindow`) and the mission booth window (the booth answered "too far away" on the
  one try).
- The market's deposit window.
