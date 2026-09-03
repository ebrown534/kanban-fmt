# kanban-fmt

A parser and pretty printer for plain-text kanban board exports.

Several kanban tools let you export a board as a flat text file: a board
name, a list of columns, and the cards in each one. People end up hand-
editing these (renaming a column, moving a card, adding a tag) and it's easy
to leave the file slightly broken: a missing dash, an empty tag, a column
defined twice. Most scripts that read this kind of file either crash with a
raw panic or fail silently. This one is built the other way around: parsing
is the whole point, and every rejection tells you the exact line and column
that's wrong and, where it helps, what to do about it.

## The format

```
board: Sprint 12

column: Todo
- Fix login bug @alice #bug,urgent
- Write onboarding docs

column: In Progress
- Refactor parser @bob #cleanup

column: Done
- Set up CI
```

Rules:

- The first non-blank line is always `board: <name>`.
- The rest of the file is a series of column blocks, each starting with
  `column: <name>`, separated by blank lines.
- Each line under a column is a card: `- <title>`, optionally followed by
  `@<assignee>` and/or `#<tag1>,<tag2>,...`.
- A column name can only be defined once. A card can have at most one
  assignee and one tag group.

## Usage

Format a file (parses it and writes the canonical form to stdout):

```
kanban-fmt board.kbx
```

Validate without printing anything:

```
kanban-fmt --check board.kbx
```

```
ok: 3 column(s), 4 card(s)
```

Format a file in place (only touches the file if the canonical form differs):

```
kanban-fmt --write board.kbx
```

Read from stdin by passing `-` instead of a path (useful in a pipeline; not
valid with `--write`, since there's no file to write back to):

```
cat board.kbx | kanban-fmt -
cat board.kbx | kanban-fmt --check -
```

## What a broken file looks like

Given this file, where a card is missing its title:

```
board: Sprint 12

column: Todo
- @alice
```

```
$ kanban-fmt board.kbx
error: card title cannot be empty
--> board.kbx:4:3
  |
4 | - @alice
  |   ^
```

And a duplicate column:

```
board: Sprint 12

column: Todo
- Fix login bug

column: Todo
- Write docs
```

```
$ kanban-fmt board.kbx
error: column 'Todo' is defined more than once
--> board.kbx:6:1
  |
6 | column: Todo
  | ^^^^^^^^^^^^
  = note: first defined on line 3
```

A file can have more than one problem. All of them are reported in one run,
not just the first. Given a file with both a missing card title and a
duplicate column:

```
board: Sprint 12

column: Todo
- @alice

column: Todo
- Write docs
```

```
$ kanban-fmt board.kbx
error: card title cannot be empty
--> board.kbx:4:3
  |
4 | - @alice
  |   ^

error: column 'Todo' is defined more than once
--> board.kbx:6:1
  |
6 | column: Todo
  | ^^^^^^^^^^^^
  = note: first defined on line 3

error: aborting due to 2 previous errors
```

## Status

Early. The parser and printer both work end to end for the format above.
See the code for what's implemented.

## Building

Standard library only, no dependencies.

```
cargo build --release
```

## License

MIT, see LICENSE.
