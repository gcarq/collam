#include <stdio.h>
#include <stdlib.h>
#include <string.h>


void test_malloc() {
    printf("Calling malloc...\n");
    char *ptr = (char *)malloc(64);
    memset(ptr, 1, 64);
    printf("ptr: %p\n", ptr);
    printf("calling free...\n");
    free(ptr);
}

void test_calloc() {
    printf("Calling calloc...\n");
    char *ptr = (char *)calloc(8, 8);
    memset(ptr, 1, 64);
    printf("ptr: %p\n", ptr);
    printf("calling free...\n");
    free(ptr);
}

int main(char *argc, char **argv) {
    /*for (int i = 0; i < 8; i++) {
        test_malloc();
    }*/
    test_malloc();
    test_calloc();
}