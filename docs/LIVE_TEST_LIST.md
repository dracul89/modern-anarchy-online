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
- [ ] The mouse on items (2026-11-12; by script: take up, put down, Escape, use, use on, a bag's three right
  clicks; later the same night also Shift+click in the inventory and in a bag, an item put into a bag and taken
  out of it by clicks, an item set on a hotbar slot by a click). Left to do by hand: a worn bag, a shop's list,
  the bank, a corpse, Shift+T. The whole list as it was: the pictures stand on their slots with no square behind them; a click takes an item up and a
  second click wears it (a wear cell), sets it on a hotbar slot, puts it into an open bag, sells it (a shop's
  list), uses it on something in the world; a right click on another item with one in hand combines the two (try
  both orders); Shift+click shows the info (inventory, wear window, a bag); a worn bag opens and closes by right
  clicks; in a bag a click takes an item up and a click on the inventory takes it out; in a corpse a click takes
  the item; Shift+T opens and closes the tradeskill window.
- [ ] Flight as the original has it (2026-11-13; `docs/FLIGHT_ORIGINAL.md`): a Ganimedes jetpack worn: the hover
  pose standing and moving, a half-metre rise, the right button's up and down pitching the flight and forward
  climbing along it, the flight's own speed; a Yalmaha worn: the craft in the character's place at a fifth of
  its size; a hoverboard no longer flies (its program has no fly spell), a hoverbike does. Tried by a forced
  flight on the test character only (it climbed at a set pitch); the pose seen in offline pictures.
- [ ] Placing by hand and stacks (2026-11-13; by script: a move to an empty cell, a swap, a split and a join):
  by hand: items stay where they are put after a relog; the same in a bag's window; Ctrl+click on a stack, the
  count box, and the split stack on an empty cell; a stack onto a like stack joins them.
- [ ] The special attacks (2026-11-15): Brawl, Burst, Aimed Shot, Sneak Attack and the rest go off at the target
  fought; pressed again while recharging the game says the skill is locked with the time left; the icon counts
  down on the hotbar and in the Actions window; "ready again" is said when it is; a key or slot for a special the
  weapon does not give says it is not available. Search says it is searching.
- [ ] Inside a city's buildings (2026-11-15): through a building's door the rooms are there (the silo, the
  headquarters), with their doors, and the way out leads back to the city. The log's "an owned building's inside ..."
  line is wanted for a test.
- [ ] The city's controller (2026-11-15): a click anywhere on it selects it ("City Controller" in the target bar); a
  double click uses it and its window opens: the upkeep tab shows the owner, the next bill's date, the upkeep and the
  charge; "Dump city advantages" lists them in the chat; the close cross closes it (a moment later: the server's
  answer does). The cloak's button shows the device's state. **Pressing it switches the cloak at once.** The log's
  "city: signal ..." lines are wanted for tests.
- [ ] Clicking past what stands in front (2026-11-15): at the shuttle in Tir County (1814, 2644), click the ring
  under the shuttle's door with an NPC before it: the first click takes the NPC, a second one in the same place (after
  half a second) the ring; a right click then uses the ring and the shuttle takes the character to ICC. A double click
  on a lone booth or NPC still uses it.
- [ ] Missions at ICC's neutral terminal (2026-11-15; at Old Athen's clan terminal the user's log of 16:33 has three requests with the fixed sliders, each answered): use the right one of the two (the individual's), Request
  Missions: offers come. The log has "missions: asked (0xdac1, ...) for kind 1 with [6, 0, 0, 0, 0, 0, 0]" (the
  sliders untouched) and the server's answer. (Tried by the user the same day with the first byte put right: no
  answer at ICC nor at Old Athen; the sliders' bytes were out of the server's range. Fixed after, not tried.) Then
  with sliders moved to both ends: offers come still, and the easy end gives lower missions than the hard end.
- [ ] A clicked offer on the planet map (2026-11-15): at a terminal with offers, click one: the chat says the
  waypoint is uploaded, the planet map (P) has a white dot at the mission's place (its Mission button centres on it),
  and in the mission's own playfield the compass has the yellow mark. Another offer moves the dot. Closing the window
  without accepting takes the dot away. Accept one: the window closes (and has no offers when a terminal is used
  again); when the mission is in the journal its place is uploaded by itself (the lines come again) and stays.
- [ ] The missions stack (2026-11-15): with one mission in the journal (Ctrl+4), accept a second at a terminal: both
  are listed, the chat says a new mission was got, and the new one's place is uploaded. Zone: both are still listed.
  Delete one: the other stays.
- [ ] A shop that only sells (2026-11-16): at a vendor paid in points (the veterans' vendor, an Ofab vendor, a
  Phasefront vendor, the ICC reward terminal: `shopkinds` lists them) the window has no "Sold items" list and an
  inventory item dropped on it stays where it was; buying works as elsewhere, **costs points, so only with the
  user's say-so**. A plain shop still has all three lists. Also to look at there: how the cost reads (it is a
  plain number; nothing says points).
- [ ] Mail with cash on delivery (2026-11-16; a paying account's test character writing to another test
  character): a number in C.O.D. asks before sending and the receiver sees it in the C.O.D. field; Take All asks
  before paying; Return To Sender sends the mail back and it leaves the list. A negative number in either money
  field is refused with the game's line. A mail with money sent along still shows it in Credits.
- [ ] The booths' names from their signs (2026-11-16): the Whom-Pah window's list at a station never visited
  (and the map's dots) names every booth's destination, the same as the sign over it; take one: it leads there,
  and the log has no "has a sign to ... and led to ..." line.
- [x] The equipment's lock (2026-11-16, the test character, scripted): `/setoption LockEquipment true`, then a
  helmet to the head: the game's line about locked equipment, nothing moved; `/setoption LockEquipment false`:
  the helmet went on. Not tried: taking something off, and a move between two backpack places with the lock on.
- [x] The mouse's turn messages (2026-11-16, the test character, the script's `face@` step): 10 as the
  steering starts to the right, 13 as it swings left, 14 as it is let go, nothing in between. The rest of the
  next item is still open.
- [ ] Steering with the mouse (2026-11-16): run with the right button held and swing the mouse left and right
  near a Whom-Pah cluster and along a wall: no rubber-banding, no zoning from beside a booth; another client
  sees the character turn smoothly. With the button held, the turn keys step sideways. The log's "sent move"
  lines show 10 and 13 as the mouse changes sides and 11 or 14 as the button is let go.
- [x] The chat server's commands (2026-11-16, the test character, scripted): `/cc addbuddy <name>` adds the
  friend and `/cc rembuddy <name>` takes it off, each with the server's line in chat and the friends list
  following (two unread packets found and read: 37 and 41); `/cc help` lists the commands; `/cc info <name>` gets
  no answer; `/rp`: the server sets stat 673 (543, then 31).
- [ ] Still open of those: the `Befriend` script file; the role-play mark as another character sees it;
  `/gfx disable <number>` of a running effect's template stops new ones of it, `enable` brings them back;
  `/bank close` shuts the bank's window. **`/name` is not to be tried on a character that is to keep its name.**
- [x] `/name` (2026-11-16, on the character the user named for it): `/name help`, `policy` and `status` are
  answered by the name service in tells; `/name nick <name>` was taken into its queue ("may take up to 24 hours
  ... changed while you are offline"). The first try sent one word and was refused: fixed (the whole of what
  follows the command goes). Not seen: the name changed afterwards, and `/name delete all`.
- [x] `/org leave` on a character in no organization (2026-11-16, the test character): the warning box, Yes
  sends the command and the server says the character is in none.
- [ ] The organization's boxes (2026-11-16), **only in an organization made for testing, with test characters**:
  `/org leave` asks in a warning box (No sends nothing, Yes leaves); `/org disband` asks with the game's text; an
  invitation from another character comes as a box naming the organization and its tax; a promotion asks the
  leader to confirm. The log has "organization box: ..." and "organization question from the server: ...".
- [ ] The Jobe Whom-Pah without Shadowlands (2026-11-15): a free account's character walks into the booth to Jobe
  (Old Athen's southern one, ICC's eastern one): an error box says the Shadowlands expansion is needed, once a touch;
  the log has "the booth to Jobe: the account's expansions (stat 389) are ...". A character with Shadowlands gets
  no box and zones. The other booths: no box.
- [ ] No take-off on a refusal (2026-11-15): use a flying item the character cannot wear (the jetpack under the
  skill): the server's line comes, the item goes back, and the character never leaves the ground. Put a Yalmaha on
  and take it off: it lifts and lands as before, a moment after the click.
- [ ] The name at a run (2026-11-15): select yourself and run, turn, jump: the name and its brackets stay over the
  head without shaking and can be read; others' names do not bob as they run either.
- [ ] An item of several actions (2026-11-15): the Ganimedes Personal Jetpack's window (and any item that had no
  "To wield" or "To wear" lines before) lists what it asks, the unmet in red: Vehicle Navigation, Airborne 81 among
  them.
