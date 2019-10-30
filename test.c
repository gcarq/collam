#include <stdio.h>
#include <stdlib.h>

int main(char *argc, char **argv) {
    printf("Calling malloc...\n");
    char *ptr = (char *)malloc(64);
    printf("ptr: %p\n", ptr);
    printf("calling free...\n");
    free(ptr);
}