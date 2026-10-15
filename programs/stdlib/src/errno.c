#include "errno.h"
/* The storage follows the lifetime of its owning pthread control block. */
extern int *__pthread_errno_location(void);
int *__errno_location(void) { return __pthread_errno_location(); }
