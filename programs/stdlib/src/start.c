#include "polyos.h"
#include "stdio.h"

char **environ;
unsigned long *__polyos_auxv;

extern int main(int argc, char** argv, char** envp);

void c_start(int argc, char** argv, char** envp) {
    environ = envp;
    char **end = envp; while (*end) ++end; __polyos_auxv = (unsigned long *)(end+1);
    int res = main(argc, argv, envp);
    if (argc > 0) {
        serial_printf("%s: exited with code %d\n", argv[0], res);
    } else {
        serial_printf("process exited with code %d\n", res);
    }
    _exit(res);
}
