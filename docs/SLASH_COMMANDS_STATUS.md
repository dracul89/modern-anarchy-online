# The slash commands: where each stands

Saved on 2026-11-12 for a later review (asked for by the user). What each command does in the original, with the
functions read, is in `GUI_DLL_PARITY.md`, section 3; what is left to try live is in `LIVE_TEST_LIST.md`
("Commands taken out on 2026-11-12, and put back the same day" and the sections after it). This page is the
short list.

## Not carried out at all

| Command | Why |
|---|---|
| `/org leave`, `/org disband` | The original asks in a confirmation box first (`N3Msg_OrgLeaveConfirmed`, `N3Msg_OrgDisbandConfirmed`); the boxes are not ported. Can be done from the original's code alone. |
| `/raid <command>` (create, list, listlocal, move, lootaccess, locks) | The message is identified (`RaidCmdIIR_c`, `4e525e58`) but not ported. `/raid` here opens the raid window, which the original's command does not. Cannot be verified without a raid. |
| `/tower`, `/terminate` | Notum Wars: `/tower` needs a service tower and runs a script; `/terminate` asks in a box first. Cannot be verified without a tower site. |
| `/cc` | The chat server's command packets are not decoded (GUI.dll FUN_100ba6f4, FUN_1016caf7). The original's `Befriend` script file is a `/cc` line, so that script does not work yet. |
| `/rp` | Turns the role-playing mark over (stat 673, bit 0x200, through `N3Msg_EventFeedback` 0x46); the message it goes out in is not read. |
| `/reclaim` | Opens the reclaim window (`N3Msg_ToggleReclaim`), which the client does not have. |
| `/name <new name>` | A rename request to the server; not decoded. Not to be tried on a character that is to keep its name. |
| `/gfx <n> on\|off`, `/bug` | Not read further. |
| `/items delete <n>` | Left out on purpose: it has the server delete an item of the account. |
| `/bank close`, `/bank open` | `close` shuts the bank's window (not done); `open` is a game master's in the original and does nothing here. |

## Carried out, but the result is unknown or unseen

- `/bank info` and `/items`: sent and not refused; nothing was shown in answer, so what they return is not known.
- `/team loot` inside a team (asking for the order; `all`, `leader`, `alpha` from the leader): needs two
  characters in a team.
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

- `/raid` opens the raid window (above).
- `/version` says the client's version; the original also asks the server for its own (action 0x9d).
- `/tell` alone opens the target's tell window: the user's wish, not the original's.
- A macro's or a filter rule's limits where the original loses data, and the script `%N` quirks kept as they are:
  `ORIGINAL_CODE_QUESTIONS.md`, 13 to 15.

## Done and confirmed live

`/afk` (with the box, the reply to tells, going away by itself), `/macro`, script files (`/script` and
`/<file>`), `/filter` (its own lines), `/dvalue`, `/played`, `/born`, `/team` outside a team, `/org info`,
`/org ranks`, `/org contract` (on an organization with no areas and one with towers).

## Suggested order for what is left

1. `/org leave` and `/org disband`: from the original's code alone.
2. `/cc`: the most useful next, since it makes the `Befriend` script work; it needs the chat library's packet
   read out of GUI.dll.
3. `/raid` and `/tower`: can be ported from the code, not verified without a raid or a tower site.
