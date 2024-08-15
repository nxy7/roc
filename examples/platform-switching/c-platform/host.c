#include <errno.h>
#include <signal.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

#ifdef _WIN32
#else
#include <sys/shm.h> // shm_open
#include <sys/mman.h> // for mmap
#include <signal.h> // for kill
#endif

void* roc_alloc(size_t size, unsigned int alignment) { return malloc(size); }

void* roc_realloc(void* ptr, size_t new_size, size_t old_size, unsigned int alignment) {
  return realloc(ptr, new_size);
}

void roc_dealloc(void* ptr, unsigned int alignment) { free(ptr); }

void roc_panic(void* ptr, unsigned int alignment) {
  char* msg = (char*)ptr;
  fprintf(stderr,
          "Application crashed with message\n\n    %s\n\nShutting down\n", msg);
  exit(1);
}

void roc_dbg(char* loc, char* msg, char* src) {
  fprintf(stderr, "[%s] %s = %s\n", loc, src, msg);
}

void* roc_memset(void* str, int c, size_t n) { return memset(str, c, n); }

int roc_shm_open(char* name, int oflag, int mode) {
#ifdef _WIN32
    return 0;
#else
    return shm_open(name, oflag, mode);
#endif
}
void* roc_mmap(void* addr, int length, int prot, int flags, int fd, int offset) {
#ifdef _WIN32
    return addr;
#else
    return mmap(addr, length, prot, flags, fd, offset);
#endif
}

int roc_getppid() {
#ifdef _WIN32
    return 0;
#else
    return getppid();
#endif
}

struct RocStr {
  char* bytes;
  size_t len;
  size_t capacity;
};

bool is_small_str(struct RocStr str) { return ((ssize_t)str.capacity) < 0; }

// Determine the length of the string, taking into
// account the small string optimization
size_t roc_str_len(struct RocStr str) {
  char* bytes = (char*)&str;
  char last_byte = bytes[sizeof(str) - 1];
  char last_byte_xored = last_byte ^ 0b10000000;
  size_t small_len = (size_t)(last_byte_xored);
  size_t big_len = str.len;

  // Avoid branch misprediction costs by always
  // determining both small_len and big_len,
  // so this compiles to a cmov instruction.
  if (is_small_str(str)) {
    return small_len;
  } else {
    return big_len;
  }
}

struct MyStruct { uint8_t x; uint8_t y; };
extern void roc__mainForHost_1_exposed_generic(struct MyStruct *out);

int main() {

  struct MyStruct r;
  roc__mainForHost_1_exposed_generic(&r);


  int x = r.x;
  int y = r.y;

  int sum = x + y;
  printf("%d + %d = %d\n", x, y, sum);
}
