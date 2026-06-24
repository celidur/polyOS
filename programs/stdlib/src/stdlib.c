#include "stdlib.h"
#include "polyos.h"
#include "string.h"

typedef struct malloc_block {
    size_t size;
    int free;
    struct malloc_block *next;
} malloc_block_t;

static malloc_block_t *malloc_head = NULL;
static int environ_owned = 0;

static size_t align_size(size_t size)
{
    return (size + sizeof(void *) - 1) & ~(sizeof(void *) - 1);
}

static int can_split_block(malloc_block_t *block, size_t size)
{
    return block->size >= size + sizeof(malloc_block_t) + sizeof(void *);
}

static void split_block(malloc_block_t *block, size_t size)
{
    if (!can_split_block(block, size)) {
        return;
    }

    char *block_data = (char *)(block + 1);
    malloc_block_t *new_block = (malloc_block_t *)(block_data + size);
    new_block->size = block->size - size - sizeof(malloc_block_t);
    new_block->free = 1;
    new_block->next = block->next;

    block->size = size;
    block->next = new_block;
}

static malloc_block_t *find_free_block(size_t size)
{
    malloc_block_t *block = malloc_head;
    while (block) {
        if (block->free && block->size >= size) {
            return block;
        }
        block = block->next;
    }

    return NULL;
}

static malloc_block_t *append_block(size_t size)
{
    malloc_block_t *block = sbrk(sizeof(malloc_block_t) + size);
    if (block == (void *)-1) {
        return NULL;
    }

    block->size = size;
    block->free = 0;
    block->next = NULL;

    if (!malloc_head) {
        malloc_head = block;
        return block;
    }

    malloc_block_t *tail = malloc_head;
    while (tail->next) {
        tail = tail->next;
    }
    tail->next = block;
    return block;
}

static void coalesce_free_blocks(void)
{
    malloc_block_t *block = malloc_head;
    while (block && block->next) {
        char *block_end = (char *)(block + 1) + block->size;
        if (block->free && block->next->free && block_end == (char *)block->next) {
            block->size += sizeof(malloc_block_t) + block->next->size;
            block->next = block->next->next;
            continue;
        }

        block = block->next;
    }
}

static void release_tail_blocks(void)
{
    while (malloc_head) {
        malloc_block_t *prev = NULL;
        malloc_block_t *block = malloc_head;
        while (block->next) {
            prev = block;
            block = block->next;
        }

        if (!block->free) {
            return;
        }

        intptr_t shrink = (intptr_t)(sizeof(malloc_block_t) + block->size);
        if (sbrk(-shrink) == (void *)-1) {
            return;
        }

        if (prev) {
            prev->next = NULL;
        } else {
            malloc_head = NULL;
        }
    }
}

void* malloc(size_t size)
{
    if (size == 0) {
        return NULL;
    }

    size = align_size(size);
    malloc_block_t *block = find_free_block(size);
    if (!block) {
        block = append_block(size);
    }

    if (!block) {
        return NULL;
    }

    split_block(block, size);
    block->free = 0;
    return (void *)(block + 1);
}

void free(void* ptr)
{
    if (!ptr) {
        return;
    }

    malloc_block_t *block = ((malloc_block_t *)ptr) - 1;
    block->free = 1;
    coalesce_free_blocks();
    release_tail_blocks();
}

static int env_name_len(const char *entry)
{
    int len = 0;
    while (entry[len] && entry[len] != '=') {
        len++;
    }
    return len;
}

static int env_name_matches(const char *entry, const char *name)
{
    int name_len = strlen(name);
    return env_name_len(entry) == name_len && strncmp(entry, name, name_len) == 0 && entry[name_len] == '=';
}

static int env_count(void)
{
    int count = 0;
    if (!environ) {
        return 0;
    }

    while (environ[count]) {
        count++;
    }
    return count;
}

static char *dup_string(const char *value)
{
    char *copy = malloc(strlen(value) + 1);
    if (!copy) {
        return NULL;
    }

    strcpy(copy, value);
    return copy;
}

static void free_environment(char **entries, int count)
{
    if (!entries) {
        return;
    }

    for (int i = 0; i < count; i++) {
        free(entries[i]);
    }
    free(entries);
}

static int ensure_owned_environment(void)
{
    if (environ_owned) {
        return 0;
    }

    int count = env_count();
    char **owned = malloc(sizeof(char *) * (count + 1));
    if (!owned) {
        return -1;
    }

    for (int i = 0; i < count; i++) {
        owned[i] = dup_string(environ[i]);
        if (!owned[i]) {
            free_environment(owned, i);
            return -1;
        }
    }

    owned[count] = NULL;
    environ = owned;
    environ_owned = 1;
    return 0;
}

char *getenv(const char *name)
{
    if (!name || !name[0] || !environ) {
        return NULL;
    }

    for (int i = 0; environ[i]; i++) {
        if (env_name_matches(environ[i], name)) {
            return environ[i] + strlen(name) + 1;
        }
    }

    return NULL;
}

int setenv(const char *name, const char *value, int overwrite)
{
    if (!name || !name[0] || !value) {
        return -1;
    }

    for (int i = 0; name[i]; i++) {
        if (name[i] == '=') {
            return -1;
        }
    }

    int count = env_count();
    int existing = -1;
    for (int i = 0; i < count; i++) {
        if (env_name_matches(environ[i], name)) {
            existing = i;
            break;
        }
    }

    if (existing >= 0 && !overwrite) {
        return 0;
    }

    if (ensure_owned_environment() < 0) {
        return -1;
    }

    int name_len = strlen(name);
    int value_len = strlen(value);
    char *entry = malloc(name_len + value_len + 2);
    if (!entry) {
        return -1;
    }

    strcpy(entry, name);
    entry[name_len] = '=';
    strcpy(entry + name_len + 1, value);

    if (existing >= 0) {
        free(environ[existing]);
        environ[existing] = entry;
        return 0;
    }

    char **new_environ = malloc(sizeof(char *) * (count + 2));
    if (!new_environ) {
        free(entry);
        return -1;
    }

    for (int i = 0; i < count; i++) {
        new_environ[i] = environ[i];
    }
    new_environ[count] = entry;
    new_environ[count + 1] = NULL;
    free(environ);
    environ = new_environ;
    return 0;
}

int unsetenv(const char *name)
{
    if (!name || !name[0] || !environ) {
        return -1;
    }

    if (ensure_owned_environment() < 0) {
        return -1;
    }

    int count = env_count();
    for (int i = 0; i < count; i++) {
        if (!env_name_matches(environ[i], name)) {
            continue;
        }

        free(environ[i]);
        for (int j = i; j < count; j++) {
            environ[j] = environ[j + 1];
        }
        return 0;
    }

    return 0;
}

char* itoa(int i){
    static char str[12];
    int loc = 11;
    str[loc] = '\0';
    char neg = 1;
    if (i >= 0){
        neg = 0;
        i = -i;
    }

    while (i){
        str[--loc] = '0' - (i % 10);
        i /= 10;
    }

    if (loc == 11){
        str[--loc] = '0';
    }
    if (neg){
        str[--loc] = '-';
    }
    return &str[loc];
}

char* hex(uint32_t i){
    static char str[12];
    int loc = 11;
    str[loc] = '\0';

    while (i){
        int rem = i % 16;
        if (rem < 10){
            str[--loc] = '0' + rem;
        } else {
            str[--loc] = 'a' + (rem - 10);
        }
        i /= 16;
    }

    if (loc == 11){
        str[--loc] = '0';
    }
    return &str[loc];

}
