# Two edges of Ctrl+/ over a block. Depth is measured in columns, so a
# line indented with a tab and one with two spaces put the marker at
# column 2; on the tab line that is inside the tab, which cannot be split,
# so the marker goes before it. And a block only half commented is not
# uncommented: every line gets a marker, the commented one a second.

%file mixed.rs "\tone();\n  two();"
%start mixed.rs
size 30 7
key Ctrl+A
key Ctrl+/

--- mixed indents, a tab and two spaces: the marker at the shallowest column, before a tab it cannot split
0|1 //  one();
 |  ~~~~~~~~~~~
1|2   // two();
 |  ~~~~~~~~~~~^
2-5|
6| …rs* · Rust · 2 lines · 2:12

key Ctrl+/
key Ctrl+S
?file mixed.rs "\tone();\n  two();"

--- and comes off again, the tab intact
0|1     one();
 |  ~~~~~~~~~~~
1|2   two();
 |  ~~~~~~~~^
6| …d.rs · Rust · 2 lines · 2:9

key Ctrl+A
paste "one();\ntwo();"
key Ctrl+/
key Ctrl+A
key Ctrl+/

--- a block only half commented is not uncommented: every line gets a marker, the commented one a second
0|1 // one();
 |  ~~~~~~~~~~
1|2 // // two();
 |  ~~~~~~~~~~~~^
6| …rs* · Rust · 2 lines · 2:13

