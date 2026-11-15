# A Makefile's recipe is a shell command, and it is coloured as Shell:
# its flags, its strings, and the command plain, as a call. What make
# expands in it before the shell sees it stays make's: `$(CC)`, the
# variables make sets for a rule, `$^`. A recipe's line starts with a
# tab, which a highlight example cannot point past; the screen lays the
# tab out, so it is checked here.

%grammar Makefile
%grammar Shell
%file Makefile "build: $(OBJS)\n\t$(CC) -o app $^ && echo \"done $$USER\"\n\t@rm -f *.tmp\n"
%start Makefile
size 60 5
wait

---ink a recipe is Shell, what make expands in it is make's
0|1 build: $(OBJS)
 |  fffff  nnnnnnn
1|2         $(CC) -o app $^ && echo "done $$USER"
 |          bbbbb nn     bb    bbbb sssssssssssss
2|3         @rm -f *.tmp
 |              nn
3|4
4|                        Makefile · Makefile · 4 lines · 1:1

