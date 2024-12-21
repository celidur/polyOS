#include <os/process.h>
#include <os/memory.h>
#include <os/config.h>
#include <os/status.h>
#include <os/task.h>
#include <os/string.h>
#include <os/file.h>
#include <os/kernel.h>
#include <os/paging.h>
#include <os/kheap.h>

struct process *current_process = NULL;

static struct process *processes[MAX_PROCESS] = {NULL};

int process_free_process(struct process* process);

static void process_init(struct process *process)
{
    memset(process, 0, sizeof(struct process));
}

struct process *process_current()
{
    return current_process;
}

struct process *process_get(int process_id)
{
    if (process_id < 0 || process_id >= MAX_PROCESS)
    {
        return NULL;
    }
    return processes[process_id];
}

static int process_load_binary(const char *filename, struct process *process)
{
    int res = 0;
    int fd = fopen(filename, "r");
    if (fd <= 0)
    {
        res = -EIO;
        goto out;
    }

    struct file_stat stat;
    res = fstat(fd, &stat);
    if (res != ALL_OK)
    {
        goto out;
    }

    void *program_data_ptr = kzalloc(stat.size);
    if (!program_data_ptr)
    {
        res = -ENOMEM;
        goto out;
    }

    if (fread(fd, program_data_ptr, stat.size) != stat.size)
    {
        res = -EIO;
        goto out;
    }

    process->filetype = PROCESS_FILETYPE_BINARY;
    process->ptr = program_data_ptr;
    process->size = stat.size;

out:
    fclose(fd);
    return res;
}

static int process_load_elf(const char *filename, struct process *process){
    struct elf_file *elf_file = NULL;
    int res = elf_load(filename, &elf_file);
    if (res != ALL_OK){
        return res;
    }

    process->filetype = PROCESS_FILETYPE_ELF;
    process->elf_file = elf_file;
    return ALL_OK;
}


static int process_load_data(const char *filename, struct process *process)
{
    int res = 0;
    res = process_load_elf(filename, process);
    if (res == -EINFORMAT){
        return process_load_binary(filename, process);
    }
    return res;
}

static int process_map_elf(struct process* process){
    struct elf_file* elf_file = process->elf_file;
    struct elf_header* header = elf_header(elf_file);
    struct elf32_phdr* phdrs = elf_pheader(header);
    for (int i = 0; i < header->e_phnum; i++){
        struct elf32_phdr* phdr = &phdrs[i];
        void* phdr_phys_adress = elf_phdr_phys_address(elf_file, phdr);
        int flags = PAGING_IS_PRESENT | PAGING_ACCESS_FROM_ALL;
        if (phdr->p_flags & PF_W){
            flags |= PAGING_IS_WRITABLE;
        }
        int res = paging_map_to(process->task->page_directory, paging_align_to_lower_page((void*) phdr->p_vaddr), paging_align_to_lower_page(phdr_phys_adress), paging_align_address(phdr_phys_adress + phdr->p_memsz), flags);
        if (res < 0){
            return res;
        }
    }
    return ALL_OK;
}

static int process_map_binary(struct process *process)
{
    paging_map_to(process->task->page_directory, (void *)PROGRAM_VIRTUAL_ADDRESS, process->ptr, paging_align_address(process->ptr + process->size), PAGING_IS_PRESENT | PAGING_IS_WRITABLE | PAGING_ACCESS_FROM_ALL);
    return 0;
}
static int process_map_memory(struct process* process)
{
    int res = 0;

    res = paging_map_to(process->task->page_directory, (void *)USER_PROGRAM_VIRTUAL_STACK_ADDRESS_END, process->stack, paging_align_address(process->stack + USER_PROGRAM_STACK_SIZE + 1), PAGING_IS_PRESENT | PAGING_IS_WRITABLE | PAGING_ACCESS_FROM_ALL);
    if (res < 0)
    {
        return res;
    }
    switch(process->filetype)
    {
        case PROCESS_FILETYPE_ELF:
            res = process_map_elf(process);
        break;

        case PROCESS_FILETYPE_BINARY:
            res = process_map_binary(process);
        break;

        default:
            kernel_panic("process_map_memory: Invalid filetype\n");
    }

    if (res < 0)
    {
       return res;
    }
    return res;
}

