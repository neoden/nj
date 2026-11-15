# Text piped in has no file name to tell its language from, so it would
# be plain text; `--language` names it, and the text is coloured from the
# start. It names the language over a file's name too.

%grammar Rust
%stdin "fn main() {}"
%start --language Rust
size 40 3
wait

---ink the pipe is Rust
0|1 fn main() {}
 |  kk ffff
1|
2|         [stdin] · Rust · 1 lines · 1:1

%reset
%file notes.txt "fn main() {}"
%start --language Rust notes.txt
size 40 3
wait

---ink a .txt file is Rust when told so
0|1 fn main() {}
 |  kk ffff
1|
2|       notes.txt · Rust · 1 lines · 1:1

