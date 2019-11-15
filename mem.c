#include <stdlib.h>
#include <stdio.h>
#include <sys/types.h>
#include <unistd.h>
#include <time.h>


int main(int agrc, char **argv) {
    long total_allocated = 0;
    int size = 1024;
    void *p[size];
    for (int i = 0; i < size; i++) {
        int alloc_size = 512;
        p[i] = malloc(alloc_size);
        total_allocated += alloc_size;
    }
    printf("Allocated %ld kb\n", total_allocated / 1000);
    printf("Sleeping ...\n");
    getchar();
}