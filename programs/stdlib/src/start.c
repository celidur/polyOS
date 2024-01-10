#include "polyos.h"

extern int main(int argc, char** argv);

void c_start(){
    struct process_arguments args;
    polyos_process_get_args(&args);

    int res = main(args.argc, args.argv);
    if (res == 0){
        
    }
}