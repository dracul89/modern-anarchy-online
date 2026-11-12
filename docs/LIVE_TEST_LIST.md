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
**`/duel` between two clients** (Ballbearing, a level 22 engineer on the paid account, and Brasscasings, a level 25
soldier with a gun on the free one, side by side in playfield 540; nobody else involved)
- `/duel` with the other targeted: the challenger gets the "Duel Challenge" box with Cancel, the challenged one the
  box with Accept and Reject and the same text as a chat line.
- Accept, pressed in the box or typed (`/duel accept`): the server counts "Duel starts in 5 seconds." down to
  "Fight!" on both sides, and the challenger is told the duel was accepted.
- The fight: each side's attack runs on its own weapon's timer; hits, misses and a weapon's own nano strike are
  told once each on both sides, the target's frame and the health bars follow. A duel does not kill: the last hit
  is cut so that the loser stays at 1 health, and the server ends it (won on one side, lost on the other).
- `/duel stop` by one side ends it at once: the one who stopped has lost, the other has won.
- `/duel draw`: "proposed, waiting" on the one side, "your opponent proposes" on the other; the other's
  `/duel draw` ends it as a draw on both.
- Reject pressed in the box: the challenger's box goes and it is told the refusal.
- Cancel pressed in the challenger's box: the challenged one's box goes, with the text for a challenge taken back.
- Not tried: the `AutoRejectDuel` option (nobody has it on), a duel between characters far apart, a challenge to
  someone already duelling, `/petduel`.
Found and fixed on the way: every weapon hit between players was told twice (see `CLIENT_UI.md`, "Duel").
Left as it is: Brasscasings walked about 215 m to stand next to Ballbearing; both are at full health again.

**Special attacks, in a duel between the same two** (2026-11-10)
- The Action window lists what the weapons in the hands give: Burst and Reload for Brasscasings' Neutrino Flash;
  Burst, Fling Shot and Reload for Ballbearing's two pistols; Full Auto for neither (it was shown to everyone).
- Burst by either, Fling Shot by Ballbearing: the hit is told on both sides with the skill for the damage type, the
  special opens the fight (the plain attacks follow by themselves), and a second Burst inside its recharge gets
  the server's "wait" text.
- Reload: the reloading line, then the one for its end a second later.
- Not tried: Full Auto, Aimed Shot, Sneak Attack, Fast Attack, the bow's attack (no weapon for them), a special
  that misses, a weapon swapped while the window is open, Brawl and Dimach.

**`/petduel` between the same two** (2026-11-10)
- `/petduel` with the other targeted: the server answers that a duel pet is needed, with or without the engineer's
  automaton out. `/petduel accept` and `/petduel reject` with nothing asked: the server answers that nobody asked.
  `/petduel stop` with no pet duel on: no answer.
- Not reachable: a pet duel itself (neither character has a duel pet, and the game's files name no such thing), so
  the challenge's line on the other side, the accepting and the results are untried.
- For the test Ballbearing ran his smallest automaton program, used the shell it made and sent the automaton away
  again with `/pet terminate`: the pet's lines came on both sides, and the inventory is as it was.

**More with the same two, and alone** (2026-11-10, third round; the second character only standing by, started
first so that the first one's window is the one in the screenshots)
- 33: `/inspect` with the other player targeted opens the info window with his page (breed, gender, title level,
  profession, level, the invader and duel figures, alignment, the "Inspect Equipment" link) and the one-line
  answer in the chat. The profession title's name is still missing (only its level is shown, as noted in
  `charinfo.rs`).
- 38: "Nano Target NCU" with the other player targeted and Hostile Only off lists his running nanos at once.
- 23, 22: `/befriend <name>` puts him under Online Friends with the two chat lines; right-click → Delete brings
  the warning box at the pointer, Yes removes him, and he is still gone after a relog. 25: the Recent Messages
  names are still there after a relog.
- A nano run with nothing targeted (older note): **found and fixed**, see below; after the fix it lands on the
  caster, the NCU window gets its icon and the strip's figure goes up.
- `/follow` (older note): the follower walks after a player who walks away, and stops 3.5 m from him when he stands.
- 45: a bag's Rename box. **Found and fixed**: the box did not take the keys (the original's gives its field the
  focus when it comes up); now the name can be typed and Enter keeps it, and the window's title changes.
- 12 (in part): a second chat window's bar and the first's each keep their own text. Nothing was sent from the
  second (its output group may be the vicinity).
**Found and fixed in this round**
- A buff run with nothing targeted landed on another player. The test client is stopped without logging out, so
  the character stands in the world for a while with the target it had; the next session takes that character
  over, shows no target, and the server still has one. The client now tells the server on entering that nothing is
  selected (and again when a target is lost).
