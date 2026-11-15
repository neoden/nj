# Scenario tests

A scenario is a file of keystrokes and what the screen should show after
them, run against a headless editor. One `.sc` per scenario, in a
directory per area; a scenario is named by its path from here,
`search/esc-ends-the-search`. The file name is the claim it makes.

```sh
cargo test --test scenarios                   # all of them
cargo test --test scenarios -- search/        # one area
cargo test --test scenarios -- esc            # names containing `esc`
cargo test --test scenarios -- --update esc   # rewrite their frames
```

## A file

```text
# Ctrl+/ comments the touched lines out with the language's marker...

%file main.rs "fn main() {\n    let x = 1;\n}"
%start main.rs
size 30 7
key Down
key Ctrl+/

--- one line commented, the marker at its own indent
0|1 fn main() {
1|2     // let x = 1;
 |--^---------------------------
2|3 }
3-5|
6| ….rs* · Rust · 3 lines · 2:1

key Ctrl+S
?file main.rs "fn main() {\n    // let x = 1;\n}"
```

The author writes everything except the frames: the comment at the top,
which says what the scenario shows and why, the instructions, the `?`
checks and the `---` lines with their captions. The frame under a `---`
line, up to the blank line that ends it, is generated. The test renders
the scenario again and compares the result with the file. `--update`
writes the fresh frames into the file and changes nothing else.

The split is deliberate. A frame is a picture: nobody can write it by
hand, so the machine writes it and the author looks at it. An exit code
or a file's contents can be stated before the run, so the author states
it as a `?` line, and a regression there is never blessed by `--update`.
A `---` with no frame under it fails, so a new scenario cannot pass
before anyone has looked at its frames.

## The lines

One instruction per line; blank lines and `#` comments are skipped. Text
is in double quotes, with `\n`, `\t`, `\"` and `\\` inside; names are
bare. The full description is the header of `src/term/scenario.rs`.

The editor's own, what a user would do:

| line | does |
| --- | --- |
| `size W H` | the screen, in cells; 40×10 until said |
| `type "TEXT"` | keystroke by keystroke; no control characters |
| `key NAME` | one key: `Enter`, `F3`, `Ctrl+Shift+L` |
| `paste "TEXT"` | one bracketed paste; the way to enter a newline |
| `click X Y [MODS]` | the left button going down at column X, row Y |
| `drag X Y [MODS]` | the pointer moving there with the button held |
| `scroll up\|down X Y [MODS]` | the wheel |
| `wait` | let every background job finish |

The run's, around the editor:

| line | does |
| --- | --- |
| `%file NAME "TEXT"` | a file in the scenario's directory |
| `%shell COMMAND` | the rest of the line, run by `sh` in the scenario's directory and waited for, five seconds at most: a command still running then is ended and the line fails; what it leaves running in the background is ended with the run |
| `%start ARGS` | start the editor with this command line; without it the editor starts as bare `nj` would, at the first line that is not a `%` line |
| `%stdin "TEXT"` | what is piped into the editor; a terminal otherwise |
| `%clipboard "TEXT"` | what the desktop clipboard holds |
| `%grammar NAME` | the scenario needs this grammar; a build without it skips the scenario |
| `%reset` | end the editor, empty the directory, start over |
| `%for NAME VALUES` | run the scenario once for each of VALUES, three or more sizes going up, as `200K 2M 20M`; a `%shell` command finds the value of its pass in the environment variable NAME |
| `%measure NAME` | start counting the editor's memory and processor time, under NAME; said again with the same name, start afresh |

The run's checks, of what the screen cannot show:

| line | holds when |
| --- | --- |
| `?exit N` | the editor has exited with code N |
| `?stdout "TEXT"` | and, as a pipeline, handed TEXT downstream |
| `?file NAME "TEXT"` | the file holds TEXT |
| `?nopath NAME` | nothing is there |
| `?clipboard "TEXT"` | the desktop clipboard holds TEXT |
| `?requested NAME GROWTH` | over the values of `%for`, the bytes the editor asked for since `%measure NAME` have a shape of growth no faster than GROWTH: `constant`, `log` or `linear` |
| `?retained NAME GROWTH` | the same for what the editor holds now over what it held at `%measure NAME` |
| `?peak NAME GROWTH` | the same for the most it held at any moment since `%measure NAME`, over what it held there |
| `?memory NAME GROWTH` | the three above in one line |
| `?time NAME GROWTH` | the same for the processor time since `%measure NAME` |

