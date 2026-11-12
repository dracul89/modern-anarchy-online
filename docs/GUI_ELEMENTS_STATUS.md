# The interface's windows: what is left

Saved on 2026-11-12 for a later review (asked for by the user), beside `SLASH_COMMANDS_STATUS.md`. It is the short
form of the window-by-window table in `GUI_DLL_PARITY.md`, section 2, which has the functions read and the
details of each gap.

**This is not a fresh audit.** The table's cells are as they were last written; only the team and organization
rows were brought up to date on the day this was saved. A few gaps listed here may already be closed.

## No window at all yet

- City terminal.
- Petitions.
- Reclaim.
- Notum Wars: tower build menu, laser target, airstrike, battle station, signup terminal, mech select, teleport
  target.
- The organization module's own views (the organization name view, the upkeep views) and its boxes for leaving,
  disbanding, invitations and promotions. Only the `/org` commands, the info page and the contract's window
  exist.

## The client's own stand-in, not the original's

- Shortcut bars: the hotbar is this client's design.
- Mission selection (the terminal's window).
- Character creation's screens.
- The Help and Whom-Pah windows.

## Ported, with these gaps

| Window | Still missing |
|---|---|
| Login | account activation and the limbo screen |
| Control center | the mini toolbar, docking into the rollup area, dragging entries |
| Chat | dragging tabs, the transparency sliders, the log window, the per-window menu in the configuration |
| Friends | the flashing row for unread tells, tell windows listed under Chat Windows, the chat window's own menu |
| Inventory, bags, bank | `labels_on_top` for the wear and inspect windows |
| Info view | nano radius, shop price, some names on a character's page |
| Skills | the suggested distribution |
| NCU and nano programs | the radius column, Auto Arrange, placing by hand, kept column widths and sort |
| Special actions | placing by hand; backstab never seen live |
| Mail | Return To Sender, C.O.D., kept column widths |
| Missions | kept columns and the active mission, Update Team Members, the shadow knowledge reward row |
| Shops and trade | the `ShopBuy` mode and the list configs; the partner's item names in a player trade; the out-of-range check |
| NPC chat | the Shop button's trade with the NPC |
| Team | invite and kick from the window, the team's side |
| Pets | the icon list of a pet's nanos, Remember Pet Names |
| Planet map | the Build and War buttons and the coordinate row, the planet map's own playfield view |
| Notifications | the saved frame and clickable links |
| Options | `HotKeys.xml`; most settings are kept but not yet acted on |
| Progress dialog | not shown while a character is entered |
| Character viewer, targeting, tradeskill, knowledge, factions, research | small pieces each; see the table |

## Waiting for a new capture of the original client

- Raid: no raid messages were captured, so Convert To Raid and Raid Locks send nothing and the raid bar is not
  drawn.
- Looking for team: the list lives on the chat server and was not captured, so Search finds nobody.
- Team invite and kick from the team window, and the partner's items in a player trade.
- City terminal, towers and reclaim, as above.

## Done, or close to it

Compass, tips, stats, perks, the daily login and rewards page.
