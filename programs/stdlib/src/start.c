#include "polyos.h"
#include "stdio.h"

char **environ;

extern int main(int argc, char** argv, char** envp);

void c_start(int argc, char** argv, char** envp) {
    environ = envp;
    int res = main(argc, argv, envp);
    if (argc > 0) {
        serial_printf("%s: exited with code %d\n", argv[0], res);
    } else {
        serial_printf("process exited with code %d\n", res);
    }
    _exit(res);
}
