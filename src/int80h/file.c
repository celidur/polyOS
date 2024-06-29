#include <os/int80/file.h>
#include <os/file.h>
#include <os/task.h>
#include <os/status.h>
#include <os/kheap.h>
#include <os/terminal.h>

void* int80h_command13_fopen(struct interrupt_frame *frame) {
    void *user_memoire = task_get_stack_item(task_current(), 0);
    char filename[1024];
    int res = copy_string_from_task(task_current(), user_memoire, filename, 1024);
    if (res < 0)
        return (void *)res;
    char str[1024];
    user_memoire = task_get_stack_item(task_current(), 1);
    res = copy_string_from_task(task_current(), user_memoire, str, 1024);
    if (res < 0)
        return (void *)res;
    filename[1023] = '\0';
    str[1023] = '\0';

    return (void *)(uint32_t)fopen(filename, str);
}

// TODO: Update this function to be more clean
void* int80h_command14_fread(struct interrupt_frame *frame) {
    int fd = (int)task_get_stack_item(task_current(), 0);
    void *ptr = (void *)task_get_stack_item(task_current(), 1);
    uint32_t size = (uint32_t)task_get_stack_item(task_current(), 2);
    int res = 0;

    void *data = kzalloc(size);
    if (!data) {
        res = -ENOMEM;
        goto out;
    }
    res = fread(fd, data, size);
    if (res < 0) {
        goto free_data;
    }

    res = copy_string_to_task(task_current(), data, ptr, size);   
free_data: 
    kfree(data);
out:
    return (void *)res;
}

void* int80h_command15_fwrite(struct interrupt_frame *frame) {
    int fd = (int)task_get_stack_item(task_current(), 0);
    void *ptr = (void *)task_get_stack_item(task_current(), 1);
    uint32_t size = (uint32_t)task_get_stack_item(task_current(), 2);
    int res = 0;

    void *data = kzalloc(size + 1);
    if (!data) {
        res = -ENOMEM;
        goto out;
    }

    res = copy_string_from_task(task_current(), ptr, data, size + 1);
    if (res < 0)
        goto free_data;

    res = fwrite(fd, data, size);
free_data:
    kfree(data);
out:
    return (void *)res;
}

void* int80h_command16_fseek(struct interrupt_frame *frame) {
    int fd = (int)task_get_stack_item(task_current(), 0);
    uint32_t offset = (uint32_t)task_get_stack_item(task_current(), 1);
    FILE_SEEK_MODE mode = (FILE_SEEK_MODE)task_get_stack_item(task_current(), 2);
    return (void *)(uint32_t)fseek(fd, offset, mode);
}

void* int80h_command17_fstat(struct interrupt_frame *frame) {
    int fd = (int)task_get_stack_item(task_current(), 0);
    void *ptr = (void *)task_get_stack_item(task_current(), 1);
    struct file_stat stat;
    int res = fstat(fd, &stat);
    if (res < 0)
        return (void *)res;
    res = copy_string_to_task(task_current(), &stat, ptr, sizeof(struct file_stat));
    return (void *)res;
}

void* int80h_command18_fclose(struct interrupt_frame *frame) {
    int fd = (int)task_get_stack_item(task_current(), 0);
    return (void *)(uint32_t)fclose(fd);
}