A `?` line that does not hold is an error naming the line. Put `?` lines
after the keys they check and before the next `---`. Once the editor has
exited there is no screen, so a `---` or a key there is an error; `?exit`
and `?stdout` say what became of it, and `%start` starts the next one.

## How a cost grows

A scenario can check how the memory and the time of some lines grow with
the size of the input:

```text
%for SIZE 200K 2M 20M
%shell mkfifo big.txt; (yes 'some text' | head -c $SIZE > big.txt &)
%start big.txt

%measure move
key PageDown
key Ctrl+End
?memory move constant
?time move log
```

`%for` runs the whole scenario once for each value, every time from an
empty directory. The value reaches only the `%shell` commands, as an
environment variable; what it is the size of, a file, a line, the number
of matches, is up to the command. Anything else the input depends on is
written in the command as it is. One `%for` to a scenario, with one
variable.

`%measure NAME` begins a count of the editor's memory and time. A `?`
line reads one number off the count:

| line | reads, since the count began |
| --- | --- |
| `?requested NAME` | every byte the editor asked for, freed since or not: the work done on memory |
| `?retained NAME` | the bytes it holds now over the bytes it held then: what the lines left behind |
| `?peak NAME` | the most bytes it held at any moment, over the bytes it held then: what the lines needed |
| `?time NAME` | the processor time it took |

`?memory NAME GROWTH` is `?requested`, `?retained` and `?peak` with the
same growth, which is the usual case. A copy made and dropped shows in
`?requested` and `?peak` and not in `?retained`. A cache that keeps
growing shows in all three. One count can be read by several lines.

After the last pass each `?` line has one cost for every value. The run
finds which shape of growth the costs have, and the line fails when that
shape is faster than the one it names:

| growth | the cost is | |
| --- | --- | --- |
| `constant` | `a` | the same at every value |
| `log` | `a + b · log(value)` | more by the same amount each time the value is ten times more |
| `linear` | `a + b · value` | more in proportion to the value |

Each shape is a formula with a part `a` that does not depend on the
value. The run fits every formula to the costs by least squares, and
also one for a square, `a + b · value²`, so that a cost growing faster
than `linear` has something to fit. The shape of the cost is the slowest
formula that the costs are on. Because `a` is in every formula, a cost
that is mostly constant still shows how the rest of it grows.

The failure names the shape found, and gives the cost at each value,
how much it varied between the passes, and how far the costs are from
each formula:

```text
line 12: the time of move is log in SIZE, not constant
  SIZE=200K  625.15µs, within 1.622µs from pass to pass
  SIZE=2M  659.195µs, within 3.193µs from pass to pass
  SIZE=20M  693.627µs, within 4.57µs from pass to pass
  the furthest point from each formula: constant 5.3%, log 0.0%, linear 2.4%, quadratic 2.7%
```

A cost is on a formula when it is within a hundredth of itself of it,
or within three times what it varied by between the passes. So a change
of a few bytes is no growth, and on a machine where time varies more the
check lets more pass instead of failing by chance. Where no formula is
that near, a buffer that grows by doubling say, the slowest of those
about as near as the nearest is the shape.

What to know when writing one:

- Three values at least: two points are on every formula. Five is what
  the scenarios here take, a thousandfold or more from the first to the
  last. With three, a time that grows a little read as `log` on one run
  and `linear` on another; with five it read the same every time.
- Widen the range downwards. A small input costs nothing to run, and
  the part of the cost that does not grow is in every formula, so it
  does not hide the growth as it would hide it from a ratio.
- The growing part has to show. Where it is under a hundredth of the
  cost at every value, or under the noise, the cost reads as `constant`.
  Take values large enough.
- For time, a growth of some percent over the whole range reads as
  `log` on one run and `constant` on another, as the noise happens to
  be. A line that says `log` passes both ways. Say `constant` for time
  only where nothing at all depends on the value.