int process_load_for_slot(const char *filename, struct process **process, int process_slot)
{
    int res = 0;
    struct process *_process = NULL;

    if (process_get(process_slot))
    {
        res = -EINVARG;
        goto out;
    }

    _process = kzalloc(sizeof(struct process));
    if (!_process)
    {
        res = -ENOMEM;
        goto out;
    }

    process_init(_process);
    res = process_load_data(filename, _process);
    if (res != ALL_OK)
    {
        goto out;
    }

    _process->stack = kzalloc(USER_PROGRAM_STACK_SIZE);
    if (!_process->stack)
    {
        res = -ENOMEM;
        goto out;
    }

    strncpy(_process->filename, filename, sizeof(_process->filename));
    _process->pid = process_slot;

    // create task
    _process->task = task_new(_process);
    if (ERROR_I(_process->task) == 0)
    {
        res = ERROR_I(_process->task);
        _process->task = NULL;
        goto out;
    }

    res = process_map_memory(_process);
    if (res != ALL_OK)
    {
        goto out;
    }

    *process = _process;
    processes[process_slot] = _process;

    struct command_argument args;
    args.next = NULL;
    strncpy(args.argument, filename + 3, sizeof(args.argument));

    res = process_inject_arguments(_process, &args);
out:
    if (ISERR(res))
    {
        if (_process)
        {
            process_free_process(_process);
            _process = NULL;
            *process = NULL;
        }
    }
    return res;
}

int process_get_free_slot()
{
    for (int i = 0; i < MAX_PROCESS; i++)
    {
        if (!processes[i])
        {
            return i;
        }
    }
    return -EISTKN;
}

int process_load(const char *filename, struct process **process)
{
    int process_slot = process_get_free_slot();
    if (process_slot < 0)
        return -EISTKN;

    return process_load_for_slot(filename, process, process_slot);
}

int task_page_task(struct task *task)
{
    user_registers();
    paging_switch(task->page_directory);
    return 0;
}

void *task_get_stack_item(struct task *task, int item)
{
    void *result = NULL;
    uint32_t *stack = (uint32_t *)task->regs.esp;
    task_page_task(task);
    result = (void *)stack[item];
    kernel_page();
    return result;
}

int process_switch(struct process *process)
{
    current_process = process;
    return 0;
}

int process_load_switch(const char *filename, struct process **process)
{
    int res = process_load(filename, process);
    if (res == ALL_OK)
    {
        process_switch(*process);
    }
    return res;
}

static int process_find_free_allocation_index(struct process *process)
{
    for (int i = 0; i < MAX_PROGRAM_ALLOCATIONS; i++)
    {
        if (!process->allocations[i].ptr)
        {
            return i;
        }
    }
    return -ENOMEM;
}

void* process_malloc(struct process* process, size_t size){
    int index = process_find_free_allocation_index(process);
    if (index < 0){
        return NULL;
    }

    void* ptr = kzalloc(size);
    if (!ptr){
        return NULL;
    }

    int res = paging_map_to(process->task->page_directory, ptr, ptr, paging_align_address(ptr + size), PAGING_IS_PRESENT | PAGING_IS_WRITABLE | PAGING_ACCESS_FROM_ALL);
    if (res < 0){
        kfree(ptr);
        return NULL;
    }

    process->allocations[index].ptr = ptr;
    process->allocations[index].size = size;
    return ptr;
}

static bool process_is_process_pointer(struct process* process, void* ptr){
    for (int i = 0; i < MAX_PROGRAM_ALLOCATIONS; i++){
        if (process->allocations[i].ptr == ptr){
            return true;
        }
    }
    return false;
}

