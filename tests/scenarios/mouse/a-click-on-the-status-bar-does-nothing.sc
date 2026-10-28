# The status bar is not the text: a click on it, with or without Shift,
# changes nothing, and a drag that strays onto it is not followed. The
# empty rows under the last line are still the text's, and a click there
# lands on the last line.

size 40 6
paste "alpha\nbeta"
key Ctrl+Home
key Shift+End

--- the first line selected
0|1 alpha
 |  ~~~~~^
1|2 beta
2-4|
5|      [scratch]* · Text · 2 lines · 1:6

click 20 5

--- a click on the status bar: nothing
same

click 20 5 Shift

--- with Shift: nothing either
same

drag 20 5

--- a drag onto the status bar is not followed
same

click 20 3

--- a click under the last line lands on it
0|1 alpha
1|2 beta
 |::----^---------------------------------
5|      [scratch]* · Text · 2 lines · 2:5

