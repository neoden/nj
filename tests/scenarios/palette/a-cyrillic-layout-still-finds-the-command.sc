# A combination is matched by the physical key, not by the letter the
# layout puts on it (ADR 7): Ctrl+Shift+з, which is Ctrl+Shift+P on a
# Cyrillic layout, opens the palette just the same. Plain letters are never
# mapped, since they are text: typed into the query they stay Cyrillic and
# match no command, which the count says.

size 40 16
paste "one\ntwo\nthree"
key Ctrl+Home
key Ctrl+Shift+з
wait

--- the palette opens as if Ctrl+Shift+P had been pressed
 0|1 one
  |::^-------------------------------------
 1|2 two
 2|────────────────────────────────────────
 3|   Indent                           Tab
 4|   Paste                         Ctrl+V
 5|   Copy                          Ctrl+C
 6|   Redo                    Ctrl+Shift+Z
 7|   Undo                          Ctrl+Z
 8|   Abort                   Ctrl+Shift+Q
 9|   Quit                          Ctrl+Q
10|   Save as                 Ctrl+Shift+S
11|   Save                          Ctrl+S
12|   Cut                           Ctrl+X
  |~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
13| >
  |   ^
14|────────────────────────────────────────
15| 25/25 …cratch]* · Text · 3 lines · 1:1

type "ыуд"

--- the letters go into the query as they are, and match nothing
 2|3 three
 3-11|
12|────────────────────────────────────────
13| > ыуд
  |      ^
15| 0/25 [scratch]* · Text · 3 lines · 1:1