- [ ] Turning and the teleporters (2026-11-15): run and jump round a Whom-Pah cluster (ICC's four) turning with the
  keys, close by but never in a booth: no zoning. Walk into one: it zones as before. A second character watching
  sees the first turn and stop turning as it does (no spinning, no walking in circles).
- [ ] Dying (2026-11-15): die: the character falls, the world goes white over three seconds once it lies, and the
  server sets it down at the place of return right after (seconds, not a minute); the log has "the death has played
  out: telling the server".
- [ ] The planet maps (2026-11-15): the planet map's list (P) has Saavick's, Saavick's 2x, the game's default,
  Dovve's and the Atlas of Rubi-Ka v1.3.1; in the Shadowlands SLmap v2.22 Normal and Wide and the game's. With the
  Atlas chosen: its four levels by zoom, and the arrow is where the character stands (try a place the map names: a
  Whom-Pah, a grid terminal, a bridge) in a few playfields, a town among them.
- [ ] Leaving a Yalmaha (2026-11-15): put it on, fly through a zone border or a door, take it off: the character has
  its size again. In the air the craft is faster than running was by its 300 `RunSpeed` (about one unit a second), and
  running on foot is faster than in earlier builds (worn items, programs and the attributes count now); the server
  does not pull the character back while running. Then log out on the craft, log in and take it off: is the character
  small? (Not known what the original does; the log's "own stat 360 from the server" lines will tell.)
- [ ] The org headquarters' inside (2026-11-15): the lobby has its red carpet, chequered floor and the fish tanks
  behind glass at both sides, and the tunnel's arches stand whole; the bar's bottles stand on the shelves and its
  curtains hang at the booths; walking into either elevator in the hall puts the character in the tactical room or
  the laboratory and it stays there, and the elevator there brings it back; each door opens as the character comes to
  it (the right one); logging out and in again inside the building shows the building.
- [ ] Scenery elsewhere after the same change (2026-11-15): about 1,200 scenery meshes moved to where their stored
  matrix puts them (bottles, grass tufts, pipes, rubble, poles). Anything that now floats or stands beside where it
  should be is worth a screenshot.
- [ ] The controller's map (2026-11-15), looking only: the Build and Demolish tabs show the city from above with its
  plots (blue), the houses where they stand (yellow) and their door dots; north is up; a drag scrolls; a click on a
  house selects it. **"Demolish Building" and "Construct Buildings" act at once and cannot be undone.** A building
  item let go on the map (Build tab) shows green on a free plot and red elsewhere; "Rotate" and "Delete" work on it.
- [ ] The Grid's "Organisation City" and "Notum Controller" exits (2026-11-15): a window titled "Vehicle" lists the
  places (the city, the tower fields) with their playfields; "Go!" with a row selected takes the character there;
  "Close" does nothing more. The log's "grid: the destinations' list ..." line is wanted for a test.
- [ ] A corpse looted with a full inventory (2026-11-15): the game says the inventory is full and the item stays in
  the corpse. A looted stack joins the stack of the same item.
- [ ] Walking and running on hillsides (2026-11-15): no sudden throw forward (Shadowlands, playfield 4310, the slope
  north of the lake near 807, 1171).
- [ ] Jobe Harbor (2026-11-15): a ring hangs under the "Jobe Platform" sign and pulses when walked under; the same
  under the other signs.
- [ ] A mail's item taken (2026-11-15): the item's picture leaves the mail, Cash shows 0, Take All goes off and
  Delete comes on.
- [ ] The overflow (2026-11-15): with a full inventory, something given (a mission reward, a claim) opens a window
  titled "Overflow" with the game's line about it; after room is made a click on the item brings it in; nothing can
  be worn while it holds something.
- [ ] ICC's landing platforms (Andromeda, 2026-11-15): the bridge pieces between the platforms lie flat and can be
  walked. The clan shops' tradeskill part: dark triangular walls beside the ramps, no slabs lying about.
- [ ] A player city (2026-11-15): its buildings stand on their plots, facing as in the original, each door the
  server shows in a doorway; the walls stop the character. The log's "city: house ..." lines name each door's place.
- [ ] A city's controller (2026-11-15): the band under its head is solid dark machinery.
- [ ] A daily reward claimed (2026-11-15): the item comes into the inventory and no second window with the
  inventory's items opens; the log says "items: the server's list of the character's own items".
- [ ] The inventory, from the original's code (2026-11-13): a right click on armour, a weapon or an implant in the
  backpack wears it and on a worn one takes it off; on a consumable or a bag it uses or opens; the menu comes by
  the right button held about a third of a second (Look At, Create Reference, Delete); a bag picked up closes its
  window. A Ganimedes jetpack worn puts the character into flight.
- [ ] The camera (2026-11-13): `/setoption camera_mode free` and how it feels against `rubber` and `lock` at
  turning, running and jumping; whether the jitter is still there in the free camera.
- [ ] From the session of 2026-11-13 morning: the Male Watchers of Old Athen wear their helmets and hold their
  cannons (and other NPCs their tools, the right way round); an item sold to a shop is gone from the inventory as
  the trade completes; a shop's window closes about 8 units from where it was opened; `game-log.txt` begins with
  "message log: on" and holds "server n3" lines.
- [ ] The user's list of 2026-11-13: a loot line in the chat reads as a sentence (not `~&!!!...`); the right
  button held on an inventory item opens its menu (Look At, Use, Create Reference, Delete), and Create Reference
  puts the item's link into the chat's line, which others can click once it is sent; `game-log.txt` appears beside
  the settings after a game. Open, each wanting a log: a shop sale, the shop window at walking away, a Yalmaha or
  Ganimedes equipped, the camera's jitter at running and jumping.
- [ ] A screenshot's notes (2026-11-13): a screenshot in the game leaves a `.txt` of the same name beside the
  picture, with the selected target's look first. Wanted: one of the Male Watcher in Jobe, selected (it is drawn
  without its gear; the cause is not known yet).
- [ ] The market's search (2026-11-13): the name field is a field (not three dots), a name typed into it finds
  items; the Advanced fields (quality, price) are wide enough to read; the page opens on a second and a third
  login without "403 Invalid session".
- [ ] The statels' own effects (2026-11-12): campfires and bonfires burn, chimneys and vents smoke, plain halos
  glow (Newland Desert, Andromeda, Arete's camps); the frame rate near many of them; nothing odd where a red halo
  now stands (a name the original's table lacks gets one). Lamps whose halo has a colour of its own (Jobe) still
  have none.
- [ ] The helpers named "invis" (2026-11-12): no light-blue slabs under the Market Search terminals of Rome Blue,
  Rome Green and West Athens, no white floors or coloured blocks in the ACG and Temple of Three Winds dungeons, no
  grey pyramid in a campfire or red cross where a halo belongs (seen gone in offline pictures of Rome Blue only).
  Walking is not changed by it: what blocks a walker is the playfield's collision surfaces.
- [ ] The occluder's bodies (2026-11-12): no pink or purple see-through boxes in Jobe Research, Nascense Frontier,
  South of Elysium, the ICC Shuttleport and Adonis City (seen gone in offline pictures of Jobe Research only).
- [ ] The launcher's own background (2026-11-12): in its settings, GUI page, Launcher group, choose "Stars and the
  steel emblem" and OK: the sky comes at once, the emblem small at the lower left under the page; untick the news
  page: the emblem large in the middle; a few stars change their brightness; "A picture of your own" with a file;
  back to "The game's pictures". PLAY works the same with each.
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

## Macros, scripts, the chat filter (2026-11-12)

Seen in a scripted run (the chat log): `/macro Hello /text the macro ran` said the macro was on the pointer, a
click on an empty hotbar slot set it down, a second click ran it; `/script nosuchscript` said "Unable to find
script"; `/aou_about` (a script file of the original's folder) ran without a word; `/filter add`, `list`,
`clear`; `/dvalue AutoAFK` shown, set, and a name that is not there. Left for a person:

- `/macro Assist /assist`: the name rides beside the pointer; set it on a slot; the slot shows "Assist" in yellow;
  its tip says the line; it is still there after a restart. Clicking somewhere else instead drops the macro
  nowhere (it is still saved, but on no bar).
- A script of your own in `%APPDATA%/ModernAnarchyOnline/scripts` with `/delay 1000` between two `/text` lines:
  a second between them. With `%1` in a line and a word after the command: the word in its place.
- Typing `/` and the first letters of a script file's name: it is in the list.
- `/filter add ^WTS`, `/filter enable`: lines of a trade channel that begin so are gone; tells are not.
  `/filter disable`: back.
- `/start https://www.anarchy-online.com`: the system's browser opens it.
- `/char&viewdist 100 800`.

## Commands taken out on 2026-11-12, and put back the same day

These were first written to go to the server as `/stuck` does, tried once (`/played`), refused by the server ("You
are not authorized to perform that command. (Your request has been logged)") and taken out. They are Gamecode's
text commands (`N3Msg_TextCommand` 0x10017743, its FUN_1003fe10), each a `CharacterAction` of its own
(`mao_net::n3::text_command`), and were put back that way and tried on the test character Maotesttwo:

- [x] `/played`: its time and date line, then the server's "<name> has played 0 days 4 hours 20 minutes 50
  seconds" (2026-11-12).
- [x] `/born`: the server's "<name> was born on Wed Nov 11 02:04:38 2026" (2026-11-12).
- [x] `/team` and `/team loot` outside a team: "You are not a member of a team!", nothing sent (2026-11-12).
- [ ] `/team` alone in a team: the three lines of help. `/team loot` in a team: the server's answer with the loot
  order. `/team loot all`, `leader`, `alpha` as the team's leader: the order changed and told to the team; as
  another member: "Only team leader can change loot order!", nothing sent. (Needs two characters in a team.)
- [ ] `/bank info`: sent on 2026-11-12 away from a bank, not refused, and nothing was shown in answer. To try with
  a bank open, and to find what the answer is (it may be a message the client does not show).
- [ ] `/tower <word>`: still not carried out (Notum Wars: `create`, and `terminate` with a tower selected, are
  Fanatic's own; the other words are another of Gamecode's text commands, not read yet). `/terminate` with its
  yes/no box the same.

- [x] `/org info`, `/org ranks`, `/org contract` outside an organization: "You are not a member of an
  organization." and "You're not in an organization." (2026-11-12).
- [x] `/org ranks` in an organization: "Governing form: ... Ranks: ..." (2026-11-12).
- [x] `/org info` and `/org contract` in an organization: the answers are read (2026-11-12, by the log: the
  organization's name, no areas, a page made; the contract with no items). Left for a person: the info window
  comes up with the page; the container window comes up titled "Organization contract" and empty.
- [x] `/org info` of an organization that controls areas, and `/org contract` of one whose contract has items
  (2026-11-12, by the log: two areas with their playfields, names, levels and types; three items). Left for a
  person: the areas at the page's end; the three items in the container window with their pictures (a click on
  them does nothing in this client).
- [ ] `/org info` with another member, or a member of another organization, selected.
- [ ] Every other `/org` command (`invite`, `kick`, `promote`, `demote`, `bank`, `tax`, the votes, `debt`, `city`,
  and the ones that set a text: `name`, `description`, `objective`, `history`, `governingform`): never sent
  except `history` once, by mistake (below). **To be tried only in an organization made for testing**, not in the
  player's own.
- [ ] `/items`: sent on 2026-11-12, not refused, nothing shown in answer.

**A command is not passed to the server before its handler is read down to the message it sends. And a command
is not tried on a character of the player's own before what it does is known: `/org history` was sent on
2026-11-12 in the belief that it only read, and it sets the organization's history (empty before and after, so
nothing was lost). The client sends such a command with an empty text when none is typed, as the original does:
`/org history`, `description`, `objective`, `name` or `governingform` alone empties that text.**

Also not carried out, and to be tried live when they are (none was ever sent): `/cc info <name>` and `/cc addbuddy
<name>` (the original's `Befriend` script file is such a line), `/rp` (seen by another character: the role-playing
mark), `/reclaim` (its window, at a reclaim terminal), `/gfx <n> on|off`. `/name <request>` asks the name service
to rename the character (sent live 2026-11-16, see above): not to be tried on a character that is to keep its
name.

## Away from the keyboard (2026-11-12)

Seen in a scripted run with two characters (the chat log of both): the lines of `/afk <text>`, `/afk status` and
`/afk`, "is afk" and "is back" in the other's vicinity chat, a tell marked "(AFK reply sent)", the reply in the
sender's chat, no reply to a reply, going away after the (shortened) time without input, coming back when the
mouse moved; and the box's keys (a text and Enter, Escape, the count's end) by their lines. Left for a person, who
can see the box:

- `/afk` alone: the box in the middle of the screen with the count running down from 30; typing stops the count;
  Enter or Ok takes the text ("Changed AFK message from ... to ..."); with the field emptied, "Using default AFK
  message."; Escape, "AFK off.". Left alone for 30 seconds it goes with the default message.
- While the box is up, moving the mouse does not end being away.
- With `AutoAFK` on (the default): `/afk brb`, hands off the mouse and keyboard, have someone send a tell (the
  reply names the hours and minutes); touch the mouse: "AFK off.".
- `/setoption AutoAFK off`, `/afk brb`, move about: still away until `/afk`; and three minutes of nothing does not
  set "Auto-away".
- Three minutes without touching anything (the real time, not the trial's): "is afk" for those around, and a tell
  is answered with "Auto-away".
- An away reply from someone using the original client: shown in their tell window without their name in front.
- A tell window open with the character: the marked tell and the reply both show in it.

## The tradeskill guide (2026-11-12)

- `/crafting nano crystal`: after about two seconds the list, the book on creating nano crystals first; its page
  reads as a text with its paragraphs. "Books and manuals only" with the box empty lists all 416.
- A page of an item of a chain (an instruction disc of some nano program): the stated step, the books that speak of
  its kind, the others of the same name in brackets; each name opens its page, Back returns.
- A thing in one of your bags or the bank (after `/find` has seen it): its name is green in the guide, the hover
  says which character and where.
- The tradeskill window (Shift and a right click on an item onto another, or the window itself) with two items that
  combine: afterwards the pages of both show "What the tradeskill window told you" with the result, its qualities
  and the skills; `tradeskills.txt` appears beside `mao.ini`. A pair you lack the skill for: the requirement is
  written down too.
- `/open assistant` and a crafting question: the results say "the game's crafting texts".

## The help window's own model (2026-11-12)

- `/open assistant`, "Sources and the model", the tick for the small model, "Fetch the model (1117 MB, once)": the
  megabytes count up; Stop and the same button again goes on from there; at the end "The model's files are here".
- A question: "The model is reading the results..." for a few seconds, then the answer growing word by word above
  the results. The game should stay smooth meanwhile; say how it feels on a machine with fewer cores.
- A question the help does not answer (a weapon's damage formula): "The sources do not say."
- Five minutes without a question: the client's memory goes down by about a gigabyte again.

## The mission helper (2026-11-12): needs a hand, it costs credits

- At a mission terminal, Request Missions: each offer in a line under the button (rewards, place by name, credits).
- "Looking for": a part of a reward's name that one of the offers has: that offer's line and box turn green.
- "Keep asking until one fits" with a part that is not on offer and a limit of 3: three requests about a second
  apart, then "Nothing fitting in 3 requests"; the credits go down by three times the level.
- The same with a part that does come up: it stops on the fitting offer and selects it.
- "Stop asking" in the middle, and closing the window in the middle: no more requests.

## The item finder (2026-11-12)

- `/find kit` on a character: its kits with where they are; then the same from another character of the account:
  the first one's things are still found, with "seen ... ago".
- "Look into my bags": no bag window opens, the button's number goes to nothing, and the bags' things are found.
  Then a bag opened by hand still gets its window.
- A bank terminal used once: the bank's things are found afterwards, also from another character.
- A thing moved from the inventory into a bag or the bank: found at its new place within a few seconds.
- A click on a row: the item's description.

## The damage meter (2026-11-12)

- `/damage` in a fight: one's own row grows with each hit; the figures a second look right against the fight's
  length; a critical hit and a miss show in the row's tip.
- With a pet out (an engineer's): its damage under one's own row with "Pets with their owners" ticked, and its own
  row without.
- In a team or beside other players: their rows, and whether what they do with nano programs is counted (a nano's
  damage comes as a health message with the program's name).
- A second fight after a pause: "Latest fight" changes, the first stays in the list; "All fights" adds them.
- Copy, then paste into a chat line: the rows as text.

## Mission terminals (2026-11-12)

- A team mission terminal while in no team: an error box ("You need to be in a team ..."), no mission window.
  (Tried by script; worth one look by hand for where the box comes up and how it reads.)
- The same terminal while in a team: the window opens, and Request Missions brings a team's missions.
- A neutral or an Omni-Tek terminal (Borealis, Omni-1): Request Missions brings missions (the request now names
  the terminal's own kind; before, only clan terminals could answer).
- With fewer credits than the character's level: Request Missions says so in an error box.

## A wrong password, the loading screen after Play (2026-11-12)

- A wrong password: a Browser window over the login window with the site's page ("Please try to re-enter your
  username and password"), the login window its usual size under it; its links open the system's browser; the
  window's close box puts it away; the right password then logs in and the page is gone.
- No connection (the network off): the page of error 1, or the words if the page cannot be had either.
- Play: the loading screen at once, in the same window as it seems (borderless), until the world shows through its
  fade; no desktop in between. In a plain window: the game's window where the list's was.
- The exclusive full-screen mode, if used: what the change between the two windows looks like.
- A new character made and played: the welcome picture's turn as before.

## Doors among rooms (2026-11-11)

- A shop: each doorway has a dark sliding door; it opens as the character comes up to it (about 4 m), the room behind
  shows and can be walked into; it shuts again some seconds after.
- Which side the door's keypad is on, against the original.
- A mission's doors: an unlocked one opens at a touch now; a locked one stays shut until it is opened as before.
- A city's building doors and the Whom-Pahs': open on approach as before.

## Helmets, interiors, running with a rifle (2026-11-11)

- A character with a helmet on: no hair through it, no second head; with glasses: the head and the glasses.
- Taking the helmet off and putting it on again: the head comes back and goes.
- Any interior (the Fair Trade shop, a flat, a mission): walls and ceilings there from inside, in every room. The walls behind the shops' terminal rows are whole (they were holes into the next room).
- Running and walking with a rifle in hand: the stride goes on without stopping; stopping and starting look right.

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
| 48 | 1 | Open the Action window on Humility and on a character with a gun that has burst or fling shot | Dimach and Brawl are there as in the original; the gun's attacks are the original's rule (traced 2026-11-18: each weapon's stat 30 bits, `WeaponItem_t` FUN_1009e84b) |
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

## The help assistant, "Ask the guides" (2026-11-12, branch `agent/help-assistant`)

- `/open assistant` and the Help menu's "Ask the guides" open the window; the field takes the keyboard (the
  character does not walk while typing); Enter and Ask both ask.
- A question the game's help covers ("how does insurance work"): numbered results from the game's help, the right
  page first; a click on a title shows the whole piece and again the short one.
- Under "Sources and the model": the game's help can be turned off and on (the results change); AO-Universe's
  checkbox shows its terms; turned on, "Read its guides now" counts pages up slowly, Stop stops it, the pages read so
  far are searched afterwards and are still there after a restart (`help/ao-universe.jsonl` beside `mao.ini`).
  A second reading fetches nothing that has not changed.
- "open the page" on an AO-Universe result opens the guide in the system's browser.
- `mao.ini` keeps the `[help]` section (sources as ticked) across a start.
- With a model server on the machine and `endpoint` / `model` set under `[help]`: "Asking the model...", then an
  answer above the results that names its sources by number; with a wrong address, the reason in red and the
  results still there. Which small model answers well is for trying.
- The game does not hitch when the window opens or while a site is being read.
- [ ] Character creation in the original's scenes (2026-11-16), from New Character on the character list: the
  intro in space with its music and two lines of text (Escape ends it, and Escape again the way down); the breed
  scene: the mouse over a figure names it, a click chooses it (its voice line, it steps to the front left), Next;
  appearance: the head's arrows, the three heights and builds, the camera at the face while the mouse is at the
  head, Back and Next; profession: a button flies the camera to that profession's figure and plays its line; name:
  Suggest Name, a bad name's text, the Rubi-Ka button makes the character, then the walk to the shuttle and the
  game. The exit button asks first; after No nothing changed, after Yes the list is back and New Character has the
  choices still. To judge by ear: the sounds' loudness, and whether the ambience keeps running.
- [ ] Inspecting a player (2026-11-16): target a player, `/inspect` (the info page), click "Inspect Equipment"
  at its end: the Inspect window with what the player wears, the player's name in its title. (The command behind
  the link was tried live; the click was not.)
- [ ] Keeper and Shade on a free account (2026-11-16): in the profession scene the two buttons are dark and do
  nothing; on the paid account they work. The launcher's log has "account: N character slots, expansion flags".
- [ ] A new character's first arrival (2026-11-16): make a character, and in the first session, without a relog:
  the survival pack shows in the inventory (fifth), right click on the nano crystals uploads the program (it is
  in the nano window at once), casting it works, the belt then goes on, the weapon goes to the hand. Before the
  fix all of these went to the wrong item until a relog.
- [ ] The character list's slots line (2026-11-16): after a login it reads as the original's, slots left of
  slots in all (the free test account: 8 in all); New Character is dark when none is left.
- [ ] Quest hand-ins on a new character (2026-11-16): the thing given to the NPC leaves the inventory when the
  trade is taken; the next reward lands where the freed place is; Alex Gibbs takes the Bio Analyzing Computer;
  the recharger and the stims show their numbers from the first arrival on and count down with each use.
- [ ] Helmets' faces (2026-11-16): Flint Novak and the dockworkers in Arete Landing have a face under the mining
  helmet, not a blue patch; a player's helmet with an open face likewise.
- [ ] The Junk Shop (2026-11-16): in Arete Landing beside the ICC Ammunition machine near Alex Gibbs, a small
  vending machine named "Junk Shop" stands the right way up and opens as a shop.
- [ ] The wounded lie (2026-11-16): Arete Landing's Wounded Dockworkers and Wounded Clan Protesters lie on the
  ground; the standing dockworkers and Flint Novak stand; a seated NPC (the ICC Secretary) sits.
- [ ] Still open from the user's list of 2026-11-16: bounding boxes (which ones was not said).
- [ ] Lists keep widths and sort (2026-11-18): in the NCU window's list mode drag a column wider and press a label
  to sort; close the game and start it again: the width and the sort are as left, and the icon grid is still in
  the order of the time left. The same for the nano programs' list and the mission journal.
- [ ] Locks (2026-11-18): use a perk action: its tile in the Actions window fades for its lock and a click on it
  says "Unable to perform action, ... perk is locked" with the time; when it is over the game's line says so.
  Click a special attack's tile while its skill is locked: the wait line. Open and close the daily rewards'
  window twice within a minute: the second close says the wait line and asks nothing.
- [ ] Missions and level-up (2026-11-18): a terminal that has no missions: the line is in the chat and the six
  boxes are empty. At a level-up the experience line comes before "You gained a new level"; at a title level (15,
  50 ...) a line with the new title: **read from the code only, tell me what the game says there**. The factions
  window opens with titles, a click shows numbers, and the choice is there after a restart.
- [ ] Flight (2026-11-18): flying, press the sit key, the walk/run key, and click the Sneak tile: each says "Action
  is not available." and nothing changes; `/camp` says you must be sitting. Sit down and switch a flight item on:
  the character stands and flies. At Arete Landing a right click on the jetpack says flying must be allowed on
  the playfield and nothing moves; in Borealis or Andromeda it is worn. Sneak: the steps are much quieter.
- [ ] The belt's deck (2026-11-18): with two deck places, a third NCU unit is refused with "This item requires a
  computer deck" (the game's line) and nothing moves; a right click on the belt, or on another belt, with units
  in the deck says the deck must be empty and nothing moves. **Tell me if a unit is refused that the game takes.**
- [ ] Small ones (2026-11-18): move the mission window's sliders, restart: they are as left. In exclusive full
  screen `/start https://...` asks first; in a window it opens at once.
- [ ] An item between two templates (2026-11-18): the mission belt's page (quality 30) says 3 deck places; compare
  a few such items' pages with the game's (value, range, price).
- [x] Action tiles, tried live the same night on Maotesttwo (pictures of the client's own window): the Stand tile
  sat the character down; the Suspended Animation tile started the timed logout ("Timed logout started", the bar)
  and a second click stopped it ("Timed logout aborted": the server answered the 0x79). The Faction window opened
  with titles. Still to try: Use, Start/End Combat, Walk/Run, and Brawl with a pistol.
- [ ] Action tiles (2026-11-18): click Use with a target, Start Combat (and again to stop), Stand (sits, then
  stands), Walk (the pace changes), Suspended Animation (the logout bar starts; again: it stops). With only a pistol
  in the hands click Brawl: "unavailable"; with a melee weapon or bare hands it goes out.
- [ ] Doors' sounds (2026-11-18): walk through Arete Landing's doors and a shop's: each opens and closes with its
  sound (some doors are silent in the game too). Tell me if a door chatters.
- [ ] Stacks in a backpack (2026-11-18): in an open backpack drop a stack on a like stack: one stack, the sum.
  Ctrl+click a stack, give a number, click an empty cell: two stacks. Then close and open the bag again and relog:
  the stacks are where they were shown (if not, tell me: the server's place for the new stack is a reading).
- [ ] Other players' sizes (2026-11-18): in a crowd the players are of three heights, and none walks through the
  floor or hangs above it; a player on a Yalmaha is not shrunk to a fifth.
- [ ] The item timer (2026-11-18): use a stim: the bar starts a moment after the click and ends as the stim
  works. A use the game refuses shows no bar.
- [ ] `/raid` (2026-11-18, needs a team of your own two characters; `create` makes the team a raid for both):
  `/raid locks` says the lock list or that there is none; `/raid listlocal` says the team; `/raid create` as the
  leader says the raid was made. Send me the log: no raid message was ever sent to the server before.
- [ ] Keys from the original (2026-11-18): rebind a key in the original client for a character, log that
  character in here: the Keys list shows it and the log has a "keys:" line.
- [ ] The five default keys (2026-11-18): Backspace switches walk and run; in Options, Keys: Sneak Attack J, Fast
  Attack N, Aimed Shot O, Bow Special Attack Period, the map overlay on Ctrl+P, and any key you had changed
  yourself as you left it.
- [ ] Others on crafts (2026-11-18): where players fly Yalmahas (a city, Borealis): they are small crafts in the
  air, not people standing on the ground; one who lands and takes it off is a person again (that half was never
  seen on the wire: tell me if a craft keeps walking about). Sitting players sit.
- [ ] The raid window (2026-11-18, your own two characters in a team): `/open raid`: empty teams, "Convert To
  Raid" enabled; as the leader click it: "raid" line, Team 1 fills with both; "Raid Locks" says the lock list. Zone
  with the team: the perks window still has its perks.
- [ ] Program times (2026-11-18): cast a long buff on yourself: the NCU window shows its full time (hours, not
  minutes) and it is still there, with its bonus in the stats, half an hour later. Have another player (your
  other character) buff you: it appears in the NCU window. Watch somebody's polymorph or hoverbike end: the form
  goes at once.
- [ ] The container item menu (2026-11-18): hold the right button on an item in a backpack: the three entries;
  Look at Item shows it; Delete Item on a **junk** item asks, and on Yes the item goes (a capture has this one).
  The bank's and a corpse's delete and the corpse's Looter entries (in a team) were never on the wire: try the
  Looter one with your second character and send me the log.
- [ ] Death reasons (2026-11-18): die to a nano program or a damage shield: the line says so.
- [ ] The pet window (2026-11-18): with a pet out and buffed, its programs are a row of small icons under its
  bar, fading as they run out; none when it has none. `/pet rename Rex` with the pet targeted: the name changes
  in the window (if not, the server's name message is not what the code says: send me the log). Call the pet
  again: it is named Rex by itself, and the log has the "pet ...: command 15" line.
- [ ] Popup menus (2026-11-18): right-click a chat window's tab: Visual, Talk to Channel and Subscribe Channels open
  sub-menus beside the menu when rested on; with a borderless window, Visual > Transparency has two sliders and the
  window follows the knob; typing the first letters of an entry highlights it. On a corpse in a team, hold the right
  button on an item: Looter opens a sub-menu.
- [ ] Options (2026-11-18): Preferences: fog Off leaves a thin fog (not none); the helmet and shoulder pad boxes
  can be ticked one by one and stay as set after closing the window; Escape closes the raid, team search and city
  terminal windows and an open backpack.
- [ ] Direct3D 12 (2026-11-18): on the launcher's Visual page choose the device's "(Dx12)" entry, restart: the
  launcher stays and shows its page, a wrong password shows the error page, and the game draws (its log's first
  lines name the device). Then choose your own device again.
- [ ] Weather (2026-11-18): in rain (you hear it) thin blue-grey streaks fall round you, straight down whichever way
  you look; in a sand or toxic storm short coloured streaks are blown past and the screen is tinted and flickers.
  If the streaks hang sideways or sit behind you, tell me. `MAO_WEATHER=0:0.9,8:5`
  (an environment variable at start) forces rain.
- [ ] City map (2026-11-18): rest the pointer on your city's map: a tip with its coordinates. Put a building item
  on the map (do NOT press Construct): the item shows a padlock in the inventory and cannot be picked up; Delete
  on the map takes the padlock off.
- [ ] Buying dialog (2026-11-18), only if you ever use the controller of land that is for sale: the window with
  the price shows. **Do not press Purchase unless you mean to buy**; Cancel closes it. Send me the log either way
  (the opening's numbers were never seen).
- [ ] Item window menus (2026-11-18): right click beside the items in the inventory: List Mode gives a list (name,
  count, quality; click a header to sort); Auto arrange > By Name packs the grid in order, Don't Auto Arrange
  lets you place by hand again with nothing moved. **Try letting an item go in list mode and in an arranged
  grid**: it must stay where it was (no move), stacks must still join, and a Ctrl+click split must land in a free
  place. Same in a backpack. On a corpse the items are sorted by name. In the wear window, right click an empty
  slot: Labels On Top.
- [ ] Lamp halos (2026-11-18): in Jobe Research (and Andromeda) the lamps have a glow, white or coloured, at dusk
  and at night. In a mission building, vents smoke and lamps glow inside the rooms.
- [ ] Corpses found lying (2026-11-18): walk up to corpses somebody else left (or leave your own kill out of sight
  and come back): a body lies there, lootable as before. Tell me if one stands upright or floats.
- [ ] Pictures in text (2026-11-18): talk to an NPC: each answer has a small orange bullet before it. Open a mission
  with item rewards in the mission window: the rewards show as icons you can click for the item's page.
- [ ] Locking worn items (2026-11-18): hold the right button on a worn item, choose Lock Item: a small lock shows on
  it. Try to take it off, to right-click it, to wear something else in its place: nothing happens or "Equipment
  tabs are locked." Unlock Item frees it; Unlock All Items is in the wear window's menu (the i button at the left
  of its title). Relog: the lock is still there.
- [ ] The bank by hand (2026-11-18): open the bank: items stay where they are, and one can be moved to an empty cell
  or change cells with another. Put an inventory item on an empty bank cell: it should stand there. Tell me if an
  item jumps elsewhere or two end up on one cell.
- [ ] The frame's menu button (2026-11-18): the small "i" at the left of a window's title: the menu under it, Close
  Window closes the window. Tell me of any window whose title is cut or overlapped by it.
- [ ] Locked doors (2026-11-18): the key doors of Will To Fight or The Crypt of Home with the key item on you, a
  mission's locked door with its key, and the doors inside an apartment: they should open as you walk up. Tell me
  of a door that opens here and the server then pulls you back from, or one the original opens and this does not;
  the log says "door ... locked, key ..." when the server locks one.
- [ ] The battlestation's signup terminal (2026-11-18): use one: the window should come up with your place in the
  queue. Sign up with a side, leave the queue. Send me the log either way: it says "battlestation queue: ..." for
  what was asked and for the server's answer; nothing of this was ever captured.
- [ ] The vehicles window (2026-11-18): Shift+V outdoors: the window lists the vehicles your side's global research
  has finished (greyed where you lack the requirements). Choose one: its description shows. Order it if you like,
  and Leave vehicle after. Send me the log either way ("vehicles: Spawn(...)" and whatever the server says next):
  nothing of this was ever captured.
- [ ] The character list's figure (2026-11-18): at the character list, watch the picked character for half a
  minute: it should move (breathing) and now and then play a gesture. Tell me if it jerks, slides or freezes.
- [ ] Options that act now (2026-11-18): in the options, turn "Zoom in and out of first person" off: zooming in
  stops just behind the character. Set the mouse wheel to "No effect": the wheel does nothing over the world. In
  first person, look down: no body unless "Show my character in first person view" is on. Zoom right up to the
  character in third person: it turns slightly see-through. Tell me what looks wrong.
- [ ] Dying (2026-11-18): let Maotesttwo die: for the seconds the character lies there, keys, mouse and typing do
  nothing and the pointer is gone; all of it comes back when the character stands at the reclaim. **Tell me at
  once if it does not come back** (the window's close button still works).
- [ ] The tower build menu (2026-11-18): with the Notum Wars expansion, the map's Build button opens the menu;
  right-click a tower or controller item: the menu comes up with it and its checklist. **Send me the log
  whatever happens** ("notum: checklist asked for ..." and the answer): the exchange was never captured. Do not
  press Build on land you do not mean to build on: it uses the item at once.
- [ ] The battlestation (2026-11-18): after signing up, the invitation box, the small map at the top with the
  points and scores, the end's box. Send me the log: none of its messages was ever captured.
- [ ] Character creation (2026-11-18): the intro's shuttle flies, the stars and suns drift; on the appearance
  scene the character moves, and its feet are on the floor at each build and height. With a Shadowlands account
  the head arrows go through more heads than before.
- [ ] A shop's clicks (2026-11-18, Maotesttwo: it costs credits): one click on a stock row adds one to "Bought
  items", a second click a second one; a click on a bought row takes it out; Shift + click opens the page; the
  right button held for a moment and let go does nothing. Ctrl + right click on a backpack item puts it under
  "Sold items"; a plain right click still uses it.
- [ ] A shop left open (2026-11-18): open a shop, walk 20 m off: the window stays; a click on a stock row there
  brings the server's "too far away" line and nothing else; Decline closes it. Send me the log if the server
  does anything else.
- [ ] Items by clicks (2026-11-18): press on a backpack item, move to another cell, let go: nothing happens. A
  click takes it up and a second click puts it down, in the inventory, the wear window, a bag and the bank; a
  click on a corpse's item still loots it.
- [ ] Combining by Shift (2026-11-18): take a tradeskill piece up, Shift + right click its partner: the combine
  goes through as before. A plain right click instead: send me the log of what the server says (the use
  message's action 5 between two backpack items was never tried here).
- [ ] An item over the ground (2026-11-18, Maotesttwo): take a cheap item up and click the ground near the
  character: the box offers to delete it; Cancel leaves it; far away the box says it cannot be dropped there.
  On a character: send me the log line of the use (action 0x20). A piece of furniture, if one is at hand, is
  asked about and dropped: **send me the log whatever happens**, the drop's message was never captured.
- [ ] The mouse in the world (2026-11-18): a terminal is clicked on its screen and not beside it; an NPC behind a wall
  cannot be clicked; a click through one's own body selects oneself, the next what stands behind. Right click on an
  NPC talks to it and leaves the target. Double click on a terminal uses it. Shift + click shows the page. **Ctrl +
  click on a second mob in a fight: send me the log** (the attack's flag 2 was never captured).
- [ ] Worn items' bonuses (2026-11-18): compare the Skills window with the original on an item whose bonus depends on the
  profession or a skill. Tell me of any number that changed for the worse.
- [ ] Doors (2026-11-18): walk up to a shop's door: it opens as you reach it and shuts behind you at once. Walk through
  a shut door's doorway between rooms: nothing stops you. **Tell me of any door that used to open and no longer does**,
  with the playfield: 684 doors count as locked now until the server says otherwise.
- [ ] Bare skin and glowing creatures (2026-11-18): hands and arms of players and human NPCs in their own breed's skin, no green anywhere, also on remains; an African or Asian Solitus has matching hands; creature eyes and lamps glow, and no creature has holes in its body
- [ ] Action window pairs (2026-11-18): sit, switch walk/run, sneak, fight, `/camp`: the tile shows the other half (also on a shortcut bar) and a click does it; before the server lists specials there are only nine tiles; a character without perks has no perk tiles
- [ ] Special attack range (2026-11-18): Brawl at a monster 6 m away says it is out of range and sends nothing; at 4 m it goes; Dimach from 10 m goes
- [ ] Container windows (2026-11-18): open two corpses: two windows, each taken from on its own; the bank's window keeps a place of its own; Escape closes the corpses' windows
- [ ] Original shortcut bars (2026-11-18): the first start fills bar 1 (Start Combat, Walk, Sit, Follow, Suspended Animation); keys 1-0, Shift+n page, Alt+n primary; drag by the strip, double-click it upright; wheel pages; drop, swap and drag out; the settings' Smart hotbars switch back
- [ ] `/pet` lines (2026-11-18): `/pet <name> follow` moves only that pet; `/pet attack` all; `/pet social wave`; with no pet the game's line
- [ ] Run speed (2026-11-18): a character with high attributes runs without rubber-banding back; compare a run's length with the original
- [ ] Mouse look (2026-11-18): turning speed as the original's at the same sensitivity; hold right, press left, let go of left: the look ends; A/D strafe while a left-button look moves
- [ ] Window fading (2026-11-18): a window fades two seconds after the mouse leaves it and comes back at once; a pinned one stays; the pin survives a restart; Escape closes a pinned window; "?" on Skills opens its help page
- [ ] Chat tab drag (2026-11-18): right-click a tab, left-click on another window's tab strip: it moves there before that tab; right-click again and left-click on empty space: a window of its own at the pointer; Escape lets go
- [ ] Playfield map window (2026-11-18): the control center's Map menu opens it; the ground shows at the original's scale; drag and let go comes back; other players' squares by side; the round corner map is there too (the default), and the client settings turn it off
- [ ] Mission item (2026-11-18): take "Plant a Bug" on Arete Landing: the recording device is in the inventory and can be used
- [ ] Inactive character (2026-11-19): Play on an inactive character opens the activation question; Cancel closes it
- [ ] Launcher licence (2026-11-19): the first start shows the licence; I accept goes on and it does not come again
- [ ] Uturn Canyon (6550) or Foundry of Nightmares: no water sheet over the low ground; a playfield with real water (Arete Landing coast) still shows it.
- [ ] In a city with many lamps or fires, the far ones show their effects; check the frame rate stays fine.
- [ ] Cast or watch a nano whose effect hangs on a limb (an effect mesh or trail on an arm or leg): it shows on the moving limb.
- [ ] Find a statel the old client drew level that should slope (a ramp or a tilted piece): it leans now and its ends meet what they should.
- [ ] City terminal map: door (blue) and guard (red) dots still show as small dots of the right colour.
- [ ] In a shop or a dungeon, what lies past the rooms is black and the fog fades into black.
- [ ] In a shop or dungeon playfield, nothing new appears inside the rooms; any statel that now shows outside them stands where the file puts it.
- [ ] Create a character: after the shuttle the game starts at once (no return to the character list), with the welcome loading screen
- [ ] With a fresh mao.ini, Shift+Enter opens the chat input and numeric-pad / starts a command
- [ ] Play a character: "Please wait" progress window over the hall for a moment, then the loading screen, then the game
- [ ] Cancel in that progress window: back at the login form, no game window appears
- [ ] Delete confirmation: the name in small letters gives the match error; typed exactly, the character is deleted
- [ ] A character with no folder under the original's Prefs: windows open at their default places, not another character's
- [ ] Entering the game: the loading picture in its own colours, not blue
- [ ] Character creation, profession scene: the text box under the professions is about 193 px high at 768 lines (234 at 1080)
- [ ] (if it can be provoked: Play a character already logged in elsewhere) the login form comes back with the error page above it
- [ ] Fail a login (wrong password), log in rightly, press Back at the character list: the error page is over the login again until closed
- [ ] Entering a playfield: the sky is right once the server's time comes (a moment of night before it, as in the original)
- [ ] Start a creation, pick a breed and profession, Back, quit the launcher, start it again, New Character: the same choices are there
- [ ] /ignore with a player targeted ignores them (game text "Ignoring character ..."); /ignore list shows them; /ignore on your own name is refused
- [ ] Another player toggles walk (the walk key) near you: their walk animation shows even when the server's reports make them look fast; creatures on paths walk/run as before.
- [ ] Walk about for a few minutes (stairs, slopes, a door): the log shows no "server names us at" line, or if it does, the character stays and later uses are not "too far away".
- [ ] Press the attack key with oneself selected: two lines ("can't attack yourself", then "unable to attack target"); in water: "can't attack in this state".
- [ ] With a debuff from a creature running on the character, the NCU window's "NCU (x/y)" stays the sum of one's own buffs.
- [ ] Zone through a door or Whom-Pah: the character stands still until the new playfield loads and then moves (no change expected when the server answers).
- [ ] Cast a nano with a cast time of a few seconds: the hand flares last the whole cast; an instant nano shows none.
- [ ] Open a window whose XML uses font="BOLD" (e.g. a control-center label or a title) and see the text bold; an ITALIC one slanted.
- [ ] Press I: only the inventory opens. Press Ctrl+1: the wear window opens alone; close the inventory and the wear window stays. Drag an item from the wear window into the inventory and back.
- [ ] Open a mission booth's window: the slider ends and the Request / Accept / Cancel buttons read as in the original.
- [ ] In the control center open Team > Team (team window), the Vehicle icon (with the expansion), and Command > Help > Chat Config: each opens its window and shows pressed while open.
- [ ] Right-click a player > Info: the Title line reads e.g. "<title name> (TitleLevel 3)", the Defender rank line a rank name.
- [ ] Click a link in an item info page and an NPC dialogue answer: it acts on the press, once.
- [ ] With the AOU Translucent theme, look at a window with buttons, check boxes and tabs (Options, Wear): controls tinted in the skin's Default, inactive tab texts black; compare with the original's look under that skin.
- [ ] /org leave: the question box comes up in the screen's middle; an organization invitation's box comes up at the pointer.
- [ ] Open the item store or market page; nothing changes for an English system (the page sees "en-US"); with MAO_BROWSER_TRACE the script object's AcceptLanguage shows the system's language.
- [ ] Hover an item with unusual requirements and a stat breakdown: names read as the game's (e.g. "Max Health", "Duck-Exp"), none as "Stat N".
- [ ] On a level 200+ character, the XP wing bar fills with shadowknowledge; on a character at the level cap it shows full, its tip "value / span".
- [ ] With no kept place for PlanetMap (or a fresh layout), open the planet map (P): it opens at the top left, about 300 by 400.
- [ ] With a nano buff that raises max health, compare the Stats window's health bar text with the server's current health: a computed maximum short of it shows as "current / max" with a full bar.
- [ ] Open the mail window, widen the Subject column and sort by Sent; close it and reopen (and restart): widths and sort are kept.
- [ ] Turn on the score bars in the Stats window's menu: each shows "score / next rank's score" (2000 for a score of 0) and fills accordingly.
- [ ] Place items by hand with one character, log in with another: the second keeps its own layout; back on the first, its layout is as it was left.
- [ ] Drag an implant onto an empty corner cell of the implant page: it goes to its own implant place.
- [ ] Open the info of a stim or a nano that locks a skill: "<skill> skill locked for N seconds" (with a second number in brackets for a character whose skill lock modifier is not 0).
- [ ] Open the bank, shut its window: the log says "the bank's window shut: told the server (action 0x6a)"; open the bank again from the terminal: it opens as before.
- [ ] Open the info of an armour piece or implant with "skills to use" (a tradeskill tool or an implant): the line shows after the modifiers.
- [ ] Use another player from more than about 6 m away: "out of range for trade" and no trade window; walk closer and use them: the trade opens.
- [ ] Wear an item whose requirements start with "On Self" (shown in its tip) without meeting them: the original's refusal lines, and the item stays where it was.
- [ ] Open the info of an item limited to one breed or sex: the requirement reads as the breed's or sex's name.
- [ ] Open a corpse, right click an item: nothing; left click it: it is taken. Right click an item in a chest: it is taken.
- [ ] Hover a weapon, an armour piece, an implant and a stack of ammunition in the inventory: the name in its rarity colour, then the info page's lines without the name repeated; compare with the original's tips in the reference shots.
- [ ] Destroy a backpack item from its menu: it leaves a moment later, with no "Item deleted" line. Try a bag with items in it (opened once): the game's refusal.
- [ ] Combine two items whose result changes name with quality (a tradeskill with a QL range): move the slider: the result's name follows.
- [ ] Drag a chest armour piece onto the implant page's chest place: nothing is sent, the item stays; onto the clothes' chest place: it is worn.
- [ ] Drag an inventory item onto an empty cell in the middle of an open backpack: it appears in that cell.
- [ ] With a ring worn on its own place and the other ring place free, right click a second ring: the worn one comes off and the new one goes on its place.
- [ ] Start the game twice in the same place: the music's first pieces differ between the runs
- [ ] Playfield map window on a Notum Wars playfield, land control on: the district and "Lvl Rng" text at the map's top left
- [ ] A character with original Prefs and no place in shortcutbars.ini: its bars appear where the original client showed them
- [ ] Attack key on a team mate: "not allowed to attack team members" and "Unable to attack target"; on oneself: both lines; while swimming: "can't attack in this state".
- [ ] In a team with a player of another side: the friendly Tab key picks the mate, the hostile one skips it; one's own pet likewise.
- [ ] Die: the world goes white at once after the death animation, the windows stay visible; on arrival at the reclaim point the white is gone at once.
- [ ] Watch a creature with no standing animation (a flier) stop: its last movement keeps playing rather than a frozen pose.
- [ ] Bring a fleeing creature below 15% health: it runs visibly slower and stays with the server's place.
- [ ] With a pet fighting: its hits and the hits on it show in a window subscribed to "Your pet hit by other", not in "Other hit by other".
- [ ] Hold the camera rotate keys and the zoom keys with the default settings: compare their speed with the original client's.
- [ ] In an interior with rooms of different districts (a city's buildings, a dungeon with a boss room), the music changes as you pass between them.
- [ ] Talk to a few NPCs (and one right after a relog): the chat opens at once; note any NPC that stays silent.
- [ ] /help, /help pet, /help nothing: the index page, the pet page, and the error line.
- [ ] Watch the journal after accepting a second mission and after a team member's mission update: note whether a mission is ever listed twice.
- [ ] Open a corpse and the overflow: no "Take all" row; a click takes one item.
- [ ] Open a mail terminal and send a mail: no line under the inbox list, "n/30" still shown.
- [ ] Inspect another player: the title reads "Inspect"; a right press on an item opens Look At / Create Reference, and both work.
- [ ] Raise a skill and Accept: no "Changes sent." line; try a disabled skill: its line still shows.
- [ ] Log in with an account that has no characters: the character screen comes up and New Character works.
- [ ] With a character lacking Shadowlands, step into a Jobe booth: no box, no teleport.
- [ ] Zone a few times running, and after a session that ended in walk mode: the server does not pull the running character back (no rubber-band).
- [ ] Cast a nano on yourself and on a target: no coloured rings, the program's own hit effect still shows.
- [ ] Outdoors in an area without fog (e.g. a clear desert): the far hills are drawn clear to the view distance, no haze; in a fogged area the fog is as before.
- [ ] Walk through grass and past trees (e.g. a park in a city): nothing bends away; plants with an animation of their own move by it, the rest stand still.
- [ ] Cast a damage nano with a particle hit effect: the hit's sparks appear at full strength on landing and fade by the effect's own timing.
- [ ] Fight a creature and heal: no numbers rise over anyone; the Combat chat lines still tell each hit and heal.
- [ ] Press the use key on a door or terminal: it opens with no "Using ..." line; with nothing targeted nothing is said.
- [ ] `/fxscript EP03_Orbital_Attack` does nothing; `/fxscript EP03_Orbital_Attack #` runs it; neither adds a chat line.
- [ ] Send an item through Deposit to Market: no client line appears.
- [ ] /inspect a player: the info page opens and no line appears in chat.
- [ ] `/viewdist 1000` and `/macro test /say hi` change the distance and put the macro on the pointer without any chat line.
- [ ] Pick another third-person camera mode in the menu: it changes and nothing is printed.
- [ ] Cast a nano with a long recharge: no "Nano programs available." line when it is over.
- [ ] Talk to an NPC and click an answer: it shows as your line once (no duplicate), the answers go until the NPC replies; give an unwanted item: it returns with no line.
- [ ] Typing /sit does not sit the character (the unknown-script line comes instead); X still sits and stands.
- [ ] No box appears at the bottom left; F12 saves a screenshot silently; /follow with a target says "Following <name> ...", without one "Cannot follow selected target."; /logout-in-fight and similar feedback lines show in chat once (not twice).
- [ ] Open inventory and a backpack: each has one tab; right-click the backpack's tab, left-click on the inventory's tab strip: one window with two tabs; relog: still together (docks.ini).
- [ ] Right-click a tab of a two-tab window, left-click on empty screen: it gets its own window there; closing a two-tab window closes both views.
- [ ] A borderless chat window: faint when not typing, solid while typing; a framed chat window fades when the pointer leaves it, not when pinned.
- [ ] A thin and a fat character (own and another player) look so in the world; a robed one wears the robed body.
- [ ] Jumping on a hoverboard plays a trick, then the board's run on landing.
- [ ] Log in again soon after a character was cut off in the world: the limbo window shows that character and counts down; Login enters it; letting it run out returns to the login form.
- [ ] A Whom-Pah pillar on Rubi-Ka shows its blue beam climbing from the top, with a small fire at its foot.
- [ ] A Shadowlands vein (e.g. playfield 4885) shows its swaying blue strands; compare with an original screenshot.
- [ ] An ICC shuttle (script ship) leaves an engine trail behind it as it flies.
- [ ] Fly with a jetpack, then take it off and put it on again: flight comes back after an ended flight only by that.
- [ ] On Arete, press the attack key at another newcomer player: "can't be attacked" and the unable-to-attack line, no attack sent.
- [ ] In a team with a pet owner, the friendly Tab key picks their pet; the hostile one does not.
- [ ] Pull a creature and stand still: it comes to about 4 m from you and fights from there; when the server's next path places it, the jump is small.
- [ ] Nano programs window: drag a program to an empty cell and onto another program (the other follows the pointer; click a cell to put it down); relog and see the cells kept; pick "By Name" and back to "Don't Auto Arrange".
- [ ] NCU window: right click, Auto Arrange > By Name and Sort Ascending reorder the icons; dead in list mode; kept after a relog.
- [ ] Type /version: the log shows "text command 0x9d" and the server's answer appears in chat (note what it says and how it comes).
- [ ] At 1080 lines high, look at whatever uses the CC17 and TT_MIN12 fonts (control center numbers, small headings) and compare with an original shot at the same height.
- [ ] Open your own character's info page: tokens, side XP bonus, veteran/victory points, ICC and FI tokens, specializations and the three ranks are there with believable numbers; inspect another player: no own-only lines, ranks 0.
- [ ] Upload a mission to the map, relog, open the journal: the mission is selected and its waypoint returns with the "uploaded" lines; zone and check again.
- [ ] In a team with a team mission: right click it, Update Team Members; the log shows "update team members of mission ..." and teammates get the mission.
- [ ] In the info view (an item page), drag across some text: it shades as selected, clicks on links still open them, and a new page clears the selection.
- [ ] Use or wield an item whose requirements start with "On Target" with and without a target selected: refused without one.
- [ ] Wear or wield items whose requirements name a running nano or a worn item: the refusal comes as in the original.
- [ ] Wear-effect lists that depend on a running nano still sum as before on the skills window.
- [ ] Open the info of a perk-locked item and a nano crystal: the requirement lines name the perk and the nano.
- [ ] Zone through a zone line and a grid exit: the picture holds still (no loading picture) and the new area appears; the chat says the changing-area line. Check the window does not stay frozen if a zoning is refused.
- [ ] `/setoption AutoCloseWindows 1`, then open Skills, then Perks from the character menu: the skills window closes; with 0 both stay.
- [ ] Place an item in the inventory's fourth column, narrow the window to three columns: the item stays in the fourth column (the grid scrolls sideways); widen it again: nothing moved.
- [ ] Take up an item you cannot wear (requirements too high) and move it over its wear place: the "can't drop" pointer shows; let it go there: the refusal is told and the item is still in hand.
- [ ] Take up a weapon you can wield: the plain pointer over a hand cell, the "can't drop" one over the belt's and beside the cells; letting it go in the hand wears it.
- [ ] Open the info of a nano or item with a proc or a cast (a weapon with a proc, a stim): the line shows "Offensive proc (x%): <nano>" or "Cast <nano>", and clicking the nano's name opens its page.
- [ ] Open the bank and walk away: about 5 m from where you stood the bank window closes (log: "walked away from the bank").
- [ ] Open a corpse and walk away: its window closes about 5 m out (log: "walked away from the container"), and the server answers without complaint.
- [ ] In a team, have a mate come into view: the log/trace shows a CharacterAction 0x80 naming them, and nothing odd follows.
- [ ] Shift+click a stock item in a shop: its page shows "Price" with the price the shop window shows.
- [ ] Open the page of a city building item or a tower and of an NF crystal: upkeep/size, tower type (Roman numerals), and the formula's nano page appear.
- [ ] In a dungeon or shop interior, ask a player standing in the next room behind a shut door for a trade: "out of range for trade" is told; with the door open it starts.
- [ ] Enter playfield 4221 (a generated-building template; its first room) and 1186: the room props stand where the walls are, none floating or missing.
- [ ] Cast a nano with a cast time of a second or more: the sparkle streaks come from both hands and follow them through the casting animation.
- [ ] A hit effect on a target (stars or lightning of a nuke) and a running buff's sparkles sit at the body part (head, chest, hands) rather than at fixed heights; check on a small and a large body.
- [ ] With the chat window shut (or no window showing Tell Messages), receive a tell: the sender's row in the friends list flashes and its folder label turns red; opening the tell window stops it.
- [ ] Right-click a chat window under "Chat Windows" in the friends list: the menu comes up and Show/Log/Rename/Delete/Copy/New work.
- [ ] Press a channel's name in a chat line: the window now talks to that channel; with Shift, "/group <name> " is typed.
- [ ] Get invited to a private chat group: the question lists the chat windows; choose one, answer Yes, and only that window shows the group.
- [ ] With ChatPGInviteAction 2, the inviter's row flashes in the friends list and pressing it brings the question.
- [ ] Open a shop booth: the stock is a grid of icons with yellow prices at the bottom right, the bought and sold lists one row high; the window can be resized and the stock grid fills it.
- [ ] Right click beside the rows (or the frame's menu button): "As a list" turns the stock into the column list; clicking a header sorts; the choice is kept the next time a shop opens.
- [ ] Buying (click a stock item), taking out (click a bought/sold item), dragging an inventory item onto a list to sell, and Shift-click pages work as before.
- [ ] Ask an NPC that takes items (Give): the "Trade" tab window appears right of the chat window, top aligned, and moves with it; the NPC's request shows where the answers were.
- [ ] Drag an item onto the grid (stops at the slot count), click it to take it out, Shift-click for its page; type more credits than you have and press V: the field drops to your credits and the trade is sent; press X: the bar closes.
- [ ] Options (F10) > Key bindings: the list shows Category / Function / Key with the keys in aqua; select "Attack", Add, press F, OK: a second Attack row with F, and F attacks in the game; Change it to G; Clear it; Reset All brings Q back; the keys survive a restart (mao.ini).
- [ ] While the Bind Key box is up, keys do not move the character or open windows (they go to the box); Cancel leaves the keys as they were.
- [ ] Fixed keys tab: the camera and target keys in aqua (NUMPAD_4 ...), chat reply/history "No Key".
- [ ] Rollup column: pages touch, 192 wide at the right edge; header shows i, title, roll and X; the wear/NCU/nano pages look right at their kept heights (rollup.ini) and the grip below each sizes it.
- [ ] Double-click a page header rolls it; holding the left button on a header and moving scrolls the column when the pages are taller than the screen.
- [ ] Right-press a page header, click on another page: it moves before that page; right-press, click in the world: it becomes a window there; right-release on a dock window's tab, click on the column: it becomes a page there.
- [ ] The "i" button on the Wear page shows the wear window's menu; on a rolled-up page, "Close".
- [ ] Option panel rollup fade level slider darkens the column.
- [ ] Without rollup.ini, the first login of a character the original played takes its RollupArea.xml pages and heights.
- [ ] A body-shape program (if one is found) changes one's own character's build on screen at once.
- [ ] Casting a nano of the instant kind (e.g. a Lend Nano program) shows no cast bar and the character does not take the casting pose.
- [ ] Near a Shadowland Anvian whose "Angry Smoke and Sparks" runs: its sparks are faint and dark, not the template's bright ones.
- [ ] In a thunderstorm with strikes close together: no more than two sky flashes at once, and the thrown rocks thin out during a long burst.
- [ ] Open a few windows and look at their frames and buttons: crisp edges, no blur; check at the usual UI scale that no piece shows uneven doubled pixels.
- [ ] Outdoors with the view distance at its lowest and highest: the world ends at the view distance (the sky shows past it), with no flicker of distant ground and no hole near the camera; the sky's domes and moons still show.
- [ ] In a tall indoor hall (e.g. a shop or a mission hall with a high ceiling), climb or jump high: the room stays shown and the camera keeps to it; in low corridors nothing changes.
- [ ] Outdoors on Rubi-Ka at night (a character with the expansion): small single-pixel stars twinkle over the star dome; nothing changes by day beyond what the dome shows.
- [ ] Look at the sun in daylight outdoors on Rubi-Ka: the ray fan's spokes fade in and out slowly, each at its own rhythm (a full cycle about 7 seconds); the centre stays steady.
- [ ] Launcher, Options, Connection page: set the dimension time-out to 45 and OK; enter a character: the loading bar runs over 45 seconds. Set a time-out to 2 and restart the launcher: it shows 5.
- [ ] Make a character from the start: the two opening lines take about five seconds to appear and stay some fourteen seconds; on the way down two new lines (docking completed, continuing DNA) appear about four seconds in; after Done the walk starts, a line shows after a second, and about six seconds after Done the picture is black and the game starts loading.
- [ ] Open a few windows (inventory, skills, options, chat with its frame) with the game's skin: the body is an even dark teal inside a thin light line, starting just under the title row; compare with a shot of the original. Try an AOU skin too.
- [ ] With no saved layout (or a new character), the main chat window spans the bottom of the screen 224 points in from each side; open a tell window and a new chat window: both come up in the middle.
- [ ] Open the Actions window (K): Reload has its own picture (not Aimed Shot's); a Reload shortcut put on a bar before this change still shows a picture; perk tiles look as in the original.
- [ ] Open the planet map (P) in a Rubi-Ka playfield (Rubi-Ka map), in a Shadowlands one such as Nascense (Shadowlands map), and in a Shadowlands dungeon (Shadowlands map).
- [ ] Select text in an info window and in an XML window's text field: the selection is light grey with the text readable over it.
- [ ] In a Notum Wars playfield, turn the map's Info (land control) on, open Build and put a tower item in: a light grey circle shows just in front of the character and follows its turns; it goes when the menu closes.
- [ ] At a mission booth, ask for missions and right-click an offer: "Look at Mission" shows its page (text, pay, reward items) in the info view; "Upload to map" marks its place again and says so. Take a mission with tips on: a mission tip may show.
- [ ] With a character whose original layout moved or sized the notification window, notifications show in that frame; without one, in the top third.
- [ ] Combine two items with a skill short (e.g. lock picks on a nano interface): the server's line shows in the tradeskill window's build text and not in the chat.
- [ ] An item is judged before anything is taken off (2026-11-18): a right click on the mission belt (Belt
  Component Platform, quality 30) with too little Computer Literacy says "Your Computer Literacy is required to be
  at least 76!" and the worn belt stays on; with enough (the skill window's number) it is swapped. A weapon or a
  utility one cannot wield says its own line the same way; armour says its line and "You're unable to wear this
  item." Watch for a refusal of something the skill window says is met (a bonus the client does not count).
- [ ] A shopkeeper who talks (2026-11-18): Antonio Stacklund's window has the Shop button (the fourth); a press
  opens his shop with its list and prices, and buying works; the same for Barry the Food Vendor and Marco Spida.
- [ ] Oneself in a fight (2026-11-17): fighting something in melee, a click on one's own character selects it (the
  fight goes on), a stim then heals, and a click on the creature selects it again.
- [ ] Locked things (2026-11-17): after a stim the stims and kits in the inventory and on the hotbar are grey
  until "First Aid skill available."; a special's slot is grey with its seconds; after a nano the programs are
  grey for the cast and recharge and "Nano programs available." follows.
- [ ] Back from the dead (2026-11-17): after a death and the arrival at the reclaim terminal the health bar shows
  the health the server gives and rises again, a heal can be used on oneself, and `/camp` logs off; the own
  corpse lies where the character fell under its "Remains of" name.
- [ ] Creatures' attacks (2026-11-17): a docker or a waste collector that attacks swings at each hit, stands in a
  fight stance between them (the docker crouched with its arms up), and stands up straight again when the fight
  is over; one that chases runs, not walks; a humanoid NPC fighting without a weapon swings something sensible.
- [ ] Remains and running creatures (2026-11-17): every creature killed turns into "Remains of ..." where it
  fell, also one shot while it ran at the character; one killed by somebody else that leaves no corpse is gone
  some ten seconds later; a creature that attacks stands next to the character when it hits, not metres away.
- [ ] The launcher (2026-11-17): it opens with its picture in it, the page comes whole with no white or black
  flash, also after the settings were open over it; closing it shows no white. On the machine that reported the
  crash: what the box says, and that machine's `crash.log`.
- [ ] A special's line of sight (CMB-022, 2026-11-19): a special attack (Brawl, Fast Attack ...) at a creature in plain
  view goes out as before; at one standing behind a wall, a rock or round a corner of a building, the game's "no line of
  sight" line shows and nothing is sent. Inside a dungeon, one in the next room behind a shut door is refused too.
- [ ] Pick-Up and Crawl tiles (WIN-112, 2026-11-19): Pick-Up with an item on the ground selected takes it into the
  inventory (with a full inventory the game's "inventory full" line, nothing sent); what the server answers to it is
  new (no capture had the message). Crawl starts crawling (others see it), the tile then shows the stop, and a second
  click stands up; while swimming it does nothing.
- [ ] Launcher PATCH (LCH-036, 2026-11-19): only when a dimension really is newer than the game folder (do not fake
  it: the patcher changes the game folder). PATCH opens AnarchyPatcher.exe for that dimension and the launcher closes.
- [ ] Icon time text (CMB-023, 2026-11-19): after a special or a nano, the Actions window tile and the nano programs'
  icon show the yellow time left (" 9s", then minutes over 60 s), as the shortcut bar does.
