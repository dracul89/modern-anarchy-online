# Questions about the original code

Places in the original client (GUI.dll and friends) where the logic looks wrong, found while porting. Each says what the
code does, why it looks like a mistake, and what the port does now. To review together: keep the original's behaviour, or
fix it in the port.

## 1. Tip patterns: `*x` is a prefix match, not a suffix match

`TipSystem_t`'s matcher FUN_100539be (GUI.dll) compares an event's data with a tip's `data` pattern from
`text/tips/TipHeader.xml`:

- `*x*`: `FindNoCase(data, x)`: contains. Fine.
- `*x`: `CompareNoCase(data, pattern + 1, len - 1)`.
- `x*`: `CompareNoCase(data, pattern, len - 1)`.

`String::CompareNoCase(a, b, n)` (Utils.dll, `ao-utils` `compare_no_case`) compares from the **start** of both strings
for `n` bytes. So `*x` compares the start of the data with `x`: it is the same prefix match as `x*`, never a suffix
match. The header has eleven `*x` patterns that are clearly meant as suffixes, for example
`*Startup Crystal - Quick Heal` (the item is "Doctor: Startup Crystal - Quick Heal" / "Nano Crystal ..."), `*Basic Board`
and `*Training` (playfields like "Arete Training ..."?). With the original's logic these tips only fire for data that
*starts* with the text after the star.

The port (`tips.rs` `matches`) treats `*x` as "ends with x", the evident intent. Decide: keep that, or match the original.

## 2. The mail list sorts dates as text

`MailListItem_c`'s compare (FUN_1010422b) sorts the Sent and Expires columns by `std::string::compare` of the shown text,
which is boost's `to_simple_string` ("2026-Oct-30 21:05:09"). Within a year the month names sort alphabetically
(Apr < Aug < Dec < Feb < Jan ...), so sorting by date puts a February mail after a December one. The item has the times
as numbers (+0xd8, +0xe0) and could compare those.

The port (`mailbox.rs`) does what the original does (sorts the text). Decide: keep, or sort by the time.

## 3. The inbox size is counted from the list, against a fixed 30

FUN_10109489 writes `InboxSize` as `"%d/%d"` with the number of rows in the list view and the constant 30 (0x1e), not
the count the server reports. Rows the list has not been given yet (or a server whose limit is not 30) show wrong. Minor.

The port shows the server's count when it has one.

## 4. Saved list columns count their widths twice

