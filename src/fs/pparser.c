#include <os/pparser.h>
#include <os/config.h>
#include <os/memory.h>
#include <os/kheap.h>
#include <os/string.h>
#include <os/status.h>

static int path_parser_path_valid_format(const char *filename)
{
    int len = strnlen(filename, MAX_PATH);
    // path is like "0:/path/to/file"
    return (len >= 3 && isdigit(filename[0]) && memcmp((void *)&filename[1], ":/", 2) == 0);
}

static int path_parser_get_drive_by_path(const char **path)
{
    if (!path_parser_path_valid_format(*path))
    {
        return -EBADPATH;
    }

    int drive_no = to_numeric_digit(*path[0]);

    // add 3 to skip "0:/" part
    *path += 3;
    return drive_no;
}

static struct path_root *path_parser_create_root(int drive_no)
{
    struct path_root *root = kmalloc(sizeof(struct path_root));
    root->drive_no = drive_no;
    root->first = NULL;
    return root;
}

static char *path_parser_get_path_part(const char **path)
{
    char *result_path_part = kmalloc(MAX_PATH);
    int i = 0;
    while (**path != '/' && **path != '\0')
    {
        result_path_part[i++] = **path;
        (*path)++;
    }
    if (**path == '/')
    {
        (*path)++;
    }
    if (i == 0)
    {
        kfree(result_path_part);
        return NULL;
    }
    return result_path_part;
}

struct path_part *path_parser_parse_path_part(struct path_part *last_part, const char **path)
{
    const char *path_part_str = path_parser_get_path_part(path);
    if (!path_part_str)
    {
        return NULL;
    }

    struct path_part *part = kmalloc(sizeof(struct path_part));
    part->part = path_part_str;
    part->next = NULL;

    if (last_part)
    {
        last_part->next = part;
    }

    return part;
}

void path_parser_free(struct path_root *root)
{
    struct path_part *part = root->first;
    while (part)
    {
        struct path_part *next = part->next;
        kfree((void *)part->part);
        kfree(part);
        part = next;
    }
    kfree(root);
}

struct path_root *path_parser_parse(const char *path, const char *cwd)
{
    const char *tmp_path = path;
    if (strlen(path) > MAX_PATH)
    {
        return NULL;
    }
    int res = path_parser_get_drive_by_path(&tmp_path);
    if (res < 0)
    {
        return NULL;
    }

    struct path_root *root = path_parser_create_root(res);
    if (!root)
    {
        return NULL;
    }

    struct path_part *first_part = path_parser_parse_path_part(NULL, &tmp_path);
    if (!first_part)
    {
        return NULL;
    }
    root->first = first_part;
    struct path_part *part = path_parser_parse_path_part(first_part, &tmp_path);
    while (part)
    {
        part = path_parser_parse_path_part(part, &tmp_path);
    }
    return root;
}