#include "stdlib.h"
#include "polyos.h"
#include "string.h"

static int environ_owned = 0;

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