- The rename boxes (a bag's, a chat window's) take the keyboard when they come up.
What was still not reached after this round is in the fourth round's list above.

**Team between the same two** (2026-11-10)
- The team window's Recruit: with nobody selected and with oneself selected the game's two texts; with the other
  selected the request goes and is told, and the other gets the invitation.
- 8: after the other's Yes both are listed with bars and the team's level; `/t <text>` from each arrives on both as
  the team's channel; after Leave, `/t` answers with the game's text for not being in a team.
- Found and fixed: one's own health and nano bars in the team window were empty.
- A member's right-click menu: three entries for the leader on the other member, the info alone for a member.
  "Make leader" hands the leadership over (the other's name in the leader's colour, one's own Recruit gone); Kick
  puts the other out, which ends a team of two.
- Not tried: the invitation's No, a third member, the menu's info entry and a left click on a member, and what the
  one put out is told.

**The rest of the list** (2026-11-10, fourth round)
- 14: Visual → Disable text input takes the bar away and brings it back; Autohide inputbar hides it while the
  pointer is elsewhere and a click in the window shows it with the keys in it.
- 16: Visual → Timeout messages: ten seconds with the pointer elsewhere and the lines are gone; with the pointer
  on the window they are back.
- 15 (in part): Mode is grey for a window of two tabs, as it should be.
- 18: Chat Configuration: New Window adds a window and its row; Rename Window's box takes the keys and the row
  takes the name; Clone Window adds a copy and Remove Window takes it away; "Hide channels subscribed to other
  windows" shortens the tree.
- 39: the nano window's schools show their programs (sixteen over Medical, Prot, Psi and Space for this
  character); a program's menu is Look at and Hide; Hide takes it out of its school, Show Hidden Items lists it, and
  there its menu's second entry puts it back.
- 40 (in part): the window's menu (List Mode, Show Hidden Items, Auto arrange); List Mode shows the list's header.
  Not tried: the list with programs in it, sorting by a label, Auto arrange.
- 19: the private group's invitation answered No: the box goes and the original's packet is sent (GUI.dll
  FUN_100a74cb sends the same one, 0x35); the server tells the inviter nothing. Answered Yes: joined, as before.
- 20: lines in the group; leaving it from the Chat Configuration's tree (right click on the group under Private
  Channels) tells both sides. **Found and fixed**: the group stayed in the tree as if it were still there (the tree
  was only built again when a window changed): now also when the channels do.
- Team: the invitation's No is told to nobody; a click on a member's row makes it the target; the menu's info
  entry opens the member's page.