`MultiListView_c::LoadColumnInfo` (0x10136a6d) restores the columns' hidden flag and width from `listview_config`. It
resets the rows' total width (`+0x260`) to `-(spacing + 1)`, then, for each saved column it finds, adds `width + 1 +
spacing` when the column is shown. After that loop it walks **all** the columns again and adds `width + 1 + spacing`
for every shown one. A shown column that was in the saved config is therefore counted twice, so after loading a
saved layout the list's content is wider than its columns (a horizontal scroll range past the last column, rows drawn
wider than the header). `AddColumn` and `SetColumnWidth` keep the total right; only the load doubles it.

The port has no saved column layouts yet (nothing to decide until it does). When it does: count each column once?

## 5. Removing a friend recolours only some folders

`FriendListView_c` FUN_100a86f6 (a friend removed) walks the Online, Offline and Recent folders to delete the friend
and to recolour each folder's label red (0xff6666) when a friend in it has unread messages. But it uses the removed
friend's id as its loop's "still looking" flag: once the friend is found and deleted the id is set to 0, and the
`do`-loop's first test `if (id == 0) return;` ends the function before the folders after it are looked at. Also a
folder's colour is decided by `id == 0 && unread_found`, so a folder that did not hold the removed friend is turned
white even when friends in it have unread messages. The folder colours then stay wrong until the next message.

The port does not colour folders by unread messages yet (it keeps no messages per friend), so nothing to decide now;
noted for when it does.

## 6. The grid's first free place assumes at least three columns

`MultiListView_c::GetFirstFreePos` (0x10133940), for a grid that is not auto-arranged, looks for the first free cell
counting `max(columns, 3)` cells to a row. In a window narrow enough to show only one or two columns, an item dropped
or added "at the first free place" can be put in a third column that is not visible (and, with fewer columns, not
reachable). Probably harmless with the default window sizes; looks like a guard against tiny windows that hides items
instead.

The port's grids are auto-arranged (the item windows), or place items by their own cells (the wear window), so this
does not come up.

## 7. Not a bug (checked): the tip window's "Next Section... (%u/%u)"

The decompile of FUN_10056acd shows `FormatAppend` with one value for two `%u`. The assembly (0x10056dae) pushes both
(the sections shown and the total), so the text is right. Listed so it is not raised again.

## 8. Guardian of Shadow's bar fills backwards below 0

`FactionBar_c`'s update (FUN_1005de7f) fills a bar to the standing's share of its band (`N3Msg_GetFactionRange`), and for
a standing below 0 uses one less that share, because a negative standing's bar is the red one drawn from the right
(direction 2): the further below 0, the fuller the red bar. Guardian of Shadow (stat 566) is special-cased to the yellow
bar drawn from the **left** (direction 1) whatever the standing, but the `1 - share` flip still applies. So a GOS
standing of -45000 (Dreaded) shows a yellow bar three quarters full from the left, the same picture as a standing of
+1000..4999 three quarters through Liked; -30001 shows it empty. With the number hidden (the titles shown instead) the bar
tells nothing about which side of 0 the standing is.

The port (`factions.rs`) does what the original does. Decide: keep, or fill GOS by its plain share (or give it the red
bar below 0 like the others).

## 9. Faction titles come from two tables that disagree

`GetFactionTitle` (Gamecode FUN_10062dcb) finds the standing's band by its place in the database's band list (0xf4265
records, FUN_100c2427: counted from 1 in list order) and then looks that number up in a table built in code
(FUN_10035eaf), not in the band record's own name. The two spell one differently ("Close friend" in code, "Close
Friend" in the database) and the lookup only works while the database's records stay in their 1..12 order; an added or
reordered band would name the wrong title.

The port uses the code's table, as the original. Probably nothing to change; noted.

## 10. The Research window is named "Vehicle" and has no title

`ResearchWindow_c`'s construction (FUN_100f750e) makes its window with the name "Vehicle" and an empty title (the first two
strings it builds before `FUN_100395fd`); its config is `ResearchConfig` and its help page "The Research window.html", so
the name looks copied from the vehicle window's code. A window's name is what its saved place and the `esc_*`/window lists
key on, so the Research window may share or overwrite the vehicle window's saved place; the empty title leaves its title bar
blank.

The port keeps the empty title (its internal name is the client's own "Research"). Decide: keep the blank title bar, or
title it "Research" (text.mdb has the control center's "Research" label).

## 11. The away reply's "hour" and "minute" look at the wrong numbers

`HandlePrivateMessage` (0x10087e59) feeds `ChatCmdFeedback_AFKReplyText` four things in this order: the character's name,
the hours away, the minutes away, the away message. The text of this install numbers its two choices `#3{1:hour | hours}`
and `#4{1:minute | minutes}`. A choice's number names the argument that decides it, so "hour" or "hours" is chosen by the
third argument, the minutes, and "minute" or "minutes" by the fourth, the message (a text, so always "minutes"): one hour
and five minutes reads "1 hours and 5 minutes", five hours and one minute "5 hour and 1 minutes". They look one too high
(`#2` and `#3` were meant).

The port feeds the same four in the same order through the same formatting, so it says what the original says. Decide:
keep it, or count the choices from the right arguments when the text has this shape.

## 12. `/afk` typed again while its box is still up comes back by itself

`/afk` alone goes away with the default message and opens the box, whose count takes the field's text after 30 seconds
(FUN_10083496 raises the view's signal with 0, FUN_100b6b65 answers it). Typing `/afk` again before that ends being away
(FUN_100b82a7, "AFK off.") but leaves the box up. When its count ends, FUN_100b6b65 finds the field's text different from
the message, which is now empty, says "Changed AFK message from '' to '...'" and calls `SetAFK` with it: the character
is away again, half a minute after saying it was back.

The port does the same (seen in a live run on 2026-11-12). Decide: keep it, or close the box when being away ends by
another way.
