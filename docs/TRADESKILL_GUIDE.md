# The tradeskill guide

The client's own window (`/crafting [text]`, `/craft`; window name `crafting`): a guide to what can be made in the
game, read from the player's own install. Asked for on 2026-11-12 ("a tradeskill helper too, a guide to all the
creations a user can craft in game"). The original client has nothing like it: its tradeskill window answers for one
pair of items at a time.

## Where the recipes are, and where they are not

**The game's files hold no table of the tradeskill processes.** Looked for on 2026-11-12:

- The resource database has no record kind for them (the kinds were surveyed; `craftsurvey` example).
- The item templates' actions are of kinds 0, 1, 3, 5, 6, 7, 8 and 111: wear, wield, use and the like. None is
  "use on another item", and no template carries the skills a combination asks for.
- The original client learns each answer from the server: put two items in the tradeskill window and the server
  says the result, its quality range and the skills needed (the `CharacterAction` messages 0xdc to 0xe4, handled in
  `tradeskill.rs`). Nothing is known to the client before that.

So a guide with every recipe, every skill and every quality rule cannot be built from the game's files alone. The
community's recipe databases were gathered by players over the years; they are not in this repository and are not
fetched (the rule here: no captured or scraped text in the repository).

What the files do hold is what Funcom wrote on the items themselves, and it is a great deal:

| | on this install (2026-11-12) |
|---|---|
| item templates | 120,820 |
| entries of the guide (items that speak of making, are named by one that does, or share its brackets) | 16,306 |
| of those, their own text speaks of making | 12,250 |
| books and manuals (long texts of instructions under a book's name) | 416 |
| entries whose text names other items | 8,045 |
| steps stated outright ("with this, that makes that") | 2,002 |

The books are the best of it: the game ships a book on creating nano crystals, a construction manual for each of
hundreds of weapons, guides to the medical libraries, engineering papers. Their texts give the tools, the order and
often the skills.

## What the guide is made of (`mao_formats::crafting`)

`Guide::of_database(db)` reads every item template's name and description (about two seconds) and keeps:

- **Entries.** An item is in the guide if its description speaks of making (`speaks_of_making`: words such as
  tradeskill, combine, assemble, "to create"), or a description that does names it, or its name ends in the same
  brackets as one of those. Templates with the same name are one entry (`templates` counts them).
- **Books** (`Entry::guide`, `is_guide`): a text of 500 characters or more that speaks of making, under a name with
  book, guide, manual, "how to" and the like. Their line breaks are kept for reading (`Entry::text`).
- **Names in a text** (`Entry::names`, `Guide::named_by`): the other entries a description names. Matching is by
  whole words, longest name first. A name of the form "A - B" is also found as "B A", and where a text says only "A"
  and nothing is called just that, the first of the family stands for it. **This is a reading of prose, not data:**
  a text that names a thing in passing is linked the same as one that needs it.
- **Steps** (`Entry::steps`): where a sentence names items and then a result phrase ("will make", "results in",
  "to create", ...) is followed by another name, that is kept as "with these, it makes that". 2,002 are read this
  way; a sentence built another way is missed, and a few will be read wrongly.
- **Kin** (`Guide::kin`): entries whose names end in the same brackets ("... (Grid Gateway)"): the links of one
  thing made in several steps.
- **Books about a thing** (`Guide::books_about`): the books whose texts speak of an entry by name or by its kind
  (`kind_of`: the name without its brackets or what follows a dash).
- **The way to a thing** (`Guide::way_to`): the stated steps followed back from a thing to its parts, to a depth.

`Guide::search` puts a book whose name fits first, then names that begin with the words, then names that hold
them, then books whose text holds them.

## The window (`craftguide.rs`)

A list on the left (what was found; "Books and manuals only" lists the books), a page on the right:

- the item's text, as written;
- "What its text says outright": its stated steps;
- "Books and manuals that speak of it or of its kind";
- "The way to it, as far as the texts say";
- "What the tradeskill window told you" (below);
- "Of the same making (the same name in brackets)";
- "Its text names" and "Named in the text of N others".

Every name is a link to its page; Back goes to the page before. A name in green is a thing the player has, with the
character and place when hovered: that comes from the item finder's files (`itemindex.rs`), so it knows what was
seen on each character the last time they were played.

The guide is read the first time the window opens, away from the frame.

### What the tradeskill window teaches (`craftguide::Book`)

Every answer the server gives in the tradeskill window is written down in `tradeskills.txt` beside the settings:
the pair of items, the result with its lowest and highest quality, and each skill requirement shown with the
qualities of the two items at the time. The pages of the items involved show these under "What the tradeskill
window told you". This is the only exact part of the guide, and it grows only with use: the server never tells the
whole table. The file is the player's own and is not shared.

## In the help assistant

The crafting texts are a source of the "Ask the guides" window (`crafting`, on by default; `mao_help::crafting_docs`):
the books, and the items whose descriptions speak of making, are longer than a line and have no bracket ending
(3,476 pages; the thousands of links of the long chains all say the same and are left to this window). So the
small model can answer "how do I make a nano crystal" from the game's own book. Tried on 2026-11-12: that question
is answered from the right texts; "what do I need to build a solar-powered pistol" found the right items but the
answer joined two texts wrongly. The answers are as good as the search, which is plain word matching.

## Not done

- Little was tried live: `/crafting nano crystal` opened the window in a session and the guide was read in 2.3 s
  (the log's "tradeskill guide:" line). The pages were looked at in previews over the game's data
  (`MAO_UI_PREVIEW=crafting`, `MAO_CRAFT=<text>`, `MAO_CRAFT_PAGE=<name>`, `MAO_CRAFT_SAMPLE=1` for a made-up guide
  with things owned and tried), and the writing down of the tradeskill window's answers is tested without a server.
- The skills a text gives ("requires 250 in Nano Programming") are not read out of the prose.
- No quality arithmetic (what quality of tool a quality of part needs): the texts rarely say, the server knows.
- A page does not offer to open the tradeskill window with the two items.
- A recipe table from outside (a player's own export, a community database with its owner's leave) could be read
  from the settings folder and shown beside the rest; nothing reads one yet.

## Tools

`cargo run --release -p mao-formats --example <name> -- <game folder> ...`:

- `craftsurvey`: the kinds of action the items have, and how many descriptions speak of combining;
- `craftsample <word> [every nth]`: the descriptions that mention a word;
- `craftbooks [least length]`: the long texts that speak of making;
- `craftguide [part of a name]...`: the guide's counts, and for each name the entries found with their steps.
