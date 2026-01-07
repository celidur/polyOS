#include "polyos.h"
#include "stdio.h"

extern int main(int argc, char** argv);

void c_start(int argc, char** argv) {
    int res = main(argc, argv);
    if (argc > 0) {
        serial_printf("%s: exited with code %d\n", argv[0], res);
    } else {
        serial_printf("process exited with code %d\n", res);
    }
}