- A logarithm and a small power of the value are not told apart.
- A scenario with `%for` has no `---`: the screen differs from pass to
  pass. The other `?` lines work, and are checked in every pass.
- Each value is run five times and the least cost is taken, so a
  scenario with five values is twenty-five runs.

Memory, what is counted:

- A block that is grown counts in `?requested` at its whole new size,
  since growing it may copy all of it, and in `?peak` as the new block
  and the old one held together for that moment.
- Handling a line and composing the frame after it, as the editor's loop
  does after every event. A `---` adds nothing to the count.
- The editor's background jobs, whenever they allocate. Where a line
  starts one, a picker filling its list say, put `wait` before the
  `?` line: without it the count is whatever the job had got to.

What is not: the run's own work, reading the scenario and printing a
frame. A block that the run takes and the editor frees, or the other
way round, is counted on one side only. Those are small and the same at
every value, the events of a key say, so they do not change a growth;
they are why `?retained` is read as a change and not as a total.

Time is the processor time the process used while the editor handled
the lines and composed the frame after each. It is not the time on the
clock: a machine busy with something else makes that longer, and this
no longer. The run's own work between the lines is left out. A background
job is in it while a line is being handled, `wait` included, so put
`wait` before the `?time` as before the lines about memory. Waiting
itself, for the disk or for a lock, takes no processor time and does
not show.

The live replay checks no growth. There the editor is another process
and the run is one pass, with the first value of `%for`; `%measure`
and the `?` lines that read it are passed over with a note.

## Frames

Under the text, the screen has two layers of letters. The marks say
where the cursor, the selection and the search hits are. The inks say
what colour each word has: keyword, string, comment and so on. A frame
shows one layer at a time, because two rows of letters under one line of
text would not be readable:

- `--- CAPTION` shows the text and the marks.
- `---ink CAPTION` shows the text and the inks.

The caption is the author's: it says what the frame should show. When
both layers matter at one point, write the two lines one after the other.

A frame is a diff. The first `---` in a file shows the whole screen; every
later `---` shows only the rows that changed since the previous `---`, or
`same` if nothing did. `---ink` works the same way, but compares with the
previous `---ink`, not with the previous `---`. So the first `---ink` in a
file is always a whole screen, however many `---` came before it.

```text
 0|1 fn main() {}
  |--------------^-------------------------
 1-10|
15| 1/25 …cratch]* · Text · 1 lines · 1:13
```

Before `|` is the row number, right-aligned; after it, the cells. A row
with letters has a second line under it, one letter per column. Empty
rows run together, `1-10|`. Trailing spaces are cut. The editor's own
furniture, line numbers, rules and the status bar, gets no letters: it is
told by where it is and what it says.

Marks: `-` the current line, `~` selected, `=` a search hit, `^` a caret.

Inks: `k` keyword, `s` string, `c` comment, `t` type, `f` function,
`n` constant, `b` builtin, `l` label, `m` module, `+` added, `-` deleted.

## The run

Each scenario runs in an empty directory of its own. The test creates it
at `target/scenarios/<pid>` and removes it when the scenario ends. The
sandbox binary for live replay uses the system's temporary directory
instead. The run changes into that directory, so a relative path typed
into a prompt or given to `%file` lands there, and nothing outside it is
touched.

The clipboard is a fake. It holds only what the scenario copies or what a
`%clipboard` line puts there, and nothing from the real desktop.

A picker streams its candidates from a worker, so a `---` right after one
opens may catch the list half filled. That is what `wait` is for: put it
before the snapshot unless the half-filled frame is the point. The
headless run does not wait on its own, and a missing `wait` shows up when
the scenario is replayed by hand.

## Replaying by hand

The same file drives a live editor, to see what the frames describe or to
find where a scenario and the editor part ways:

```sh
tools/replay sandbox          # in one terminal: the editor, under the run's conditions
tools/replay run FILE.sc      # in another: the scenario, line by line
```

Each line is shown and sent on Enter; `a` sends the rest, `q` stops. At a
`---` the frame is worked out as the test works it out and compared with
the file, and the two are printed where they differ. Debug builds only,
like the headless run.
