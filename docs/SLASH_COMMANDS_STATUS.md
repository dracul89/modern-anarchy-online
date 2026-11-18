# The slash commands: where each stands

Saved on 2026-11-12 for a later review (asked for by the user). What each command does in the original, with the
functions read, is in `GUI_DLL_PARITY.md`, section 3; what is left to try live is in `LIVE_TEST_LIST.md`
("Commands taken out on 2026-11-12, and put back the same day" and the sections after it). This page is the
short list.

## Not carried out at all

| Command | Why |
|---|---|
| `/raid <command>` (create, list, listlocal, move, lootaccess, locks) | Ported 2026-11-18 from the code (`raidcommands.rs`, `RaidCmdIIR_c` `4e525e58`, the answer `RaidIIR_c`), but no raid message was ever sent to a server: cannot be verified without a team of two. |
| `/tower`, `/terminate` | Notum Wars: `/tower` needs a service tower and runs a script; `/terminate` asks in a box first. Cannot be verified without a tower site. |
| `/reclaim` | Opens the reclaim window (`N3Msg_ToggleReclaim`), which the client does not have. |
| `/bug` | Left out on purpose (2026-11-16): the original reports to its makers' bug database about their own client. |
| `/items delete <n>` | Left out on purpose: it has the server delete an item of the account. |
| `/bank open` | A game master's in the original; does nothing here. |

## Carried out, but the result is unknown or unseen

- (2026-11-16, from the original's code; **sent live the same day on the test character**: `/cc addbuddy`,
  `/cc rembuddy` and `/cc help` work, `/rp` sets the stat, `/cc info` gets no answer) `/cc <words>` (the chat
  server's command packet 120; the `Befriend` script is `/cc addbuddy %1`), `/cc info <name>` (packet 110;
  nothing comes back), `/rp` (`CharacterAction` 0xa6 with stat 673's bit 0x200 turned over), `/name <request>` (a tell to
  `name-request` with all that follows the command; **sent live 2026-11-16**: `nick <name>`, `first`, `last`,
  `policy`, `status`, `delete all`, `help` are the service's words, and a nick request was queued; **not to be
  tried on a character that is to keep its name**). `/gfx enable|disable <number>`
  and `/bank close` send nothing and are done.
- `/bank info` and `/items`: sent and not refused; nothing was shown in answer, so what they return is not known.
- `/team loot` inside a team (asking for the order; `all`, `leader`, `alpha` from the leader): needs two
  characters in a team.
- `/org leave` and `/org disband` (2026-11-16): each asks in its box first, as the original; a yes sends command
  0x10 (leave) or 6 (disband, with the selected target). **Never sent; to be tried only in an organization made
  for testing.**
- Every `/org` sub-command except `info`, `ranks` and `contract`: `invite`, `kick [name]`, `promote`, `demote`,
  `bank [add|remove <n>]`, `tax`, `debt`, `city`, `startvote`, `vote`, `stopvote`, and the ones that set a text
  (`name`, `description`, `objective`, `history`, `governingform`, `create`). Ported from the original's code and
  never sent, apart from `history` once by mistake (it sets the text; it was empty before and after). **To be
  tried only in an organization made for testing.** The text setters are sent as typed, an empty text included,
  as the original sends them (the user's decision): alone they empty that text.
- `/org info` with another member, or a member of another organization, selected.
- The organization's contract window: it shows the items; a click or a drop on it sends nothing here, as what the
  original does there was not read.
- `/invite`, `/kick`, `/leave`, `/assist`: ran without a fault, their effect not confirmed.
- `/start`, a script with `/delay`, the chat filter on a real channel line, a macro after a restart.

## Differs from the original, knowingly

- (`/raid` opened the raid window until 2026-11-18; it is the raid's commands now, and the window is `/open raid`.)
- `/version` says the client's version; the original also asks the server for its own (action 0x9d).
- `/tell` alone opens the target's tell window: the user's wish, not the original's.
- A macro's or a filter rule's limits where the original loses data, and the script `%N` quirks kept as they are:
  `ORIGINAL_CODE_QUESTIONS.md`, 13 to 15.

## Done and confirmed live

`/afk` (with the box, the reply to tells, going away by itself), `/macro`, script files (`/script` and
`/<file>`), `/filter` (its own lines), `/dvalue`, `/played`, `/born`, `/team` outside a team, `/org info`,
`/org ranks`, `/org contract` (on an organization with no areas and one with towers).

## Suggested order for what is left

1. (Done 2026-11-16: `/org leave` and `/org disband`.)
2. (Done 2026-11-16: `/cc`.)
3. `/tower`: can be ported from the code, not verified without a tower site. (`/raid` was ported so on 2026-11-18.)
