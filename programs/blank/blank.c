
#include "polyos.h"
#include "stdlib.h"

int main (int argc, char** argv) {
    print("Hello, world!\n");
    print(itoa(1234));
    void* ptr = malloc(10);
    if(ptr){
        print("malloc success\n");
        free(ptr);
    }
    while (1){}
    return 0;
}