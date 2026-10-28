# What a file is indented with is what the editor indents it with, found
# by looking at the file. In a file of tabs, Tab inserts a tab, Backspace in
# the indentation takes one tab, Enter carries the tabs over, and Shift+Tab
# takes one tab off. On the screen a tab is blanks up to the next stop, so
# the frame cannot tell it from spaces: the file can.

%file tabs.txt "fn a() {\n\tone();\n\tif b {\n\t\ttwo();\n\t}\n}"
%start tabs.txt
size 30 8
key Down
key End
key Enter

--- Enter after `one();` carries the tab over
0|1 fn a() {
1|2     one();
2|3
 |::----^-----------------------
3|4     if b {
4|5         two();
5|6     }
6|7 }
7| …txt* · Text · 7 lines · 3:5

type "x"
key Tab
type "y"

--- Tab in the middle of a line: the cursor jumps to the next stop
2|3     x   y
 |::---------^------------------
7| …xt* · Text · 7 lines · 3:10

key Home
key Backspace

--- Backspace in the indentation takes the whole tab
2|3 x   y
 |::^---------------------------
7| …txt* · Text · 7 lines · 3:1

key Down
key Down
key Home
key Shift+Tab
key Ctrl+S
?file tabs.txt "fn a() {\n\tone();\nx\ty\n\tif b {\n\ttwo();\n\t}\n}"

--- Shift+Tab on `two();` takes one tab off; saved, every indent is a tab
2|3 x   y
4|5     two();
 |::----^-----------------------
7| ….txt · Text · 7 lines · 5:5

