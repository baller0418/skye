printf 'movn x2, #17\n' > t.s
aarch64-linux-gnu-as t.s -o t.o && aarch64-linux-gnu-objdump -d t.o