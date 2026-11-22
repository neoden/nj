# Ctrl+Shift+P opens the command palette: a query line and above it every
# command that can be asked for by name, with the key it is bound to beside
# it. The list grows upward from the query, the best match nearest it, and
# that one is highlighted; Up walks away from the query, Down back. The
# list is sifted as the query is typed: a command stays when the letters
# typed occur in its name in that order, ranked by how well they fit, and
# the status bar counts the matches. Enter runs the highlighted command,
# and the palette is gone before the command does its work.

size 40 16
paste "one\ntwo\nthree"
key Ctrl+Home
key Ctrl+Shift+P
wait

--- the palette opens: every command, the one nearest the query highlighted
 0|1 one
  |--^-------------------------------------
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
15| 26/26 …cratch]* · Text · 3 lines · 1:1

type "sel"

--- the list is sifted as the query is typed
 2|3 three
 3-5|
 6|────────────────────────────────────────
 7|   Add cursor below      Alt+Shift+Down
 8|   Select all occurrences  Ctrl+Shift+L
 9|   Select next occurrence        Ctrl+D
10|   Select line                   Ctrl+L
11|   Select all                    Ctrl+A
12|   Set language
  |~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
13| > sel
  |      ^
15| 6/26 [scratch]* · Text · 3 lines · 1:1

key Up

--- Up moves the highlight away from the query
11|   Select all                    Ctrl+A
  |~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
12|   Set language

key Enter

--- Enter runs the highlighted command, Select all, and closes the palette
 0|1 one
  |  ~~~~
 1|2 two
  |  ~~~~
 2|3 three
  |  ~~~~~^
 6-14|
15|      [scratch]* · Text · 3 lines · 3:6

