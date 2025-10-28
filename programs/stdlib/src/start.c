#include "polyos.h"
#include "stdio.h"

extern int main(int argc, char** argv);

void c_start(){
    struct process_arguments args;
    polyos_process_get_args(&args);

    int res = main(args.argc, args.argv);
    if (args.argc > 0) {
        serial_printf("%s: exited with code %d\n", args.argv[0], res);
    } else {
        serial_printf("process exited with code %d\n", res);
    }
}