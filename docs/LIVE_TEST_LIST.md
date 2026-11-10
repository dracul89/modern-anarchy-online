# What still needs trying live

Written 2026-11-10. Everything here is built and passes its tests or previews, but has not been used against the live
server. Each item says what to do and what should happen. Tick one off by deleting it (and the "not tried live" note
in `CLIENT_UI.md` it came from).

Needs: **1** = one character is enough, **2** = a second character on line, **org** = a character in an organization,
**team** = two characters teamed.

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
