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