- Still not reached: 17 (always on top, always behind), 15 for a window alone in its frame (borderless and its
  transparency), 24 and 26 (ignore, mail from a friend's menu), 41 (a program's Look at page), 46 and 47 (a bag
  window's kept place, a corpse or the bank beside it), and the private group's end when its owner leaves.

**A new player's first steps** (2026-11-10: a fresh character, Maotesttwo, a Solitus soldier made on the free
account with `net <game> create`, in Arete Landing; scripted, other new players about and left alone)
- Made and entered: the starter items come into the inventory over the first sessions (a bag came a session later,
  moving the others' places).
- Wearing: the rifle goes into the right hand and the server confirms it; a nano crystal dragged there is refused.
  **Found and fixed**: the refused item stayed shown in the hand until the next login (the client showed every
  move at once and never took one back).
- Nano crystals used from the inventory: both are learnt and gone from the inventory.
- Skills: "Suggested IP distribution" hands the points out and lists the changed skills; Save Changes sends them
  (1500 points down to 347, health 34 to 52, the same after a relog). The first IP spent live.
- A fight: the nearest cleaning robot targeted, shot dead, "You received 145 xp", the remains opened and both
  items taken; the experience bar and the credits follow.
- Zoning: into the cargo shuttle's room and back out through its door, each a new connection, the room's map and
  its two NPCs there. **Found and fixed**: a target followed before the zoning was still followed after it.
- Talking to an NPC: the window with the NPC's lines and the answers. **Found and fixed**: an answer clicked on a
  space between two of its words did nothing (the words were clickable, the gaps not). One NPC answers that the
  character is too low in level, as it did the original's player in the capture.
- The four things found and left at first, looked at again the same day:
  - An NPC whose chat was open when the client was stopped answers no new opening until it is told the old one is
    closed. **Fixed** (this client's own way round it): an NPC silent for two seconds is told so and asked once
    more. Seen live: the window opens two seconds late and fills.
  - An item could be dragged to a place it cannot be worn in. **Fixed**: the original's own check before anything is
    sent (`N3Msg_IsItemPossibleToWear`: the item's class, its placement mask, its "wear" flag, its requirements).
    Seen live: glasses dragged to the right hand stay where they are.
  - The character was taken into the shuttle's room while fighting about 8 m from its door. Checked: the client
    asked for no zoning there (its log has no teleport try), so the server did it by itself; walking right up to
    the Shuttle Door also goes in. Nothing to fix; why the server did it during the fight is not known.
  - The camera stood outside the shuttle's room once, looking at its walls (279 of the room's 296 parts drawn, 97
    in a normal view). **Not reproduced** in two tries of the same walk, with and without the NPC's window: left
    open.
- **Found and fixed** (the user saw it in a screenshot): the blue ring at the shuttle's door stood on its edge and
  did not move; it lies on the ground now and its band runs outwards, and since the blended materials it fades
  as the original's does (compared with the original's capture screenshots).
- **Found and fixed** (the user saw it): the cargo crates' doors swung open and shut by themselves; a thing's model
  plays only as its `AnimPlay` stat says now, and the crates stand shut.
- **Found and fixed**: the things announced while entering the first playfield of a run had no name and no model and
  could not be used (the cargo box of the first quest among them).
- The first quest's second step: the cargo box selected and used from 3 m, the mission's line changes to the return
  to its giver.
- The return: the quest's giver followed and used, the reward told (experience, 1040 credits, the credits shown go
  from 5 to 1,045) and his window with the next question. **Found and fixed**: the reward's line showed the text's
  markup for "credit or credits" (the texts' choices were not known to the client).
- The second quest (its giver is under attack and answers only between the attacks): his talk, the fire
  suppressant from him, **the item dragged from the inventory onto a gas fire** (the fire is put out and the quest
  moves on), the return, and **the item handed back through the NPC's trade slot** (dragged in, Accept): the reward,
  1080 credits and an item. **Found and fixed**: the fire went on burning after the server took it away; the
  busy NPC said its line twice.
- A level gained (twice: by a reward and by a kill). **Found and fixed**: the experience bar was empty after it
  until the next login (the level's own message was not read). Health and nano grow with the level as told.
- Nine robots shot on the upper deck, 260 experience each at level 2.
- The way down to the ground (twenty waypoints taken from the original's capture, `/waypoint` each: ramps and
  decks, no place where the character was caught) and the third quest's giver; his talk and the quest.
  **Found and fixed**: a `/waypoint` while following somebody went nowhere.
- A shop: **found and fixed**: the machine answered "too far away" from 4 m (the client used another machine's
  id). Now its window opens with the list and the prices; a row double clicked and Accept buys it (100 arrows, 105
  credits).
- Not a fault after all: a corner where the character seemed caught (the test's own route cut through stacked
  crates the original's player walked round; `net <game> caught <playfield> <x> <y> <z>` shows whether a place can
  be left).
- The third quest's robots (2026-11-11): six of the seven junkyard robots shot, each telling how many are left;
  their remains opened and what was in them taken. A fight from 3 m where the server let no shot through for 80
  seconds (something between the two, it seems: one step nearer and it shot).
- **Dying** (three times with the client running): the killing hit, the death, a minute lying dead, then the
  server's move to the arrival pad with part of the health. **Found and fixed**: nothing was told and the client
  went on sending attacks while dead (and the server took them); now the game's line for the death is told, the
  health is nothing, and the attack and use keys do nothing until the character is alive again.
- The third quest's last robot and its item (the mission's line is the delivery now). Not done: the delivery.
- **Found and fixed** (the user said what it was and sent the original's picture): the junkyard's boss robot was
  drawn as a blue box. A creature's size comes in its update and sizes of 256 percent and more lost the creature
  its model; creatures are also drawn at their sizes now (they were all at their templates' own).
- Level 4 reached by kills (the level's own message not yet seen with the fix in).
- **Found and fixed** (the user saw them): the rifle was not in the character's hands; the health, nano and
  experience bars were drawn off their frames (they are the original's upright bars in the wings now).
- The launcher (2026-11-11, `mao-client <game> launch`): the page, the dimensions downloaded, the settings' three
  pages, PLAY into the login with the hall behind it. Not tried: a login from it by hand, a changed device.
- **Found and fixed** (the user played the character, 2026-11-11): a seated character stood up at any key (now
  only the sit key, and it turns where it sits); the recharger kit could not be used for that reason (sit, F1,
  right click: heals); the camera was left behind at a return from death (a new one behind the character now);
  the world fades to white while dead (the times a guess); a recharger's use and the auras played the casting
  animation (only nanos that take time to cast do now). All seen live after the fixes, with two more deaths.
- Not reached: the item's hand-over to Alex, level 5 and the way into ICC, a
  team with a second new character. The character is level 4 with 2,070 credits on the arrival platform.

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

## The unattended pass of 2026-11-11: to look at together

Done while the user was away; each is in `CLIENT_UI.md` under the same date.

- **Creatures' sizes.** Name tags and click areas follow each creature's mesh: a large creature's tag above its head,
  a small one's close over it; clicking a large creature anywhere on its body targets it; small creatures next to
  one another are told apart.
- **Remains.** A killed creature's body lies where its remains are (never in a wall or under a floor), and can be
  clicked there.
- **The Shadowlands' sky.** Towards the horizon the area's fog colour, the vortex overhead; no opaque blue sheet.
  Found and fixed off the game only: the first thing to look at.
- **The orbital strike.** `/fxscript EP03_Orbital_Attack`: rings, a column and a white flash where the camera
  stands, with its sound. Other scripts whose meshes carry effects show them too.
- **The frame limit.** With a limit set: steadier than before.
- **Casting.** A character whose nano initiative brings a nano's time to nothing casts it without the stance.

## Tells: `%t`, tell windows, the menu on a name (2026-11-11)

Tried live (two of the user's characters): `/tell` alone with the other selected, a line typed in the tell window
with `%t` in it, `/r` back, the right button's menu on the name in the tell window ("Send Tell", "Ignore" twice),
an unknown name and `%x` as a name (the connection stayed).

To try by hand:
- The left button on a name in a chat line: the tell window. With Shift or Ctrl held: `/tell <name> ` typed.
- The right button on a name in the main chat window while lines are coming in (the menu must be the name's, and
  the window's own menu anywhere else).
- The friends list's left button on a character: the tell window.
- A tell window closed and opened again: the earlier tells are in it. Two tell windows at once.
- `/tell %t` and `%t` in a plain line with nothing selected: the original's "no target" words go out.
- The chat connection ending (not seen): "The chat connection ended", then "Connected to the chat server again."
  within some seconds, and tells work again.
- A busy chat window (a city, OOC channels): scrolling and the frame rate with every line marked up.
- A buff bot's `help`: its second tell, the list, shows as a link; the link opens the page; the page's own links send
  their tells.

## Deleting a mail (2026-11-11)

- A mail with its item still in it: Delete asks, then says the attachments must be taken first; the mail stays.
- Take All, then Delete: the question, then the row goes at once and the message window closes.
- Delete in the message window answered No: the window stays.

## Item store, daily rewards, pending rewards (2026-11-11)

Seen live: the item store's lists and tabs; a daily reward claimed and delivered; both claim requests sent.

To try by hand:
- Daily Rewards on the main account: press the day's square, then Claim; close the window; the item arrives.
- "Pending Rewards" in the menu with something waiting.
- The item store: "Claim Items", and a purchase carried through (costs points: the user's call).
- Customer Service's page.

## The wear window's squares (2026-11-11)

- The wear window, each tab: the squares reach from side to side, no scroll bars; dragging an item onto a square and
  the tooltips still find the right square.
- The inspect window of another character (not photographed): the same.

## `/fxscript` (2026-11-11)

`/fxscript <script>` runs an effect script of the game's `cd_image/twk` folder (the name without `.txt`). To try:
`EP03_Orbital_Attack`, `EP03_Bomber_Event_01`, `Alien_Event_Flyby_01` to `_05`, `Alien_Event_Mother_01`,
`Alien_Event_Landing_01`, `Unicorn_Event_Lander_01`; the atmospheres (`Tweak_Shadowlands_Atmosphere_SubSea`,
`_Lavaland`, `_Penumbra`, `Tweak_Rubi-Ka_Atmosphere`) for fog and under water. The chat says whether the script ran;
what does not show is an effect kind not drawn yet (start the client with `MAO_FXS=1` to see the kinds started).

## The camera while running, the login's window (2026-11-11)

Both were found by the user and fixed the same day. To look at again: running, at a high run speed above all, with
the camera behind and to the side: no shake, no judder. Entering the game from the character list: no login window
left behind the game's. Still open: what is wrong with the Shadowlands' sky.

## Compressed textures (2026-11-11)

On by default (the launcher's Visual page, "Compressed textures"). To look at while playing: the sky at dusk and at
night, glows and beams of nano effects, water and lava, faces close up: banding or blocks that the plain textures
(the box cleared) do not show. And whether a zone with many players stutters more as their textures come in.

## The faces turned away (2026-11-11)

Back faces are no longer drawn, as in the original. Seen without fault in Aegean, Arete Landing, its shuttle room
and the login hall. To look at while playing elsewhere: a wall, a floor, a piece of clothing or of a creature that is
missing or see-through where the original shows it, above all in mission dungeons, city interiors, shops, vehicles
and the Shadowlands. `MAO_CULL=show` paints what is left out magenta; `MAO_CULL=off` draws both sides as before.

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