static void process_allocation_unjoin(struct process* process, void* ptr){
    for (int i = 0; i < MAX_PROGRAM_ALLOCATIONS; i++){
        if (process->allocations[i].ptr == ptr){
            process->allocations[i].ptr = NULL;
            process->allocations[i].size = 0;
        }
    }
}

static struct process_allocation* process_get_allocation_by_addr(struct process* process, void* ptr){
    for (int i = 0; i < MAX_PROGRAM_ALLOCATIONS; i++){
        if (process->allocations[i].ptr == ptr){
            return &process->allocations[i];
        }
    }
    return NULL;
}

void process_free(struct process* process, void* ptr){
    struct process_allocation* allocation = process_get_allocation_by_addr(process, ptr);
    if (!allocation){
        return;
    }
    int res = paging_map_to(process->task->page_directory, allocation->ptr, allocation->ptr, paging_align_address(allocation->ptr + allocation->size), 0x00);
    if (res < 0){
        return;
    }

    process_allocation_unjoin(process, ptr);
    kfree(ptr);
}

void process_get_arguments(struct process* process, int* argc, char*** argv){
    *argc = process->arguments.argc;
    *argv = process->arguments.argv;
}

int process_count_command_arguments(struct command_argument* root_command){
    int count = 0;
    struct command_argument* current = root_command;
    while(current){
        count++;
        current = current->next;
    }
    return count;
}


int process_inject_arguments(struct process* process, struct command_argument* root_command){
    struct command_argument* current = root_command;
    int i = 0;
    int argc = process_count_command_arguments(root_command);
    if (argc == 0){
        return -EIO;
    }

    char** argv = process_malloc(process, sizeof(char*) * argc);
    if (!argv){
        return -ENOMEM;
    }

    while (current){
        char* arguement_str = process_malloc(process, sizeof(current->argument));
        if (!arguement_str){
            return -ENOMEM;
        }
        strncpy(arguement_str, current->argument, sizeof(current->argument));
        argv[i] = arguement_str;
        i++;
        current = current->next;
    }
    process->arguments.argc = argc;
    if (process->arguments.argv) {
        kfree(process->arguments.argv);
    }
    process->arguments.argv = argv;
    return ALL_OK;
}

static int process_terminate_allocations(struct process* process){
    for (int i = 0; i < MAX_PROGRAM_ALLOCATIONS; i++){
        if (process->allocations[i].ptr){
            process_free(process, process->allocations[i].ptr);
        }
    }
    return ALL_OK;
}

static int process_free_binary_data(struct process* process){
    if (process->ptr)
    {
        kfree(process->ptr);
    }
    return ALL_OK;
}

static int process_free_elf_data(struct process* process){
    if (process->elf_file)
    {
        elf_close(process->elf_file);
    }
    return ALL_OK;
}

static int process_free_program_data(struct process* process){
    switch(process->filetype){
        case PROCESS_FILETYPE_ELF:
            return process_free_elf_data(process);
        case PROCESS_FILETYPE_BINARY:
            return process_free_binary_data(process);
        default:
            return -EINFORMAT;
    }
}

void process_switch_to_any(){
    for (int i = 0; i < MAX_PROCESS; i++){
        if (processes[i]){
            process_switch(processes[i]);
            return;
        }
    }
    kernel_panic("No processes to switch too\n");
}

static void process_unlink(struct process* process){
    processes[process->pid] = NULL;
    if (current_process == process){
        process_switch_to_any();
    }
}

int process_free_process(struct process* process){
    process_terminate_allocations(process);
    process_free_program_data(process);

    if (process->stack)
    {    
        kfree(process->stack);
        process->stack = NULL;
    }

    if (process->task)
    {
        task_free(process->task);
        process->task = NULL;
    }

    kfree(process);

    return ALL_OK;
}

int process_terminate(struct process* process)
{
    // Unlink the process from the process array.
    process_unlink(process);
    int res = process_free_process(process);
    if (res < 0)
    {
        goto out;
    }

out:
    return res;
}