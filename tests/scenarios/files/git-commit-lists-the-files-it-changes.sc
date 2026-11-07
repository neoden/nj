# The message git asks for at a commit. In its comments git lists the
# files the commit changes, each line after a tab, which a highlight
# example cannot point past; the screen lays the tab out, so it is checked
# here. How each file changes is coloured: a file added green, one deleted
# red, as in a diff; the others a keyword; the paths strings.

%grammar Git commit
%file COMMIT_EDITMSG "Fix the parse of a deleted SQL comment\n# Please enter the commit message for your changes. Lines starting\n# with '#' will be ignored, and an empty message aborts the commit.\n#\n# On branch master\n# Changes to be committed:\n#\tmodified:   Cargo.toml\n#\tnew file:   queries/diff/highlights.scm\n#\tdeleted:    old.txt\n#\trenamed:    a.rs -> b.rs\n#\n"
%start COMMIT_EDITMSG
size 60 13

---ink the files a commit changes, coloured by how
 0| 1 Fix the parse of a deleted SQL comment
  |   kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
 1| 2 # Please enter the commit message for your changes. Lines
  |   ccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
 2| 3 # with '#' will be ignored, and an empty message aborts t
  |   ccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
 3| 4 #
  |   c
 4| 5 # On branch master
  |   ccccccccccccnnnnnn
 5| 6 # Changes to be committed:
  |   cckkkkkkkkkkkkkkkkkkkkkkkk
 6| 7 #   modified:   Cargo.toml
  |   cccckkkkkkkkkcccssssssssss
 7| 8 #   new file:   queries/diff/highlights.scm
  |   cccc+++++++++cccsssssssssssssssssssssssssss
 8| 9 #   deleted:    old.txt
  |   cccc--------ccccsssssss
 9|10 #   renamed:    a.rs -> b.rs
  |   cccckkkkkkkkccccssssc  cssss
10|11 #
  |   c
11|12
12|               COMMIT_EDITMSG · Git commit · 12 lines · 1:1

