#ifndef PATHPPARSER_H
#define PATHPPARSER_H

struct path_root
{
    int drive_no;
    struct path_part *first;
};

struct path_part
{
    const char *part;
    struct path_part *next;
};

struct path_root *path_parser_parse(const char *path, const char *cwd);
void path_parser_free(struct path_root *root);

#endif
