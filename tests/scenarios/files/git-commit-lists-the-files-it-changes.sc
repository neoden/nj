# The message git asks for at a commit. In its comments git lists the
# files the commit changes, each line after a tab, which a highlight
# example cannot point past; the screen lays the tab out, so it is checked
# here. How each file changes is coloured: a file added green, one deleted
# red, as in a diff; the others a keyword; the paths strings. The same in
# Russian, as git 2.55 writes it under a Russian locale, its first lines
# left in English.

%grammar Git commit
%file COMMIT_EDITMSG "Fix the parse of a deleted SQL comment\n# Please enter the commit message for your changes. Lines starting\n# with '#' will be ignored, and an empty message aborts the commit.\n#\n# On branch master\n# Changes to be committed:\n#\tmodified:   Cargo.toml\n#\tnew file:   queries/diff/highlights.scm\n#\tdeleted:    old.txt\n#\trenamed:    a.rs -> b.rs\n#\n"
%start COMMIT_EDITMSG
size 60 13
wait

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

%reset
%file COMMIT_EDITMSG "\n# Please enter the commit message for your changes. Lines starting\n# with '#' will be ignored.\n#\n# Текущая ветка: master\n# Изменения, которые будут включены в коммит:\n#\tизменено:      a.txt\n#\tновый файл:    q.txt\n#\tудалено:       s.txt\n#\n# Изменения, которые не в индексе для коммита:\n#\tизменено:      c.txt\n#\n"
%start COMMIT_EDITMSG
size 60 14
wait

---ink the same in Russian
 0| 1
 1| 2 # Please enter the commit message for your changes. Lines
  |   ccccccccccccccccccccccccccccccccccccccccccccccccccccccccc
 2| 3 # with '#' will be ignored.
  |   ccccccccccccccccccccccccccc
 3| 4 #
  |   c
 4| 5 # Текущая ветка: master
  |   cccccccccccccccccnnnnnn
 5| 6 # Изменения, которые будут включены в коммит:
  |   cckkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
 6| 7 #   изменено:      a.txt
  |   cccckkkkkkkkkccccccsssss
 7| 8 #   новый файл:    q.txt
  |   cccc+++++++++++ccccsssss
 8| 9 #   удалено:       s.txt
  |   cccc--------cccccccsssss
 9|10 #
  |   c
10|11 # Изменения, которые не в индексе для коммита:
  |   cckkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk
11|12 #   изменено:      c.txt
  |   cccckkkkkkkkkccccccsssss
12|13 #
  |   c
13|               COMMIT_EDITMSG · Git commit · 14 lines · 1:1

